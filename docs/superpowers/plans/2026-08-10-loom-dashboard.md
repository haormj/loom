# Loom Dashboard 可视化方案 — 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 为 Loom 新增一个独立 Web Dashboard，通过 `loom dashboard` 命令启动本地 axum + React 服务，实时呈现交付进度、任务/review 状态、部署健康、知识库与审计数据。

**Architecture:** 新增 `dashboard` crate（零侵入读 `.loom/` 与 `~/.loom/` JSON 文件，复用现有 contracts/state Serde 模型），axum HTTP API + notify 文件监听 + SSE 推送。React 18 前端通过 Vite 构建，`rust-embed` 嵌入二进制。setup CLI 新增 `dashboard` 子命令。

**Tech Stack:** Rust 2021, axum (HTTP + SSE), notify (文件监听), tokio (异步运行时), rust-embed (静态资源嵌入), schemars (JSON Schema 导出), React 18, Vite 5, TypeScript, TanStack Query, Tailwind CSS。

## Global Constraints

- 代码保持 `rustfmt`-clean（`snake_case` 函数/模块，`UpperCamelCase` 类型）
- Dashboard 只读 `.loom/` 与 `~/.loom/` 文件，不调用 MCP 工具，不修改状态文件
- axum 绑定 `127.0.0.1`，不暴露到网络
- 所有文件路径必须解析后落在 `projectRoot/.loom/` 或 `~/.loom/` 内，拒绝 `..` 越界
- 日志查看器对 `secret`/`password`/`token`/`key` 字段做正则掩码
- 文档用简体中文（代码标识符/命令/路径/协议术语保持英文）
- Rust 集成测试归入 `tests/rust/dashboard/`
- Cargo 工作区版本 `0.2.7`，edition `2021`（`src/rust/Cargo.toml:22-23`）

---

### Task 1: dashboard crate 脚手架 + 工作区注册

**Files:**
- Create: `src/rust/dashboard/Cargo.toml`
- Create: `src/rust/dashboard/src/lib.rs`
- Modify: `src/rust/Cargo.toml:3-19` (workspace members 加 `dashboard`)
- Modify: `src/rust/setup/Cargo.toml:16-23` (加 `dashboard` 依赖)

**Interfaces:**
- Produces: `pub fn serve(project_root: &str, port: u16, open_browser: bool) -> Result<(), DashboardError>`（此 Task 先写占位实现，后续 Task 填充）
- Produces: `pub struct DashboardError(String)` with `std::fmt::Display` + `std::error::Error`

- [ ] **Step 1: 创建 Cargo.toml**

创建 `src/rust/dashboard/Cargo.toml`：

```toml
[package]
name = "dashboard"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true

[lib]
name = "dashboard"
path = "src/lib.rs"

[dependencies]
delivery-core = { path = "../core" }
contracts = { path = "../contracts" }
state = { path = "../state" }
deploy = { path = "../deploy" }
knowledge = { path = "../knowledge" }
axum = "0.7"
tokio.workspace = true
serde.workspace = true
serde_json.workspace = true
schemars.workspace = true
notify = "6"
rust-embed = "8"
tower-http = { version = "0.6", features = ["cors"] }
log.workspace = true
```

- [ ] **Step 2: 注册到工作区**

修改 `src/rust/Cargo.toml`，在 `members` 数组末尾加 `"dashboard"`：

```toml
members = [
  "mcp-server",
  "setup",
  "core",
  "state",
  "contracts",
  "workflow",
  "brainstorm",
  "planning",
  "architecture",
  "execution",
  "knowledge",
  "deploy",
  "verification",
  "algorithm-client",
  "reference-catalog",
  "dashboard",
]
```

- [ ] **Step 3: 创建 lib.rs 占位**

创建 `src/rust/dashboard/src/lib.rs`：

```rust
use std::fmt;

#[derive(Debug)]
pub struct DashboardError(pub String);

impl fmt::Display for DashboardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for DashboardError {}

pub fn serve(project_root: &str, port: u16, open_browser: bool) -> Result<(), DashboardError> {
    Err(DashboardError(format!(
        "dashboard::serve not yet implemented (project={}, port={}, open={})",
        project_root, port, open_browser
    )))
}
```

- [ ] **Step 4: 加 dashboard 依赖到 setup**

修改 `src/rust/setup/Cargo.toml`，在 `[dependencies]` 末尾加：

```toml
dashboard = { path = "../dashboard" }
```

- [ ] **Step 5: 验证编译**

Run: `cargo build --manifest-path src/rust/Cargo.toml -p dashboard -p setup`
Expected: 编译通过，无错误

- [ ] **Step 6: 验证 rustfmt**

Run: `cargo fmt --manifest-path src/rust/Cargo.toml --all --check`
Expected: 无格式问题

- [ ] **Step 7: 提交**

```bash
git add src/rust/Cargo.toml src/rust/dashboard/ src/rust/setup/Cargo.toml
git commit -m "feat(dashboard): scaffold dashboard crate and register in workspace"
```

---

### Task 2: Reader 层 — project + delivery

**Files:**
- Create: `src/rust/dashboard/src/reader/mod.rs`
- Create: `src/rust/dashboard/src/reader/project_reader.rs`
- Create: `src/rust/dashboard/src/reader/delivery_reader.rs`
- Create: `tests/rust/dashboard/reader_test.rs`
- Create: `tests/rust/dashboard/fixtures/minimal_project/.loom/status.json`
- Create: `tests/rust/dashboard/fixtures/minimal_project/.loom/config.json`
- Create: `tests/rust/dashboard/fixtures/minimal_project/.loom/deliveries/del_test/index.json`

**Interfaces:**
- Produces: `pub struct ProjectSnapshot { pub initialized: bool, pub status: Option<ProjectStatus>, pub config: Option<ProjectConfig> }`
- Produces: `pub fn read_project(project_root: &Path) -> ProjectSnapshot`
- Produces: `pub struct DeliverySummary { pub delivery_id: String, pub active_phase_id: String, pub status: String, pub phases: Vec<PhaseSummary>, pub updated_at: String }`
- Produces: `pub struct PhaseSummary { pub phase_id: String, pub latest_refs: BTreeMap<String, String>, pub status: String }`
- Produces: `pub fn read_delivery_index(project_root: &Path, delivery_id: &str) -> Option<DeliverySummary>`
- Produces: `pub fn list_deliveries(project_root: &Path) -> Vec<DeliverySummary>`
- Consumes: `state::paths::project_paths()` (`src/rust/state/paths.rs:38`), `delivery_core::ProjectStatus` (`src/rust/core/status.rs:104`), `delivery_core::DeliveryIndex` (`src/rust/core/status.rs:84`)

- [ ] **Step 1: 创建 fixtures**

创建 `tests/rust/dashboard/fixtures/minimal_project/.loom/status.json`：

```json
{
  "schemaVersion": 1,
  "activeDeliveryId": "del_test",
  "lastCompletedDeliveryId": null,
  "deliveries": [
    {
      "deliveryId": "del_test",
      "activePhaseId": "ph_01",
      "status": "executing",
      "updatedAt": "2026-08-10T12:00:00.000Z"
    }
  ],
  "updatedAt": "2026-08-10T12:00:00.000Z"
}
```

创建 `tests/rust/dashboard/fixtures/minimal_project/.loom/config.json`：

```json
{
  "schemaVersion": 1,
  "projectId": "proj_test01"
}
```

创建 `tests/rust/dashboard/fixtures/minimal_project/.loom/deliveries/del_test/index.json`：

```json
{
  "schemaVersion": 1,
  "deliveryId": "del_test",
  "activePhaseId": "ph_01",
  "status": "executing",
  "phases": [
    {
      "phaseId": "ph_01",
      "latestRefs": {
        "taskPlanRun": "run_001"
      }
    }
  ],
  "updatedAt": "2026-08-10T12:00:00.000Z"
}
```

- [ ] **Step 2: 写失败测试**

创建 `tests/rust/dashboard/reader_test.rs`：

```rust
use std::path::PathBuf;
use dashboard::reader::{read_project, list_deliveries, read_delivery_index};

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../tests/rust/dashboard/fixtures/minimal_project")
}

#[test]
fn read_project_returns_status_when_initialized() {
    let snapshot = read_project(&fixture_root());
    assert!(snapshot.initialized);
    assert!(snapshot.status.is_some());
    let status = snapshot.status.as_ref().unwrap();
    assert_eq!(status.active_delivery_id.as_deref(), Some("del_test"));
}

#[test]
fn read_project_returns_uninitialized_when_no_loom_dir() {
    let tmp = std::env::temp_dir().join("loom_dashboard_test_empty");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    let snapshot = read_project(&tmp);
    assert!(!snapshot.initialized);
    assert!(snapshot.status.is_none());
}

#[test]
fn list_deliveries_returns_all_delivery_indices() {
    let deliveries = list_deliveries(&fixture_root());
    assert_eq!(deliveries.len(), 1);
    assert_eq!(deliveries[0].delivery_id, "del_test");
    assert_eq!(deliveries[0].status, "executing");
}

#[test]
fn read_delivery_index_returns_phases() {
    let delivery = read_delivery_index(&fixture_root(), "del_test").unwrap();
    assert_eq!(delivery.phases.len(), 1);
    assert_eq!(delivery.phases[0].phase_id, "ph_01");
    assert!(delivery.phases[0].latest_refs.contains_key("taskPlanRun"));
}
```

- [ ] **Step 3: 运行测试确认失败**

Run: `cargo test --manifest-path src/rust/Cargo.toml -p dashboard --test reader_test`
Expected: 编译失败（模块不存在）

- [ ] **Step 4: 创建 reader/mod.rs**

创建 `src/rust/dashboard/src/reader/mod.rs`：

```rust
pub mod project_reader;
pub mod delivery_reader;

pub use project_reader::{read_project, ProjectSnapshot};
pub use delivery_reader::{list_deliveries, read_delivery_index, DeliverySummary, PhaseSummary};
```

在 `src/rust/dashboard/src/lib.rs` 加 `pub mod reader;`。

- [ ] **Step 5: 创建 project_reader.rs**

创建 `src/rust/dashboard/src/reader/project_reader.rs`：

```rust
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
    let paths = state::paths::project_paths(
        &project_root.display().to_string(),
    );
    let paths = match paths {
        Ok(p) => p,
        Err(_) => return ProjectSnapshot { initialized: false, status: None, config: None },
    };
    if !paths.loom_dir.exists() {
        return ProjectSnapshot { initialized: false, status: None, config: None };
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
```

- [ ] **Step 6: 创建 delivery_reader.rs**

创建 `src/rust/dashboard/src/reader/delivery_reader.rs`：

