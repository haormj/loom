use serde::Serialize;

use knowledge::paths::{chunks_dir, registry_file};

#[derive(Debug, Clone, Serialize)]
pub struct KnowledgeSourceSummary {
    pub source_id: String,
    pub name: String,
    pub enabled: bool,
    pub document_count: usize,
    pub current_build_id: Option<String>,
}

pub fn list_knowledge_sources() -> Vec<KnowledgeSourceSummary> {
    let registry_path = match registry_file() {
        Ok(p) => p,
        Err(_) => return vec![],
    };
    let data = match std::fs::read_to_string(&registry_path) {
        Ok(d) => d,
        Err(_) => return vec![],
    };
    let registry: serde_json::Value = match serde_json::from_str(&data) {
        Ok(v) => v,
        Err(_) => return vec![],
    };
    let empty_sources: Vec<serde_json::Value> = vec![];
    let sources = registry
        .get("sources")
        .and_then(|s| s.as_array())
        .unwrap_or(&empty_sources);
    sources
        .iter()
        .map(|src| {
            let source_id = src
                .get("sourceId")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let document_paths = src
                .get("documentPaths")
                .and_then(|v| v.as_array())
                .map(|a| a.len())
                .unwrap_or(0);
            KnowledgeSourceSummary {
                source_id: source_id.clone(),
                name: src
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                enabled: src.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true),
                document_count: document_paths,
                current_build_id: src
                    .get("currentBuildId")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
            }
        })
        .collect()
}

#[allow(dead_code)]
pub fn read_chunk_body(source_id: &str, build_id: &str, chunk_id: &str) -> Option<String> {
    let chunk_path = chunks_dir(source_id, build_id)
        .ok()?
        .join(format!("{chunk_id}.txt"));
    std::fs::read_to_string(&chunk_path).ok()
}
