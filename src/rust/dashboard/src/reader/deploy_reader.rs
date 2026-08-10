use std::path::Path;

use deploy::paths::deployment_paths;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct DeploySnapshot {
    pub prepared: bool,
    pub state: Option<serde_json::Value>,
    pub spec: Option<serde_json::Value>,
    pub repair_action: Option<serde_json::Value>,
    pub failure: Option<serde_json::Value>,
    pub log_tail: Vec<String>,
    pub log_ref: Option<String>,
}

const LOG_TAIL_LINES: usize = 500;

pub fn read_deploy_state(project_root: &Path) -> DeploySnapshot {
    let paths = deployment_paths(project_root);
    let state = read_json_value(&paths.state_file);
    let spec = read_json_value(&paths.spec_file);
    let repair_action = read_json_value(&paths.repair_action_file);
    let failure = read_json_value(&paths.failure_file);
    let log_tail = read_log_tail(&paths.log_file, LOG_TAIL_LINES);
    let log_ref = if paths.log_file.exists() {
        Some(
            paths
                .log_file
                .strip_prefix(project_root)
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| paths.log_file.display().to_string()),
        )
    } else {
        None
    };
    DeploySnapshot {
        prepared: spec.is_some(),
        state,
        spec,
        repair_action,
        failure,
        log_tail,
        log_ref,
    }
}

fn read_json_value(path: &Path) -> Option<serde_json::Value> {
    let data = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}

fn read_log_tail(path: &Path, max_lines: usize) -> Vec<String> {
    let data = match std::fs::read_to_string(path) {
        Ok(d) => d,
        Err(_) => return vec![],
    };
    let lines: Vec<&str> = data.lines().collect();
    let start = if lines.len() > max_lines {
        lines.len() - max_lines
    } else {
        0
    };
    lines[start..].iter().map(|s| s.to_string()).collect()
}
