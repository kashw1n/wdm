import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";

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
  const [speedLimit, setSpeedLimit] = useState(0);

  useEffect(() => {
    invoke<number>("get_connections").then(setConnections);
    invoke<string>("get_download_folder").then(setDownloadFolder);
    invoke<number>("get_speed_limit").then(setSpeedLimit);
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
  const interruptedDownloads = history.filter(
    (h) =>
      (h.status === "Paused" || h.status === "Failed" || h.status === "Downloading") &&
      h.resumable &&
      !downloads.has(h.id)
  );

  return (
    <div className="min-h-screen bg-dark-900 p-4 sm:p-6">
      <div className="max-w-4xl mx-auto space-y-4 sm:space-y-6">
        {/* Header */}
        <header className="flex items-center justify-between gap-4">
          <div className="flex items-center gap-2 sm:gap-3 min-w-0">
            <div className="w-8 h-8 sm:w-10 sm:h-10 rounded-xl bg-gradient-to-br from-accent to-accent-dim flex items-center justify-center shadow-glow-sm flex-shrink-0">
              <svg className="w-5 h-5 sm:w-6 sm:h-6 text-dark-900" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" />
              </svg>
            </div>
            <h1 className="text-lg sm:text-xl font-semibold text-gray-100 truncate">
              WDM
              <span className="text-gray-500 font-normal ml-2 text-xs sm:text-sm hidden xs:inline">Web Download Manager</span>
            </h1>
          </div>
          <div className="flex gap-1.5 sm:gap-2 flex-shrink-0">
            <button
              onClick={() => setShowHistory(!showHistory)}
              className={`btn-secondary text-xs sm:text-sm px-2 sm:px-4 ${showHistory ? 'border-accent text-accent' : ''}`}
            >
              <svg className="w-4 h-4 sm:mr-1.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
              </svg>
              <span className="hidden sm:inline">History</span>
            </button>
            <button
              onClick={() => setShowSettings(!showSettings)}
              className={`btn-secondary text-xs sm:text-sm px-2 sm:px-4 ${showSettings ? 'border-accent text-accent' : ''}`}
            >
              <svg className="w-4 h-4 sm:mr-1.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
              </svg>
              <span className="hidden sm:inline">Settings</span>
            </button>
          </div>
        </header>

        {/* Settings Panel */}
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

        {/* History Panel */}
        {showHistory && (
          <HistoryPanel
            history={history}
            clearHistory={clearHistory}
            removeFromHistory={removeFromHistory}
          />
        )}

        {/* Add Download Form */}
        <AddDownloadForm
          url={url}
          setUrl={setUrl}
          loading={loading}
          onSubmit={fetchInfo}
        />

        {/* Error Message */}
        {error && (
          <div className="bg-red-500/10 border border-red-500/30 rounded-xl p-4 animate-fade-in">
            <div className="flex items-center gap-3">
              <svg className="w-5 h-5 text-red-400 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
              </svg>
              <p className="text-red-400 text-sm">{error}</p>
            </div>
          </div>
        )}

        {/* Rename Prompt */}
        {renamePrompt.show && (
          <RenamePrompt
            originalName={renamePrompt.originalName}
            customFilename={customFilename}
            setCustomFilename={setCustomFilename}
            onConfirm={handleRenameConfirm}
            onCancel={handleRenameCancel}
          />
        )}

        {/* File Info */}
        {urlInfo && !renamePrompt.show && (
          <FileInfo
            urlInfo={urlInfo}
            onClose={() => setUrlInfo(null)}
            onDownload={startDownload}
          />
        )}

        {/* Interrupted Downloads */}
        {interruptedDownloads.length > 0 && !showHistory && (
          <section className="space-y-3">
            <h2 className="text-sm font-medium text-amber-400 flex items-center gap-2">
              <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
              </svg>
              Resume Downloads
            </h2>
            {interruptedDownloads.map((item) => (
              <div
                key={item.id}
                className="card border-amber-500/30 bg-amber-500/5 flex items-center justify-between"
              >
                <div className="flex-1 min-w-0">
                  <p className="font-medium text-gray-200 truncate">{item.filename}</p>
                  <p className="text-sm text-gray-500">
                    {formatBytes(item.downloaded)} / {formatBytes(item.total_size)}
                    <span className="ml-2 text-amber-400">
                      ({((item.downloaded / item.total_size) * 100).toFixed(1)}%)
                    </span>
                  </p>
                </div>
                <button
                  onClick={() => resumeInterruptedDownload(item.id)}
                  className="btn-success ml-4"
                >
                  Resume
                </button>
              </div>
            ))}
          </section>
        )}

        {/* Active Downloads */}
        {activeDownloads.length > 0 && (
          <section className="space-y-3">
            <h2 className="text-sm font-medium text-gray-400 flex items-center gap-2">
              <svg className="w-4 h-4 text-accent" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" />
              </svg>
              Active Downloads
              <span className="badge badge-info">{activeDownloads.length}</span>
            </h2>
            {activeDownloads.map((download) => (
              <DownloadItem
                key={download.id}
                download={download}
                onPause={() => pauseDownload(download.id)}
                onResume={() => resumeDownload(download.id)}
                onCancel={() => cancelDownload(download.id)}
              />
            ))}
          </section>
        )}

        {/* Completed Downloads */}
        {completedDownloads.length > 0 && (
          <section className="space-y-3">
            <h2 className="text-sm font-medium text-gray-400 flex items-center gap-2">
              <svg className="w-4 h-4 text-emerald-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
              </svg>
              Completed
            </h2>
            {completedDownloads.map((download) => (
              <div key={download.id} className="card opacity-75">
                <div className="flex items-center justify-between">
                  <div className="flex-1 min-w-0">
                    <p className="font-medium text-gray-300 truncate">{download.filename}</p>
                    {download.completed && (
                      <p className="text-xs text-gray-500 truncate mt-1">{download.completedPath}</p>
                    )}
                    {download.error && (
                      <p className="text-sm text-red-400 mt-1">{download.error}</p>
                    )}
                    {download.progress?.status === "cancelled" && (
                      <p className="text-sm text-amber-400 mt-1">Cancelled</p>
                    )}
                  </div>
                  <button
                    onClick={() => removeDownload(download.id)}
                    className="btn-ghost text-sm ml-4"
                  >
                    <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                    </svg>
                  </button>
                </div>
              </div>
            ))}
          </section>
        )}

        {/* Empty State */}
        {activeDownloads.length === 0 && completedDownloads.length === 0 && interruptedDownloads.length === 0 && !urlInfo && !showSettings && !showHistory && (
          <div className="text-center py-16">
            <div className="w-20 h-20 mx-auto mb-4 rounded-full bg-dark-800 flex items-center justify-center">
              <svg className="w-10 h-10 text-gray-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" />
              </svg>
            </div>
            <p className="text-gray-500 text-sm">Paste a URL above to start downloading</p>
          </div>
        )}
      </div>
    </div>
  );
}

export default App;
