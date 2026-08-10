use axum::{
    body::Body,
    http::{header, StatusCode, Uri},
    response::Response,
};

pub fn serve_asset(uri: &Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let asset = if path.is_empty() || path == "index.html" {
        Some(("index.html", INDEX_HTML))
    } else {
        None
    };
    match asset {
        Some((name, content)) => {
            let mime = mime_type(name);
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, mime)
                .body(Body::from(content))
                .unwrap()
        }
        None => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("not found"))
            .unwrap(),
    }
}

fn mime_type(name: &str) -> &'static str {
    if name.ends_with(".html") {
        "text/html; charset=utf-8"
    } else if name.ends_with(".js") {
        "application/javascript; charset=utf-8"
    } else if name.ends_with(".css") {
        "text/css; charset=utf-8"
    } else {
        "application/octet-stream"
    }
}

const INDEX_HTML: &str = r#"<!DOCTYPE html>
<html>
<head><meta charset="utf-8"><title>Loom Dashboard</title></head>
<body>
<h1>Loom Dashboard</h1>
<p>Frontend not yet built. API is available at <code>/api/*</code>.</p>
</body>
</html>"#;
