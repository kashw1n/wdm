use crate::ytdlp::get_ytdlp_path;
use crate::state::AppState;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

/// Video site URL patterns
const VIDEO_PATTERNS: &[&str] = &[
    r"youtube\.com/watch",
    r"youtube\.com/shorts/",
    r"youtu\.be/",
    r"twitter\.com/.*/status/",
    r"x\.com/.*/status/",
    r"tiktok\.com/",
    r"instagram\.com/(p|reel|reels)/",
    r"vimeo\.com/",
    r"twitch\.tv/",
    r"dailymotion\.com/",
    r"facebook\.com/.*/videos/",
    r"reddit\.com/.*/comments/",
    r"streamable\.com/",
    r"v\.redd\.it/",
];

/// Check if a URL is a video site URL
pub fn is_video_url(url: &str) -> bool {
    for pattern in VIDEO_PATTERNS {
        if let Ok(re) = Regex::new(pattern) {
            if re.is_match(url) {
                return true;
            }
        }
    }
    false
}

/// Video format information from yt-dlp
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoFormat {
    pub format_id: String,
    pub ext: String,
    pub resolution: Option<String>,
    pub filesize: Option<u64>,
    pub filesize_approx: Option<u64>,
    pub vcodec: Option<String>,
    pub acodec: Option<String>,
    pub fps: Option<f64>,
    pub tbr: Option<f64>,
    pub format_note: Option<String>,
}

/// Video information from yt-dlp
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoInfo {
    pub url: String,
    pub title: String,
    pub duration: Option<f64>,
    pub thumbnail: Option<String>,
    pub uploader: Option<String>,
    pub view_count: Option<u64>,
    pub formats: Vec<VideoFormat>,
    pub best_format: Option<String>,
}

/// Progress information for video download
#[derive(Debug, Clone, Serialize)]
pub struct VideoProgress {
    pub id: String,
    pub status: String,
    pub percent: f64,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub speed: f64,
    pub eta: Option<u64>,
    pub filename: String,
}

/// Handle for a video download process
pub struct VideoDownloadHandle {
    #[allow(dead_code)]
    pub id: String,
    pub cancelled: AtomicBool,
    pub process: Mutex<Option<Child>>,
}

impl VideoDownloadHandle {
    pub fn new(id: String) -> Self {
        Self {
            id,
            cancelled: AtomicBool::new(false),
            process: Mutex::new(None),
        }
    }
}

/// Fetch video information using yt-dlp
pub async fn fetch_video_info(url: &str) -> Result<VideoInfo, String> {
    let ytdlp_path = get_ytdlp_path();

    if !ytdlp_path.exists() {
        return Err("yt-dlp not installed".to_string());
    }

    let output = Command::new(&ytdlp_path)
        .args([
            "--dump-json",
            "--no-download",
            "--no-warnings",
            "--no-playlist",
            url,
        ])
        .output()
        .await
        .map_err(|e| format!("Failed to run yt-dlp: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("yt-dlp error: {}", stderr));
    }

    let json_str = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&json_str).map_err(|e| format!("Failed to parse JSON: {}", e))?;

    // Parse formats
    let formats: Vec<VideoFormat> = if let Some(formats_arr) = json.get("formats").and_then(|f| f.as_array()) {
        formats_arr
            .iter()
            .filter_map(|f| {
                let format_id = f.get("format_id")?.as_str()?.to_string();
                let ext = f.get("ext").and_then(|e| e.as_str()).unwrap_or("mp4").to_string();

                // Skip formats without video or audio
                let vcodec = f.get("vcodec").and_then(|v| v.as_str()).map(|s| s.to_string());
                let acodec = f.get("acodec").and_then(|a| a.as_str()).map(|s| s.to_string());

                Some(VideoFormat {
                    format_id,
                    ext,
                    resolution: f.get("resolution").and_then(|r| r.as_str()).map(|s| s.to_string()),
                    filesize: f.get("filesize").and_then(|s| s.as_u64()),
                    filesize_approx: f.get("filesize_approx").and_then(|s| s.as_u64()),
                    vcodec,
                    acodec,
                    fps: f.get("fps").and_then(|fps| fps.as_f64()),
                    tbr: f.get("tbr").and_then(|tbr| tbr.as_f64()),
                    format_note: f.get("format_note").and_then(|n| n.as_str()).map(|s| s.to_string()),
                })
            })
            .collect()
    } else {
        Vec::new()
    };

    // Filter and simplify formats for the UI
    let simplified_formats = simplify_formats(&formats);

    Ok(VideoInfo {
        url: url.to_string(),
        title: json
            .get("title")
            .and_then(|t| t.as_str())
            .unwrap_or("Unknown")
            .to_string(),
        duration: json.get("duration").and_then(|d| d.as_f64()),
        thumbnail: json
            .get("thumbnail")
            .and_then(|t| t.as_str())
            .map(|s| s.to_string()),
        uploader: json
            .get("uploader")
            .and_then(|u| u.as_str())
            .map(|s| s.to_string()),
        view_count: json.get("view_count").and_then(|v| v.as_u64()),
        formats: simplified_formats,
        best_format: Some("best".to_string()),
    })
}

