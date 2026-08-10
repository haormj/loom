use std::path::PathBuf;

use axum::{
    extract::{Path, State},
    Json,
};
use contracts::TaskPlanRun;

use crate::reader::read_task_plan_run;
use crate::routes::{validate_path_segment, AppState};

pub async fn list(
    State(state): State<AppState>,
    Path((delivery_id, phase_id)): Path<(String, String)>,
) -> Json<Option<TaskPlanRun>> {
    if !validate_path_segment(&delivery_id) || !validate_path_segment(&phase_id) {
        return Json(None);
    }
    let root = PathBuf::from(state.project_root.as_str());
    Json(read_task_plan_run(&root, &delivery_id, &phase_id))
}
