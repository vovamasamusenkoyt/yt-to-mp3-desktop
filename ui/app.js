// Tauri IPC helper
const invoke = window.__TAURI__ ? window.__TAURI__.core.invoke : async () => null;
const listen = window.__TAURI__ ? window.__TAURI__.event.listen : async () => () => {};

// DOM Elements
const outputPathEl = document.getElementById('output-path');
const browseFolderBtn = document.getElementById('browse-folder-btn');

const urlInput = document.getElementById('video-url');
const pasteBtn = document.getElementById('paste-btn');
const clearBtn = document.getElementById('clear-btn');
const searchBtn = document.getElementById('search-btn');
const searchSpinner = document.getElementById('search-spinner');

const errorBox = document.getElementById('error-box');
const errorMessage = document.getElementById('error-message');

const previewSection = document.getElementById('preview-section');
const videoThumb = document.getElementById('video-thumb');
const videoDuration = document.getElementById('video-duration');
const videoTitle = document.getElementById('video-title');
const videoAuthor = document.getElementById('video-author');
const videoViews = document.getElementById('video-views');

const progressSection = document.getElementById('progress-section');
const stageText = document.getElementById('stage-text');
const progressPercent = document.getElementById('progress-percent');
const progressFill = document.getElementById('progress-fill');
const statSize = document.getElementById('stat-size');
const statSpeed = document.getElementById('stat-speed');
const statEta = document.getElementById('stat-eta');

const completeSection = document.getElementById('complete-section');
const savedFilepathEl = document.getElementById('saved-filepath');

// State
let selectedFolder = '';
let currentVideoInfo = null;
let lastSavedFile = '';

// Initialize
async function init() {
  try {
    // Get default folder from Rust
    selectedFolder = await invoke('get_default_folder_cmd');
    outputPathEl.textContent = selectedFolder || 'Папка не выбрана';
    outputPathEl.title = selectedFolder;

    // Listen to download progress events from Rust
    await listen('download-progress', (event) => {
      const data = event.payload;
      handleProgressUpdate(data);
    });
  } catch (err) {
    console.warn('Tauri init fallback (running outside Tauri window):', err);
    selectedFolder = 'C:\\Users\\User\\Music';
    outputPathEl.textContent = selectedFolder;
  }
}

// Select folder
browseFolderBtn.addEventListener('click', async () => {
  try {
    const chosen = await invoke('select_output_folder_cmd');
    if (chosen) {
      selectedFolder = chosen;
      outputPathEl.textContent = selectedFolder;
      outputPathEl.title = selectedFolder;
    }
  } catch (err) {
    showError('Не удалось открыть диалог выбора папки: ' + err);
  }
});

// Input controls
urlInput.addEventListener('input', () => {
  if (urlInput.value.trim().length > 0) {
    clearBtn.classList.remove('hidden');
  } else {
    clearBtn.classList.add('hidden');
  }
});

clearBtn.addEventListener('click', () => {
  urlInput.value = '';
  clearBtn.classList.add('hidden');
  hideError();
  urlInput.focus();
});

pasteBtn.addEventListener('click', async () => {
  try {
    const text = await navigator.clipboard.readText();
    if (text) {
      urlInput.value = text.trim();
      clearBtn.classList.remove('hidden');
      hideError();
      fetchInfo();
    }
  } catch (err) {
    urlInput.focus();
  }
});

function showError(msg) {
  errorMessage.textContent = msg;
  errorBox.classList.remove('hidden');
}

function hideError() {
  errorBox.classList.add('hidden');
}

function formatViews(num) {
  if (!num) return '0 просмотров';
  if (num >= 1000000) return (num / 1000000).toFixed(1) + ' млн просмотров';
  if (num >= 1000) return (num / 1000).toFixed(1) + ' тыс. просмотров';
  return num + ' просмотров';
}

// Fetch Video Information
async function fetchInfo() {
  const url = urlInput.value.trim();
  if (!url) {
    showError('Пожалуйста, введите ссылку на YouTube');
    return;
  }

  hideError();
  previewSection.classList.add('hidden');
  progressSection.classList.add('hidden');
  completeSection.classList.add('hidden');

  searchBtn.disabled = true;
  searchSpinner.classList.remove('hidden');

  try {
    const info = await invoke('get_video_info_cmd', { url });
    currentVideoInfo = info;

    videoThumb.src = info.thumbnail || '';
    videoDuration.textContent = info.duration_str || '--:--';
    videoTitle.textContent = info.title || 'Видео';
    videoAuthor.textContent = info.uploader || 'Неизвестный автор';
    videoViews.textContent = formatViews(info.view_count);

    previewSection.classList.remove('hidden');
  } catch (err) {
    showError(err || 'Не удалось получить информацию о видео');
  } finally {
    searchBtn.disabled = false;
    searchSpinner.classList.add('hidden');
  }
}

// Start Download and MP3 conversion
async function startDownload() {
  if (!currentVideoInfo) return;

  const url = currentVideoInfo.url || urlInput.value.trim();
  const qualityEl = document.querySelector('input[name="quality"]:checked');
  const quality = qualityEl ? qualityEl.value : '192';

  hideError();
  previewSection.classList.add('hidden');
  completeSection.classList.add('hidden');
  progressSection.classList.remove('hidden');

  // Reset progress UI
  progressFill.style.width = '0%';
  progressPercent.textContent = '0%';
  stageText.textContent = 'Подготовка к конвертации...';
  statSize.textContent = '';
  statSpeed.textContent = '';
  statEta.textContent = '';

  try {
    await invoke('start_download_cmd', {
      url,
      quality,
      outputDir: selectedFolder
    });
  } catch (err) {
    progressSection.classList.add('hidden');
    previewSection.classList.remove('hidden');
    showError(err || 'Ошибка запуска загрузки');
  }
}

// Handle real-time progress events from Rust
function handleProgressUpdate(data) {
  if (data.error) {
    progressSection.classList.add('hidden');
    previewSection.classList.remove('hidden');
    showError(data.error);
    return;
  }

  const pct = Math.min(Math.max(data.percent || 0, 0), 100);
  progressFill.style.width = `${pct}%`;
  progressPercent.textContent = `${Math.round(pct)}%`;

  if (data.stage) stageText.textContent = data.stage;
  if (data.total_size) statSize.textContent = data.total_size;
  if (data.speed) statSpeed.textContent = data.speed;
  if (data.eta) statEta.textContent = `Осталось: ${data.eta}`;

  if (data.finished) {
    progressSection.classList.add('hidden');
    completeSection.classList.remove('hidden');
    lastSavedFile = data.output_path || '';
    savedFilepathEl.textContent = lastSavedFile;
  }
}

// Open Explorer with file selected
async function openFileInFolder() {
  if (!lastSavedFile) return;
  try {
    await invoke('open_in_folder_cmd', { filePath: lastSavedFile });
  } catch (err) {
    showError('Не удалось открыть Проводник: ' + err);
  }
}

// Reset UI for next conversion
function resetUI() {
  urlInput.value = '';
  clearBtn.classList.add('hidden');
  previewSection.classList.add('hidden');
  progressSection.classList.add('hidden');
  completeSection.classList.add('hidden');
  hideError();
  currentVideoInfo = null;
  lastSavedFile = '';
  urlInput.focus();
}

// Run init on load
window.addEventListener('DOMContentLoaded', init);