/// Simplify formats for UI display
fn simplify_formats(formats: &[VideoFormat]) -> Vec<VideoFormat> {
    let mut seen_resolutions: HashMap<String, VideoFormat> = HashMap::new();

    for format in formats {
        // Skip audio-only or video-only formats for the main list
        let has_video = format.vcodec.as_ref().map(|v| v != "none").unwrap_or(false);
        let has_audio = format.acodec.as_ref().map(|a| a != "none").unwrap_or(false);

        if let Some(resolution) = &format.resolution {
            let key = if has_video && has_audio {
                resolution.clone()
            } else if has_video {
                format!("{} (video only)", resolution)
            } else if has_audio {
                "audio only".to_string()
            } else {
                continue;
            };

            // Keep the format with the largest filesize for each resolution
            if let Some(existing) = seen_resolutions.get(&key) {
                let existing_size = existing.filesize.or(existing.filesize_approx).unwrap_or(0);
                let new_size = format.filesize.or(format.filesize_approx).unwrap_or(0);
                if new_size > existing_size {
                    seen_resolutions.insert(key, format.clone());
                }
            } else {
                seen_resolutions.insert(key, format.clone());
            }
        }
    }

    let mut result: Vec<VideoFormat> = seen_resolutions.into_values().collect();

    // Sort by resolution (higher first)
    result.sort_by(|a, b| {
        let a_height = extract_height(&a.resolution);
        let b_height = extract_height(&b.resolution);
        b_height.cmp(&a_height)
    });

    // Add a "best" option at the top
    let best = VideoFormat {
        format_id: "best".to_string(),
        ext: "mp4".to_string(),
        resolution: Some("Best Quality".to_string()),
        filesize: None,
        filesize_approx: None,
        vcodec: Some("auto".to_string()),
        acodec: Some("auto".to_string()),
        fps: None,
        tbr: None,
        format_note: Some("Best video + audio".to_string()),
    };

    let mut final_result = vec![best];
    final_result.extend(result);
    final_result
}

/// Extract height from resolution string like "1920x1080"
fn extract_height(resolution: &Option<String>) -> u32 {
    resolution
        .as_ref()
        .and_then(|r| {
            r.split('x')
                .last()
                .and_then(|h| h.parse::<u32>().ok())
        })
        .unwrap_or(0)
}

