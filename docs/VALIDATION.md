# Validation scope

Local Linux validation covers media download/playback, real speech recognition, Vulkan GPU and CPU fallback, source-language display, online translation of synthetic text, offline translation across ten target languages, a two-pass Turkish→French route, language-pack download/checksum/extraction, cancellation, cache reuse and subtitle export.

The browser suite covers all ten interface/speech choices, RTL, light/dark themes and translation method selection. A separate test samples animation frames across 20 navigation transitions and six popup openings. Rust tests cover parsing, device recommendations, media streaming and translation boundaries/routes.

These checks do not imply universal GPU, distribution, accessibility or minimum-device certification. Release packages are prepared locally. The optional GitHub Actions workflow runs only when manually requested. A green build does not substitute for Store certification or manual end-to-end use on every supported device.

The Windows x64 application was cross-compiled locally using cargo-xwin. Windows speech recognition (automatic English detection), French translation, FFmpeg and Vulkan device discovery were exercised through an isolated Wine prefix. The NSIS installer was built locally; the MSIX was packaged using Microsoft MSIX SDK with its validation parser enabled and its Store identity inspected. Native Windows GUI installation and Microsoft Store certification still require testing on Windows.
