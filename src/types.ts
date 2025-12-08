export interface UrlInfo {
  url: string;
  filename: string;
  size: number | null;
  resumable: boolean;
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

export interface Download {
  id: string;
  filename: string;
  url: string;
  size: number;
  progress: DownloadProgress | null;
  completed: boolean;
  completedPath?: string;
  error?: string;
}

export interface FileExistsInfo {
  exists: boolean;
  suggested_name: string;
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