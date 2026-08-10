use std::path::PathBuf;

use axum::{extract::State, Json};
use serde::Serialize;

use crate::reader::{read_deploy_state, DeploySnapshot};
use crate::routes::AppState;

#[derive(Serialize)]
pub struct DeployResponse {
    pub prepared: bool,
    pub state: Option<serde_json::Value>,
    pub log_tail: Vec<String>,
    pub log_ref: Option<String>,
    pub repair_action: Option<serde_json::Value>,
    pub failure: Option<serde_json::Value>,
}

pub async fn status(State(state): State<AppState>) -> Json<DeployResponse> {
    let root = PathBuf::from(state.project_root.as_str());
    let snap = read_deploy_state(&root);
    Json(DeployResponse {
        prepared: snap.prepared,
        state: snap.state,
        log_tail: snap.log_tail,
        log_ref: snap.log_ref,
        repair_action: snap.repair_action,
        failure: snap.failure,
    })
}

pub async fn logs(State(state): State<AppState>) -> Json<DeployResponse> {
    status(State(state)).await
}