/// Download video using yt-dlp
pub async fn download_video(
    app: AppHandle,
    id: String,
    url: String,
    format_id: String,
    output_dir: String,
    handle: Arc<VideoDownloadHandle>,
    concurrent_fragments: u32,
    speed_limit: u64,
) -> Result<String, String> {
    let ytdlp_path = get_ytdlp_path();

    if !ytdlp_path.exists() {
        return Err("yt-dlp not installed".to_string());
    }

    // Build output template
    let output_template = format!("{}/%(title)s.%(ext)s", output_dir);

    // Build yt-dlp command
    let mut cmd = Command::new(&ytdlp_path);

    let mut args = vec![
        "--newline".to_string(),
        "--progress".to_string(),
        "--progress-template".to_string(),
        "download:WDM:%(progress._percent_str)s|%(progress.downloaded_bytes)s|%(progress.total_bytes)s|%(progress.total_bytes_estimate)s|%(progress.speed)s|%(progress.eta)s".to_string(),
        "-f".to_string(),
        format_id,
        "-o".to_string(),
        output_template,
        "--no-playlist".to_string(),
    ];

    // Add concurrent fragment downloads (for HLS/DASH streams)
    if concurrent_fragments > 1 {
        args.push("--concurrent-fragments".to_string());
        args.push(concurrent_fragments.to_string());
    }

    // Add speed limit if set
    if speed_limit > 0 {
        args.push("--limit-rate".to_string());
        args.push(format!("{}K", speed_limit / 1024)); // Convert to KB/s
    }

    args.push(url);

    cmd.args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Failed to spawn yt-dlp: {}", e))?;

    let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;
    let stderr = child.stderr.take().ok_or("Failed to capture stderr")?;

    let reader = BufReader::new(stdout);
    let mut lines = reader.lines();

    let mut final_filename = String::new();

    // Emit initial progress
    app.emit("download-progress", serde_json::json!({
        "id": id,
        "downloaded": 0,
        "total": 0,
        "speed": 0.0,
        "status": "starting",
        "chunk_progress": [],
        "eta": null
    })).ok();

    // Update history status to downloading
    {
        let state = app.state::<AppState>();
        let mut history = state.history.write().await;
        history.update_download(&id, |r| {
            r.status = crate::persistence::DownloadStatus::Downloading;
        });
        let _ = history.save().await;
    }

    let mut last_history_update = std::time::Instant::now();

    // Process output lines
    while let Ok(Some(line)) = lines.next_line().await {
        // Check for cancellation
        if handle.cancelled.load(Ordering::Relaxed) {
            child.kill().await.ok();
            
            // Update history
            let state = app.state::<AppState>();
            let mut history = state.history.write().await;
            history.update_download(&id, |r| {
                r.status = crate::persistence::DownloadStatus::Cancelled;
            });
            let _ = history.save().await;

            return Err("Download cancelled".to_string());
        }



        // Parse progress line (custom template format)
        // The output will start with "WDM:" because "download:" is the type selector
        if line.starts_with("WDM:") {
            if let Some(progress) = parse_progress_line(&line, &id) {
                app.emit("download-progress", serde_json::json!({
                    "id": progress.id,
                    "downloaded": progress.downloaded_bytes,
                    "total": progress.total_bytes,
                    "speed": progress.speed,
                    "status": "downloading",
                    "chunk_progress": [],
                    "eta": progress.eta,
                    "percent": progress.percent
                })).ok();

                // Update history periodically (every 1 second)
                if last_history_update.elapsed().as_secs() >= 1 {
                    let state = app.state::<AppState>();
                    let mut history = state.history.write().await;
                    history.update_video_progress(&id, progress.downloaded_bytes, progress.total_bytes);
                    let _ = history.save().await;
                    last_history_update = std::time::Instant::now();
                }
            }
        }

        // Check for destination line
        if line.contains("Destination:") {
            if let Some(part) = line.split("Destination:").nth(1) {
                final_filename = part.trim().to_string();
                // Update filename in history immediately if found
                let state = app.state::<AppState>();
                let mut history = state.history.write().await;
                history.update_download(&id, |r| {
                    r.filename = PathBuf::from(&final_filename)
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    r.file_path = final_filename.clone();
                });
            }
        }
        
        // Check for "Merging formats into"
        if line.contains("Merging formats into") {
            if let Some(part) = line.split("into").nth(1) {
                 // Clean up quotes if present
                 let fname = part.trim().trim_matches('"').to_string();
                 if !fname.is_empty() {
                     final_filename = fname;
                     // Update filename in history
                    let state = app.state::<AppState>();
                    let mut history = state.history.write().await;
                    history.update_download(&id, |r| {
                        r.filename = PathBuf::from(&final_filename)
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string();
                        r.file_path = final_filename.clone();
                    });
                 }
            }
        }

        // Check for already downloaded
        if line.contains("has already been downloaded") {
            // Extract filename from the message
            let parts: Vec<&str> = line.split_whitespace().collect();
            for part in parts.iter() {
                if part.ends_with(".mp4")
                    || part.ends_with(".webm")
                    || part.ends_with(".mkv")
                    || part.ends_with(".m4a")
                {
                    final_filename = part.to_string();
                    break;
                }
            }
        }

        // Check for merge message - emit merging status
        if line.contains("[Merger]") || line.contains("Merging formats") {
            app.emit("download-progress", serde_json::json!({
                "id": id,
                "downloaded": 0,
                "total": 0,
                "speed": 0.0,
                "status": "merging",
                "chunk_progress": [],
                "eta": null
            })).ok();
        }
    }

    // Wait for process to complete
    let status = child
        .wait()
        .await
        .map_err(|e| format!("Failed to wait for yt-dlp: {}", e))?;

    if !status.success() {
        // Read stderr for error message
        let stderr_reader = BufReader::new(stderr);
        let mut stderr_lines = stderr_reader.lines();
        let mut error_msg = String::new();
        while let Ok(Some(line)) = stderr_lines.next_line().await {
            error_msg.push_str(&line);
            error_msg.push('\n');
        }

        // Update history to failed
        let state = app.state::<AppState>();
        let mut history = state.history.write().await;
        history.update_download(&id, |r| {
            r.status = crate::persistence::DownloadStatus::Failed;
        });
        let _ = history.save().await;

        return Err(format!("yt-dlp failed: {}", error_msg));
    }

    // Update history to completed
    {
        let state = app.state::<AppState>();
        let mut history = state.history.write().await;
        history.update_download(&id, |r| {
            r.status = crate::persistence::DownloadStatus::Completed;
            // Ensure path is updated if we have it
            if !final_filename.is_empty() {
                 r.filename = PathBuf::from(&final_filename)
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                 r.file_path = final_filename.clone();
            }
        });
        let _ = history.save().await;
    }

    // Emit completion
    app.emit(
        "download-complete",
        serde_json::json!({
            "id": id,
            "path": final_filename,
            "filename": PathBuf::from(&final_filename).file_name().unwrap_or_default().to_string_lossy(),
            "total_size": 0 // We might not know the final size easily, or could stat the file
        }),
    )
    .ok();

    Ok(final_filename)
}

