//! WebKit/GStreamer cannot reliably seek asset:// media. Serve only explicitly opened files,
//! through unguessable URLs bound to loopback, using bounded streaming and HTTP byte ranges.
use axum::{
    body::Body,
    extract::{Path as TokenPath, Request, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use std::{
    collections::VecDeque,
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use tokio::{net::TcpListener, sync::OnceCell, task::JoinHandle};
use tower::ServiceExt;
use tower_http::services::ServeFile;
use uuid::Uuid;

#[derive(Clone)]
struct Registry {
    files: Arc<Mutex<VecDeque<(String, PathBuf)>>>,
    address: SocketAddr,
}
struct RunningServer {
    registry: Registry,
    task: JoinHandle<()>,
}
impl Drop for RunningServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}
#[derive(Default)]
pub struct MediaServer {
    running: OnceCell<RunningServer>,
}

impl MediaServer {
    pub async fn register(&self, path: &Path) -> Result<String, String> {
        let server = self
            .running
            .get_or_try_init(|| async {
                let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
                    .await
                    .map_err(|e| format!("Yerel oynatıcı başlatılamadı: {e}"))?;
                let address = listener.local_addr().map_err(|e| e.to_string())?;
                let registry = Registry {
                    files: Arc::new(Mutex::new(VecDeque::new())),
                    address,
                };
                let router = Router::new()
                    .route("/media/{token}", get(serve_media))
                    .with_state(registry.clone());
                let task = tokio::spawn(async move {
                    let _ = axum::serve(listener, router).await;
                });
                Ok::<_, String>(RunningServer { registry, task })
            })
            .await?;
        let mut files = server.registry.files.lock().map_err(|e| e.to_string())?;
        let token = if let Some(index) = files.iter().position(|(_, saved)| saved == path) {
            files.remove(index).ok_or("Medya kaydı bulunamadı")?.0
        } else {
            Uuid::new_v4().to_string()
        };
        files.push_back((token.clone(), path.to_path_buf()));
        while files.len() > 64 {
            files.pop_front();
        }
        Ok(format!("http://{}/media/{token}", server.registry.address))
    }
}
async fn serve_media(
    State(registry): State<Registry>,
    TokenPath(token): TokenPath<String>,
    request: Request,
) -> Response {
    let expected_host = registry.address.to_string();
    if request
        .headers()
        .get(header::HOST)
        .and_then(|s| s.to_str().ok())
        != Some(expected_host.as_str())
    {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let origin = request.headers().get(header::ORIGIN).cloned();
    if let Some(ref origin) = origin {
        let allowed = matches!(
            origin.to_str().unwrap_or(""),
            "tauri://localhost"
                | "http://tauri.localhost"
                | "https://tauri.localhost"
                | "http://localhost:1420"
                | "http://127.0.0.1:1420"
        );
        if !allowed {
            return StatusCode::FORBIDDEN.into_response();
        }
    }
    let path = match registry.files.lock() {
        Ok(files) => files
            .iter()
            .find(|(key, _)| key == &token)
            .map(|(_, path)| path.clone()),
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    let Some(path) = path else {
        return StatusCode::NOT_FOUND.into_response();
    };
    // ServeFile handles HEAD, content type, satisfiable/unsatisfiable ranges and streaming.
    let mut response = ServeFile::new(path)
        .oneshot(request)
        .await
        .unwrap_or_else(|never| match never {})
        .map(Body::new);
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, "private, no-store".parse().unwrap());
    response
        .headers_mut()
        .insert(header::X_CONTENT_TYPE_OPTIONS, "nosniff".parse().unwrap());
    if let Some(origin) = origin {
        response
            .headers_mut()
            .insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, origin);
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn opened_media_supports_head_and_ranges_without_exposing_other_paths() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Türkçe video.mp4");
        let bytes = (0..260_000).map(|n| (n % 251) as u8).collect::<Vec<_>>();
        tokio::fs::write(&path, &bytes).await.unwrap();
        let server = MediaServer::default();
        let url = server.register(&path).await.unwrap();
        assert_eq!(server.register(&path).await.unwrap(), url);
        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .header("Range", "bytes=100-199")
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 206);
        assert_eq!(response.headers()["content-range"], "bytes 100-199/260000");
        assert_eq!(&response.bytes().await.unwrap()[..], &bytes[100..200]);
        let head = client.head(&url).send().await.unwrap();
        assert_eq!(head.status(), 200);
        assert_eq!(head.headers()["content-length"], "260000");
        assert!(head.bytes().await.unwrap().is_empty());
        assert_eq!(
            client
                .get(&url)
                .header("Range", "bytes=999999-")
                .send()
                .await
                .unwrap()
                .status(),
            416
        );
        assert_eq!(
            client
                .get(&url)
                .header("Origin", "https://unrelated.example")
                .send()
                .await
                .unwrap()
                .status(),
            403
        );
        assert_eq!(
            client
                .get(&url)
                .header("Host", "unrelated.example")
                .send()
                .await
                .unwrap()
                .status(),
            400
        );
        let missing = format!(
            "{}/media/{}",
            url.split("/media/").next().unwrap(),
            Uuid::new_v4()
        );
        assert_eq!(client.get(missing).send().await.unwrap().status(), 404);
        let response = client
            .get(&url)
            .header("Origin", "tauri://localhost")
            .header("Range", "bytes=-10")
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 206);
        assert_eq!(
            &response.bytes().await.unwrap()[..],
            &bytes[bytes.len() - 10..]
        );
    }
}
