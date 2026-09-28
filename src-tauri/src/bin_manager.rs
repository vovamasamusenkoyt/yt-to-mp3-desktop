use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

// Embed the Windows executables directly into the binary at compile-time
static YT_DLP_EXE_BYTES: &[u8] = include_bytes!("../bin/yt-dlp.exe");
static FFMPEG_EXE_BYTES: &[u8] = include_bytes!("../bin/ffmpeg.exe");

pub struct BinPaths {
    pub ytdlp: PathBuf,
    pub ffmpeg: PathBuf,
}

/// Prepares and returns the paths to yt-dlp and ffmpeg.
/// On Windows, extracts the embedded yt-dlp.exe and ffmpeg.exe to %TEMP%/yt-to-mp3-bin/
/// On Linux/Unix (development/testing), prefers system yt-dlp/ffmpeg if present.
pub fn ensure_binaries() -> Result<BinPaths, String> {
    let temp_dir = std::env::temp_dir().join("yt-to-mp3-bin");
    fs::create_dir_all(&temp_dir)
        .map_err(|e| format!("Failed to create temp directory for binaries: {}", e))?;

    let ytdlp_target = temp_dir.join(if cfg!(windows) { "yt-dlp.exe" } else { "yt-dlp" });
    let ffmpeg_target = temp_dir.join(if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" });

    // On Windows, always extract the embedded binaries if missing or different size
    if cfg!(windows) {
        extract_if_needed(&ytdlp_target, YT_DLP_EXE_BYTES)?;
        extract_if_needed(&ffmpeg_target, FFMPEG_EXE_BYTES)?;
        return Ok(BinPaths {
            ytdlp: ytdlp_target,
            ffmpeg: ffmpeg_target,
        });
    }

    // On non-Windows (Linux/macOS during testing), check system PATH first
    if let (Some(ytdlp_sys), Some(ffmpeg_sys)) = (which("yt-dlp"), which("ffmpeg")) {
        return Ok(BinPaths {
            ytdlp: ytdlp_sys,
            ffmpeg: ffmpeg_sys,
        });
    }

    // Check python venv from sibling scratch project if available
    let venv_ytdlp = PathBuf::from("/home/vmko/.gemini/antigravity/scratch/yt-to-mp3/venv/bin/yt-dlp");
    let sys_ffmpeg = PathBuf::from("/usr/bin/ffmpeg");
    if venv_ytdlp.exists() && sys_ffmpeg.exists() {
        return Ok(BinPaths {
            ytdlp: venv_ytdlp,
            ffmpeg: sys_ffmpeg,
        });
    }

    // Fallback: extract embedded files
    extract_if_needed(&ytdlp_target, YT_DLP_EXE_BYTES)?;
    extract_if_needed(&ffmpeg_target, FFMPEG_EXE_BYTES)?;

    Ok(BinPaths {
        ytdlp: ytdlp_target,
        ffmpeg: ffmpeg_target,
    })
}

fn extract_if_needed(target: &Path, bytes: &[u8]) -> Result<(), String> {
    if target.exists() {
        if let Ok(meta) = fs::metadata(target) {
            if meta.len() == bytes.len() as u64 {
                return Ok(()); // Already extracted and size matches
            }
        }
    }

    let mut file = File::create(target)
        .map_err(|e| format!("Failed to write embedded binary {:?}: {}", target, e))?;
    file.write_all(bytes)
        .map_err(|e| format!("Failed writing bytes to {:?}: {}", target, e))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(target)
            .map_err(|e| e.to_string())?
            .permissions();
        perms.set_mode(0o755);
        let _ = fs::set_permissions(target, perms);
    }

    Ok(())
}

fn which(binary_name: &str) -> Option<PathBuf> {
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let candidate = dir.join(binary_name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}
