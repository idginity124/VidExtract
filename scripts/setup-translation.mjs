/** Prepare native resources on the OS being packaged. Node 22+, CMake and a C++ compiler required. */
import { createHash } from "node:crypto";
import { createReadStream, createWriteStream } from "node:fs";
import { chmod, copyFile, mkdir, mkdtemp, readdir, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { pipeline } from "node:stream/promises";
import { spawnSync } from "node:child_process";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const destination = join(root, "src-tauri", "binaries");
const windows = process.platform === "win32";
const suffix = windows ? ".exe" : "";
const whisperRevision = "d1be6fde11ac6e0407606b4e42fe72d34add8037"; // whisper.cpp v1.9.5
if (!["win32", "linux"].includes(process.platform) || process.arch !== "x64") throw new Error("Bu paketleme betiği Windows/Linux x64 içindir.");
const scratch = await mkdtemp(join(windows?(process.env.RUNNER_TEMP||tmpdir()):tmpdir(), "ve-t-"));
await mkdir(destination, { recursive: true });
const headers = { "User-Agent": "VidExtract-build" };
if (process.env.GITHUB_TOKEN) headers.Authorization = `Bearer ${process.env.GITHUB_TOKEN}`;
const manifest = {};
function run(program, args, cwd) {
  const result = spawnSync(program, args, { cwd, stdio: "inherit", shell: false });
  if (result.error || result.status !== 0) throw new Error(`${program} başarısız: ${result.error?.message ?? result.status}`);
}
async function sha256(file) {
  const hash = createHash("sha256");
  for await (const chunk of createReadStream(file)) hash.update(chunk);
  return hash.digest("hex");
}
async function download(url, path, expectedHash) {
  console.log(`İndiriliyor: ${url}`);
  const response = await fetch(url, { signal: AbortSignal.timeout(900_000) });
  if (!response.ok || !response.body) throw new Error(`İndirme başarısız: HTTP ${response.status}`);
  await pipeline(response.body, createWriteStream(path));
  const actualHash = await sha256(path);
  if (expectedHash && actualHash !== expectedHash.replace(/^sha256:/, "")) throw new Error(`SHA-256 uyuşmuyor: ${url}`);
  return actualHash;
}
async function release(repo) {
  const response = await fetch(`https://api.github.com/repos/${repo}/releases/latest`, { headers });
  if (!response.ok) throw new Error(`GitHub sürümü okunamadı: ${repo}, ${response.status}`);
  return response.json();
}
async function releaseAsset(repo, name) {
  const info = await release(repo);
  const asset = info.assets.find((a) => a.name === name);
  if (!asset?.digest?.startsWith("sha256:")) throw new Error(`Doğrulanabilir sürüm dosyası bulunamadı: ${repo}/${name}`);
  const archive = join(scratch, name);
  const hash = await download(asset.browser_download_url, archive, asset.digest);
  manifest[name] = { repository: repo, version: info.tag_name, sha256: hash, url: asset.browser_download_url };
  return archive;
}
async function unpack(archive, target) {
  await mkdir(target, { recursive: true });
  // tar on supported Windows runners also handles ZIP archives.
  if (!windows && archive.endsWith(".zip")) run("unzip", ["-q", archive, "-d", target]);
  else run("tar", ["-xf", archive, "-C", target]);
}
async function findFile(directory, name) {
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const path = join(directory, entry.name);
    if (entry.isFile() && entry.name === name) return path;
    if (entry.isDirectory()) { const found = await findFile(path, name); if (found) return found; }
  }
}
async function install(source, name) {
  if (!source) throw new Error(`Araç bulunamadı: ${name}`);
  const target = join(destination, name);
  await copyFile(source, target);
  if (!windows) await chmod(target, 0o755);
}
try {
  const ct=join(scratch,"c"),sp=join(scratch,"s");
  run("git",["clone","--depth","1","--branch","v4.8.3","https://github.com/OpenNMT/CTranslate2.git",ct]);
  run("git",["submodule","update","--init","--depth","1","third_party/cpu_features","third_party/spdlog"],ct);
  run("git",["submodule","update","--init","--recursive","--depth","1","third_party/ruy"],ct);
  run("git",["clone","--depth","1","--branch","v0.2.1","https://github.com/google/sentencepiece.git",sp]);
  const include=join(scratch,"include");await mkdir(join(include,"nlohmann"),{recursive:true});
  await download("https://raw.githubusercontent.com/nlohmann/json/v3.12.0/single_include/nlohmann/json.hpp",join(include,"nlohmann/json.hpp"));
  const build=join(scratch,"b");
  run("cmake",["-S",join(root,"src-tauri/native"),"-B",build,"-DCMAKE_BUILD_TYPE=Release","-DCMAKE_POLICY_VERSION_MINIMUM=3.5",`-DCT2_ROOT=${ct}`,`-DSPM_ROOT=${sp}`,`-DJSON_ROOT=${include}`,...(windows?["-G","Ninja","-DCMAKE_C_COMPILER=cl","-DCMAKE_CXX_COMPILER=cl","-DCMAKE_MSVC_RUNTIME_LIBRARY=MultiThreaded"]:[])]);
  run("cmake",["--build",build,"--config","Release","--target","translate-cpu","--parallel","4"]);
  await install(await findFile(build,`translate-cpu${suffix}`),`translate-cpu${suffix}`);
  for(const [source,name] of [[join(ct,"LICENSE"),"CTranslate2-LICENSE"],[join(sp,"LICENSE"),"SentencePiece-LICENSE"],[join(ct,"third_party/ruy/LICENSE"),"Ruy-LICENSE"],[join(ct,"third_party/cpu_features/LICENSE"),"cpu-features-LICENSE"],[join(ct,"third_party/spdlog/LICENSE"),"spdlog-LICENSE"],[join(ct,"third_party/ruy/third_party/cpuinfo/LICENSE"),"cpuinfo-LICENSE"]])await install(source,name);
  await download("https://raw.githubusercontent.com/nlohmann/json/v3.12.0/LICENSE.MIT",join(destination,"nlohmann-json-LICENSE"));
  await writeFile(join(destination,"translation-versions.json"),JSON.stringify({engine:"CTranslate2",version:"4.8.3",sentencepiece:"0.2.1",compute:"CPU int8; runtime ISA dispatch; four-caption batches",packsCatalogSha256:await sha256(join(root,"src-tauri/translation-packs.json"))},null,2));
} finally {await rm(scratch,{recursive:true,force:true});}
