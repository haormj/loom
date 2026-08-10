use std::path::PathBuf;

use axum::{
    extract::{Path, State},
    Json,
};
use contracts::ReviewResult;

use crate::reader::read_latest_review;
use crate::routes::AppState;

pub async fn latest(
    State(state): State<AppState>,
    Path((delivery_id, phase_id)): Path<(String, String)>,
) -> Json<Option<ReviewResult>> {
    let root = PathBuf::from(state.project_root.as_str());
    Json(read_latest_review(&root, &delivery_id, &phase_id))
}
