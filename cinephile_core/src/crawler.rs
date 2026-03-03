use std::path::PathBuf;
use walkdir::WalkDir;
use mime_guess;
use futures_util::stream::{self, Stream};

pub fn scan_directories(paths: Vec<PathBuf>) -> impl Stream<Item = PathBuf> {
    let mut files = Vec::new();

    for path in paths {
        for entry in WalkDir::new(path)
            .into_iter()
            .filter_entry(|e| !is_hidden(e))
            .filter_map(|e| e.ok()) 
        {
            if entry.file_type().is_file() {
                if is_video_file(entry.path()) {
                    files.push(entry.path().to_path_buf());
                }
            }
        }
    }

    stream::iter(files)
}

fn is_hidden(entry: &walkdir::DirEntry) -> bool {
    entry.file_name()
         .to_str()
         .map(|s| s.starts_with('.'))
         .unwrap_or(false)
}

fn is_video_file(path: &std::path::Path) -> bool {
    let extensions = ["mkv", "mp4", "avi", "mov"];
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        if extensions.contains(&ext.to_lowercase().as_str()) {
            return true;
        }
    }
    
    // Fallback to mime_guess
    if let Some(mime) = mime_guess::from_path(path).first() {
        return mime.type_() == "video";
    }

    false
}
