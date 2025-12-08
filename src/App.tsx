import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import "./App.css";

import {
  Download,
  DownloadInfo,
  UrlInfo,
  FileExistsInfo,
  DownloadProgress,
  DownloadComplete,
  DownloadError,
} from "./types";
import { formatBytes } from "./utils";

import { DownloadItem } from "./components/DownloadItem";
import { SettingsPanel } from "./components/SettingsPanel";
import { HistoryPanel } from "./components/HistoryPanel";
import { RenamePrompt } from "./components/RenamePrompt";
import { FileInfo } from "./components/FileInfo";
import { AddDownloadForm } from "./components/AddDownloadForm";

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
  const [history, setHistory] = useState<DownloadInfo[]>([]);
  const [showHistory, setShowHistory] = useState(false);
  const [downloadFolder, setDownloadFolder] = useState("");
  const [speedLimit, setSpeedLimit] = useState(0); // 0 = unlimited

  useEffect(() => {
    // Load initial settings
    invoke<number>("get_connections").then(setConnections);
    invoke<string>("get_download_folder").then(setDownloadFolder);
    invoke<number>("get_speed_limit").then(setSpeedLimit);

    // Load download history
    loadHistory();

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
      // Refresh history
      loadHistory();
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
      // Refresh history
      loadHistory();
    });

    return () => {
      unlistenProgress.then((fn) => fn());
      unlistenComplete.then((fn) => fn());
      unlistenError.then((fn) => fn());
    };
  }, []);

  async function loadHistory() {
    try {
      const hist = await invoke<DownloadInfo[]>("get_download_history");
      setHistory(hist);
    } catch (e) {
      console.error("Failed to load history:", e);
    }
  }

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

    const currentUrlInfo = { ...urlInfo };
    setError(null);

    try {
      const fileCheck = await invoke<FileExistsInfo>("check_file_exists", {
        filename: currentUrlInfo.filename,
      });

      if (fileCheck.exists) {
        setRenamePrompt({
          show: true,
          originalName: currentUrlInfo.filename,
          suggestedName: fileCheck.suggested_name,
          urlInfo: currentUrlInfo,
        });
        setCustomFilename(fileCheck.suggested_name);
        return;
      }

      await proceedWithDownload(currentUrlInfo, currentUrlInfo.filename);
    } catch (e) {
      setError(String(e));
    }
  }

  async function proceedWithDownload(info: UrlInfo, filename: string) {
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
      loadHistory();
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

  async function resumeInterruptedDownload(id: string) {
    try {
      // Add to active downloads UI
      const histItem = history.find((h) => h.id === id);
      if (histItem) {
        setDownloads((prev) => {
          const newMap = new Map(prev);
          newMap.set(id, {
            id: id,
            filename: histItem.filename,
            url: histItem.url,
            size: histItem.total_size,
            progress: {
              id: id,
              downloaded: histItem.downloaded,
              total: histItem.total_size,
              speed: 0,
              status: "downloading",
              chunk_progress: [],
            },
            completed: false,
          });
          return newMap;
        });
      }

      await invoke("resume_interrupted_download", { id });
    } catch (e) {
      setError(String(e));
    }
  }

  async function removeFromHistory(id: string) {
    try {
      await invoke("remove_from_history", { id });
      loadHistory();
    } catch (e) {
      console.error("Failed to remove from history:", e);
    }
  }

  async function clearHistory() {
    try {
      await invoke("clear_download_history");
      loadHistory();
    } catch (e) {
      console.error("Failed to clear history:", e);
    }
  }

  async function updateConnections(value: number) {
    try {
      await invoke("set_connections", { connections: value });
      setConnections(value);
    } catch (e) {
      console.error("Failed to set connections:", e);
    }
  }

  async function selectDownloadFolder() {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "Select Download Folder",
      });
      if (selected) {
        await invoke("set_download_folder", { folder: selected });
        setDownloadFolder(selected);
      }
    } catch (e) {
      console.error("Failed to select folder:", e);
    }
  }

  async function resetDownloadFolder() {
    try {
      const defaultFolder = await invoke<string>("reset_download_folder");
      setDownloadFolder(defaultFolder);
    } catch (e) {
      console.error("Failed to reset folder:", e);
    }
  }

  async function updateSpeedLimit(limit: number) {
    try {
      await invoke("set_speed_limit", { limit });
      setSpeedLimit(limit);
    } catch (e) {
      console.error("Failed to set speed limit:", e);
    }
  }

  const activeDownloads = Array.from(downloads.values()).filter(
    (d) => !d.completed && !d.error && d.progress?.status !== "cancelled"
  );
  const completedDownloads = Array.from(downloads.values()).filter(
    (d) => d.completed || d.error || d.progress?.status === "cancelled"
  );

  // Filter history for interrupted downloads that can be resumed
  const interruptedDownloads = history.filter(
    (h) =>
      (h.status === "Paused" || h.status === "Failed" || h.status === "Downloading") &&
      h.resumable &&
      !downloads.has(h.id)
  );

  return (
    <main className="container">
      <div className="header">
        <h1>Web Download Manager</h1>
        <div className="header-buttons">
          <button
            className="history-btn"
            onClick={() => setShowHistory(!showHistory)}
          >
            History
          </button>
          <button
            className="settings-btn"
            onClick={() => setShowSettings(!showSettings)}
          >
            Settings
          </button>
        </div>
      </div>

      {showSettings && (
        <SettingsPanel
          connections={connections}
          downloadFolder={downloadFolder}
          speedLimit={speedLimit}
          updateConnections={updateConnections}
          selectDownloadFolder={selectDownloadFolder}
          resetDownloadFolder={resetDownloadFolder}
          updateSpeedLimit={updateSpeedLimit}
        />
      )}

      {showHistory && (
        <HistoryPanel
          history={history}
          clearHistory={clearHistory}
          removeFromHistory={removeFromHistory}
        />
      )}

      <AddDownloadForm
        url={url}
        setUrl={setUrl}
        loading={loading}
        onSubmit={fetchInfo}
      />

      {error && <p className="error">{error}</p>}

      {renamePrompt.show && (
        <RenamePrompt
          originalName={renamePrompt.originalName}
          customFilename={customFilename}
          setCustomFilename={setCustomFilename}
          onConfirm={handleRenameConfirm}
          onCancel={handleRenameCancel}
        />
      )}

      {urlInfo && !renamePrompt.show && (
        <FileInfo
          urlInfo={urlInfo}
          onClose={() => setUrlInfo(null)}
          onDownload={startDownload}
        />
      )}

      {interruptedDownloads.length > 0 && !showHistory && (
        <div className="interrupted-section">
          <h2>Resume Downloads</h2>
          {interruptedDownloads.map((item) => (
            <div key={item.id} className="interrupted-item">
              <div className="interrupted-info">
                <span className="filename">{item.filename}</span>
                <span className="interrupted-progress">
                  {formatBytes(item.downloaded)} / {formatBytes(item.total_size)} (
                  {((item.downloaded / item.total_size) * 100).toFixed(1)}%)
                </span>
              </div>
              <button
                className="resume-btn"
                onClick={() => resumeInterruptedDownload(item.id)}
              >
                Resume
              </button>
            </div>
          ))}
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
                <button
                  className="remove-btn"
                  onClick={() => removeDownload(download.id)}
                >
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

export default App;