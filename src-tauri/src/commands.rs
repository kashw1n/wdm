use crate::downloader::{download_chunked, download_single};
use crate::persistence::{DownloadRecord, DownloadStatus};
use crate::state::{
    AppState, DownloadError, DownloadHandle, DownloadInfo, FileExistsInfo, UrlInfo,
};
use crate::utils::{extract_filename_from_url, generate_unique_filename};
use crate::video::{
    download_video, fetch_video_info, is_video_url, VideoDownloadHandle, VideoInfo,
};
use crate::ytdlp::{ensure_ytdlp, get_ytdlp_version, is_ytdlp_installed};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};
use std::process::Command;

#[tauri::command]
pub async fn fetch_url_info(url: String) -> Result<UrlInfo, String> {
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

#[tauri::command]
pub async fn check_file_exists(app: AppHandle, filename: String) -> Result<FileExistsInfo, String> {
    let state = app.state::<AppState>();
    let settings = state.settings.read().await;
    let download_dir = settings.get_download_folder();

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

#[tauri::command]
pub async fn get_download_history(app: AppHandle) -> Result<Vec<DownloadInfo>, String> {
    let state = app.state::<AppState>();
    let history = state.history.read().await;

    let downloads: Vec<DownloadInfo> = history
        .get_all_downloads()
        .into_iter()
        .map(|r| DownloadInfo {
            id: r.id.clone(),
            url: r.url.clone(),
            filename: r.filename.clone(),
            file_path: r.file_path.clone(),
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
pub async fn clear_download_history(app: AppHandle) -> Result<(), String> {
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
        if let Some(record) = history.downloads.get(&id) {
            let file_path = PathBuf::from(&record.file_path);
            if let Some(parent) = file_path.parent() {
                let temp_dir = parent.join(format!(".wdm_temp_{}", id));
                let _ = tokio::fs::remove_dir_all(temp_dir).await;
            }
            
            let part_path = file_path.with_extension(format!(
                "{}.part",
                file_path.extension().and_then(|e| e.to_str()).unwrap_or("")
            ));
            let _ = tokio::fs::remove_file(part_path).await;
        }
        history.remove_download(&id);
    }

    history.save().await?;
    Ok(())
}

#[tauri::command]
pub async fn remove_from_history(app: AppHandle, id: String) -> Result<(), String> {
    let state = app.state::<AppState>();
    let mut history = state.history.write().await;
    
    if let Some(record) = history.downloads.get(&id) {
        let file_path = PathBuf::from(&record.file_path);
        if let Some(parent) = file_path.parent() {
            let temp_dir = parent.join(format!(".wdm_temp_{}", id));
            let _ = tokio::fs::remove_dir_all(temp_dir).await;
        }

        let part_path = file_path.with_extension(format!(
            "{}.part",
            file_path.extension().and_then(|e| e.to_str()).unwrap_or("")
        ));
        let _ = tokio::fs::remove_file(part_path).await;
    }

    history.remove_download(&id);
    history.save().await?;
    Ok(())
}

#[tauri::command]
pub async fn start_download(
    app: AppHandle,
    url: String,
    filename: String,
    size: u64,
    resumable: bool,
) -> Result<String, String> {
    let state = app.state::<AppState>();
    let settings = state.settings.read().await;
    let num_connections = settings.connections;
    let download_dir = settings.get_download_folder();
    drop(settings);

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
        false, // is_video
        None,  // thumbnail
    );

    {
        let mut history = state.history.write().await;
        history.add_download(record);
        history.save().await?;
    }

    // Create download handle
    let speed_limit = {
        let settings = state.settings.read().await;
        settings.speed_limit
    };
    let handle = Arc::new(DownloadHandle {
        id: download_id.clone(),
        cancelled: AtomicBool::new(false),
        paused: AtomicBool::new(false),
        chunk_downloaded: (0..num_connections)
            .map(|_| Arc::new(AtomicU64::new(0)))
            .collect(),
        speed_limit: AtomicU64::new(speed_limit),
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
pub async fn resume_interrupted_download(app: AppHandle, id: String) -> Result<(), String> {
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

    let speed_limit = {
        let settings = state.settings.read().await;
        settings.speed_limit
    };
    let handle = Arc::new(DownloadHandle {
        id: id.clone(),
        cancelled: AtomicBool::new(false),
        paused: AtomicBool::new(false),
        chunk_downloaded,
        speed_limit: AtomicU64::new(speed_limit),
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
pub async fn cancel_download(app: AppHandle, id: String) -> Result<(), String> {
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
pub async fn pause_download(app: AppHandle, id: String) -> Result<(), String> {
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
pub async fn resume_download(app: AppHandle, id: String) -> Result<(), String> {
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
pub async fn set_connections(app: AppHandle, connections: u64) -> Result<(), String> {
    if connections < 1 || connections > 32 {
        return Err("Connections must be between 1 and 32".to_string());
    }

    let state = app.state::<AppState>();
    let mut settings = state.settings.write().await;
    settings.connections = connections;
    settings.save().await?;
    Ok(())
}

#[tauri::command]
pub async fn get_connections(app: AppHandle) -> Result<u64, String> {
    let state = app.state::<AppState>();
    let settings = state.settings.read().await;
    Ok(settings.connections)
}

#[tauri::command]
pub async fn get_download_folder(app: AppHandle) -> Result<String, String> {
    let state = app.state::<AppState>();
    let settings = state.settings.read().await;
    Ok(settings.get_download_folder().to_string_lossy().to_string())
}

#[tauri::command]
pub async fn set_download_folder(app: AppHandle, folder: String) -> Result<(), String> {
    let path = PathBuf::from(&folder);
    if !path.exists() {
        return Err("Folder does not exist".to_string());
    }
    if !path.is_dir() {
        return Err("Path is not a directory".to_string());
    }

    let state = app.state::<AppState>();
    let mut settings = state.settings.write().await;
    settings.download_folder = Some(folder);
    settings.save().await?;
    Ok(())
}

#[tauri::command]
pub async fn reset_download_folder(app: AppHandle) -> Result<String, String> {
    let state = app.state::<AppState>();
    let mut settings = state.settings.write().await;
    settings.download_folder = None;
    settings.save().await?;
    Ok(settings.get_download_folder().to_string_lossy().to_string())
}

#[tauri::command]
pub async fn get_speed_limit(app: AppHandle) -> Result<u64, String> {
    let state = app.state::<AppState>();
    let settings = state.settings.read().await;
    Ok(settings.speed_limit)
}

#[tauri::command]
pub async fn set_speed_limit(app: AppHandle, limit: u64) -> Result<(), String> {
    let state = app.state::<AppState>();

    // Update settings
    {
        let mut settings = state.settings.write().await;
        settings.speed_limit = limit;
        settings.save().await?;
    }

    // Update all active downloads
    let downloads = state.downloads.read().await;
    for handle in downloads.values() {
        handle.speed_limit.store(limit, Ordering::Relaxed);
    }

    Ok(())
}

#[tauri::command]
pub async fn open_file(path: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .arg(path)
            .spawn()
            .map_err(|e| format!("Failed to open file: {}", e))?;
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(path)
            .spawn()
            .map_err(|e| format!("Failed to open file: {}", e))?;
    }
    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|e| format!("Failed to open file: {}", e))?;
    }
    Ok(())
}

#[tauri::command]
pub async fn show_in_folder(path: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .args(["/select,", &path])
            .spawn()
            .map_err(|e| format!("Failed to show in folder: {}", e))?;
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .args(["-R", &path])
            .spawn()
            .map_err(|e| format!("Failed to show in folder: {}", e))?;
    }
    #[cfg(target_os = "linux")]
    {
        // Linux file managers vary; try dbus or fallback to opening parent dir
        let path_buf = PathBuf::from(&path);
        let parent = path_buf.parent().unwrap_or(std::path::Path::new("/"));
        Command::new("xdg-open")
            .arg(parent)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }
    Ok(())
}

// ============================================================================
// Video Download Commands
// ============================================================================

/// Check if a URL is a video site URL
#[tauri::command]
pub fn check_video_url(url: String) -> bool {
    is_video_url(&url)
}

/// Check if yt-dlp is installed
#[tauri::command]
pub fn check_ytdlp_installed() -> bool {
    is_ytdlp_installed()
}

/// Ensure yt-dlp is installed (download if needed)
#[tauri::command]
pub async fn install_ytdlp(app: AppHandle) -> Result<String, String> {
    let progress_callback = {
        let app = app.clone();
        move |downloaded: u64, total: u64| {
            let _ = app.emit(
                "ytdlp-download-progress",
                serde_json::json!({
                    "downloaded": downloaded,
                    "total": total
                }),
            );
        }
    };

    let path = ensure_ytdlp(progress_callback).await?;
    Ok(path.to_string_lossy().to_string())
}

/// Get yt-dlp version
#[tauri::command]
pub async fn get_ytdlp_ver() -> Result<String, String> {
    get_ytdlp_version().await
}

/// Fetch video information from URL
#[tauri::command]
pub async fn get_video_info(url: String) -> Result<VideoInfo, String> {
    fetch_video_info(&url).await
}

/// Start a video download
#[tauri::command]
pub async fn start_video_download(
    app: AppHandle,
    url: String,
    format_id: String,
) -> Result<String, String> {
    let state = app.state::<AppState>();
    let settings = state.settings.read().await;
    let download_dir = settings.get_download_folder();
    let connections = settings.connections;
    let speed_limit = settings.speed_limit;
    drop(settings);

    let download_id = format!("video_{}", chrono::Utc::now().timestamp_millis());
    let download_dir_str = download_dir.to_string_lossy().to_string();

    // Create download handle
    let handle = Arc::new(VideoDownloadHandle::new(download_id.clone()));

    // Create history record for video
    // Note: We might want to fetch title first or update it later. 
    // For now use ID or generic name. The frontend has the title, we could pass it or update it later.
    // Ideally the `get_video_info` result should be cached or passed back. 
    // Since we don't have it here easily without re-fetching or changing API, 
    // let's assume we update it during download progress if possible or just use a placeholder.
    // However, better UX: use "Video Download" and update later.
    let record = crate::persistence::DownloadRecord::new(
        download_id.clone(),
        url.clone(),
        "Video Download".to_string(), // Placeholder, should be updated when download starts
        format!("{}/%(title)s.%(ext)s", download_dir_str), // Template path
        0, // Unknown size initially
        false, // Not resumable in same way
        1, // 1 'connection' logic for our tracking
        true, // is_video
        None, // thumbnail - could pass if we change API
    );

    {
        let mut history = state.history.write().await;
        history.add_download(record);
        history.save().await?;
    }

    {
        let mut video_downloads = state.video_downloads.write().await;
        video_downloads.insert(download_id.clone(), Arc::clone(&handle));
    }

    let app_clone = app.clone();
    let download_id_clone = download_id.clone();

    // Use connections setting for concurrent fragments (capped at 16 for yt-dlp)
    let concurrent_fragments = (connections as u32).min(16);

    tokio::spawn(async move {
        let result = download_video(
            app_clone.clone(),
            download_id_clone.clone(),
            url,
            format_id,
            download_dir.to_string_lossy().to_string(),
            handle,
            concurrent_fragments,
            speed_limit,
        )
        .await;

        let state = app_clone.state::<AppState>();
        let mut video_downloads = state.video_downloads.write().await;
        video_downloads.remove(&download_id_clone);

        if let Err(e) = result {
            if !e.contains("cancelled") {
                let _ = app_clone.emit(
                    "video-error",
                    serde_json::json!({
                        "id": download_id_clone,
                        "error": e
                    }),
                );
            }
        }
    });

    Ok(download_id)
}

/// Cancel a video download
#[tauri::command]
pub async fn cancel_video_download(app: AppHandle, id: String) -> Result<(), String> {
    let state = app.state::<AppState>();
    let video_downloads = state.video_downloads.read().await;

    if let Some(handle) = video_downloads.get(&id) {
        handle.cancelled.store(true, Ordering::SeqCst);

        // Try to kill the process
        let mut process = handle.process.lock().await;
        if let Some(child) = process.as_mut() {
            let _ = child.kill().await;
        }

        Ok(())
    } else {
        Err("Video download not found".to_string())
    }
}