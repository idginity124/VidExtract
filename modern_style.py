SERVICE_COLORS = {
    "default": {
        "dark": {
            "primary": "#FF0000",
            "background": "#0F0F0F",
            "text": "#FFFFFF",
            "secondary": "#1E1E1E"
        },
        "light": {
            "primary": "#FF0000",
            "background": "#EDEDED",
            "text": "#000000",
            "secondary": "#FFFFFF"
        }
    },
    "youtube": {
        "dark": {
            "primary": "#FF0000",
            "background": "#0F0F0F",
            "text": "#FFFFFF",
            "secondary": "#1E1E1E"
        },
        "light": {
            "primary": "#FF0000", 
            "background": "#EDEDED",
            "text": "#000000",
            "secondary": "#FFFFFF"
        }
    },
    "twitter": {
        "dark": {
            "primary": "#1DA1F2",
            "background": "#0F0F0F",
            "text": "#FFFFFF",
            "secondary": "#15202B"
        },
        "light": {
            "primary": "#1DA1F2",
            "background": "#FFFFFF",
            "text": "#000000",
            "secondary": "#F5F8FA"
        }
    },
    "facebook": {
        "dark": {
            "primary": "#1877F2",
            "background": "#18191A",
            "text": "#E4E6EB",
            "secondary": "#242526"
        },
        "light": {
            "primary": "#1877F2",
            "background": "#F0F2F5",
            "text": "#1C1E21",
            "secondary": "#FFFFFF"
        }
    },
    "tiktok": {
        "dark": {
            "primary": "#FF0050",
            "background": "#0F0F0F",
            "text": "#FFFFFF",
            "secondary": "#1E1E1E"
        },
        "light": {
            "primary": "#FF0050",
            "background": "#FFFFFF",
            "text": "#000000",
            "secondary": "#F8F8F8"
        }
    },
    "instagram": {
        "dark": {
            "primary": "#E4405F",
            "background": "#0F0F0F",
            "text": "#FFFFFF",
            "secondary": "#1E1E1E"
        },
        "light": {
            "primary": "#E4405F",
            "background": "#FFFFFF",
            "text": "#000000",
            "secondary": "#F8F8F8"
        }
    },
    "soundcloud": {
        "dark": {
            "primary": "#FF5500",
            "background": "#0F0F0F",
            "text": "#FFFFFF",
            "secondary": "#1E1E1E"
        },
        "light": {
            "primary": "#FF5500",
            "background": "#EDEDED",
            "text": "#000000",
            "secondary": "#FFFFFF"
        }
    },
    "reddit": {
        "dark": {
            "primary": "#FF4500",
            "background": "#0F0F0F",
            "text": "#FFFFFF",
            "secondary": "#1E1E1E"
        },
        "light": {
            "primary": "#FF4500",
            "background": "#EDEDED",
            "text": "#000000",
            "secondary": "#FFFFFF"
        }
    }
}

