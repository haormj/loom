use std::collections::BTreeMap;
use std::path::Path;

use delivery_core::{DeliveryIndex, DeliveryLifecycleStatus};

use state::paths::delivery_dir;

#[derive(Debug, Clone)]
pub struct PhaseSummary {
    pub phase_id: String,
    pub latest_refs: BTreeMap<String, String>,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct DeliverySummary {
    pub delivery_id: String,
    pub active_phase_id: String,
    pub status: String,
    pub phases: Vec<PhaseSummary>,
    pub updated_at: String,
}

fn lifecycle_status_to_string(status: &DeliveryLifecycleStatus) -> String {
    serde_json::to_string(status)
        .ok()
        .map(|raw| raw.trim_matches('"').to_string())
        .unwrap_or_default()
}

impl From<DeliveryIndex> for DeliverySummary {
    fn from(idx: DeliveryIndex) -> Self {
        let status_str = lifecycle_status_to_string(&idx.status);
        let phases: Vec<PhaseSummary> = idx
            .phases
            .iter()
            .map(|p| PhaseSummary {
                phase_id: p.phase_id.clone(),
                latest_refs: p.latest_refs.clone(),
                status: status_str.clone(),
            })
            .collect();
        DeliverySummary {
            delivery_id: idx.delivery_id,
            active_phase_id: idx.active_phase_id,
            status: status_str,
            phases,
            updated_at: idx.updated_at,
        }
    }
}

pub fn list_deliveries(project_root: &Path) -> Vec<DeliverySummary> {
    let deliveries_dir = project_root.join(".loom").join("deliveries");
    let mut entries = match std::fs::read_dir(&deliveries_dir) {
        Ok(e) => e.flatten().collect::<Vec<_>>(),
        Err(_) => return vec![],
    };
    entries.sort_by_key(|e| e.path());
    entries
        .iter()
        .filter_map(|entry| {
            if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                return None;
            }
            let dir_name = entry.file_name().to_string_lossy().to_string();
            read_delivery_index(project_root, &dir_name)
        })
        .collect()
}

pub fn read_delivery_index(project_root: &Path, delivery_id: &str) -> Option<DeliverySummary> {
    let index_path = delivery_dir(project_root, delivery_id).join("index.json");
    let data = std::fs::read_to_string(&index_path).ok()?;
    let idx: DeliveryIndex = serde_json::from_str(&data).ok()?;
    Some(idx.into())
}
