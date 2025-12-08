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
  id: string;
  downloaded: number;
  total: number;
  speed: number;
  status: string;
  chunk_progress: ChunkProgress[];
}

interface DownloadComplete {
  id: string;
  path: string;
  filename: string;
  total_size: number;
}

interface DownloadError {
  id: string;
  error: string;
}

interface Download {
  id: string;
  filename: string;
  url: string;
  size: number;
  progress: DownloadProgress | null;
  completed: boolean;
  completedPath?: string;
  error?: string;
}

interface FileExistsInfo {
  exists: boolean;
  suggested_name: string;
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
  const [downloads, setDownloads] = useState<Map<string, Download>>(new Map());
  const [connections, setConnections] = useState(8);
  const [showSettings, setShowSettings] = useState(false);
  const [renamePrompt, setRenamePrompt] = useState<{
    show: boolean;
    originalName: string;
    suggestedName: string;
    urlInfo: UrlInfo | null;
  }>({ show: false, originalName: "", suggestedName: "", urlInfo: null });
  const [customFilename, setCustomFilename] = useState("");

  useEffect(() => {
    // Load initial connections setting
    invoke<number>("get_connections").then(setConnections);

    const unlistenProgress = listen<DownloadProgress>("download-progress", (event) => {
      const progress = event.payload;
      setDownloads((prev) => {
        const newMap = new Map(prev);
        const download = newMap.get(progress.id);
        if (download) {
          newMap.set(progress.id, { ...download, progress });
        }
        return newMap;
      });
    });

    const unlistenComplete = listen<DownloadComplete>("download-complete", (event) => {
      const complete = event.payload;
      setDownloads((prev) => {
        const newMap = new Map(prev);
        const download = newMap.get(complete.id);
        if (download) {
          newMap.set(complete.id, {
            ...download,
            completed: true,
            completedPath: complete.path,
            progress: null,
          });
        }
        return newMap;
      });
    });

    const unlistenError = listen<DownloadError>("download-error", (event) => {
      const err = event.payload;
      setDownloads((prev) => {
        const newMap = new Map(prev);
        const download = newMap.get(err.id);
        if (download) {
          newMap.set(err.id, { ...download, error: err.error, progress: null });
        }
        return newMap;
      });
    });

    return () => {
      unlistenProgress.then((fn) => fn());
      unlistenComplete.then((fn) => fn());
      unlistenError.then((fn) => fn());
    };
  }, []);

  async function fetchInfo() {
    if (!url.trim()) return;

    setLoading(true);
    setError(null);
    setUrlInfo(null);

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

    // Capture values before async operations to avoid closure issues
    const currentUrlInfo = { ...urlInfo };

    setError(null);

    try {
      // Check if file already exists
      const fileCheck = await invoke<FileExistsInfo>("check_file_exists", {
        filename: currentUrlInfo.filename,
      });

      if (fileCheck.exists) {
        // Show rename prompt
        setRenamePrompt({
          show: true,
          originalName: currentUrlInfo.filename,
          suggestedName: fileCheck.suggested_name,
          urlInfo: currentUrlInfo,
        });
        setCustomFilename(fileCheck.suggested_name);
        return;
      }

      // File doesn't exist, proceed with download
      await proceedWithDownload(currentUrlInfo, currentUrlInfo.filename);
    } catch (e) {
      setError(String(e));
    }
  }

  async function proceedWithDownload(info: UrlInfo, filename: string) {
    // Clear form
    setUrl("");
    setUrlInfo(null);
    setRenamePrompt({ show: false, originalName: "", suggestedName: "", urlInfo: null });

    try {
      const downloadId = await invoke<string>("start_download", {
        url: info.url,
        filename: filename,
        size: info.size || 0,
        resumable: info.resumable,
      });

      // Add to downloads list
      setDownloads((prev) => {
        const newMap = new Map(prev);
        newMap.set(downloadId, {
          id: downloadId,
          filename: filename,
          url: info.url,
          size: info.size || 0,
          progress: null,
          completed: false,
        });
        return newMap;
      });
    } catch (e) {
      setError(String(e));
    }
  }

  function handleRenameConfirm() {
    if (renamePrompt.urlInfo && customFilename.trim()) {
      proceedWithDownload(renamePrompt.urlInfo, customFilename.trim());
    }
  }

  function handleRenameCancel() {
    setRenamePrompt({ show: false, originalName: "", suggestedName: "", urlInfo: null });
    setCustomFilename("");
  }

  async function pauseDownload(id: string) {
    try {
      await invoke("pause_download", { id });
    } catch (e) {
      console.error("Failed to pause:", e);
    }
  }

  async function resumeDownload(id: string) {
    try {
      await invoke("resume_download", { id });
    } catch (e) {
      console.error("Failed to resume:", e);
    }
  }

  async function cancelDownload(id: string) {
    try {
      await invoke("cancel_download", { id });
    } catch (e) {
      console.error("Failed to cancel:", e);
    }
  }

  function removeDownload(id: string) {
    setDownloads((prev) => {
      const newMap = new Map(prev);
      newMap.delete(id);
      return newMap;
    });
  }

