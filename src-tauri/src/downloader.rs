use std::process::Stdio;
use serde::{Deserialize, Serialize};
use regex::Regex;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tauri::{AppHandle, Emitter};

use crate::bin_manager::BinPaths;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VideoInfo {
    pub id: String,
    pub title: String,
    pub uploader: String,
    pub duration: Option<u64>,
    pub duration_str: String,
    pub thumbnail: Option<String>,
    pub view_count: Option<u64>,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProgressUpdate {
    pub percent: f32,
    pub stage: String,
    pub speed: String,
    pub eta: String,
    pub downloaded_size: String,
    pub total_size: String,
    pub finished: bool,
    pub output_path: Option<String>,
    pub error: Option<String>,
}

fn format_seconds(seconds: Option<u64>) -> String {
    match seconds {
        Some(s) => {
            let m = (s / 60) % 60;
            let h = s / 3600;
            let s_rem = s % 60;
            if h > 0 {
                format!("{:02}:{:02}:{:02}", h, m, s_rem)
            } else {
                format!("{:02}:{:02}", m, s_rem)
            }
        }
        None => "--:--".to_string(),
    }
}

pub async fn extract_video_info(bin_paths: &BinPaths, url: &str) -> Result<VideoInfo, String> {
    let mut cmd = Command::new(&bin_paths.ytdlp);
    cmd.arg("--dump-json")
        .arg("--skip-download")
        .arg("--no-playlist")
        .arg(url)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // On Windows, prevent console window flashing
    #[cfg(windows)]
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW

    let output = cmd.output().await.map_err(|e| format!("Failed to run yt-dlp: {}", e))?;

    if !output.status.success() {
        let err_str = String::from_utf8_lossy(&output.stderr);
        return Err(format!("yt-dlp error: {}", err_str));
    }

    let json_str = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value = serde_json::from_str(&json_str)
        .map_err(|e| format!("Failed to parse metadata JSON: {}", e))?;

    let duration_num = v.get("duration").and_then(|d| d.as_u64());

    Ok(VideoInfo {
        id: v.get("id").and_then(|x| x.as_str()).unwrap_or("").to_string(),
        title: v.get("title").and_then(|x| x.as_str()).unwrap_or("Audio").to_string(),
        uploader: v.get("uploader").or_else(|| v.get("channel"))
            .and_then(|x| x.as_str()).unwrap_or("Unknown").to_string(),
        duration: duration_num,
        duration_str: format_seconds(duration_num),
        thumbnail: v.get("thumbnail").and_then(|x| x.as_str()).map(|s| s.to_string()),
        view_count: v.get("view_count").and_then(|x| x.as_u64()),
        url: url.to_string(),
    })
}

pub async fn run_download(
    bin_paths: &BinPaths,
    url: &str,
    quality: &str,
    output_dir: &str,
    app: AppHandle,
) -> Result<String, String> {
    let out_template = format!("{}/%(title)s.%(ext)s", output_dir.trim_end_matches(['/', '\\']));

    let mut cmd = Command::new(&bin_paths.ytdlp);
    cmd.arg("-x")
        .arg("--audio-format")
        .arg("mp3")
        .arg("--audio-quality")
        .arg(quality)
        .arg("--ffmpeg-location")
        .arg(&bin_paths.ffmpeg)
        .arg("--newline")
        .arg("--no-playlist")
        .arg("-o")
        .arg(&out_template)
        .arg(url)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(windows)]
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW

    let mut child = cmd.spawn().map_err(|e| format!("Failed to spawn yt-dlp: {}", e))?;

    let stdout = child.stdout.take().ok_or("Failed to capture child stdout")?;
    let mut reader = BufReader::new(stdout).lines();

    // Regex patterns for parsing yt-dlp progress output
    // e.g. [download]  45.2% of  12.50MiB at  3.45MiB/s ETA 00:02
    let progress_re = Regex::new(r"\[download\]\s+([0-9\.]+)%\s+of\s+~?([0-9\.]+[A-Za-z]+)\s+at\s+([0-9\.]+[A-Za-z/]+)\s+ETA\s+([0-9:]+)").unwrap();
    let dest_re = Regex::new(r"\[(?:ExtractAudio|download)\] Destination:\s*(.*\.mp3)").unwrap();

    let mut final_mp3_path: Option<String> = None;

    // Send initial status
    let _ = app.emit("download-progress", ProgressUpdate {
        percent: 1.0,
        stage: "Подключение к YouTube...".to_string(),
        speed: "".to_string(),
        eta: "".to_string(),
        downloaded_size: "".to_string(),
        total_size: "".to_string(),
        finished: false,
        output_path: None,
        error: None,
    });

    while let Ok(Some(line)) = reader.next_line().await {
        let trimmed = line.trim();

        if let Some(caps) = progress_re.captures(trimmed) {
            let pct: f32 = caps.get(1).and_then(|m| m.as_str().parse().ok()).unwrap_or(0.0);
            let total = caps.get(2).map(|m| m.as_str().to_string()).unwrap_or_default();
            let speed = caps.get(3).map(|m| m.as_str().to_string()).unwrap_or_default();
            let eta = caps.get(4).map(|m| m.as_str().to_string()).unwrap_or_default();

            let _ = app.emit("download-progress", ProgressUpdate {
                percent: (pct * 0.95).min(95.0),
                stage: "Скачивание аудиопотока...".to_string(),
                speed,
                eta,
                downloaded_size: "".to_string(),
                total_size: total,
                finished: false,
                output_path: None,
                error: None,
            });
        } else if trimmed.contains("[ExtractAudio]") {
            if let Some(caps) = dest_re.captures(trimmed) {
                if let Some(dest) = caps.get(1) {
                    final_mp3_path = Some(dest.as_str().to_string());
                }
            }
            let _ = app.emit("download-progress", ProgressUpdate {
                percent: 96.0,
                stage: "Конвертация в MP3 (FFmpeg)...".to_string(),
                speed: "".to_string(),
                eta: "".to_string(),
                downloaded_size: "".to_string(),
                total_size: "".to_string(),
                finished: false,
                output_path: None,
                error: None,
            });
        } else if let Some(caps) = dest_re.captures(trimmed) {
            if let Some(dest) = caps.get(1) {
                final_mp3_path = Some(dest.as_str().to_string());
            }
        }
    }

    let status = child.wait().await.map_err(|e| format!("Process wait failed: {}", e))?;

    if status.success() {
        let result_path = final_mp3_path.unwrap_or_else(|| {
            format!("{}/output.mp3", output_dir)
        });

        let _ = app.emit("download-progress", ProgressUpdate {
            percent: 100.0,
            stage: "Готово! Файл сохранён.".to_string(),
            speed: "".to_string(),
            eta: "".to_string(),
            downloaded_size: "".to_string(),
            total_size: "".to_string(),
            finished: true,
            output_path: Some(result_path.clone()),
            error: None,
        });

        Ok(result_path)
    } else {
        let err_msg = "Ошибка при загрузке или конвертации видео".to_string();
        let _ = app.emit("download-progress", ProgressUpdate {
            percent: 0.0,
            stage: "Ошибка".to_string(),
            speed: "".to_string(),
            eta: "".to_string(),
            downloaded_size: "".to_string(),
            total_size: "".to_string(),
            finished: false,
            output_path: None,
            error: Some(err_msg.clone()),
        });
        Err(err_msg)
    }
}
