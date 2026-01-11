from PySide6.QtCore import (
    Qt, Signal, Slot, QThread, QPropertyAnimation, QTimer, 
    QRectF, QRect, QSize, QEasingCurve, QUrl
)
from PySide6.QtWidgets import (
    QApplication, QWidget, QVBoxLayout, QHBoxLayout, QLabel, 
    QLineEdit, QPushButton, QComboBox, QFileDialog, QProgressBar,
    QSplitter, QMessageBox, QDialog, QScrollArea, QGraphicsOpacityEffect,
    QSplashScreen, QGraphicsBlurEffect, QCheckBox, QSpacerItem, QSizePolicy,
    QGroupBox, QTabWidget, QProgressDialog 
)
from PySide6.QtMultimedia import QMediaPlayer, QVideoSink, QVideoFrame

from PySide6.QtGui import (
    QPixmap, QPainter, QLinearGradient, QColor, QFont, 
    QPen, QScreen, QTextDocument, QImage, QIcon
)
import os
import re
import yt_dlp
import subprocess
import requests
import sys
import traceback   
import time
from modern_style import get_service_theme, SERVICE_COLORS
import threading
import unicodedata
from languages import LANGUAGES
import shutil
from logger_setup import logger

from settings_manager import (
    settings_manager, 
    KEY_DOWNLOAD_FOLDER, 
    KEY_LANGUAGE_INDEX, 
    KEY_THEME_MODE, 
    KEY_COOKIE_PATH,
    KEY_CLIPBOARD_MONITOR
)

import zipfile
import io
if os.name == 'nt':
    import winreg 
    import ctypes 

APP_DATA_PATH = os.path.join(os.getenv('LOCALAPPDATA'), 'VidExtract')
TARGET_PATH = os.path.join(APP_DATA_PATH, "ffmpeg") 
TARGET_BIN_PATH = os.path.join(TARGET_PATH, "bin")

def resource_path(relative_path):
    """ PyInstaller tarafından oluşturulan geçici yoldaki varlıklara erişmek için. """
    try:
        base_path = sys._MEIPASS
    except Exception:
        base_path = os.path.abspath(".")

    return os.path.join(base_path, relative_path)


def get_latest_ffmpeg_url():
    """
    GitHub API'sini kullanarak her zaman en güncel FFmpeg indirme linkini bulur.
    Eğer API çalışmazsa, yedek olarak kalıcı bir link (Gyan.dev) kullanır.
    """
    try:
        logger.info("En güncel FFmpeg sürümü GitHub'dan sorgulanıyor...")
        api_url = "https://api.github.com/repos/BtbN/FFmpeg-Builds/releases/latest"
        
        response = requests.get(api_url, timeout=10)
        
        if response.status_code == 200:
            data = response.json()

            for asset in data.get('assets', []):
                name = asset.get('name', '').lower()

                if "win64-gpl.zip" in name and "shared" not in name:
                    found_url = asset['browser_download_url']
                    logger.info(f"Güncel URL bulundu: {found_url}")
                    return found_url
                    
    except Exception as e:
        logger.warning(f"GitHub API hatası: {e}. Yedek link kullanılıyor.")

    return "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip"

class DownloadThread(QThread):
    progress_signal = Signal(int, str) 
    finished_signal = Signal(str)      

    def __init__(self, url, download_folder, quality, video_format, output_format, 
                 download_type_key, language_code, download_subs, sub_langs, 
                 cookie_file_path): 
        super().__init__()
        self.url = url
        self.download_folder = download_folder
        self.quality = quality
        self.video_format = video_format
        self.output_format = output_format
        self.download_type_key = download_type_key
        self.language_code = language_code
        self.download_subs = download_subs
        self.sub_langs = sub_langs
        self.cookie_file_path = cookie_file_path
    
    def run(self):
        try:
            is_playlist = (self.download_type_key == "playlist")
            
            audio_string = LANGUAGES[self.language_code]["list_type_audio"]

            logger.info(f"İndirme işlemi başlatıldı: {self.url} | Format: {self.video_format}")
            
            if is_playlist:
                try:
                    with yt_dlp.YoutubeDL({'quiet': True, 'noplaylist': False, 'extract_flat': True}) as ydl_info:
                        info = ydl_info.extract_info(self.url, download=False)
                        playlist_title = self.sanitize_filename(info.get('title', 'oynatma_listesi'))
                except Exception as e:
                    logger.warning(f"Playlist başlığı alınamadı, varsayılan kullanılıyor. Hata: {e}")
                    playlist_title = self.sanitize_filename(f"oynatma_listesi_{int(time.time())}")
                
                output_path_base = os.path.join(self.download_folder, playlist_title)
                raw_output_template = os.path.join(output_path_base, '%(playlist_autonumber)s - %(title)s.%(ext)s')
            else:
                output_path_base = self.download_folder
                raw_output_template = os.path.join(output_path_base, '%(title)s.%(ext)s')

            if not os.path.exists(output_path_base):
                os.makedirs(output_path_base, exist_ok=True)

            postprocessors = []
            
            target_format_key = "mp4" 
            lang_dict = LANGUAGES[self.language_code]
            for key, value in lang_dict.items():
                if value == self.output_format:
                    target_format_key = key
                    break
            
            target_format_ext = target_format_key.lower() 

            if self.video_format == audio_string: 
                ydl_format_string = "bestaudio/best"
                postprocessors.append({
                    'key': 'FFmpegExtractAudio',
                    'preferredcodec': target_format_ext,
                    'preferredquality': '192', 
                })
                if target_format_ext == 'mp3':
                    postprocessors[0]['preferredquality'] = '320'
                elif target_format_ext == 'm4a':
                    postprocessors[0]['preferredquality'] = '256'

            else: 
                if self.quality == "best": 
                    ydl_format_string = "bestvideo+bestaudio/best"
                else: 
                    ydl_format_string = self.get_yt_dlp_format(self.quality, self.video_format)
                
                postprocessors.append({
                    'key': 'FFmpegVideoRemuxer',
                    'preferedformat': target_format_ext,
                })

            ffmpeg_path = None 

            if hasattr(sys, '_MEIPASS'):
                meipass_ffmpeg_path = os.path.join(sys._MEIPASS, 'ffmpeg', 'bin')
                if os.path.exists(os.path.join(meipass_ffmpeg_path, "ffmpeg.exe")):
                    ffmpeg_path = meipass_ffmpeg_path
                    logger.info(f"FFmpeg (MEIPASS) yolu tespit edildi: {ffmpeg_path}")
            
            if not ffmpeg_path and os.path.exists(os.path.join(TARGET_BIN_PATH, "ffmpeg.exe")):
                ffmpeg_path = TARGET_BIN_PATH
                logger.info(f"FFmpeg (AppData) kullanılıyor: {ffmpeg_path}")

            if not ffmpeg_path and shutil.which("ffmpeg"):
                logger.info("FFmpeg (Sistem PATH) kullanılacak.")
                ffmpeg_path = None
                 
            ydl_opts = {
                'format': ydl_format_string,
                'outtmpl': raw_output_template,
                'quiet': False,
                'progress_hooks': [self._progress_hook],
                'postprocessors': postprocessors,
                'noplaylist': not is_playlist,
                'nooverwrites': False,
                'ffmpeg_location': ffmpeg_path,
                'ignoreerrors': is_playlist, 
            }
            
            if self.cookie_file_path and os.path.exists(self.cookie_file_path):
                print(f"Kullanılan çerez dosyası: {self.cookie_file_path}")
                ydl_opts['cookiefile'] = self.cookie_file_path

            if self.download_subs and self.video_format != audio_string: 
                lang_list = [lang.strip() for lang in self.sub_langs.split(',')] if self.sub_langs else ['en', 'tr'] 
                
                ydl_opts['writesubtitles'] = True
                ydl_opts['subtitleslangs'] = lang_list
                ydl_opts['writeautomaticsub'] = True 
                ydl_opts['embedsubtitles'] = True 
            
            with yt_dlp.YoutubeDL(ydl_opts) as ydl:
                ydl.download([self.url]) 

            success_message = f"✅ İndirme tamamlandı!\nDosyalar '{output_path_base}' klasörüne kaydedildi."
            self.finished_signal.emit(success_message)

        except Exception as e:
            error_message = f"Hata oluştu: {str(e)}\n{traceback.format_exc()}"
            self.finished_signal.emit(error_message)

    def sanitize_filename(self,filename, max_length=100):
        filename = unicodedata.normalize('NFKD', filename).encode('ASCII', 'ignore').decode('utf-8')
        filename = re.sub(r'[\\/*?:"<>|]', '', filename)
        filename = re.sub(r'[^a-zA-Z0-9\s\-_#\.]', '', filename)
        filename = re.sub(r'[\s]+', ' ', filename)  
        filename = re.sub(r'[_\-]+', '_', filename) 
        filename = filename.strip()
        if len(filename) > max_length:
            filename = filename[:max_length].rsplit(' ', 1)[0]  
        return filename
    
    def get_yt_dlp_format(self, quality, video_format):
        height = re.search(r'^\d+', quality)
        if height:
            height_str = height.group(0)
            return f"bestvideo[height<={height_str}]+bestaudio/best[height<={height_str}]"
        else:
            return "bestvideo[height<=720]+bestaudio/best[height<=720]"
    
    def _progress_hook(self, d):
        try:
            if d.get('status') == 'downloading':
                total_size = d.get('total_bytes') or d.get('total_bytes_estimate', 0)
                downloaded_size = d.get('downloaded_bytes', 0)
                speed = d.get('speed', 0)  
                eta = d.get('eta', 0)  

                playlist_index = d.get('info_dict', {}).get('playlist_autonumber')
                n_entries = d.get('info_dict', {}).get('n_entries')
                
                playlist_prefix = ""
                if playlist_index and n_entries:
                    playlist_prefix = f"**[Video {playlist_index}/{n_entries}]**\n"

                if total_size > 0:
                    percent = int(downloaded_size / total_size * 100)
                    downloaded_mb = downloaded_size / 1024 / 1024
                    total_mb = total_size / 1024 / 1024

                    if speed >= 1_048_576: 
                        speed_str = f"{speed / 1_048_576:.2f} MB/s"
                    elif speed >= 1024: 
                        speed_str = f"{speed / 1024:.2f} KB/s"
                    else:
                        speed_str = f"{speed:.2f} B/s"

                    if eta > 60:
                        eta_str = f"{eta // 60}m {eta % 60}s"
                    else:
                        eta_str = f"{eta}s"

                    progress_message = (
                        f"{playlist_prefix}📥 İndirme devam ediyor: **{percent}% tamamlandı**\n"
                        f"🔹 {downloaded_mb:.2f} MB / {total_mb:.2f} MB\n"
                        f"🚀 Hız: {speed_str} | ⏳ Kalan süre: {eta_str}"
                    )
                    self.progress_signal.emit(percent, progress_message)
                else:
                    self.progress_signal.emit(0, f"{playlist_prefix}⏳ İndirme başlatılıyor... Lütfen bekleyin.")

            elif d.get('status') == 'finished':
                playlist_index = d.get('info_dict', {}).get('playlist_autonumber')
                if not playlist_index:
                    self.progress_signal.emit(100, "✅ **İndirme tamamlandı!**")
                else:
                    self.progress_signal.emit(100, f"✅ **[Video {d['info_dict']['playlist_autonumber']}]** tamamlandı!")

            elif d.get('status') == 'error':
                error_message = d.get('error', 'Bilinmeyen hata')
                if isinstance(error_message, dict):
                    error_message = f"⚠️ Hata: {error_message.get('code', 'Bilinmeyen kod')}, {error_message.get('message', 'Açıklama bulunamadı.')}"
                self.progress_signal.emit(0, error_message)

        except KeyError as e:
            self.progress_signal.emit(0, f"⚠️ Eksik veri hatası: {str(e)}")
        except Exception as e:
            self.progress_signal.emit(0, f" {str(e)}")


