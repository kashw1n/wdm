interface AddDownloadFormProps {
  url: string;
  setUrl: (url: string) => void;
  loading: boolean;
  onSubmit: () => void;
}

export function AddDownloadForm({ url, setUrl, loading, onSubmit }: AddDownloadFormProps) {
  return (
    <form
      className="row"
      onSubmit={(e) => {
        e.preventDefault();
        onSubmit();
      }}
    >
      <input
        value={url}
        onChange={(e) => setUrl(e.currentTarget.value)}
        placeholder="Enter URL to download..."
        style={{ flex: 1 }}
      />
      <button type="submit" disabled={loading}>
        {loading ? "Checking..." : "Check URL"}
      </button>
    </form>
  );
}