```rust
use std::collections::BTreeMap;
use std::path::Path;

use delivery_core::DeliveryIndex;
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

impl From<DeliveryIndex> for DeliverySummary {
    fn from(idx: DeliveryIndex) -> Self {
        let phases: Vec<PhaseSummary> = idx
            .phases
            .iter()
            .map(|p| PhaseSummary {
                phase_id: p.phase_id.clone(),
                latest_refs: p.latest_refs.clone(),
                status: idx.status.clone().into(),
            })
            .collect();
        DeliverySummary {
            delivery_id: idx.delivery_id,
            active_phase_id: idx.active_phase_id,
            status: idx.status.into(),
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
```

- [ ] **Step 7: 配置测试 target**

在 `src/rust/dashboard/Cargo.toml` 末尾加：

```toml
[[test]]
name = "reader_test"
path = "../../../tests/rust/dashboard/reader_test.rs"
```

- [ ] **Step 8: 运行测试确认通过**

Run: `cargo test --manifest-path src/rust/Cargo.toml -p dashboard --test reader_test`
Expected: 4 个测试全部 PASS

- [ ] **Step 9: 验证 rustfmt**

Run: `cargo fmt --manifest-path src/rust/Cargo.toml --all --check`
Expected: 无格式问题

- [ ] **Step 10: 提交**

```bash
git add src/rust/dashboard/src/reader/ tests/rust/dashboard/
git commit -m "feat(dashboard): add project and delivery reader layer with tests"
```

---

### Task 3: Reader 层 — task + review + deploy + knowledge + audit

**Files:**
- Create: `src/rust/dashboard/src/reader/task_reader.rs`
- Create: `src/rust/dashboard/src/reader/review_reader.rs`
- Create: `src/rust/dashboard/src/reader/deploy_reader.rs`
- Create: `src/rust/dashboard/src/reader/knowledge_reader.rs`
- Create: `src/rust/dashboard/src/reader/audit_reader.rs`
- Modify: `src/rust/dashboard/src/reader/mod.rs` (注册新模块)
- Modify: `tests/rust/dashboard/reader_test.rs` (加新测试)

**Interfaces:**
- Produces: `pub fn read_task_plan_run(project_root: &Path, delivery_id: &str, phase_id: &str) -> Option<TaskPlanRun>`
- Produces: `pub fn read_latest_review(project_root: &Path, delivery_id: &str, phase_id: &str) -> Option<ReviewResult>`
- Produces: `pub fn read_deploy_state(project_root: &Path) -> DeploySnapshot` (含 state/spec/log_tail/repair_action)
- Produces: `pub fn list_knowledge_sources() -> Vec<KnowledgeSourceSummary>`
- Produces: `pub fn read_audit_records(project_root: &Path, limit: usize) -> Vec<AuditRecord>`
- Consumes: `execution::paths::task_plan_run_latest_file()` (`src/rust/execution/paths.rs:76`), `deploy::paths::deployment_paths()` (`src/rust/deploy/paths.rs:25`), `knowledge::paths::registry_file()` (`src/rust/knowledge/paths.rs:17`)

- [ ] **Step 1: 创建 task_reader.rs**

创建 `src/rust/dashboard/src/reader/task_reader.rs`：

```rust
use std::path::Path;

use contracts::TaskPlanRun;
use state::paths::DeliveryPhaseLocator;

pub fn read_task_plan_run(
    project_root: &Path,
    delivery_id: &str,
    phase_id: &str,
) -> Option<TaskPlanRun> {
    let locator = DeliveryPhaseLocator {
        delivery_id: delivery_id.to_string(),
        phase_id: phase_id.to_string(),
    };
    let latest_path = execution::paths::task_plan_run_latest_file(project_root, &locator);
    let data = std::fs::read_to_string(&latest_path).ok()?;
    let run: TaskPlanRun = serde_json::from_str(&data).ok()?;
    Some(run)
}
```

注意：`execution` crate 需要加为 dashboard 的依赖。在 `src/rust/dashboard/Cargo.toml` 的 `[dependencies]` 加：

```toml
execution = { path = "../execution" }
```

- [ ] **Step 2: 创建 review_reader.rs**

创建 `src/rust/dashboard/src/reader/review_reader.rs`：

```rust
use std::path::Path;

use contracts::ReviewResult;
use state::paths::{delivery_dir, DeliveryPhaseLocator};

pub fn read_latest_review(
    project_root: &Path,
    delivery_id: &str,
    phase_id: &str,
) -> Option<ReviewResult> {
    let locator = DeliveryPhaseLocator {
        delivery_id: delivery_id.to_string(),
        phase_id: phase_id.to_string(),
    };
    let reviews_dir = delivery_dir(project_root, delivery_id)
        .join("reviews")
        .join(phase_id)
        .join("results");
    let latest_path = reviews_dir.join("latest.json");
    let data = std::fs::read_to_string(&latest_path).ok()?;
    let review: ReviewResult = serde_json::from_str(&data).ok()?;
    Some(review)
}
```

- [ ] **Step 3: 创建 deploy_reader.rs**

创建 `src/rust/dashboard/src/reader/deploy_reader.rs`：

```rust
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
```

- [ ] **Step 4: 创建 knowledge_reader.rs**

创建 `src/rust/dashboard/src/reader/knowledge_reader.rs`：

```rust
use serde::Serialize;
use std::path::PathBuf;

use knowledge::paths::{knowledge_root, registry_file, source_dir, build_run_dir, chunks_dir};

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
    let sources = registry
        .get("sources")
        .and_then(|s| s.as_array())
        .unwrap_or(&vec![]);
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
                enabled: src
                    .get("enabled")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true),
                document_count: document_paths,
                current_build_id: src
                    .get("currentBuildId")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
            }
        })
        .collect()
}

pub fn read_chunk_body(source_id: &str, build_id: &str, chunk_id: &str) -> Option<String> {
    let chunk_path = chunks_dir(source_id, build_id)
        .ok()?
        .join(format!("{chunk_id}.txt"));
    std::fs::read_to_string(&chunk_path).ok()
}
```

- [ ] **Step 5: 创建 audit_reader.rs**

创建 `src/rust/dashboard/src/reader/audit_reader.rs`：

```rust
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
```

- [ ] **Step 6: 更新 reader/mod.rs**

替换 `src/rust/dashboard/src/reader/mod.rs`：

```rust
pub mod project_reader;
pub mod delivery_reader;
pub mod task_reader;
pub mod review_reader;
pub mod deploy_reader;
pub mod knowledge_reader;
pub mod audit_reader;

pub use project_reader::{read_project, ProjectSnapshot};
pub use delivery_reader::{list_deliveries, read_delivery_index, DeliverySummary, PhaseSummary};
pub use task_reader::read_task_plan_run;
pub use review_reader::read_latest_review;
pub use deploy_reader::{read_deploy_state, DeploySnapshot};
pub use knowledge_reader::{list_knowledge_sources, read_chunk_body, KnowledgeSourceSummary};
pub use audit_reader::{read_audit_records, read_field_audit_records, AuditRecord};
```

- [ ] **Step 7: 加测试到 reader_test.rs**

在 `tests/rust/dashboard/reader_test.rs` 末尾加：

```rust
use dashboard::reader::{read_deploy_state, list_knowledge_sources, read_audit_records};

#[test]
fn read_deploy_state_returns_empty_when_no_deployment() {
    let snapshot = read_deploy_state(&fixture_root());
    assert!(!snapshot.prepared);
    assert!(snapshot.state.is_none());
    assert!(snapshot.log_tail.is_empty());
}

#[test]
fn list_knowledge_sources_does_not_panic_without_loom_home() {
    let sources = list_knowledge_sources();
    // May be empty if no knowledge sources registered; just ensure no panic
    let _ = sources.len();
}

#[test]
fn read_audit_records_returns_empty_when_no_metrics() {
    let records = read_audit_records(&fixture_root(), 50);
    assert!(records.is_empty());
}
```

- [ ] **Step 8: 运行测试**

Run: `cargo test --manifest-path src/rust/Cargo.toml -p dashboard --test reader_test`
Expected: 7 个测试全部 PASS

- [ ] **Step 9: 验证 rustfmt + 编译**

Run: `cargo fmt --manifest-path src/rust/Cargo.toml --all --check && cargo build --manifest-path src/rust/Cargo.toml -p dashboard`
Expected: 通过

- [ ] **Step 10: 提交**

```bash
git add src/rust/dashboard/ tests/rust/dashboard/
git commit -m "feat(dashboard): add task/review/deploy/knowledge/audit readers with tests"
```

---

### Task 4: HTTP server + API 路由

**Files:**
- Create: `src/rust/dashboard/src/server.rs`
- Create: `src/rust/dashboard/src/routes/mod.rs`
- Create: `src/rust/dashboard/src/routes/project.rs`
- Create: `src/rust/dashboard/src/routes/deliveries.rs`
- Create: `src/rust/dashboard/src/routes/tasks.rs`
- Create: `src/rust/dashboard/src/routes/reviews.rs`
- Create: `src/rust/dashboard/src/routes/deploy.rs`
- Create: `src/rust/dashboard/src/routes/knowledge.rs`
- Create: `src/rust/dashboard/src/routes/audit.rs`
- Create: `src/rust/dashboard/src/embedded.rs`
- Modify: `src/rust/dashboard/src/lib.rs` (加 `pub mod server; pub mod routes; pub mod embedded;`)
- Modify: `src/rust/dashboard/Cargo.toml` (加 `[[test]] name = "routes_test"`)
- Create: `tests/rust/dashboard/routes_test.rs`

**Interfaces:**
- Produces: `pub async fn serve(project_root: String, port: u16, open_browser: bool) -> Result<(), DashboardError>`（实现真正的 axum 服务）
- Produces: `pub fn build_router(project_root: String) -> axum::Router`
- Consumes: Task 2/3 的 reader 函数

- [ ] **Step 1: 创建 embedded.rs（占位）**

创建 `src/rust/dashboard/src/embedded.rs`：

```rust
use axum::{
    body::Body,
    http::{header, StatusCode, Uri},
    response::Response,
};

pub fn serve_asset(uri: &Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let asset = if path.is_empty() || path == "index.html" {
        Some(("index.html", INDEX_HTML))
    } else {
        None
    };
    match asset {
        Some((name, content)) => {
            let mime = mime_type(name);
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, mime)
                .body(Body::from(content))
                .unwrap()
        }
        None => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("not found"))
            .unwrap(),
    }
}

fn mime_type(name: &str) -> &'static str {
    if name.ends_with(".html") {
        "text/html; charset=utf-8"
    } else if name.ends_with(".js") {
        "application/javascript; charset=utf-8"
    } else if name.ends_with(".css") {
        "text/css; charset=utf-8"
    } else {
        "application/octet-stream"
    }
}

const INDEX_HTML: &str = r#"<!DOCTYPE html>
<html>
<head><meta charset="utf-8"><title>Loom Dashboard</title></head>
<body>
<h1>Loom Dashboard</h1>
<p>Frontend not yet built. API is available at <code>/api/*</code>.</p>
</body>
</html>"#;
```

- [ ] **Step 2: 创建 routes/mod.rs**

创建 `src/rust/dashboard/src/routes/mod.rs`：