class UpdateThread(QThread):
    finished_signal = Signal(str)

    def __init__(self):
        super().__init__()

    def run(self):
        try:
            command = [sys.executable, "-m", "yt_dlp", "-U"]
            
            startupinfo = None
            if os.name == 'nt':
                startupinfo = subprocess.STARTUPINFO()
                startupinfo.dwFlags |= subprocess.STARTF_USESHOWWINDOW
                startupinfo.wShowWindow = subprocess.SW_HIDE

            result = subprocess.run(
                command,
                capture_output=True,
                text=True,
                encoding='utf-8',
                startupinfo=startupinfo,
                check=False 
            )

            if result.returncode == 0:
                message = result.stdout.strip()
                if not message:
                    message = result.stderr.strip()
            else:
                message = result.stderr.strip()
                if not message:
                    message = f"Bilinmeyen bir hata oluştu (Kod: {result.returncode})"

            self.finished_signal.emit(message)

        except Exception as e:
            self.finished_signal.emit(f"Güncelleme işlemi çalıştırılamadı: {str(e)}")

class FFmpegDownloadThread(QThread):
    progress_signal = Signal(int, str) 
    finished_signal = Signal(bool, str) 

    def run(self):
        try:
          
            self.progress_signal.emit(0, "Güncel sürüm aranıyor...")
            
            current_ffmpeg_url = get_latest_ffmpeg_url()
            
            self.progress_signal.emit(5, "İndirme başlatılıyor...")
            
            response = requests.get(current_ffmpeg_url, stream=True)
            response.raise_for_status()
            
            total_size = int(response.headers.get('content-length', 0))
            downloaded_size = 0
            chunk_size = 8192
            
            zip_data = io.BytesIO()
            
            for data in response.iter_content(chunk_size=chunk_size):
                zip_data.write(data)
                downloaded_size += len(data)
                if total_size > 0:
                    percent = int(downloaded_size / total_size * 100)
                    self.progress_signal.emit(percent, f"İndiriliyor... {downloaded_size / 1024 / 1024:.1f}MB / {total_size / 1024 / 1024:.1f}MB")
            
            self.progress_signal.emit(100, "Ayıklanıyor...")
            
            if os.path.exists(TARGET_BIN_PATH):
                try:
                    shutil.rmtree(TARGET_PATH, ignore_errors=True)
                except Exception as e:
                    print(f"Eski FFmpeg klasörü silinemedi, devam ediliyor: {e}")
                    
            os.makedirs(TARGET_BIN_PATH) 

            with zipfile.ZipFile(zip_data) as z:
                for file_info in z.infolist():
                    parts = file_info.filename.split('/')
                    if len(parts) > 1 and parts[-2] == 'bin' and parts[-1] != '':
                        filename = parts[-1]
                        target_file_path = os.path.join(TARGET_BIN_PATH, filename)
                        with open(target_file_path, 'wb') as f:
                            f.write(z.read(file_info.filename))
            
            self.progress_signal.emit(100, "PATH ayarlanıyor...")
            
            if os.name == 'nt':
                self.add_to_path(TARGET_BIN_PATH)
            
            self.finished_signal.emit(True, f"FFmpeg başarıyla {TARGET_BIN_PATH} konumuna kuruldu ve PATH'e eklendi.")

        except PermissionError:
            self.finished_signal.emit(False, f"İzin Hatası! '{TARGET_PATH}' dizinine yazılamıyor. Başka bir programın dosyaları kilitlemediğinden emin olun.")
        except Exception as e:
            self.finished_signal.emit(False, f"Bir hata oluştu: {str(e)}\n{traceback.format_exc()}")

    def add_to_path(self, path_to_add):
        if os.name != 'nt':
            return 

        try:
            key_path = r"Environment"
            key = winreg.OpenKey(winreg.HKEY_CURRENT_USER, key_path, 0, winreg.KEY_READ | winreg.KEY_WRITE)
            
            try:
                current_path, _ = winreg.QueryValueEx(key, "PATH")
            except FileNotFoundError:
                current_path = "" 
            
            paths = current_path.split(';')
            
            old_ffmpeg_path = r"C:\ffmpeg\bin"
            paths = [p for p in paths if p != old_ffmpeg_path and p != f"{old_ffmpeg_path}\\"]

            if path_to_add in paths or f"{path_to_add}\\" in paths:
                winreg.CloseKey(key)
                self.progress_signal.emit(100, "PATH zaten güncel.")
                return 

            paths.append(path_to_add)
            new_path = ";".join([p for p in paths if p])
            
            winreg.SetValueEx(key, "PATH", 0, winreg.REG_EXPAND_SZ, new_path)
            winreg.CloseKey(key)

            HWND_BROADCAST = 0xFFFF
            WM_SETTINGCHANGE = 0x1A
            SMTO_ABORTIFHUNG = 0x0002
            
            result = ctypes.c_long()
            ctypes.windll.user32.SendMessageTimeoutW(
                HWND_BROADCAST,
                WM_SETTINGCHANGE,
                0,
                "Environment", 
                SMTO_ABORTIFHUNG,
                5000, 
                ctypes.byref(result)
            )
        except Exception as e:
            print(f"PATH güncellenemedi, ancak bu kritik bir hata değil: {e}")
            raise Exception(f"FFmpeg kuruldu ancak PATH güncellenemedi: {e}")

