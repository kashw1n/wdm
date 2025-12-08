use std::path::PathBuf;

pub fn extract_filename_from_url(url: &str) -> Option<String> {
    url.split('?').next()
        .and_then(|path| path.split('/').last())
        .filter(|s| !s.is_empty() && s.contains('.'))
        .map(|s| s.to_string())
}

pub fn generate_unique_filename(dir: &PathBuf, filename: &str) -> String {
    let path = std::path::Path::new(filename);
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");

    let mut counter = 1;
    loop {
        let new_name = if ext.is_empty() {
            format!("{} ({})", stem, counter)
        } else {
            format!("{} ({}).{}", stem, counter, ext)
        };

        if !dir.join(&new_name).exists() {
            return new_name;
        }
        counter += 1;
    }
}
