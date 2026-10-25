VidExtract 2.3.0 opens the Tauri 2 / React / Rust desktop source under GPLv3.

- Windows x64 installer, Linux x64 AppImage and Debian/Ubuntu DEB.
- Local speech recognition with automatic source-language detection and optional Vulkan GPU acceleration; CPU fallback remains available.
- Online MyMemory translation without required keys/accounts, or small offline language-direction packs.
- Timestamp search, side-by-side translation, TXT/SRT/VTT export and MP4 clips.
- Ten interface languages, dark/light themes and corrected stationary dropdown/tab transitions.

Choose the `.exe` on Windows, `.AppImage` for portable Linux use or `.deb` for Debian/Ubuntu. The `-store.msix` file is an unsigned Microsoft Store submission artifact, not a normal sideload installer. Store publication and certification are separate from this GitHub release.

Model/language packs download only when requested. Online translation sends text to MyMemory and has a daily quota. See the repository's privacy policy and third-party notices. Windows EXE downloads are currently unsigned. SHA-256 manifests and dependency notices are attached for each platform.

Built on the maintainer’s Linux PC. Windows workers were smoke-tested through Wine; native Windows GUI installation and Store certification remain pending. See docs/VALIDATION.md for the validation scope.
