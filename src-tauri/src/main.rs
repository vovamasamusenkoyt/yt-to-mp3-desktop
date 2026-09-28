// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod bin_manager;
mod downloader;

use std::path::PathBuf;
use tauri::AppHandle;

use crate::downloader::{extract_video_info, run_download, VideoInfo};

#[tauri::command]
async fn get_video_info_cmd(url: String) -> Result<VideoInfo, String> {
    let bins = bin_manager::ensure_binaries()?;
    extract_video_info(&bins, &url).await
}

#[tauri::command]
async fn start_download_cmd(
    app: AppHandle,
    url: String,
    quality: String,
    output_dir: String,
) -> Result<String, String> {
    let bins = bin_manager::ensure_binaries()?;
    
    // Validate output dir
    let out_path = PathBuf::from(&output_dir);
    if !out_path.exists() {
        std::fs::create_dir_all(&out_path)
            .map_err(|e| format!("Failed to create output directory: {}", e))?;
    }

    tokio::spawn(async move {
        let _ = run_download(&bins, &url, &quality, &output_dir, app).await;
    });

    Ok("Download started".to_string())
}

#[tauri::command]
fn select_output_folder_cmd() -> Option<String> {
    let folder = rfd::FileDialog::new()
        .set_title("Выберите папку для сохранения музыки")
        .pick_folder();

    folder.map(|p| p.to_string_lossy().to_string())
}

#[tauri::command]
fn get_default_folder_cmd() -> String {
    if let Some(audio) = dirs::audio_dir() {
        return audio.to_string_lossy().to_string();
    }
    if let Some(download) = dirs::download_dir() {
        return download.to_string_lossy().to_string();
    }
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .to_string_lossy()
        .to_string()
}

#[tauri::command]
fn open_in_folder_cmd(file_path: String) -> Result<(), String> {
    let path = PathBuf::from(&file_path);

    #[cfg(windows)]
    {
        use std::process::Command;
        if path.exists() {
            // Select the file in Windows Explorer
            let _ = Command::new("explorer")
                .args(["/select,", &file_path])
                .spawn();
            return Ok(());
        } else if let Some(parent) = path.parent() {
            let _ = Command::new("explorer")
                .arg(parent)
                .spawn();
            return Ok(());
        }
    }

    // Fallback or non-Windows
    let target = if path.exists() {
        path.parent().unwrap_or(&path).to_path_buf()
    } else {
        path
    };

    open::that(target).map_err(|e| format!("Failed to open folder: {}", e))?;
    Ok(())
}

fn main() {
    // Background extraction of embedded binaries so they are ready
    std::thread::spawn(|| {
        let _ = bin_manager::ensure_binaries();
    });

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_video_info_cmd,
            start_download_cmd,
            select_output_folder_cmd,
            get_default_folder_cmd,
            open_in_folder_cmd
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