class SettingsDialog(QDialog):
    def __init__(self, parent=None):
        super().__init__(parent)
        
        self.parent_app = parent 
        
        self.setMinimumWidth(600)
        self.setStyleSheet(parent.styleSheet()) 
        
        self.tabs = QTabWidget()
        
        self.general_tab = QWidget()
        gen_layout = QVBoxLayout(self.general_tab)
        gen_layout.setSpacing(15)

        self.lang_label = QLabel()
        gen_layout.addWidget(self.lang_label)
        self.language_combo = QComboBox()
        self.language_combo.addItems(["Türkçe", "English", "Español", "Deutsch", "Français", "Italiano", "Português", "Русский", "عربي", "中文"])
        
        saved_index = settings_manager.get_setting(KEY_LANGUAGE_INDEX)
        if isinstance(saved_index, int) and saved_index > 0:
             self.language_combo.setCurrentIndex(saved_index - 1)
        else:
             self.language_combo.setCurrentIndex(0) 

        self.language_combo.currentIndexChanged.connect(self.apply_language_change)
        
        gen_layout.addWidget(self.language_combo)
        
        gen_layout.addSpacerItem(QSpacerItem(20, 20, QSizePolicy.Minimum, QSizePolicy.Fixed))

        self.theme_button = QPushButton()
        self.theme_button.clicked.connect(self.apply_theme_change)
        gen_layout.addWidget(self.theme_button, alignment=Qt.AlignCenter)
        
        gen_layout.addSpacerItem(QSpacerItem(20, 20, QSizePolicy.Minimum, QSizePolicy.Fixed))

        self.update_button = QPushButton()
        self.update_button.clicked.connect(self.apply_update_check)
        gen_layout.addWidget(self.update_button, alignment=Qt.AlignCenter)
        
        gen_layout.addStretch()

        self.advanced_tab = QWidget()
        adv_layout = QVBoxLayout(self.advanced_tab)
        adv_layout.setSpacing(15)

        self.cookie_group = QGroupBox()
        cookie_group_layout = QVBoxLayout(self.cookie_group)
        
        self.cookie_label = QLabel("Çerez Dosyası Seç (.txt):") 
        cookie_group_layout.addWidget(self.cookie_label)
        
        cookie_layout = QHBoxLayout()
        self.cookie_entry = QLineEdit()
        self.cookie_entry.setPlaceholderText(".../cookies.txt")
        cookie_layout.addWidget(self.cookie_entry)
        
        self.browse_cookie_button = QPushButton("Gözat") 
        self.browse_cookie_button.clicked.connect(self.browse_cookie_file)
        cookie_layout.addWidget(self.browse_cookie_button)
        
        cookie_group_layout.addLayout(cookie_layout)

        self.save_cookie_button = QPushButton("Kaydet") 
        self.save_cookie_button.clicked.connect(self.save_cookie_path)
        cookie_group_layout.addWidget(self.save_cookie_button, alignment=Qt.AlignLeft)
        
        adv_layout.addWidget(self.cookie_group)
        
        self.ffmpeg_group = QGroupBox() 
        ffmpeg_group_layout = QVBoxLayout(self.ffmpeg_group)
        
        self.ffmpeg_status_label = QLabel() 
        ffmpeg_group_layout.addWidget(self.ffmpeg_status_label)
        
        self.install_ffmpeg_button = QPushButton() 
        self.install_ffmpeg_button.clicked.connect(self.apply_ffmpeg_install)
        ffmpeg_group_layout.addWidget(self.install_ffmpeg_button, alignment=Qt.AlignLeft)
        
        self.ffmpeg_tooltip_label = QLabel()
        self.ffmpeg_tooltip_label.setStyleSheet("font-size: 12px; color: #888888;")
        self.ffmpeg_tooltip_label.setWordWrap(True)
        ffmpeg_group_layout.addWidget(self.ffmpeg_tooltip_label)
        
        adv_layout.addWidget(self.ffmpeg_group)
        
        adv_layout.addStretch()

        self.tabs.addTab(self.general_tab, "")
        self.tabs.addTab(self.advanced_tab, "")

        main_layout = QVBoxLayout(self)
        main_layout.addWidget(self.tabs)

        self.close_button = QPushButton()
        self.close_button.clicked.connect(self.accept) 
        main_layout.addWidget(self.close_button, alignment=Qt.AlignCenter)
        
        self.load_settings()
        self.update_language() 
        self.check_ffmpeg_status()

    def apply_ffmpeg_install(self):
        self.install_ffmpeg_button.setEnabled(False) 
        self.parent_app.trigger_ffmpeg_installation()
        
    def check_ffmpeg_status(self):
        found = False
        if shutil.which("ffmpeg") or os.path.exists(os.path.join(TARGET_BIN_PATH, "ffmpeg.exe")):
            found = True
        
        lang_code = self.parent_app.get_current_language_code()
        lang = LANGUAGES.get(lang_code, LANGUAGES["en"])
        
        if found:
            self.ffmpeg_status_label.setText(lang.get("ffmpeg_status_found", "Durum: Bulundu"))
            self.ffmpeg_status_label.setStyleSheet("color: #00FF00;") 
            self.install_ffmpeg_button.setText(lang.get("ffmpeg_button_reinstall", "Yeniden Kur / Güncelle"))
            self.ffmpeg_tooltip_label.setText(lang.get("ffmpeg_tooltip_found", f"FFmpeg sisteminizde (AppData veya PATH) bulundu."))
        else:
            self.ffmpeg_status_label.setText(lang.get("ffmpeg_status_missing", "Durum: Bulunamadı"))
            self.ffmpeg_status_label.setStyleSheet("color: #FF0000;") 
            self.install_ffmpeg_button.setText(lang.get("ffmpeg_button_install", "Otomatik Kur"))
            self.ffmpeg_tooltip_label.setText(lang.get("ffmpeg_tooltip_missing", f"FFmpeg, AppData klasörünüze kurulacak ve PATH'e eklenecek."))

    def update_language(self):
        lang_code = self.parent_app.get_current_language_code()
        lang = LANGUAGES.get(lang_code, LANGUAGES["en"])
        
        self.setWindowTitle(lang.get("settings_title", "Settings"))
        
        self.tabs.setTabText(0, lang.get("settings_tab_general", "General"))
        self.tabs.setTabText(1, lang.get("settings_tab_advanced", "Advanced"))
        
        self.lang_label.setText(lang.get("language", "Language Selection") + ":")
        self.theme_button.setText(lang.get("theme", "Change Theme"))
        self.update_button.setText(lang.get("update_button", "Check for Updates"))
        
        self.cookie_group.setTitle(lang.get("settings_group_cookie", "Cookies"))
        self.cookie_label.setText(lang.get("settings_cookie_label", "Cookies File Path (.txt):"))
        self.ffmpeg_group.setTitle(lang.get("settings_group_ffmpeg", "FFmpeg Installation"))
        self.check_ffmpeg_status() 

        self.close_button.setText(lang.get("settings_close_button", "Close"))

    def apply_language_change(self):
        new_index = self.language_combo.currentIndex() + 1 
        settings_manager.save_setting(KEY_LANGUAGE_INDEX, new_index)
        
        self.parent_app.change_language()
        self.update_language() 

    def apply_theme_change(self):
        self.parent_app.toggle_theme() 
        new_dialog_style = get_service_theme(
            self.parent_app.service, 
            self.parent_app.is_dark_mode
        )
        self.setStyleSheet(new_dialog_style)

    def apply_update_check(self):
        self.parent_app.start_update_check()
        
    def browse_cookie_file(self):
        fileName, _ = QFileDialog.getOpenFileName(self, 
            "Çerez Dosyasını Seç", 
            "", 
            "Text Files (*.txt);;All Files (*)")
        if fileName:
            self.cookie_entry.setText(fileName)

    def load_settings(self):
        try:
            cookie_path = settings_manager.get_setting(KEY_COOKIE_PATH)
            self.cookie_entry.setText(cookie_path)
        except Exception as e:
            logger.warning(f"Ayarlar diyaloğu yüklenirken hata: {e}")

    def save_cookie_path(self):
        try:
            settings_manager.save_setting(KEY_COOKIE_PATH, self.cookie_entry.text())
            self.parent_app.load_settings() 
            QMessageBox.information(self, "Başarılı", "Çerez dosyası yolu kaydedildi.")
        except Exception as e:
            logger.warning(f"Çerez yolu kaydedilirken hata: {e}")

class ServiceSelectionScreen(QWidget):
    def __init__(self):
        super().__init__()
        self.setWindowTitle("VidExtract - Servis Seçimi")
        self.setGeometry(100, 100, 450, 350)
        self.setStyleSheet(get_service_theme("default", is_dark_mode=True))
        
        self.setWindowIcon(QIcon(resource_path("assets/app_icon.ico")))

        self.main_app_window = None 
        
        self.initUI()
        
        self.setWindowOpacity(0.0)
        self.resize(430, 330)
        QTimer.singleShot(100, self.start_combined_appearance_animation)

    def initUI(self):
        main_layout = QVBoxLayout(self)
        main_layout.setContentsMargins(30, 30, 30, 30)
        
        self.title_label = QLabel("Hangi servisten indirmek istiyorsunuz?", self)
        self.title_label.setAlignment(Qt.AlignCenter)
        self.title_label.setStyleSheet("font-size: 22px; font-weight: bold; padding-bottom: 20px;")
        
        self.youtube_button = QPushButton("YouTube", self)
        self.youtube_button.setObjectName("youtube_button")
        self.youtube_button.setMinimumHeight(60)
        
        self.twitter_button = QPushButton("Twitter (X)", self)
        self.twitter_button.setObjectName("twitter_button")
        self.twitter_button.setMinimumHeight(60)

        self.facebook_button = QPushButton("Facebook", self)
        self.facebook_button.setObjectName("facebook_button")
        self.facebook_button.setMinimumHeight(60)

        self.tiktok_button = QPushButton("TikTok", self)
        self.tiktok_button.setObjectName("tiktok_button")
        self.tiktok_button.setMinimumHeight(60)
        
        self.instagram_button = QPushButton("Instagram", self)
        self.instagram_button.setObjectName("instagram_button")
        self.instagram_button.setMinimumHeight(60)

        self.soundcloud_button = QPushButton("SoundCloud", self)
        self.soundcloud_button.setObjectName("soundcloud_button")
        self.soundcloud_button.setMinimumHeight(60)

        self.reddit_button = QPushButton("Reddit", self)
        self.reddit_button.setObjectName("reddit_button")
        self.reddit_button.setMinimumHeight(60)

        main_layout.addWidget(self.title_label)
        main_layout.addSpacerItem(QSpacerItem(20, 20, QSizePolicy.Minimum, QSizePolicy.Expanding))
        main_layout.addWidget(self.youtube_button)
        main_layout.addWidget(self.twitter_button)
        main_layout.addWidget(self.facebook_button)
        main_layout.addWidget(self.tiktok_button)
        main_layout.addWidget(self.instagram_button)
        main_layout.addWidget(self.soundcloud_button)
        main_layout.addWidget(self.reddit_button)
        main_layout.addSpacerItem(QSpacerItem(20, 20, QSizePolicy.Minimum, QSizePolicy.Expanding))

        self.setLayout(main_layout)
        
        self.youtube_button.clicked.connect(lambda: self.launch_service("youtube"))
        self.twitter_button.clicked.connect(lambda: self.launch_service("twitter"))
        self.facebook_button.clicked.connect(lambda: self.launch_service("facebook"))
        self.tiktok_button.clicked.connect(lambda: self.launch_service("tiktok"))
        self.instagram_button.clicked.connect(lambda: self.launch_service("instagram"))
        self.soundcloud_button.clicked.connect(lambda: self.launch_service("soundcloud"))
        self.reddit_button.clicked.connect(lambda: self.launch_service("reddit")) 

    def launch_service(self, service_name):
        self.main_app_window = DownloaderApp(service=service_name, parent_selection_screen=self)
        self.main_app_window.start_combined_appearance_animation()
        self.hide()

    def start_combined_appearance_animation(self):
        self.show()
        fade_anim = QPropertyAnimation(self, b"windowOpacity")
        fade_anim.setDuration(500)
        fade_anim.setStartValue(0.0)
        fade_anim.setEndValue(1.0)
        
        zoom_anim = QPropertyAnimation(self, b"size")
        zoom_anim.setDuration(500)
        zoom_anim.setStartValue(QSize(430, 330))
        zoom_anim.setEndValue(QSize(450, 350))
        zoom_anim.setEasingCurve(QEasingCurve.OutBack)
        
        fade_anim.start()
        zoom_anim.start()
        self.fade_anim = fade_anim
        self.zoom_anim = zoom_anim

