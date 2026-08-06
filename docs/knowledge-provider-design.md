# Loom 知识模块 — OpenViking 知识库 Provider 设计文档

## 背景

loom 知识模块（`src/rust/knowledge/`）此前仅支持本地文件系统知识库。文档必须是本地文件，通过内置的构建流水线（解析 → 分块 → BM25/TF-IDF → LLM 语义增强）进行索引。

本设计新增 **OpenViking** 作为内置知识库 provider，使企业能够接入外部 OpenViking 上下文数据库，无需运行本地构建流水线。

## 现有架构

| 模块 | 职责 |
|------|------|
| `models.rs` | 数据模型：Registry、Source、Chunk、LexicalIndex、SemanticState |
| `paths.rs` | 存储路径，固定为 `~/.loom/knowledge/` |
| `store.rs` | JSON 文件 I/O，registry/pending 管理 |
| `operations.rs` | MCP 工具操作：add/update/remove/enable/disable/list/status |
| `builder.rs` | 文档发现 → 解析 → 分块 → 词法索引 → 语义状态 |
| `search.rs` | BM25（通过 Python worker）+ 语义匹配 + block 亲和度评分 |
| `semantic.rs` | 语义包提交 → 校验 → 发布 |
| `inspect.rs` | 从本地文件读取 chunk 正文 |

**耦合点**：存储仅支持本地；构建流水线硬编码；`search_cards()` 直接读取本地 chunks.json 并调用 Python worker；没有抽象层将存储/检索与实现分离。

## 设计方案

### KnowledgeProvider Trait

新增 `provider.rs` 模块，定义抽象接口：

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

封装现有的本地文件系统搜索/检视逻辑。行为不变，仅将逻辑从自由函数迁移到 trait 实现。实际搜索仍由 `search_cards()` 内联处理（性能优化），`LocalKnowledgeProvider` 主要用于 `inspect_chunk` 的统一调度。

### OpenVikingProvider

连接 OpenViking HTTP 服务器（默认端口 1933），将其 API 映射到 loom 的 `KnowledgeChunkCard` / `KnowledgeInspectChunkResult` 模型。

| loom 操作 | OpenViking API |
|-----------|----------------|
| `search` | `POST /api/v1/search/find` |
| `inspect_chunk` | `GET /api/v1/content/read?uri={uri}` |

认证与多租户头：

| 配置字段 | HTTP 头 |
|---------|---------|
| `api_key_env`（环境变量名） | `Authorization: Bearer {key}` + `X-API-Key: {key}` |
| `account` | `X-OpenViking-Account` |
| `user` | `X-OpenViking-User` |

### 数据模型映射

OpenViking `MatchedContext` → loom `KnowledgeChunkCard`：

| loom 字段 | OpenViking 来源 |
|-----------|-----------------|
| `source_id` / `source_name` | provider 配置 |
| `build_id` | 固定值 `"openviking"` |
| `chunk_id` | `MatchedContext.uri`（viking:// URI） |
| `document_title` | 从 URI 路径段推导 |
| `heading_path` | 从 URI 路径段推导 |
| `summary` | `MatchedContext.abstract` |
| `matched_labels` | 空（OpenViking 无 loom 格式标签） |
| `score` | `MatchedContext.score` |

搜索结果聚合三个类别的上下文：`resources`、`memories`、`skills`，全部映射为 `KnowledgeChunkCard`。

### 分数过滤

OpenViking 返回的每个 `MatchedContext` 携带 `score` 字段，直接映射为 `KnowledgeChunkCard.score`。provider 在返回卡片前按 `min_score` 阈值过滤：`score >= min_score` 的卡片保留，低于阈值的丢弃。`min_score` 通过 `OpenVikingProviderConfig.min_score` 配置，缺省时取默认值 `0.2`。全部卡片被过滤时返回空集，该源无贡献，搜索管线自动用其他知识源（含本地源）结果补位。

### Registry 模型扩展

`KnowledgeSource` 新增 `provider` 字段（默认为 `Local`，向后兼容）：

```rust
pub enum KnowledgeProviderConfig {
    Local,
    OpenViking(OpenVikingProviderConfig),
}

pub struct OpenVikingProviderConfig {
    pub endpoint: String,               // OpenViking 服务地址
    pub api_key_env: Option<String>,    // API Key 所在环境变量名
    pub account: Option<String>,        // X-OpenViking-Account 头
    pub user: Option<String>,           // X-OpenViking-User 头
    pub target_uri: String,             // 搜索目标 URI，默认 "viking://resources/"
    pub timeout_secs: Option<u64>,      // 超时秒数，默认 10
    pub min_score: Option<f64>,         // 最低相关性分数阈值，默认 0.2
}
```

