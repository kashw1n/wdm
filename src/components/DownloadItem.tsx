import { Download } from "../types";
import { formatBytes, formatSpeed } from "../utils";

interface DownloadItemProps {
  download: Download;
  onPause: () => void;
  onResume: () => void;
  onCancel: () => void;
}

export function DownloadItem({ download, onPause, onResume, onCancel }: DownloadItemProps) {
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