class DownloaderApp(QWidget):
    video_info_signal = Signal(dict)

    def __init__(self, service="youtube", parent_selection_screen=None):
        super().__init__()
        
        self.service = service
        self.parent_selection_screen = parent_selection_screen
        self.info_panel_animation = None
        self.is_dark_mode = True 
        self.cookie_file_path = ""

        self.setWindowIcon(QIcon(resource_path("assets/app_icon.ico")))
        
        if self.service == "youtube":
            self.setWindowTitle("YouTube Video and Audio Downloader")
        elif self.service == "twitter":
            self.setWindowTitle("Twitter (X) Video Downloader")
        elif self.service == "facebook":
            self.setWindowTitle("Facebook Video Downloader")
        elif self.service == "tiktok":
            self.setWindowTitle("TikTok Video Downloader")
        elif self.service == "instagram":
            self.setWindowTitle("Instagram Video & Reels Downloader")
        elif self.service == "soundcloud":
            self.setWindowTitle("SoundCloud Audio Downloader")
        elif self.service == "reddit":
            self.setWindowTitle("Reddit Video Downloader")

        self.setGeometry(100, 100, 800, 600)
        
        self.service_style = get_service_theme(self.service, self.is_dark_mode)
        self.setStyleSheet(self.service_style)
        
        self.current_video_title = "thumbnail"
        
        self.initUI()
        
        self.load_settings() 
        self.check_ffmpeg() 

        if self.service != "youtube":
            self.download_type_label.setVisible(False)
            self.download_type_combo.setVisible(False)
            self.list_type_label.setVisible(False)
            self.list_type_combo.setVisible(False)
            self.quality_label.setVisible(False)
            self.quality_combo.setVisible(False)
            self.subtitle_checkbox.setVisible(False)
            self.subtitle_lang_label.setVisible(False)
            self.subtitle_lang_entry.setVisible(False)

            if self.service == "soundcloud":
                self.format_label.setVisible(True)
                self.format_combo.setVisible(True)
                self.populate_audio_formats() 
            else:
                self.format_label.setVisible(False)
                self.format_combo.setVisible(False)
        
        self.options_group.setVisible(self.service == "youtube" or self.service == "soundcloud")
        
        if self.service == "soundcloud":
            self.download_type_label.setVisible(False)
            self.download_type_combo.setVisible(False)
            self.list_type_label.setVisible(False)
            self.list_type_combo.setVisible(False)
            self.quality_label.setVisible(False)
            self.quality_combo.setVisible(False)
            self.subtitle_checkbox.setVisible(False)
            self.subtitle_lang_label.setVisible(False)
            self.subtitle_lang_entry.setVisible(False)
            self.format_label.setVisible(True)
            self.format_combo.setVisible(True)

        self.setWindowOpacity(0.0)
        self.resize(780, 580)
        
        self.setAttribute(Qt.WA_DeleteOnClose)
        QApplication.clipboard().dataChanged.connect(self.check_clipboard)

    def initUI(self):
        layout = QVBoxLayout()
        layout.setContentsMargins(15, 15, 15, 15)
        layout.setSpacing(10) 

        top_button_layout = QHBoxLayout()
        top_button_layout.setContentsMargins(0, 0, 0, 10)

        self.go_back_button = QPushButton("", self)
        self.go_back_button.clicked.connect(self.go_back_to_selection)
        self.go_back_button.setStyleSheet("max-width: 180px;") 
        top_button_layout.addWidget(self.go_back_button)

        top_button_layout.addStretch() 
        
        self.settings_button = QPushButton("⚙", self)
        self.settings_button.clicked.connect(self.show_settings_dialog)
        self.settings_button.setStyleSheet("max-width: 60px; font-size: 20px;")
        top_button_layout.addWidget(self.settings_button)
        
        layout.addLayout(top_button_layout)

        self.title_label_dynamic = QLabel("", self)
        self.title_label_dynamic.setAlignment(Qt.AlignCenter)
        self.title_label_dynamic.setStyleSheet("font-size: 24px; font-weight: bold;")
        layout.addWidget(self.title_label_dynamic)

        splitter = QSplitter(Qt.Horizontal)

        left_widget = QWidget()
        left_layout = QVBoxLayout(left_widget)
        left_layout.setContentsMargins(0, 0, 10, 0)

        self.input_group = QGroupBox("") 
        self.input_group.setStyleSheet("font-size: 18px; font-weight: bold;")
        input_group_layout = QVBoxLayout(self.input_group)
        input_group_layout.setSpacing(10)

        url_layout = QHBoxLayout()
        self.url_label = QLabel("", self)
        url_layout.addWidget(self.url_label)
        self.url_entry = QLineEdit(self)
        url_layout.addWidget(self.url_entry)
        self.validate_button = QPushButton("", self)
        self.url_entry.editingFinished.connect(self.validate_url)
        url_layout.addWidget(self.validate_button)
        self.validate_button.setVisible(False)
        input_group_layout.addLayout(url_layout)
        self.clipboard_checkbox = QCheckBox("Panoyu İzle", self)
        self.clipboard_checkbox.setChecked(True)
        input_group_layout.addWidget(self.clipboard_checkbox, alignment=Qt.AlignRight)

        folder_layout = QHBoxLayout()
        self.folder_label = QLabel("", self)
        folder_layout.addWidget(self.folder_label)
        self.folder_entry = QLineEdit(self)
        folder_layout.addWidget(self.folder_entry)
        self.browse_button = QPushButton("", self)
        self.browse_button.clicked.connect(self.browse_folder)
        folder_layout.addWidget(self.browse_button)
        input_group_layout.addLayout(folder_layout)
        
        left_layout.addWidget(self.input_group)
        
        self.options_group = QGroupBox("")
        self.options_group.setStyleSheet("font-size: 18px; font-weight: bold;")
        options_group_layout = QVBoxLayout(self.options_group)
        options_group_layout.setSpacing(10)

        self.download_type_label = QLabel("", self)
        options_group_layout.addWidget(self.download_type_label)
        self.download_type_combo = QComboBox(self)
        options_group_layout.addWidget(self.download_type_combo)

        self.list_type_label = QLabel(" ", self)
        options_group_layout.addWidget(self.list_type_label)
        self.list_type_combo = QComboBox(self)
        options_group_layout.addWidget(self.list_type_combo)

        self.quality_label = QLabel(" ", self)
        options_group_layout.addWidget(self.quality_label)
        self.quality_combo = QComboBox(self)
        options_group_layout.addWidget(self.quality_combo)
        
        self.subtitle_checkbox = QCheckBox(self)
        self.subtitle_checkbox.toggled.connect(self.toggle_subtitle_options)
        options_group_layout.addWidget(self.subtitle_checkbox)
        
        self.subtitle_lang_label = QLabel(self)
        self.subtitle_lang_label.setVisible(False)
        options_group_layout.addWidget(self.subtitle_lang_label)
        
        self.subtitle_lang_entry = QLineEdit(self)
        self.subtitle_lang_entry.setPlaceholderText("en,tr,es...")
        self.subtitle_lang_entry.setVisible(False)
        options_group_layout.addWidget(self.subtitle_lang_entry)

        self.format_label = QLabel("", self)
        options_group_layout.addWidget(self.format_label)
        self.format_combo = QComboBox(self)
        options_group_layout.addWidget(self.format_combo)
        
        left_layout.addWidget(self.options_group)
        
        left_layout.addStretch()
        left_widget.setMinimumWidth(450)

        right_widget = QWidget(self)
        right_layout = QVBoxLayout(right_widget)
        right_layout.setSpacing(10)
        
        self.info_panel = QWidget()
        info_panel_layout = QVBoxLayout(self.info_panel)
        info_panel_layout.setContentsMargins(0, 0, 0, 0) 
        info_panel_layout.setSpacing(10)
        
        self.info_panel_opacity = QGraphicsOpacityEffect(self.info_panel)
        self.info_panel.setGraphicsEffect(self.info_panel_opacity)
        self.info_panel_opacity.setOpacity(0.0) 
        self.info_panel.setVisible(False) 

        self.title_label = QLabel("", self)
        self.title_label.setAlignment(Qt.AlignCenter)
        self.title_label.setStyleSheet("font-size: 16px; font-weight: bold;")
        self.title_label.setWordWrap(True)
        info_panel_layout.addWidget(self.title_label)

        self.info_label = QLabel("", self)
        self.info_label.setObjectName("infoLabel")
        self.info_label.setAlignment(Qt.AlignCenter)
        info_panel_layout.addWidget(self.info_label) 

        self.thumbnail_label = QLabel(self)
        self.thumbnail_label.setMinimumSize(400, 225)
        self.thumbnail_label.setAlignment(Qt.AlignCenter)
        self.thumbnail_label.setStyleSheet("border: 2px solid #76D7C4; border-radius: 5px;")
        info_panel_layout.addWidget(self.thumbnail_label, 1) 

        self.save_thumbnail_button = QPushButton(self)
        self.save_thumbnail_button.clicked.connect(self.save_thumbnail)
        self.save_thumbnail_button.setStyleSheet("max-width: 200px; font-size: 12px; padding: 8px 12px;")
        
        thumbnail_button_layout = QHBoxLayout()
        thumbnail_button_layout.addStretch()
        thumbnail_button_layout.addWidget(self.save_thumbnail_button)
        thumbnail_button_layout.addStretch()
        info_panel_layout.addLayout(thumbnail_button_layout) 

        self.progress_bar = ShimmerProgressBar(self)
        self.progress_bar.setMinimum(0)
        self.progress_bar.setMaximum(100)
        info_panel_layout.addWidget(self.progress_bar) 

        self.download_button = QPushButton("", self)
        self.download_button.setStyleSheet("background-color: #76D7C4; color: white;")
        self.download_button.clicked.connect(self.start_download)
        info_panel_layout.addWidget(self.download_button) 

        right_layout.addWidget(self.info_panel, 1)

        splitter.addWidget(left_widget)
        splitter.addWidget(right_widget)
        splitter.setStretchFactor(0, 0)
        splitter.setStretchFactor(1, 1)
        splitter.setCollapsible(0, False)
        splitter.setCollapsible(1, False)
        
        layout.addWidget(splitter, 1)
        self.setLayout(layout)

        self.full_title_text = ""
        self.typed_title = ""
        self.title_index = 0

        self.title_timer = QTimer(self)
        self.title_timer.timeout.connect(self.animate_title)
        
        self.change_language()
        self.list_type_combo.currentIndexChanged.connect(self.toggle_quality)
        
        self.video_info_signal.connect(self.show_video_info)
    
    @Slot()
    def check_clipboard(self):
        if not hasattr(self, 'clipboard_checkbox') or not self.clipboard_checkbox.isChecked():
            return
        
        clipboard_text = QApplication.clipboard().text().strip()
        
        if not clipboard_text:
            return

        if clipboard_text == self.url_entry.text().strip():
            return

        regex = ""
        if self.service == "youtube":
            regex = r"(https?://)?(www\.)?(youtube\.com|youtu\.be|music\.youtube\.com)"
        elif self.service == "twitter":
            regex = r"(twitter\.com|x\.com)"
        elif self.service == "facebook":
            regex = r"(facebook\.com|fb\.watch)"
        elif self.service == "instagram":
            regex = r"(instagram\.com)"
        elif self.service == "tiktok":
            regex = r"(tiktok\.com)"
        elif self.service == "soundcloud":
            regex = r"(soundcloud\.com)"
        elif self.service == "reddit":
             regex = r"(reddit\.com)"

        if regex and re.search(regex, clipboard_text):
            logger.info(f"Pano takibi: Uygun URL tespit edildi -> {clipboard_text}")
            self.url_entry.setText(clipboard_text)
            self.validate_url() 
            
            self.activateWindow() 
            self.raise_()

    @Slot()
    def go_back_to_selection(self):
        if self.parent_selection_screen:
            screen_geometry = self.parent_selection_screen.screen().availableGeometry()
            self.parent_selection_screen.move(
                (screen_geometry.width() - self.parent_selection_screen.width()) // 2,
                (screen_geometry.height() - self.parent_selection_screen.height()) // 2
            )
            self.parent_selection_screen.start_combined_appearance_animation()
        self.close()

    def load_settings(self):
        try:
            folder = settings_manager.get_setting(KEY_DOWNLOAD_FOLDER)
            self.folder_entry.setText(folder)
            
            theme_mode = settings_manager.get_setting(KEY_THEME_MODE)
            self.is_dark_mode = (theme_mode == "dark")
            
            self.cookie_file_path = settings_manager.get_setting(KEY_COOKIE_PATH)
            
        except Exception as e:
            print(f"Ayarlar yüklenirken hata: {e}")

    def save_settings(self):
        try:
            settings_manager.save_setting(KEY_DOWNLOAD_FOLDER, self.folder_entry.text())
            settings_manager.save_setting(KEY_THEME_MODE, "dark" if self.is_dark_mode else "light")
        except Exception as e:
            print(f"Ayarlar kaydedilirken hata: {e}")

    def closeEvent(self, event):
        self.save_settings()
        super().closeEvent(event)

    def check_ffmpeg(self):
        found = False
        
        if shutil.which("ffmpeg") and shutil.which("ffprobe"):
            found = True
            print("FFmpeg sistem PATH'inde bulundu.")

        elif hasattr(sys, '_MEIPASS'):
             meipass_path = os.path.join(sys._MEIPASS, 'ffmpeg', 'bin')
             if os.path.exists(os.path.join(meipass_path, "ffmpeg.exe")):
                 found = True
                 print("FFmpeg, paketlenmiş (MEIPASS) yolda bulundu.")
                 
        elif os.path.exists(os.path.join(TARGET_BIN_PATH, "ffmpeg.exe")):
            found = True
            print(f"FFmpeg '{TARGET_BIN_PATH}' içinde bulundu.")
        
        if not found:
            print("FFmpeg bulunamadı.")
            lang = LANGUAGES.get(self.get_current_language_code(), LANGUAGES["en"])
            
            QMessageBox.warning(self, 
                lang.get("ffmpeg_missing_title", "FFmpeg Missing"),
                lang.get("ffmpeg_warning_text", "FFmpeg bulunamadı. Lütfen Ayarlar -> Gelişmiş menüsünden kurun.")
            )

    def trigger_ffmpeg_installation(self):
        print("FFmpeg indirmesi başlatılıyor.")
        self.start_ffmpeg_download()

    def start_ffmpeg_download(self):
        parent_widget = self.parent_selection_screen if self.parent_selection_screen else self 
        
        self.progress_dialog = QProgressDialog("FFmpeg İndiriliyor...", "İptal", 0, 100, parent_widget)
        self.progress_dialog.setWindowTitle("FFmpeg Kurulumu")
        self.progress_dialog.setWindowModality(Qt.WindowModal)
        self.progress_dialog.setAutoClose(False) 
        self.progress_dialog.setAutoReset(False)
        self.progress_dialog.show()

        self.ffmpeg_thread = FFmpegDownloadThread()
        self.ffmpeg_thread.progress_signal.connect(self.update_ffmpeg_progress)
        self.ffmpeg_thread.finished_signal.connect(self.finish_ffmpeg_download)
        
        self.progress_dialog.canceled.connect(self.ffmpeg_thread.terminate) 
        self.ffmpeg_thread.start()

    @Slot(int, str)
    def update_ffmpeg_progress(self, percent, status):
        if hasattr(self, 'progress_dialog'):
            self.progress_dialog.setValue(percent)
            self.progress_dialog.setLabelText(status)

    @Slot(bool, str)
    def finish_ffmpeg_download(self, success, message):
        if hasattr(self, 'progress_dialog'):
            self.progress_dialog.close() 
        
        parent_widget = self.parent_selection_screen if self.parent_selection_screen else self 
        
        settings_dialog = self.findChild(SettingsDialog)

        if success:
            try:
                current_path = os.environ.get("PATH", "")
                if TARGET_BIN_PATH not in current_path:
                    os.environ["PATH"] = TARGET_BIN_PATH + os.pathsep + current_path
                    print(f"PATH anında güncellendi.")
                
                self.check_ffmpeg() 
                
                if settings_dialog:
                    settings_dialog.check_ffmpeg_status()
                    
                lang_code = self.get_current_language_code()
                if lang_code == "tr":
                    yeni_mesaj = message + "\n\nFFmpeg başarıyla kuruldu ve uygulama için hemen aktifleştirildi. Yeniden başlatma gerekmiyor."
                else:
                    yeni_mesaj = message + "\n\nFFmpeg successfully installed and activated. A restart is no longer required."

                QMessageBox.information(parent_widget, "Başarılı", yeni_mesaj)

            except Exception as e:
                print(f"PATH anında güncellenirken hata: {e}")
                QMessageBox.information(parent_widget, "Başarılı", message + "\n\nPATH değişikliğinin tam olarak uygulanması için uygulamayı yeniden başlatmanız gerekebilir.")
        else:
            QMessageBox.critical(parent_widget, "Hata", message)

        if settings_dialog:
            settings_dialog.install_ffmpeg_button.setEnabled(True)

    def toggle_subtitle_options(self, checked):
        self.subtitle_lang_label.setVisible(checked)
        self.subtitle_lang_entry.setVisible(checked)

    def sanitize_filename(self,filename, max_length=100):
        filename = unicodedata.normalize('NFKD', filename).encode('ASCII', 'ignore').decode('utf-8')
        filename = re.sub(r'[\\/*?:"<>|]', '', filename)
        filename = re.sub(r'[^a-zA-Z0-9\s\-_#\.]', '', filename)
        filename = re.sub(r'[\s]+', ' ', filename)  
        filename = re.sub(r'[_\-]+', '_', filename) 
        filename = filename.strip()
        if len(filename) > max_length:
            try:
                filename = filename[:max_length].rsplit(' ', 1)[0]
            except IndexError:
                filename = filename[:max_length]
        return filename

    def save_thumbnail(self):
        pixmap = self.thumbnail_label.pixmap()
        if not pixmap or pixmap.isNull():
            return
        safe_title = self.sanitize_filename(self.current_video_title)
        default_path = os.path.join(self.folder_entry.text(), f"{safe_title}.png")
        fileName, _ = QFileDialog.getSaveFileName(self, 
            LANGUAGES[self.get_current_language_code()].get("save_thumbnail", "Save Thumbnail"), 
            default_path, 
            "Images (*.png *.jpg)")
        if fileName:
            if pixmap.save(fileName):
                lang = LANGUAGES.get(self.get_current_language_code(), LANGUAGES["en"])
                QMessageBox.information(self, 
                    lang.get("thumbnail_save_success_title", "Success"),
                    lang.get("thumbnail_save_success_text", "Thumbnail saved to {path}").format(path=fileName)
                )
            else:
                QMessageBox.warning(self, "Hata", "Kapak resmi kaydedilemedi.")

    def populate_audio_formats(self):
        language_code = self.get_current_language_code()
        lang = LANGUAGES.get(language_code, LANGUAGES["en"])
        
        self.format_label.setText(lang["format_label_audio"])
        self.format_combo.clear()
        self.format_combo.addItems([
            lang["mp3"], lang["m4a"], lang["ogg"], 
            lang["flac"], lang["opus"], lang["wav"]
        ])

    def toggle_quality(self):
        if self.service == "soundcloud":
            self.populate_audio_formats()
            return
        video_format = self.list_type_combo.currentText()
        self.format_combo.clear()
        language_code = self.get_current_language_code()
        if language_code not in LANGUAGES:
             language_code = "en"
        lang = LANGUAGES[language_code]
        if video_format == lang["list_type_audio"]: 
            self.quality_label.setVisible(False)  
            self.quality_combo.setVisible(False)  
            self.subtitle_checkbox.setVisible(False)
            self.subtitle_lang_label.setVisible(False)
            self.subtitle_lang_entry.setVisible(False)
            self.populate_audio_formats()
        else: 
            if self.service == "youtube":
                self.quality_label.setVisible(True)
                self.quality_combo.setVisible(True)  
                self.subtitle_checkbox.setVisible(True)
                self.toggle_subtitle_options(self.subtitle_checkbox.isChecked())
            
            self.format_label.setText(lang["format_label_video"])
            self.format_combo.addItems([
                lang["mp4"], lang["webm"], lang["mkv"]
            ])

    def toggle_theme(self, save=True):
        self.is_dark_mode = not self.is_dark_mode
        self.service_style = get_service_theme(self.service, self.is_dark_mode)
        self.setStyleSheet(self.service_style)
        
        if save: 
            self.save_settings()

        overlay = QWidget(self)
        overlay.setStyleSheet("background-color: white;" if self.is_dark_mode else "background-color: black;")
        overlay.setGeometry(self.rect())
        overlay.show()
        overlay.raise_()
        opacity_effect = QGraphicsOpacityEffect(overlay)
        overlay.setGraphicsEffect(opacity_effect)
        opacity_effect.setOpacity(1.0)
        fade_out = QPropertyAnimation(opacity_effect, b"opacity", self)
        fade_out.setDuration(300)
        fade_out.setStartValue(1.0)
        fade_out.setEndValue(0.0)
        def remove_overlay():
            overlay.deleteLater()
        fade_out.finished.connect(remove_overlay)
        fade_out.start()

    def get_current_language_code(self):
        try:
            lang_index = settings_manager.get_setting(KEY_LANGUAGE_INDEX)
            
            lang_list = ["", "türkçe", "english", "español", "deutsch", "français", "italiano", "português", "русский", "عربي", "中文"]
            selected_language = lang_list[lang_index].lower()
            
            language_code = {
                "türkçe": "tr",
                "english": "en",
                "español": "es",
                "deutsch": "de",
                "français": "fr",
                "italiano": "it",
                "português": "pt",
                "русский": "ru",
                "عربي": "ar",
                "中文": "zh"
            }.get(selected_language, "tr") 
            return language_code
        except (IndexError, KeyError, Exception) as e:
            print(f"Dil ayarı okunurken hata: {e}. Varsayılan 'tr' kullanılıyor.")
            return "tr" 

    def change_language(self):
        language_code = self.get_current_language_code()
        lang = LANGUAGES.get(language_code, LANGUAGES["tr"])

        if self.service == "youtube":
            title_key = "title"
            default_title = "YouTube Video and Audio Downloader"
        elif self.service == "twitter":
            title_key = "title_twitter"
            default_title = "Twitter (X) Video Downloader"
        elif self.service == "facebook":
            title_key = "title_facebook"
            default_title = "Facebook Video Downloader"
        elif self.service == "tiktok":
            title_key = "title_tiktok"
            default_title = "TikTok Video Downloader"
        elif self.service == "instagram":
            title_key = "title_instagram"
            default_title = "Instagram Video & Reels Downloader"
        elif self.service == "soundcloud":
            title_key = "title_soundcloud"
            default_title = "SoundCloud Audio Downloader"
        elif self.service == "reddit":
            title_key = "title_reddit"
            default_title = "Reddit Video Downloader"
        
        window_title = lang.get(title_key, default_title)
        self.setWindowTitle(window_title)
        self.title_label_dynamic.setText(window_title)
        self.full_title_text = window_title

        self.input_group.setTitle(lang.get("group_input", "Input"))
        self.options_group.setTitle(lang.get("group_options", "Download Options"))

        self.validate_button.setText(lang.get("validate", "Validate"))
        self.download_button.setText(lang.get("download", "Download"))
        self.url_label.setText(lang.get("url_label", "URL:"))
        self.folder_label.setText(lang.get("folder_label", "Download Folder:"))
        self.format_label.setText(lang.get("format_label", "Format:"))
        self.list_type_label.setText(lang.get("format_label", "Format:")) 
        self.download_type_label.setText(lang.get("download_type", "Download Type:"))
        
        self.download_type_combo.clear() 
        self.download_type_combo.addItems([
            lang.get("type_video", "Single Video"),
            lang.get("type_playlist", "Playlist")
        ])

        self.list_type_combo.clear() 
        self.list_type_combo.addItems([
            lang.get("list_type_video", "Video"),
            lang.get("list_type_audio", "Audio")
        ])

        self.quality_combo.clear()
        
        self.subtitle_checkbox.setText(lang.get("subtitle_download", "Download Subtitles"))
        self.subtitle_lang_label.setText(lang.get("subtitle_languages", "Subtitle Languages (e.g., en,tr):"))
        self.save_thumbnail_button.setText(lang.get("save_thumbnail", "Save Thumbnail"))

        self.browse_button.setText(lang.get("browse", "Browse"))
        self.go_back_button.setText(lang.get("go_back", "Go Back"))
        self.settings_button.setToolTip(lang.get("settings_button", "Settings"))

        self.toggle_quality() 

        self.typed_title = ""
        self.title_index = 0
        if self.title_timer.isActive():
            self.title_timer.stop()
        self.title_timer.start(80)

    def validate_url(self):
        url = self.url_entry.text().strip()
        if not url:
            QMessageBox.warning(self, "Hata", "Lütfen bir URL giriniz!")
            return
        self.validate_button.setEnabled(False) 
        current_lang_code = self.get_current_language_code()
        lang = LANGUAGES[current_lang_code]
        if self.service == "youtube":
            regex = r"(https?://)?(www\.)?(youtube\.com/watch\?v=|youtu\.be/|youtube\.com/playlist\?list=|music\.youtube\.com/watch\?v=)"
            error_msg = lang.get("youtube_invalid_url", "Geçersiz YouTube URL'si!")
        elif self.service == "twitter":
            regex = r"(https?://)?(www\.)?(twitter\.com|x\.com)/(.+?)/status/(\d+)"
            error_msg = lang.get("twitter_invalid_url", "Geçersiz Twitter (X) URL'si!")
        elif self.service == "facebook":
            regex = r"(https?://)?(www\.)?(facebook\.com|fb\.watch)/(watch|video\.php|.+/videos|reel)(.+)"
            error_msg = lang.get("facebook_invalid_url", "Geçersiz Facebook URL'si!")
        elif self.service == "tiktok":
            regex = r"(https?://)?(www\.)?(tiktok\.com/(@.+/video|\d+)|vm\.tiktok\.com/([a-zA-Z0-9]+))"
            error_msg = lang.get("tiktok_invalid_url", "Geçersiz TikTok URL'si!")
        elif self.service == "instagram":
            regex = r"(https?://)?(www\.)?(instagram\.com)/(p|reel|tv)/([^/?#&]+)"
            error_msg = lang.get("instagram_invalid_url", "Geçersiz Instagram URL'si!")
        elif self.service == "soundcloud":
            regex = r"(https?://)?(www\.)?(soundcloud\.com)/(.+)"
            error_msg = lang.get("soundcloud_invalid_url", "Geçersiz SoundCloud URL'si!")
        elif self.service == "reddit":
            regex = r"(https?://)?(www\.)?(reddit\.com)/r/([^/]+)/comments/([^/?#&]+)"
            error_msg = lang.get("reddit_invalid_url", "Geçersiz Reddit URL'si!")
        else:
            regex = r"http" 
            error_msg = "Bilinmeyen servis."
        match = re.search(regex, url)
        if match:
            self.fetch_video_info(url) 
            if self.service == "youtube":
                if "playlist?list=" in url or ("&list=" in url and "watch?v=" in url):
                    self.download_type_combo.setCurrentText(lang["type_playlist"])
                else:
                    self.download_type_combo.setCurrentText(lang["type_video"])
        else:
            QMessageBox.critical(self, "Hata", error_msg)
            self.thumbnail_label.clear()
            self.validate_button.setEnabled(True)
            return
        
    def fetch_video_info(self, url):
        def fetch_info():
            video_info = self.get_video_info(url) 
            self.video_info_signal.emit(video_info)
        thread = threading.Thread(target=fetch_info)
        thread.start()

    def get_video_info(self, url):                          
        try:
            ydl_opts = {
                'quiet': True,
                'extract_flat': False, 
                'noplaylist': True,
            }
            
            cookie_path = settings_manager.get_setting(KEY_COOKIE_PATH)
            
            if cookie_path and os.path.exists(cookie_path):
                print(f"Bilgi alınırken kullanılan çerez: {cookie_path}")
                ydl_opts['cookiefile'] = cookie_path

            if self.service == "youtube":
                if "playlist?list=" in url or "&list=" in url:
                    ydl_opts['noplaylist'] = False
                    ydl_opts['extract_flat'] = True
            
            with yt_dlp.YoutubeDL(ydl_opts) as ydl:
                info_dict = ydl.extract_info(url, download=False)
                
                title = info_dict.get('title', 'Başlık Bulunamadı')
                thumbnail = info_dict.get('thumbnail')
                
                uploader = info_dict.get('uploader', 'Bilinmiyor')
                duration = info_dict.get('duration_string', 'N/A')
                views = info_dict.get('view_count')
                views_str = f"{views:,}" if views is not None else "N/A"
                
                quality_list = []
                if self.service == "youtube":
                    video_heights = set()
                    
                    if 'entries' in info_dict and info_dict['entries']:
                        first_video_url = info_dict['entries'][0].get('url')
                        if first_video_url:
                            try:
                                ydl_video_opts = ydl_opts.copy()
                                ydl_video_opts['noplaylist'] = True
                                ydl_video_opts['extract_flat'] = False
                                
                                with yt_dlp.YoutubeDL(ydl_video_opts) as ydl_video:
                                    video_info_dict = ydl_video.extract_info(first_video_url, download=False)
                                    title = info_dict.get('title', 'Playlist Başlığı') 
                                    thumbnail = video_info_dict.get('thumbnail', info_dict.get('thumbnail'))
                                    for f in video_info_dict.get('formats', []):
                                        if f.get('vcodec') != 'none' and f.get('height'):
                                            video_heights.add(f['height'])
                            except Exception as e:
                                print(f"İlk video formatları alınamadı: {e}")
                                
                    else: 
                        for f in info_dict.get('formats', []):
                            if f.get('vcodec') != 'none' and f.get('height'):
                                video_heights.add(f['height'])

                    quality_map = { 
                        4320: "4320p (8K)", 2160: "2160p (4K)", 1440: "1440p", 
                        1080: "1080p", 720: "720p", 480: "480p", 360: "360p", 
                        240: "240p", 144: "144p" 
                    }
                    sorted_heights = sorted(list(video_heights), reverse=True)
                    for h in sorted_heights:
                        if h in quality_map:
                             quality_list.append(quality_map[h])
                        elif h > 4320:
                             quality_list.append(f"{h}p (UHD)")
                        else:
                             quality_list.append(f"{h}p")
                
                if not quality_list: 
                     quality_list = ["N/A"] 

                return {
                    'title': title,
                    'thumbnail': thumbnail,
                    'quality_list': quality_list,
                    'uploader': uploader,
                    'duration': duration,
                    'views': views_str
                }
        except Exception as e:
            print(f"Bilgi alınırken hata: {e}")
            return {"error": str(e)}

    @Slot(dict) 
    def show_video_info(self, video_info):
        self.validate_button.setEnabled(True) 
        
        if "error" in video_info or not video_info:
            QMessageBox.warning(self, "Hata", f"Video bilgileri alınamadı. URL'yi veya ağ bağlantınızı kontrol edin.\n{video_info.get('error','')}")
            
            if self.info_panel_animation and self.info_panel_animation.state() == QPropertyAnimation.Running:
                self.info_panel_animation.stop() 
            
            self.info_panel.setVisible(False)
            self.info_panel_opacity.setOpacity(0.0)
            return
        
        self.current_video_title = video_info.get("title", "Başlık Bulunamadı")
        self.title_label.setText(self.current_video_title)
        
        info_text = (
            f"👤 {video_info.get('uploader', 'N/A')}   |   "
            f"⏳ {video_info.get('duration', 'N/A')}   |   "
            f"👁️ {video_info.get('views', 'N/A')}"
        )
        self.info_label.setText(info_text)
        
        if self.service == "youtube":
            qualities = video_info.get('quality_list')
            self.quality_combo.clear()
            if qualities:
                self.quality_combo.addItems(qualities)
                self.quality_combo.setCurrentIndex(0) 
        
        thumbnail_url = video_info.get("thumbnail")
        if thumbnail_url:
            self.show_thumbnail(thumbnail_url)
        else:
            self.thumbnail_label.clear()

        self.progress_bar.setValue(0) 
        self.progress_bar.setFormat("") 
        self.download_button.setEnabled(True)
        
        self.info_panel.setVisible(True)
        
        if self.info_panel_animation and self.info_panel_animation.state() == QPropertyAnimation.Running:
            self.info_panel_animation.stop()
            
        self.info_panel_animation = QPropertyAnimation(self.info_panel_opacity, b"opacity", self)
        self.info_panel_animation.setDuration(500) 
        self.info_panel_animation.setStartValue(self.info_panel_opacity.opacity()) 
        self.info_panel_animation.setEndValue(1.0) 
        self.info_panel_animation.setEasingCurve(QEasingCurve.OutQuad) 
        
        self.info_panel_animation.finished.connect(self._clear_animation_reference)
        
        self.info_panel_animation.start(QPropertyAnimation.DeleteWhenStopped)

    @Slot()
    def _clear_animation_reference(self):
        self.info_panel_animation = None
    
    def animate_title(self):
        if self.title_index < len(self.full_title_text):
            self.typed_title += self.full_title_text[self.title_index]
            self.title_label_dynamic.setText(self.typed_title)
            self.title_index += 1
        else:
            self.title_timer.stop()
    
    def start_combined_appearance_animation(self):
        self.show()
        fade_anim = QPropertyAnimation(self, b"windowOpacity")
        fade_anim.setDuration(500)
        fade_anim.setStartValue(0.0)
        fade_anim.setEndValue(1.0)
        zoom_anim = QPropertyAnimation(self, b"size")
        zoom_anim.setDuration(500)
        zoom_anim.setStartValue(QSize(780, 580))
        zoom_anim.setEndValue(QSize(800, 600))
        zoom_anim.setEasingCurve(QEasingCurve.OutBack)
        fade_anim.start()
        zoom_anim.start()
        self.fade_anim = fade_anim
        self.zoom_anim = zoom_anim

    def show_thumbnail(self, thumbnail_url):
        try:
            response = requests.get(thumbnail_url, timeout=10)
            if response.status_code == 200:
                image_data = response.content
                pixmap = QPixmap()
                if pixmap.loadFromData(image_data):
                    scaled_pixmap = pixmap.scaled(
                        self.thumbnail_label.size(),
                        Qt.KeepAspectRatio,
                        Qt.SmoothTransformation
                    )
                    self.thumbnail_label.setPixmap(scaled_pixmap)
                    self.thumbnail_label.setAlignment(Qt.AlignCenter)
                    
                else:
                    self.thumbnail_label.setText("Görsel yüklenemedi")
            else:
                self.thumbnail_label.setText(f"HTTP Hatası: {response.status_code}")
        except Exception as e:
            self.thumbnail_label.setText(f"Hata: {str(e)}")

    def start_zoom_animation(self):
        self.setFixedSize(780, 580)
        zoom_anim = QPropertyAnimation(self, b"size")
        zoom_anim.setDuration(500)
        zoom_anim.setStartValue(self.size())
        zoom_anim.setEndValue(QSize(800, 600))
        zoom_anim.setEasingCurve(QEasingCurve.OutBack)
        zoom_anim.start()
        self.zoom_anim = zoom_anim

    def start_download(self):
        if hasattr(self, 'download_thread') and self.download_thread.isRunning():
            lang_code = self.get_current_language_code()
            lang = LANGUAGES.get(lang_code, LANGUAGES["en"])
            QMessageBox.warning(self, 
                lang.get("download_error", "Hata"), 
                "Zaten devam eden bir indirme var. Lütfen bitmesini bekleyin."
            )
            return
        url = self.url_entry.text().strip()
        folder = self.folder_entry.text().strip()
        current_lang_code = self.get_current_language_code()
        lang = LANGUAGES[current_lang_code]
        if self.service == "youtube":
            quality = self.quality_combo.currentText()
            video_format = self.list_type_combo.currentText()
            output_format = self.format_combo.currentText()
            download_type_text = self.download_type_combo.currentText()
            download_type_key = "playlist" if download_type_text == lang["type_playlist"] else "video"
            download_subs = self.subtitle_checkbox.isChecked()
            sub_langs = self.subtitle_lang_entry.text().strip()
        elif self.service == "soundcloud": 
            quality = "best"
            video_format = lang["list_type_audio"] 
            output_format = self.format_combo.currentText()
            download_type_key = "video" 
            download_subs = False
            sub_langs = ""
        else: 
            quality = "best"
            video_format = lang["list_type_video"] 
            output_format = lang["mp4"] 
            download_type_key = "video"
            download_subs = False
            sub_langs = ""
        if not url:
            QMessageBox.warning(self, "Hata", "Lütfen geçerli bir URL giriniz!")
            return
        self.download_button.setEnabled(False)
        self.download_button.setText(lang.get("download_starting", "Download starting..."))
        try:
            self.download_thread = DownloadThread(
                url, folder, quality, video_format, output_format, 
                download_type_key, current_lang_code, download_subs, 
                sub_langs, self.cookie_file_path 
            )
            self.download_thread.progress_signal.connect(self.update_progress)
            self.download_thread.finished_signal.connect(self.download_finished)
            self.download_thread.start()
            self.show_cancel_button()  
        except Exception as e:
            self.download_button.setEnabled(True)
            self.download_button.setText(lang.get("download", "Download"))
            QMessageBox.critical(self, "Hata", f"İndirme başlatılamadı: {e}")

    def show_settings_dialog(self):
        dialog = SettingsDialog(parent=self)
        dialog.exec()

    def start_update_check(self):
        lang = LANGUAGES.get(self.get_current_language_code(), LANGUAGES["en"])
        QMessageBox.information(self, 
            lang.get("update_button", "Check for Updates"),
            lang.get("update_checking", "Checking..."))

        self.update_thread = UpdateThread()
        self.update_thread.finished_signal.connect(self.show_update_result)
        self.update_thread.start()

    @Slot(str)
    def show_update_result(self, message):
        lang = LANGUAGES.get(self.get_current_language_code(), LANGUAGES["en"])
        message_lower = message.lower()
        if "already up-to-date" in message_lower:
            QMessageBox.information(self, 
                "Başarılı", 
                lang.get("update_success", "yt-dlp is already up to date.")
            )
        elif "updated" in message_lower or "successfully" in message_lower:
             QMessageBox.information(self, 
                "Başarılı", 
                lang.get("update_available", "yt-dlp updated successfully!")
            )
        else:
            QMessageBox.warning(self, 
                "Hata", 
                lang.get("update_error", "An error occurred...").format(error=message)
            )

    def show_cancel_button(self):
        current_lang_code = self.get_current_language_code()
        cancel_text_key = "download_canceled" 
        cancel_text = "Cancel"
        lang_text = LANGUAGES[current_lang_code].get(cancel_text_key, "Canceled")
        if "İptal Edildi" in lang_text: cancel_text = "İptal Et"
        elif "Canceled" in lang_text: cancel_text = "Cancel"
        elif "Cancelada" in lang_text: cancel_text = "Cancelar"
        elif "Abgebrochen" in lang_text: cancel_text = "Abbrechen"
        elif "Annulé" in lang_text: cancel_text = "Annuler"
        if not hasattr(self, 'cancel_button'):
            self.cancel_button = QPushButton(cancel_text, self)
            self.cancel_button.setStyleSheet("background-color: #ff4d4d; color: white;") 
            self.cancel_button.clicked.connect(self.cancel_download)
            self.download_button.parentWidget().layout().addWidget(self.cancel_button)
        self.cancel_button.setVisible(True)
        self.cancel_button.setEnabled(True)
        self.cancel_button.setText(cancel_text)

    def cancel_download(self):
        current_lang_code = self.get_current_language_code()
        lang = LANGUAGES[current_lang_code]
        if hasattr(self, 'download_thread') and self.download_thread.isRunning():
            try:
                self.download_thread.terminate()
                self.download_thread.wait()
            except Exception as e:
                print(f"Thread sonlandırılırken hata: {e}")
            self.download_button.setEnabled(True)
            self.download_button.setText(lang.get("download", "Download"))
            self.progress_bar.setValue(0)
            self.progress_bar.setFormat(lang.get("download_canceled", "Download canceled."))
            if hasattr(self, 'cancel_button'):
                self.cancel_button.setEnabled(False)
                self.cancel_button.setText(lang.get("download_canceled", "Download canceled."))
            QMessageBox.information(self, "İptal", lang.get("download_canceled", "Download canceled."))

    @Slot(int, str)
    def update_progress(self, percent, status):
        self.progress_bar.setValue(percent)
        self.progress_bar.setFormat(status) 

    @Slot(str)
    def download_finished(self, status):
        current_lang_code = self.get_current_language_code()
        lang = LANGUAGES[current_lang_code]
        self.download_button.setEnabled(True)
        self.download_button.setText(lang.get("download", "Download"))
        if isinstance(status, str):
            if "✅" in status or "completed" in status.lower() or "tamamlandı" in status.lower():
                self.progress_bar.setValue(100)
                self.progress_bar.setFormat(lang.get("download_complete", "Download completed successfully!"))
                QMessageBox.information(self, "Başarı", status)
            elif "❌" in status or "error" in status.lower() or "hata" in status.lower():
                self.progress_bar.setFormat(lang.get("download_error", "An error occurred during download."))
                QMessageBox.critical(self, "Hata", status)
            else:
                QMessageBox.information(self, "Bilgi", status)
        if hasattr(self, 'cancel_button'):
            opacity_effect = QGraphicsOpacityEffect(self.cancel_button)
            self.cancel_button.setGraphicsEffect(opacity_effect)
            self.fade_animation = QPropertyAnimation(opacity_effect, b"opacity", self)
            self.fade_animation.setDuration(500)
            self.fade_animation.setStartValue(1.0)
            self.fade_animation.setEndValue(0.0)
            def hide_button():
                self.cancel_button.setVisible(False)
                self.cancel_button.setGraphicsEffect(None)
            self.fade_animation.finished.connect(hide_button)
            self.fade_animation.start()
  
    def browse_folder(self):
        folder_selected = QFileDialog.getExistingDirectory(self, "Klasör Seç")
        if folder_selected:
            self.folder_entry.setText(folder_selected)
            self.save_settings()