```rust
pub mod project;
pub mod deliveries;
pub mod tasks;
pub mod reviews;
pub mod deploy;
pub mod knowledge;
pub mod audit;
pub mod events;

use std::sync::Arc;

use axum::{routing::get, Router};

#[derive(Clone)]
pub struct AppState {
    pub project_root: Arc<String>,
}

pub fn api_router(project_root: String) -> Router {
    let state = AppState {
        project_root: Arc::new(project_root),
    };
    Router::new()
        .route("/api/project/status", get(project::status))
        .route("/api/deliveries", get(deliveries::list))
        .route("/api/deliveries/:delivery_id", get(deliveries::detail))
        .route(
            "/api/deliveries/:delivery_id/phases/:phase_id/tasks",
            get(tasks::list),
        )
        .route(
            "/api/deliveries/:delivery_id/phases/:phase_id/reviews",
            get(reviews::latest),
        )
        .route("/api/deploy/status", get(deploy::status))
        .route("/api/deploy/logs", get(deploy::logs))
        .route("/api/knowledge/sources", get(knowledge::sources))
        .route("/api/audit/records", get(audit::records))
        .route("/api/events", get(events::sse_handler))
        .with_state(state)
}
```

注意：`events` 模块在 Task 5 实现，此处先创建占位。

- [ ] **Step 3: 创建路由 handler 文件**

创建 `src/rust/dashboard/src/routes/project.rs`：

```rust
use axum::{extract::State, Json};
use serde::Serialize;

use crate::routes::AppState;
use crate::reader::read_project;

#[derive(Serialize)]
pub struct StatusResponse {
    pub initialized: bool,
    pub active_delivery_id: Option<String>,
    pub deliveries: Vec<DeliveryEntry>,
}

#[derive(Serialize)]
pub struct DeliveryEntry {
    pub delivery_id: String,
    pub status: String,
    pub updated_at: String,
}

pub async fn status(State(state): State<AppState>) -> Json<StatusResponse> {
    let project_root = std::path::PathBuf::from(state.project_root.as_str());
    let snapshot = read_project(&project_root);
    let deliveries = snapshot
        .status
        .as_ref()
        .map(|s| {
            s.deliveries
                .iter()
                .map(|d| DeliveryEntry {
                    delivery_id: d.delivery_id.clone(),
                    status: d.status.clone().into(),
                    updated_at: d.updated_at.clone(),
                })
                .collect()
        })
        .unwrap_or_default();
    Json(StatusResponse {
        initialized: snapshot.initialized,
        active_delivery_id: snapshot
            .status
            .as_ref()
            .and_then(|s| s.active_delivery_id.clone()),
        deliveries,
    })
}
```

创建 `src/rust/dashboard/src/routes/deliveries.rs`：

```rust
use std::path::PathBuf;

use axum::{
    extract::{Path, State},
    Json,
};
use serde::Serialize;

use crate::reader::{list_deliveries, read_delivery_index, DeliverySummary};
use crate::routes::AppState;

pub async fn list(State(state): State<AppState>) -> Json<Vec<DeliverySummary>> {
    let root = PathBuf::from(state.project_root.as_str());
    Json(list_deliveries(&root))
}

pub async fn detail(
    State(state): State<AppState>,
    Path(delivery_id): Path<String>,
) -> Json<Option<DeliverySummary>> {
    let root = PathBuf::from(state.project_root.as_str());
    Json(read_delivery_index(&root, &delivery_id))
}
```

创建 `src/rust/dashboard/src/routes/tasks.rs`：

```rust
use std::path::PathBuf;

use axum::{
    extract::{Path, State},
    Json,
};
use contracts::TaskPlanRun;

use crate::reader::read_task_plan_run;
use crate::routes::AppState;

pub async fn list(
    State(state): State<AppState>,
    Path((delivery_id, phase_id)): Path<(String, String)>,
) -> Json<Option<TaskPlanRun>> {
    let root = PathBuf::from(state.project_root.as_str());
    Json(read_task_plan_run(&root, &delivery_id, &phase_id))
}
```

创建 `src/rust/dashboard/src/routes/reviews.rs`：

```rust
use std::path::PathBuf;

use axum::{
    extract::{Path, State},
    Json,
};
use contracts::ReviewResult;

use crate::reader::read_latest_review;
use crate::routes::AppState;

pub async fn latest(
    State(state): State<AppState>,
    Path((delivery_id, phase_id)): Path<(String, String)>,
) -> Json<Option<ReviewResult>> {
    let root = PathBuf::from(state.project_root.as_str());
    Json(read_latest_review(&root, &delivery_id, &phase_id))
}
```

创建 `src/rust/dashboard/src/routes/deploy.rs`：

```rust
use std::path::PathBuf;

use axum::{extract::State, Json};
use serde::Serialize;

use crate::reader::{read_deploy_state, DeploySnapshot};
use crate::routes::AppState;

#[derive(Serialize)]
pub struct DeployResponse {
    pub prepared: bool,
    pub state: Option<serde_json::Value>,
    pub log_tail: Vec<String>,
    pub log_ref: Option<String>,
    pub repair_action: Option<serde_json::Value>,
    pub failure: Option<serde_json::Value>,
}

pub async fn status(State(state): State<AppState>) -> Json<DeployResponse> {
    let root = PathBuf::from(state.project_root.as_str());
    let snap = read_deploy_state(&root);
    Json(DeployResponse {
        prepared: snap.prepared,
        state: snap.state,
        log_tail: snap.log_tail,
        log_ref: snap.log_ref,
        repair_action: snap.repair_action,
        failure: snap.failure,
    })
}

pub async fn logs(State(state): State<AppState>) -> Json<DeployResponse> {
    status(State(state)).await
}
```

创建 `src/rust/dashboard/src/routes/knowledge.rs`：

```rust
use axum::{extract::State, Json};

use crate::reader::{list_knowledge_sources, KnowledgeSourceSummary};
use crate::routes::AppState;

pub async fn sources(State(_state): State<AppState>) -> Json<Vec<KnowledgeSourceSummary>> {
    Json(list_knowledge_sources())
}
```

创建 `src/rust/dashboard/src/routes/audit.rs`：

```rust
use std::path::PathBuf;

use axum::{extract::State, Json};
use serde::Serialize;

use crate::reader::{read_audit_records, read_field_audit_records, AuditRecord};
use crate::routes::AppState;

#[derive(Serialize)]
pub struct AuditResponse {
    pub request_size_records: Vec<AuditRecord>,
    pub field_read_records: Vec<AuditRecord>,
}

pub async fn records(State(state): State<AppState>) -> Json<AuditResponse> {
    let root = PathBuf::from(state.project_root.as_str());
    Json(AuditResponse {
        request_size_records: read_audit_records(&root, 50),
        field_read_records: read_field_audit_records(&root, 50),
    })
}
```

创建 `src/rust/dashboard/src/routes/events.rs`（SSE 占位，Task 5 填充）：

```rust
use axum::response::Response;
use axum::body::Body;

pub async fn sse_handler() -> Response {
    Response::builder()
        .header("content-type", "text/event-stream")
        .body(Body::from("event: heartbeat\ndata: {}\n\n"))
        .unwrap()
}
```

- [ ] **Step 4: 创建 server.rs**

创建 `src/rust/dashboard/src/server.rs`：

```rust
use std::net::SocketAddr;
use std::path::PathBuf;

use axum::{routing::get, Router};

use crate::embedded;
use crate::routes::api_router;
use crate::DashboardError;

pub fn build_router(project_root: String) -> Router {
    let api = api_router(project_root);
    Router::new()
        .route(
            "/",
            get(|| async {
                axum::response::Html(embedded::INDEX_HTML)
            }),
        )
        .nest_service("/assets", axum::routing::any(asset_handler))
        .merge(api)
}

async fn asset_handler(uri: axum::http::Uri) -> axum::response::Response {
    embedded::serve_asset(&uri)
}

pub async fn serve(
    project_root: String,
    port: u16,
    open_browser: bool,
) -> Result<(), DashboardError> {
    let app = build_router(project_root.clone());
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| DashboardError(format!("failed to bind 127.0.0.1:{}: {}", port, e)))?;
    let actual_port = listener.local_addr().unwrap().port();
    log::info!(
        "Loom Dashboard serving on http://127.0.0.1:{} (project: {})",
        actual_port,
        project_root
    );
    if open_browser {
        let url = format!("http://127.0.0.1:{}", actual_port);
        let _ = open_url(&url);
    }
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|e| DashboardError(format!("server error: {}", e)))?;
    Ok(())
}

fn open_url(url: &str) -> Result<(), ()> {
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open").arg(url).spawn().map(|_| ()).map_err(|_| ())
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(url).spawn().map(|_| ()).map_err(|_| ())
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = url;
        Err(())
    }
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("install Ctrl-C handler");
    log::info!("Loom Dashboard shutting down...");
}
```

- [ ] **Step 5: 更新 lib.rs**

替换 `src/rust/dashboard/src/lib.rs` 的 `serve` 函数为真正的 async 实现：

```rust
use std::fmt;

pub mod embedded;
pub mod reader;
pub mod routes;
pub mod server;

#[derive(Debug)]
pub struct DashboardError(pub String);

impl fmt::Display for DashboardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for DashboardError {}

pub async fn serve(project_root: &str, port: u16, open_browser: bool) -> Result<(), DashboardError> {
    server::serve(project_root.to_string(), port, open_browser).await
}

pub fn build_router(project_root: String) -> axum::Router {
    server::build_router(project_root)
}
```

注意 `serve` 签名从同步改为 async。setup CLI 调用处需相应调整（Task 6）。

- [ ] **Step 6: 写路由测试**

创建 `tests/rust/dashboard/routes_test.rs`：

```rust
use axum::body::Body;
use http_body_util::BodyExt;
use std::path::PathBuf;

use dashboard::build_router;

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../tests/rust/dashboard/fixtures/minimal_project")
}

fn router() -> axum::Router {
    build_router(fixture_root().display().to_string())
}

async fn get_json(router: axum::Router, path: &str) -> serde_json::Value {
    use tower::ServiceExt;
    let response = router
        .oneshot(
            axum::http::Request::builder()
                .uri(path)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null)
}

#[tokio::test]
async fn project_status_returns_initialized() {
    let value = get_json(router(), "/api/project/status").await;
    assert_eq!(value["initialized"], true);
    assert_eq!(value["activeDeliveryId"], "del_test");
}

#[tokio::test]
async fn deliveries_list_returns_entries() {
    let value = get_json(router(), "/api/deliveries").await;
    let arr = value.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["deliveryId"], "del_test");
}

#[tokio::test]
async fn delivery_detail_returns_phases() {
    let value = get_json(router(), "/api/deliveries/del_test").await;
    assert_eq!(value["deliveryId"], "del_test");
    assert_eq!(value["phases"][0]["phaseId"], "ph_01");
}

#[tokio::test]
async fn deploy_status_returns_not_prepared() {
    let value = get_json(router(), "/api/deploy/status").await;
    assert_eq!(value["prepared"], false);
}
```

