# Third-party software and models

VidExtract's original source code is licensed under **GPL-3.0-only**. Third-party projects retain their own licenses. Source access and licenses are provided separately; optional model weights are not included in the application repository or installer.

| Component | Role | Upstream / source | License |
|---|---|---|---|
| Tauri | Native desktop shell | https://github.com/tauri-apps/tauri | MIT / Apache-2.0 |
| React | Interface | https://github.com/facebook/react | MIT |
| Lucide | Interface icons | https://github.com/lucide-icons/lucide | ISC |
| Inter / Noto Sans fonts | Local interface fonts | https://github.com/fontsource/fontsource | SIL OFL-1.1 |
| yt-dlp | Media downloads | https://github.com/yt-dlp/yt-dlp | Unlicense; bundled dependencies have separate licenses |
| FFmpeg / FFprobe | Media processing | https://ffmpeg.org/ ; https://github.com/BtbN/FFmpeg-Builds | The distributed BtbN GPL build is GPLv3; included libraries retain their licenses |
| Deno | JavaScript runtime for supported extractors | https://github.com/denoland/deno | MIT; third-party notices also apply |
| whisper.cpp | Local transcription | https://github.com/ggml-org/whisper.cpp/tree/d1be6fde11ac6e0407606b4e42fe72d34add8037 | MIT |
| Whisper model weights | Optional speech models | https://huggingface.co/ggerganov/whisper.cpp | MIT |
| CTranslate2 4.8.3 | Offline translation | https://github.com/OpenNMT/CTranslate2/tree/v4.8.3 | MIT |
| SentencePiece 0.2.1 | Offline tokenizer | https://github.com/google/sentencepiece/tree/v0.2.1 | Apache-2.0 |
| Ruy / cpuinfo / cpu_features | CPU computation and detection | https://github.com/google/ruy ; https://github.com/pytorch/cpuinfo ; https://github.com/google/cpu_features | Apache-2.0 / BSD-2-Clause / Apache-2.0 |
| spdlog / nlohmann-json | Native engine dependencies | https://github.com/gabime/spdlog ; https://github.com/nlohmann/json | MIT |
| Argos language packs | Optional translation data | https://github.com/argosopentech/argospm-index | See each downloaded package's metadata / README and upstream model license |
| GStreamer / GTK / WebKitGTK | Linux media and UI runtime | Distribution packages and their source repositories | LGPL / BSD and plugin-specific licenses |

## Exact versions and notices

Each platform's release includes `tool-versions.json`, `translation-versions.json` and dependency license notices. The packaged `binaries` directory contains license texts. `scripts/collect-licenses.mjs` collects npm and Cargo dependency notices, including local font licenses, before packaging. The media fixture's provenance is documented in [tests/fixtures/README.md](tests/fixtures/README.md).

## Rebuilding third-party tools

- [scripts/setup-tools.mjs](scripts/setup-tools.mjs) downloads upstream release assets with SHA-256 checks and builds whisper.cpp from a fixed commit.
- [scripts/setup-translation.mjs](scripts/setup-translation.mjs) builds the CPU translation worker, including its pinned native dependencies and their original licenses.
- [src-tauri/native](src-tauri/native) contains VidExtract's translation adapter and CMake build configuration.
- [src-tauri/translation-packs.json](src-tauri/translation-packs.json) records the source URL, version, byte count and SHA-256 of every optional language pack.

For FFmpeg and its enabled libraries, use the source revisions, dependency scripts and patches in the BtbN build repository associated with the release asset, alongside FFmpeg's printed build/version configuration and the platform's tool manifest. BtbN's source/build instructions are at https://github.com/BtbN/FFmpeg-Builds. Retain corresponding source access and all notices when redistributing these components; VidExtract's GPL license does not replace their individual requirements.
