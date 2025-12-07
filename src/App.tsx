import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./App.css";

interface UrlInfo {
  url: string;
  filename: string;
  size: number | null;
  resumable: boolean;
}

interface ChunkProgress {
  id: number;
  downloaded: number;
  total: number;
}

interface DownloadProgress {
  downloaded: number;
  total: number;
  speed: number;
  chunk_progress: ChunkProgress[];
}

interface DownloadComplete {
  path: string;
  filename: string;
  total_size: number;
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return bytes + " B";
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(2) + " KB";
  if (bytes < 1024 * 1024 * 1024) return (bytes / (1024 * 1024)).toFixed(2) + " MB";
  return (bytes / (1024 * 1024 * 1024)).toFixed(2) + " GB";
}

function formatSpeed(bytesPerSec: number): string {
  if (bytesPerSec < 1024) return bytesPerSec.toFixed(0) + " B/s";
  if (bytesPerSec < 1024 * 1024) return (bytesPerSec / 1024).toFixed(2) + " KB/s";
  if (bytesPerSec < 1024 * 1024 * 1024) return (bytesPerSec / (1024 * 1024)).toFixed(2) + " MB/s";
  return (bytesPerSec / (1024 * 1024 * 1024)).toFixed(2) + " GB/s";
}

function App() {
  const [url, setUrl] = useState("");
  const [urlInfo, setUrlInfo] = useState<UrlInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [downloading, setDownloading] = useState(false);
  const [progress, setProgress] = useState<DownloadProgress | null>(null);
  const [completedPath, setCompletedPath] = useState<string | null>(null);

  useEffect(() => {
    const unlistenProgress = listen<DownloadProgress>("download-progress", (event) => {
      setProgress(event.payload);
    });

    const unlistenComplete = listen<DownloadComplete>("download-complete", (event) => {
      setDownloading(false);
      setCompletedPath(event.payload.path);
    });

    return () => {
      unlistenProgress.then((fn) => fn());
      unlistenComplete.then((fn) => fn());
    };
  }, []);

  async function fetchInfo() {
    if (!url.trim()) return;

    setLoading(true);
    setError(null);
    setUrlInfo(null);
    setCompletedPath(null);
    setProgress(null);

    try {
      const info = await invoke<UrlInfo>("fetch_url_info", { url });
      setUrlInfo(info);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  async function startDownload() {
    if (!urlInfo) return;

    setDownloading(true);
    setError(null);
    setCompletedPath(null);
    setProgress(null);

    try {
      await invoke<string>("start_download", {
        url: urlInfo.url,
        filename: urlInfo.filename,
        size: urlInfo.size || 0,
        resumable: urlInfo.resumable,
      });
    } catch (e) {
      setError(String(e));
      setDownloading(false);
    }
  }

  const overallPercent = progress
    ? ((progress.downloaded / progress.total) * 100).toFixed(1)
    : "0";

  return (
    <main className="container">
      <h1>Web Download Manager</h1>

      <form
        className="row"
        onSubmit={(e) => {
          e.preventDefault();
          fetchInfo();
        }}
      >
        <input
          value={url}
          onChange={(e) => setUrl(e.currentTarget.value)}
          placeholder="Enter URL to download..."
          style={{ flex: 1 }}
          disabled={downloading}
        />
        <button type="submit" disabled={loading || downloading}>
          {loading ? "Checking..." : "Check URL"}
        </button>
      </form>

      {error && <p className="error">{error}</p>}

      {urlInfo && !downloading && !completedPath && (
        <div className="file-info">
          <h3>File Info</h3>
          <p><strong>Filename:</strong> {urlInfo.filename}</p>
          <p><strong>Size:</strong> {urlInfo.size ? formatBytes(urlInfo.size) : "Unknown"}</p>
          <p><strong>Resumable:</strong> {urlInfo.resumable ? "Yes (multi-connection)" : "No (single connection)"}</p>
          <button onClick={startDownload} className="download-btn">
            Download
          </button>
        </div>
      )}

      {downloading && progress && (
        <div className="download-progress">
          <h3>Downloading: {urlInfo?.filename}</h3>

          <div className="overall-progress">
            <div className="progress-bar">
              <div
                className="progress-fill"
                style={{ width: `${overallPercent}%` }}
              />
            </div>
            <div className="progress-stats">
              <span>{formatBytes(progress.downloaded)} / {formatBytes(progress.total)}</span>
              <span>{overallPercent}%</span>
              <span>{formatSpeed(progress.speed)}</span>
            </div>
          </div>

          {progress.chunk_progress.length > 1 && (
            <div className="chunk-progress">
              <h4>Connections ({progress.chunk_progress.length})</h4>
              <div className="chunks">
                {progress.chunk_progress.map((chunk) => (
                  <div key={chunk.id} className="chunk">
                    <div className="chunk-bar">
                      <div
                        className="chunk-fill"
                        style={{ width: `${(chunk.downloaded / chunk.total) * 100}%` }}
                      />
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>
      )}

      {completedPath && (
        <div className="download-complete">
          <h3>Download Complete!</h3>
          <p>Saved to: {completedPath}</p>
        </div>
      )}
    </main>
  );
}

export default App;
