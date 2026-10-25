//! Runs actual offline translation in isolated application storage.
use std::{path::PathBuf, time::Instant};
use tauri::{ipc::Channel, Manager};
use vidextract_lib::{desktop_builder, translation, AppState};
fn main() {
    let root = PathBuf::from(
        std::env::var("VIDEXTRACT_TRANSLATION_SMOKE_DIR").expect("isolated fixture directory"),
    );
    desktop_builder().setup(move|app|{let handle=app.handle().clone();tauri::async_runtime::spawn(async move{
 let result:Result<serde_json::Value,String>=async{
 let data=handle.path().app_data_dir().map_err(|e|e.to_string())?;
 if !data.starts_with(&root){return Err("Use isolated XDG_DATA_HOME".into());}
 let method=std::env::var("VIDEXTRACT_TRANSLATION_METHOD").unwrap_or("offline".into());
 std::fs::create_dir_all(data.join("transcripts")).map_err(|e|e.to_string())?;
 if std::env::var("VIDEXTRACT_TEST_PACK_DOWNLOAD").as_deref()==Ok("1"){translation::install_translation_model(handle.clone(),handle.state::<AppState>(),"en".into(),"fr".into(),"pack-download".into(),Channel::new(|_|Ok(()))).await?;let status=serde_json::to_value(translation::translation_status(handle.clone(),"en".into(),"fr".into())?).map_err(|e|e.to_string())?;if status["offlineInstalled"]!=true{return Err("Pack not ready after download".into());}}
 let id=uuid::Uuid::new_v4().to_string();
 let mut original=serde_json::json!({"id":id,"mediaPath":"/tmp/video.mp4","mediaTitle":"video.mp4","createdAt":1,"language":"en","segments":[{"start":0.5,"end":2.0,"text":"Hello! Welcome to our school."},{"start":2.0,"end":5.0,"text":"You can save the subtitles and watch the video offline."}]});
 if std::env::var("VIDEXTRACT_TRANSLATION_SOURCE").as_deref()==Ok("tr") {original["language"]="tr".into();original["segments"][0]["text"]="Merhaba! Okulumuza hoş geldiniz.".into();original["segments"][1]["text"]="Altyazıları kaydedebilir ve videoyu çevrimdışı izleyebilirsiniz.".into();}
 if std::env::var("VIDEXTRACT_TEST_LIVE_API").as_deref()==Ok("1"){original["segments"][0]["text"]="Good morning! Thank you for joining today's lesson.".into();original["segments"][1]["text"]="You can read the translated text next to the original.".into();}
 let source=serde_json::to_vec(&original).map_err(|e|e.to_string())?;let source_path=data.join("transcripts").join(format!("{id}.json"));std::fs::write(&source_path,&source).map_err(|e|e.to_string())?;
 let mut rows=vec![];
 let targets=std::env::var("VIDEXTRACT_TRANSLATION_TARGETS").unwrap_or("tr,de,es,fr,pt,ru,ar,hi,zh".into());
 for target in targets.split(','){
 let start=Instant::now();let output=translation::translate_transcript(handle.clone(),handle.state::<AppState>(),id.clone(),target.into(),format!("translate-{target}"),Channel::new(|_|Ok(())),Some(method.clone())).await?;
 let elapsed=start.elapsed().as_secs_f64();
 if output.segments.len()!=2||output.segments[0].start!=0.5||output.segments[1].end!=5.0||output.segments.iter().any(|s|s.text.trim().is_empty()){return Err("Translation lost segments or timestamps".into());}
 let start=Instant::now();let cached=translation::translate_transcript(handle.clone(),handle.state::<AppState>(),id.clone(),target.into(),format!("cached-{target}"),Channel::new(|_|Ok(())),Some(method.clone())).await?;
 if cached.segments[0].text!=output.segments[0].text||start.elapsed().as_secs_f64()>1.0{return Err("Cache did not return immediately".into());}
 let export=root.join(format!("translated-{target}.srt"));translation::export_translation(handle.clone(),id.clone(),target.into(),"srt".into(),export.to_string_lossy().into(),Some(method.clone())).await?;
 let text=std::fs::read_to_string(export).map_err(|e|e.to_string())?;if !text.contains("00:00:00,500 --> 00:00:02,000")||!text.contains(&output.segments[0].text){return Err("Translated subtitle export incorrect".into());}
 rows.push(serde_json::json!({"language":target,"seconds":elapsed,"segments":output.segments}));
 }
 if std::fs::read(&source_path).map_err(|e|e.to_string())?!=source{return Err("Original transcript changed".into());}
 let cancel_id=uuid::Uuid::new_v4().to_string();let mut cancel_source=original.clone();cancel_source["id"]=cancel_id.clone().into();cancel_source["segments"][0]["text"]=format!("Please cancel this translation test {cancel_id}.").into();std::fs::write(data.join("transcripts").join(format!("{cancel_id}.json")),serde_json::to_vec(&cancel_source).unwrap()).map_err(|e|e.to_string())?;
 let cancel_target=targets.split(',').find(|target|original["language"]!=*target).unwrap_or("tr").to_string();
 let cancel_handle=handle.clone();tauri::async_runtime::spawn(async move{tokio::time::sleep(std::time::Duration::from_millis(10)).await;let _=vidextract_lib::cancel_download(cancel_handle.state::<AppState>(),"translation-cancel".into());});
 let canceled=translation::translate_transcript(handle.clone(),handle.state::<AppState>(),cancel_id.clone(),cancel_target.clone(),"translation-cancel".into(),Channel::new(|_|Ok(())),Some(method.clone())).await;
 if !canceled.is_err_and(|e|e.contains("İptal"))||translation::cached_translation(handle.clone(),cancel_id,cancel_target,Some(method.clone())).await?.is_some(){return Err("Cancellation wrote incomplete cache".into());}
 Ok(serde_json::json!({"passed":true,"runs":rows,"originalPreserved":true,"cacheAndExports":true,"cancellation":true}))
 }.await;let passed=result.is_ok();let value=result.unwrap_or_else(|e|serde_json::json!({"passed":false,"error":e}));let _=std::fs::write(root.join("translation-result.json"),value.to_string());handle.exit(if passed{0}else{1});});Ok(())}).run(tauri::generate_context!()).expect("Translation smoke application");
}
