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
    <div className="card card-hover">
      {/* Header */}
      <div className="flex items-center justify-between mb-3 gap-2">
        <div className="flex items-center gap-2 sm:gap-3 flex-1 min-w-0">
          <div className="w-7 h-7 sm:w-8 sm:h-8 rounded-lg bg-accent/10 flex items-center justify-center flex-shrink-0">
            {isPaused ? (
              <svg className="w-3.5 h-3.5 sm:w-4 sm:h-4 text-amber-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10 9v6m4-6v6m7-3a9 9 0 11-18 0 9 9 0 0118 0z" />
              </svg>
            ) : (
              <svg className="w-3.5 h-3.5 sm:w-4 sm:h-4 text-accent animate-pulse" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" />
              </svg>
            )}
          </div>
          <span className="font-medium text-gray-200 truncate text-sm sm:text-base">{download.filename}</span>
        </div>
        <div className="flex gap-1.5 sm:gap-2 flex-shrink-0">
          {isPaused ? (
            <button onClick={onResume} className="btn-success text-xs sm:text-sm px-2 sm:px-3" title="Resume">
              <svg className="w-3.5 h-3.5 sm:w-4 sm:h-4 sm:mr-1 inline-block" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z" />
              </svg>
              <span className="hidden sm:inline">Resume</span>
            </button>
          ) : (
            <button onClick={onPause} className="btn-warning text-xs sm:text-sm px-2 sm:px-3" title="Pause">
              <svg className="w-3.5 h-3.5 sm:w-4 sm:h-4 sm:mr-1 inline-block" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10 9v6m4-6v6" />
              </svg>
              <span className="hidden sm:inline">Pause</span>
            </button>
          )}
          <button onClick={onCancel} className="btn-danger text-xs sm:text-sm px-2 sm:px-3" title="Cancel">
            <svg className="w-3.5 h-3.5 sm:w-4 sm:h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>
      </div>

      {progress && (
        <>
          {/* Progress Bar */}
          <div className="relative">
            <div className="progress-bar">
              <div
                className="progress-fill"
                style={{ width: `${percent}%` }}
              />
            </div>
            {/* Glow effect when downloading */}
            {!isPaused && (
              <div
                className="absolute top-0 left-0 h-full rounded-full bg-accent/20 blur-sm transition-all duration-150"
                style={{ width: `${percent}%` }}
              />
            )}
          </div>

          {/* Stats */}
          <div className="flex flex-col sm:flex-row sm:items-center justify-between mt-2 sm:mt-3 text-xs sm:text-sm gap-1 sm:gap-0">
            <div className="flex items-center gap-2 sm:gap-4">
              <span className="text-gray-400">
                {formatBytes(progress.downloaded)}
                <span className="text-gray-600 mx-1">/</span>
                {formatBytes(progress.total)}
              </span>
              <span className="text-accent font-medium">{percent}%</span>
            </div>
            <div className="flex items-center gap-2">
              {isPaused ? (
                <span className="badge badge-warning">Paused</span>
              ) : (
                <span className="text-gray-300 font-medium">
                  <svg className="w-3 h-3 sm:w-3.5 sm:h-3.5 mr-1 inline-block text-accent" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 7h8m0 0v8m0-8l-8 8-4-4-6 6" />
                  </svg>
                  {formatSpeed(progress.speed)}
                </span>
              )}
            </div>
          </div>

          {/* Chunk Progress */}
          {progress.chunk_progress.length > 1 && (
            <div className="mt-2 sm:mt-3 pt-2 sm:pt-3 border-t border-dark-600">
              <div className="flex items-center gap-1 mb-1.5 sm:mb-2">
                <svg className="w-3 h-3 text-gray-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 6h16M4 12h16M4 18h16" />
                </svg>
                <span className="text-xs text-gray-500">{progress.chunk_progress.length} connections</span>
              </div>
              <div className="flex gap-0.5 sm:gap-1">
                {progress.chunk_progress.map((chunk) => (
                  <div key={chunk.id} className="flex-1">
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
