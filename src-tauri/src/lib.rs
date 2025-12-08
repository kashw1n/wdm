mod persistence;

use futures::stream::StreamExt;
use persistence::{DownloadHistory, DownloadRecord, DownloadStatus};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};
use tokio::fs::File;
use tokio::io::{AsyncSeekExt, AsyncWriteExt};
use tokio::sync::RwLock;

const DEFAULT_CONNECTIONS: u64 = 8;

pub struct AppState {
    downloads: RwLock<HashMap<String, Arc<DownloadHandle>>>,
    settings: RwLock<Settings>,
    history: RwLock<DownloadHistory>,
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
    pub status: String,
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

#[derive(Clone, Serialize)]
pub struct FileExistsInfo {
    pub exists: bool,
    pub suggested_name: String,
}

// Frontend-friendly download record
#[derive(Clone, Serialize)]
pub struct DownloadInfo {
    pub id: String,
    pub url: String,
    pub filename: String,
    pub total_size: u64,
    pub downloaded: u64,
    pub status: String,
    pub resumable: bool,
    pub created_at: i64,
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

#[tauri::command]
async fn check_file_exists(filename: String) -> Result<FileExistsInfo, String> {
    let download_dir = dirs::download_dir()
        .ok_or_else(|| "Could not find downloads directory".to_string())?;

    let file_path = download_dir.join(&filename);

    if file_path.exists() {
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
async fn get_download_history(app: AppHandle) -> Result<Vec<DownloadInfo>, String> {
    let state = app.state::<AppState>();
    let history = state.history.read().await;

    let downloads: Vec<DownloadInfo> = history
        .get_all_downloads()
        .into_iter()
        .map(|r| DownloadInfo {
            id: r.id.clone(),
            url: r.url.clone(),
            filename: r.filename.clone(),
            total_size: r.total_size,
            downloaded: r.total_downloaded(),
            status: format!("{:?}", r.status),
            resumable: r.resumable,
            created_at: r.created_at,
        })
        .collect();

    Ok(downloads)
}

#[tauri::command]
async fn clear_download_history(app: AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let mut history = state.history.write().await;

    // Only remove completed, failed, and cancelled downloads
    let ids_to_remove: Vec<String> = history
        .downloads
        .iter()
        .filter(|(_, r)| {
            r.status == DownloadStatus::Completed
                || r.status == DownloadStatus::Failed
                || r.status == DownloadStatus::Cancelled
        })
        .map(|(id, _)| id.clone())
        .collect();

    for id in ids_to_remove {
        history.remove_download(&id);
    }

    history.save().await?;
    Ok(())
}

#[tauri::command]
async fn remove_from_history(app: AppHandle, id: String) -> Result<(), String> {
    let state = app.state::<AppState>();
    let mut history = state.history.write().await;
    history.remove_download(&id);
    history.save().await?;
    Ok(())
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
    let download_id = format!("{}_{}", filename, chrono::Utc::now().timestamp_millis());

    // Create and save download record
    let record = DownloadRecord::new(
        download_id.clone(),
        url.clone(),
        filename.clone(),
        file_path.to_string_lossy().to_string(),
        size,
        resumable,
        num_connections,
    );

    {
        let mut history = state.history.write().await;
        history.add_download(record);
        history.save().await?;
    }

    // Create download handle
    let handle = Arc::new(DownloadHandle {
        id: download_id.clone(),
        cancelled: AtomicBool::new(false),
        paused: AtomicBool::new(false),
        chunk_downloaded: (0..num_connections)
            .map(|_| Arc::new(AtomicU64::new(0)))
            .collect(),
    });

    {
        let mut downloads = state.downloads.write().await;
        downloads.insert(download_id.clone(), Arc::clone(&handle));
    }

    let app_clone = app.clone();
    let download_id_clone = download_id.clone();

    tokio::spawn(async move {
        // Update status to downloading
        {
            let state = app_clone.state::<AppState>();
            let mut history = state.history.write().await;
            history.update_download(&download_id_clone, |r| {
                r.status = DownloadStatus::Downloading;
            });
            let _ = history.save().await;
        }

        let result = if resumable && size > 0 {
            download_chunked(app_clone.clone(), handle, url, file_path, size, num_connections, None).await
        } else {
            download_single(app_clone.clone(), handle, url, file_path).await
        };

        let state = app_clone.state::<AppState>();
        let mut downloads = state.downloads.write().await;
        downloads.remove(&download_id_clone);
        drop(downloads);

        // Update history based on result
        let mut history = state.history.write().await;
        match &result {
            Ok(_) => {
                history.update_download(&download_id_clone, |r| {
                    r.status = DownloadStatus::Completed;
                });
            }
            Err(e) if e.contains("cancelled") => {
                history.update_download(&download_id_clone, |r| {
                    r.status = DownloadStatus::Cancelled;
                });
            }
            Err(_) => {
                history.update_download(&download_id_clone, |r| {
                    r.status = DownloadStatus::Failed;
                });
            }
        }
        let _ = history.save().await;

        if let Err(e) = result {
            if !e.contains("cancelled") {
                let _ = app_clone.emit("download-error", DownloadError {
                    id: download_id_clone,
                    error: e,
                });
            }
        }
    });

    Ok(download_id)
}

#[tauri::command]
async fn resume_interrupted_download(app: AppHandle, id: String) -> Result<(), String> {
    let state = app.state::<AppState>();

    // Get the download record
    let record = {
        let history = state.history.read().await;
        history.get_download(&id).cloned()
    };

    let record = record.ok_or("Download not found in history")?;

    if record.status != DownloadStatus::Paused
        && record.status != DownloadStatus::Failed
        && record.status != DownloadStatus::Downloading
    {
        return Err("Download cannot be resumed".to_string());
    }

    if !record.resumable {
        return Err("This download does not support resuming".to_string());
    }

    let num_connections = record.num_connections;
    let file_path = PathBuf::from(&record.file_path);

    // Create download handle with existing progress
    let chunk_downloaded: Vec<Arc<AtomicU64>> = record
        .chunks
        .iter()
        .map(|c| Arc::new(AtomicU64::new(c.downloaded)))
        .collect();

    let handle = Arc::new(DownloadHandle {
        id: id.clone(),
        cancelled: AtomicBool::new(false),
        paused: AtomicBool::new(false),
        chunk_downloaded,
    });

    {
        let mut downloads = state.downloads.write().await;
        downloads.insert(id.clone(), Arc::clone(&handle));
    }

    let app_clone = app.clone();
    let id_clone = id.clone();
    let url = record.url.clone();
    let total_size = record.total_size;
    let chunks = record.chunks.clone();

    tokio::spawn(async move {
        {
            let state = app_clone.state::<AppState>();
            let mut history = state.history.write().await;
            history.update_download(&id_clone, |r| {
                r.status = DownloadStatus::Downloading;
            });
            let _ = history.save().await;
        }

        let result = download_chunked(
            app_clone.clone(),
            handle,
            url,
            file_path,
            total_size,
            num_connections,
            Some(chunks),
        )
        .await;

        let state = app_clone.state::<AppState>();
        let mut downloads = state.downloads.write().await;
        downloads.remove(&id_clone);
        drop(downloads);

        let mut history = state.history.write().await;
        match &result {
            Ok(_) => {
                history.update_download(&id_clone, |r| {
                    r.status = DownloadStatus::Completed;
                });
            }
            Err(e) if e.contains("cancelled") => {
                history.update_download(&id_clone, |r| {
                    r.status = DownloadStatus::Cancelled;
                });
            }
            Err(_) => {
                history.update_download(&id_clone, |r| {
                    r.status = DownloadStatus::Failed;
                });
            }
        }
        let _ = history.save().await;

        if let Err(e) = result {
            if !e.contains("cancelled") {
                let _ = app_clone.emit("download-error", DownloadError {
                    id: id_clone,
                    error: e,
                });
            }
        }
    });

    Ok(())
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

    {
        let downloads = state.downloads.read().await;
        if let Some(handle) = downloads.get(&id) {
            handle.paused.store(true, Ordering::SeqCst);
        } else {
            return Err("Download not found".to_string());
        }
    }

    // Save paused state to history
    let mut history = state.history.write().await;
    history.update_download(&id, |r| {
        r.status = DownloadStatus::Paused;
    });
    history.save().await?;

    Ok(())
}

#[tauri::command]
async fn resume_download(app: AppHandle, id: String) -> Result<(), String> {
    let state = app.state::<AppState>();

    {
        let downloads = state.downloads.read().await;
        if let Some(handle) = downloads.get(&id) {
            handle.paused.store(false, Ordering::SeqCst);
        } else {
            return Err("Download not found".to_string());
        }
    }

    let mut history = state.history.write().await;
    history.update_download(&id, |r| {
        r.status = DownloadStatus::Downloading;
    });
    history.save().await?;

    Ok(())
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
    existing_chunks: Option<Vec<persistence::ChunkRecord>>,
) -> Result<String, String> {
    let chunk_size = total_size / num_connections;
    let download_id = handle.id.clone();

    // Calculate chunk ranges (use existing or create new)
    let chunks: Vec<(u64, u64, u64, u64)> = if let Some(existing) = existing_chunks {
        existing
            .into_iter()
            .map(|c| (c.id, c.start, c.end, c.downloaded))
            .collect()
    } else {
        (0..num_connections)
            .map(|i| {
                let start = i * chunk_size;
                let end = if i == num_connections - 1 {
                    total_size - 1
                } else {
                    (i + 1) * chunk_size - 1
                };
                (i, start, end, 0u64)
            })
            .collect()
    };

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .map_err(|e| format!("Failed to create client: {}", e))?;

    let temp_dir = file_path.parent().unwrap().join(format!(".wdm_temp_{}", download_id));
    tokio::fs::create_dir_all(&temp_dir)
        .await
        .map_err(|e| format!("Failed to create temp directory: {}", e))?;

    // Progress reporter
    let app_clone = app.clone();
    let handle_clone = Arc::clone(&handle);
    let chunk_sizes: Vec<u64> = chunks.iter().map(|(_, start, end, _)| end - start + 1).collect();
    let download_id_clone = download_id.clone();
    let app_for_save = app.clone();
    let id_for_save = download_id.clone();

    let progress_handle = tokio::spawn(async move {
        let mut last_total = 0u64;
        let mut save_counter = 0u32;

        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            if handle_clone.cancelled.load(Ordering::SeqCst) {
                break;
            }

            let is_paused = handle_clone.paused.load(Ordering::SeqCst);

            let chunk_progress: Vec<ChunkProgress> = handle_clone
                .chunk_downloaded
                .iter()
                .enumerate()
                .map(|(i, downloaded)| ChunkProgress {
                    id: i as u64,
                    downloaded: downloaded.load(Ordering::Relaxed),
                    total: chunk_sizes[i],
                })
                .collect();

            let total_downloaded: u64 = chunk_progress.iter().map(|c| c.downloaded).sum();
            let speed = if is_paused {
                0.0
            } else {
                (total_downloaded.saturating_sub(last_total)) as f64 * 10.0
            };
            last_total = total_downloaded;

            let status = if is_paused { "paused" } else { "downloading" };

            let progress = DownloadProgress {
                id: download_id_clone.clone(),
                downloaded: total_downloaded,
                total: total_size,
                speed,
                status: status.to_string(),
                chunk_progress: chunk_progress.clone(),
            };

            let _ = app_clone.emit("download-progress", &progress);

            // Save progress to history every second (10 iterations)
            save_counter += 1;
            if save_counter >= 10 {
                save_counter = 0;
                let state = app_for_save.state::<AppState>();
                let mut history = state.history.write().await;
                for cp in &chunk_progress {
                    history.update_chunk_progress(&id_for_save, cp.id, cp.downloaded);
                }
                let _ = history.save().await;
            }

            if total_downloaded >= total_size {
                break;
            }
        }
    });

    // Download chunks in parallel
    let mut handles_vec = Vec::new();

    for (chunk_id, start, end, already_downloaded) in chunks {
        // Skip completed chunks
        let chunk_total = end - start + 1;
        if already_downloaded >= chunk_total {
            continue;
        }

        let client = client.clone();
        let url = url.clone();
        let temp_dir = temp_dir.clone();
        let downloaded = Arc::clone(&handle.chunk_downloaded[chunk_id as usize]);
        let handle_clone = Arc::clone(&handle);

        // Set initial progress for resumed chunks
        downloaded.store(already_downloaded, Ordering::Relaxed);

        let task = tokio::spawn(async move {
            download_chunk(
                client,
                url,
                temp_dir,
                chunk_id,
                start,
                end,
                already_downloaded,
                downloaded,
                handle_clone,
            )
            .await
        });
        handles_vec.push((chunk_id, task));
    }

    // Wait for all chunks
    let mut chunk_paths: Vec<(u64, PathBuf)> = Vec::new();

    for (chunk_id, task) in handles_vec {
        match task.await {
            Ok(Ok(path)) => chunk_paths.push((chunk_id, path)),
            Ok(Err(e)) => {
                if !handle.cancelled.load(Ordering::SeqCst) {
                    progress_handle.abort();
                    return Err(e);
                }
            }
            Err(e) => {
                progress_handle.abort();
                return Err(format!("Task failed: {}", e));
            }
        }
    }

    progress_handle.abort();

    if handle.cancelled.load(Ordering::SeqCst) {
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

    // Sort chunk paths by ID and merge
    chunk_paths.sort_by_key(|(id, _)| *id);

    // Add any pre-existing chunk files
    for i in 0..num_connections {
        let chunk_path = temp_dir.join(format!("chunk_{}", i));
        if chunk_path.exists() && !chunk_paths.iter().any(|(id, _)| *id == i) {
            chunk_paths.push((i, chunk_path));
        }
    }
    chunk_paths.sort_by_key(|(id, _)| *id);

    let paths: Vec<PathBuf> = chunk_paths.into_iter().map(|(_, p)| p).collect();
    merge_chunks(&paths, &file_path).await?;

    let _ = tokio::fs::remove_dir_all(&temp_dir).await;

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
    already_downloaded: u64,
    downloaded: Arc<AtomicU64>,
    handle: Arc<DownloadHandle>,
) -> Result<PathBuf, String> {
    let chunk_path = temp_dir.join(format!("chunk_{}", chunk_id));
    let actual_start = start + already_downloaded;

    if actual_start > end {
        return Ok(chunk_path);
    }

    let response = client
        .get(&url)
        .header("Range", format!("bytes={}-{}", actual_start, end))
        .send()
        .await
        .map_err(|e| format!("Chunk {} request failed: {}", chunk_id, e))?;

    if !response.status().is_success() && response.status() != reqwest::StatusCode::PARTIAL_CONTENT {
        return Err(format!("Chunk {} HTTP error: {}", chunk_id, response.status()));
    }

    let mut file = tokio::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open(&chunk_path)
        .await
        .map_err(|e| format!("Failed to open chunk file: {}", e))?;

    // Seek to position if resuming
    if already_downloaded > 0 {
        file.seek(std::io::SeekFrom::Start(already_downloaded))
            .await
            .map_err(|e| format!("Failed to seek: {}", e))?;
    }

    let mut stream = response.bytes_stream();

    while let Some(chunk_result) = stream.next().await {
        if handle.cancelled.load(Ordering::SeqCst) {
            return Err("Cancelled".to_string());
        }

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

        while handle.paused.load(Ordering::SeqCst) {
            if handle.cancelled.load(Ordering::SeqCst) {
                drop(file);
                let _ = tokio::fs::remove_file(&file_path).await;
                return Err("Download cancelled".to_string());
            }
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
        .setup(|app| {
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let history = DownloadHistory::load().await;
                let state = handle.state::<AppState>();
                *state.history.write().await = history;
            });
            Ok(())
        })
        .manage(AppState {
            downloads: RwLock::new(HashMap::new()),
            settings: RwLock::new(Settings::default()),
            history: RwLock::new(DownloadHistory::default()),
        })
        .invoke_handler(tauri::generate_handler![
            fetch_url_info,
            check_file_exists,
            start_download,
            resume_interrupted_download,
            cancel_download,
            pause_download,
            resume_download,
            set_connections,
            get_connections,
            get_download_history,
            clear_download_history,
            remove_from_history
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
