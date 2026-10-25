# Building VidExtract

Use Node.js 22+, stable Rust, Git, CMake and a C/C++ compiler. Only x64 Windows and Linux packages are currently produced. Downloaded native tools, build output, models and personal files are excluded from Git.

## Linux

Ubuntu 22.04 is the release build baseline:

```bash
sudo apt update
sudo apt install build-essential cmake git pkg-config libssl-dev libwebkit2gtk-4.1-dev librsvg2-dev patchelf libayatana-appindicator3-dev libgstreamer1.0-dev gstreamer1.0-plugins-base gstreamer1.0-plugins-good gstreamer1.0-plugins-bad gstreamer1.0-libav gstreamer1.0-pulseaudio
```

Install a Vulkan SDK including headers, loader, `glslc` and SPIRV-Headers CMake configuration. The workflow uses LunarG SDK 1.4.341.1. Export `VULKAN_SDK` and its environment paths as directed by the SDK. A GPU driver is not required to compile the worker; compatible Vulkan drivers are needed to use GPU acceleration.

## Windows

Install Visual Studio 2022 C++ Build Tools, Windows SDK, Node.js 22, Rust MSVC, Git, CMake and Vulkan SDK. Use a Visual Studio developer shell. WebView2 is needed for the app UI. The Windows GitHub runner handles the compiler and SDK setup in the workflow.

## Prepare and run

```bash
npm ci
npm run setup:tools
npm run setup:translation
npm run licenses
npm run tauri dev
```

Native tool preparation downloads verified release assets and builds whisper.cpp and the CTranslate2 adapter. It can take several minutes and is cached in CI. Speech/translation model weights are downloaded by the application only when requested, not during the app build.

```bash
# Windows NSIS installer
npm run tauri -- build --bundles nsis
# Windows Store package, after building the app
powershell -File scripts/package-msix.ps1
# Linux AppImage + DEB, including media plugins
npm run package:linux
```

The Store package uses [packaging/windows/identity.json](packaging/windows/identity.json). Its unsigned `.msix` is for Partner Center submission; it is not a directly installable unsigned release for end users. The EXE installer is a separate GitHub distribution.

## Checks

```bash
npm test
npm run build
npx playwright install chromium
npm run test:ui
node scripts/test-stability.mjs
npm run test:media
cargo test --manifest-path src-tauri/Cargo.toml --locked
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

Linux package playback is tested under real WebKitGTK after packaging: `xvfb-run -a npm run test:playback`. CI also needs PulseAudio, xvfb and xauth for this check. The browser UI test uses controlled IPC; the Rust/media/playback checks cover real native components.

Keep version numbers in package.json, package-lock.json, Cargo.toml, Cargo.lock and tauri.conf.json synchronized. A release tag such as `v2.3.0` runs both platform builds and publishes their artifacts only if both succeed.
