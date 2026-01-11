<div align="center">
  <img src="assets/app_icon.ico" alt="VidExtract Logo" width="120" height="120">

  # 🎬 VidExtract
  ### The Ultimate Video & Audio Downloader

  <p>
    <img src="https://img.shields.io/github/v/release/idginity124/VidExtract?style=for-the-badge&color=2ea44f&label=LATEST%20VERSION" alt="Version">
    <img src="https://img.shields.io/badge/PYTHON-3.10%2B-3776AB?style=for-the-badge&logo=python&logoColor=white" alt="Python">
    <img src="https://img.shields.io/badge/PLATFORM-WINDOWS-0078D6?style=for-the-badge&logo=windows&logoColor=white" alt="Platform">
    <img src="https://img.shields.io/badge/GUI-PySide6-41CD52?style=for-the-badge&logo=qt&logoColor=white" alt="PySide6">
    <img src="https://img.shields.io/badge/LICENSE-MIT-yellow?style=for-the-badge" alt="License">
  </p>

  <p>
    <strong>VidExtract</strong> is a powerful desktop application with a modern interface that allows you to download videos from YouTube, Reddit, Instagram, TikTok, and more with a single click.
  </p>

  <p>
    <a href="#-download--installation">📥 Download (EXE)</a> •
    <a href="#-screenshots">📸 Screenshots</a> •
    <a href="#-key-features">🌟 Features</a> •
    <a href="#-developer-setup">🛠️ Source Code</a>
  </p>
</div>

---

## 📸 Screenshots

Check out the application's modern and user-friendly interface.

| **Service Selection** | **Video Download** |
|:---:|:---:|
| <img src="docs/main_menu.png" width="100%" alt="Main Menu"> | <img src="docs/download_screen.png" width="100%" alt="Download Screen"> |
| *All popular platforms in one screen* | *Detailed video info and progress bar* |

| **YouTube Mode** | **Settings (General & Advanced)** |
|:---:|:---:|
| <img src="docs/youtube_input.png" width="100%" alt="YouTube Input"> | <img src="docs/settings_advanced.png" width="100%" alt="Settings"> |
| *Fast pasting with auto clipboard monitoring* | *Auto FFmpeg setup and theme options* |

---

## 🌟 Key Features

* **📺 Multi-Platform Support:** Seamlessly download from YouTube, Reddit, Twitter (X), Facebook, Instagram, TikTok, and SoundCloud.
* **📋 Smart Clipboard Monitor:** Automatically detects copied URLs and pastes them into the box. Just copy and download!
* **🌍 10+ Language Support:** Turkish, English, German, Spanish, French, Russian, Arabic, Chinese...
* **⚙️ Auto FFmpeg Setup:** No more complex settings! The app automatically downloads and installs FFmpeg for you.
* **🎨 Modern & Responsive Design:** Built with PySide6, featuring Dark/Light mode support.
* **🚀 High Quality:** Supports 4K, 8K video downloads and high-quality audio conversion (MP3, M4A, FLAC).
* **📂 Playlist Support:** Download entire YouTube playlists in a single click.

---

## 📥 Download & Installation

No need to install Python! You can download the ready-to-use `.exe` file and start using it immediately.

1.  Go to the **[Releases](https://github.com/idginity124/VidExtract/releases)** section on the right side of the page.
2.  Download the latest `VidExtract.exe` file.
3.  Double-click to run. (No installation required!)

> **Note:** If you see a Windows SmartScreen warning, click "More Info" -> "Run Anyway".

---

## 🛠️ Developer Setup

If you want to develop the project or run it from source code:

```bash
# 1. Clone the Project
git clone https://github.com/idginity124/VidExtract.git
cd VidExtract

# 2. Install Requirements
pip install -r requirements.txt

# 3. Run the App
python vidextract.py
