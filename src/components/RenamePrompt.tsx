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
    <div className="rename-prompt">
      <h3>File Already Exists</h3>
      <p>
        A file named <strong>{originalName}</strong> already exists
        in your Downloads folder.
      </p>
      <div className="rename-input-row">
        <label>Save as:</label>
        <input
          type="text"
          value={customFilename}
          onChange={(e) => setCustomFilename(e.target.value)}
          placeholder="Enter new filename"
        />
      </div>
      <div className="rename-buttons">
        <button onClick={onConfirm} className="confirm-btn">
          Download
        </button>
        <button onClick={onCancel} className="cancel-btn">
          Cancel
        </button>
      </div>
    </div>
  );
}
