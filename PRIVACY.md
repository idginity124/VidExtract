# VidExtract Privacy Policy

**Last updated: 25 October 2026**
**Publisher: Ekrem Bulgan**

VidExtract is an open-source desktop application for downloading, transcribing, searching and translating media. You do not need a VidExtract account. The application does not implement advertising, analytics or a publisher-operated data collection server.

## Files and speech recognition

Videos, audio, transcriptions, download history, device preferences and translation caches are stored on your computer. Speech recognition runs locally through whisper.cpp. VidExtract does not upload your audio or video to a speech-recognition service. Local playback uses a loopback server on 127.0.0.1 with random access tokens; it does not publish your files to the internet.

The application reads CPU, available RAM and graphics-adapter information to choose suitable settings. These hardware measurements are used locally.

## Online translation: MyMemory

When you select online translation and start translating, the text being translated is sent over HTTPS to **MyMemory, operated by Translated**. The provider receives the text, your network IP address and normal HTTP connection information. Do not submit confidential or personal text unless you are comfortable sending it to this provider.

An email address is optional. If you save one for MyMemory's higher free allowance, it is stored locally and included in online translation requests. You can remove it by clearing the email field and saving. No payment card or VidExtract account is required. VidExtract uses the provider's translation lookup endpoint, not the endpoint for contributing translations to a public translation memory. Provider handling and retention are governed by its own terms and policies, not by VidExtract.

- [MyMemory API and optional email parameter](https://mymemory.translated.net/doc/spec.php)
- [MyMemory usage limits](https://mymemory.translated.net/doc/usagelimits.php)
- [MyMemory terms](https://mymemory.translated.net/doc/tos.php)

Completed text segments are cached locally to avoid resending the same text. This also allows completed parts to be reused after cancellation or a network error.

## Offline translation

When you select offline translation, language packs are downloaded from the Argos model hosting service when you request them. The model host receives ordinary download connection information, including your IP address. After installation, translation runs locally using CTranslate2 and SentencePiece; the text is not sent to a translation provider. Some language pairs use English as an intermediate language.

## Downloads, models and clipboard

Analyzing and downloading a media URL connects to that website and any required media hosting endpoints through yt-dlp. Those services receive your IP address and the requested URL. Optional speech model downloads connect to Hugging Face; desktop distributions and dependency downloads may connect to GitHub and Microsoft services.

If clipboard assistance is enabled, VidExtract reads the clipboard while the app is focused to recognize media links. Unrelated clipboard content is not uploaded by a VidExtract analytics service. You can disable clipboard assistance in Settings.

## Removal and support

You control exported files and downloads through your file manager. Transcripts, settings, history, cached translations and downloaded models are kept in the application's operating-system data directory. Uninstalling may leave these local files; remove the application data directory separately if you want to erase them. Microsoft Store packaging may redirect the application data directory to its package-specific location.

Support: [GitHub Issues](https://github.com/idginity124/VidExtract/issues). Issues are public. Do not attach private transcripts, email addresses, media files or other personal data to a public issue. This policy will be updated when the application's data practices change.
