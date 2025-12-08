import { UrlInfo } from "../types";
import { formatBytes } from "../utils";

interface FileInfoProps {
  urlInfo: UrlInfo;
  onClose: () => void;
  onDownload: () => void;
}

export function FileInfo({ urlInfo, onClose, onDownload }: FileInfoProps) {
  return (
    <div className="file-info">
      <div className="file-info-header">
        <h3>File Info</h3>
        <button className="close-btn" onClick={onClose} title="Close">
          ×
        </button>
      </div>
      <p>
        <strong>Filename:</strong> {urlInfo.filename}
      </p>
      <p>
        <strong>Size:</strong> {urlInfo.size ? formatBytes(urlInfo.size) : "Unknown"}
      </p>
      <p>
        <strong>Resumable:</strong>{" "}
        {urlInfo.resumable ? "Yes (multi-connection)" : "No (single connection)"}
      </p>
      <button onClick={onDownload} className="download-btn">
        Download
      </button>
    </div>
  );
}
