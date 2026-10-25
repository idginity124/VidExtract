//! Real Tauri application/IPC service smoke test; uses an isolated XDG_DATA_HOME on Linux.
//! VIDEXTRACT_NATIVE_SMOKE_DIR must contain speech.mp4, ggml-base.bin, plus an HTTP server at :18462.
use std::path::PathBuf;
use tauri::{ipc::Channel, Manager};
use vidextract_lib::{desktop_builder, AppState};

fn main() {
    let root = PathBuf::from(
        std::env::var("VIDEXTRACT_NATIVE_SMOKE_DIR").expect("Set the fixture directory"),
    );
    let report = root.join("native-result.json");
    desktop_builder()
        .setup(move |app| {
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let result: Result<(), String> = async {
                    let data = handle.path().app_data_dir().map_err(|e| e.to_string())?;
                    // Refuse to modify a real user's application directory.
                    if !data.starts_with(&root) { return Err("Use an isolated XDG_DATA_HOME under the fixture directory".into()); }
                    std::fs::create_dir_all(data.join("models")).map_err(|e| e.to_string())?;
                    let model=data.join("models/ggml-base.bin");
                    if model.exists() {std::fs::remove_file(&model).map_err(|e|e.to_string())?;}
                    let cancel_handle=handle.clone();
                    tauri::async_runtime::spawn(async move {
                        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                        let _=vidextract_lib::cancel_download(cancel_handle.state::<AppState>(),"native-model-cancel".into());
                    });
                    let canceled=vidextract_lib::studio::install_model(handle.clone(),handle.state::<AppState>(),"native-model-cancel".into(),Channel::new(|_|Ok(())),None).await;
                    if canceled.is_ok() || model.exists() || data.join("models/ggml-base.partial").exists() {return Err("Canceled model download left incomplete data".into());}
                    vidextract_lib::studio::install_model(handle.clone(),handle.state::<AppState>(),"native-model".into(),Channel::new(|_|Ok(())),None).await?;
                    let status = vidextract_lib::check_dependencies(handle.clone()).await?;
                    let status = serde_json::to_value(status).map_err(|e| e.to_string())?;
                    for key in ["ytDlp", "ffmpeg", "ffprobe", "whisper", "modelReady"] { if status[key] != true {return Err(format!("Dependency not ready: {key}: {status}"));} }
                    let url = "http://127.0.0.1:18462/speech.mp4".to_string();
                    vidextract_lib::analyze_url(handle.clone(), url.clone()).await?;
                    let request = serde_json::from_value(serde_json::json!({"jobId":"native-download","url":url,"outputDir":root.join("native-downloads"),"mode":"video","quality":"1080p","format":"mp4","playlist":false,"subtitles":false,"saveThumbnail":false})).map_err(|e| e.to_string())?;
                    vidextract_lib::start_download(handle.clone(), handle.state::<AppState>(), request, Channel::new(|_| Ok(()))).await?;
                    let path = root.join("speech.mp4").to_string_lossy().to_string();
                    vidextract_lib::studio::prepare_media(handle.clone(),path.clone()).await?;
                    let transcript = vidextract_lib::studio::transcribe_media(handle.clone(),handle.state::<AppState>(),path.clone(),"en".into(),"native-transcribe".into(),Channel::new(|_| Ok(())),None,None).await?;
                    let value=serde_json::to_value(&transcript).map_err(|e|e.to_string())?;
                    let id=value["id"].as_str().ok_or("No transcript id")?.to_string();
                    if !value["segments"].as_array().ok_or("No segments")?.iter().any(|s|s["text"].as_str().unwrap_or("").to_lowercase().contains("country")) {return Err("Transcript does not contain expected speech".into());}
                    for format in ["txt","srt","vtt"] {
                        vidextract_lib::studio::export_transcript(handle.clone(),id.clone(),format.into(),root.join(format!("native-export.{format}")).to_string_lossy().into()).await?;
                    }
                    if vidextract_lib::studio::list_transcripts(handle.clone()).await?.is_empty() {return Err("Transcript not persisted".into());}
                    let clip=root.join("native-clip.mp4");let _=std::fs::remove_file(&clip);
                    vidextract_lib::studio::extract_clip(handle.clone(),handle.state::<AppState>(),path.clone(),clip.to_string_lossy().into(),1.0,3.0,"native-clip".into()).await?;
                    // Existing outputs must remain untouched.
                    if vidextract_lib::studio::extract_clip(handle.clone(),handle.state::<AppState>(),path,clip.to_string_lossy().into(),1.0,3.0,"native-clip-existing".into()).await.is_ok() {return Err("Existing output was overwritten".into());}
                    std::fs::write(root.join("device-profile.json"),serde_json::to_vec_pretty(&status).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
                    if root.join("turkish.mp4").is_file() {
                        for model in ["tiny","small"] {
                            if model=="small"&&std::env::var_os("VIDEXTRACT_TEST_SMALL").is_none(){continue;}
                            vidextract_lib::studio::install_model(handle.clone(),handle.state::<AppState>(),format!("install-{model}"),Channel::new(|_|Ok(())),Some(model.into())).await?;
                            let output=vidextract_lib::studio::transcribe_media(handle.clone(),handle.state::<AppState>(),root.join("turkish.mp4").to_string_lossy().into(),"tr".into(),format!("turkish-{model}"),Channel::new(|_|Ok(())),Some(model.into()),None).await?;
                            let value=serde_json::to_value(&output).map_err(|e|e.to_string())?;
                            let text=value["segments"].as_array().ok_or("No Turkish segments")?.iter().filter_map(|s|s["text"].as_str()).collect::<Vec<_>>().join(" ").to_lowercase();
                            if value["language"]!="tr"||!["bir","bu","ve","kas","güç","adam"].iter().any(|word|text.contains(word)){return Err(format!("Turkish {model} transcription not recognized: {text}"));}
                            std::fs::write(root.join(format!("turkish-{model}.json")),serde_json::to_vec_pretty(&output).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
                        }
                    }
                    Ok(())
                }.await;
                let success=result.is_ok();
                let _=std::fs::write(report,serde_json::json!({"passed":success,"error":result.err()}).to_string());
                handle.exit(if success {0} else {1});
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Native smoke test app failed");
}
