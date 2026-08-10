use std::path::Path;

use delivery_core::ProjectStatus;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ProjectConfig {
    #[serde(rename = "schemaVersion")]
    pub schema_version: u32,
    #[serde(rename = "projectId")]
    pub project_id: String,
}

#[derive(Debug, Clone)]
pub struct ProjectSnapshot {
    pub initialized: bool,
    pub status: Option<ProjectStatus>,
    pub config: Option<ProjectConfig>,
}

pub fn read_project(project_root: &Path) -> ProjectSnapshot {
    let paths = state::paths::project_paths(&project_root.display().to_string());
    let paths = match paths {
        Ok(p) => p,
        Err(_) => {
            return ProjectSnapshot {
                initialized: false,
                status: None,
                config: None,
            }
        }
    };
    if !paths.loom_dir.exists() {
        return ProjectSnapshot {
            initialized: false,
            status: None,
            config: None,
        };
    }
    let status = read_json_file::<ProjectStatus>(&paths.status_file);
    let config = read_json_file::<ProjectConfig>(&paths.config_file);
    ProjectSnapshot {
        initialized: true,
        status,
        config,
    }
}

fn read_json_file<T: for<'de> serde::Deserialize<'de>>(path: &Path) -> Option<T> {
    let data = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}
