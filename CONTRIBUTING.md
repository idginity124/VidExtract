# Contributing

Thank you for helping improve VidExtract. Contributions to the original application code are accepted under GPL-3.0-only.

Start with [BUILDING.md](BUILDING.md). Keep changes focused, describe the user-visible problem and include the validation relevant to your change. Do not commit models, downloaded executables, user media, local settings, API credentials, build output or signing keys.

- Interface: `src/` (React / TypeScript), translations in `src/i18n/`.
- Desktop services: `src-tauri/src/` (Rust / Tauri).
- Small offline translation adapter: `src-tauri/native/` (C++).
- Reproducible tool preparation / packaging: `scripts/`.

All ten locale catalogs must contain the same keys and placeholders. Preserve keyboard navigation, Arabic RTL, reduced motion and dark/light themes. Device recommendations must continue to work without a dedicated GPU.

For bugs, provide OS/version, VidExtract version, relevant steps and the smallest non-private sample that reproduces the problem. Do not post private transcripts or links. Security issues should follow [SECURITY.md](SECURITY.md).
