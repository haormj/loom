pub mod audit;
pub mod deliveries;
pub mod deploy;
pub mod events;
pub mod knowledge;
pub mod project;
pub mod reviews;
pub mod tasks;

use std::sync::Arc;

use axum::{routing::get, Router};
use tokio::sync::broadcast;

#[derive(Clone)]
pub struct AppState {
    pub project_root: Arc<String>,
    pub event_rx:
        Arc<tokio::sync::Mutex<Option<broadcast::Receiver<crate::watcher::DashboardEvent>>>>,
}

pub fn validate_path_segment(segment: &str) -> bool {
    !segment.is_empty()
        && !segment.contains("..")
        && !segment.contains('/')
        && !segment.contains('\\')
        && !segment.contains('\0')
}

pub fn api_router(
    project_root: String,
    event_rx: Option<broadcast::Receiver<crate::watcher::DashboardEvent>>,
) -> Router {
    let state = AppState {
        project_root: Arc::new(project_root),
        event_rx: Arc::new(tokio::sync::Mutex::new(event_rx)),
    };
    Router::new()
        .route("/api/project/status", get(project::status))
        .route("/api/deliveries", get(deliveries::list))
        .route("/api/deliveries/:delivery_id", get(deliveries::detail))
        .route(
            "/api/deliveries/:delivery_id/phases/:phase_id/tasks",
            get(tasks::list),
        )
        .route(
            "/api/deliveries/:delivery_id/phases/:phase_id/reviews",
            get(reviews::latest),
        )
        .route("/api/deploy/status", get(deploy::status))
        .route("/api/deploy/logs", get(deploy::logs))
        .route("/api/knowledge/sources", get(knowledge::sources))
        .route("/api/audit/records", get(audit::records))
        .route("/api/events", get(events::sse_handler))
        .with_state(state)
}