class ShimmerProgressBar(QProgressBar):
    def __init__(self, parent=None):
        super().__init__(parent)
        self.shimmer_offset = 0
        self.timer = QTimer(self)
        self.timer.timeout.connect(self.update_shimmer)
        self.timer.start(25)
        self.setTextVisible(True)
        self.setFont(QFont("Segoe UI", 10, QFont.Bold))
        self.custom_text = ""

    def setFormat(self, text):
        self.custom_text = text
        self.update() 

    def text(self):
        return self.custom_text

    def update_shimmer(self):
        self.shimmer_offset += 6
        if self.shimmer_offset > self.width() + 150:
            self.shimmer_offset = -150
        self.update()

    def paintEvent(self, event):
        painter = QPainter(self)
        rect = self.rect()
        radius = 12
        painter.setRenderHint(QPainter.Antialiasing)
        painter.setBrush(QColor(40, 40, 40))
        painter.setPen(QPen(QColor(60, 60, 60), 1.5))
        style = self.styleSheet()
        if "background-color: #D3D3D3" in style: 
             painter.setBrush(QColor(211, 211, 211))
             painter.setPen(QPen(QColor(180, 180, 180), 1.5))
        painter.drawRoundedRect(rect, radius, radius)
        progress_ratio = self.value() / self.maximum() if self.maximum() > 0 else 0
        filled_width = int(progress_ratio * rect.width())
        filled_rect = QRectF(rect.left(), rect.top(), filled_width, rect.height())
        base_color_start = QColor(255, 60, 60)
        base_color_end = QColor(200, 0, 0)
        if "background-color: #D3D3D3" in style: 
             base_color_start = QColor(0, 120, 255)
             base_color_end = QColor(0, 80, 200)
        base_gradient = QLinearGradient(rect.left(), rect.top(), rect.right(), rect.bottom())
        base_gradient.setColorAt(0.0, base_color_start)
        base_gradient.setColorAt(1.0, base_color_end)
        painter.setBrush(base_gradient)
        painter.setPen(Qt.NoPen)
        painter.drawRoundedRect(filled_rect, radius, radius)
        shimmer_width = 120
        gradient = QLinearGradient(self.shimmer_offset, 0, self.shimmer_offset + shimmer_width, 0)
        gradient.setColorAt(0.0, QColor(255, 255, 255, 0))
        gradient.setColorAt(0.5, QColor(255, 255, 255, 120))
        gradient.setColorAt(1.0, QColor(255, 255, 255, 0))
        painter.setBrush(gradient)
        painter.drawRoundedRect(filled_rect, radius, radius)
        text_color = QColor(255, 255, 255) 
        if "background-color: #D3D3D3" in style: 
             text_color = QColor(0, 0, 0) 
        painter.setPen(text_color)
        text_rect = rect.adjusted(5, 0, -5, 0)
        text_doc = self.document()
        html_text = self.text().replace("**", "<b>").replace("</b>", "</b>", 1).replace("**", "</b>")
        html_text = html_text.replace("\n", "<br>") 
        text_doc.setHtml(f"<div style='color: {text_color.name()}; text-align: center;'>{html_text}</div>")
        text_doc.setTextWidth(text_rect.width())
        painter.save()
        painter.translate(text_rect.left(), text_rect.top() + (text_rect.height() - text_doc.size().height()) / 2)
        text_doc.drawContents(painter)
        painter.restore()

    def document(self):
        if not hasattr(self, '_document'):
            self._document = QTextDocument(self)
            self._document.setDefaultFont(self.font())
        return self._document

