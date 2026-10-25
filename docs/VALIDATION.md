# Validation scope

Local Linux validation covers media download/playback, real speech recognition, Vulkan GPU and CPU fallback, source-language display, online translation of synthetic text, offline translation across ten target languages, a two-pass Turkish→French route, language-pack download/checksum/extraction, cancellation, cache reuse and subtitle export.

The browser suite covers all ten interface/speech choices, RTL, light/dark themes and translation method selection. A separate test samples animation frames across 20 navigation transitions and six popup openings. Rust tests cover parsing, device recommendations, media streaming and translation boundaries/routes.

These checks do not imply universal GPU, distribution, accessibility or minimum-device certification. Release packages are prepared locally. The optional GitHub Actions workflow runs only when manually requested. A green build does not substitute for Store certification or manual end-to-end use on every supported device.
