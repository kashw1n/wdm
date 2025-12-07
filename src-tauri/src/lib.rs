use serde::Serialize;

#[derive(Serialize)]
pub struct UrlInfo {
    pub url: String,
    pub filename: String,
    pub size: Option<u64>,
    pub resumable: bool,
}

#[tauri::command]
async fn fetch_url_info(url: String) -> Result<UrlInfo, String> {
    let client = reqwest::Client::new();

    let response = client
        .head(&url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()));
    }

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

    // Try to get filename from Content-Disposition or URL
    let filename = headers
        .get(reqwest::header::CONTENT_DISPOSITION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| {
            v.split("filename=").nth(1).map(|s| s.trim_matches('"').to_string())
        })
        .unwrap_or_else(|| {
            url.split('/').last().unwrap_or("download").to_string()
        });

    Ok(UrlInfo {
        url,
        filename,
        size,
        resumable,
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![fetch_url_info])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