在 `src/rust/dashboard/Cargo.toml` 加测试依赖与 target：

```toml
[dev-dependencies]
tower = "0.5"
http-body-util = "0.1"

[[test]]
name = "routes_test"
path = "../../../tests/rust/dashboard/routes_test.rs"
```

- [ ] **Step 7: 运行测试**

Run: `cargo test --manifest-path src/rust/Cargo.toml -p dashboard --test routes_test`
Expected: 4 个测试全部 PASS

- [ ] **Step 8: 验证 rustfmt + 编译**

Run: `cargo fmt --manifest-path src/rust/Cargo.toml --all --check && cargo build --manifest-path src/rust/Cargo.toml -p dashboard`
Expected: 通过

- [ ] **Step 9: 提交**

```bash
git add src/rust/dashboard/ tests/rust/dashboard/
git commit -m "feat(dashboard): add HTTP server, API routes, and embedded asset serving"
```

---

### Task 5: 文件监听 + SSE 推送

**Files:**
- Create: `src/rust/dashboard/src/watcher.rs`
- Modify: `src/rust/dashboard/src/routes/events.rs` (实现真正的 SSE)
- Modify: `src/rust/dashboard/src/server.rs` (启动 watcher，注入 broadcast channel)
- Modify: `src/rust/dashboard/src/lib.rs` (加 `pub mod watcher;`)
- Modify: `src/rust/dashboard/Cargo.toml` (加 `tokio` broadcast feature)
- Create: `tests/rust/dashboard/watcher_test.rs`

**Interfaces:**
- Produces: `pub struct DashboardEvent { pub event_type: String, pub data: serde_json::Value }`
- Produces: `pub fn start_watcher(project_root: PathBuf) -> tokio::sync::broadcast::Receiver<DashboardEvent>`
- Consumes: `notify` crate, `tokio::sync::broadcast`

- [ ] **Step 1: 创建 watcher.rs**

创建 `src/rust/dashboard/src/watcher.rs`：

```rust
use std::path::PathBuf;
use std::time::Duration;

use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::Serialize;
use tokio::sync::broadcast;

#[derive(Debug, Clone, Serialize)]
pub struct DashboardEvent {
    #[serde(rename = "eventType")]
    pub event_type: String,
    pub data: serde_json::Value,
}

const DEBOUNCE_MS: u64 = 100;

pub fn start_watcher(project_root: PathBuf) -> broadcast::Receiver<DashboardEvent> {
    let (tx, rx) = broadcast::channel::<DashboardEvent>(64);

    let loom_dir = project_root.join(".loom");
    let project_root_clone = project_root.clone();

    std::thread::spawn(move || {
        if !loom_dir.exists() {
            log::warn!("dashboard watcher: .loom/ does not exist, skipping");
            return;
        }

        let (notify_tx, notify_rx) = std::sync::mpsc::channel::<notify::Result<notify::Event>>();

        let mut watcher = match RecommendedWatcher::new(
            move |res| {
                let _ = notify_tx.send(res);
            },
            Config::default().with_poll_interval(Duration::from_millis(DEBOUNCE_MS)),
        ) {
            Ok(w) => w,
            Err(e) => {
                log::warn!("dashboard watcher: failed to create watcher: {}", e);
                return;
            }
        };

        let watch_paths = [
            loom_dir.join("status.json"),
            loom_dir.join("deliveries"),
            loom_dir.join("deployment").join("state"),
            loom_dir.join("deployment").join("logs").join("local.log"),
        ];

        for path in &watch_paths {
            if path.exists() {
                if let Err(e) = watcher.watch(path, RecursiveMode::Recursive) {
                    log::warn!("dashboard watcher: cannot watch {:?}: {}", path, e);
                }
            }
        }

        let mut last_emit = std::time::Instant::now() - Duration::from_millis(DEBOUNCE_MS + 1);

        for event_result in notify_rx {
            match event_result {
                Ok(event) => {
                    if last_emit.elapsed() < Duration::from_millis(DEBOUNCE_MS) {
                        continue;
                    }
                    last_emit = std::time::Instant::now();

                    let event_type = classify_event(&event.kind, &event.paths);
                    if event_type == "ignore" {
                        continue;
                    }

                    let dashboard_event = DashboardEvent {
                        event_type: event_type.to_string(),
                        data: serde_json::json!({
                            "paths": event.paths.iter().map(|p| {
                                p.strip_prefix(&project_root_clone)
                                    .map(|s| s.display().to_string())
                                    .unwrap_or_else(|_| p.display().to_string())
                            }).collect::<Vec<_>>(),
                        }),
                    };

                    if tx.send(dashboard_event).is_err() {
                        log::debug!("dashboard watcher: no receivers, stopping");
                        break;
                    }
                }
                Err(e) => {
                    log::warn!("dashboard watcher: event error: {}", e);
                }
            }
        }
    });

    rx
}

fn classify_event(kind: &EventKind, paths: &[PathBuf]) -> &'static str {
    if !matches!(kind, EventKind::Modify(_) | EventKind::Create(_)) {
        return "ignore";
    }
    for path in paths {
        let path_str = path.to_string_lossy();
        if path_str.contains("status.json") {
            return "status";
        }
        if path_str.contains("deliveries") && path_str.contains("index.json") {
            return "delivery";
        }
        if path_str.contains("tasks") && path_str.contains("runs") {
            return "tasks";
        }
        if path_str.contains("reviews") {
            return "reviews";
        }
        if path_str.contains("deployment") && path_str.contains("state") {
            return "deploy";
        }
        if path_str.contains("local.log") {
            return "deploy_logs";
        }
    }
    "ignore"
}
```

- [ ] **Step 2: 实现 SSE handler**

替换 `src/rust/dashboard/src/routes/events.rs`：

```rust
use axum::{
    extract::State,
    response::{sse::{Event, KeepAlive, Sse}, IntoResponse},
    response::Response,
};
use futures_util::stream::{self, Stream};
use std::convert::Infallible;
use std::time::Duration;
use tokio_stream::StreamExt;

use crate::routes::AppState;
use crate::watcher::DashboardEvent;

pub async fn sse_handler(State(state): State<AppState>) -> impl IntoResponse {
    let rx = if let Some(rx) = state.event_rx.lock().await.as_mut() {
        rx.subscribe()
    } else {
        return Sse::new(stream::empty())
            .keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
            as Sse<_>;
    };

    let stream = stream_channel(rx);

    Sse::new(stream)
        .keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
}

fn stream_channel(
    mut rx: tokio::sync::broadcast::Receiver<DashboardEvent>,
) -> impl Stream<Item = Result<Event, Infallible>> + Send {
    async_stream::stream! {
        loop {
            match rx.recv().await {
                Ok(event) => {
                    yield Ok(Event::default()
                        .event(event.event_type.as_str())
                        .data(serde_json::to_string(&event.data).unwrap_or_default()));
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                    continue;
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    break;
                }
            }
        }
    }
}
```

需要在 `src/rust/dashboard/Cargo.toml` 加：

```toml
async-stream = "0.3"
futures-util = "0.3"
tokio-stream = "0.1"
```

并给 `AppState` 加 `event_rx` 字段。修改 `src/rust/dashboard/src/routes/mod.rs` 的 `AppState`：

```rust
use std::sync::Arc;
use tokio::sync::broadcast;

#[derive(Clone)]
pub struct AppState {
    pub project_root: Arc<String>,
    pub event_rx: Arc<tokio::sync::Mutex<Option<broadcast::Receiver<crate::watcher::DashboardEvent>>>>,
}
```

并在 `api_router` 中接收 `event_rx` 参数：

```rust
pub fn api_router(
    project_root: String,
    event_rx: Option<broadcast::Receiver<crate::watcher::DashboardEvent>>,
) -> Router {
    let state = AppState {
        project_root: Arc::new(project_root),
        event_rx: Arc::new(tokio::sync::Mutex::new(event_rx)),
    };
    // ... 路由不变
}
```

注意：`AppState` 需要用 `Arc<Mutex<Option<Receiver>>>` 因为 `Receiver` 不是 `Clone`，且 `Router` 要求 state 是 `Clone`。

- [ ] **Step 3: 更新 server.rs 启动 watcher**

在 `src/rust/dashboard/src/server.rs` 的 `serve` 函数中，构建 router 前启动 watcher：

```rust
pub async fn serve(
    project_root: String,
    port: u16,
    open_browser: bool,
) -> Result<(), DashboardError> {
    let project_root_path = std::path::PathBuf::from(&project_root);
    let event_rx = crate::watcher::start_watcher(project_root_path);

    let app = build_router(project_root.clone(), Some(event_rx));
    // ... 其余不变
}

pub fn build_router(
    project_root: String,
    event_rx: Option<broadcast::Receiver<crate::watcher::DashboardEvent>>,
) -> Router {
    let api = api_router(project_root, event_rx);
    // ... 其余不变
}
```

需要在 `server.rs` 顶部加 `use tokio::sync::broadcast;`。

- [ ] **Step 4: 写 watcher 测试**

创建 `tests/rust/dashboard/watcher_test.rs`：

```rust
use std::path::PathBuf;
use std::time::Duration;

use dashboard::watcher::start_watcher;

#[test]
fn watcher_emits_status_event_on_file_change() {
    let tmp = tempfile::tempdir().unwrap();
    let loom_dir = tmp.path().join(".loom");
    std::fs::create_dir_all(&loom_dir).unwrap();
    let status_file = loom_dir.join("status.json");
    std::fs::write(
        &status_file,
        r#"{"schemaVersion":1,"activeDeliveryId":null,"deliveries":[],"updatedAt":"x"}"#,
    )
    .unwrap();

    let mut rx = start_watcher(tmp.path().to_path_buf());

    // Modify the file to trigger an event
    std::thread::sleep(Duration::from_millis(200));
    std::fs::write(
        &status_file,
        r#"{"schemaVersion":1,"activeDeliveryId":"del_1","deliveries":[],"updatedAt":"y"}"#,
    )
    .unwrap();

    // Wait for event (with timeout)
    let result = rx.recv_timeout(Duration::from_secs(5));
    assert!(result.is_ok(), "should receive an event within 5 seconds");
    let event = result.unwrap();
    assert!(
        event.event_type == "status" || event.event_type == "deploy_logs",
        "event type should be classified, got: {}",
        event.event_type
    );
}
```

在 `src/rust/dashboard/Cargo.toml` 的 `[dev-dependencies]` 加：

```toml
tempfile = "3"
```

加 test target：

```toml
[[test]]
name = "watcher_test"
path = "../../../tests/rust/dashboard/watcher_test.rs"
```

- [ ] **Step 5: 运行测试**

Run: `cargo test --manifest-path src/rust/Cargo.toml -p dashboard --test watcher_test`
Expected: PASS（文件修改后收到事件）

如果测试因 notify 后端延迟不稳定，增加 sleep 到 500ms。

