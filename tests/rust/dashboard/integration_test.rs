use std::path::PathBuf;

use dashboard::build_router;

fn full_fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../tests/rust/dashboard/fixtures/full_delivery")
}

async fn get_json(router: axum::Router, path: &str) -> serde_json::Value {
    use axum::body::Body;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    let response = router
        .oneshot(
            axum::http::Request::builder()
                .uri(path)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null)
}

#[tokio::test]
async fn full_delivery_project_status() {
    let router = build_router(full_fixture_root().display().to_string());
    let value = get_json(router, "/api/project/status").await;
    assert_eq!(value["initialized"], true);
    assert_eq!(value["activeDeliveryId"], "del_full");
}

#[tokio::test]
async fn full_delivery_list() {
    let router = build_router(full_fixture_root().display().to_string());
    let value = get_json(router, "/api/deliveries").await;
    assert_eq!(value.as_array().unwrap().len(), 1);
    assert_eq!(value[0]["deliveryId"], "del_full");
}

#[tokio::test]
async fn full_delivery_tasks() {
    let router = build_router(full_fixture_root().display().to_string());
    let value = get_json(router, "/api/deliveries/del_full/phases/ph_01/tasks").await;
    assert_eq!(value["runId"], "run_001");
    assert_eq!(value["summary"]["total"], 3);
    assert_eq!(value["summary"]["completed"], 1);
    assert_eq!(value["summary"]["running"], 1);
}

#[tokio::test]
async fn full_delivery_deploy() {
    let router = build_router(full_fixture_root().display().to_string());
    let value = get_json(router, "/api/deploy/status").await;
    assert_eq!(value["prepared"], true);
    assert!(value["logTail"].as_array().unwrap().len() > 0);
}

#[tokio::test]
async fn index_html_served() {
    use axum::body::Body;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    let router = build_router(full_fixture_root().display().to_string());
    let response = router
        .oneshot(
            axum::http::Request::builder()
                .uri("/")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let html = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(html.contains("root") || html.contains("Loom Dashboard") || html.contains("html"));
}
