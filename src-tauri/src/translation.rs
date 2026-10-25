use crate::{
    data_dir, process,
    studio::{self, Segment, StudioEvent, TempPath, Transcript},
    tool, AppState,
};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    time::Duration,
};
use tauri::{ipc::Channel, AppHandle, State};
use tokio::io::AsyncWriteExt;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
#[derive(Deserialize)]
struct Pack {
    from: String,
    to: String,
    version: String,
    url: String,
    bytes: u64,
    sha256: String,
    #[serde(rename = "expandedBytes")]
    expanded_bytes: u64,
}
fn packs() -> Vec<Pack> {
    serde_json::from_str(include_str!("../translation-packs.json")).expect("bundled pack catalog")
}
fn route(from: &str, to: &str) -> Result<Vec<Pack>, String> {
    if from == to {
        return Ok(vec![]);
    }
    let mut pairs = vec![];
    if from != "en" {
        pairs.push((from, "en"));
    }
    if to != "en" {
        pairs.push(("en", to));
    }
    let mut catalog = packs();
    pairs
        .into_iter()
        .map(|(a, b)| {
            let index = catalog
                .iter()
                .position(|p| p.from == a && p.to == b)
                .ok_or("TRANSLATION_OFFLINE_LANGUAGE")?;
            Ok(catalog.remove(index))
        })
        .collect()
}
fn pack_path(app: &AppHandle, p: &Pack) -> Result<PathBuf, String> {
    Ok(data_dir(app)?
        .join("models/translation-packs")
        .join(format!("{}-{}-{}", p.from, p.to, p.version)))
}
fn pack_ready(path: &Path, p: &Pack) -> bool {
    path.join("sentencepiece.model").is_file()
        && path.join("model/model.bin").is_file()
        && std::fs::read_to_string(path.join("verified.sha256")).is_ok_and(|hash| hash == p.sha256)
}
#[derive(Default, Deserialize, Serialize)]
pub struct TranslationSettings {
    pub email: String,
}
fn read_settings(app: &AppHandle) -> TranslationSettings {
    std::fs::read(
        data_dir(app)
            .unwrap_or_default()
            .join("translation-settings.json"),
    )
    .ok()
    .and_then(|b| serde_json::from_slice(&b).ok())
    .unwrap_or_default()
}
#[tauri::command]
pub async fn save_translation_settings(app: AppHandle, email: String) -> Result<(), String> {
    let email = email.trim().to_string();
    if !email.is_empty()
        && (email.len() > 254
            || !email.contains('@')
            || email.contains(char::is_whitespace)
            || email.contains(['\r', '\n']))
    {
        return Err("TRANSLATION_EMAIL".into());
    }
    atomic_json(
        &data_dir(&app)?.join("translation-settings.json"),
        &TranslationSettings { email },
    )
    .await
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationStatus {
    email: String,
    daily_limit: usize,
    offline_supported: bool,
    offline_installed: bool,
    offline_bytes: u64,
    offline_engine: bool,
}
#[tauri::command]
pub fn translation_status(
    app: AppHandle,
    source_language: String,
    target_language: String,
) -> Result<TranslationStatus, String> {
    let settings = read_settings(&app);
    let route = route(&source_language, &target_language);
    let mut bytes = 0;
    let mut installed = true;
    if let Ok(ref route) = route {
        for pack in route {
            if !pack_ready(&pack_path(&app, pack)?, pack) {
                installed = false;
                bytes += pack.bytes;
            }
        }
    } else {
        installed = false;
    }
    Ok(TranslationStatus {
        daily_limit: if settings.email.is_empty() {
            5000
        } else {
            50000
        },
        email: settings.email,
        offline_supported: route.is_ok(),
        offline_installed: installed,
        offline_bytes: bytes,
        offline_engine: tool(&app, "translate-cpu").is_file(),
    })
}
#[tauri::command]
pub async fn open_translation_terms() -> Result<(), String> {
    #[cfg(windows)]
    let mut cmd = process::command(Path::new("rundll32.exe"));
    #[cfg(windows)]
    cmd.arg("url.dll,FileProtocolHandler");
    #[cfg(not(windows))]
    let mut cmd = process::command(Path::new("xdg-open"));
    cmd.arg("https://mymemory.translated.net/doc/usagelimits.php");
    process::checked(cmd, 15).await.map(|_| ())
}
async fn atomic_json(path: &Path, value: &impl Serialize) -> Result<(), String> {
    tokio::fs::create_dir_all(path.parent().ok_or("Kayıt klasörü bulunamadı")?)
        .await
        .map_err(|e| e.to_string())?;
    let temp = TempPath(path.with_extension(format!("{}.partial", Uuid::new_v4())));
    tokio::fs::write(
        &temp.0,
        serde_json::to_vec(value).map_err(|e| e.to_string())?,
    )
    .await
    .map_err(|e| e.to_string())?;
    tokio::fs::rename(&temp.0, path)
        .await
        .map_err(|e| e.to_string())
}
fn extract_pack(
    archive: &Path,
    destination: &Path,
    p: &Pack,
    token: &CancellationToken,
) -> Result<(), String> {
    let mut zip = zip::ZipArchive::new(std::fs::File::open(archive).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    if zip.len() > 500 {
        return Err("Geçersiz dil paketi".into());
    }
    let mut total = 0;
    for i in 0..zip.len() {
        if token.is_cancelled() {
            return Err("İptal edildi".into());
        }
        let mut file = zip.by_index(i).map_err(|e| e.to_string())?;
        let safe = file.enclosed_name().ok_or("Geçersiz paket yolu")?;
        let relative = safe.components().skip(1).collect::<PathBuf>();
        if relative.as_os_str().is_empty() || file.is_dir() {
            continue;
        }
        if !(relative.starts_with("model")
            || [
                "sentencepiece.model",
                "metadata.json",
                "README.md",
                "LICENSE",
                "LICENSE.txt",
            ]
            .iter()
            .any(|n| relative == Path::new(n)))
        {
            continue;
        }
        total += file.size();
        if total > p.expanded_bytes + 2 * 1024 * 1024 || file.size() > 500 * 1024 * 1024 {
            return Err("Geçersiz paket boyutu".into());
        }
        let output = destination.join(&relative);
        std::fs::create_dir_all(output.parent().ok_or("Geçersiz paket")?)
            .map_err(|e| e.to_string())?;
        let mut writer = std::fs::File::create(output).map_err(|e| e.to_string())?;
        std::io::copy(&mut file, &mut writer).map_err(|e| e.to_string())?;
    }
    let meta: Value = serde_json::from_slice(
        &std::fs::read(destination.join("metadata.json")).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    if meta["from_code"] != p.from
        || meta["to_code"] != p.to
        || !destination.join("model/model.bin").is_file()
        || !destination.join("sentencepiece.model").is_file()
    {
        return Err("Dil paketi doğrulanamadı".into());
    }
    std::fs::write(destination.join("verified.sha256"), &p.sha256).map_err(|e| e.to_string())
}
#[tauri::command]
pub async fn install_translation_model(
    app: AppHandle,
    state: State<'_, AppState>,
    source_language: String,
    target_language: String,
    job_id: String,
    on_event: Channel<StudioEvent>,
) -> Result<(), String> {
    let route = route(&source_language, &target_language)?;
    let token = state.register(&job_id)?;
    let result=async{
  let _permit=tokio::select!{p=state.studio.acquire()=>p.map_err(|e|e.to_string())?,_=token.cancelled()=>return Err("İptal edildi".into())};
  let client=reqwest::Client::builder().user_agent("VidExtract/2.3.0").connect_timeout(Duration::from_secs(20)).timeout(Duration::from_secs(1800)).build().map_err(|_|"TRANSLATION_NETWORK")?;
  let mut completed=0u64;let total=route.iter().filter(|p|!pack_ready(&pack_path(&app,p).unwrap_or_default(),p)).map(|p|p.bytes).sum::<u64>().max(1);
  for pack in route{
   let destination=pack_path(&app,&pack)?;if pack_ready(&destination,&pack){continue;}
   tokio::fs::create_dir_all(destination.parent().ok_or("Paket klasörü bulunamadı")?).await.map_err(|e|e.to_string())?;
   let archive=TempPath(destination.with_extension(format!("{}.download",Uuid::new_v4())));let response=tokio::select!{r=client.get(&pack.url).send()=>r.map_err(|_|"TRANSLATION_NETWORK")?.error_for_status().map_err(|_|"TRANSLATION_NETWORK")?,_=token.cancelled()=>return Err("İptal edildi".into())};
   let mut file=tokio::fs::File::create(&archive.0).await.map_err(|e|e.to_string())?;let mut hash=Sha256::new();let mut downloaded=0;let mut stream=response.bytes_stream();let mut last=101;
   loop{let chunk=tokio::select!{r=stream.next()=>r,_=token.cancelled()=>return Err("İptal edildi".into())};let Some(chunk)=chunk else{break};let chunk=chunk.map_err(|_|"TRANSLATION_NETWORK")?;downloaded+=chunk.len() as u64;if downloaded>pack.bytes{return Err("Dil paketi doğrulanamadı".into());}hash.update(&chunk);file.write_all(&chunk).await.map_err(|e|e.to_string())?;let percent=(completed+downloaded)*100/total;if percent!=last{studio::emit(&on_event,"translationModel",Some(percent as f64),"");last=percent;}}
   file.flush().await.map_err(|e|e.to_string())?;drop(file);if downloaded!=pack.bytes||format!("{:x}",hash.finalize())!=pack.sha256{return Err("Dil paketi doğrulanamadı".into());}
   let temporary=TempPath(destination.with_extension(format!("{}.partial",Uuid::new_v4())));let unpack=temporary.0.clone();let input=archive.0.clone();let cancel=token.clone();
   tokio::task::spawn_blocking(move||extract_pack(&input,&unpack,&pack,&cancel)).await.map_err(|e|e.to_string())??;
   if token.is_cancelled(){return Err("İptal edildi".into());}
   if destination.exists(){tokio::fs::remove_dir_all(&destination).await.map_err(|e|e.to_string())?;}tokio::fs::rename(&temporary.0,&destination).await.map_err(|e|e.to_string())?;completed+=downloaded;
  }Ok(())
 }.await;
    state.finish(&job_id);
    result
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Translation {
    pub transcript_id: String,
    pub target_language: String,
    pub segments: Vec<Segment>,
}
fn mode(value: Option<&str>) -> Result<&str, String> {
    match value.unwrap_or("online") {
        "online" => Ok("online"),
        "offline" => Ok("offline"),
        _ => Err("Geçersiz çeviri yöntemi".into()),
    }
}
async fn source(
    app: &AppHandle,
    id: &str,
    target: &str,
    method: &str,
) -> Result<(Transcript, PathBuf), String> {
    if !studio::SPEECH_LANGUAGES.contains(&target) {
        return Err("Geçersiz çeviri dili".into());
    }
    let id = Uuid::parse_str(id).map_err(|_| "Geçersiz transkript")?;
    let bytes = tokio::fs::read(
        data_dir(app)?
            .join("transcripts")
            .join(format!("{id}.json")),
    )
    .await
    .map_err(|e| e.to_string())?;
    let fingerprint = format!("{:x}", Sha256::digest(&bytes));
    let transcript: Transcript = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    Ok((
        transcript,
        data_dir(app)?.join("translations").join(format!(
            "{id}-{target}-{method}-v2-{}.json",
            &fingerprint[..16]
        )),
    ))
}
#[tauri::command]
pub async fn cached_translation(
    app: AppHandle,
    id: String,
    target_language: String,
    translation_mode: Option<String>,
) -> Result<Option<Translation>, String> {
    let (_, path) = source(
        &app,
        &id,
        &target_language,
        mode(translation_mode.as_deref())?,
    )
    .await?;
    match tokio::fs::read(path).await {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|e| e.to_string()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}
// MyMemory permits 500 UTF-8 bytes per request. Prefer word/sentence boundaries, never split a code point.
fn units(segments: &[Segment]) -> Vec<(usize, String)> {
    let mut out = vec![];
    for (index, row) in segments.iter().enumerate() {
        let mut text = row.text.trim();
        while !text.is_empty() {
            let mut boundary = text.len().min(450);
            while !text.is_char_boundary(boundary) {
                boundary -= 1;
            }
            if boundary < text.len() {
                if let Some((i, c)) = text[..boundary].char_indices().rev().find(|(i, c)| {
                    *i > boundary / 2 && (c.is_whitespace() || ".!?。！？".contains(*c))
                }) {
                    boundary = i + c.len_utf8();
                }
            }
            let part = text[..boundary].trim();
            if !part.is_empty() {
                out.push((index, part.into()));
            }
            text = text[boundary..].trim_start();
        }
    }
    out
}
fn chunk_path(app: &AppHandle, source: &str, target: &str, text: &str) -> Result<PathBuf, String> {
    let hash = format!("{:x}", Sha256::digest(format!("{source}|{target}|{text}")));
    Ok(data_dir(app)?
        .join("translations/chunks-mymemory-v1")
        .join(format!("{hash}.json")))
}
async fn online(
    app: &AppHandle,
    transcript: &Transcript,
    target: &str,
    parts: &[(usize, String)],
    event: &Channel<StudioEvent>,
) -> Result<Vec<String>, String> {
    let settings = read_settings(app);
    let limit = if settings.email.is_empty() {
        5000
    } else {
        50000
    };
    let mut unique = HashSet::new();
    let mut needed = 0;
    for (_, text) in parts {
        if unique.insert(text) && !chunk_path(app, &transcript.language, target, text)?.is_file() {
            needed += text.chars().count();
        }
    }
    if needed > limit {
        return Err("TRANSLATION_TOO_LONG".into());
    }
    let client = reqwest::Client::builder()
        .user_agent("VidExtract/2.3.0")
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(40))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| "TRANSLATION_NETWORK")?;
    let pair = format!("{}|{}", transcript.language, target);
    let mut rows = vec![];
    let mut memory = HashMap::<String, String>::new();
    for (index, (_, text)) in parts.iter().enumerate() {
        studio::emit(
            event,
            "translating",
            Some(index as f64 / parts.len().max(1) as f64 * 100.0),
            "",
        );
        if let Some(cached) = memory.get(text) {
            rows.push(cached.clone());
            continue;
        }
        let path = chunk_path(app, &transcript.language, target, text)?;
        let cached = tokio::fs::read(&path)
            .await
            .ok()
            .and_then(|b| serde_json::from_slice::<String>(&b).ok())
            .filter(|s| !s.trim().is_empty());
        let value = if let Some(cached) = cached {
            cached
        } else {
            let mut query = vec![
                ("q", text.as_str()),
                ("langpair", pair.as_str()),
                ("mt", "1"),
            ];
            if !settings.email.is_empty() {
                query.push(("de", &settings.email));
            }
            let response = client
                .get("https://api.mymemory.translated.net/get")
                .query(&query)
                .send()
                .await
                .map_err(|_| "TRANSLATION_NETWORK")?;
            if response.status() == 429 {
                return Err("TRANSLATION_QUOTA".into());
            }
            if !response.status().is_success() {
                return Err("TRANSLATION_NETWORK".into());
            }
            let value: Value = response.json().await.map_err(|_| "TRANSLATION_RESPONSE")?;
            if value["quotaFinished"] == true
                || value["responseStatus"] == 429
                || value["responseDetails"]
                    .as_str()
                    .is_some_and(|s| s.to_uppercase().contains("LIMIT"))
            {
                return Err("TRANSLATION_QUOTA".into());
            }
            if value["responseStatus"] != 200 {
                return Err("TRANSLATION_RESPONSE".into());
            }
            let translated = value["responseData"]["translatedText"]
                .as_str()
                .ok_or("TRANSLATION_RESPONSE")?;
            let translated = html_escape::decode_html_entities(translated)
                .trim()
                .to_string();
            if translated.is_empty() || translated.starts_with("MYMEMORY WARNING") {
                return Err("TRANSLATION_RESPONSE".into());
            }
            atomic_json(&path, &translated).await?;
            translated
        };
        memory.insert(text.clone(), value.clone());
        rows.push(value);
    }
    Ok(rows)
}
async fn offline(
    app: &AppHandle,
    transcript: &Transcript,
    target: &str,
    parts: &[(usize, String)],
    event: &Channel<StudioEvent>,
    token: CancellationToken,
) -> Result<Vec<String>, String> {
    let device = crate::hardware::memory_profile().await?;
    if device.available_memory < 700 * 1024 * 1024 {
        return Err("INSUFFICIENT_MEMORY".into());
    }
    let route = route(&transcript.language, target)?;
    let work = TempPath(data_dir(app)?.join("work").join(Uuid::new_v4().to_string()));
    tokio::fs::create_dir_all(&work.0)
        .await
        .map_err(|e| e.to_string())?;
    let mut texts = parts.iter().map(|(_, s)| s.clone()).collect::<Vec<_>>();
    for (index, pack) in route.iter().enumerate() {
        let directory = pack_path(app, pack)?;
        if !pack_ready(&directory, pack) {
            return Err("TRANSLATION_MODEL_REQUIRED".into());
        }
        let meta: Value = serde_json::from_slice(
            &tokio::fs::read(directory.join("metadata.json"))
                .await
                .map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        let input = work.0.join("input.json");
        let output = work.0.join("output.json");
        tokio::fs::write(
            &input,
            serde_json::to_vec(
                &json!({"texts":texts,"prefix":meta["target_prefix"].as_str().unwrap_or("")}),
            )
            .map_err(|e| e.to_string())?,
        )
        .await
        .map_err(|e| e.to_string())?;
        let mut cmd = process::command(&tool(app, "translate-cpu"));
        cmd.arg(directory)
            .arg(&input)
            .arg(&output)
            .arg(device.threads.to_string());
        process::run(cmd, token.clone(), |line| {
            if let Some((_, percent)) = line.split_once("progress =") {
                if let Ok(value) = percent.trim().parse::<f64>() {
                    studio::emit(
                        event,
                        "translating",
                        Some(
                            (index as f64 + value.clamp(0.0, 100.0) / 100.0)
                                / route.len().max(1) as f64
                                * 100.0,
                        ),
                        "",
                    );
                }
            }
        })
        .await?;
        texts = serde_json::from_slice(&tokio::fs::read(output).await.map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        if texts.len() != parts.len() {
            return Err("TRANSLATION_RESPONSE".into());
        }
    }
    Ok(texts)
}
#[tauri::command]
pub async fn translate_transcript(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    target_language: String,
    job_id: String,
    on_event: Channel<StudioEvent>,
    translation_mode: Option<String>,
) -> Result<Translation, String> {
    let method = mode(translation_mode.as_deref())?;
    let (transcript, path) = source(&app, &id, &target_language, method).await?;
    if let Some(cached) = cached_translation(
        app.clone(),
        id.clone(),
        target_language.clone(),
        Some(method.into()),
    )
    .await?
    {
        return Ok(cached);
    }
    let token = state.register(&job_id)?;
    let result = tokio::select! {result=async{
     let _permit=state.studio.acquire().await.map_err(|e|e.to_string())?;if let Some(cached)=cached_translation(app.clone(),id.clone(),target_language.clone(),Some(method.into())).await?{return Ok(cached);}
     let parts=units(&transcript.segments);studio::emit(&on_event,"translating",Some(0.0),"");let texts=if method=="offline"{offline(&app,&transcript,&target_language,&parts,&on_event,token.clone()).await?}else{online(&app,&transcript,&target_language,&parts,&on_event).await?};
     let mut output=transcript.segments.clone();for row in &mut output{row.text.clear();}for ((index,_),text) in parts.iter().zip(texts){if !output[*index].text.is_empty(){output[*index].text.push(' ');}output[*index].text.push_str(text.trim());}
     let result=Translation{transcript_id:id,target_language,segments:output};atomic_json(&path,&result).await?;studio::emit(&on_event,"completed",Some(100.0),"");Ok(result)
    }=>result,_=token.cancelled()=>Err("İptal edildi".into())};
    state.finish(&job_id);
    result
}
#[tauri::command]
pub async fn export_translation(
    app: AppHandle,
    id: String,
    target_language: String,
    format: String,
    path: String,
    translation_mode: Option<String>,
) -> Result<(), String> {
    let translated = cached_translation(app, id, target_language, translation_mode)
        .await?
        .ok_or("Önce çeviriyi tamamla")?;
    tokio::fs::write(
        path,
        studio::render_transcript(&translated.segments, &format)?,
    )
    .await
    .map_err(|e| e.to_string())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn chunks_fit_utf8_limit_without_losing_unicode() {
        let input = "ğ你🙂".repeat(400);
        let parts = units(&[Segment {
            start: 0.0,
            end: 1.0,
            text: input.clone(),
        }]);
        assert!(parts.iter().all(|(_, s)| s.len() <= 450));
        assert_eq!(parts.into_iter().map(|(_, s)| s).collect::<String>(), input);
    }
    #[test]
    fn all_language_pairs_have_bounded_pivot_routes() {
        for from in studio::SPEECH_LANGUAGES {
            for to in studio::SPEECH_LANGUAGES {
                let route = route(from, to).unwrap();
                assert!(route.len() <= 2);
                for p in route {
                    assert!(p.bytes < 400_000_000);
                    assert_eq!(p.sha256.len(), 64);
                }
            }
        }
        assert!(route("../../file", "tr").is_err());
    }
}
