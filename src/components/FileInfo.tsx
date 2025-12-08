import { UrlInfo } from "../types";
import { formatBytes } from "../utils";

interface FileInfoProps {
  urlInfo: UrlInfo;
  onClose: () => void;
  onDownload: () => void;
}

export function FileInfo({ urlInfo, onClose, onDownload }: FileInfoProps) {
  return (
    <div className="card animate-fade-in">
      <div className="flex items-start justify-between mb-4">
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 rounded-lg bg-dark-700 flex items-center justify-center">
            <svg className="w-5 h-5 text-accent" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
            </svg>
          </div>
          <h3 className="font-semibold text-gray-100">File Information</h3>
        </div>
        <button
          onClick={onClose}
          className="text-gray-500 hover:text-gray-300 transition-colors p-1 hover:bg-dark-600 rounded"
        >
          <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>

      <div className="space-y-3 mb-5">
        <div className="flex items-start gap-3">
          <span className="text-gray-500 text-sm w-20 flex-shrink-0">Filename</span>
          <span className="text-gray-200 text-sm break-all font-medium">{urlInfo.filename}</span>
        </div>
        <div className="flex items-center gap-3">
          <span className="text-gray-500 text-sm w-20 flex-shrink-0">Size</span>
          <span className="text-gray-200 text-sm">
            {urlInfo.size ? (
              <span className="badge badge-info">{formatBytes(urlInfo.size)}</span>
            ) : (
              <span className="text-gray-500">Unknown</span>
            )}
          </span>
        </div>
        <div className="flex items-center gap-3">
          <span className="text-gray-500 text-sm w-20 flex-shrink-0">Mode</span>
          {urlInfo.resumable ? (
            <span className="badge badge-success">
              <svg className="w-3 h-3 mr-1 inline-block" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 10V3L4 14h7v7l9-11h-7z" />
              </svg>
              Multi-connection
            </span>
          ) : (
            <span className="badge badge-warning">Single connection</span>
          )}
        </div>
      </div>

      <button onClick={onDownload} className="btn-primary w-full shadow-glow">
        <svg className="w-5 h-5 mr-2 inline-block" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" />
        </svg>
        Start Download
      </button>
    </div>
  );
}
