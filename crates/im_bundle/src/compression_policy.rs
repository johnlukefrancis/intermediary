// Path: crates/im_bundle/src/compression_policy.rs
// Description: Compression policy for bundle entries based on extension and size

use std::path::Path;

use zip::CompressionMethod;

const STORE_THRESHOLD_BYTES: u64 = 32 * 1024 * 1024;

const STORED_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "webp", "gif", "zip", "gz", "tgz", "bz2", "xz", "zst", "7z", "rar",
    "pdf", "mp4", "m4v", "mov", "mkv", "mp3", "m4a", "aac", "ogg", "flac", "wav", "woff2", "woff",
    "ttf", "otf", "ico", "heic", "avif",
];

const STORED_SUFFIXES: &[&str] = &[".tar.gz", ".tar.bz2", ".tar.xz", ".tar.zst"];

pub fn compression_method_for(archive_path: &str, size_bytes: u64) -> CompressionMethod {
    if size_bytes >= STORE_THRESHOLD_BYTES {
        return CompressionMethod::Stored;
    }

    let lower = archive_path.to_ascii_lowercase();
    for suffix in STORED_SUFFIXES {
        if lower.ends_with(suffix) {
            return CompressionMethod::Stored;
        }
    }

    let ext = Path::new(&lower)
        .extension()
        .and_then(|value| value.to_str());
    if let Some(ext) = ext {
        if STORED_EXTENSIONS.contains(&ext) {
            return CompressionMethod::Stored;
        }
    }

    CompressionMethod::Deflated
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stores_for_known_compressed_extensions() {
        assert_eq!(
            compression_method_for("image.PNG", 10),
            CompressionMethod::Stored
        );
        assert_eq!(
            compression_method_for("video.mp4", 10),
            CompressionMethod::Stored
        );
        assert_eq!(
            compression_method_for("fonts/font.woff2", 10),
            CompressionMethod::Stored
        );
        assert_eq!(
            compression_method_for("archive.tar.gz", 10),
            CompressionMethod::Stored
        );
    }

    #[test]
    fn deflates_for_code_by_default() {
        assert_eq!(
            compression_method_for("src/main.rs", 1024),
            CompressionMethod::Deflated
        );
        assert_eq!(
            compression_method_for("docs/readme.md", 1024),
            CompressionMethod::Deflated
        );
    }

    #[test]
    fn stores_for_large_files_even_if_text() {
        let size = STORE_THRESHOLD_BYTES + 1;
        assert_eq!(
            compression_method_for("logs/big.txt", size),
            CompressionMethod::Stored
        );
    }
}
