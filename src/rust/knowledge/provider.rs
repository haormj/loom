use std::time::Duration;

use log::{debug, info, warn};
use serde::Deserialize;

use crate::{
    mcp_models::{KnowledgeChunkCard, KnowledgeInspectChunkResult},
    models::{KnowledgeProviderConfig, KnowledgeSource, OpenVikingProviderConfig},
    store::{KnowledgeError, KnowledgeResult},
};

const OPENVIKING_BUILD_ID: &str = "openviking";
const DEFAULT_TIMEOUT_SECS: u64 = 10;
const DEFAULT_MIN_SCORE: f64 = 0.2;

pub trait KnowledgeProvider: Send + Sync {
    fn provider_type(&self) -> &str;
    fn search(
        &self,
        query: &str,
        semantic_focus: &[String],
        block: Option<&str>,
        limit: usize,
    ) -> KnowledgeResult<Vec<KnowledgeChunkCard>>;
    fn inspect_chunk(&self, chunk_id: &str) -> KnowledgeResult<KnowledgeInspectChunkResult>;
}

pub fn create_provider(source: &KnowledgeSource) -> KnowledgeResult<Box<dyn KnowledgeProvider>> {
    match &source.provider {
        KnowledgeProviderConfig::Local => {
            let build_id = source.current_build_id.as_deref().ok_or_else(|| {
                KnowledgeError::invalid(format!(
                    "local knowledge source '{}' has no build",
                    source.name
                ))
            })?;
            debug!(
                "create_provider: source '{}' -> Local (build_id={})",
                source.name, build_id
            );
            Ok(Box::new(LocalKnowledgeProvider {
                source_id: source.source_id.clone(),
                source_name: source.name.clone(),
                build_id: build_id.to_string(),
            }))
        }
        KnowledgeProviderConfig::OpenViking(config) => {
            let api_key = config
                .api_key_env
                .as_ref()
                .and_then(|env| std::env::var(env).ok())
                .filter(|value| !value.is_empty());
            match (&config.api_key_env, &api_key) {
                (Some(env_name), Some(_)) => debug!(
                    "create_provider: source '{}' -> OpenViking (endpoint={}, apiKeyEnv={} -> resolved)",
                    source.name, config.endpoint, env_name
                ),
                (Some(env_name), None) => warn!(
                    "create_provider: source '{}' -> OpenViking (endpoint={}, apiKeyEnv={} -> env var NOT SET, requests will be unauthenticated)",
                    source.name, config.endpoint, env_name
                ),
                (None, None) => debug!(
                    "create_provider: source '{}' -> OpenViking (endpoint={}, no apiKeyEnv configured)",
                    source.name, config.endpoint
                ),
                _ => unreachable!(),
            }
            Ok(Box::new(OpenVikingProvider::new(
                source.source_id.clone(),
                source.name.clone(),
                config.clone(),
                api_key,
            )))
        }
    }
}

pub fn is_local_provider(source: &KnowledgeSource) -> bool {
    matches!(source.provider, KnowledgeProviderConfig::Local)
}

pub struct LocalKnowledgeProvider {
    pub source_id: String,
    pub source_name: String,
    pub build_id: String,
}

impl KnowledgeProvider for LocalKnowledgeProvider {
    fn provider_type(&self) -> &str {
        "local"
    }

    fn search(
        &self,
        _query: &str,
        _semantic_focus: &[String],
        _block: Option<&str>,
        _limit: usize,
    ) -> KnowledgeResult<Vec<KnowledgeChunkCard>> {
        Err(KnowledgeError::invalid(
            "local provider search is handled inline by search_cards; this method should not be called",
        ))
    }

    fn inspect_chunk(&self, chunk_id: &str) -> KnowledgeResult<KnowledgeInspectChunkResult> {
        let chunk = crate::inspect::read_chunk_body(&self.source_id, &self.build_id, chunk_id)?;
        let chunks_file: crate::models::ChunksFile = crate::store::read_json(
            &crate::paths::chunks_file(&self.source_id, &self.build_id)?,
        )?;
        let meta = chunks_file
            .chunks
            .iter()
            .find(|c| c.chunk_id == chunk_id)
            .ok_or_else(|| {
                KnowledgeError::invalid(format!("knowledge chunk not found: {chunk_id}"))
            })?;
        Ok(KnowledgeInspectChunkResult {
            document_title: meta.document_title.clone(),
            heading_path: meta.heading_path.clone(),
            text: chunk,
        })
    }
}

