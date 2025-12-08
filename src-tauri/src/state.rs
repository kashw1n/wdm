use crate::persistence::DownloadHistory;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use tokio::sync::RwLock;

pub const DEFAULT_CONNECTIONS: u64 = 8;

pub struct AppState {
    pub downloads: RwLock<HashMap<String, Arc<DownloadHandle>>>,
    pub settings: RwLock<Settings>,
    pub history: RwLock<DownloadHistory>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Settings {
    pub connections: u64,
    pub download_folder: Option<String>,
    #[serde(default)]
    pub speed_limit: u64, // bytes per second, 0 = unlimited
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            connections: DEFAULT_CONNECTIONS,
            download_folder: None,
            speed_limit: 0,
        }
    }
}

impl Settings {
    fn get_settings_path() -> PathBuf {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("wdm")
            .join("settings.json")
    }

    pub async fn load() -> Self {
        let path = Self::get_settings_path();
        if !path.exists() {
            return Self::default();
        }
        match tokio::fs::read_to_string(&path).await {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub async fn save(&self) -> Result<(), String> {
        let path = Self::get_settings_path();
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| format!("Failed to create settings directory: {}", e))?;
        }
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize settings: {}", e))?;
        tokio::fs::write(&path, content)
            .await
            .map_err(|e| format!("Failed to write settings: {}", e))?;
        Ok(())
    }

    pub fn get_download_folder(&self) -> PathBuf {
        self.download_folder
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(|| dirs::download_dir().unwrap_or_else(|| PathBuf::from(".")))
    }
}

pub struct DownloadHandle {
    pub id: String,
    pub cancelled: AtomicBool,
    pub paused: AtomicBool,
    pub chunk_downloaded: Vec<Arc<AtomicU64>>,
    pub speed_limit: AtomicU64, // bytes per second, 0 = unlimited
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