def _generate_theme_qss(palette):
    return f"""
    QWidget {{
        background-color: {palette['background']};
        color: {palette['text']};
        font-family: 'Segoe UI', sans-serif;
        font-size: 16px;
    }}

    QLabel#title {{
        font-size: 28px;
        font-weight: 700;
        color: {palette['primary']};
        padding: 20px 0;
        text-align: center;
    }}
    
    QLabel#infoLabel {{
        font-size: 14px;
        color: {palette['text']};
        padding: 5px 0;
        text-align: center;
    }}

    QLineEdit, QComboBox {{
        background-color: {palette['secondary']};
        color: {palette['text']};
        border: 2px solid {palette['primary']};
        border-radius: 8px;
        padding: 8px;
        font-size: 14px;
    }}

    QComboBox:hover, QLineEdit:hover {{
        border-color: {palette['primary']};
        background-color: {palette['secondary']};
    }}

    QCheckBox {{
        spacing: 10px;
        color: {palette['text']};
    }}
    QCheckBox::indicator {{
        width: 20px;
        height: 20px;
        background-color: {palette['secondary']};
        border: 2px solid {palette['primary']};
        border-radius: 5px;
    }}
    QCheckBox::indicator:checked {{
        background-color: {palette['primary']};
        border: 2px solid {palette['secondary']};
    }}

    QPushButton {{
        background-color: {palette['primary']};
        color: white;
        font-size: 16px;
        font-weight: bold;
        border-radius: 15px;
        padding: 12px 20px;
    }}

    QPushButton:hover {{
        background-color: {palette['primary']};
    }}
    
    QProgressBar {{
        background-color: {palette['secondary']};
        border: 2px solid {palette['primary']};
        border-radius: 10px;
        height: 25px;
        color: {palette['text']};
    }}

    QProgressBar::chunk {{
        background-color: {palette['primary']};
        border-radius: 10px;
    }}

    QMessageBox {{
        background-color: {palette['background']};
        color: {palette['text']};
        font-size: 16px;
    }}

    QScrollArea {{
        background-color: {palette['background']};
        padding: 15px;
    }}

    QTabWidget::pane {{
        border-top: 2px solid {palette['primary']};
        background: {palette['secondary']};
        border-radius: 8px;
        margin-top: -2px;
    }}

    QTabBar::tab {{
        font-size: 16px;
        font-weight: bold;
        padding: 10px 20px;
        background: transparent;
        color: {palette['text']};
        border: none;
        border-top-left-radius: 8px;
        border-top-right-radius: 8px;
    }}

    QTabBar::tab:!selected {{
        background: {palette['background']};
        color: #AAAAAA;
        margin-top: 4px;
    }}

    QTabBar::tab:!selected:hover {{
        background: {palette['secondary']};
        color: {palette['text']};
    }}

    QTabBar::tab:selected {{
        background: {palette['secondary']};
        color: {palette['primary']};
        border-bottom: 2px solid {palette['secondary']};
    }}

    QPushButton#youtube_button {{
        background-color: #FF0000;
        color: white;
        font-size: 20px;
        font-weight: bold;
        border-radius: 10px;
    }}
    QPushButton#youtube_button:hover {{ background-color: #CC0000; }}

    QPushButton#twitter_button {{
        background-color: #000000;
        color: white;
        font-size: 20px;
        font-weight: bold;
        border-radius: 10px;
    }}
    QPushButton#twitter_button:hover {{ background-color: #333333; }}

    QPushButton#facebook_button {{
        background-color: #1877F2;
        color: white;
        font-size: 20px;
        font-weight: bold;
        border-radius: 10px;
    }}
    QPushButton#facebook_button:hover {{ background-color: #166FE5; }}

    QPushButton#tiktok_button {{
        background: qlineargradient(x1:0, y1:0, x2:1, y2:0, stop:0 #000000, stop:1 #FF0050);
        color: white;
        font-size: 20px;
        font-weight: bold;
        border-radius: 10px;
    }}
    QPushButton#tiktok_button:hover {{
        background: qlineargradient(x1:0, y1:0, x2:1, y2:0, stop:0 #333333, stop:1 #FF3366);
    }}

    QPushButton#instagram_button {{
        background: qlineargradient(x1:0, y1:0, x2:1, y2:0, stop:0 #833AB4, stop:0.5 #FD1D1D, stop:1 #F77737);
        color: white;
        font-size: 20px;
        font-weight: bold;
        border-radius: 10px;
    }}
    QPushButton#instagram_button:hover {{
        background: qlineargradient(x1:0, y1:0, x2:1, y2:0, stop:0 #9A4FC6, stop:0.5 #FF4747, stop:1 #FF9957);
    }}

    QPushButton#soundcloud_button {{
        background-color: #FF5500;
        color: white;
        font-size: 20px;
        font-weight: bold;
        border-radius: 10px;
    }}
    QPushButton#soundcloud_button:hover {{ background-color: #FF7733; }}
    
    QPushButton#reddit_button {{
        background-color: #FF4500;
        color: white;
        font-size: 20px;
        font-weight: bold;
        border-radius: 10px;
    }}
    QPushButton#reddit_button:hover {{ background-color: #CC3700; }}
    """

def get_service_theme(service_name="default", is_dark_mode=True):
    colors = SERVICE_COLORS.get(service_name, SERVICE_COLORS["default"])
    palette = colors.get("dark" if is_dark_mode else "light", colors.get("dark"))
    return _generate_theme_qss(palette)

RESPONSIVE_LAYOUT = """
    QWidget {
        min-width: 900px;
        min-height: 650px;
    }
    
    QLabel {
        font-size: 18px;
    }

    QPushButton#main_action_button {
        width: 150px;
        height: 45px;
    }

    QLineEdit, QComboBox {
        width: 320px;
        height: 35px;
    }

    QProgressBar {
        width: 100%;
        height: 35px;
    }
"""