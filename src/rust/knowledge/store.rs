use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use chrono::{DateTime, Local, TimeZone, Utc};
use log::{debug, info};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{
    models::{
        KnowledgeProviderConfig, KnowledgeRegistry, KnowledgeSource, OpenVikingProviderConfig,
        PendingQueue,
    },
    paths,
};

#[derive(Debug, Clone)]
pub struct PendingQueueRecord {
    pub file: PathBuf,
    pub queue: PendingQueue,
}

pub type KnowledgeResult<T> = Result<T, KnowledgeError>;

#[derive(Debug, Error)]
pub enum KnowledgeError {
    #[error("{0}")]
    Invalid(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("ZIP error: {0}")]
    Zip(#[from] zip::result::ZipError),
}

impl KnowledgeError {
    pub fn invalid(message: impl Into<String>) -> Self {
        Self::Invalid(message.into())
    }
}

pub fn ensure_dir(path: &Path) -> KnowledgeResult<()> {
    if path.exists() {
        if !path.is_dir() {
            return Err(KnowledgeError::invalid(format!(
                "path exists but is not a directory: {}",
                path.display()
            )));
        }
        return Ok(());
    }
    fs::create_dir_all(path)?;
    Ok(())
}

pub fn read_json<T: DeserializeOwned>(path: &Path) -> KnowledgeResult<T> {
    let raw = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&raw)?)
}

pub fn read_json_or<T: DeserializeOwned>(path: &Path, fallback: T) -> KnowledgeResult<T> {
    if !path.exists() {
        return Ok(fallback);
    }
    read_json(path)
}

pub fn write_json(path: &Path, value: &impl Serialize) -> KnowledgeResult<()> {
    let text = format!("{}\n", serde_json::to_string_pretty(value)?);
    write_text(path, &text)
}

pub fn write_text(path: &Path, text: &str) -> KnowledgeResult<()> {
    if let Some(parent) = path.parent() {
        ensure_dir(parent)?;
    }
    let tmp = tmp_path(path);
    {
        let mut file = fs::File::create(&tmp)?;
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
    }
    fs::rename(tmp, path)?;
    Ok(())
}