pub struct OpenVikingProvider {
    source_id: String,
    source_name: String,
    endpoint: String,
    api_key: Option<String>,
    account: Option<String>,
    user: Option<String>,
    target_uri: String,
    timeout: Duration,
    min_score: f64,
}

impl OpenVikingProvider {
    pub fn new(
        source_id: String,
        source_name: String,
        config: OpenVikingProviderConfig,
        api_key: Option<String>,
    ) -> Self {
        let timeout = Duration::from_secs(config.timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS));
        let min_score = config.min_score.unwrap_or(DEFAULT_MIN_SCORE);
        Self {
            source_id,
            source_name,
            endpoint: config.endpoint.trim_end_matches('/').to_string(),
            api_key,
            account: config.account,
            user: config.user,
            target_uri: config.target_uri,
            timeout,
            min_score,
        }
    }

    fn build_request(&self, method: &str, path: &str) -> ureq::Request {
        let url = format!("{}{}", self.endpoint, path);
        let mut request = ureq::request(method, &url)
            .timeout(self.timeout)
            .set("Content-Type", "application/json");
        if let Some(key) = &self.api_key {
            request = request.set("Authorization", &format!("Bearer {key}"));
            request = request.set("X-API-Key", key);
        }
        if let Some(account) = &self.account {
            request = request.set("X-OpenViking-Account", account);
        }
        if let Some(user) = &self.user {
            request = request.set("X-OpenViking-User", user);
        }
        request
    }
}

impl KnowledgeProvider for OpenVikingProvider {
    fn provider_type(&self) -> &str {
        "openviking"
    }

    fn search(
        &self,
        query: &str,
        semantic_focus: &[String],
        _block: Option<&str>,
        limit: usize,
    ) -> KnowledgeResult<Vec<KnowledgeChunkCard>> {
        let full_query = if semantic_focus.is_empty() {
            query.to_string()
        } else {
            format!("{} {}", query, semantic_focus.join(" "))
        };
        let body = serde_json::json!({
            "query": full_query,
            "target_uri": self.target_uri,
            "limit": limit,
        });
        info!(
            "openviking[{}]: search query={:?} focus={} limit={} min_score={}",
            self.source_name,
            truncate_for_log(query, 100),
            semantic_focus.len(),
            limit,
            self.min_score
        );
        let response = self
            .build_request("POST", "/api/v1/search/find")
            .send_json(body)
            .map_err(|error| map_ureq_error(error, &self.source_name))?;
        let result: OpenVikingFindResponse = response
            .into_json()
            .map_err(|error| KnowledgeError::invalid(format!("invalid OpenViking response: {error}")))?;
        let resource_count = result.result.resources.len();
        let memory_count = result.result.memories.len();
        let skill_count = result.result.skills.len();
        let mut cards = Vec::new();
        for ctx in result.result.resources {
            cards.push(context_to_card(
                &ctx,
                &self.source_id,
                &self.source_name,
            ));
        }
        for ctx in result.result.memories {
            cards.push(context_to_card(
                &ctx,
                &self.source_id,
                &self.source_name,
            ));
        }
        for ctx in result.result.skills {
            cards.push(context_to_card(
                &ctx,
                &self.source_id,
                &self.source_name,
            ));
        }
        let before = cards.len();
        cards.retain(|card| card.score >= self.min_score);
        let filtered = before - cards.len();
        info!(
            "openviking[{}]: result {} returned (resources={}, memories={}, skills={}), {} filtered by min_score={}, {} kept",
            self.source_name,
            before,
            resource_count,
            memory_count,
            skill_count,
            filtered,
            self.min_score,
            cards.len()
        );
        Ok(cards)
    }

