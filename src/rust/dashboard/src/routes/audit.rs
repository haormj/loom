use std::path::PathBuf;

use axum::{extract::State, Json};
use serde::Serialize;

use crate::reader::{read_audit_records, read_field_audit_records, AuditRecord};
use crate::routes::AppState;

#[derive(Serialize)]
pub struct AuditResponse {
    pub request_size_records: Vec<AuditRecord>,
    pub field_read_records: Vec<AuditRecord>,
}

pub async fn records(State(state): State<AppState>) -> Json<AuditResponse> {
    let root = PathBuf::from(state.project_root.as_str());
    Json(AuditResponse {
        request_size_records: read_audit_records(&root, 50),
        field_read_records: read_field_audit_records(&root, 50),
    })
}
