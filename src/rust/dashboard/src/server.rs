use std::net::SocketAddr;
use std::path::PathBuf;

use axum::{routing::get, Router};
use tokio::sync::broadcast;

use crate::embedded;
use crate::routes::api_router;
use crate::DashboardError;

pub fn build_router(
    project_root: String,
    event_rx: Option<broadcast::Receiver<crate::watcher::DashboardEvent>>,
) -> Router {
    let api = api_router(project_root, event_rx);
    Router::new()
        .route("/", get(root_handler))
        .nest_service("/assets", axum::routing::any(asset_handler))
        .merge(api)
        .fallback(fallback_handler)
}

async fn root_handler() -> axum::response::Response {
    embedded::serve_asset(&"/".parse().unwrap())
}

async fn fallback_handler(uri: axum::http::Uri) -> axum::response::Response {
    embedded::serve_asset(&uri)
}

async fn asset_handler(uri: axum::http::Uri) -> axum::response::Response {
    embedded::serve_asset(&uri)
}

pub async fn serve(
    project_root: String,
    port: u16,
    open_browser: bool,
) -> Result<(), DashboardError> {
    let project_root_path = std::path::PathBuf::from(&project_root);
    let event_rx = crate::watcher::start_watcher(project_root_path);

    let app = build_router(project_root.clone(), Some(event_rx));
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| DashboardError(format!("failed to bind 127.0.0.1:{}: {}", port, e)))?;
    let actual_port = listener.local_addr().unwrap().port();
    log::info!(
        "Loom Dashboard serving on http://127.0.0.1:{} (project: {})",
        actual_port,
        project_root
    );
    if open_browser {
        let url = format!("http://127.0.0.1:{}", actual_port);
        let _ = open_url(&url);
    }
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|e| DashboardError(format!("server error: {}", e)))?;
    Ok(())
}

fn open_url(url: &str) -> Result<(), ()> {
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(url)
            .spawn()
            .map(|_| ())
            .map_err(|_| ())
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(url)
            .spawn()
            .map(|_| ())
            .map_err(|_| ())
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = url;
        Err(())
    }
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("install Ctrl-C handler");
    log::info!("Loom Dashboard shutting down...");
}
