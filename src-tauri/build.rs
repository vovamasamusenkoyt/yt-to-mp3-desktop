use std::fs;
use std::path::Path;

fn main() {
    // Ensure placeholder files exist if fetch_bins.sh hasn't been run yet
    let bin_dir = Path::new("bin");
    let _ = fs::create_dir_all(bin_dir);
    for binary in &["yt-dlp.exe", "ffmpeg.exe"] {
        let p = bin_dir.join(binary);
        if !p.exists() {
            let _ = fs::write(&p, b"placeholder_binary");
        }
    }

    tauri_build::build();
}
