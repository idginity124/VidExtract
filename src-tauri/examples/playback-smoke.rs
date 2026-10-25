//! Exercise the actual history -> studio -> <video> path in WebKitGTK, including in an AppDir.
//! Run with isolated XDG_DATA_HOME; reports are JSONL. Never touches real history or media contents.
use std::{io::Write, path::PathBuf};
use tauri::{webview::PageLoadEvent, Listener, Manager};

fn main() {
    let media = std::env::var("VIDEXTRACT_PLAYBACK_MEDIA").expect("Set VIDEXTRACT_PLAYBACK_MEDIA");
    let report =
        PathBuf::from(std::env::var("VIDEXTRACT_PLAYBACK_REPORT").expect("Set report path"));
    let root = report.parent().expect("report parent").to_path_buf();
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(&report, "").unwrap();
    let source = std::env::var("VIDEXTRACT_PLAYBACK_SCRIPT")
        .ok()
        .map(std::fs::read_to_string)
        .transpose()
        .expect("read test script")
        .unwrap_or_else(|| include_str!("../tests/playback-smoke.js").into());
    let script = source.replace(
        "__MEDIA_PATH_JSON__",
        &serde_json::to_string(&media).unwrap(),
    );
    let mut context = tauri::generate_context!();
    // This example embeds the production frontend even in dev builds.
    context.config_mut().build.dev_url = None;
    if std::env::var_os("VIDEXTRACT_PLAYBACK_HTTP_TEST").is_some() {
        context.config_mut().app.security.csp = None;
    }
    vidextract_lib::desktop_builder()
        .setup(move |app| {
            let data = app.path().app_data_dir()?;
            if !data.starts_with(&root) {
                return Err("Use XDG_DATA_HOME inside the report directory".into());
            }
            let handle = app.handle().clone();
            let report_copy = report.clone();
            app.listen("vidextract-playback-smoke", move |event| {
                if let Ok(mut file) = std::fs::OpenOptions::new().append(true).open(&report_copy) {
                    let _ = writeln!(file, "{}", event.payload());
                }
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(event.payload()) {
                    if value["phase"] == "passed" {
                        handle.exit(0);
                    }
                    if value["phase"] == "failed" {
                        handle.exit(1);
                    }
                }
            });
            let timeout_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(70)).await;
                if let Ok(mut file) = std::fs::OpenOptions::new().append(true).open(report) {
                    let _ = writeln!(
                        file,
                        "{{\"phase\":\"failed\",\"error\":\"WebView heartbeat/playback timeout\"}}"
                    );
                }
                timeout_handle.exit(2);
            });
            Ok(())
        })
        .on_page_load(move |webview, payload| {
            if matches!(payload.event(), PageLoadEvent::Finished) {
                let _ = webview.eval(&script);
            }
        })
        .run(context)
        .expect("Playback test failed to start");
}
