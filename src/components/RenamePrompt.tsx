interface RenamePromptProps {
  originalName: string;
  customFilename: string;
  setCustomFilename: (name: string) => void;
  onConfirm: () => void;
  onCancel: () => void;
}

export function RenamePrompt({
  originalName,
  customFilename,
  setCustomFilename,
  onConfirm,
  onCancel,
}: RenamePromptProps) {
  return (
    <div className="card border-amber-500/30 bg-amber-500/5 animate-fade-in">
      <div className="flex items-start gap-3 mb-4">
        <div className="w-10 h-10 rounded-lg bg-amber-500/20 flex items-center justify-center flex-shrink-0">
          <svg className="w-5 h-5 text-amber-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
          </svg>
        </div>
        <div>
          <h3 className="font-semibold text-amber-400">File Already Exists</h3>
          <p className="text-sm text-gray-400 mt-1">
            A file named <span className="text-gray-200 font-medium">{originalName}</span> already exists in your Downloads folder.
          </p>
        </div>
      </div>

      <div className="space-y-3">
        <div>
          <label className="text-sm text-gray-400 mb-1.5 block">Save as:</label>
          <input
            type="text"
            value={customFilename}
            onChange={(e) => setCustomFilename(e.target.value)}
            placeholder="Enter new filename"
            className="input"
            autoFocus
          />
        </div>

        <div className="flex gap-2 pt-2">
          <button
            onClick={onConfirm}
            disabled={!customFilename.trim()}
            className="btn-primary flex-1"
          >
            <svg className="w-4 h-4 mr-2 inline-block" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" />
            </svg>
            Download
          </button>
          <button onClick={onCancel} className="btn-secondary">
            Cancel
          </button>
        </div>
      </div>
    </div>
  );
}