class VideoSplashScreen(QWidget):
    finished = Signal()

    def __init__(self, video_path):
        super().__init__()
        
        self.setWindowFlags(
            Qt.FramelessWindowHint | 
            Qt.WindowStaysOnTopHint | 
            Qt.Tool
        )
        self.setAttribute(Qt.WA_TranslucentBackground)
        self.setAttribute(Qt.WA_DeleteOnClose)

        self.setStyleSheet("background: transparent;")

        self.current_frame = QImage()

        self.player = QMediaPlayer(self)
        self.sink = QVideoSink(self)
        self.player.setVideoSink(self.sink) 

        self.sink.videoFrameChanged.connect(self.on_frame_changed)

        video_file_url = QUrl.fromLocalFile(os.path.abspath(video_path))
        self.player.setSource(video_file_url)

        self.player.playbackStateChanged.connect(self.on_state_changed)
        
        self.player.play()

    @Slot(QVideoFrame)
    def on_frame_changed(self, frame):
        if frame.isValid():
            self.current_frame = frame.toImage().copy()
            self.update() 

    def paintEvent(self, event):
        painter = QPainter(self)
        
        if not self.current_frame.isNull():
            target_rect = self.rect()
            img = self.current_frame.scaled(
                target_rect.size(), 
                Qt.KeepAspectRatio, 
                Qt.SmoothTransformation
            )
            
            x = (target_rect.width() - img.width()) / 2
            y = (target_rect.height() - img.height()) / 2
            
            painter.drawImage(int(x), int(y), img)
        else:
            painter.fillRect(self.rect(), Qt.transparent)

    @Slot(QMediaPlayer.PlaybackState)
    def on_state_changed(self, state):
        if state == QMediaPlayer.PlaybackState.StoppedState:
            self.finished.emit()

