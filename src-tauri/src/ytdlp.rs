use reqwest::Client;
use std::path::PathBuf;
use tokio::fs;
use tokio::io::AsyncWriteExt;

#[cfg(target_os = "macos")]
const YTDLP_URL: &str = "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_macos";

#[cfg(target_os = "windows")]
const YTDLP_URL: &str = "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe";

#[cfg(target_os = "linux")]
const YTDLP_URL: &str = "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_linux";

#[cfg(target_os = "macos")]
const YTDLP_BINARY_NAME: &str = "yt-dlp";

#[cfg(target_os = "windows")]
const YTDLP_BINARY_NAME: &str = "yt-dlp.exe";

#[cfg(target_os = "linux")]
const YTDLP_BINARY_NAME: &str = "yt-dlp";

/// Get the directory where yt-dlp binary is stored
pub fn get_ytdlp_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("wdm")
        .join("bin")
}

/// Get the full path to the yt-dlp binary
pub fn get_ytdlp_path() -> PathBuf {
    get_ytdlp_dir().join(YTDLP_BINARY_NAME)
}

/// Check if yt-dlp is installed in our app directory
pub fn is_ytdlp_installed() -> bool {
    get_ytdlp_path().exists()
}

/// Download yt-dlp binary from GitHub releases
pub async fn download_ytdlp<F>(progress_callback: F) -> Result<PathBuf, String>
where
    F: Fn(u64, u64) + Send + 'static,
{
    let ytdlp_dir = get_ytdlp_dir();
    let ytdlp_path = get_ytdlp_path();

    // Create bin directory if it doesn't exist
    fs::create_dir_all(&ytdlp_dir)
        .await
        .map_err(|e| format!("Failed to create bin directory: {}", e))?;

    // Download the binary
    let client = Client::builder()
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .get(YTDLP_URL)
        .send()
        .await
        .map_err(|e| format!("Failed to download yt-dlp: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Failed to download yt-dlp: HTTP {}",
            response.status()
        ));
    }

    let total_size = response.content_length().unwrap_or(0);
    let mut downloaded: u64 = 0;

    // Create temporary file
    let temp_path = ytdlp_path.with_extension("tmp");
    let mut file = fs::File::create(&temp_path)
        .await
        .map_err(|e| format!("Failed to create temp file: {}", e))?;

    // Stream download with progress
    let mut stream = response.bytes_stream();
    use futures::StreamExt;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Download error: {}", e))?;
        file.write_all(&chunk)
            .await
            .map_err(|e| format!("Failed to write file: {}", e))?;

        downloaded += chunk.len() as u64;
        progress_callback(downloaded, total_size);
    }

    file.flush()
        .await
        .map_err(|e| format!("Failed to flush file: {}", e))?;
    drop(file);

    // Rename temp file to final path
    fs::rename(&temp_path, &ytdlp_path)
        .await
        .map_err(|e| format!("Failed to rename temp file: {}", e))?;

    // Make executable on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&ytdlp_path)
            .await
            .map_err(|e| format!("Failed to get file metadata: {}", e))?
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&ytdlp_path, perms)
            .await
            .map_err(|e| format!("Failed to set executable permission: {}", e))?;
    }

    Ok(ytdlp_path)
}

/// Ensure yt-dlp is installed, download if needed
pub async fn ensure_ytdlp<F>(progress_callback: F) -> Result<PathBuf, String>
where
    F: Fn(u64, u64) + Send + 'static,
{
    if is_ytdlp_installed() {
        Ok(get_ytdlp_path())
    } else {
        download_ytdlp(progress_callback).await
    }
}

/// Get yt-dlp version
pub async fn get_ytdlp_version() -> Result<String, String> {
    let ytdlp_path = get_ytdlp_path();

    if !ytdlp_path.exists() {
        return Err("yt-dlp not installed".to_string());
    }

    let output = tokio::process::Command::new(&ytdlp_path)
        .arg("--version")
        .output()
        .await
        .map_err(|e| format!("Failed to run yt-dlp: {}", e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err("Failed to get yt-dlp version".to_string())
    }
}
