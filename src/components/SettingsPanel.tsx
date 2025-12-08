interface SettingsPanelProps {
  connections: number;
  downloadFolder: string;
  speedLimit: number;
  updateConnections: (value: number) => void;
  selectDownloadFolder: () => void;
  resetDownloadFolder: () => void;
  updateSpeedLimit: (limit: number) => void;
}

export function SettingsPanel({
  connections,
  downloadFolder,
  speedLimit,
  updateConnections,
  selectDownloadFolder,
  resetDownloadFolder,
  updateSpeedLimit,
}: SettingsPanelProps) {
  return (
    <div className="settings-panel">
      <h3>Settings</h3>
      <div className="setting-row">
        <label>Connections per download:</label>
        <select
          value={connections}
          onChange={(e) => updateConnections(Number(e.target.value))}
        >
          {[1, 2, 4, 8, 16, 32].map((n) => (
            <option key={n} value={n}>
              {n}
            </option>
          ))}
        </select>
      </div>
      <div className="setting-row folder-row">
        <label>Download folder:</label>
        <span className="folder-path" title={downloadFolder}>
          {downloadFolder}
        </span>
        <button className="folder-btn" onClick={selectDownloadFolder}>
          Browse
        </button>
        <button className="folder-reset-btn" onClick={resetDownloadFolder}>
          Reset
        </button>
      </div>
      <div className="setting-row">
        <label>Speed limit:</label>
        <select
          value={speedLimit}
          onChange={(e) => updateSpeedLimit(Number(e.target.value))}
        >
          <option value={0}>Unlimited</option>
          <option value={512 * 1024}>512 KB/s</option>
          <option value={1024 * 1024}>1 MB/s</option>
          <option value={2 * 1024 * 1024}>2 MB/s</option>
          <option value={5 * 1024 * 1024}>5 MB/s</option>
          <option value={10 * 1024 * 1024}>10 MB/s</option>
          <option value={20 * 1024 * 1024}>20 MB/s</option>
          <option value={50 * 1024 * 1024}>50 MB/s</option>
        </select>
      </div>
    </div>
  );
}
