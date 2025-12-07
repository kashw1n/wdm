import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

interface UrlInfo {
  url: string;
  filename: string;
  size: number | null;
  resumable: boolean;
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return bytes + " B";
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(2) + " KB";
  if (bytes < 1024 * 1024 * 1024) return (bytes / (1024 * 1024)).toFixed(2) + " MB";
  return (bytes / (1024 * 1024 * 1024)).toFixed(2) + " GB";
}

function App() {
  const [url, setUrl] = useState("");
  const [urlInfo, setUrlInfo] = useState<UrlInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  async function fetchInfo() {
    if (!url.trim()) return;

    setLoading(true);
    setError(null);
    setUrlInfo(null);

    try {
      const info = await invoke<UrlInfo>("fetch_url_info", { url });
      setUrlInfo(info);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  return (
    <main className="container">
      <h1>Download Manager</h1>

      <form
        className="row"
        onSubmit={(e) => {
          e.preventDefault();
          fetchInfo();
        }}
      >
        <input
          value={url}
          onChange={(e) => setUrl(e.currentTarget.value)}
          placeholder="Enter URL to analyze..."
          style={{ flex: 1 }}
        />
        <button type="submit" disabled={loading}>
          {loading ? "Checking..." : "Check URL"}
        </button>
      </form>

      {error && <p style={{ color: "red" }}>{error}</p>}

      {urlInfo && (
        <div style={{ marginTop: "1rem", textAlign: "left" }}>
          <h3>File Info</h3>
          <p><strong>Filename:</strong> {urlInfo.filename}</p>
          <p><strong>Size:</strong> {urlInfo.size ? formatBytes(urlInfo.size) : "Unknown"}</p>
          <p><strong>Resumable:</strong> {urlInfo.resumable ? "Yes" : "No"}</p>
        </div>
      )}
    </main>
  );
}

export default App;
