use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::sync::RwLock;

// Default number of connections
const DEFAULT_CONNECTIONS: u64 = 8;

// Download state shared across the app
pub struct AppState {
    downloads: RwLock<HashMap<String, Arc<DownloadHandle>>>,
    settings: RwLock<Settings>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Settings {
    pub connections: u64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            connections: DEFAULT_CONNECTIONS,
        }
    }
}

// Handle to control a download
pub struct DownloadHandle {
    pub id: String,
    pub cancelled: AtomicBool,
    pub paused: AtomicBool,
    pub chunk_downloaded: Vec<Arc<AtomicU64>>,
}

#[derive(Clone, Serialize)]
pub struct UrlInfo {
    pub url: String,
    pub filename: String,
    pub size: Option<u64>,
    pub resumable: bool,
}

#[derive(Clone, Serialize)]
pub struct DownloadProgress {
    pub id: String,
    pub downloaded: u64,
    pub total: u64,
    pub speed: f64,
    pub status: String, // "downloading", "paused", "completed", "error", "cancelled"
    pub chunk_progress: Vec<ChunkProgress>,
}

#[derive(Clone, Serialize)]
pub struct ChunkProgress {
    pub id: u64,
    pub downloaded: u64,
    pub total: u64,
}

#[derive(Clone, Serialize)]
pub struct DownloadComplete {
    pub id: String,
    pub path: String,
    pub filename: String,
    pub total_size: u64,
}

#[derive(Clone, Serialize)]
pub struct DownloadError {
    pub id: String,
    pub error: String,
}

#[tauri::command]
async fn fetch_url_info(url: String) -> Result<UrlInfo, String> {
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .map_err(|e| format!("Failed to create client: {}", e))?;

    let response = client
        .head(&url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()));
    }

    let final_url = response.url().to_string();
    let headers = response.headers();

    let size = headers
        .get(reqwest::header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse().ok());

    let resumable = headers
        .get(reqwest::header::ACCEPT_RANGES)
        .and_then(|v| v.to_str().ok())
        .map(|v| v == "bytes")
        .unwrap_or(false);

    let filename = headers
        .get(reqwest::header::CONTENT_DISPOSITION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| {
            v.split("filename=").nth(1).map(|s| s.trim_matches('"').to_string())
        })
        .or_else(|| extract_filename_from_url(&final_url))
        .or_else(|| extract_filename_from_url(&url))
        .unwrap_or_else(|| "download".to_string());

    Ok(UrlInfo {
        url: final_url,
        filename,
        size,
        resumable,
    })
}

fn extract_filename_from_url(url: &str) -> Option<String> {
    url.split('?').next()
        .and_then(|path| path.split('/').last())
        .filter(|s| !s.is_empty() && s.contains('.'))
        .map(|s| s.to_string())
}

#[derive(Clone, Serialize)]
pub struct FileExistsInfo {
    pub exists: bool,
    pub suggested_name: String,
}

#[tauri::command]
async fn check_file_exists(filename: String) -> Result<FileExistsInfo, String> {
    let download_dir = dirs::download_dir()
        .ok_or_else(|| "Could not find downloads directory".to_string())?;

    let file_path = download_dir.join(&filename);

    if file_path.exists() {
        // Generate a suggested name with counter
        let suggested = generate_unique_filename(&download_dir, &filename);
        Ok(FileExistsInfo {
            exists: true,
            suggested_name: suggested,
        })
    } else {
        Ok(FileExistsInfo {
            exists: false,
            suggested_name: filename,
        })
    }
}

fn generate_unique_filename(dir: &PathBuf, filename: &str) -> String {
    let path = std::path::Path::new(filename);
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");

    let mut counter = 1;
    loop {
        let new_name = if ext.is_empty() {
            format!("{} ({})", stem, counter)
        } else {
            format!("{} ({}).{}", stem, counter, ext)
        };

        if !dir.join(&new_name).exists() {
            return new_name;
        }
        counter += 1;
    }
}

