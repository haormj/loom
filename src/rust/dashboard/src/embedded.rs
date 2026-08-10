use axum::{
    body::Body,
    http::{header, StatusCode, Uri},
    response::Response,
};

pub const INDEX_HTML: &str = "<!DOCTYPE html><html><body>Frontend not built</body></html>";

#[cfg(not(feature = "dev-no-embed"))]
#[derive(rust_embed::RustEmbed)]
#[folder = "embedded/dist/"]
struct DashboardAssets;

#[cfg(not(feature = "dev-no-embed"))]
pub fn serve_asset(uri: &Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let file_path = if path.is_empty() { "index.html" } else { path };

    match DashboardAssets::get(file_path) {
        Some(asset) => {
            let mime = mime_type(file_path);
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, mime)
                .body(Body::from(asset.data.into_owned()))
                .unwrap()
        }
        None => {
            // SPA fallback: serve index.html for client-side routing
            match DashboardAssets::get("index.html") {
                Some(index) => Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
                    .body(Body::from(index.data.into_owned()))
                    .unwrap(),
                None => Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Body::from("frontend not built"))
                    .unwrap(),
            }
        }
    }
}

#[cfg(feature = "dev-no-embed")]
pub fn serve_asset(_uri: &Uri) -> Response {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
        .body(Body::from(INDEX_HTML))
        .unwrap()
}

#[cfg(not(feature = "dev-no-embed"))]
fn mime_type(name: &str) -> &'static str {
    if name.ends_with(".html") {
        "text/html; charset=utf-8"
    } else if name.ends_with(".js") {
        "application/javascript; charset=utf-8"
    } else if name.ends_with(".css") {
        "text/css"
    } else if name.ends_with(".svg") {
        "image/svg+xml"
    } else if name.ends_with(".png") {
        "image/png"
    } else {
        "application/octet-stream"
    }
}
