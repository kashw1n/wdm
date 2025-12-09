import { DownloadItem as Download } from "../types";
import { formatBytes, formatSpeed } from "../utils";

interface DownloadItemProps {
  download: Download;
  onPause: () => void;
  onResume: () => void;
  onCancel: () => void;
}

function formatEta(seconds: number | null | undefined): string {
  if (!seconds) return "";
  const m = Math.floor(seconds / 60);
  const s = Math.floor(seconds % 60);
  if (m > 0) {
    return `${m}m ${s}s`;
  }
  return `${s}s`;
}

export function DownloadItem({ download, onPause, onResume, onCancel }: DownloadItemProps) {
  const isVideo = download.type === 'video';
  // Use progress directly if it's a number (for video/unified), or calculate from downloaded/total
  const percent = typeof download.progress === 'number' 
    ? download.progress.toFixed(1) 
    : (download.totalSize > 0 
        ? ((download.downloaded / download.totalSize) * 100).toFixed(1) 
        : "0");
        
  const isPaused = download.status === "paused";
  const isMerging = download.status === "merging";
  const isStarting = download.status === "starting";

  return (
    <div className="card card-hover">
      {/* Header */}
      <div className="flex items-center justify-between mb-3 gap-2">
        <div className="flex items-center gap-2 sm:gap-3 flex-1 min-w-0">
          
          {/* Icon or Thumbnail */}
          {download.thumbnail ? (
            <div className="w-10 h-7 sm:w-12 sm:h-8 rounded overflow-hidden bg-dark-700 flex-shrink-0">
              <img
                src={download.thumbnail}
                alt=""
                className="w-full h-full object-cover"
              />
            </div>
          ) : (
            <div className="w-7 h-7 sm:w-8 sm:h-8 rounded-lg bg-accent/10 flex items-center justify-center flex-shrink-0">
              {isVideo ? (
                 <svg className="w-3.5 h-3.5 sm:w-4 sm:h-4 text-red-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z" />
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                </svg>
              ) : isPaused ? (
                <svg className="w-3.5 h-3.5 sm:w-4 sm:h-4 text-amber-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10 9v6m4-6v6m7-3a9 9 0 11-18 0 9 9 0 0118 0z" />
                </svg>
              ) : (
                <svg className="w-3.5 h-3.5 sm:w-4 sm:h-4 text-accent animate-pulse" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" />
                </svg>
              )}
            </div>
          )}
          
          <span className="font-medium text-gray-200 truncate text-sm sm:text-base">
            {download.videoTitle || download.filename}
          </span>
        </div>
        
        <div className="flex gap-1.5 sm:gap-2 flex-shrink-0">
          {!isVideo && (
            <>
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
            </>
          )}
          
          <button onClick={onCancel} className="btn-danger text-xs sm:text-sm px-2 sm:px-3" title="Cancel">
            <svg className="w-3.5 h-3.5 sm:w-4 sm:h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>
      </div>

      <>
        {/* Progress Bar */}
        <div className="relative">
          <div className="progress-bar">
            <div
              className={`progress-fill ${isStarting ? 'animate-pulse' : ''}`}
              style={{ width: isStarting ? '100%' : `${percent}%`, opacity: isStarting ? 0.3 : 1 }}
            />
          </div>
          {/* Glow effect when downloading */}
          {!isPaused && !isStarting && (
            <div
              className="absolute top-0 left-0 h-full rounded-full bg-accent/20 blur-sm transition-all duration-150"
              style={{ width: `${percent}%` }}
            />
          )}
        </div>

        {/* Stats */}
        <div className="flex flex-col sm:flex-row sm:items-center justify-between mt-2 sm:mt-3 text-xs sm:text-sm gap-1 sm:gap-0">
          
          {/* Status Text for special states */}
          {isStarting ? (
            <span className="text-gray-400 flex items-center gap-2">
              <svg className="w-4 h-4 animate-spin" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
              </svg>
              Starting download...
            </span>
          ) : isMerging ? (
             <span className="text-gray-400 flex items-center gap-2">
              <svg className="w-4 h-4 animate-spin" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
              </svg>
              Merging video and audio...
            </span>
          ) : (
             <div className="flex items-center gap-2 sm:gap-4">
              <span className="text-gray-400">
                {formatBytes(download.downloaded)}
                {download.totalSize > 0 && (
                  <>
                    <span className="text-gray-600 mx-1">/</span>
                    {formatBytes(download.totalSize)}
                  </>
                )}
              </span>
              <span className="text-accent font-medium">{percent}%</span>
            </div>
          )}

          <div className="flex items-center gap-2">
            {isPaused ? (
              <span className="badge badge-warning">Paused</span>
            ) : (
              <>
               {!isStarting && !isMerging && (
                <span className="text-gray-300 font-medium">
                  <svg className="w-3 h-3 sm:w-3.5 sm:h-3.5 mr-1 inline-block text-accent" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 7h8m0 0v8m0-8l-8 8-4-4-6 6" />
                  </svg>
                  {formatSpeed(download.speed)}
                </span>
               )}
               {download.eta !== undefined && download.eta !== null && (
                 <span className="text-gray-500 ml-2">
                    ETA: {formatEta(download.eta)}
                 </span>
               )}
              </>
            )}
          </div>
        </div>

        {/* Chunk Progress (Files only) */}
        {!isVideo && (download as any).progress?.chunk_progress?.length > 1 && (
          <div className="mt-2 sm:mt-3 pt-2 sm:pt-3 border-t border-dark-600">
            <div className="flex items-center gap-1 mb-1.5 sm:mb-2">
              <svg className="w-3 h-3 text-gray-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 6h16M4 12h16M4 18h16" />
              </svg>
              <span className="text-xs text-gray-500">{(download as any).progress.chunk_progress.length} connections</span>
            </div>
            <div className="flex gap-0.5 sm:gap-1">
              {(download as any).progress.chunk_progress.map((chunk: any) => (
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
    </div>
  );
}