#[tauri::command]
async fn start_download(
    app: AppHandle,
    url: String,
    filename: String,
    size: u64,
    resumable: bool,
) -> Result<String, String> {
    let state = app.state::<AppState>();
    let settings = state.settings.read().await;
    let num_connections = settings.connections;
    drop(settings);

    let download_dir = dirs::download_dir()
        .ok_or_else(|| "Could not find downloads directory".to_string())?;

    let file_path = download_dir.join(&filename);

    // Generate unique download ID
    let download_id = format!("{}_{}", filename, chrono::Utc::now().timestamp_millis());

    // Create download handle
    let handle = Arc::new(DownloadHandle {
        id: download_id.clone(),
        cancelled: AtomicBool::new(false),
        paused: AtomicBool::new(false),
        chunk_downloaded: (0..num_connections)
            .map(|_| Arc::new(AtomicU64::new(0)))
            .collect(),
    });

    // Store handle
    {
        let mut downloads = state.downloads.write().await;
        downloads.insert(download_id.clone(), Arc::clone(&handle));
    }

    // Spawn download task
    let app_clone = app.clone();
    let download_id_clone = download_id.clone();

    tokio::spawn(async move {
        let result = if resumable && size > 0 {
            download_chunked(app_clone.clone(), handle, url, file_path, size, num_connections).await
        } else {
            download_single(app_clone.clone(), handle, url, file_path).await
        };

        // Remove from active downloads
        let state = app_clone.state::<AppState>();
        let mut downloads = state.downloads.write().await;
        downloads.remove(&download_id_clone);

        if let Err(e) = result {
            let _ = app_clone.emit("download-error", DownloadError {
                id: download_id_clone,
                error: e,
            });
        }
    });

    Ok(download_id)
}

#[tauri::command]
async fn cancel_download(app: AppHandle, id: String) -> Result<(), String> {
    let state = app.state::<AppState>();
    let downloads = state.downloads.read().await;

    if let Some(handle) = downloads.get(&id) {
        handle.cancelled.store(true, Ordering::SeqCst);
        Ok(())
    } else {
        Err("Download not found".to_string())
    }
}

#[tauri::command]
async fn pause_download(app: AppHandle, id: String) -> Result<(), String> {
    let state = app.state::<AppState>();
    let downloads = state.downloads.read().await;

    if let Some(handle) = downloads.get(&id) {
        handle.paused.store(true, Ordering::SeqCst);
        Ok(())
    } else {
        Err("Download not found".to_string())
    }
}

#[tauri::command]
async fn resume_download(app: AppHandle, id: String) -> Result<(), String> {
    let state = app.state::<AppState>();
    let downloads = state.downloads.read().await;

    if let Some(handle) = downloads.get(&id) {
        handle.paused.store(false, Ordering::SeqCst);
        Ok(())
    } else {
        Err("Download not found".to_string())
    }
}

#[tauri::command]
async fn set_connections(app: AppHandle, connections: u64) -> Result<(), String> {
    if connections < 1 || connections > 32 {
        return Err("Connections must be between 1 and 32".to_string());
    }

    let state = app.state::<AppState>();
    let mut settings = state.settings.write().await;
    settings.connections = connections;
    Ok(())
}

#[tauri::command]
async fn get_connections(app: AppHandle) -> Result<u64, String> {
    let state = app.state::<AppState>();
    let settings = state.settings.read().await;
    Ok(settings.connections)
}

