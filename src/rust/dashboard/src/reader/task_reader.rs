use std::path::Path;

use contracts::TaskPlanRun;
use state::paths::{from_project_relative, DeliveryPhaseLocator};

pub fn read_task_plan_run(
    project_root: &Path,
    delivery_id: &str,
    phase_id: &str,
) -> Option<TaskPlanRun> {
    let locator = DeliveryPhaseLocator {
        delivery_id: delivery_id.to_string(),
        phase_id: phase_id.to_string(),
    };
    let latest_path = execution::paths::task_plan_run_latest_file(project_root, &locator);
    let envelope: serde_json::Value = read_json_file(&latest_path)?;
    let run_ref = envelope.get("runRef").and_then(|v| v.as_str())?;
    let run_path = from_project_relative(project_root, run_ref).ok()?;
    read_json_file::<TaskPlanRun>(&run_path)
}

fn read_json_file<T: for<'de> serde::Deserialize<'de>>(path: &Path) -> Option<T> {
    let data = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}
