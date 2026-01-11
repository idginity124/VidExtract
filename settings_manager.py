from logger_setup import logger
from PySide6.QtCore import QSettings

class _SettingsManager:
    def __init__(self):
        self.qsettings = QSettings("VidExtract", "DownloaderApp")

        self.KEY_DOWNLOAD_FOLDER = "download_folder"
        self.KEY_LANGUAGE_INDEX = "language_index"
        self.KEY_THEME_MODE = "theme_mode"
        self.KEY_COOKIE_PATH = "cookie_file_path"
        self.KEY_CLIPBOARD_MONITOR = "clipboard_monitor"
        
        self.DEFAULT_VALUES = {
            self.KEY_DOWNLOAD_FOLDER: "downloads",
            self.KEY_LANGUAGE_INDEX: 1, 
            self.KEY_THEME_MODE: "dark",
            self.KEY_COOKIE_PATH: "",
            self.KEY_CLIPBOARD_MONITOR: True 
        }

    def save_setting(self, key, value):
        if value is None:
            logger.warning(f"Ayarlar: '{key}' anahtarı için None değeri kaydedilmeye çalışıldı.")
            return

        self.qsettings.setValue(key, value)
        self.qsettings.sync() 

    def get_setting(self, key):
        if key not in self.DEFAULT_VALUES:
            raise KeyError(f"Ayar anahtarı '{key}' için varsayılan değer tanımlanmamış.")

        default_value = self.DEFAULT_VALUES[key]
        expected_type = type(default_value)
        value = self.qsettings.value(key, defaultValue=default_value, type=expected_type)
        
        try:
            if expected_type == int:
                return int(value)
            elif expected_type == bool:
                return value_to_bool(value)
            return expected_type(value)
        except (ValueError, TypeError):
            return default_value
            
    def get_all_settings(self):
        return {
            key: self.get_setting(key)
            for key in self.DEFAULT_VALUES
        }

def value_to_bool(value):
    if isinstance(value, bool):
        return value
    if isinstance(value, str):
        return value.lower() in ('true', '1', 't', 'yes', 'y')
    if isinstance(value, int):
        return value != 0
    return False

settings_manager = _SettingsManager()

KEY_DOWNLOAD_FOLDER = settings_manager.KEY_DOWNLOAD_FOLDER
KEY_LANGUAGE_INDEX = settings_manager.KEY_LANGUAGE_INDEX
KEY_THEME_MODE = settings_manager.KEY_THEME_MODE
KEY_COOKIE_PATH = settings_manager.KEY_COOKIE_PATH
KEY_CLIPBOARD_MONITOR = settings_manager.KEY_CLIPBOARD_MONITOR