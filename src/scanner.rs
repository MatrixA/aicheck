use anyhow::Result;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::i18n;

/// Supported MIME types for analysis.
const SUPPORTED_TYPES: &[&str] = &[
    "image/jpeg",
    "image/png",
    "image/webp",
    "image/avif",
    "image/heif",
    "image/tiff",
    "image/gif",
    "image/bmp",
    "video/mp4",
    "video/quicktime",
    "video/x-msvideo",
    "video/webm",
    "audio/mpeg",
    "audio/mp4",
    "audio/wav",
    "audio/x-wav",
    "application/pdf",
];

fn is_supported(path: &Path) -> bool {
    // Try magic-byte detection first
    if let Ok(Some(kind)) = infer::get_from_path(path) {
        if SUPPORTED_TYPES.contains(&kind.mime_type()) {
            return true;
        }
    }
    // Fall back to extension for types infer might miss
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        matches!(
            ext.to_lowercase().as_str(),
            "jpg"
                | "jpeg"
                | "png"
                | "webp"
                | "avif"
                | "heif"
                | "heic"
                | "tiff"
                | "tif"
                | "gif"
                | "bmp"
                | "mp4"
                | "mov"
                | "avi"
                | "webm"
                | "mp3"
                | "m4a"
                | "wav"
                | "pdf"
        )
    } else {
        false
    }
}

/// Discover files to analyze from the given paths.
pub fn discover_files(paths: &[PathBuf], recursive: bool) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    for path in paths {
        if path.is_file() {
            files.push(path.clone());
        } else if path.is_dir() {
            let walker = if recursive {
                WalkDir::new(path)
            } else {
                WalkDir::new(path).max_depth(1)
            };

            for entry in walker.into_iter().filter_map(|e| e.ok()) {
                let p = entry.path();
                if p.is_file() && is_supported(p) {
                    files.push(p.to_path_buf());
                }
            }
        } else {
            eprintln!(
                "{}",
                i18n::t(
                    "scanner_not_found",
                    &[("path", &path.display().to_string())]
                )
            );
        }
    }

    files.sort();
    Ok(files)
}
