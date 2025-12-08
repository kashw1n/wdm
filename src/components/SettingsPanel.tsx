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
    <div className="panel">
      <div className="flex items-center gap-2 sm:gap-3 mb-4">
        <div className="w-7 h-7 sm:w-8 sm:h-8 rounded-lg bg-dark-700 flex items-center justify-center">
          <svg className="w-3.5 h-3.5 sm:w-4 sm:h-4 text-accent" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
          </svg>
        </div>
        <h3 className="font-semibold text-gray-100 text-sm sm:text-base">Settings</h3>
      </div>

      <div className="space-y-4">
        {/* Connections */}
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-2 sm:gap-4">
          <div className="flex items-center gap-2 sm:gap-3">
            <svg className="w-4 h-4 text-gray-500 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 10V3L4 14h7v7l9-11h-7z" />
            </svg>
            <div>
              <label className="text-xs sm:text-sm text-gray-300">Connections per download</label>
              <p className="text-xs text-gray-500 hidden sm:block">More connections = faster downloads</p>
            </div>
          </div>
          <select
            value={connections}
            onChange={(e) => updateConnections(Number(e.target.value))}
            className="select text-sm flex-shrink-0"
          >
            {[1, 2, 4, 8, 16, 32].map((n) => (
              <option key={n} value={n}>
                {n} {n === 1 ? 'connection' : 'connections'}
              </option>
            ))}
          </select>
        </div>

        {/* Speed Limit */}
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-2 sm:gap-4">
          <div className="flex items-center gap-2 sm:gap-3">
            <svg className="w-4 h-4 text-gray-500 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 7h8m0 0v8m0-8l-8 8-4-4-6 6" />
            </svg>
            <div>
              <label className="text-xs sm:text-sm text-gray-300">Speed limit</label>
              <p className="text-xs text-gray-500 hidden sm:block">Limit bandwidth usage</p>
            </div>
          </div>
          <select
            value={speedLimit}
            onChange={(e) => updateSpeedLimit(Number(e.target.value))}
            className="select text-sm flex-shrink-0"
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

        {/* Download Folder */}
        <div className="space-y-2">
          <div className="flex items-center gap-2 sm:gap-3">
            <svg className="w-4 h-4 text-gray-500 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
            </svg>
            <label className="text-xs sm:text-sm text-gray-300">Download folder</label>
          </div>
          <div className="flex flex-col sm:flex-row gap-2">
            <div className="flex-1 px-3 py-2 bg-dark-700 rounded-lg text-xs sm:text-sm text-gray-400 truncate border border-dark-600 min-w-0">
              {downloadFolder}
            </div>
            <div className="flex gap-2 flex-shrink-0">
              <button onClick={selectDownloadFolder} className="btn-secondary text-xs sm:text-sm flex-1 sm:flex-none">
                Browse
              </button>
              <button onClick={resetDownloadFolder} className="btn-ghost text-xs sm:text-sm flex-1 sm:flex-none">
                Reset
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
