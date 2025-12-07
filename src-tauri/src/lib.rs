use futures::stream::StreamExt;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

const NUM_CONNECTIONS: u64 = 8;

#[derive(Serialize)]
pub struct UrlInfo {
    pub url: String,
    pub filename: String,
    pub size: Option<u64>,
    pub resumable: bool,
}

#[derive(Clone, Serialize)]
pub struct DownloadProgress {
    pub downloaded: u64,
    pub total: u64,
    pub speed: f64,         // bytes per second
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
    pub path: String,
    pub filename: String,
    pub total_size: u64,
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

    // Get the final URL after redirects
    let final_url = response.url().to_string();
    let headers = response.headers();

    // Get file size from Content-Length header
    let size = headers
        .get(reqwest::header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse().ok());

    // Check if server supports resume (Accept-Ranges: bytes)
    let resumable = headers
        .get(reqwest::header::ACCEPT_RANGES)
        .and_then(|v| v.to_str().ok())
        .map(|v| v == "bytes")
        .unwrap_or(false);

    // Try to get filename from Content-Disposition header first
    let filename = headers
        .get(reqwest::header::CONTENT_DISPOSITION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| {
            v.split("filename=").nth(1).map(|s| s.trim_matches('"').to_string())
        })
        .or_else(|| {
            // Try to extract from final URL path (handles redirects)
            extract_filename_from_url(&final_url)
        })
        .or_else(|| {
            // Fallback to original URL
            extract_filename_from_url(&url)
        })
        .unwrap_or_else(|| "download".to_string());

    Ok(UrlInfo {
        url: final_url,
        filename,
        size,
        resumable,
    })
}

fn extract_filename_from_url(url: &str) -> Option<String> {
    // Parse URL and get path, ignoring query params
    url.split('?').next()
        .and_then(|path| path.split('/').last())
        .filter(|s| !s.is_empty() && s.contains('.'))
        .map(|s| s.to_string())
}

#[tauri::command]
async fn start_download(
    app: AppHandle,
    url: String,
    filename: String,
    size: u64,
    resumable: bool,
) -> Result<String, String> {
    // Get downloads directory
    let download_dir = dirs::download_dir()
        .ok_or_else(|| "Could not find downloads directory".to_string())?;

    let file_path = download_dir.join(&filename);

    if resumable && size > 0 {
        // Multi-connection chunked download
        download_chunked(app, url, file_path, size).await
    } else {
        // Single connection fallback
        download_single(app, url, file_path).await
    }
}

async fn download_chunked(
    app: AppHandle,
    url: String,
    file_path: PathBuf,
    total_size: u64,
) -> Result<String, String> {
    let num_chunks = NUM_CONNECTIONS;
    let chunk_size = total_size / num_chunks;

    // Calculate chunk ranges
    let mut chunks: Vec<(u64, u64, u64)> = Vec::new(); // (id, start, end)
    for i in 0..num_chunks {
        let start = i * chunk_size;
        let end = if i == num_chunks - 1 {
            total_size - 1 // Last chunk gets the remainder
        } else {
            (i + 1) * chunk_size - 1
        };
        chunks.push((i, start, end));
    }

    // Shared progress tracking
    let chunk_downloaded: Vec<Arc<AtomicU64>> = (0..num_chunks)
        .map(|_| Arc::new(AtomicU64::new(0)))
        .collect();

    let client = reqwest::Client::new();

    // Create temp directory for chunks
    let temp_dir = file_path.parent().unwrap().join(".wdm_temp");
    tokio::fs::create_dir_all(&temp_dir)
        .await
        .map_err(|e| format!("Failed to create temp directory: {}", e))?;

    // Spawn progress reporter
    let app_clone = app.clone();
    let chunk_downloaded_clone: Vec<Arc<AtomicU64>> = chunk_downloaded.iter().map(Arc::clone).collect();
    let chunk_sizes: Vec<u64> = chunks.iter().map(|(_, start, end)| end - start + 1).collect();

    let progress_handle = tokio::spawn(async move {
        let mut last_total = 0u64;
        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            let chunk_progress: Vec<ChunkProgress> = chunk_downloaded_clone
                .iter()
                .enumerate()
                .map(|(i, downloaded)| ChunkProgress {
                    id: i as u64,
                    downloaded: downloaded.load(Ordering::Relaxed),
                    total: chunk_sizes[i],
                })
                .collect();

            let total_downloaded: u64 = chunk_progress.iter().map(|c| c.downloaded).sum();
            let speed = (total_downloaded - last_total) as f64 * 10.0; // bytes/sec (100ms interval)
            last_total = total_downloaded;

            let progress = DownloadProgress {
                downloaded: total_downloaded,
                total: total_size,
                speed,
                chunk_progress,
            };

            let _ = app_clone.emit("download-progress", &progress);

            if total_downloaded >= total_size {
                break;
            }
        }
    });

    // Download all chunks in parallel
    let mut handles = Vec::new();

    for (chunk_id, start, end) in chunks {
        let client = client.clone();
        let url = url.clone();
        let temp_dir = temp_dir.clone();
        let downloaded = Arc::clone(&chunk_downloaded[chunk_id as usize]);

        let handle = tokio::spawn(async move {
            download_chunk(client, url, temp_dir, chunk_id, start, end, downloaded).await
        });
        handles.push(handle);
    }

    // Wait for all chunks to complete
    let results: Vec<Result<PathBuf, String>> = futures::future::join_all(handles)
        .await
        .into_iter()
        .map(|r| r.map_err(|e| format!("Task failed: {}", e))?)
        .collect();

    // Check for errors
    let chunk_paths: Vec<PathBuf> = results
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;

    // Stop progress reporter
    progress_handle.abort();

    // Merge chunks into final file
    merge_chunks(&chunk_paths, &file_path).await?;

    // Cleanup temp files
    for path in &chunk_paths {
        let _ = tokio::fs::remove_file(path).await;
    }
    let _ = tokio::fs::remove_dir(&temp_dir).await;

    // Emit completion event
    let complete = DownloadComplete {
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
    url: String,
    file_path: PathBuf,
) -> Result<String, String> {
    let client = reqwest::Client::new();

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

    while let Some(chunk_result) = stream.next().await {
        let bytes = chunk_result.map_err(|e| format!("Stream error: {}", e))?;
        file.write_all(&bytes)
            .await
            .map_err(|e| format!("Write error: {}", e))?;
        downloaded += bytes.len() as u64;

        // Emit progress every 100ms
        if last_emit.elapsed().as_millis() >= 100 {
            let progress = DownloadProgress {
                downloaded,
                total: total_size,
                speed: 0.0, // Could calculate this
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
        .invoke_handler(tauri::generate_handler![fetch_url_info, start_download])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
