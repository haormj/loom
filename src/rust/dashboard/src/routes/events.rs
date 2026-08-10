use axum::body::Body;
use axum::response::Response;

pub async fn sse_handler() -> Response {
    Response::builder()
        .header("content-type", "text/event-stream")
        .body(Body::from("event: heartbeat\ndata: {}\n\n"))
        .unwrap()
}
