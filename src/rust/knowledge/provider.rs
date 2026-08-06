use std::time::Duration;

use serde::Deserialize;

use crate::{
    mcp_models::{KnowledgeChunkCard, KnowledgeInspectChunkResult},
    models::{KnowledgeProviderConfig, KnowledgeSource, OpenVikingProviderConfig},
    store::{KnowledgeError, KnowledgeResult},
};

const OPENVIKING_BUILD_ID: &str = "openviking";
const DEFAULT_TIMEOUT_SECS: u64 = 10;

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
}

impl OpenVikingProvider {
    pub fn new(
        source_id: String,
        source_name: String,
        config: OpenVikingProviderConfig,
        api_key: Option<String>,
    ) -> Self {
        let timeout = Duration::from_secs(config.timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS));
        Self {
            source_id,
            source_name,
            endpoint: config.endpoint,
            api_key,
            account: config.account,
            user: config.user,
            target_uri: config.target_uri,
            timeout,
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
        let response = self
            .build_request("POST", "/api/v1/search/find")
            .send_json(body)
            .map_err(|error| map_ureq_error(error, &self.source_name))?;
        let result: OpenVikingFindResponse = response
            .into_json()
            .map_err(|error| KnowledgeError::invalid(format!("invalid OpenViking response: {error}")))?;
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
        Ok(cards)
    }

    fn inspect_chunk(&self, chunk_id: &str) -> KnowledgeResult<KnowledgeInspectChunkResult> {
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
    }
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
            KnowledgeError::invalid(format!(
                "OpenViking source '{source_name}' returned HTTP {code}: {body}"
            ))
        }
        ureq::Error::Transport(transport) => KnowledgeError::invalid(format!(
            "OpenViking source '{source_name}' unreachable: {transport}"
        )),
    }
}