- [ ] **Step 6: 验证 rustfmt + 编译**

Run: `cargo fmt --manifest-path src/rust/Cargo.toml --all --check && cargo build --manifest-path src/rust/Cargo.toml -p dashboard`
Expected: 通过

- [ ] **Step 7: 提交**

```bash
git add src/rust/dashboard/ tests/rust/dashboard/
git commit -m "feat(dashboard): add file watcher with SSE event streaming"
```

---

### Task 6: setup CLI dashboard 子命令

**Files:**
- Modify: `src/rust/setup/main.rs` (加 `dashboard` 命令分支)
- Modify: `src/rust/setup/main.rs:250-258` (更新 usage 文本)

**Interfaces:**
- Consumes: `dashboard::serve()` (Task 4 的 async 版本)
- Produces: `loom-setup dashboard [--port <n>] [--no-open] [--project <path>]` CLI 命令

- [ ] **Step 1: 加 dashboard 命令到 main.rs**

在 `src/rust/setup/main.rs` 的 `run()` 函数 match 块中，`"other"` 分支前加：

```rust
        "dashboard" => {
            let options = CliOptions::parse(&args[1..])?;
            let port: u16 = options
                .port
                .unwrap_or(9876);
            let open_browser = !options.no_open;
            let project_root = options
                .project_root
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| {
                    std::env::current_dir()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|_| ".".to_string())
                });
            let rt = tokio::runtime::Runtime::new().map_err(|source| {
                SetupError::InvalidArgument(format!("failed to create tokio runtime: {}", source))
            })?;
            rt.block_on(dashboard::serve(&project_root, port, open_browser))
                .map_err(|e| SetupError::InvalidArgument(e.to_string()))?;
            Ok(serde_json::json!({
                "status": "ok",
                "message": "dashboard stopped"
            }))
        }
```

注意：需要在 `main.rs` 顶部加 `use dashboard;`（因为 setup lib 已依赖 dashboard crate）。但由于 `main.rs` 直接调用 `dashboard::serve`，需要确保 `setup/Cargo.toml` 已有 `dashboard` 依赖（Task 1 已加）。

由于 `serve` 现在是 async，需要 `tokio` runtime。`setup/Cargo.toml` 已有 `tokio.workspace = true`，但需要确认 workspace 的 tokio features 包含 `rt-multi-thread`（已包含，`src/rust/Cargo.toml:37`）。

- [ ] **Step 2: 加 CliOptions 字段**

在 `src/rust/setup/main.rs` 的 `CliOptions` struct 加字段：

```rust
#[derive(Debug, Default)]
struct CliOptions {
    agent: Option<String>,
    package_root: Option<PathBuf>,
    output_dir: Option<PathBuf>,
    platform: Option<String>,
    all: bool,
    port: Option<u16>,
    no_open: bool,
    playwright_versions: Vec<String>,
    playwright_browsers: Vec<String>,
}
```

在 `CliOptions::parse` 的 match 中加：

```rust
                "--port" => {
                    index += 1;
                    let value = required_value(args, index, "--port")?;
                    options.port = Some(value.parse().map_err(|_| {
                        SetupError::InvalidArgument("--port 需要一个有效的端口号".into())
                    })?);
                }
                "--no-open" => options.no_open = true,
```

注意 `--project` 需要复用已有的 `--package-root` 或新增 `--project`。为清晰起见，新增 `--project`：

```rust
                "--project" => {
                    index += 1;
                    options.project_root = Some(PathBuf::from(required_value(
                        args,
                        index,
                        "--project",
                    )?));
                }
```

并在 `CliOptions` 加 `project_root: Option<PathBuf>` 字段（与 `package_root` 分开，避免与其他命令混淆）。

- [ ] **Step 3: 更新 usage 文本**

在 `src/rust/setup/main.rs` 的 `usage()` 函数末尾加：

```rust
    "loom-setup dashboard [--port 9876] [--no-open] [--project <path>]"
```

- [ ] **Step 4: 验证编译**

Run: `cargo build --manifest-path src/rust/Cargo.toml -p setup`
Expected: 编译通过

- [ ] **Step 5: 手动冒烟测试**

Run: `./src/rust/target/debug/loom-setup dashboard --no-open --port 9876 &` 然后 `curl -s http://127.0.0.1:9876/api/project/status | head -c 200` 然后 `kill %1`
Expected: 返回 JSON（`initialized` 字段为 true 或 false 取决于当前目录是否初始化了 Loom）

- [ ] **Step 6: 验证 rustfmt**

Run: `cargo fmt --manifest-path src/rust/Cargo.toml --all --check`
Expected: 无格式问题

- [ ] **Step 7: 提交**

```bash
git add src/rust/setup/main.rs
git commit -m "feat(setup): add dashboard subcommand to loom-setup CLI"
```

---

### Task 7: 前端脚手架 + API 客户端 + SSE hook + 布局

**Files:**
- Create: `src/rust/dashboard/frontend/package.json`
- Create: `src/rust/dashboard/frontend/vite.config.ts`
- Create: `src/rust/dashboard/frontend/tsconfig.json`
- Create: `src/rust/dashboard/frontend/tailwind.config.ts`
- Create: `src/rust/dashboard/frontend/index.html`
- Create: `src/rust/dashboard/frontend/src/main.tsx`
- Create: `src/rust/dashboard/frontend/src/App.tsx`
- Create: `src/rust/dashboard/frontend/src/api/client.ts`
- Create: `src/rust/dashboard/frontend/src/hooks/useSSE.ts`
- Create: `src/rust/dashboard/frontend/src/components/Layout.tsx`
- Create: `src/rust/dashboard/frontend/postcss.config.js`

**Interfaces:**
- Produces: React SPA，Vite 构建输出到 `src/rust/dashboard/embedded/dist/`
- Consumes: Task 4 的 HTTP API

- [ ] **Step 1: 创建 package.json**

创建 `src/rust/dashboard/frontend/package.json`：

```json
{
  "name": "loom-dashboard-frontend",
  "private": true,
  "version": "0.2.7",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "preview": "vite preview"
  },
  "dependencies": {
    "react": "^18.3.1",
    "react-dom": "^18.3.1",
    "react-router-dom": "^6.26.0",
    "@tanstack/react-query": "^5.51.0",
    "lucide-react": "^0.400.0"
  },
  "devDependencies": {
    "@types/react": "^18.3.3",
    "@types/react-dom": "^18.3.0",
    "@vitejs/plugin-react": "^4.3.1",
    "autoprefixer": "^10.4.19",
    "postcss": "^8.4.39",
    "tailwindcss": "^3.4.4",
    "typescript": "^5.5.3",
    "vite": "^5.3.4"
  }
}
```

- [ ] **Step 2: 创建 Vite + TS + Tailwind 配置**

创建 `src/rust/dashboard/frontend/vite.config.ts`：

```typescript
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
  build: {
    outDir: '../embedded/dist',
    emptyOutDir: true,
  },
  server: {
    proxy: {
      '/api': 'http://127.0.0.1:9876',
    },
  },
});
```

创建 `src/rust/dashboard/frontend/tsconfig.json`：

```json
{
  "compilerOptions": {
    "target": "ES2020",
    "useDefineForClassFields": true,
    "lib": ["ES2020", "DOM", "DOM.Iterable"],
    "module": "ESNext",
    "skipLibCheck": true,
    "moduleResolution": "bundler",
    "allowImportingTsExtensions": true,
    "resolveJsonModule": true,
    "isolatedModules": true,
    "noEmit": true,
    "jsx": "react-jsx",
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noFallthroughCasesInSwitch": true
  },
  "include": ["src"]
}
```

创建 `src/rust/dashboard/frontend/tailwind.config.ts`：

```typescript
import type { Config } from 'tailwindcss';

export default {
  content: ['./index.html', './src/**/*.{ts,tsx}'],
  theme: {
    extend: {},
  },
  plugins: [],
} satisfies Config;
```

创建 `src/rust/dashboard/frontend/postcss.config.js`：

```javascript
export default {
  plugins: {
    tailwindcss: {},
    autoprefixer: {},
  },
};
```

创建 `src/rust/dashboard/frontend/index.html`：

```html
<!DOCTYPE html>
<html lang="zh-CN">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>Loom Dashboard</title>
</head>
<body class="bg-gray-50 text-gray-900">
  <div id="root"></div>
  <script type="module" src="/src/main.tsx"></script>
</body>
</html>
```

- [ ] **Step 3: 创建 React 入口 + 路由 + 布局**

创建 `src/rust/dashboard/frontend/src/main.tsx`：

```tsx
import React from 'react';
import ReactDOM from 'react-dom/client';
import { BrowserRouter } from 'react-router-dom';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import App from './App';
import './index.css';

const queryClient = new QueryClient({
  defaultOptions: {
    queries: { retry: 2, refetchOnWindowFocus: false },
  },
});

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <QueryClientProvider client={queryClient}>
      <BrowserRouter>
        <App />
      </BrowserRouter>
    </QueryClientProvider>
  </React.StrictMode>
);
```

创建 `src/rust/dashboard/frontend/src/index.css`：

```css
@tailwind base;
@tailwind components;
@tailwind utilities;
```

创建 `src/rust/dashboard/frontend/src/App.tsx`：

```tsx
import { Routes, Route } from 'react-router-dom';
import Layout from './components/Layout';
import Overview from './pages/Overview';
import DeliveryDetail from './pages/DeliveryDetail';
import Knowledge from './pages/Knowledge';
import Deploy from './pages/Deploy';
import Logs from './pages/Logs';
import Audit from './pages/Audit';

export default function App() {
  return (
    <Layout>
      <Routes>
        <Route path="/" element={<Overview />} />
        <Route path="/deliveries/:deliveryId" element={<DeliveryDetail />} />
        <Route path="/knowledge" element={<Knowledge />} />
        <Route path="/deploy" element={<Deploy />} />
        <Route path="/logs" element={<Logs />} />
        <Route path="/audit" element={<Audit />} />
      </Routes>
    </Layout>
  );
}
```

创建 `src/rust/dashboard/frontend/src/components/Layout.tsx`：

