# Loom Knowledge Module — OpenViking Provider Design

## Background

The loom knowledge module (`src/rust/knowledge/`) currently only supports local
file-system knowledge bases. Documents must be local files, indexed through the
built-in build pipeline (parse → chunk → BM25/TF-IDF → LLM semantic enrichment).

This design adds **OpenViking** as a built-in knowledge provider so enterprises
can connect an external OpenViking context database without running the local
build pipeline.

## Current Architecture

| Module | Responsibility |
|--------|----------------|
| `models.rs` | Data models: Registry, Source, Chunk, LexicalIndex, SemanticState |
| `paths.rs` | Storage paths hardcoded to `~/.loom/knowledge/` |
| `store.rs` | JSON file I/O, registry/pending management |
| `operations.rs` | MCP tool operations: add/update/remove/enable/disable/list/status |
| `builder.rs` | Document discovery → parse → chunk → lexical index → semantic state |
| `search.rs` | BM25 (via Python worker) + semantic matching + block affinity scoring |
| `semantic.rs` | Semantic pack submit → validate → publish |
| `inspect.rs` | Read chunk body from local file |

**Coupling points**: storage is local-only; the build pipeline is hardcoded;
`search_cards()` reads local chunks.json and calls the Python worker directly;
no abstraction layer separates storage/retrieval from implementation.

## Design

### KnowledgeProvider Trait

A new `provider.rs` module defines the abstraction:

```rust
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
```

### LocalKnowledgeProvider

Wraps the existing search/inspect logic for local file-system sources. No
behavior change — logic moves from free functions to trait implementation.

### OpenVikingProvider

Connects to an OpenViking HTTP server (default port 1933) and maps its API
to loom's `KnowledgeChunkCard` / `KnowledgeInspectChunkResult` models.

| loom operation | OpenViking API |
|----------------|----------------|
| `search` | `POST /api/v1/search/find` |
| `inspect_chunk` | `GET /api/v1/content/read?uri={uri}` |

Authentication and multi-tenancy headers:

| Config field | HTTP header |
|--------------|-------------|
| `api_key` | `Authorization: Bearer {key}` |
| `account` | `X-OpenViking-Account` |
| `user` | `X-OpenViking-User` |

### Data Model Mapping

OpenViking `MatchedContext` → loom `KnowledgeChunkCard`:

| loom field | OpenViking source |
|------------|-------------------|
| `source_id` / `source_name` | provider config |
| `build_id` | fixed `"openviking"` |
| `chunk_id` | `MatchedContext.uri` (viking:// URI) |
| `document_title` | derived from URI path |
| `heading_path` | derived from URI path segments |
| `summary` | `MatchedContext.abstract` |
| `matched_labels` | empty (OpenViking has no loom-format labels) |
| `score` | `MatchedContext.score` |

### Registry Model Extension

`KnowledgeSource` gains a `provider` field (defaults to `Local` for backward
compatibility):

```rust
pub enum KnowledgeProviderConfig {
    Local,
    OpenViking(OpenVikingProviderConfig),
}

pub struct OpenVikingProviderConfig {
    pub endpoint: String,
    pub api_key_env: Option<String>,   // env var name for token
    pub account: Option<String>,       // X-OpenViking-Account
    pub user: Option<String>,          // X-OpenViking-User
    pub target_uri: String,            // default "viking://resources/"
    pub timeout_secs: Option<u64>,     // default 10
}
```

### Provider Factory

`create_provider(source) -> Box<dyn KnowledgeProvider>` dispatches based on
the provider config variant.

### Configuration File

OpenViking sources are registered via `~/.loom/knowledge/providers.yaml`:

```yaml
sources:
  - name: confluence-kb
    provider:
      type: openviking
      endpoint: http://confluence-openviking.internal:1933
      apiKeyEnv: CONFLUENCE_OV_KEY
      account: acme
      user: alice
      targetUri: viking://resources/confluence/
      timeoutSecs: 10
```

`load_registry()` merges `providers.yaml` sources into the registry (by name,
yaml does not override existing json sources).

### Search/Inspect Integration

`search_cards()` iterates all enabled sources via `create_provider()` and
aggregates results. Single provider failure (timeout, HTTP error) does not
block other sources.

`inspect_chunk()` dispatches to the source's provider.

### Existing MCP Tools

No new MCP tools. Existing tools auto-adapt:
- `knowledgeSearch` / `knowledgeBrainstormContext` / `knowledgeInspectChunk` — work for both local and OpenViking sources
- `knowledgeList` / `knowledgeStatus` / `knowledgeEnable` / `knowledgeDisable` / `knowledgeRemove` — work as-is
- `knowledgeBuild` / `knowledgeResume` / `knowledgeSemanticSubmitFile` — return error for OpenViking sources

### Fault Isolation

A single provider failure (network timeout, HTTP error) does not block the
overall search — other sources return results normally.

### Dependency

`ureq = { version = "2.12", features = ["json"] }` — lightweight synchronous
HTTP client, no tokio dependency.

## Files Changed

| File | Change |
|------|--------|
| `knowledge/provider.rs` | New — trait + LocalKnowledgeProvider + OpenVikingProvider + factory |
| `knowledge/models.rs` | Add provider field to KnowledgeSource; new config types |
| `knowledge/operations.rs` | Merge providers.yaml; build/resume error for openviking |
| `knowledge/search.rs` | search_cards via create_provider |
| `knowledge/inspect.rs` | inspect_chunk via create_provider |
| `knowledge/lib.rs` | Export new module |
| `knowledge/Cargo.toml` | Add ureq dependency |

## Implementation Phases

1. **Phase 1**: Define trait + LocalKnowledgeProvider refactor (pure refactor, existing tests pass)
2. **Phase 2**: OpenVikingProvider + providers.yaml config + search/inspect integration + tests
