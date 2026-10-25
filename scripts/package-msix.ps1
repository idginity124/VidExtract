param(
  [string]$IdentityName = $env:STORE_IDENTITY_NAME,
  [string]$Publisher = $env:STORE_PUBLISHER,
  [string]$PublisherDisplayName = $env:STORE_PUBLISHER_DISPLAY_NAME
)
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$config = Get-Content (Join-Path $root 'package.json') -Raw | ConvertFrom-Json
$preview = $false
if ([string]::IsNullOrWhiteSpace($IdentityName)) {
  $identityPath = Join-Path $root 'packaging/windows/identity.json'
  if (-not (Test-Path $identityPath)) { $identityPath = Join-Path $root 'packaging/windows/identity.example.json'; $preview = $true }
  $identity = Get-Content $identityPath -Raw | ConvertFrom-Json
  $IdentityName = $identity.name
  $Publisher = $identity.publisher
  $PublisherDisplayName = $identity.publisherDisplayName
}
if ([string]::IsNullOrWhiteSpace($Publisher) -or [string]::IsNullOrWhiteSpace($PublisherDisplayName)) { throw 'All three Partner Center identity values are required.' }
if ($IdentityName -notmatch '^[A-Za-z0-9.-]{3,50}$') { throw 'Invalid package identity name.' }
$version = "$($config.version).0"
if ($version -notmatch '^\d+\.\d+\.\d+\.0$') { throw 'A three-part release version is required.' }
$stage = Join-Path $root 'store-output/staging'
if (Test-Path $stage) { Remove-Item $stage -Recurse -Force }
New-Item -ItemType Directory -Path (Join-Path $stage 'Assets') -Force | Out-Null
Copy-Item (Join-Path $root 'src-tauri/target/release/vidextract.exe') (Join-Path $stage 'VidExtract.exe')
Copy-Item (Join-Path $root 'src-tauri/binaries') (Join-Path $stage 'binaries') -Recurse
foreach ($file in @('LICENSE','THIRD_PARTY.md','PRIVACY.md')) { Copy-Item (Join-Path $root $file) $stage }
Copy-Item (Join-Path $root 'src-tauri/icons/StoreLogo.png') (Join-Path $stage 'Assets/StoreLogo.png')
Copy-Item (Join-Path $root 'src-tauri/icons/Square44x44Logo.png') (Join-Path $stage 'Assets/Square44x44Logo.png')
Copy-Item (Join-Path $root 'src-tauri/icons/Square150x150Logo.png') (Join-Path $stage 'Assets/Square150x150Logo.png')
$eName = [System.Security.SecurityElement]::Escape($IdentityName)
$ePublisher = [System.Security.SecurityElement]::Escape($Publisher)
$eDisplay = [System.Security.SecurityElement]::Escape($PublisherDisplayName)
$manifest = @"
<?xml version="1.0" encoding="utf-8"?>
<Package xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10"
 xmlns:uap="http://schemas.microsoft.com/appx/manifest/uap/windows10"
 xmlns:rescap="http://schemas.microsoft.com/appx/manifest/foundation/windows10/restrictedcapabilities"
 IgnorableNamespaces="uap rescap">
 <Identity Name="$eName" Publisher="$ePublisher" Version="$version" ProcessorArchitecture="x64" />
 <Properties><DisplayName>VidExtract</DisplayName><PublisherDisplayName>$eDisplay</PublisherDisplayName><Logo>Assets\StoreLogo.png</Logo></Properties>
 <Resources><Resource Language="en-us"/><Resource Language="tr-tr"/><Resource Language="de-de"/><Resource Language="es-es"/><Resource Language="fr-fr"/><Resource Language="pt-br"/><Resource Language="ru-ru"/><Resource Language="ar-sa"/><Resource Language="hi-in"/><Resource Language="zh-cn"/></Resources>
 <Dependencies><TargetDeviceFamily Name="Windows.Desktop" MinVersion="10.0.19041.0" MaxVersionTested="10.0.26100.0"/></Dependencies>
 <Applications><Application Id="VidExtract" Executable="VidExtract.exe" EntryPoint="Windows.FullTrustApplication">
 <uap:VisualElements DisplayName="VidExtract" Description="Download, transcribe, search and translate videos."
 BackgroundColor="transparent" Square44x44Logo="Assets\Square44x44Logo.png" Square150x150Logo="Assets\Square150x150Logo.png"/>
 </Application></Applications>
 <Capabilities><rescap:Capability Name="runFullTrust"/></Capabilities>
</Package>
"@
[IO.File]::WriteAllText((Join-Path $stage 'AppxManifest.xml'), $manifest, [Text.UTF8Encoding]::new($false))
$makeappx = Get-ChildItem "${env:ProgramFiles(x86)}/Windows Kits/10/bin/*/x64/makeappx.exe" | Sort-Object FullName -Descending | Select-Object -First 1
if (-not $makeappx) { throw 'Windows SDK MakeAppx.exe was not found.' }
$suffix = if ($preview) { '-preview-identity' } else { '-store' }
$output = Join-Path $root "store-output/VidExtract_$($config.version)_x64$suffix.msix"
& $makeappx.FullName pack /d $stage /p $output /o
if ($LASTEXITCODE -ne 0) { throw 'MSIX validation or packaging failed.' }
Write-Host "Unsigned MSIX prepared: $output"
if ($preview) { Write-Host 'Preview identity only. Replace with Partner Center values before Store submission.' }
