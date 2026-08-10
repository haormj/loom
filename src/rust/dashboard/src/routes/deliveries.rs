use std::path::PathBuf;

use axum::{
    extract::{Path, State},
    Json,
};
use serde::Serialize;

use crate::reader::{list_deliveries, read_delivery_index, DeliverySummary};
use crate::routes::AppState;

pub async fn list(State(state): State<AppState>) -> Json<Vec<DeliverySummary>> {
    let root = PathBuf::from(state.project_root.as_str());
    Json(list_deliveries(&root))
}

pub async fn detail(
    State(state): State<AppState>,
    Path(delivery_id): Path<String>,
) -> Json<Option<DeliverySummary>> {
    let root = PathBuf::from(state.project_root.as_str());
    Json(read_delivery_index(&root, &delivery_id))
}
