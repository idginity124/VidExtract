//! Isolated real-engine check: GPU, forced CPU, and unavailable-driver fallback.
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Instant,
};
use tauri::{ipc::Channel, Manager};
use vidextract_lib::{desktop_builder, AppState};
fn main() {
    let root = PathBuf::from(
        std::env::var("VIDEXTRACT_GPU_SMOKE_DIR").expect("isolated fixture directory"),
    );
    desktop_builder().setup(move|app|{
 let handle=app.handle().clone();
 tauri::async_runtime::spawn(async move{
 let result:Result<serde_json::Value,String>=async{
 let data=handle.path().app_data_dir().map_err(|e|e.to_string())?;
 if !data.starts_with(&root){return Err("Use isolated XDG_DATA_HOME".into());}
 std::fs::create_dir_all(data.join("models")).map_err(|e|e.to_string())?;
 std::fs::copy(root.join("ggml-base.bin"),data.join("models/ggml-base.bin")).map_err(|e|e.to_string())?;
 let available=vidextract_lib::hardware::vulkan_available(&handle).await;
 let expected=std::env::var("VIDEXTRACT_EXPECT_GPU").as_deref()==Ok("1");
 if available!=expected{return Err(format!("Unexpected GPU availability: {available}"));}
 let mut rows=vec![];
 for mode in if expected{vec!["gpu","cpu","auto"]}else{vec!["gpu"]}{
 let events=Arc::new(Mutex::new(Vec::new()));let sink=events.clone();let start=Instant::now();
 let transcript=vidextract_lib::studio::transcribe_media(handle.clone(),handle.state::<AppState>(),root.join("speech.mp4").to_string_lossy().into(),"auto".into(),format!("gpu-smoke-{mode}"),Channel::new(move|body|{sink.lock().unwrap().push(format!("{body:?}"));Ok(())}),Some("base".into()),Some(mode.into())).await?;
 let value=serde_json::to_value(transcript).map_err(|e|e.to_string())?;
 if value["language"]!="en"{return Err("Automatic language detection failed".into());}
 let text=value["segments"].as_array().ok_or("No segments")?.iter().filter_map(|s|s["text"].as_str()).collect::<Vec<_>>().join(" ");
 if !text.to_lowercase().contains("country"){return Err(format!("Wrong transcript: {text}"));}
 let log=events.lock().unwrap().join("\n");
 let phase=if mode=="cpu"{"cpuActive"}else if expected{"gpuActive"}else{"cpuFallback"};
 if !log.contains(phase){return Err(format!("Missing backend event {phase}: {log}"));}
 rows.push(serde_json::json!({"mode":mode,"seconds":start.elapsed().as_secs_f64(),"text":text,"events":log}));
 }
 Ok(serde_json::json!({"passed":true,"gpuAvailable":available,"runs":rows}))
 }.await;
 let passed=result.is_ok();let report=result.unwrap_or_else(|e|serde_json::json!({"passed":false,"error":e}));
 let _=std::fs::write(root.join(if std::env::var("VIDEXTRACT_EXPECT_GPU").as_deref()==Ok("1"){ "gpu-result.json" }else{"fallback-result.json"}),report.to_string());
 handle.exit(if passed{0}else{1});
 });Ok(())}).run(tauri::generate_context!()).expect("GPU smoke application");
}
