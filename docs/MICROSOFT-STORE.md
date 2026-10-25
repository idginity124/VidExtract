# Microsoft Store submission

VidExtract uses the **MSIX or PWA app** Store product type and a full-trust **MSIX desktop package**. It is not a PWA. Direct-download NSIS EXE installers remain separate GitHub release assets.

## Registered identity

The non-secret package metadata lives in `packaging/windows/identity.json`:

- Name: `EkremBulgan.VidExtract`
- Publisher: `CN=AA8BCA5B-124C-4770-90E7-C0618BC9398D`
- Publisher display name: `Ekrem Bulgan`
- Store ID: `9P5V676WV12Z`

The workflow builds the native Windows application, copies the required resources and tools, generates a manifest for all ten interface languages, and validates/packages it with Windows SDK MakeAppx. Version 2.3.0 becomes MSIX version 2.3.0.0. Store versions must increase and the fourth component is reserved as zero.

## Upload

1. Build and validate the local Windows package.
2. Find `VidExtract_2.3.0_x64-store.msix` in the exported release directory.
3. In Partner Center → VidExtract → Submission → Packages, upload that MSIX. Do not upload the Linux package or NSIS installer to the MSIX product.
4. Complete descriptions, screenshots, privacy URL, age questionnaire and restricted-capability explanation.
5. Submit only after reviewing the certification preview. Building a package does not itself submit or publish the application.

The package declares `runFullTrust`: it is a desktop tool that invokes local FFmpeg, yt-dlp, speech-recognition and translation workers, and reads/writes files selected by the user. It does not install drivers or a privileged system service.

Microsoft signs accepted Store MSIX packages. The unsigned submission artifact cannot be installed by ordinary sideloading without a matching development or distribution signature. No self-signed development certificate is included in public releases.

## Listing resources

- Privacy URL after the repository update: https://github.com/idginity124/VidExtract/blob/main/PRIVACY.md
- Support: https://github.com/idginity124/VidExtract/issues
- Listing copy: [STORE-LISTING.md](STORE-LISTING.md)
- Primary category: Photo + video; secondary: Productivity.

Disclose that transcription models and offline translation language packs are optional external downloads, online translation sends text to MyMemory and has a free quota, and automated results can contain errors. The Windows UI requires Microsoft Edge WebView2; verify the supported runtime on the certification machine. Do not claim accessibility certification or tested minimum RAM until those have been verified.

References: [Microsoft MSIX preparation](https://learn.microsoft.com/en-us/windows/msix/desktop/desktop-to-uwp-manual-conversion), [Store signing](https://learn.microsoft.com/en-us/windows/apps/publish/get-started), [Tauri distribution](https://v2.tauri.app/distribute/microsoft-store/).

## Local packaging from a Linux cross-build

Stage Windows-only tools in `packaging/windows/binaries` and build the application with cargo-xwin. Set `MAKEMSIX_PATH` to the official Microsoft MSIX SDK `makemsix` executable (built with `USE_VALIDATION_PARSER=ON`), then run `node scripts/package-msix-local.mjs`. It reads the checked-in product identity and writes the unsigned submission package to `store-output`. The native Windows PowerShell packaging script remains available for Windows SDK MakeAppx.
