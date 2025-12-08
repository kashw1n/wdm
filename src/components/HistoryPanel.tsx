import { DownloadInfo } from "../types";
import { formatBytes } from "../utils";
import { invoke } from "@tauri-apps/api/core";
import "./HistoryPanel.css";

interface HistoryPanelProps {
  history: DownloadInfo[];
  clearHistory: () => void;
  removeFromHistory: (id: string) => void;
}

export function HistoryPanel({ history, clearHistory, removeFromHistory }: HistoryPanelProps) {
  const finishedDownloads = history.filter(
    (h) => h.status === "Completed" || h.status === "Failed" || h.status === "Cancelled"
  );

  async function openFile(path: string) {
    try {
      await invoke("open_file", { path });
    } catch (e) {
      console.error("Failed to open file:", e);
    }
  }

  async function showInFolder(path: string) {
    try {
      await invoke("show_in_folder", { path });
    } catch (e) {
      console.error("Failed to show in folder:", e);
    }
  }

  return (
    <div className="history-panel">
      <div className="history-header">
        <h3>Download History</h3>
        {finishedDownloads.length > 0 && (
          <button className="clear-history-btn" onClick={clearHistory}>
            Clear All
          </button>
        )}
      </div>
      {finishedDownloads.length === 0 ? (
        <p className="no-history">No download history</p>
      ) : (
        <div className="history-list">
          {finishedDownloads.map((item) => (
            <div key={item.id} className="history-item">
              <div className="history-item-info">
                <span className="history-filename">{item.filename}</span>
                <span className="history-size">{formatBytes(item.total_size)}</span>
                <span className={`history-status status-${item.status.toLowerCase()}`}>
                  {item.status}
                </span>
              </div>
              <div className="history-item-actions">
                {item.status === "Completed" && (
                    <>
                        <button className="action-btn" onClick={() => openFile(item.file_path)}>
                            Open
                        </button>
                        <button className="action-btn" onClick={() => showInFolder(item.file_path)}>
                            Folder
                        </button>
                    </>
                )}
                <button
                  className="remove-history-btn"
                  onClick={() => removeFromHistory(item.id)}
                >
                  Remove
                </button>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}