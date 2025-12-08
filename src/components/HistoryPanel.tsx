import { DownloadInfo } from "../types";
import { formatBytes } from "../utils";

interface HistoryPanelProps {
  history: DownloadInfo[];
  clearHistory: () => void;
  removeFromHistory: (id: string) => void;
}

export function HistoryPanel({ history, clearHistory, removeFromHistory }: HistoryPanelProps) {
  const finishedDownloads = history.filter(
    (h) => h.status === "Completed" || h.status === "Failed" || h.status === "Cancelled"
  );

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
