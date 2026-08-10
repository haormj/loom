use std::path::Path;

use serde::Serialize;
use state::paths::project_paths;

#[derive(Debug, Clone, Serialize)]
pub struct AuditRecord {
    pub line: serde_json::Value,
    pub raw: String,
}

pub fn read_audit_records(project_root: &Path, limit: usize) -> Vec<AuditRecord> {
    let paths = match project_paths(&project_root.display().to_string()) {
        Ok(p) => p,
        Err(_) => return vec![],
    };
    read_jsonl_tail(&paths.request_size_audit_file, limit)
}

pub fn read_field_audit_records(project_root: &Path, limit: usize) -> Vec<AuditRecord> {
    let paths = match project_paths(&project_root.display().to_string()) {
        Ok(p) => p,
        Err(_) => return vec![],
    };
    read_jsonl_tail(&paths.field_read_audit_file, limit)
}

fn read_jsonl_tail(path: &Path, limit: usize) -> Vec<AuditRecord> {
    let data = match std::fs::read_to_string(path) {
        Ok(d) => d,
        Err(_) => return vec![],
    };
    let lines: Vec<&str> = data.lines().filter(|l| !l.trim().is_empty()).collect();
    let start = if lines.len() > limit {
        lines.len() - limit
    } else {
        0
    };
    lines[start..]
        .iter()
        .filter_map(|line| {
            let parsed: serde_json::Value = serde_json::from_str(line).ok()?;
            Some(AuditRecord {
                line: parsed,
                raw: line.to_string(),
            })
        })
        .collect()
}