pub fn remove_file_if_exists(path: &Path) -> KnowledgeResult<()> {
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

pub fn remove_dir_if_exists(path: &Path) -> KnowledgeResult<()> {
    if path.exists() {
        fs::remove_dir_all(path)?;
    }
    Ok(())
}

pub fn load_registry() -> KnowledgeResult<KnowledgeRegistry> {
    read_json_or(&paths::registry_file()?, KnowledgeRegistry::empty())
}

pub fn load_merged_registry() -> KnowledgeResult<KnowledgeRegistry> {
    let mut registry = load_registry()?;
    let yaml_sources = load_providers_yaml()?;
    if yaml_sources.is_empty() {
        return Ok(registry);
    }
    info!(
        "load_merged_registry: merging {} providers.yaml sources into {} registry sources",
        yaml_sources.len(),
        registry.sources.len()
    );
    let now = now_string();
    for entry in yaml_sources {
        let yaml_source = yaml_source_to_knowledge_source(entry, &now);
        match registry.sources.iter_mut().find(|existing| {
            existing.source_id == yaml_source.source_id || existing.name == yaml_source.name
        }) {
            Some(existing) => {
                debug!(
                    "load_merged_registry: updating provider for existing source '{}'",
                    existing.name
                );
                existing.provider = yaml_source.provider;
            }
            None => {
                debug!(
                    "load_merged_registry: adding new OpenViking source '{}'",
                    yaml_source.name
                );
                registry.sources.push(yaml_source);
            }
        }
    }
    registry
        .sources
        .sort_by(|left, right| left.name.cmp(&right.name));
    Ok(registry)
}

fn load_providers_yaml() -> KnowledgeResult<Vec<ProvidersYamlSource>> {
    let path = paths::providers_yaml_file()?;
    if !path.exists() {
        debug!("load_providers_yaml: {} not found", path.display());
        return Ok(vec![]);
    }
    debug!("load_providers_yaml: loading from {}", path.display());
    let raw = fs::read_to_string(&path)?;
    let file: ProvidersYamlFile = serde_yaml::from_str(&raw)?;
    debug!("load_providers_yaml: {} sources found", file.sources.len());
    Ok(file.sources)
}

#[derive(Debug, Deserialize)]
struct ProvidersYamlFile {
    #[serde(default)]
    sources: Vec<ProvidersYamlSource>,
}

#[derive(Debug, Deserialize)]
struct ProvidersYamlSource {
    name: String,
    #[serde(default)]
    enabled: Option<bool>,
    endpoint: String,
    #[serde(default)]
    api_key_env: Option<String>,
    #[serde(default)]
    account: Option<String>,
    #[serde(default)]
    user: Option<String>,
    #[serde(default)]
    target_uri: Option<String>,
    #[serde(default)]
    timeout_secs: Option<u64>,
}

fn yaml_source_to_knowledge_source(entry: ProvidersYamlSource, now: &str) -> KnowledgeSource {
    let source_id = provider_source_id(&entry.name);
    KnowledgeSource {
        source_id,
        name: entry.name,
        enabled: entry.enabled.unwrap_or(true),
        document_paths: vec![],
        current_build_id: Some("openviking".to_string()),
        created_at: now.to_string(),
        updated_at: now.to_string(),
        last_built_at: None,
        provider: KnowledgeProviderConfig::OpenViking(OpenVikingProviderConfig {
            endpoint: entry.endpoint,
            api_key_env: entry.api_key_env,
            account: entry.account,
            user: entry.user,
            target_uri: entry
                .target_uri
                .unwrap_or_else(|| "viking://resources/".to_string()),
            timeout_secs: entry.timeout_secs,
        }),
    }
}

fn provider_source_id(name: &str) -> String {
    let safe = name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    let mut hasher = Sha256::new();
    hasher.update(name.as_bytes());
    let digest = format!("{:x}", hasher.finalize());
    format!("ksrc_ov_{safe}_{}", &digest[..8])
}

pub fn save_registry(registry: &KnowledgeRegistry) -> KnowledgeResult<()> {
    write_json(&paths::registry_file()?, registry)
}

pub fn load_pending(source_id: &str, source_name: &str) -> KnowledgeResult<PendingQueue> {
    read_json_or(
        &paths::pending_file(source_id)?,
        PendingQueue::empty(source_id, source_name),
    )
}

pub fn save_pending(queue: &PendingQueue) -> KnowledgeResult<()> {
    write_json(&paths::pending_file(&queue.source_id)?, queue)
}

pub fn list_pending_records() -> KnowledgeResult<Vec<PendingQueueRecord>> {
    let pending_dir = paths::pending_dir()?;
    if !pending_dir.exists() {
        return Ok(vec![]);
    }
    let mut records = Vec::new();
    for entry in fs::read_dir(pending_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        if entry.path().extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        records.push(PendingQueueRecord {
            file: entry.path(),
            queue: read_json(&entry.path())?,
        });
    }
    records.sort_by(|left, right| {
        left.queue
            .source_name
            .cmp(&right.queue.source_name)
            .then_with(|| left.queue.source_id.cmp(&right.queue.source_id))
    });
    Ok(records)
}

pub fn load_pending_by_name(name: &str) -> KnowledgeResult<Option<PendingQueue>> {
    Ok(list_pending_records()?
        .into_iter()
        .find(|record| record.queue.source_name == name)
        .map(|record| record.queue))
}

pub fn remove_pending_by_name(name: &str) -> KnowledgeResult<bool> {
    let mut removed = false;
    for record in list_pending_records()? {
        if record.queue.source_name == name {
            remove_file_if_exists(&record.file)?;
            removed = true;
        }
    }
    Ok(removed)
}

pub fn now_millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

pub fn now_string() -> String {
    Utc::now().to_rfc3339()
}

pub fn local_time(value: &str) -> String {
    DateTime::parse_from_rfc3339(value)
        .map(|time| {
            time.with_timezone(&Local)
                .format("%Y-%m-%d %H:%M:%S %Z")
                .to_string()
        })
        .unwrap_or_else(|_| value.to_string())
}

pub fn local_time_optional(value: &Option<String>) -> Option<String> {
    value.as_deref().map(local_time)
}

pub fn local_time_zone() -> String {
    let now = Local
        .timestamp_opt(Utc::now().timestamp(), 0)
        .single()
        .unwrap_or_else(Local::now);
    now.format("%Z").to_string()
}

pub fn canonical_path(path: &Path) -> KnowledgeResult<PathBuf> {
    path.canonicalize().map_err(|error| {
        KnowledgeError::invalid(format!("invalid path {}: {error}", path.display()))
    })
}

fn tmp_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("knowledge");
    path.with_file_name(format!("{file_name}.tmp-{}", now_millis()))
}
