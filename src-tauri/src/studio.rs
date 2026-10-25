use crate::{data_dir, process, tool, AppState};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{ipc::Channel, AppHandle, Manager, State};
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

pub struct ModelSpec {
    pub id: &'static str,
    pub bytes: u64,
    pub sha256: &'static str,
}
pub const MODELS: [ModelSpec; 3] = [
    ModelSpec {
        id: "tiny",
        bytes: 77_691_713,
        sha256: "be07e048e1e599ad46341c8d2a135645097a538221678b7acdd1b1919c6e1b21",
    },
    ModelSpec {
        id: "base",
        bytes: 147_951_465,
        sha256: "60ed5bc3dd14eea856493d334349b405782ddcaf0028d4b5df4088345fba2efe",
    },
    ModelSpec {
        id: "small",
        bytes: 487_601_967,
        sha256: "1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b",
    },
];
fn specification(id: &str) -> Result<&'static ModelSpec, String> {
    MODELS
        .iter()
        .find(|m| m.id == id)
        .ok_or_else(|| "Geçersiz konuşma modeli".into())
}
pub fn model_file(app: &AppHandle, id: &str) -> Result<PathBuf, String> {
    specification(id)?;
    Ok(data_dir(app)?.join("models").join(format!("ggml-{id}.bin")))
}
pub fn model_path(app: &AppHandle) -> Result<PathBuf, String> {
    model_file(app, "base")
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    id: String,
    bytes: u64,
    installed: bool,
    minimum_memory: u64,
}
pub fn model_catalog(app: &AppHandle) -> Result<Vec<ModelInfo>, String> {
    MODELS
        .iter()
        .map(|m| {
            Ok(ModelInfo {
                id: m.id.into(),
                bytes: m.bytes,
                installed: model_file(app, m.id)?.is_file(),
                minimum_memory: crate::hardware::minimum_memory(m.id),
            })
        })
        .collect()
}
fn canonical_media(path: &str) -> Result<PathBuf, String> {
    let path = dunce::canonicalize(path).map_err(|_| "Medya dosyası bulunamadı")?;
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !path.is_file()
        || ![
            "mp4", "mkv", "webm", "mov", "avi", "m4v", "mp3", "wav", "m4a", "flac", "ogg", "opus",
            "aac",
        ]
        .contains(&extension.as_str())
    {
        return Err("Desteklenen bir video veya ses dosyası seç".into());
    }
    Ok(path)
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaInfo {
    path: String,
    title: String,
    duration: f64,
    playback_url: String,
}
#[tauri::command]
pub async fn prepare_media(app: AppHandle, path: String) -> Result<MediaInfo, String> {
    let path = canonical_media(&path)?;
    let mut cmd = process::command(&tool(&app, "ffprobe"));
    cmd.args([
        "-v",
        "error",
        "-show_entries",
        "format=duration",
        "-of",
        "json",
    ])
    .arg(&path);
    let result = process::checked(cmd, 30).await?;
    let value: Value = serde_json::from_str(&result).map_err(|e| e.to_string())?;
    let duration = value["format"]["duration"]
        .as_str()
        .and_then(|s| s.parse::<f64>().ok())
        .filter(|d| d.is_finite() && *d > 0.0)
        .ok_or("Dosya süresi okunamadı")?;
    let playback_url = app.state::<AppState>().media.register(&path).await?;
    Ok(MediaInfo {
        title: path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into(),
        path: path.to_string_lossy().into(),
        duration,
        playback_url,
    })
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudioEvent {
    phase: String,
    progress: Option<f64>,
    message: String,
}
pub(crate) fn emit(
    channel: &Channel<StudioEvent>,
    phase: &str,
    progress: Option<f64>,
    message: &str,
) {
    let _ = channel.send(StudioEvent {
        phase: phase.into(),
        progress,
        message: message.into(),
    });
}
// Temporary audio/model files are removed on success, errors and cancellation.
pub(crate) struct TempPath(pub(crate) PathBuf);
impl Drop for TempPath {
    fn drop(&mut self) {
        if self.0.is_dir() {
            let _ = std::fs::remove_dir_all(&self.0);
        } else {
            let _ = std::fs::remove_file(&self.0);
        }
    }
}
#[tauri::command]
pub async fn install_model(
    app: AppHandle,
    state: State<'_, AppState>,
    job_id: String,
    on_event: Channel<StudioEvent>,
    model_id: Option<String>,
) -> Result<(), String> {
    let spec = specification(model_id.as_deref().unwrap_or("base"))?;
    install_file(
        app.clone(),
        state,
        job_id,
        on_event,
        spec.bytes,
        spec.sha256,
        format!(
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-{}.bin",
            spec.id
        ),
        model_file(&app, spec.id)?,
    )
    .await
}
#[allow(clippy::too_many_arguments)]
pub(crate) async fn install_file(
    app: AppHandle,
    state: State<'_, AppState>,
    job_id: String,
    on_event: Channel<StudioEvent>,
    bytes: u64,
    sha256: &str,
    url: String,
    destination: PathBuf,
) -> Result<(), String> {
    let _ = app;
    let token = state.register(&job_id)?;
    let result=async {
        let _permit=tokio::select! {p=state.studio.acquire()=>p.map_err(|e|e.to_string())?,_=token.cancelled()=>return Err("İptal edildi".into())};
        if destination.is_file() {emit(&on_event,"ready",Some(100.0),"Model hazır");return Ok(());}
        tokio::fs::create_dir_all(destination.parent().ok_or("Model klasörü bulunamadı")?).await.map_err(|e|e.to_string())?;
        let temporary=TempPath(destination.with_extension("partial"));
        let client=reqwest::Client::builder().connect_timeout(std::time::Duration::from_secs(30)).timeout(std::time::Duration::from_secs(1800)).build().map_err(|e|e.to_string())?;
        emit(&on_event,"model",Some(0.0),"Konuşma modeli indiriliyor");
        let response=tokio::select! {
            r=client.get(&url).send()=>r.map_err(|e|format!("Model indirilemedi: {e}"))?.error_for_status().map_err(|e|e.to_string())?,
            _=token.cancelled()=>return Err("İptal edildi".into())
        };
        let mut stream=response.bytes_stream();
        let mut file=tokio::fs::File::create(&temporary.0).await.map_err(|e|e.to_string())?;
        let mut digest=Sha256::new(); let mut downloaded=0u64; let mut last_percent=0u64;
        loop {
            let chunk=tokio::select! {r=stream.next()=>r,_=token.cancelled()=>return Err("İptal edildi".into())};
            let Some(chunk)=chunk else {break}; let chunk=chunk.map_err(|e|e.to_string())?;
            downloaded+=chunk.len() as u64;
            if downloaded>bytes {return Err("Model boyutu doğrulanamadı".into());}
            file.write_all(&chunk).await.map_err(|e|e.to_string())?;digest.update(&chunk);
            let percent=downloaded*100/bytes;
            if percent>last_percent {emit(&on_event,"model",Some(percent as f64),"Model indiriliyor");last_percent=percent;}
        }
        file.flush().await.map_err(|e|e.to_string())?;drop(file);
        if downloaded!=bytes || format!("{:x}",digest.finalize())!=sha256 {return Err("Model SHA-256 doğrulaması başarısız; yeniden dene".into());}
        tokio::fs::rename(&temporary.0,&destination).await.map_err(|e|e.to_string())?;
        emit(&on_event,"ready",Some(100.0),"Model doğrulandı, çevrimdışı kullanıma hazır");Ok(())
    }.await;
    state.finish(&job_id);
    result
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Segment {
    pub start: f64,
    pub end: f64,
    pub text: String,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Transcript {
    pub id: String,
    pub media_path: String,
    pub media_title: String,
    pub created_at: u64,
    pub language: String,
    pub segments: Vec<Segment>,
}
pub(crate) fn parse_segments(value: &Value) -> Result<Vec<Segment>, String> {
    let rows = value["transcription"]
        .as_array()
        .ok_or("Transkript çıktısı okunamadı")?;
    let mut segments = Vec::new();
    for row in rows {
        let start = row["offsets"]["from"]
            .as_f64()
            .ok_or("Zaman damgası okunamadı")?
            / 1000.0;
        let end = row["offsets"]["to"]
            .as_f64()
            .ok_or("Zaman damgası okunamadı")?
            / 1000.0;
        let text = row["text"].as_str().unwrap_or("").trim().to_string();
        if !start.is_finite() || !end.is_finite() || start < 0.0 || end < start {
            return Err("Geçersiz zaman damgası".into());
        }
        if !text.is_empty() {
            segments.push(Segment { start, end, text });
        }
    }
    if segments.is_empty() {
        return Err("Bu dosyada konuşma algılanamadı".into());
    }
    Ok(segments)
}
pub const SPEECH_LANGUAGES: [&str; 10] =
    ["tr", "en", "es", "fr", "de", "pt", "ru", "ar", "hi", "zh"];
fn chunk_offsets(csv: &str) -> Result<Vec<f64>, String> {
    csv.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let mut fields = line.rsplitn(3, ',');
            let end = fields
                .next()
                .and_then(|s| s.parse::<f64>().ok())
                .ok_or("Ses bölümü okunamadı")?;
            let start = fields
                .next()
                .and_then(|s| s.parse::<f64>().ok())
                .ok_or("Ses bölümü okunamadı")?;
            if !start.is_finite() || !end.is_finite() || start < 0.0 || end < start {
                return Err("Geçersiz ses bölümü".into());
            }
            Ok(start)
        })
        .collect()
}
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri injects app, state and event channel alongside request fields.
pub async fn transcribe_media(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
    language: String,
    job_id: String,
    on_event: Channel<StudioEvent>,
    model_id: Option<String>,
    processing_mode: Option<String>,
) -> Result<Transcript, String> {
    if language != "auto" && !SPEECH_LANGUAGES.contains(&language.as_str()) {
        return Err("Desteklenmeyen dil".into());
    }
    let mode = processing_mode.as_deref().unwrap_or("auto");
    if !["auto", "gpu", "cpu"].contains(&mode) {
        return Err("Geçersiz işleme modu".into());
    }
    let media = canonical_media(&path)?;
    let selected_model = model_id.as_deref().unwrap_or("base");
    let model = model_file(&app, selected_model)?;
    if !model.is_file() {
        return Err("Önce seçili konuşma modelini indir".into());
    }
    let token = state.register(&job_id)?;
    let result=async {
        emit(&on_event,"queued",None,"İşlem sırası bekleniyor");
        let _permit=tokio::select!{p=state.studio.acquire()=>p.map_err(|e|e.to_string())?,_=token.cancelled()=>return Err("İptal edildi".into())};
        let device=crate::hardware::memory_profile().await?;
        if device.available_memory<crate::hardware::minimum_memory(selected_model){return Err("INSUFFICIENT_MEMORY".into());}
        let threads=device.threads.to_string();
        let mut use_gpu=mode!="cpu" && crate::hardware::vulkan_available(&app).await;
        if mode=="gpu" && !use_gpu{emit(&on_event,"cpuFallback",None,"GPU kullanılamıyor, CPU ile devam ediliyor");}
        let id=Uuid::new_v4().to_string();let root=data_dir(&app)?;let workspace=TempPath(root.join("work").join(&id));
        tokio::fs::create_dir_all(&workspace.0).await.map_err(|e|e.to_string())?;
        let list=workspace.0.join("chunks.csv");
        emit(&on_event,"audio",None,"Konuşma için ses hazırlanıyor");
        let mut cmd=process::command(&tool(&app,"ffmpeg"));
        cmd.args(["-nostdin","-v","error","-i"]).arg(&media).args(["-threads",&threads,"-vn","-ar","16000","-ac","1","-c:a","pcm_s16le","-f","segment","-segment_time","600","-reset_timestamps","1","-segment_list_type","csv","-segment_list"]).arg(&list).arg(workspace.0.join("audio-%05d.wav"));
        process::run(cmd,token.clone(),|_|{}).await?;
        // Bound decoder working memory for long lectures; CSV offsets preserve exact seek times.
        let offsets=chunk_offsets(&tokio::fs::read_to_string(&list).await.map_err(|e|e.to_string())?)?;
        if offsets.is_empty(){return Err("Bu dosyada konuşma algılanamadı".into());}
        let mut segments=Vec::new();let mut detected=language.clone();
        for (index,offset) in offsets.iter().enumerate(){
            let output=workspace.0.join(format!("transcript-{index:05}"));
            emit(&on_event,"transcribing",Some(index as f64/offsets.len() as f64*100.0),"Konuşmalar cihazında metne dönüştürülüyor");
            loop {
                let executable=if use_gpu {tool(&app,"whisper-cli-vulkan")} else {crate::speech_tool(&app)};
                let mut cmd=process::command(&executable);
                if !use_gpu{cmd.arg("-ng");}
                cmd.arg("-m").arg(&model).arg("-f").arg(workspace.0.join(format!("audio-{index:05}.wav"))).args(["-l",&language,"-t",&threads,"-oj","-pp","-of"]).arg(&output);
                emit(&on_event,if use_gpu{"gpuActive"}else{"cpuActive"},None,if use_gpu{"GPU · Vulkan"}else{"CPU"});
                let result=process::run(cmd,token.clone(),|line|{
                    if let Some((_,tail))=line.split_once("progress ="){
                        if let Ok(percent)=tail.trim().trim_end_matches('%').trim().parse::<f64>(){
                            emit(&on_event,"transcribing",Some(((index as f64+percent.clamp(0.0,100.0)/100.0)/offsets.len() as f64*100.0).min(100.0)),"Konuşmalar metne dönüştürülüyor");
                        }
                    }
                }).await;
                match result {
                    Ok(_)=>break,
                    Err(_) if use_gpu && !token.is_cancelled()=>{
                        use_gpu=false;
                        emit(&on_event,"cpuFallback",None,"GPU işlemi tamamlanamadı, CPU ile yeniden deneniyor");
                    }
                    Err(error)=>return Err(error),
                }
            }
            let value:Value=serde_json::from_slice(&tokio::fs::read(output.with_extension("json")).await.map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
            if index==0{detected=value["result"]["language"].as_str().unwrap_or(&language).into();}
            if value["transcription"].as_array().is_some_and(|rows|rows.is_empty()){continue;}
            for mut segment in parse_segments(&value)?{segment.start+=offset;segment.end+=offset;segments.push(segment);}
        }
        if segments.is_empty(){return Err("Bu dosyada konuşma algılanamadı".into());}
        let transcript=Transcript{id:id.clone(),media_path:media.to_string_lossy().into(),media_title:media.file_name().unwrap_or_default().to_string_lossy().into(),created_at:SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e|e.to_string())?.as_secs(),language:detected,segments};
        let directory=root.join("transcripts");tokio::fs::create_dir_all(&directory).await.map_err(|e|e.to_string())?;
        tokio::fs::write(directory.join(format!("{id}.json")),serde_json::to_vec_pretty(&transcript).map_err(|e|e.to_string())?).await.map_err(|e|e.to_string())?;
        emit(&on_event,"completed",Some(100.0),"Transkript hazır");Ok(transcript)
    }.await;
    state.finish(&job_id);
    result
}
#[tauri::command]
pub async fn list_transcripts(app: AppHandle) -> Result<Vec<Transcript>, String> {
    let path = data_dir(&app)?.join("transcripts");
    let mut items = Vec::new();
    if !path.exists() {
        return Ok(items);
    }
    let mut dir = tokio::fs::read_dir(path).await.map_err(|e| e.to_string())?;
    while let Some(entry) = dir.next_entry().await.map_err(|e| e.to_string())? {
        if entry.path().extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        if let Ok(bytes) = tokio::fs::read(entry.path()).await {
            if let Ok(item) = serde_json::from_slice::<Transcript>(&bytes) {
                items.push(item);
            }
        }
    }
    items.sort_by_key(|item| std::cmp::Reverse(item.created_at));
    Ok(items)
}
fn timestamp(seconds: f64, separator: char) -> String {
    let millis = (seconds.max(0.0) * 1000.0).round() as u64;
    format!(
        "{:02}:{:02}:{:02}{}{:03}",
        millis / 3_600_000,
        (millis / 60_000) % 60,
        (millis / 1000) % 60,
        separator,
        millis % 1000
    )
}
pub(crate) fn render_transcript(segments: &[Segment], format: &str) -> Result<String, String> {
    match format {
        "txt" => Ok(segments
            .iter()
            .map(|s| s.text.as_str())
            .collect::<Vec<_>>()
            .join("\n")),
        "srt" | "vtt" => {
            let mut result = if format == "vtt" {
                "WEBVTT\n\n".to_string()
            } else {
                String::new()
            };
            let separator = if format == "vtt" { '.' } else { ',' };
            for (i, s) in segments.iter().enumerate() {
                result.push_str(&format!(
                    "{}\n{} --> {}\n{}\n\n",
                    i + 1,
                    timestamp(s.start, separator),
                    timestamp(s.end, separator),
                    s.text
                ));
            }
            Ok(result)
        }
        _ => Err("Desteklenmeyen dışa aktarma biçimi".into()),
    }
}
#[tauri::command]
pub async fn export_transcript(
    app: AppHandle,
    id: String,
    format: String,
    path: String,
) -> Result<(), String> {
    let id = Uuid::parse_str(&id).map_err(|_| "Geçersiz transkript kimliği")?;
    let bytes = tokio::fs::read(
        data_dir(&app)?
            .join("transcripts")
            .join(format!("{id}.json")),
    )
    .await
    .map_err(|e| e.to_string())?;
    let transcript: Transcript = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    let output = render_transcript(&transcript.segments, &format)?;
    tokio::fs::write(path, output)
        .await
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub async fn extract_clip(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
    output: String,
    start: f64,
    end: f64,
    job_id: String,
) -> Result<(), String> {
    if !start.is_finite() || !end.is_finite() || start < 0.0 || end <= start {
        return Err("Geçerli başlangıç ve bitiş zamanı gir".into());
    }
    let media = prepare_media(app.clone(), path).await?;
    if end > media.duration + 0.1 {
        return Err("Bitiş zamanı video süresini aşıyor".into());
    }
    let output = PathBuf::from(output);
    if !output.is_absolute() || output.extension().and_then(|s| s.to_str()) != Some("mp4") {
        return Err("Çıktı için bir MP4 dosyası seç".into());
    }
    if output.exists() {
        return Err("Bu dosya zaten var; yeni bir dosya adı seç".into());
    }
    let token = state.register(&job_id)?;
    let result=async {
        let _permit=tokio::select! {p=state.studio.acquire()=>p.map_err(|e|e.to_string())?,_=token.cancelled()=>return Err("İptal edildi".into())};
        let temporary=TempPath(output.with_file_name(format!(".vidextract-{}.mp4",Uuid::new_v4())));
        let threads=crate::hardware::memory_profile().await?.threads.to_string();
        let mut cmd=process::command(&tool(&app,"ffmpeg"));
        cmd.args(["-nostdin","-v","error","-n","-ss",&start.to_string(),"-i",&media.path,"-t",&(end-start).to_string(),"-map","0:v:0?","-map","0:a:0?","-c:v","libx264","-threads",&threads,"-preset","veryfast","-crf","20","-c:a","aac","-movflags","+faststart"]).arg(&temporary.0);
        process::run(cmd,token.clone(),|_|{}).await?;
        // A hard link atomically refuses to overwrite a destination created during encoding.
        tokio::fs::hard_link(&temporary.0,&output).await.map_err(|e|format!("Kesit kaydedilemedi; başka bir dosya adı dene: {e}"))?;
        Ok(())
    }.await;
    state.finish(&job_id);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn audio_chunks_use_true_offsets_and_multilingual_models() {
        assert_eq!(
            chunk_offsets(
                "audio-00000.wav,0.000000,600.064000\naudio-00001.wav,600.064000,900.150000\n"
            )
            .unwrap(),
            vec![0.0, 600.064]
        );
        assert!(chunk_offsets("a,-1,4").is_err());
        assert!(chunk_offsets("a,NaN,4").is_err());
        for model in MODELS {
            assert!(!model.id.ends_with(".en"));
            assert_eq!(model.sha256.len(), 64);
        }
        assert_eq!(SPEECH_LANGUAGES.len(), 10);
        assert!(SPEECH_LANGUAGES.contains(&"hi"));
        assert!(SPEECH_LANGUAGES.contains(&"pt"));
    }
    #[test]
    fn whisper_milliseconds_become_seekable_seconds() {
        let value = serde_json::json!({"transcription":[{"offsets":{"from":1250,"to":2700},"text":"  Merhaba dünya. "}]});
        let s = parse_segments(&value).unwrap();
        assert_eq!(s[0].start, 1.25);
        assert_eq!(s[0].text, "Merhaba dünya.");
        assert!(parse_segments(&serde_json::json!({"transcription":[]})).is_err());
    }
    #[test]
    fn subtitle_exports_preserve_unicode_and_carry_milliseconds() {
        let segments = vec![Segment {
            start: 59.9999,
            end: 3661.25,
            text: "Öğrenmeye devam!".into(),
        }];
        let srt = render_transcript(&segments, "srt").unwrap();
        assert!(srt.contains("00:01:00,000 --> 01:01:01,250"));
        assert!(srt.contains("Öğrenmeye devam!"));
        assert!(render_transcript(&segments, "vtt")
            .unwrap()
            .starts_with("WEBVTT\n\n1\n00:01:00.000"));
        assert_eq!(
            render_transcript(&segments, "txt").unwrap(),
            "Öğrenmeye devam!"
        );
        assert!(render_transcript(&segments, "html").is_err());
    }
}
