(async () => {
  const send = (phase, extra = {}) => window.__TAURI_INTERNALS__.invoke("plugin:event|emit", {event:"vidextract-playback-smoke",payload:{phase,...extra}});
  const sleep = (ms) => new Promise(resolve => setTimeout(resolve, ms));
  async function until(fn, name, timeout = 30000) {
    const end = Date.now() + timeout;
    while (Date.now() < end) { const value = fn(); if (value) return value; await sleep(100); }
    throw new Error(`Timeout: ${name}`);
  }
  try {
    if (!sessionStorage.getItem("playback-smoke-seeded")) {
      localStorage.setItem("ve-locale","tr");
      localStorage.setItem("ve-download-history", JSON.stringify([{id:"playback-smoke", title:"Playback regression",thumbnail:null,mode:"video",format:"MP4",progress:100,speed:"—",eta:"—",status:"completed",message:"Tamamlandı",outputPath:__MEDIA_PATH_JSON__}]));
      localStorage.setItem("ve-clipboard-monitor", "false");
      sessionStorage.setItem("playback-smoke-seeded", "1"); location.reload(); return;
    }
    await send("frontend-ready");
    const heartbeat = setInterval(() => send("heartbeat", {ready:document.querySelector("video")?.readyState ?? null}), 2000);
    const history = await until(() => [...document.querySelectorAll(".nav-item")].find(el => el.textContent.trim() === "Geçmiş"), "history navigation");
    history.click();
    (await until(() => document.querySelector('button[aria-label="Stüdyoda aç"]'), "history magnifier")).click();
    await send("history-magnifier-clicked");
    const video = await until(() => document.querySelector("video"), "video element");
    await send("video-created", {src:video.currentSrc || video.src, mp4:video.canPlayType("video/mp4"), h264:video.canPlayType('video/mp4; codecs="avc1.640028, mp4a.40.2"')});
    video.muted = true;
    await until(() => video.readyState >= 2 || video.error, "video frames", 45000);
    if (video.error) throw new Error(`Video error ${video.error.code}: ${video.error.message}`);
    await send("video-ready", {duration:video.duration, ready:video.readyState});
    await video.play();
    const start = video.currentTime;
    await until(() => video.currentTime > start + .6, "playback advances");
    video.pause();
    video.currentTime = Math.min(video.duration / 2, 60);
    await until(() => !video.seeking && video.readyState >= 2, "seek finishes");
    await send("video-seeked", {time:video.currentTime});
    [...document.querySelectorAll(".nav-item")].find(el => el.textContent.trim() === "Ana sayfa").click();
    await until(() => document.querySelector(".page-header h1")?.textContent === "Yeni indirme", "UI remains responsive");
    clearInterval(heartbeat);
    await send("passed");
  } catch (error) {await send("failed", {error:String(error)});}
})();
