use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs;

/// Persistent download record - saved to disk
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct DownloadRecord {
    pub id: String,
    pub url: String,
    pub filename: String,
    pub file_path: String,
    pub total_size: u64,
    pub resumable: bool,
    pub status: DownloadStatus,
    pub num_connections: u64,
    pub chunks: Vec<ChunkRecord>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum DownloadStatus {
    Pending,
    Downloading,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ChunkRecord {
    pub id: u64,
    pub start: u64,
    pub end: u64,
    pub downloaded: u64,
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct DownloadHistory {
    pub downloads: HashMap<String, DownloadRecord>,
}

impl DownloadHistory {
    fn get_data_path() -> PathBuf {
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("wdm");
        data_dir
    }

    fn get_history_file() -> PathBuf {
        Self::get_data_path().join("downloads.json")
    }

    pub async fn load() -> Self {
        let path = Self::get_history_file();

        if !path.exists() {
            return Self::default();
        }

        match fs::read_to_string(&path).await {
            Ok(content) => {
                serde_json::from_str(&content).unwrap_or_default()
            }
            Err(_) => Self::default(),
        }
    }

    pub async fn save(&self) -> Result<(), String> {
        let data_dir = Self::get_data_path();

        // Create directory if it doesn't exist
        fs::create_dir_all(&data_dir)
            .await
            .map_err(|e| format!("Failed to create data directory: {}", e))?;

        let path = Self::get_history_file();
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize history: {}", e))?;

        fs::write(&path, content)
            .await
            .map_err(|e| format!("Failed to write history file: {}", e))?;

        Ok(())
    }

    pub fn add_download(&mut self, record: DownloadRecord) {
        self.downloads.insert(record.id.clone(), record);
    }

    pub fn update_download(&mut self, id: &str, updater: impl FnOnce(&mut DownloadRecord)) {
        if let Some(record) = self.downloads.get_mut(id) {
            updater(record);
            record.updated_at = chrono::Utc::now().timestamp();
        }
    }

    pub fn update_chunk_progress(&mut self, id: &str, chunk_id: u64, downloaded: u64) {
        if let Some(record) = self.downloads.get_mut(id) {
            if let Some(chunk) = record.chunks.iter_mut().find(|c| c.id == chunk_id) {
                chunk.downloaded = downloaded;
            }
            record.updated_at = chrono::Utc::now().timestamp();
        }
    }

    pub fn get_download(&self, id: &str) -> Option<&DownloadRecord> {
        self.downloads.get(id)
    }

    pub fn get_all_downloads(&self) -> Vec<&DownloadRecord> {
        let mut downloads: Vec<_> = self.downloads.values().collect();
        downloads.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        downloads
    }

    pub fn remove_download(&mut self, id: &str) {
        self.downloads.remove(id);
    }
}

impl DownloadRecord {
    pub fn new(
        id: String,
        url: String,
        filename: String,
        file_path: String,
        total_size: u64,
        resumable: bool,
        num_connections: u64,
    ) -> Self {
        let chunk_size = total_size / num_connections;
        let mut chunks = Vec::new();

        for i in 0..num_connections {
            let start = i * chunk_size;
            let end = if i == num_connections - 1 {
                total_size - 1
            } else {
                (i + 1) * chunk_size - 1
            };
            chunks.push(ChunkRecord {
                id: i,
                start,
                end,
                downloaded: 0,
            });
        }

        let now = chrono::Utc::now().timestamp();
        Self {
            id,
            url,
            filename,
            file_path,
            total_size,
            resumable,
            status: DownloadStatus::Pending,
            num_connections,
            chunks,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn total_downloaded(&self) -> u64 {
        self.chunks.iter().map(|c| c.downloaded).sum()
    }
}