    fn inspect_chunk(&self, chunk_id: &str) -> KnowledgeResult<KnowledgeInspectChunkResult> {
        debug!(
            "openviking[{}]: inspect_chunk uri={}",
            self.source_name, chunk_id
        );
        let encoded = urlencoding::encode(chunk_id);
        let path = format!("/api/v1/content/read?uri={encoded}");
        let response = self
            .build_request("GET", &path)
            .call()
            .map_err(|error| map_ureq_error(error, &self.source_name))?;
        let result: OpenVikingReadResponse = response
            .into_json()
            .map_err(|error| KnowledgeError::invalid(format!("invalid OpenViking response: {error}")))?;
        let (document_title, heading_path) = parse_viking_uri(chunk_id);
        Ok(KnowledgeInspectChunkResult {
            document_title,
            heading_path,
            text: result.result,
        })
    }
}

#[derive(Deserialize)]
struct OpenVikingFindResponse {
    result: OpenVikingFindResult,
}

#[derive(Deserialize)]
struct OpenVikingFindResult {
    #[serde(default)]
    resources: Vec<OpenVikingMatchedContext>,
    #[serde(default)]
    memories: Vec<OpenVikingMatchedContext>,
    #[serde(default)]
    skills: Vec<OpenVikingMatchedContext>,
}

#[derive(Deserialize)]
struct OpenVikingMatchedContext {
    uri: String,
    #[serde(default)]
    abstract_text: Option<String>,
    #[serde(default, rename = "abstract")]
    abstract_field: Option<String>,
    #[allow(dead_code)]
    #[serde(default)]
    level: i32,
    #[serde(default)]
    score: f64,
}

#[derive(Deserialize)]
struct OpenVikingReadResponse {
    result: String,
}

fn context_to_card(
    ctx: &OpenVikingMatchedContext,
    source_id: &str,
    source_name: &str,
) -> KnowledgeChunkCard {
    let (document_title, heading_path) = parse_viking_uri(&ctx.uri);
    let summary = ctx
        .abstract_text
        .as_deref()
        .or(ctx.abstract_field.as_deref())
        .map(str::to_string);
    KnowledgeChunkCard {
        source_id: source_id.to_string(),
        source_name: source_name.to_string(),
        build_id: OPENVIKING_BUILD_ID.to_string(),
        chunk_id: ctx.uri.clone(),
        document_title,
        heading_path,
        summary,
        matched_labels: vec![],
        score: ctx.score,
        source_kind: crate::mcp_models::KnowledgeSourceKind::Provider,
    }
}

fn truncate_for_log(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let truncated: String = s.chars().take(max).collect();
    format!("{truncated}...")
}

fn parse_viking_uri(uri: &str) -> (String, Vec<String>) {
    let path = uri.strip_prefix("viking://").unwrap_or(uri);
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    let relevant: Vec<&str> = segments
        .iter()
        .skip_while(|s| matches!(**s, "resources" | "user" | "memories" | "skills"))
        .copied()
        .collect();
    let file_name = relevant
        .last()
        .map(|s| s.rsplit_once('.').map(|(name, _)| name).unwrap_or(s))
        .unwrap_or("document");
    let heading_path = relevant[..relevant.len().saturating_sub(1)]
        .iter()
        .map(|s| s.to_string())
        .collect();
    (file_name.to_string(), heading_path)
}

