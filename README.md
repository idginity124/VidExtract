<div align="center">
  <img src="assets/brand-mark.png" width="100" alt="VidExtract logo">
  <h1>VidExtract</h1>
  <p><strong>Download a video. Find the sentence. Jump to the moment.</strong></p>
  <p>Windows · Linux · Local transcription · Online & offline translation</p>
  <p>
    <a href="https://github.com/idginity124/VidExtract/actions/workflows/desktop.yml"><img alt="Desktop builds" src="https://github.com/idginity124/VidExtract/actions/workflows/desktop.yml/badge.svg"></a>
    <a href="LICENSE"><img alt="GPLv3" src="https://img.shields.io/badge/license-GPLv3-blue"></a>
    <a href="https://github.com/idginity124/VidExtract/releases"><img alt="Downloads" src="https://img.shields.io/github/downloads/idginity124/VidExtract/total"></a>
  </p>
  <p><a href="https://github.com/idginity124/VidExtract/releases">Download</a> · <a href="docs/README.tr.md">Türkçe</a> · <a href="PRIVACY.md">Privacy</a> · <a href="CONTRIBUTING.md">Contribute</a></p>
</div>

VidExtract brings media downloading, local speech recognition, timestamp search, translation and video clips into one desktop app. Built with **Tauri 2, React and Rust**. The 2.x desktop source is now available under GPLv3; the former Python interface is retired.

![VidExtract Studio](docs/screenshots/studio-dark.png)

## Download

Get packages from [GitHub Releases](https://github.com/idginity124/VidExtract/releases). Verify the platform's `SHA256SUMS` file if needed.

| Platform | Package | Notes |
|---|---|---|
| Windows x64 | `.exe` installer | Windows 10/11 with Microsoft Edge WebView2; the installer can set up the runtime |
| Linux x64 | `.AppImage` | Portable package; make executable before launching |
| Debian / Ubuntu x64 | `.deb` | Install with `sudo apt install ./VidExtract_*.deb` |
| Microsoft Store | `.msix` submission package | Store publication is being prepared; the submission package is not a signed sideload installer |

Windows direct-download installers are not currently Authenticode signed. Microsoft Store signs accepted MSIX packages during publication. Linux release builds run on Ubuntu 22.04 to avoid building against a newer workstation's glibc. Other distributions and all GPUs are not universally certified.

```bash
chmod +x VidExtract_*.AppImage
./VidExtract_*.AppImage
```

## What you can do

- Download video or audio, choose format/quality, process playlists and manage a queue.
- Open an existing media file and transcribe locally with multilingual Tiny, Base or Small models.
- Detect the spoken language automatically and display its full name.
- Search transcripts and click a timestamp to play that moment.
- Translate beside the original text, then export TXT, SRT or VTT.
- Save a selected time range as an MP4 clip.
- Choose dark/light appearance and one of ten interface languages, including right-to-left Arabic.

**Languages:** Turkish, English, Spanish, French, German, Portuguese, Russian, Arabic, Hindi and Chinese. Interface, source speech language and translation target are independent.

## Local speech recognition and GPU

Speech recognition runs through whisper.cpp on your computer. Models are downloaded once when requested: Tiny ~78 MB, Base ~148 MB, Small ~488 MB. CPU/RAM measurements guide the recommendation and thread count. Compatible devices can use **Vulkan GPU acceleration**; CPU mode remains available, and GPU failures retry on CPU. Portable CPU and AVX2 workers are selected according to actual CPU instruction support.

Long recordings are processed in bounded audio sections. Automatic transcription and translation can contain mistakes; review important exports.

## Two translation choices

**Online:** MyMemory requires no account, API key or model download. Its published anonymous allowance is 5,000 characters/day; a valid optional email can raise that to 50,000. Only the text is sent, with normal network metadata and the email if supplied. [Provider limits](https://mymemory.translated.net/doc/usagelimits.php) may change. Completed segments are cached locally to avoid repeat requests.

**Offline:** download only the required language-direction packs. Each is approximately **65–286 MB**; some pairs require two packs through English. The exact missing download size is shown before installation. A small CTranslate2 CPU worker processes bounded batches; no translation text leaves the computer. Language/model quality varies.

## Build from source

See [BUILDING.md](BUILDING.md) for dependencies, Vulkan SDK and platform commands. Initial preparation needs internet, Node.js 22+, stable Rust, Git, CMake and a C++ compiler.

```bash
npm ci
npm run setup:tools
npm run setup:translation
npm run licenses
npm run tauri dev
```

```bash
npm test
npm run build
npm run test:ui                 # first: npx playwright install chromium
cargo test --manifest-path src-tauri/Cargo.toml --locked
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

[GitHub Actions](.github/workflows/desktop.yml) builds Windows and Linux independently, exercises the UI/media code and prepares installers plus a Store MSIX. Tagged releases publish only after both platform jobs pass.

## Microsoft Store and privacy

Store ID: `9P5V676WV12Z`. Publication is in progress; reserving a name is not the same as having a live listing. See [the Store packaging guide](docs/MICROSOFT-STORE.md) and [listing copy](docs/STORE-LISTING.md).

Read [PRIVACY.md](PRIVACY.md) for clipboard behavior, local files, device detection and external services. Download only media you are authorized to access; site availability and permitted use remain subject to the source service and applicable rights.

## Open source

Copyright © 2026 Ekrem Bulgan. VidExtract's original code is **GPL-3.0-only**. Third-party software and models retain their licenses; see [THIRD_PARTY.md](THIRD_PARTY.md). Contributions and reproducible bug reports are welcome through [GitHub Issues](https://github.com/idginity124/VidExtract/issues).
