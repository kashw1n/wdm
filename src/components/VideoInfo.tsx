import { useState } from "react";
import { VideoInfo as VideoInfoType, VideoFormat } from "../types";
import { formatBytes } from "../utils";

interface VideoInfoProps {
  info: VideoInfoType;
  onDownload: (formatId: string) => void;
  onCancel: () => void;
}

function formatDuration(seconds: number | null): string {
  if (!seconds) return "Unknown";
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = Math.floor(seconds % 60);
  if (h > 0) {
    return `${h}:${m.toString().padStart(2, "0")}:${s.toString().padStart(2, "0")}`;
  }
  return `${m}:${s.toString().padStart(2, "0")}`;
}

function formatViews(count: number | null): string {
  if (!count) return "";
  if (count >= 1_000_000) {
    return `${(count / 1_000_000).toFixed(1)}M views`;
  }
  if (count >= 1_000) {
    return `${(count / 1_000).toFixed(1)}K views`;
  }
  return `${count} views`;
}

function getFormatLabel(format: VideoFormat): string {
  if (format.format_id === "best") {
    return "Best Quality";
  }
  const parts: string[] = [];
  if (format.resolution) {
    parts.push(format.resolution);
  }
  if (format.format_note) {
    parts.push(`(${format.format_note})`);
  }
  return parts.join(" ") || format.format_id;
}

function getFormatSize(format: VideoFormat): string {
  const size = format.filesize || format.filesize_approx;
  if (!size) return "";
  return formatBytes(size);
}

export function VideoInfoComponent({ info, onDownload, onCancel }: VideoInfoProps) {
  const [selectedFormat, setSelectedFormat] = useState(info.best_format || "best");

  return (
    <div className="panel">
      <div className="flex items-start gap-4">
        {/* Thumbnail */}
        {info.thumbnail && (
          <div className="w-32 h-20 sm:w-40 sm:h-24 flex-shrink-0 rounded-lg overflow-hidden bg-dark-700">
            <img
              src={info.thumbnail}
              alt={info.title}
              className="w-full h-full object-cover"
            />
          </div>
        )}

        {/* Info */}
        <div className="flex-1 min-w-0">
          <h3 className="font-semibold text-gray-100 text-sm sm:text-base line-clamp-2 mb-2">
            {info.title}
          </h3>
          <div className="flex flex-wrap items-center gap-2 text-xs sm:text-sm text-gray-400">
            {info.uploader && (
              <span className="flex items-center gap-1">
                <svg className="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z" />
                </svg>
                {info.uploader}
              </span>
            )}
            {info.duration && (
              <span className="flex items-center gap-1">
                <svg className="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
                </svg>
                {formatDuration(info.duration)}
              </span>
            )}
            {info.view_count && (
              <span className="flex items-center gap-1">
                <svg className="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z" />
                </svg>
                {formatViews(info.view_count)}
              </span>
            )}
          </div>
        </div>
      </div>

      {/* Format Selector */}
      <div className="mt-4 pt-4 border-t border-dark-600">
        <div className="flex flex-col sm:flex-row sm:items-center gap-3">
          <div className="flex items-center gap-2 flex-1">
            <svg className="w-4 h-4 text-gray-500 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M7 4v16M17 4v16M3 8h4m10 0h4M3 12h18M3 16h4m10 0h4M4 20h16a1 1 0 001-1V5a1 1 0 00-1-1H4a1 1 0 00-1 1v14a1 1 0 001 1z" />
            </svg>
            <select
              value={selectedFormat}
              onChange={(e) => setSelectedFormat(e.target.value)}
              className="select flex-1 text-sm"
            >
              {info.formats.map((format) => (
                <option key={format.format_id} value={format.format_id}>
                  {getFormatLabel(format)}
                  {getFormatSize(format) && ` - ${getFormatSize(format)}`}
                </option>
              ))}
            </select>
          </div>

          <div className="flex gap-2">
            <button
              onClick={onCancel}
              className="btn-ghost text-sm px-4"
            >
              Cancel
            </button>
            <button
              onClick={() => onDownload(selectedFormat)}
              className="btn-primary text-sm px-4"
            >
              <svg className="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" />
              </svg>
              Download
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