fn map_ureq_error(error: ureq::Error, source_name: &str) -> KnowledgeError {
    match error {
        ureq::Error::Status(code, response) => {
            let body = response.into_string().unwrap_or_default();
            warn!(
                "openviking[{}]: HTTP {} response body: {}",
                source_name, code, body
            );
            KnowledgeError::invalid(format!(
                "OpenViking source '{source_name}' returned HTTP {code}: {body}"
            ))
        }
        ureq::Error::Transport(transport) => {
            warn!(
                "openviking[{}]: transport error: {}",
                source_name, transport
            );
            KnowledgeError::invalid(format!(
                "OpenViking source '{source_name}' unreachable: {transport}"
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::OpenVikingProviderConfig;

    #[test]
    fn endpoint_trailing_slash_is_stripped() {
        let provider = OpenVikingProvider::new(
            "s1".to_string(),
            "test".to_string(),
            OpenVikingProviderConfig {
                endpoint: "http://127.0.0.1:1933/".to_string(),
                api_key_env: None,
                account: None,
                user: None,
                target_uri: "viking://resources/".to_string(),
                timeout_secs: None,
                min_score: None,
            },
            None,
        );
        assert_eq!(provider.endpoint, "http://127.0.0.1:1933");
    }

    #[test]
    fn endpoint_without_trailing_slash_is_unchanged() {
        let provider = OpenVikingProvider::new(
            "s1".to_string(),
            "test".to_string(),
            OpenVikingProviderConfig {
                endpoint: "http://127.0.0.1:1933".to_string(),
                api_key_env: None,
                account: None,
                user: None,
                target_uri: "viking://resources/".to_string(),
                timeout_secs: None,
                min_score: None,
            },
            None,
        );
        assert_eq!(provider.endpoint, "http://127.0.0.1:1933");
    }

    #[test]
    fn build_request_url_has_no_double_slash() {
        let provider = OpenVikingProvider::new(
            "s1".to_string(),
            "test".to_string(),
            OpenVikingProviderConfig {
                endpoint: "http://127.0.0.1:1933/".to_string(),
                api_key_env: None,
                account: None,
                user: None,
                target_uri: "viking://resources/".to_string(),
                timeout_secs: None,
                min_score: None,
            },
            None,
        );
        let url = format!("{}{}", provider.endpoint, "/api/v1/search/find");
        assert_eq!(url, "http://127.0.0.1:1933/api/v1/search/find");
        assert!(!url.contains("//api"), "URL should not contain double slash before path");
    }

    #[test]
    fn default_min_score_applied_when_not_configured() {
        let provider = OpenVikingProvider::new(
            "s1".to_string(),
            "test".to_string(),
            OpenVikingProviderConfig {
                endpoint: "http://127.0.0.1:1933".to_string(),
                api_key_env: None,
                account: None,
                user: None,
                target_uri: "viking://resources/".to_string(),
                timeout_secs: None,
                min_score: None,
            },
            None,
        );
        assert!((provider.min_score - DEFAULT_MIN_SCORE).abs() < f64::EPSILON);
    }

    #[test]
    fn configured_min_score_overrides_default() {
        let provider = OpenVikingProvider::new(
            "s1".to_string(),
            "test".to_string(),
            OpenVikingProviderConfig {
                endpoint: "http://127.0.0.1:1933".to_string(),
                api_key_env: None,
                account: None,
                user: None,
                target_uri: "viking://resources/".to_string(),
                timeout_secs: None,
                min_score: Some(0.5),
            },
            None,
        );
        assert!((provider.min_score - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn cards_below_min_score_are_filtered() {
        let cards = vec![
            context_to_card(
                &OpenVikingMatchedContext {
                    uri: "viking://resources/high.md".to_string(),
                    abstract_text: Some("high".to_string()),
                    abstract_field: None,
                    level: 0,
                    score: 0.9,
                },
                "s1",
                "test",
            ),
            context_to_card(
                &OpenVikingMatchedContext {
                    uri: "viking://resources/low.md".to_string(),
                    abstract_text: Some("low".to_string()),
                    abstract_field: None,
                    level: 0,
                    score: 0.1,
                },
                "s1",
                "test",
            ),
            context_to_card(
                &OpenVikingMatchedContext {
                    uri: "viking://resources/borderline.md".to_string(),
                    abstract_text: Some("borderline".to_string()),
                    abstract_field: None,
                    level: 0,
                    score: 0.2,
                },
                "s1",
                "test",
            ),
        ];
        let min_score = 0.2;
        let mut filtered = cards;
        filtered.retain(|card| card.score >= min_score);
        assert_eq!(filtered.len(), 2, "cards at or above threshold are kept");
        assert!(filtered.iter().all(|card| card.score >= min_score));
    }

    #[test]
    fn all_cards_filtered_when_all_below_min_score() {
        let cards = vec![context_to_card(
            &OpenVikingMatchedContext {
                uri: "viking://resources/low.md".to_string(),
                abstract_text: Some("low".to_string()),
                abstract_field: None,
                level: 0,
                score: 0.05,
            },
            "s1",
            "test",
        )];
        let min_score = 0.2;
        let mut filtered = cards;
        filtered.retain(|card| card.score >= min_score);
        assert!(filtered.is_empty(), "all-low results yield empty set");
    }
}