**安全设计**：`api_key_env` 只存环境变量名，不存明文密钥。运行时从环境变量读取实际 token。

### Provider 工厂

`create_provider(source) -> Box<dyn KnowledgeProvider>` 根据 provider 配置变体进行分发。`is_local_provider(source) -> bool` 辅助函数用于判断是否为本地 provider。

### 配置文件

OpenViking 知识源通过 `~/.loom/knowledge/providers.yaml` 注册：

```yaml
sources:
  - name: confluence-kb
    endpoint: http://confluence-openviking.internal:1933
    apiKeyEnv: CONFLUENCE_OV_KEY
    account: acme
    user: alice
    targetUri: viking://resources/confluence/
    timeoutSecs: 10
```

`load_merged_registry()` 在加载时将 `providers.yaml` 中的知识源合并到 registry 中（按名称匹配，yaml 不覆盖已存在的 json 知识源，但会更新其 provider 配置）。

### 搜索/检视集成

`search_cards()` 遍历所有启用的知识源，通过 `create_provider()` 分流：
- **本地源**：走现有 BM25 + 语义匹配路径（内联处理，性能最优）
- **OpenViking 源**：走 `provider.search()` 远程调用

单个 provider 故障（超时、HTTP 错误）不阻断其他知识源的搜索，错误信息输出到 stderr 并 `continue`。

`inspect_chunk()` 根据知识源的 provider 类型分发：本地源读本地文件，OpenViking 源走 `provider.inspect_chunk()` 远程调用。

### 现有 MCP 工具

不新增 MCP 工具。现有工具自动适配：

| MCP 工具 | OpenViking 源行为 |
|----------|-------------------|
| `knowledgeSearch` | ✅ 正常搜索 |
| `knowledgeBrainstormContext` | ✅ 正常搜索 |
| `knowledgeInspectChunk` | ✅ 正常检视 |
| `knowledgeList` | ✅ 正常列出 |
| `knowledgeStatus` | ✅ 正常显示状态 |
| `knowledgeEnable` / `knowledgeDisable` | ✅ 正常启用/禁用 |
| `knowledgeRemove` | ✅ 正常移除 |
| `knowledgeBuild` | ❌ 返回错误（构建仅限本地源） |
| `knowledgeResume` | ❌ 返回错误 |
| `knowledgeSemanticSubmitFile` | ❌ 返回错误 |

### 故障隔离

单个 provider 故障（网络超时、HTTP 错误）不阻断整体搜索 — 其他知识源正常返回结果。错误信息记录到 stderr。

### 依赖

`ureq = { version = "2.12", features = ["json"] }` — 轻量级同步 HTTP 客户端，无 tokio 依赖。`urlencoding = "2.1"` — URI 编码。

## 变更文件清单

| 文件 | 变更 |
|------|------|
| `knowledge/provider.rs` | 新增 — trait + LocalKnowledgeProvider + OpenVikingProvider + 工厂 |
| `knowledge/models.rs` | 新增 `provider` 字段；新增 `KnowledgeProviderConfig`、`OpenVikingProviderConfig` 类型 |
| `knowledge/store.rs` | 新增 `load_merged_registry()`、`load_providers_yaml()` — 合并 providers.yaml |
| `knowledge/operations.rs` | 读操作改用 `load_merged_registry()`；构造 `KnowledgeSource` 时补 `provider` 字段 |
| `knowledge/search.rs` | `search_cards()` 通过 provider 分流搜索 |
| `knowledge/inspect.rs` | `inspect_chunk()` 通过 provider 分流检视 |
| `knowledge/builder.rs` | `build_source()`/`resume_source()` 对 OpenViking 源返回错误 |
| `knowledge/paths.rs` | 新增 `providers_yaml_file()` |
| `knowledge/lib.rs` | 新增 `pub mod provider` + re-exports |
| `knowledge/Cargo.toml` | 新增 `ureq`、`urlencoding` 依赖 |
| `src/rust/Cargo.toml` | workspace 新增 `ureq`、`urlencoding` 依赖 |

## 实施阶段

1. **阶段一**：定义 trait + LocalKnowledgeProvider 重构（纯重构，现有测试全绿）
2. **阶段二**：OpenVikingProvider + providers.yaml 配置 + 搜索/检视集成 + 测试验证
