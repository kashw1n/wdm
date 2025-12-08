interface AddDownloadFormProps {
  url: string;
  setUrl: (url: string) => void;
  loading: boolean;
  onSubmit: () => void;
}

export function AddDownloadForm({ url, setUrl, loading, onSubmit }: AddDownloadFormProps) {
  return (
    <form
      onSubmit={(e) => {
        e.preventDefault();
        onSubmit();
      }}
      className="flex gap-2 sm:gap-3"
    >
      <div className="relative flex-1 min-w-0">
        <div className="absolute inset-y-0 left-0 pl-3 sm:pl-4 flex items-center pointer-events-none">
          <svg className="w-4 h-4 sm:w-5 sm:h-5 text-gray-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13.828 10.172a4 4 0 00-5.656 0l-4 4a4 4 0 105.656 5.656l1.102-1.101m-.758-4.899a4 4 0 005.656 0l4-4a4 4 0 00-5.656-5.656l-1.1 1.1" />
          </svg>
        </div>
        <input
          type="text"
          value={url}
          onChange={(e) => setUrl(e.currentTarget.value)}
          placeholder="Paste URL..."
          className="input pl-10 sm:pl-12 text-sm sm:text-base py-2.5 sm:py-3"
        />
      </div>
      <button
        type="submit"
        disabled={loading || !url.trim()}
        className="btn-primary whitespace-nowrap text-sm sm:text-base px-3 sm:px-4 flex-shrink-0"
      >
        {loading ? (
          <span className="flex items-center gap-1.5 sm:gap-2">
            <svg className="w-4 h-4 animate-spin" fill="none" viewBox="0 0 24 24">
              <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" />
              <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
            </svg>
            <span className="hidden sm:inline">Checking...</span>
          </span>
        ) : (
          <span className="flex items-center gap-1.5 sm:gap-2">
            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
            </svg>
            <span className="hidden sm:inline">Check URL</span>
          </span>
        )}
      </button>
    </form>
  );
}