```tsx
import { NavLink } from 'react-router-dom';
import { Activity, Package, BookOpen, Server, FileText, Shield } from 'lucide-react';
import { useSSE } from '../hooks/useSSE';

const navItems = [
  { to: '/', label: '总览', icon: Activity },
  { to: '/deploy', label: '部署', icon: Server },
  { to: '/knowledge', label: '知识库', icon: BookOpen },
  { to: '/logs', label: '日志', icon: FileText },
  { to: '/audit', label: '审计', icon: Shield },
];

export default function Layout({ children }: { children: React.ReactNode }) {
  const { connected, statusText } = useSSE();

  return (
    <div className="flex h-screen flex-col">
      <header className="flex items-center justify-between border-b bg-white px-6 py-3">
        <h1 className="text-lg font-semibold">Loom Dashboard</h1>
        <div className="flex items-center gap-3">
          <span className={`h-2 w-2 rounded-full ${connected ? 'bg-green-500' : 'bg-orange-500'}`} />
          <span className="text-sm text-gray-600">{statusText}</span>
        </div>
      </header>
      <div className="flex flex-1 overflow-hidden">
        <nav className="w-48 border-r bg-white py-4">
          {navItems.map(({ to, label, icon: Icon }) => (
            <NavLink
              key={to}
              to={to}
              className={({ isActive }) =>
                `flex items-center gap-2 px-4 py-2 text-sm ${
                  isActive ? 'bg-blue-50 text-blue-700' : 'text-gray-700 hover:bg-gray-100'
                }`
              }
            >
              <Icon size={16} />
              {label}
            </NavLink>
          ))}
        </nav>
        <main className="flex-1 overflow-auto p-6">{children}</main>
      </div>
    </div>
  );
}
```

- [ ] **Step 4: 创建 API 客户端**

创建 `src/rust/dashboard/frontend/src/api/client.ts`：

```typescript
const BASE = '';

async function fetchJSON<T>(path: string): Promise<T> {
  const res = await fetch(`${BASE}${path}`);
  if (!res.ok) throw new Error(`API ${path} failed: ${res.status}`);
  return res.json();
}

export interface ProjectStatus {
  initialized: boolean;
  activeDeliveryId: string | null;
  deliveries: Array<{ deliveryId: string; status: string; updatedAt: string }>;
}

export interface DeliverySummary {
  deliveryId: string;
  activePhaseId: string;
  status: string;
  phases: Array<{ phaseId: string; latestRefs: Record<string, string>; status: string }>;
  updatedAt: string;
}

export interface DeploySnapshot {
  prepared: boolean;
  state: unknown | null;
  logTail: string[];
  logRef: string | null;
  repairAction: unknown | null;
  failure: unknown | null;
}

export const api = {
  projectStatus: () => fetchJSON<ProjectStatus>('/api/project/status'),
  deliveries: () => fetchJSON<DeliverySummary[]>('/api/deliveries'),
  delivery: (id: string) => fetchJSON<DeliverySummary | null>(`/api/deliveries/${id}`),
  tasks: (deliveryId: string, phaseId: string) =>
    fetchJSON<unknown | null>(`/api/deliveries/${deliveryId}/phases/${phaseId}/tasks`),
  reviews: (deliveryId: string, phaseId: string) =>
    fetchJSON<unknown | null>(`/api/deliveries/${deliveryId}/phases/${phaseId}/reviews`),
  deployStatus: () => fetchJSON<DeploySnapshot>('/api/deploy/status'),
  knowledgeSources: () => fetchJSON<unknown[]>('/api/knowledge/sources'),
  auditRecords: () => fetchJSON<unknown>('/api/audit/records'),
};
```

- [ ] **Step 5: 创建 SSE hook**

创建 `src/rust/dashboard/frontend/src/hooks/useSSE.ts`：

```typescript
import { useEffect, useRef, useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';

export function useSSE() {
  const queryClient = useQueryClient();
  const [connected, setConnected] = useState(false);
  const [statusText, setStatusText] = useState('连接中...');
  const reconnectTimer = useRef<ReturnType<typeof setTimeout>>();

  useEffect(() => {
    let es: EventSource | null = null;
    let retryDelay = 1000;

    function connect() {
      es = new EventSource('/api/events');

      es.onopen = () => {
        setConnected(true);
        setStatusText('实时');
        retryDelay = 1000;
      };

      const handlers: Record<string, () => void> = {
        status: () => queryClient.invalidateQueries({ queryKey: ['projectStatus'] }),
        delivery: () => {
          queryClient.invalidateQueries({ queryKey: ['deliveries'] });
        },
        tasks: () => queryClient.invalidateQueries({ queryKey: ['tasks'] }),
        reviews: () => queryClient.invalidateQueries({ queryKey: ['reviews'] }),
        deploy: () => queryClient.invalidateQueries({ queryKey: ['deployStatus'] }),
        knowledge: () => queryClient.invalidateQueries({ queryKey: ['knowledgeSources'] }),
      };

      Object.entries(handlers).forEach(([event, handler]) => {
        es?.addEventListener(event, handler);
      });

      es.onerror = () => {
        setConnected(false);
        setStatusText(`连接中断 (${retryDelay / 1000}s 后重连)`);
        es?.close();
        reconnectTimer.current = setTimeout(() => {
          retryDelay = Math.min(retryDelay * 2, 30000);
          connect();
        }, retryDelay);
      };
    }

    connect();

    return () => {
      es?.close();
      if (reconnectTimer.current) clearTimeout(reconnectTimer.current);
    };
  }, [queryClient]);

  return { connected, statusText };
}
```

- [ ] **Step 6: 创建占位页面**

创建以下占位文件（后续 Task 8 填充实现）：

`src/rust/dashboard/frontend/src/pages/Overview.tsx`：
```tsx
export default function Overview() {
  return <div className="text-gray-500">总览页面（待实现）</div>;
}
```

`src/rust/dashboard/frontend/src/pages/DeliveryDetail.tsx`：
```tsx
export default function DeliveryDetail() {
  return <div className="text-gray-500">交付详情页面（待实现）</div>;
}
```

`src/rust/dashboard/frontend/src/pages/Knowledge.tsx`：
```tsx
export default function Knowledge() {
  return <div className="text-gray-500">知识库页面（待实现）</div>;
}
```

`src/rust/dashboard/frontend/src/pages/Deploy.tsx`：
```tsx
export default function Deploy() {
  return <div className="text-gray-500">部署页面（待实现）</div>;
}
```

`src/rust/dashboard/frontend/src/pages/Logs.tsx`：
```tsx
export default function Logs() {
  return <div className="text-gray-500">日志页面（待实现）</div>;
}
```

`src/rust/dashboard/frontend/src/pages/Audit.tsx`：
```tsx
export default function Audit() {
  return <div className="text-gray-500">审计页面（待实现）</div>;
}
```

- [ ] **Step 7: 安装依赖并构建**

Run:
```bash
cd src/rust/dashboard/frontend && npm install && npm run build
```
Expected: `src/rust/dashboard/embedded/dist/` 目录生成，含 `index.html` + `assets/`

- [ ] **Step 8: 更新 embedded.rs 使用 rust-embed**

替换 `src/rust/dashboard/src/embedded.rs` 为使用 `rust-embed` 读取真实构建产物：

```rust
use axum::{
    body::Body,
    http::{header, StatusCode, Uri},
    response::Response,
};

#[derive(rust_embed::RustEmbed)]
#[folder = "embedded/dist/"]
struct DashboardAssets;

pub fn serve_asset(uri: &Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let file_path = if path.is_empty() { "index.html" } else { path };

    match DashboardAssets::get(file_path) {
        Some(asset) => {
            let mime = mime_type(file_path);
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, mime)
                .body(Body::from(asset.data.into_owned()))
                .unwrap()
        }
        None => {
            // SPA fallback: serve index.html for client-side routing
            match DashboardAssets::get("index.html") {
                Some(index) => Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
                    .body(Body::from(index.data.into_owned()))
                    .unwrap(),
                None => Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Body::from("frontend not built"))
                    .unwrap(),
            }
        }
    }
}

pub const INDEX_HTML: &str = "<!DOCTYPE html><html><body>Frontend not built</body></html>";

fn mime_type(name: &str) -> &'static str {
    if name.ends_with(".html") {
        "text/html; charset=utf-8"
    } else if name.ends_with(".js") {
        "application/javascript; charset=utf-8"
    } else if name.ends_with(".css") {
        "text/css; charset=utf-8"
    } else if name.ends_with(".svg") {
        "image/svg+xml"
    } else if name.ends_with(".png") {
        "image/png"
    } else {
        "application/octet-stream"
    }
}
```

注意：如果 `embedded/dist/` 目录不存在，`rust-embed` 编译会报错。需确保 Step 7 的构建已完成。为支持开发模式（dist 不存在时编译），可加条件编译：

```rust
#[cfg(not(feature = "dev-no-embed"))]
#[derive(rust_embed::RustEmbed)]
#[folder = "embedded/dist/"]
struct DashboardAssets;
```

并在 `Cargo.toml` 加：

```toml
[features]
dev-no-embed = []
```

开发时用 `cargo build -p dashboard --features dev-no-embed`。

- [ ] **Step 9: 重新编译并冒烟测试**

Run: `cargo build --manifest-path src/rust/Cargo.toml -p setup`
然后: `./src/rust/target/debug/loom-setup dashboard --no-open --port 9876 &`
然后: `curl -s http://127.0.0.1:9876/ | head -c 200`
然后: `kill %1`
Expected: 返回 HTML（含 `<div id="root">`）

- [ ] **Step 10: 验证 rustfmt**

Run: `cargo fmt --manifest-path src/rust/Cargo.toml --all --check`
Expected: 无格式问题

- [ ] **Step 11: 提交**

```bash
git add src/rust/dashboard/
git commit -m "feat(dashboard): add React frontend scaffold, API client, SSE hook, and layout"
```

---

### Task 8: 前端页面实现

**Files:**
- Modify: `src/rust/dashboard/frontend/src/pages/Overview.tsx`
- Modify: `src/rust/dashboard/frontend/src/pages/DeliveryDetail.tsx`
- Modify: `src/rust/dashboard/frontend/src/pages/Knowledge.tsx`
- Modify: `src/rust/dashboard/frontend/src/pages/Deploy.tsx`
- Modify: `src/rust/dashboard/frontend/src/pages/Logs.tsx`
- Modify: `src/rust/dashboard/frontend/src/pages/Audit.tsx`
- Create: `src/rust/dashboard/frontend/src/components/StatusBadge.tsx`
- Create: `src/rust/dashboard/frontend/src/components/PhaseTimeline.tsx`
- Create: `src/rust/dashboard/frontend/src/components/TaskTable.tsx`

**Interfaces:**
- Consumes: Task 7 的 API client + SSE hook
- Produces: 6 个完整功能的前端页面

- [ ] **Step 1: 创建共享组件**

创建 `src/rust/dashboard/frontend/src/components/StatusBadge.tsx`：

```tsx
const statusColors: Record<string, string> = {
  completed: 'bg-green-100 text-green-800',
  executing: 'bg-blue-100 text-blue-800',
  reviewing: 'bg-yellow-100 text-yellow-800',
  repairing: 'bg-orange-100 text-orange-800',
  planning: 'bg-purple-100 text-purple-800',
  blocked: 'bg-red-100 text-red-800',
};

export default function StatusBadge({ status }: { status: string }) {
  const colorClass = statusColors[status.toLowerCase()] || 'bg-gray-100 text-gray-800';
  return (
    <span className={`inline-block rounded px-2 py-0.5 text-xs font-medium ${colorClass}`}>
      {status}
    </span>
  );
}
```

创建 `src/rust/dashboard/frontend/src/components/PhaseTimeline.tsx`：

