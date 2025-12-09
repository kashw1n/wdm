export interface DownloadItem {
  id: string;
  url: string;
  filename: string;
  totalSize: number;
  downloaded: number;
  speed: number;
  progress: number;
  status: 'downloading' | 'paused' | 'error' | 'completed' | 'cancelled' | 'merging' | 'starting';
  created_at: number;
  // Video specific
  type?: 'file' | 'video';
  thumbnail?: string | null;
  videoTitle?: string;
  eta?: number | null;
  completedPath?: string; // Add this as it is used in App.tsx
  error?: string; // Add this as it is used in App.tsx
}

export interface ChunkProgress {
  id: number;
  downloaded: number;
  total: number;
}

export interface DownloadProgress {
  id: string;
  downloaded: number;
  total: number;
  speed: number;
  status: string;
  chunk_progress: ChunkProgress[];
  // Optional extras from video downloader
  eta?: number;
  percent?: number;
}

export interface DownloadComplete {
  id: string;
  path: string;
  filename: string;
  total_size: number;
}

export interface DownloadError {
  id: string;
  error: string;
}

export interface UrlInfo {
  url: string;
  filename: string;
  size: number | null;
  content_type?: string;
  accept_ranges?: boolean;
  resumable: boolean;
}

export interface FileExistsInfo {
  exists: boolean;
  suggested_name: string;
}

export interface DownloadHistoryItem {
  id: string;
  url: string;
  filename: string;
  path: string;
  size: number;
  status: string;
  resumable: boolean;
  created_at: number;
}

export interface DownloadInfo {
  id: string;
  url: string;
  filename: string;
  file_path: string;
  total_size: number;
  downloaded: number;
  status: string;
  resumable: boolean;
  created_at: number;
}

// Video types
export interface VideoFormat {
  format_id: string;
  ext: string;
  resolution: string | null;
  filesize: number | null;
  filesize_approx: number | null;
  vcodec: string | null;
  acodec: string | null;
  fps: number | null;
  tbr: number | null;
  format_note: string | null;
}

export interface VideoInfo {
  url: string;
  title: string;
  duration: number | null;
  thumbnail: string | null;
  uploader: string | null;
  view_count: number | null;
  formats: VideoFormat[];
  best_format: string | null;
}