import { createServer } from "node:http";
import { createReadStream } from "node:fs";
import { access, mkdir, readFile, writeFile } from "node:fs/promises";
import { createHash } from "node:crypto";
import { resolve, join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { tmpdir } from "node:os";
import { spawn } from "node:child_process";
const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const dir = process.env.VIDEXTRACT_TEST_DIR || join(tmpdir(), "vidextract-smoke");
await mkdir(dir, { recursive: true });
const exe = process.platform === "win32" ? ".exe" : "";
const native = (name) => join(root, "src-tauri", "binaries", `${name}${exe}`);
async function tool(name) { try { await access(native(name)); return native(name); } catch { return `${name}${exe}`; } }
async function fetchFile(url, name, hash) {
  const path = join(dir, name);
  let bytes; try { bytes = await readFile(path); } catch {}
  if (!bytes || createHash("sha256").update(bytes).digest("hex") !== hash) {
    const response = await fetch(url); if (!response.ok) throw new Error(`Fixture download: ${response.status}`);
    bytes = Buffer.from(await response.arrayBuffer());
    if (createHash("sha256").update(bytes).digest("hex") !== hash) throw new Error(`Checksum mismatch: ${name}`);
    await writeFile(path, bytes);
  }
  return path;
}
function run(program, args, env = {}) {
  return new Promise((resolve, reject) => {
    const child = spawn(program, args, { cwd: root, stdio: "inherit", env: { ...process.env, ...env }, shell: false });
    child.on("error", reject); child.on("exit", (code) => code === 0 ? resolve() : reject(new Error(`${program}: ${code}`)));
  });
}
await fetchFile("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin", "ggml-base.bin", "60ed5bc3dd14eea856493d334349b405782ddcaf0028d4b5df4088345fba2efe");
const wav = await fetchFile("https://raw.githubusercontent.com/ggml-org/whisper.cpp/d1be6fde11ac6e0407606b4e42fe72d34add8037/samples/jfk.wav", "jfk.wav", "59dfb9a4acb36fe2a2affc14bacbee2920ff435cb13cc314a08c13f66ba7860e");
const ffmpeg = await tool("ffmpeg");
await run(ffmpeg, ["-nostdin", "-v", "error", "-y", "-f", "lavfi", "-i", "color=c=0x171b27:s=640x360:r=24", "-i", wav, "-shortest", "-c:v", "libx264", "-c:a", "aac", "-movflags", "+faststart", join(dir, "speech.mp4")]);
const server = createServer((request, response) => {
  if (request.url !== "/speech.mp4") { response.writeHead(404).end(); return; }
  response.setHeader("Content-Type", "video/mp4");
  createReadStream(join(dir, "speech.mp4")).pipe(response);
});
await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
try {
  await run("cargo", ["test", "--manifest-path", "src-tauri/Cargo.toml", "--", "--ignored", "--nocapture"], {
    VIDEXTRACT_TEST_DIR: dir, VIDEXTRACT_TEST_URL: `http://127.0.0.1:${server.address().port}/speech.mp4`, VIDEXTRACT_YTDLP: await tool("yt-dlp"), VIDEXTRACT_FFMPEG: ffmpeg,
  });
} finally { server.closeAllConnections(); server.close(); }