async fn download_chunked(
    app: AppHandle,
    handle: Arc<DownloadHandle>,
    url: String,
    file_path: PathBuf,
    total_size: u64,
    num_connections: u64,
) -> Result<String, String> {
    let chunk_size = total_size / num_connections;
    let download_id = handle.id.clone();

    // Calculate chunk ranges
    let mut chunks: Vec<(u64, u64, u64)> = Vec::new();
    for i in 0..num_connections {
        let start = i * chunk_size;
        let end = if i == num_connections - 1 {
            total_size - 1
        } else {
            (i + 1) * chunk_size - 1
        };
        chunks.push((i, start, end));
    }

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .map_err(|e| format!("Failed to create client: {}", e))?;

    // Create temp directory for chunks
    let temp_dir = file_path.parent().unwrap().join(format!(".wdm_temp_{}", download_id));
    tokio::fs::create_dir_all(&temp_dir)
        .await
        .map_err(|e| format!("Failed to create temp directory: {}", e))?;

    // Spawn progress reporter
    let app_clone = app.clone();
    let handle_clone = Arc::clone(&handle);
    let chunk_sizes: Vec<u64> = chunks.iter().map(|(_, start, end)| end - start + 1).collect();
    let download_id_clone = download_id.clone();

    let progress_handle = tokio::spawn(async move {
        let mut last_total = 0u64;
        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            if handle_clone.cancelled.load(Ordering::SeqCst) {
                break;
            }

            let is_paused = handle_clone.paused.load(Ordering::SeqCst);

            let chunk_progress: Vec<ChunkProgress> = handle_clone.chunk_downloaded
                .iter()
                .enumerate()
                .map(|(i, downloaded)| ChunkProgress {
                    id: i as u64,
                    downloaded: downloaded.load(Ordering::Relaxed),
                    total: chunk_sizes[i],
                })
                .collect();

            let total_downloaded: u64 = chunk_progress.iter().map(|c| c.downloaded).sum();
            let speed = if is_paused { 0.0 } else { (total_downloaded.saturating_sub(last_total)) as f64 * 10.0 };
            last_total = total_downloaded;

            let status = if is_paused { "paused" } else { "downloading" };

            let progress = DownloadProgress {
                id: download_id_clone.clone(),
                downloaded: total_downloaded,
                total: total_size,
                speed,
                status: status.to_string(),
                chunk_progress,
            };

            let _ = app_clone.emit("download-progress", &progress);

            if total_downloaded >= total_size {
                break;
            }
        }
    });

    // Download all chunks in parallel
    let mut handles_vec = Vec::new();

    for (chunk_id, start, end) in chunks {
        let client = client.clone();
        let url = url.clone();
        let temp_dir = temp_dir.clone();
        let downloaded = Arc::clone(&handle.chunk_downloaded[chunk_id as usize]);
        let handle_clone = Arc::clone(&handle);

        let task = tokio::spawn(async move {
            download_chunk(client, url, temp_dir, chunk_id, start, end, downloaded, handle_clone).await
        });
        handles_vec.push(task);
    }

    // Wait for all chunks to complete
    let results: Vec<Result<PathBuf, String>> = futures::future::join_all(handles_vec)
        .await
        .into_iter()
        .map(|r| r.map_err(|e| format!("Task failed: {}", e))?)
        .collect();

    // Stop progress reporter
    progress_handle.abort();

    // Check if cancelled
    if handle.cancelled.load(Ordering::SeqCst) {
        // Cleanup temp files
        let _ = tokio::fs::remove_dir_all(&temp_dir).await;

        let _ = app.emit("download-progress", DownloadProgress {
            id: download_id,
            downloaded: 0,
            total: total_size,
            speed: 0.0,
            status: "cancelled".to_string(),
            chunk_progress: vec![],
        });

        return Err("Download cancelled".to_string());
    }

    // Check for errors
    let chunk_paths: Vec<PathBuf> = results
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;

    // Merge chunks into final file
    merge_chunks(&chunk_paths, &file_path).await?;

    // Cleanup temp files
    let _ = tokio::fs::remove_dir_all(&temp_dir).await;

    // Emit completion event
    let complete = DownloadComplete {
        id: download_id.clone(),
        path: file_path.to_string_lossy().to_string(),
        filename: file_path.file_name().unwrap().to_string_lossy().to_string(),
        total_size,
    };
    let _ = app.emit("download-complete", &complete);

    Ok(file_path.to_string_lossy().to_string())
}