if __name__ == '__main__':
    try:
        app = QApplication(sys.argv)
                
        video_splash_path = resource_path("assets/giris.mp4")

        if not os.path.exists(video_splash_path):
            print(f"Hata: Splash video dosyası bulunamadı: {video_splash_path}")
            selection_window = ServiceSelectionScreen()
            screen_geometry = app.primaryScreen().availableGeometry()
            selection_window.move(
                (screen_geometry.width() - selection_window.width()) // 2,
                (screen_geometry.height() - selection_window.height()) // 2
            )
        else:
            global splash 
            splash = VideoSplashScreen(video_splash_path)
            
            screen = app.primaryScreen()
            screen_geometry = screen.availableGeometry()
            
            splash_width = 400 
            splash_height = 400
            
            splash.resize(splash_width, splash_height)
            
            splash.move(
                (screen_geometry.width() - splash_width) // 2,
                (screen_geometry.height() - splash_height) // 2,
            ) 
            splash.show() 
            
            def show_selection_window():
                global selection_window
                selection_window = ServiceSelectionScreen()
                screen_geometry = app.primaryScreen().availableGeometry()
                selection_window.move(
                    (screen_geometry.width() - selection_window.width()) // 2,
                    (screen_geometry.height() - selection_window.height()) // 2
                )
                if 'splash' in globals():
                    splash.close()
            
            splash.finished.connect(show_selection_window)

        sys.exit(app.exec())
        
    except Exception as e:
        logger.critical("Uygulama kritik bir hata ile durdu!", exc_info=True)
        sys.exit(1)