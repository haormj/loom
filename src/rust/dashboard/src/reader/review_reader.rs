use std::path::Path;

use contracts::ReviewResult;
use state::paths::{from_project_relative, DeliveryPhaseLocator};

pub fn read_latest_review(
    project_root: &Path,
    delivery_id: &str,
    phase_id: &str,
) -> Option<ReviewResult> {
    let locator = DeliveryPhaseLocator {
        delivery_id: delivery_id.to_string(),
        phase_id: phase_id.to_string(),
    };
    let latest_path = execution::paths::review_latest_file(project_root, &locator);
    let envelope: serde_json::Value = read_json_file(&latest_path)?;
    let result_ref = envelope.get("reviewResultRef").and_then(|v| v.as_str())?;
    let result_path = from_project_relative(project_root, result_ref).ok()?;
    read_json_file::<ReviewResult>(&result_path)
}

fn read_json_file<T: for<'de> serde::Deserialize<'de>>(path: &Path) -> Option<T> {
    let data = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}