  async function updateConnections(value: number) {
    try {
      await invoke("set_connections", { connections: value });
      setConnections(value);
    } catch (e) {
      console.error("Failed to set connections:", e);
    }
  }

  const activeDownloads = Array.from(downloads.values()).filter(
    (d) => !d.completed && !d.error && d.progress?.status !== "cancelled"
  );
  const completedDownloads = Array.from(downloads.values()).filter(
    (d) => d.completed || d.error || d.progress?.status === "cancelled"
  );

  return (
    <main className="container">
      <div className="header">
        <h1>Web Download Manager</h1>
        <button className="settings-btn" onClick={() => setShowSettings(!showSettings)}>
          Settings
        </button>
      </div>

      {showSettings && (
        <div className="settings-panel">
          <h3>Settings</h3>
          <div className="setting-row">
            <label>Connections per download:</label>
            <select
              value={connections}
              onChange={(e) => updateConnections(Number(e.target.value))}
            >
              {[1, 2, 4, 8, 16, 32].map((n) => (
                <option key={n} value={n}>
                  {n}
                </option>
              ))}
            </select>
          </div>
        </div>
      )}

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
        />
        <button type="submit" disabled={loading}>
          {loading ? "Checking..." : "Check URL"}
        </button>
      </form>

      {error && <p className="error">{error}</p>}

      {renamePrompt.show && (
        <div className="rename-prompt">
          <h3>File Already Exists</h3>
          <p>
            A file named <strong>{renamePrompt.originalName}</strong> already exists in your Downloads folder.
          </p>
          <div className="rename-input-row">
            <label>Save as:</label>
            <input
              type="text"
              value={customFilename}
              onChange={(e) => setCustomFilename(e.target.value)}
              placeholder="Enter new filename"
            />
          </div>
          <div className="rename-buttons">
            <button onClick={handleRenameConfirm} className="confirm-btn">
              Download
            </button>
            <button onClick={handleRenameCancel} className="cancel-btn">
              Cancel
            </button>
          </div>
        </div>
      )}

      {urlInfo && !renamePrompt.show && (
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

      {activeDownloads.length > 0 && (
        <div className="downloads-section">
          <h2>Active Downloads</h2>
          {activeDownloads.map((download) => (
            <DownloadItem
              key={download.id}
              download={download}
              onPause={() => pauseDownload(download.id)}
              onResume={() => resumeDownload(download.id)}
              onCancel={() => cancelDownload(download.id)}
            />
          ))}
        </div>
      )}

      {completedDownloads.length > 0 && (
        <div className="downloads-section">
          <h2>Completed</h2>
          {completedDownloads.map((download) => (
            <div key={download.id} className="download-item completed">
              <div className="download-header">
                <span className="filename">{download.filename}</span>
                <button className="remove-btn" onClick={() => removeDownload(download.id)}>
                  x
                </button>
              </div>
              {download.completed && (
                <p className="completed-path">Saved to: {download.completedPath}</p>
              )}
              {download.error && <p className="error-msg">{download.error}</p>}
              {download.progress?.status === "cancelled" && (
                <p className="cancelled-msg">Cancelled</p>
              )}
            </div>
          ))}
        </div>
      )}
    </main>
  );
}

interface DownloadItemProps {
  download: Download;
  onPause: () => void;
  onResume: () => void;
  onCancel: () => void;
}

function DownloadItem({ download, onPause, onResume, onCancel }: DownloadItemProps) {
  const progress = download.progress;
  const isPaused = progress?.status === "paused";
  const percent = progress
    ? ((progress.downloaded / progress.total) * 100).toFixed(1)
    : "0";

  return (
    <div className="download-item">
      <div className="download-header">
        <span className="filename">{download.filename}</span>
        <div className="download-controls">
          {isPaused ? (
            <button className="control-btn resume" onClick={onResume} title="Resume">
              Resume
            </button>
          ) : (
            <button className="control-btn pause" onClick={onPause} title="Pause">
              Pause
            </button>
          )}
          <button className="control-btn cancel" onClick={onCancel} title="Cancel">
            Cancel
          </button>
        </div>
      </div>

      {progress && (
        <>
          <div className="progress-bar">
            <div className="progress-fill" style={{ width: `${percent}%` }} />
          </div>
          <div className="progress-stats">
            <span>
              {formatBytes(progress.downloaded)} / {formatBytes(progress.total)}
            </span>
            <span>{percent}%</span>
            <span>{isPaused ? "Paused" : formatSpeed(progress.speed)}</span>
          </div>

          {progress.chunk_progress.length > 1 && (
            <div className="chunk-progress">
              <div className="chunks">
                {progress.chunk_progress.map((chunk) => (
                  <div key={chunk.id} className="chunk">
                    <div className="chunk-bar">
                      <div
                        className="chunk-fill"
                        style={{
                          width: `${(chunk.downloaded / chunk.total) * 100}%`,
                        }}
                      />
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}
        </>
      )}
    </div>
  );
}

export default App;
