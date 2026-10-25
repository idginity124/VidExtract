use crate::{process, studio};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Mutex,
};
use tauri::{ipc::Channel, AppHandle, Manager, State};
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;

pub struct AppState {
    pub(crate) jobs: Mutex<HashMap<String, CancellationToken>>,
    pub(crate) downloads: Semaphore,
    pub(crate) studio: Semaphore,
    pub(crate) media: crate::media::MediaServer,
}
impl Default for AppState {
    fn default() -> Self {
        let mut memory = sysinfo::System::new();
        memory.refresh_memory();
        let concurrency = if memory.total_memory() < 6 * 1024 * 1024 * 1024
            || memory.available_memory() < 2 * 1024 * 1024 * 1024
            || std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1)
                <= 4
        {
            1
        } else {
            2
        };
        Self {
            jobs: Mutex::new(HashMap::new()),
            downloads: Semaphore::new(concurrency),
            studio: Semaphore::new(1),
            media: crate::media::MediaServer::default(),
        }
    }
}
impl AppState {
    pub(crate) fn register(&self, id: &str) -> Result<CancellationToken, String> {
        if id.is_empty() || id.len() > 128 {
            return Err("Geçersiz işlem kimliği".into());
        }
        let mut jobs = self.jobs.lock().map_err(|e| e.to_string())?;
        if jobs.contains_key(id) {
            return Err("Bu işlem zaten çalışıyor".into());
        }
        let token = CancellationToken::new();
        jobs.insert(id.into(), token.clone());
        Ok(token)
    }
    pub(crate) fn finish(&self, id: &str) {
        if let Ok(mut jobs) = self.jobs.lock() {
            jobs.remove(id);
        }
    }
}

pub fn tool(app: &AppHandle, name: &str) -> PathBuf {
    let filename = format!("{name}{}", std::env::consts::EXE_SUFFIX);
    if let Ok(resources) = app.path().resource_dir() {
        let bundled = resources.join("binaries").join(&filename);
        if bundled.is_file() {
            return bundled;
        }
    }
    let development = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("binaries")
        .join(&filename);
    if cfg!(debug_assertions) && development.is_file() {
        return development;
    }
    PathBuf::from(filename)
}

pub fn optimized_speech_supported() -> bool {
    #[cfg(target_arch = "x86_64")]
    {
        std::is_x86_feature_detected!("avx2")
            && std::is_x86_feature_detected!("fma")
            && std::is_x86_feature_detected!("f16c")
            && std::is_x86_feature_detected!("sse4.2")
            && std::is_x86_feature_detected!("bmi2")
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        false
    }
}
pub fn speech_tool(app: &AppHandle) -> PathBuf {
    if optimized_speech_supported() {
        let optimized = tool(app, "whisper-cli-avx2");
        if optimized.is_file() {
            return optimized;
        }
    }
    tool(app, "whisper-cli")
}

pub fn data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let path = app.path().app_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    Ok(path)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyStatus {
    yt_dlp: bool,
    ffmpeg: bool,
    ffprobe: bool,
    yt_dlp_version: Option<String>,
    default_download_dir: String,
    whisper: bool,
    model_ready: bool,
    device: crate::hardware::DeviceProfile,
    models: Vec<studio::ModelInfo>,
}
async fn version(app: &AppHandle, name: &str, arg: &str) -> Option<String> {
    let executable = if name == "whisper-cli" {
        speech_tool(app)
    } else {
        tool(app, name)
    };
    let mut cmd = process::command(&executable);
    cmd.arg(arg);
    process::checked(cmd, 15)
        .await
        .ok()
        .map(|s| s.lines().next().unwrap_or("").to_string())
}
#[tauri::command]
pub async fn check_dependencies(app: AppHandle) -> Result<DependencyStatus, String> {
    let (yt, ffmpeg, ffprobe, whisper) = tokio::join!(
        version(&app, "yt-dlp", "--version"),
        version(&app, "ffmpeg", "-version"),
        version(&app, "ffprobe", "-version"),
        version(&app, "whisper-cli", "--help")
    );
    let download_dir = app
        .path()
        .download_dir()
        .or_else(|_| app.path().home_dir())
        .map_err(|e| e.to_string())?;
    let mut device = crate::hardware::inspect_device().await?;
    device.gpu_acceleration = crate::hardware::vulkan_available(&app).await;
    Ok(DependencyStatus {
        yt_dlp: yt.is_some(),
        ffmpeg: ffmpeg.is_some(),
        ffprobe: ffprobe.is_some(),
        yt_dlp_version: yt,
        default_download_dir: download_dir.to_string_lossy().into(),
        whisper: whisper.is_some(),
        model_ready: studio::model_path(&app)?.is_file(),
        device,
        models: studio::model_catalog(&app)?,
    })
}