/// Parse yt-dlp progress line
fn parse_progress_line(line: &str, id: &str) -> Option<VideoProgress> {
    // Format: WDM:PERCENT_STR|DOWNLOADED|TOTAL|TOTAL_ESTIMATE|SPEED|ETA
    let content = line.strip_prefix("WDM:")?;
    let parts: Vec<&str> = content.split('|').collect();

    if parts.len() < 6 {
        return None;
    }

    // Helper to parse numeric strings that might be floats or "NA"
    let parse_num = |s: &str| -> f64 {
        if s.contains("NA") { 0.0 } else { s.trim().parse().unwrap_or(0.0) }
    };

    // Try to parse percent from string (e.g. " 10.5%")
    let percent_str = parts[0].trim().trim_end_matches('%');
    let mut percent: f64 = parse_num(percent_str);

    // Parse bytes as f64 first to handle "1234.0", then cast to u64
    let downloaded: u64 = parse_num(parts[1]) as u64;
    
    let total_exact: u64 = parse_num(parts[2]) as u64;
    let total_est: u64 = parse_num(parts[3]) as u64;
    let mut total = if total_exact > 0 { total_exact } else { total_est };

    let speed: f64 = parse_num(parts[4]);
    let eta_str = parts[5].trim();
    let eta: Option<u64> = if eta_str.contains("NA") { None } else { eta_str.parse().ok() };

    // Strategy for Total Size:
    // 1. If we have a valid percentage (> 0) and downloaded bytes, we can infer the total size.
    //    This makes the progress bar consistent (e.g. if 10MB is 50%, Total MUST be 20MB).
    // 2. This is often more reliable than yt-dlp's total estimate which can be off or "NA".
    if percent > 0.01 && downloaded > 0 {
        let inferred_total = (downloaded as f64 / (percent / 100.0)) as u64;
        
        // Use inferred total if:
        // - Reported total is 0 (unknown)
        // - Reported total is suspiciously small (< 50KB) (likely manifest size)
        // - Reported total is less than downloaded (impossible)
        if total == 0 || total < 50 * 1024 || total < downloaded {
            total = inferred_total;
        }
    }

    // Fallback: If percent is 0 but we have totals, calculate percent
    if percent == 0.0 && total > 0 {
        percent = (downloaded as f64 / total as f64) * 100.0;
    }

    Some(VideoProgress {
        id: id.to_string(),
        status: "downloading".to_string(),
        percent,
        downloaded_bytes: downloaded,
        total_bytes: total,
        speed,
        eta,
        filename: String::new(),
    })
}
