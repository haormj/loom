use axum::{extract::State, Json};

use crate::reader::{list_knowledge_sources, KnowledgeSourceSummary};
use crate::routes::AppState;

pub async fn sources(State(_state): State<AppState>) -> Json<Vec<KnowledgeSourceSummary>> {
    Json(list_knowledge_sources())
}
