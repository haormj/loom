use axum::body::Body;
use http_body_util::BodyExt;
use std::path::PathBuf;

use dashboard::build_router;

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../tests/rust/dashboard/fixtures/minimal_project")
}

fn router() -> axum::Router {
    build_router(fixture_root().display().to_string())
}

async fn get_json(router: axum::Router, path: &str) -> serde_json::Value {
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
async fn project_status_returns_initialized() {
    let value = get_json(router(), "/api/project/status").await;
    assert_eq!(value["initialized"], true);
    assert_eq!(value["activeDeliveryId"], "del_test");
}

#[tokio::test]
async fn deliveries_list_returns_entries() {
    let value = get_json(router(), "/api/deliveries").await;
    let arr = value.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["deliveryId"], "del_test");
}

#[tokio::test]
async fn delivery_detail_returns_phases() {
    let value = get_json(router(), "/api/deliveries/del_test").await;
    assert_eq!(value["deliveryId"], "del_test");
    assert_eq!(value["phases"][0]["phaseId"], "ph_01");
}

#[tokio::test]
async fn deploy_status_returns_not_prepared() {
    let value = get_json(router(), "/api/deploy/status").await;
    assert_eq!(value["prepared"], false);
}
