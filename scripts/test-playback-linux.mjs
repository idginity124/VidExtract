// Exercise real WebKitGTK with the same libraries/plugins shipped in the AppImage.
import { spawn } from "node:child_process";
import { access, copyFile, mkdir, mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
if (process.platform !== "linux") throw new Error("This test targets the Linux AppImage runtime.");
const root=resolve(dirname(fileURLToPath(import.meta.url)),"..");
const appDir=join(root,"src-tauri/target/release/bundle/appimage/VidExtract.AppDir");
await access(join(appDir,"AppRun"));
const media=resolve(process.argv[2] || join(root,"tests/fixtures/speech.mp4"));
await access(media);
function run(program,args,env={}) {
  return new Promise((resolve,reject)=>{
    const child=spawn(program,args,{cwd:root,stdio:"inherit",env:{...process.env,...env}});
    child.on("error",reject);child.on("exit",code=>code===0?resolve():reject(new Error(`${program}: exit ${code}`)));
  });
}
await run("cargo",["build","--release","--manifest-path","src-tauri/Cargo.toml","--features","tauri/custom-protocol","--example","playback-smoke"]);
const scratch=await mkdtemp(join(tmpdir(),"vidextract-playback-"));
const report=join(scratch,"run/report.jsonl");
try {
  const clone=join(scratch,"AppDir");await mkdir(clone);
  await run("cp",["-a","--reflink=auto",`${appDir}/.`,clone]);
  await copyFile(join(root,"src-tauri/target/release/examples/playback-smoke"),join(clone,"usr/bin/vidextract"));
  await run(join(clone,"AppRun"),[],{
    APPDIR:clone,XDG_DATA_HOME:join(scratch,"run/data"),XDG_CACHE_HOME:join(scratch,"run/cache"),
    GST_REGISTRY_1_0:join(scratch,"run/gst-registry.bin"),VIDEXTRACT_PLAYBACK_MEDIA:media,VIDEXTRACT_PLAYBACK_REPORT:report,
    // Prevent a developer's diagnostic script override from changing the test.
    VIDEXTRACT_PLAYBACK_SCRIPT:join(root,"src-tauri/tests/playback-smoke.js"),
  });
  const events=(await readFile(report,"utf8")).trim().split("\n").map(line=>JSON.parse(line));
  if (!events.some(e=>e.phase==="passed")) throw new Error("Native playback test did not pass");
  console.log("PASS: packaged WebKit history -> studio -> video playback -> seek -> responsive navigation.");
} finally {
  await mkdir(join(root,"test-results"),{recursive:true});
  try {await copyFile(report,join(root,"test-results/native-playback.jsonl"));} catch {}
  await rm(scratch,{recursive:true,force:true});
}