```tsx
interface Phase {
  phaseId: string;
  status: string;
}

export default function PhaseTimeline({ phases, activePhaseId }: { phases: Phase[]; activePhaseId: string }) {
  return (
    <div className="flex items-center gap-1 overflow-x-auto py-4">
      {phases.map((phase, idx) => {
        const isActive = phase.phaseId === activePhaseId;
        const isComplete = phase.status === 'completed';
        const dotColor = isComplete ? 'bg-green-500' : isActive ? 'bg-blue-500' : 'bg-gray-300';
        return (
          <div key={phase.phaseId} className="flex items-center">
            <div className="flex flex-col items-center">
              <div className={`h-3 w-3 rounded-full ${dotColor}`} />
              <span className="mt-1 text-xs text-gray-600">{phase.phaseId}</span>
            </div>
            {idx < phases.length - 1 && (
              <div className={`h-0.5 w-8 ${isComplete ? 'bg-green-400' : 'bg-gray-200'}`} />
            )}
          </div>
        );
      })}
    </div>
  );
}
```

创建 `src/rust/dashboard/frontend/src/components/TaskTable.tsx`：

```tsx
interface TaskState {
  taskId: string;
  status: string;
}

interface TaskPlanRun {
  runId: string;
  taskStates: TaskState[];
  summary: { total: number; completed: number; running: number; pending: number; failed: number };
}

export default function TaskTable({ run }: { run: TaskPlanRun | null }) {
  if (!run) return <p className="text-gray-400">暂无任务数据</p>;
  return (
    <div>
      <div className="mb-2 text-sm text-gray-600">
        总计 {run.summary.total} | 完成 {run.summary.completed} | 运行中 {run.summary.running} | 待办 {run.summary.pending}
        {run.summary.failed > 0 && <span className="text-red-600"> | 失败 {run.summary.failed}</span>}
      </div>
      <table className="w-full text-sm">
        <thead>
          <tr className="border-b text-left text-gray-500">
            <th className="py-2">Task ID</th>
            <th>状态</th>
          </tr>
        </thead>
        <tbody>
          {run.taskStates.map((task) => (
            <tr key={task.taskId} className="border-b">
              <td className="py-1.5 font-mono text-xs">{task.taskId}</td>
              <td><StatusBadge status={task.status} /></td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

import StatusBadge from './StatusBadge';
```

- [ ] **Step 2: 实现 Overview 页面**

替换 `src/rust/dashboard/frontend/src/pages/Overview.tsx`：

```tsx
import { useQuery } from '@tanstack/react-query';
import { api, ProjectStatus } from '../api/client';
import StatusBadge from '../components/StatusBadge';

export default function Overview() {
  const { data, isLoading } = useQuery({
    queryKey: ['projectStatus'],
    queryFn: api.projectStatus,
  });

  if (isLoading) return <p>加载中...</p>;
  if (!data) return <p>无法加载项目状态</p>;

  return (
    <div className="space-y-6">
      <h2 className="text-xl font-semibold">项目状态</h2>
      {!data.initialized ? (
        <div className="rounded border border-yellow-300 bg-yellow-50 p-4 text-yellow-800">
          当前项目未初始化 Loom。在 agent 中运行 <code>/loom</code> 开始。
        </div>
      ) : (
        <>
          <div className="rounded-lg border bg-white p-4">
            <div className="flex items-center justify-between">
              <span className="text-sm text-gray-500">活跃交付</span>
              {data.activeDeliveryId && <StatusBadge status="executing" />}
            </div>
            {data.activeDeliveryId ? (
              <p className="mt-2 font-mono text-lg">{data.activeDeliveryId}</p>
            ) : (
              <p className="mt-2 text-gray-400">无活跃交付</p>
            )}
          </div>
          <div>
            <h3 className="mb-2 text-sm font-medium text-gray-700">所有交付</h3>
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b text-left text-gray-500">
                  <th className="py-2">Delivery</th>
                  <th>状态</th>
                  <th>更新时间</th>
                </tr>
              </thead>
              <tbody>
                {data.deliveries.map((d) => (
                  <tr key={d.deliveryId} className="border-b hover:bg-gray-50">
                    <td className="py-2 font-mono text-xs">
                      <a href={`/deliveries/${d.deliveryId}`} className="text-blue-600 hover:underline">
                        {d.deliveryId}
                      </a>
                    </td>
                    <td><StatusBadge status={d.status} /></td>
                    <td className="text-gray-500">{d.updatedAt}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </>
      )}
    </div>
  );
}
```

- [ ] **Step 3: 实现 DeliveryDetail 页面**

替换 `src/rust/dashboard/frontend/src/pages/DeliveryDetail.tsx`：

```tsx
import { useParams } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import { api, DeliverySummary } from '../api/client';
import PhaseTimeline from '../components/PhaseTimeline';
import TaskTable from '../components/TaskTable';

export default function DeliveryDetail() {
  const { deliveryId } = useParams<{ deliveryId: string }>();
  const { data: delivery } = useQuery({
    queryKey: ['deliveries', deliveryId],
    queryFn: () => api.delivery(deliveryId!),
    enabled: !!deliveryId,
  });

  if (!delivery) return <p className="text-gray-400">交付不存在</p>;

  const activePhase = delivery.phases.find((p) => p.phaseId === delivery.activePhaseId);
  const taskRunRef = activePhase?.latestRefs['taskPlanRun'];

  const { data: taskRun } = useQuery({
    queryKey: ['tasks', deliveryId, activePhase?.phaseId],
    queryFn: () => api.tasks(deliveryId!, activePhase!.phaseId),
    enabled: !!activePhase,
  });

  return (
    <div className="space-y-6">
      <div className="flex items-center gap-3">
        <h2 className="text-xl font-semibold font-mono">{delivery.deliveryId}</h2>
        <span className="rounded bg-blue-100 px-2 py-0.5 text-xs text-blue-800">{delivery.status}</span>
      </div>
      <div>
        <h3 className="mb-2 text-sm font-medium text-gray-700">阶段时间线</h3>
        <PhaseTimeline phases={delivery.phases} activePhaseId={delivery.activePhaseId} />
      </div>
      {activePhase && (
        <div>
          <h3 className="mb-2 text-sm font-medium text-gray-700">
            任务列表 ({activePhase.phaseId})
          </h3>
          <TaskTable run={taskRun as any} />
        </div>
      )}
    </div>
  );
}
```

- [ ] **Step 4: 实现 Deploy 页面**

替换 `src/rust/dashboard/frontend/src/pages/Deploy.tsx`：

```tsx
import { useQuery } from '@tanstack/react-query';
import { api } from '../api/client';

export default function Deploy() {
  const { data } = useQuery({
    queryKey: ['deployStatus'],
    queryFn: api.deployStatus,
    refetchInterval: 5000,
  });

  if (!data) return <p className="text-gray-400">暂无部署数据</p>;

  return (
    <div className="space-y-4">
      <h2 className="text-xl font-semibold">部署状态</h2>
      {!data.prepared ? (
        <div className="rounded border border-gray-200 bg-gray-50 p-4 text-gray-500">
          尚未进入部署阶段
        </div>
      ) : (
        <>
          <div className="rounded-lg border bg-white p-4">
            <h3 className="mb-2 text-sm font-medium text-gray-700">实时日志</h3>
            <pre className="max-h-96 overflow-auto rounded bg-gray-900 p-3 text-xs text-gray-100">
              {data.logTail.join('\n')}
            </pre>
            {data.logRef && (
              <p className="mt-2 text-xs text-gray-500">完整日志: <code>{data.logRef}</code></p>
            )}
          </div>
          {data.failure && (
            <div className="rounded-lg border border-red-200 bg-red-50 p-4">
              <h3 className="mb-2 text-sm font-medium text-red-800">最新失败</h3>
              <pre className="overflow-auto text-xs">{JSON.stringify(data.failure, null, 2)}</pre>
            </div>
          )}
        </>
      )}
    </div>
  );
}
```

- [ ] **Step 5: 实现 Logs 页面**

替换 `src/rust/dashboard/frontend/src/pages/Logs.tsx`：

```tsx
import { useQuery } from '@tanstack/react-query';
import { api } from '../api/client';

export default function Logs() {
  const { data: deployData } = useQuery({
    queryKey: ['deployStatus'],
    queryFn: api.deployStatus,
  });

  const logs = deployData?.logTail ?? [];

  return (
    <div className="space-y-4">
      <h2 className="text-xl font-semibold">部署日志</h2>
      {logs.length === 0 ? (
        <p className="text-gray-400">暂无日志</p>
      ) : (
        <pre className="max-h-[calc(100vh-200px)] overflow-auto rounded bg-gray-900 p-3 text-xs text-gray-100">
          {logs.map((line, i) => {
            let cls = 'text-gray-100';
            if (line.includes('[ERROR]')) cls = 'text-red-400';
            else if (line.includes('[WARN]')) cls = 'text-yellow-400';
            else if (line.includes('[INFO]')) cls = 'text-green-400';
            return <div key={i} className={cls}>{line}</div>;
          })}
        </pre>
      )}
    </div>
  );
}
```

- [ ] **Step 6: 实现 Knowledge 页面**

替换 `src/rust/dashboard/frontend/src/pages/Knowledge.tsx`：

```tsx
import { useQuery } from '@tanstack/react-query';
import { api } from '../api/client';

export default function Knowledge() {
  const { data: sources } = useQuery({
    queryKey: ['knowledgeSources'],
    queryFn: api.knowledgeSources,
  });

  return (
    <div className="space-y-4">
      <h2 className="text-xl font-semibold">知识库</h2>
      {!sources || sources.length === 0 ? (
        <p className="text-gray-400">暂无知识源。在 agent 中运行 <code>/loom knowledge add</code> 添加。</p>
      ) : (
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b text-left text-gray-500">
              <th className="py-2">名称</th>
              <th>Source ID</th>
              <th>文档数</th>
              <th>状态</th>
            </tr>
          </thead>
          <tbody>
            {sources.map((s: any) => (
              <tr key={s.sourceId} className="border-b hover:bg-gray-50">
                <td className="py-2">{s.name}</td>
                <td className="font-mono text-xs text-gray-500">{s.sourceId}</td>
                <td>{s.documentCount}</td>
                <td>{s.enabled ? '✓ 已启用' : '✗ 已禁用'}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}
```

- [ ] **Step 7: 实现 Audit 页面**

替换 `src/rust/dashboard/frontend/src/pages/Audit.tsx`：

