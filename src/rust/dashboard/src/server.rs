use std::net::SocketAddr;
use std::path::PathBuf;

use axum::{routing::get, Router};

use crate::embedded;
use crate::routes::api_router;
use crate::DashboardError;

pub fn build_router(project_root: String) -> Router {
    let api = api_router(project_root);
    Router::new()
        .route(
            "/",
            get(|| async { axum::response::Html(embedded::INDEX_HTML) }),
        )
        .nest_service("/assets", axum::routing::any(asset_handler))
        .merge(api)
}

async fn asset_handler(uri: axum::http::Uri) -> axum::response::Response {
    embedded::serve_asset(&uri)
}

pub async fn serve(
    project_root: String,
    port: u16,
    open_browser: bool,
) -> Result<(), DashboardError> {
    let app = build_router(project_root.clone());
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
