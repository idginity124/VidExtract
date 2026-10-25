// GStreamer helper locations differ between Debian/Ubuntu and Arch/CachyOS.
// Stage only executables: on Arch the scanner lives alongside the plugin .so files.
import { spawn, spawnSync } from "node:child_process";
import { access, copyFile, mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
if (process.platform !== "linux") throw new Error("Use this packaging command on Linux.");
const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
function pkgVariable(name) {
  const result=spawnSync("pkg-config", [`--variable=${name}`, "gstreamer-1.0"], {encoding:"utf8"});
  if (result.status !== 0 || !result.stdout.trim()) throw new Error("GStreamer development tools are required for Linux media packaging.");
  return result.stdout.trim();
}
const helpers=await mkdtemp(join(tmpdir(),"vidextract-gst-helpers-"));
try {
  const scannerDir=pkgVariable("pluginscannerdir");
  await copyFile(join(scannerDir,"gst-plugin-scanner"),join(helpers,"gst-plugin-scanner"));
  try {await copyFile(join(scannerDir,"gst-ptp-helper"),join(helpers,"gst-ptp-helper"));} catch {}
  const args=process.argv.includes("--bundle-only") ? ["bundle","--bundles","appimage"] : ["build","--bundles","deb,appimage"];
  const status=await new Promise((resolve,reject)=>{
    const child=spawn(process.execPath,[join(root,"node_modules/@tauri-apps/cli/tauri.js"),...args],{
      cwd:root,stdio:"inherit",env:{...process.env,GSTREAMER_HELPERS_DIR:helpers,GSTREAMER_PLUGINS_DIR:pkgVariable("pluginsdir")},
    });
    child.on("error",reject);child.on("exit",code=>resolve(code ?? 1));
  });
  if (status !== 0) process.exitCode=status;
  else {
    const appDir=join(root,"src-tauri/target/release/bundle/appimage/VidExtract.AppDir");
    for (const file of ["usr/lib/gstreamer-1.0/libgstapp.so","usr/lib/gstreamer-1.0/libgstautodetect.so","usr/lib/gstreamer-1.0/libgstplayback.so","usr/lib/gstreamer-1.0/libgstisomp4.so","usr/lib/gstreamer-1.0/libgstlibav.so","usr/lib/gstreamer1.0/gstreamer-1.0/gst-plugin-scanner"]) {
      await access(join(appDir,file));
    }
    console.log("AppImage media plugins and scanner verified.");
  }
} finally {await rm(helpers,{recursive:true,force:true});}
