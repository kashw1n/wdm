use crate::persistence::ChunkRecord;
use crate::state::{AppState, ChunkProgress, DownloadComplete, DownloadHandle, DownloadProgress};
use futures::stream::StreamExt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};
use tokio::fs::File;
use tokio::io::{AsyncSeekExt, AsyncWriteExt};

pub async fn download_chunked(
    app: AppHandle,
    handle: Arc<DownloadHandle>,
    url: String,
    file_path: PathBuf,
    total_size: u64,
    num_connections: u64,
    existing_chunks: Option<Vec<ChunkRecord>>,
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
    let num_chunks = handle.chunk_downloaded.len() as u64;
    let mut throttle_start = std::time::Instant::now();
    let mut throttle_bytes = 0u64;

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

        // Speed limiting: each chunk gets (total_limit / num_chunks) bandwidth
        let speed_limit = handle.speed_limit.load(Ordering::Relaxed);
        if speed_limit > 0 {
            throttle_bytes += bytes.len() as u64;
            let chunk_limit = speed_limit / num_chunks;
            let elapsed = throttle_start.elapsed().as_secs_f64();
            let expected_time = throttle_bytes as f64 / chunk_limit as f64;

            if expected_time > elapsed {
                let delay_ms = ((expected_time - elapsed) * 1000.0) as u64;
                if delay_ms > 5 {
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                }
            }

            // Reset every second
            if throttle_start.elapsed().as_secs() >= 1 {
                throttle_start = std::time::Instant::now();
                throttle_bytes = 0;
            }
        }
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

pub async fn download_single(
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
