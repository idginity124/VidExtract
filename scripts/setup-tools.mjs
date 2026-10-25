/** Prepare native resources on the OS being packaged. Node 22+, CMake and a C++ compiler required. */
import { createHash } from "node:crypto";
import { createReadStream, createWriteStream } from "node:fs";
import { chmod, copyFile, mkdir, mkdtemp, readdir, readFile, rename, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { pipeline } from "node:stream/promises";
import { spawnSync } from "node:child_process";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const destination = process.argv.includes("--target-windows") ? join(root,"packaging/windows/binaries") : join(root, "src-tauri", "binaries");
const windows = process.platform === "win32" || process.argv.includes("--target-windows");
const suffix = windows ? ".exe" : "";
const whisperRevision = "d1be6fde11ac6e0407606b4e42fe72d34add8037"; // whisper.cpp v1.9.5
if (!["win32", "linux"].includes(process.platform) || process.arch !== "x64") throw new Error("Bu paketleme betiği Windows/Linux x64 içindir.");
const scratch = await mkdtemp(join(process.platform==="win32"?(process.env.RUNNER_TEMP||tmpdir()):tmpdir(), "ve-"));
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
  if (process.platform !== "win32" && archive.endsWith(".zip")) run("unzip", ["-q", archive, "-d", target]);
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
  if (!process.argv.includes("--whisper-only") && !process.argv.includes("--vulkan-only")) {
    const ytName = windows ? "yt-dlp.exe" : "yt-dlp_linux";
    await install(await releaseAsset("yt-dlp/yt-dlp", ytName), `yt-dlp${suffix}`);
    const ffName = windows ? "ffmpeg-master-latest-win64-gpl.zip" : "ffmpeg-master-latest-linux64-gpl.tar.xz";
    const ffRoot = join(scratch, "ffmpeg");
    await unpack(await releaseAsset("BtbN/FFmpeg-Builds", ffName), ffRoot);
    for (const name of ["ffmpeg", "ffprobe"]) await install(await findFile(ffRoot, `${name}${suffix}`), `${name}${suffix}`);
    // Preserve upstream license text and build/source information with the redistributed binaries.
    for (const name of ["LICENSE.txt", "LICENSE", "COPYING.GPLv3", "README.txt"]) {
      const source = await findFile(ffRoot, name); if (source) await install(source, `FFmpeg-${name}`);
    }
    const denoName = windows ? "deno-x86_64-pc-windows-msvc.zip" : "deno-x86_64-unknown-linux-gnu.zip";
    const denoRoot = join(scratch, "deno");
    await unpack(await releaseAsset("denoland/deno", denoName), denoRoot);
    await install(await findFile(denoRoot, `deno${suffix}`), `deno${suffix}`);
    for (const [repo, ref, path, name] of [["yt-dlp/yt-dlp", "master", "LICENSE", "yt-dlp-LICENSE"], ["denoland/deno", "main", "LICENSE.md", "deno-LICENSE.md"]]) {
      await download(`https://raw.githubusercontent.com/${repo}/${ref}/${path}`, join(destination, name));
    }
  }
  if (!process.argv.includes("--download-only")) {
  const sourceRoot = join(scratch, "whisper");
  const sourceArchive = join(scratch, "whisper.tar.gz");
  const sourceHash = await download(`https://codeload.github.com/ggml-org/whisper.cpp/tar.gz/${whisperRevision}`, sourceArchive);
  manifest.whisper = { repository: "ggml-org/whisper.cpp", revision: whisperRevision, sourceSha256: sourceHash };
  await unpack(sourceArchive, sourceRoot);
  const entries = await readdir(sourceRoot); let source = join(sourceRoot, entries[0]);
  if (process.platform==="win32") {const short=join(scratch,"w");await rename(source,short);source=short;}
  // Probe the same compiled backend without loading a model; no separate Vulkan tool is required.
  const cli=join(source,"examples","cli","cli.cpp");
  let cliText=await readFile(cli,"utf8");
  cliText='#include "ggml-backend.h"\n'+cliText.replace('int main(int argc, char ** argv) {',`int main(int argc, char ** argv) {
    if (argc == 2 && std::string(argv[1]) == "--vidextract-probe") {
        ggml_backend_load_all();
        for (size_t i = 0; i < ggml_backend_dev_count(); ++i) {
            auto dev = ggml_backend_dev_get(i);
            if (ggml_backend_dev_type(dev) == GGML_BACKEND_DEVICE_TYPE_GPU) {
                printf("VIDEXTRACT_GPU:%s\\n", ggml_backend_dev_description(dev));
                return 0;
            }
        }
        return 2;
    }`);
  await writeFile(cli,cliText);
  // v1.9.5 locates SPIRV-Headers but misses its imported include directories.
  const vkCmake=join(source,"ggml","src","ggml-vulkan","CMakeLists.txt");
  await writeFile(vkCmake,(await readFile(vkCmake,"utf8")).replace('PRIVATE Vulkan::Vulkan)', 'PRIVATE Vulkan::Vulkan SPIRV-Headers::SPIRV-Headers)'));
  // Ship a portable CPU executable plus an AVX2 variant. Runtime CPUID selects safely.
  for (const variant of (process.argv.includes("--vulkan-only") ? ["vulkan"] : ["portable","avx2","vulkan"])) {
    const optimized=variant==="avx2";
    const build = join(source, `build-${variant}`);
    const flag=optimized?"ON":"OFF";
    run("cmake", ["-S",source,"-B",build,"-DCMAKE_BUILD_TYPE=Release","-DBUILD_SHARED_LIBS=OFF","-DGGML_NATIVE=OFF",`-DGGML_VULKAN=${variant==="vulkan"?"ON":"OFF"}`,"-DGGML_OPENMP=OFF","-DWHISPER_BUILD_TESTS=OFF","-DWHISPER_BUILD_SERVER=OFF",`-DGGML_SSE42=${flag}`,`-DGGML_AVX=${flag}`,`-DGGML_AVX2=${flag}`,`-DGGML_FMA=${flag}`,`-DGGML_F16C=${flag}`,`-DGGML_BMI2=${flag}`,"-DGGML_AVX512=OFF",...(windows?["-G","Ninja","-DCMAKE_C_COMPILER=cl","-DCMAKE_CXX_COMPILER=cl","-DCMAKE_MSVC_RUNTIME_LIBRARY=MultiThreaded"]:[])]);
    run("cmake", ["--build",build,"--config","Release","--target","whisper-cli","--parallel","4"]);
    await install(await findFile(join(build,"bin"),`whisper-cli${suffix}`),`whisper-cli${variant==="portable"?"":`-${variant}`}${suffix}`);
  }
  await install(join(source, "LICENSE"), "whisper-LICENSE");
  }
  let previous = {}; try { previous = JSON.parse(await readFile(join(destination, "tool-versions.json"), "utf8")); } catch {}
  await writeFile(join(destination, "tool-versions.json"), JSON.stringify({ ...previous, ...manifest }, null, 2) + "\n");
  await writeFile(join(destination, "THIRD-PARTY-NOTICES.txt"), "VidExtract native tools\n\nyt-dlp: Unlicense (see yt-dlp-LICENSE; standalone dependencies have their own licenses). https://github.com/yt-dlp/yt-dlp\nFFmpeg GPL build: GPLv3 or later. https://github.com/BtbN/FFmpeg-Builds\nBuild scripts and matching source references: see tool-versions.json and upstream build metadata. Redistributing FFmpeg requires fulfilling its GPL source obligations.\nwhisper.cpp: MIT. https://github.com/ggml-org/whisper.cpp\nDeno: MIT. https://github.com/denoland/deno\nWhisper tiny/base/small models (optional downloads): MIT. https://huggingface.co/ggerganov/whisper.cpp\n");
  console.log(`Araçlar hazır: ${destination}`);
} finally { await rm(scratch, { recursive: true, force: true }); }
