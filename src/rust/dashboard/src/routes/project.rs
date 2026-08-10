use axum::{extract::State, Json};
use serde::Serialize;

use crate::reader::read_project;
use crate::routes::AppState;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusResponse {
    pub initialized: bool,
    pub active_delivery_id: Option<String>,
    pub deliveries: Vec<DeliveryEntry>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeliveryEntry {
    pub delivery_id: String,
    pub status: String,
    pub updated_at: String,
}

pub async fn status(State(state): State<AppState>) -> Json<StatusResponse> {
    let project_root = std::path::PathBuf::from(state.project_root.as_str());
    let snapshot = read_project(&project_root);
    let deliveries = snapshot
        .status
        .as_ref()
        .map(|s| {
            s.deliveries
                .iter()
                .map(|d| DeliveryEntry {
                    delivery_id: d.delivery_id.clone(),
                    status: serde_json::to_string(&d.status)
                        .unwrap_or_default()
                        .trim_matches('"')
                        .to_string(),
                    updated_at: d.updated_at.clone(),
                })
                .collect()
        })
        .unwrap_or_default();
    Json(StatusResponse {
        initialized: snapshot.initialized,
        active_delivery_id: snapshot
            .status
            .as_ref()
            .and_then(|s| s.active_delivery_id.clone()),
        deliveries,
    })
}
