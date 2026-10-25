// Package locally with Microsoft MSIX SDK makemsix or Windows SDK MakeAppx.
import {mkdir,rm,cp,copyFile,readFile,writeFile} from 'node:fs/promises';
import {resolve,join,dirname} from 'node:path';
import {fileURLToPath} from 'node:url';
import {spawn} from 'node:child_process';
const root=resolve(dirname(fileURLToPath(import.meta.url)),'..');
const config=JSON.parse(await readFile(join(root,'package.json'),'utf8'));
const identity=JSON.parse(await readFile(join(root,'packaging/windows/identity.json'),'utf8'));
const output=join(root,'store-output'),stage=join(output,'staging');await rm(stage,{recursive:true,force:true});await mkdir(join(stage,'Assets'),{recursive:true});
const exe=resolve(process.env.VIDEXTRACT_WINDOWS_EXE??join(root,'src-tauri/target/x86_64-pc-windows-msvc/release/vidextract.exe'));
await copyFile(exe,join(stage,'VidExtract.exe'));await cp(join(root,'packaging/windows/binaries'),join(stage,'binaries'),{recursive:true});
for(const name of ['LICENSE','PRIVACY.md','THIRD_PARTY.md'])await copyFile(join(root,name),join(stage,name));
for(const name of ['StoreLogo.png','Square44x44Logo.png','Square150x150Logo.png'])await copyFile(join(root,'src-tauri/icons',name),join(stage,'Assets',name));
const xml=s=>s.replaceAll('&','&amp;').replaceAll('"','&quot;').replaceAll('<','&lt;').replaceAll('>','&gt;');
const languages=['en-us','tr-tr','de-de','es-es','fr-fr','pt-br','ru-ru','ar-sa','hi-in','zh-cn'];
await writeFile(join(stage,'AppxManifest.xml'),`<?xml version="1.0" encoding="utf-8"?>
<Package xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10" xmlns:uap="http://schemas.microsoft.com/appx/manifest/uap/windows10" xmlns:rescap="http://schemas.microsoft.com/appx/manifest/foundation/windows10/restrictedcapabilities" IgnorableNamespaces="uap rescap">
<Identity Name="${xml(identity.name)}" Publisher="${xml(identity.publisher)}" Version="${config.version}.0" ProcessorArchitecture="x64"/>
<Properties><DisplayName>VidExtract</DisplayName><PublisherDisplayName>${xml(identity.publisherDisplayName)}</PublisherDisplayName><Logo>Assets\\StoreLogo.png</Logo></Properties>
<Resources>${languages.map(l=>`<Resource Language="${l}"/>`).join('')}</Resources>
<Dependencies><TargetDeviceFamily Name="Windows.Desktop" MinVersion="10.0.19041.0" MaxVersionTested="10.0.26100.0"/></Dependencies>
<Applications><Application Id="VidExtract" Executable="VidExtract.exe" EntryPoint="Windows.FullTrustApplication"><uap:VisualElements DisplayName="VidExtract" Description="Download, transcribe, search and translate videos." BackgroundColor="transparent" Square44x44Logo="Assets\\Square44x44Logo.png" Square150x150Logo="Assets\\Square150x150Logo.png"/></Application></Applications>
<Capabilities><rescap:Capability Name="runFullTrust"/></Capabilities></Package>`);
const makeappx=process.env.MAKEAPPX_PATH,makemsix=process.env.MAKEMSIX_PATH;if(!makeappx&&!makemsix)throw new Error('Set MAKEAPPX_PATH (Windows SDK) or MAKEMSIX_PATH (Microsoft MSIX SDK).');
const windowsPath=p=>process.platform==='win32'?p:`Z:${p.replaceAll('/','\\')}`;
const msix=join(output,`VidExtract_${config.version}_x64-store.msix`);
const args=makemsix?['pack','-d',stage,'-p',msix]:['pack','/d',windowsPath(stage),'/p',windowsPath(msix),'/o'];
const result=await new Promise((res,rej)=>{const child=spawn(makemsix??(process.platform==='win32'?makeappx:'wine'),makemsix||process.platform==='win32'?args:[makeappx,...args],{stdio:'inherit',cwd:root,env:process.env});child.on('error',rej);child.on('exit',res);});
if(result!==0)throw new Error(`Microsoft package tool failed: ${result}`);
console.log(`Store submission package: ${msix}`);
