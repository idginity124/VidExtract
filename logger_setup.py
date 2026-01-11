import logging
import os
import sys
from logging.handlers import RotatingFileHandler

def setup_logger(name="VidExtract"):
    """
    Uygulama genelinde kullanılacak Logger yapılandırması.
    Logları 'AppData/Local/VidExtract/logs/app_debug.log' dosyasına kaydeder.
    """
    
    app_data_path = os.path.join(os.getenv('LOCALAPPDATA'), 'VidExtract')
    log_dir = os.path.join(app_data_path, 'logs')
    
    if not os.path.exists(log_dir):
        os.makedirs(log_dir, exist_ok=True)
        
    log_file_path = os.path.join(log_dir, 'app_debug.log')

    logger = logging.getLogger(name)
    logger.setLevel(logging.DEBUG)

    formatter = logging.Formatter('%(asctime)s - %(levelname)s - %(module)s - %(message)s')

    file_handler = RotatingFileHandler(log_file_path, maxBytes=5*1024*1024, backupCount=2, encoding='utf-8')
    file_handler.setFormatter(formatter)
    file_handler.setLevel(logging.DEBUG)

    console_handler = logging.StreamHandler(sys.stdout)
    console_handler.setFormatter(formatter)
    console_handler.setLevel(logging.INFO)

    if not logger.handlers:
        logger.addHandler(file_handler)
        logger.addHandler(console_handler)

    return logger

logger = setup_logger()