fn validate_url(input: &str) -> Result<(), String> {
    let parsed = url::Url::parse(input).map_err(|_| "Geçerli bir HTTP/HTTPS bağlantısı gir")?;
    if !matches!(parsed.scheme(), "http" | "https")
        || parsed.host_str().is_none()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
    {
        return Err("Geçerli bir HTTP/HTTPS bağlantısı gir".into());
    }
    Ok(())
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoInfo {
    id: String,
    title: String,
    uploader: String,
    duration: Option<f64>,
    thumbnail: Option<String>,
    webpage_url: String,
    extractor: String,
    view_count: Option<u64>,
    is_live: bool,
}
fn text(value: &Value, key: &str) -> String {
    value[key].as_str().unwrap_or_default().into()
}
#[tauri::command]
pub async fn analyze_url(app: AppHandle, url: String) -> Result<VideoInfo, String> {
    validate_url(&url)?;
    let mut cmd = process::command(&tool(&app, "yt-dlp"));
    let deno = tool(&app, "deno");
    cmd.args(["--js-runtimes", &format!("deno:{}", deno.to_string_lossy())]);
    cmd.args([
        "--ignore-config",
        "--no-playlist",
        "--dump-single-json",
        "--skip-download",
        "--no-warnings",
        "--socket-timeout",
        "20",
        "--",
        &url,
    ]);
    let result = process::checked(cmd, 90).await?;
    let v: Value = serde_json::from_str(&result).map_err(|_| "Video bilgisi okunamadı")?;
    Ok(VideoInfo {
        id: text(&v, "id"),
        title: text(&v, "title"),
        uploader: text(&v, "uploader"),
        duration: v["duration"].as_f64(),
        thumbnail: v["thumbnail"].as_str().map(Into::into),
        webpage_url: v["webpage_url"].as_str().unwrap_or(&url).into(),
        extractor: text(&v, "extractor"),
        view_count: v["view_count"].as_u64(),
        is_live: v["is_live"].as_bool().unwrap_or(false),
    })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadRequest {
    job_id: String,
    url: String,
    output_dir: String,
    mode: String,
    quality: String,
    format: String,
    playlist: bool,
    subtitles: bool,
    subtitle_language: Option<String>,
    save_thumbnail: bool,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadEvent {
    kind: String,
    job_id: String,
    progress: Option<f64>,
    speed: Option<String>,
    eta: Option<String>,
    message: String,
    output_path: Option<String>,
}
impl DownloadEvent {
    fn new(kind: &str, id: &str, message: &str) -> Self {
        Self {
            kind: kind.into(),
            job_id: id.into(),
            progress: None,
            speed: None,
            eta: None,
            message: message.into(),
            output_path: None,
        }
    }
}
fn download_args(r: &DownloadRequest) -> Result<Vec<String>, String> {
    validate_url(&r.url)?;
    let audio = ["mp3", "m4a", "flac", "wav", "opus"];
    let video = ["mp4", "mkv", "webm"];
    if (r.mode != "audio" && r.mode != "video")
        || !(if r.mode == "audio" {
            audio.contains(&r.format.as_str())
        } else {
            video.contains(&r.format.as_str())
        })
    {
        return Err("Geçersiz çıktı formatı".into());
    }
    let mut args = vec![
        "--ignore-config",
        "--newline",
        "--no-colors",
        "--no-simulate",
        "--progress",
        "--progress-delta",
        "0.5",
        "--socket-timeout",
        "30",
        "--retries",
        "3",
        "--windows-filenames",
        "--no-overwrites",
        "--progress-template",
        "download:VE_PROGRESS:%(progress)j",
        "--print",
        "after_move:VE_FILE:%(filepath)j",
        "-P",
        &r.output_dir,
        "-o",
        "%(title).160B [%(id)s].%(ext)s",
    ]
    .into_iter()
    .map(String::from)
    .collect::<Vec<_>>();
    args.push(
        if r.playlist {
            "--yes-playlist"
        } else {
            "--no-playlist"
        }
        .into(),
    );
    if r.mode == "audio" {
        args.extend([
            "-x".into(),
            "--audio-format".into(),
            r.format.clone(),
            "--audio-quality".into(),
            "0".into(),
        ]);
    } else {
        let height = if r.quality == "Best" {
            String::new()
        } else {
            let height = r
                .quality
                .strip_suffix('p')
                .ok_or("Geçersiz kalite")?
                .parse::<u32>()
                .map_err(|_| "Geçersiz kalite")?;
            if !(144..=4320).contains(&height) {
                return Err("Geçersiz kalite".into());
            }
            format!("[height<=?{height}]")
        };
        args.extend([
            "-f".into(),
            format!("bv*{height}+ba/b{height}"),
            "--merge-output-format".into(),
            "mkv".into(),
            if r.format == "mkv" {
                "--remux-video".into()
            } else {
                "--recode-video".into()
            },
            r.format.clone(),
        ]);
    }
    if r.subtitles {
        let language = r.subtitle_language.as_deref().unwrap_or("tr");
        if !studio::SPEECH_LANGUAGES.contains(&language) {
            return Err("Desteklenmeyen altyazı dili".into());
        }
        let subtitles = if language == "en" {
            "en.*".to_string()
        } else {
            format!("{language}.*,en.*")
        };
        args.extend([
            "--write-subs".into(),
            "--write-auto-subs".into(),
            "--sub-langs".into(),
            subtitles,
            "--sub-format".into(),
            "srt/best".into(),
        ]);
    }
    if r.save_thumbnail {
        args.push("--write-thumbnail".into());
    }
    Ok(args)
}
fn parse_progress(line: &str, id: &str) -> Option<DownloadEvent> {
    let v: Value = serde_json::from_str(line.strip_prefix("VE_PROGRESS:")?).ok()?;
    let downloaded = v["downloaded_bytes"].as_f64().unwrap_or(0.0);
    let total = v["total_bytes"]
        .as_f64()
        .or_else(|| v["total_bytes_estimate"].as_f64());
    let mut event = DownloadEvent::new(
        if v["status"] == "finished" {
            "processing"
        } else {
            "progress"
        },
        id,
        if v["status"] == "finished" {
            "Dosya işleniyor"
        } else {
            "İndiriliyor"
        },
    );
    event.progress = total
        .filter(|n| *n > 0.0)
        .map(|n| (100.0 * downloaded / n).clamp(0.0, 100.0));
    event.speed = v["speed"]
        .as_f64()
        .map(|s| format!("{:.1} MB/s", s / 1_000_000.0));
    event.eta = v["eta"].as_f64().map(|s| format!("{} sn", s.ceil() as u64));
    Some(event)
}
#[tauri::command]
pub async fn start_download(
    app: AppHandle,
    state: State<'_, AppState>,
    request: DownloadRequest,
    on_event: Channel<DownloadEvent>,
) -> Result<(), String> {
    let mut args = download_args(&request)?;
    let folder = PathBuf::from(&request.output_dir);
    if !folder.is_absolute() {
        return Err("Mutlak bir indirme klasörü seç".into());
    }
    tokio::fs::create_dir_all(&folder)
        .await
        .map_err(|e| format!("Klasör oluşturulamadı: {e}"))?;
    let ffmpeg = tool(&app, "ffmpeg");
    if ffmpeg.is_absolute() {
        args.extend(["--ffmpeg-location".into(), ffmpeg.to_string_lossy().into()]);
    }
    let deno = tool(&app, "deno");
    args.extend([
        "--js-runtimes".into(),
        format!("deno:{}", deno.to_string_lossy()),
    ]);
    let threads = crate::hardware::memory_profile().await?.threads;
    args.extend([
        "--postprocessor-args".into(),
        format!("ffmpeg:-threads {threads}"),
    ]);
    args.extend(["--".into(), request.url.clone()]);
    let token = state.register(&request.job_id)?;
    let result: Result<(), String> = async {
        let _permit = tokio::select! {
            permit=state.downloads.acquire()=>permit.map_err(|e|e.to_string())?,
            _=token.cancelled()=>return Err("İptal edildi".into()),
        };
        let _ = on_event.send(DownloadEvent::new(
            "started",
            &request.job_id,
            "İndirme başladı",
        ));
        let mut cmd = process::command(&tool(&app, "yt-dlp"));
        cmd.args(args);
        let mut output_path = None;
        process::run(cmd, token.clone(), |line| {
            if let Some(event) = parse_progress(line, &request.job_id) {
                let _ = on_event.send(event);
            }
            if let Some(value) = line.strip_prefix("VE_FILE:") {
                if let Ok(path) = serde_json::from_str::<String>(value) {
                    output_path = Some(path);
                }
            }
        })
        .await?;
        if output_path.is_none() {
            return Err("İndirilebilir bir dosya oluşmadı".into());
        }
        let mut event = DownloadEvent::new("completed", &request.job_id, "Tamamlandı");
        event.output_path = output_path;
        event.progress = Some(100.0);
        let _ = on_event.send(event);
        Ok(())
    }
    .await;
    state.finish(&request.job_id);
    if let Err(ref message) = result {
        let _ = on_event.send(DownloadEvent::new(
            if token.is_cancelled() {
                "cancelled"
            } else {
                "error"
            },
            &request.job_id,
            message,
        ));
    }
    result
}
#[tauri::command]
pub fn cancel_download(state: State<'_, AppState>, job_id: String) -> Result<(), String> {
    if let Some(token) = state.jobs.lock().map_err(|e| e.to_string())?.get(&job_id) {
        token.cancel();
    }
    Ok(())
}
#[tauri::command]
async fn reveal_in_file_manager(path: String) -> Result<(), String> {
    let canonical = dunce::canonicalize(path).map_err(|_| "Klasör bulunamadı")?;
    let folder = if canonical.is_dir() {
        canonical
    } else {
        canonical.parent().ok_or("Klasör bulunamadı")?.to_path_buf()
    };
    #[cfg(windows)]
    let mut cmd = process::command(Path::new("explorer.exe"));
    #[cfg(not(windows))]
    let mut cmd = process::command(Path::new("xdg-open"));
    cmd.arg(folder);
    process::checked(cmd, 15).await?;
    Ok(())
}
pub fn desktop_builder() -> tauri::Builder<tauri::Wry> {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            check_dependencies,
            crate::hardware::inspect_device,
            analyze_url,
            start_download,
            cancel_download,
            reveal_in_file_manager,
            studio::prepare_media,
            studio::install_model,
            studio::transcribe_media,
            studio::list_transcripts,
            studio::export_transcript,
            crate::translation::translation_status,
            crate::translation::save_translation_settings,
            crate::translation::open_translation_terms,
            crate::translation::install_translation_model,
            crate::translation::translate_transcript,
            crate::translation::cached_translation,
            crate::translation::export_translation,
            studio::extract_clip
        ])
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    desktop_builder()
        .build(tauri::generate_context!())
        .expect("VidExtract başlatılamadı")
        .run(|app, event| {
            if matches!(event, tauri::RunEvent::Exit) {
                if let Ok(jobs) = app.state::<AppState>().jobs.lock() {
                    for token in jobs.values() {
                        token.cancel();
                    }
                }
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn urls_cannot_be_tool_options_or_local_files() {
        for input in [
            "--exec=evil",
            "file:///etc/passwd",
            "https://user:pass@example.org",
            "javascript:alert(1)",
        ] {
            assert!(validate_url(input).is_err());
        }
        assert!(validate_url("https://example.org/watch?v=abc&list=def").is_ok());
    }
    fn request() -> DownloadRequest {
        DownloadRequest {
            job_id: "test".into(),
            url: "https://example.org/video".into(),
            output_dir: "/tmp/video folder".into(),
            mode: "video".into(),
            quality: "1080p".into(),
            format: "mp4".into(),
            playlist: false,
            subtitles: false,
            subtitle_language: None,
            save_thumbnail: false,
        }
    }
    #[test]
    fn download_options_preserve_paths_and_reject_invalid_formats() {
        let mut r = request();
        let args = download_args(&r).unwrap();
        assert!(args.contains(&"/tmp/video folder".to_string()));
        assert!(args.contains(&"bv*[height<=?1080]+ba/b[height<=?1080]".to_string()));
        r.subtitles = true;
        r.subtitle_language = Some("zh".into());
        assert!(download_args(&r).unwrap().contains(&"zh.*,en.*".into()));
        r.format = "--exec=evil".into();
        assert!(download_args(&r).is_err());
        r.format = "mp4".into();
        r.quality = "0p".into();
        assert!(download_args(&r).is_err());
    }
    #[test]
    fn real_progress_handles_missing_totals() {
        let e=parse_progress(r#"VE_PROGRESS:{"downloaded_bytes":50,"total_bytes":100,"speed":2000000,"eta":3,"status":"downloading"}"#,"a").unwrap();
        assert_eq!(e.progress, Some(50.0));
        assert_eq!(e.speed.as_deref(), Some("2.0 MB/s"));
        assert_eq!(
            parse_progress(r#"VE_PROGRESS:{"downloaded_bytes":50}"#, "a")
                .unwrap()
                .progress,
            None
        );
    }
    #[tokio::test]
    #[ignore = "Requires native tools, a model and the local HTTP fixture; run npm run test:media"]
    async fn actual_download_transcription_and_subtitle_pipeline() {
        let root =
            PathBuf::from(std::env::var("VIDEXTRACT_TEST_DIR").expect("test fixture directory"));
        let url = std::env::var("VIDEXTRACT_TEST_URL").expect("local test media URL");
        let mut req = request();
        req.url = url.clone();
        req.output_dir = root.join("downloaded").to_string_lossy().into();
        std::fs::create_dir_all(&req.output_dir).unwrap();
        let yt = std::env::var("VIDEXTRACT_YTDLP").unwrap_or_else(|_| "yt-dlp".into());
        let ffmpeg = std::env::var("VIDEXTRACT_FFMPEG").unwrap_or_else(|_| "ffmpeg".into());
        let mut analysis = process::command(Path::new(&yt));
        analysis.args([
            "--ignore-config",
            "--no-playlist",
            "--dump-single-json",
            "--skip-download",
            "--",
            &url,
        ]);
        let metadata: Value =
            serde_json::from_str(&process::checked(analysis, 60).await.unwrap()).unwrap();
        assert!(metadata["title"].as_str().is_some());
        let mut args = download_args(&req).unwrap();
        if Path::new(&ffmpeg).is_absolute() {
            args.extend(["--ffmpeg-location".into(), ffmpeg.clone()]);
        }
        args.extend(["--".into(), url]);
        let mut cmd = process::command(Path::new(&yt));
        cmd.args(args);
        let mut paths = Vec::new();
        process::run(cmd, CancellationToken::new(), |line| {
            if let Some(raw) = line.strip_prefix("VE_FILE:") {
                paths.push(serde_json::from_str::<String>(raw).unwrap());
            }
        })
        .await
        .unwrap();
        assert_eq!(paths.len(), 1);
        assert!(Path::new(&paths[0]).is_file());
        // Every format exposed by the UI must produce an actual file, including transcoding H.264 to WebM.
        for format in ["mkv", "webm", "mp3", "m4a", "flac", "wav", "opus"] {
            req.format = format.into();
            req.mode = if ["mkv", "webm"].contains(&format) {
                "video".into()
            } else {
                "audio".into()
            };
            req.output_dir = root
                .join(format!("format-{format}"))
                .to_string_lossy()
                .into();
            std::fs::create_dir_all(&req.output_dir).unwrap();
            let mut args = download_args(&req).unwrap();
            if Path::new(&ffmpeg).is_absolute() {
                args.extend(["--ffmpeg-location".into(), ffmpeg.clone()]);
            }
            args.extend(["--".into(), req.url.clone()]);
            let mut cmd = process::command(Path::new(&yt));
            cmd.args(args);
            let result = process::checked(cmd, 120)
                .await
                .unwrap_or_else(|e| panic!("{format} failed: {e}"));
            let file = result
                .lines()
                .find_map(|line| line.strip_prefix("VE_FILE:"))
                .map(|v| serde_json::from_str::<String>(v).unwrap())
                .expect("No converted file");
            assert_eq!(
                Path::new(&file).extension().unwrap().to_str().unwrap(),
                format
            );
            assert!(std::fs::metadata(file).unwrap().len() > 1000);
        }
        let wav = root.join("converted.wav");
        let mut cmd = process::command(Path::new(&ffmpeg));
        cmd.args([
            "-nostdin",
            "-v",
            "error",
            "-y",
            "-i",
            &paths[0],
            "-vn",
            "-ar",
            "16000",
            "-ac",
            "1",
            "-c:a",
            "pcm_s16le",
        ])
        .arg(&wav);
        process::checked(cmd, 60).await.unwrap();
        let whisper = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("binaries")
            .join(format!("whisper-cli{}", std::env::consts::EXE_SUFFIX));
        let output = root.join("result");
        let mut cmd = process::command(&whisper);
        cmd.arg("-m")
            .arg(root.join("ggml-base.bin"))
            .arg("-f")
            .arg(&wav)
            .args(["-l", "en", "-t", "4", "-oj", "-pp", "-of"])
            .arg(&output);
        process::checked(cmd, 180).await.unwrap();
        let value: Value =
            serde_json::from_slice(&std::fs::read(output.with_extension("json")).unwrap()).unwrap();
        let segments = studio::parse_segments(&value).unwrap();
        let text = studio::render_transcript(&segments, "txt").unwrap();
        assert!(
            text.to_lowercase().contains("country"),
            "Unexpected transcript: {text}"
        );
        let srt = studio::render_transcript(&segments, "srt").unwrap();
        assert!(srt.contains(" --> "));
        std::fs::write(root.join("result.srt"), srt).unwrap();
        // Re-encode a real excerpt and verify a complete MP4 is produced.
        let mut cmd = process::command(Path::new(&ffmpeg));
        cmd.args([
            "-nostdin",
            "-v",
            "error",
            "-y",
            "-ss",
            "1.2",
            "-i",
            &paths[0],
            "-t",
            "2.5",
            "-c:v",
            "libx264",
            "-c:a",
            "aac",
            "-movflags",
            "+faststart",
        ])
        .arg(root.join("clip.mp4"));
        process::checked(cmd, 60).await.unwrap();
        assert!(std::fs::metadata(root.join("clip.mp4")).unwrap().len() > 1000);
    }
    #[tokio::test]
    #[ignore = "Requires FFmpeg; run npm run test:media"]
    async fn native_process_cancellation_is_prompt() {
        let ffmpeg = std::env::var("VIDEXTRACT_FFMPEG").unwrap_or_else(|_| "ffmpeg".into());
        let mut cmd = process::command(Path::new(&ffmpeg));
        cmd.args([
            "-nostdin",
            "-v",
            "error",
            "-re",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440",
            "-f",
            "null",
            "-",
        ]);
        let cancel = CancellationToken::new();
        let trigger = cancel.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            trigger.cancel();
        });
        let start = std::time::Instant::now();
        assert_eq!(
            process::run(cmd, cancel, |_| {}).await.unwrap_err(),
            "İptal edildi"
        );
        assert!(start.elapsed() < std::time::Duration::from_secs(3));
    }
}