async fn download_chunk(
    client: reqwest::Client,
    url: String,
    temp_dir: PathBuf,
    chunk_id: u64,
    start: u64,
    end: u64,
    downloaded: Arc<AtomicU64>,
    handle: Arc<DownloadHandle>,
) -> Result<PathBuf, String> {
    let chunk_path = temp_dir.join(format!("chunk_{}", chunk_id));

    let response = client
        .get(&url)
        .header("Range", format!("bytes={}-{}", start, end))
        .send()
        .await
        .map_err(|e| format!("Chunk {} request failed: {}", chunk_id, e))?;

    if !response.status().is_success() && response.status() != reqwest::StatusCode::PARTIAL_CONTENT {
        return Err(format!("Chunk {} HTTP error: {}", chunk_id, response.status()));
    }

    let mut file = File::create(&chunk_path)
        .await
        .map_err(|e| format!("Failed to create chunk file: {}", e))?;

    let mut stream = response.bytes_stream();

    while let Some(chunk_result) = stream.next().await {
        // Check for cancellation
        if handle.cancelled.load(Ordering::SeqCst) {
            return Err("Cancelled".to_string());
        }

        // Wait while paused
        while handle.paused.load(Ordering::SeqCst) {
            if handle.cancelled.load(Ordering::SeqCst) {
                return Err("Cancelled".to_string());
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        let bytes = chunk_result.map_err(|e| format!("Stream error: {}", e))?;
        file.write_all(&bytes)
            .await
            .map_err(|e| format!("Write error: {}", e))?;
        downloaded.fetch_add(bytes.len() as u64, Ordering::Relaxed);
    }

    file.flush().await.map_err(|e| format!("Flush error: {}", e))?;

    Ok(chunk_path)
}

async fn merge_chunks(chunk_paths: &[PathBuf], output_path: &PathBuf) -> Result<(), String> {
    let mut output = File::create(output_path)
        .await
        .map_err(|e| format!("Failed to create output file: {}", e))?;

    for path in chunk_paths {
        let data = tokio::fs::read(path)
            .await
            .map_err(|e| format!("Failed to read chunk: {}", e))?;
        output
            .write_all(&data)
            .await
            .map_err(|e| format!("Failed to write to output: {}", e))?;
    }

    output.flush().await.map_err(|e| format!("Flush error: {}", e))?;

    Ok(())
}

async fn download_single(
    app: AppHandle,
    handle: Arc<DownloadHandle>,
    url: String,
    file_path: PathBuf,
) -> Result<String, String> {
    let download_id = handle.id.clone();
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .map_err(|e| format!("Failed to create client: {}", e))?;

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()));
    }

    let total_size = response.content_length().unwrap_or(0);
    let mut downloaded: u64 = 0;

    let mut file = File::create(&file_path)
        .await
        .map_err(|e| format!("Failed to create file: {}", e))?;

    let mut stream = response.bytes_stream();
    let mut last_emit = std::time::Instant::now();
    let mut last_downloaded = 0u64;

    while let Some(chunk_result) = stream.next().await {
        // Check for cancellation
        if handle.cancelled.load(Ordering::SeqCst) {
            drop(file);
            let _ = tokio::fs::remove_file(&file_path).await;

            let _ = app.emit("download-progress", DownloadProgress {
                id: download_id,
                downloaded: 0,
                total: total_size,
                speed: 0.0,
                status: "cancelled".to_string(),
                chunk_progress: vec![],
            });

            return Err("Download cancelled".to_string());
        }

        // Wait while paused
        while handle.paused.load(Ordering::SeqCst) {
            if handle.cancelled.load(Ordering::SeqCst) {
                drop(file);
                let _ = tokio::fs::remove_file(&file_path).await;
                return Err("Download cancelled".to_string());
            }

            // Emit paused status
            let _ = app.emit("download-progress", DownloadProgress {
                id: download_id.clone(),
                downloaded,
                total: total_size,
                speed: 0.0,
                status: "paused".to_string(),
                chunk_progress: vec![ChunkProgress {
                    id: 0,
                    downloaded,
                    total: total_size,
                }],
            });

            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        let bytes = chunk_result.map_err(|e| format!("Stream error: {}", e))?;
        file.write_all(&bytes)
            .await
            .map_err(|e| format!("Write error: {}", e))?;
        downloaded += bytes.len() as u64;

        // Emit progress every 100ms
        if last_emit.elapsed().as_millis() >= 100 {
            let speed = (downloaded - last_downloaded) as f64 * 10.0;
            last_downloaded = downloaded;

            let progress = DownloadProgress {
                id: download_id.clone(),
                downloaded,
                total: total_size,
                speed,
                status: "downloading".to_string(),
                chunk_progress: vec![ChunkProgress {
                    id: 0,
                    downloaded,
                    total: total_size,
                }],
            };
            let _ = app.emit("download-progress", &progress);
            last_emit = std::time::Instant::now();
        }
    }

    file.flush().await.map_err(|e| format!("Flush error: {}", e))?;

    // Emit completion
    let complete = DownloadComplete {
        id: download_id,
        path: file_path.to_string_lossy().to_string(),
        filename: file_path.file_name().unwrap().to_string_lossy().to_string(),
        total_size,
    };
    let _ = app.emit("download-complete", &complete);

    Ok(file_path.to_string_lossy().to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            downloads: RwLock::new(HashMap::new()),
            settings: RwLock::new(Settings::default()),
        })
        .invoke_handler(tauri::generate_handler![
            fetch_url_info,
            check_file_exists,
            start_download,
            cancel_download,
            pause_download,
            resume_download,
            set_connections,
            get_connections
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
