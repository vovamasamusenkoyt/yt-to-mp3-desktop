#!/usr/bin/env bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "=== Fetching Windows binaries for monolithic build ==="

# 1. Fetch yt-dlp.exe
if [ ! -f "yt-dlp.exe" ] || [ ! -s "yt-dlp.exe" ]; then
    echo "Downloading yt-dlp.exe..."
    curl -L -o yt-dlp.exe "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe"
    echo "yt-dlp.exe downloaded ($(du -h yt-dlp.exe | cut -f1))"
else
    echo "yt-dlp.exe already present ($(du -h yt-dlp.exe | cut -f1))"
fi

# 2. Fetch ffmpeg.exe (extract from yt-dlp official ffmpeg builds)
if [ ! -f "ffmpeg.exe" ] || [ ! -s "ffmpeg.exe" ]; then
    echo "Downloading ffmpeg build for Windows..."
    TMP_ZIP="ffmpeg_win64.zip"
    rm -f "$TMP_ZIP"
    
    # Download ffmpeg zip (master latest win64 gpl)
    curl -L -o "$TMP_ZIP" "https://github.com/yt-dlp/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl.zip"
    
    echo "Extracting ffmpeg.exe using python..."
    PYTHON_BIN="python"
    if ! command -v python &> /dev/null; then
        PYTHON_BIN="python3"
    fi

    "$PYTHON_BIN" -c "
import zipfile, shutil
with zipfile.ZipFile('ffmpeg_win64.zip', 'r') as z:
    for name in z.namelist():
        if name.endswith('bin/ffmpeg.exe'):
            with z.open(name) as src, open('ffmpeg.exe', 'wb') as dst:
                shutil.copyfileobj(src, dst)
            print('Extracted ffmpeg.exe successfully')
            break
"
    rm -f "$TMP_ZIP"
    echo "ffmpeg.exe ready ($(du -h ffmpeg.exe | cut -f1))"
else
    echo "ffmpeg.exe already present ($(du -h ffmpeg.exe | cut -f1))"
fi

echo "=== All binaries ready in src-tauri/bin/ ==="
ls -lh "$SCRIPT_DIR"