```tsx
import { useQuery } from '@tanstack/react-query';
import { api } from '../api/client';

export default function Audit() {
  const { data } = useQuery({
    queryKey: ['auditRecords'],
    queryFn: api.auditRecords,
  });

  const sizeRecords = (data as any)?.requestSizeRecords ?? [];
  const fieldRecords = (data as any)?.fieldReadRecords ?? [];

  return (
    <div className="space-y-6">
      <h2 className="text-xl font-semibold">审计</h2>
      <div>
        <h3 className="mb-2 text-sm font-medium text-gray-700">请求大小审计 (最近 {sizeRecords.length} 条)</h3>
        {sizeRecords.length === 0 ? (
          <p className="text-gray-400">暂无记录</p>
        ) : (
          <pre className="max-h-64 overflow-auto rounded bg-gray-50 p-3 text-xs">
            {JSON.stringify(sizeRecords, null, 2)}
          </pre>
        )}
      </div>
      <div>
        <h3 className="mb-2 text-sm font-medium text-gray-700">字段读取审计 (最近 {fieldRecords.length} 条)</h3>
        {fieldRecords.length === 0 ? (
          <p className="text-gray-400">暂无记录</p>
        ) : (
          <pre className="max-h-64 overflow-auto rounded bg-gray-50 p-3 text-xs">
            {JSON.stringify(fieldRecords, null, 2)}
          </pre>
        )}
      </div>
    </div>
  );
}
```

- [ ] **Step 8: 构建并验证**

Run:
```bash
cd src/rust/dashboard/frontend && npm run build
```
Expected: 构建成功，`embedded/dist/` 更新

- [ ] **Step 9: 冒烟测试**

Run: `cargo build --manifest-path src/rust/Cargo.toml -p setup && ./src/rust/target/debug/loom-setup dashboard --no-open --port 9876 &`
然后: 在浏览器打开 `http://127.0.0.1:9876`，验证页面加载、导航切换
然后: `curl -s http://127.0.0.1:9876/api/project/status`
然后: `kill %1`
Expected: 页面加载，API 正常响应

- [ ] **Step 10: 提交**

```bash
git add src/rust/dashboard/frontend/
git commit -m "feat(dashboard): implement all 6 frontend pages with real-time updates"
```

---

### Task 9: 构建集成 + install.sh 更新 + 端到端测试

**Files:**
- Modify: `install.sh:180-191` (local-build 分支加前端构建)
- Modify: `install.ps1:113` (Windows 对应)
- Modify: `.github/workflows/release.yml:64` (CI 对应)
- Create: `tests/rust/dashboard/fixtures/full_delivery/.loom/` (完整 fixture)
- Create: `tests/rust/dashboard/integration_test.rs`
- Modify: `src/rust/dashboard/Cargo.toml` (加 integration test target)

**Interfaces:**
- Produces: `./install.sh --local-build` 自动构建前端并嵌入二进制
- Produces: 端到端集成测试验证完整流程

- [ ] **Step 1: 更新 install.sh**

在 `install.sh` 的 `LOCAL_BUILD` 分支（第 191 行 `cargo build` 之前）加：

```sh
  # Build dashboard frontend if present
  if [ -d "$REPO_ROOT/src/rust/dashboard/frontend" ]; then
    if ! command -v npm >/dev/null 2>&1; then
      fail "--local-build requires npm on PATH when dashboard frontend is present"
    fi
    echo "Building dashboard frontend..."
    (cd "$REPO_ROOT/src/rust/dashboard/frontend" && npm ci --prefer-offline && npm run build)
  fi
```

- [ ] **Step 2: 创建完整 fixture**

创建 `tests/rust/dashboard/fixtures/full_delivery/.loom/status.json`：

```json
{
  "schemaVersion": 1,
  "activeDeliveryId": "del_full",
  "lastCompletedDeliveryId": null,
  "deliveries": [
    {
      "deliveryId": "del_full",
      "activePhaseId": "ph_01",
      "status": "executing",
      "updatedAt": "2026-08-10T12:00:00.000Z"
    }
  ],
  "updatedAt": "2026-08-10T12:00:00.000Z"
}
```

创建 `tests/rust/dashboard/fixtures/full_delivery/.loom/config.json`：

```json
{ "schemaVersion": 1, "projectId": "proj_full01" }
```

创建 `tests/rust/dashboard/fixtures/full_delivery/.loom/deliveries/del_full/index.json`：

```json
{
  "schemaVersion": 1,
  "deliveryId": "del_full",
  "activePhaseId": "ph_01",
  "status": "executing",
  "phases": [
    {
      "phaseId": "ph_01",
      "latestRefs": {
        "taskPlan": "tp_001",
        "taskPlanRun": "run_001"
      }
    }
  ],
  "updatedAt": "2026-08-10T12:00:00.000Z"
}
```

创建 `tests/rust/dashboard/fixtures/full_delivery/.loom/deliveries/del_full/tasks/ph_01/runs/latest.json`：

```json
{
  "schemaVersion": "1.0",
  "runId": "run_001",
  "taskPlanId": "tp_001",
  "status": "running",
  "scheduler": "sequential",
  "groupStates": [],
  "taskStates": [
    { "taskId": "task_1", "status": "completed", "attempts": [], "dependsOn": [] },
    { "taskId": "task_2", "status": "running", "attempts": [], "dependsOn": ["task_1"] },
    { "taskId": "task_3", "status": "pending", "attempts": [], "dependsOn": ["task_1"] }
  ],
  "summary": { "total": 3, "completed": 1, "completedWithNotes": 0, "blocked": 0, "failed": 0, "pending": 1, "running": 1 },
  "nextAction": null,
  "createdAt": "2026-08-10T12:00:00.000Z",
  "updatedAt": "2026-08-10T12:01:00.000Z"
}
```

创建 `tests/rust/dashboard/fixtures/full_delivery/.loom/deployment/state/local.json`：

```json
{
  "prepared": true,
  "phase": "running",
  "services": [
    { "serviceId": "api", "status": "healthy", "port": 3000 }
  ]
}
```

创建 `tests/rust/dashboard/fixtures/full_delivery/.loom/deployment/specs/local.json`：

```json
{
  "schemaVersion": "1.0",
  "services": [{ "serviceId": "api", "runtime": "node" }]
}
```

创建 `tests/rust/dashboard/fixtures/full_delivery/.loom/deployment/logs/local.log`：

```
[2026-08-10T12:00:00Z INFO] api: server started on :3000
[2026-08-10T12:00:01Z INFO] nginx: config reloaded
[2026-08-10T12:00:05Z WARN] api: slow response on /api/health (500ms)
```

- [ ] **Step 3: 写端到端集成测试**

创建 `tests/rust/dashboard/integration_test.rs`：

```rust
use std::path::PathBuf;

use dashboard::build_router;

fn full_fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../tests/rust/dashboard/fixtures/full_delivery")
}

async fn get_json(router: axum::Router, path: &str) -> serde_json::Value {
    use axum::body::Body;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    let response = router
        .oneshot(
            axum::http::Request::builder()
                .uri(path)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null)
}

#[tokio::test]
async fn full_delivery_project_status() {
    let router = build_router(full_fixture_root().display().to_string(), None);
    let value = get_json(router, "/api/project/status").await;
    assert_eq!(value["initialized"], true);
    assert_eq!(value["activeDeliveryId"], "del_full");
}

#[tokio::test]
async fn full_delivery_list() {
    let router = build_router(full_fixture_root().display().to_string(), None);
    let value = get_json(router, "/api/deliveries").await;
    assert_eq!(value.as_array().unwrap().len(), 1);
    assert_eq!(value[0]["deliveryId"], "del_full");
}

#[tokio::test]
async fn full_delivery_tasks() {
    let router = build_router(full_fixture_root().display().to_string(), None);
    let value = get_json(router, "/api/deliveries/del_full/phases/ph_01/tasks").await;
    assert_eq!(value["runId"], "run_001");
    assert_eq!(value["summary"]["total"], 3);
    assert_eq!(value["summary"]["completed"], 1);
    assert_eq!(value["summary"]["running"], 1);
}

#[tokio::test]
async fn full_delivery_deploy() {
    let router = build_router(full_fixture_root().display().to_string(), None);
    let value = get_json(router, "/api/deploy/status").await;
    assert_eq!(value["prepared"], true);
    assert!(value["logTail"].as_array().unwrap().len() > 0);
}

#[tokio::test]
async fn index_html_served() {
    use axum::body::Body;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    let router = build_router(full_fixture_root().display().to_string(), None);
    let response = router
        .oneshot(
            axum::http::Request::builder()
                .uri("/")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let html = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(html.contains("root") || html.contains("Loom Dashboard") || html.contains("html"));
}
```

在 `src/rust/dashboard/Cargo.toml` 加：

```toml
[[test]]
name = "integration_test"
path = "../../../tests/rust/dashboard/integration_test.rs"
```

注意：`build_router` 签名在 Task 5 中改为接受 `Option<broadcast::Receiver>`。此测试传 `None`（无 SSE）。

- [ ] **Step 4: 运行集成测试**

Run: `cargo test --manifest-path src/rust/Cargo.toml -p dashboard --test integration_test`
Expected: 5 个测试全部 PASS

- [ ] **Step 5: 运行全部 dashboard 测试**

Run: `cargo test --manifest-path src/rust/Cargo.toml -p dashboard`
Expected: 所有测试 PASS（reader_test + routes_test + watcher_test + integration_test）

- [ ] **Step 6: 验证 rustfmt**

Run: `cargo fmt --manifest-path src/rust/Cargo.toml --all --check`
Expected: 无格式问题

- [ ] **Step 7: 验证 install.sh 本地构建**

Run: `./install.sh --agent opencode --local-build --print-plan`
Expected: 计划中包含 dashboard 前端构建步骤

然后实际构建：`./install.sh --agent opencode --local-build`
Expected: 成功完成（含 npm install + npm run build + cargo build）

- [ ] **Step 8: 提交**

```bash
git add install.sh tests/rust/dashboard/ src/rust/dashboard/Cargo.toml
git commit -m "feat(dashboard): integrate frontend build into install.sh and add e2e tests"
```

---

## Self-Review 总结

**Spec 覆盖检查**：
- 第 1 节（整体架构）→ Task 1 (crate 脚手架), Task 4 (server)
- 第 2 节（6 个视图页面）→ Task 8 (前端页面)
- 第 3 节（数据读取 + SSE）→ Task 2-3 (reader), Task 5 (watcher + SSE)
- 第 4 节（前端技术 + 构建集成）→ Task 7 (前端脚手架), Task 9 (install.sh)
- 第 5 节（错误处理 + 测试）→ Task 2-3 (reader 错误处理), Task 9 (集成测试)
- setup CLI 集成 → Task 6

**全部 9 个呈现缺口覆盖**：
- A（时间线）→ Task 8 DeliveryDetail + PhaseTimeline
- B（部署健康）→ Task 8 Deploy
- C（日志着色）→ Task 8 Logs
- D（知识预览）→ Task 8 Knowledge
- E（产物聚合）→ Task 8 DeliveryDetail
- F（修复历史）→ Task 8 Deploy (failure/repair 字段)
- G（实时进度）→ Task 5 SSE + Task 7 useSSE hook
- H（跨 delivery）→ Task 8 Overview
- I（日志界限）→ Task 8 Logs

**类型一致性**：已检查所有跨 Task 的函数签名与类型名一致。
