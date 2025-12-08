import { DownloadInfo } from "../types";
import { formatBytes } from "../utils";
import { invoke } from "@tauri-apps/api/core";

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

  function getStatusBadge(status: string) {
    switch (status) {
      case "Completed":
        return <span className="badge badge-success">Completed</span>;
      case "Failed":
        return <span className="badge badge-danger">Failed</span>;
      case "Cancelled":
        return <span className="badge badge-neutral">Cancelled</span>;
      default:
        return <span className="badge badge-info">{status}</span>;
    }
  }

  return (
    <div className="panel">
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-3">
          <div className="w-8 h-8 rounded-lg bg-dark-700 flex items-center justify-center">
            <svg className="w-4 h-4 text-accent" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
          </div>
          <h3 className="font-semibold text-gray-100">Download History</h3>
        </div>
        {finishedDownloads.length > 0 && (
          <button onClick={clearHistory} className="btn-danger text-sm">
            <svg className="w-4 h-4 mr-1 inline-block" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
            </svg>
            Clear All
          </button>
        )}
      </div>

      {finishedDownloads.length === 0 ? (
        <div className="text-center py-8">
          <svg className="w-12 h-12 mx-auto text-gray-600 mb-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
          </svg>
          <p className="text-gray-500 text-sm">No download history yet</p>
        </div>
      ) : (
        <div className="space-y-2 max-h-80 overflow-y-auto">
          {finishedDownloads.map((item) => (
            <div
              key={item.id}
              className="flex items-center justify-between p-3 bg-dark-700/50 rounded-lg hover:bg-dark-700 transition-colors group"
            >
              <div className="flex-1 min-w-0 mr-4">
                <p className="font-medium text-gray-200 truncate text-sm">{item.filename}</p>
                <div className="flex items-center gap-2 mt-1">
                  <span className="text-xs text-gray-500">{formatBytes(item.total_size)}</span>
                  {getStatusBadge(item.status)}
                </div>
              </div>
              <div className="flex gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                {item.status === "Completed" && (
                  <>
                    <button
                      onClick={() => openFile(item.file_path)}
                      className="p-2 text-gray-400 hover:text-accent hover:bg-dark-600 rounded-lg transition-colors"
                      title="Open file"
                    >
                      <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14" />
                      </svg>
                    </button>
                    <button
                      onClick={() => showInFolder(item.file_path)}
                      className="p-2 text-gray-400 hover:text-accent hover:bg-dark-600 rounded-lg transition-colors"
                      title="Show in folder"
                    >
                      <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
                      </svg>
                    </button>
                  </>
                )}
                <button
                  onClick={() => removeFromHistory(item.id)}
                  className="p-2 text-gray-400 hover:text-red-400 hover:bg-dark-600 rounded-lg transition-colors"
                  title="Remove from history"
                >
                  <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                  </svg>
                </button>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
