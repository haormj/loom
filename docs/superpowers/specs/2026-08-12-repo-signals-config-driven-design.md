# 仓库技术栈检测配置驱动重构设计

## 背景与动机

Loom 当前通过 `compact_repo_signals`(`src/rust/planning/technical_baseline.rs:824`)检测用户项目的技术栈。该函数调用 5 个硬编码 Rust 函数(`collect_node_signals`/`collect_java_signals`/`collect_python_signals`/`collect_rust_signals`/`collect_go_signals`),仅识别有限的 manifest 文件和框架关键词。例如 Python 只检测 `pyproject.toml` 和 `requirements.txt`,不识别 `setup.py`/`setup.cfg`/`Pipfile`/`poetry.lock`。

技术栈检测→确认→指导文档加载形成完整链路,共 6 步:

```
①检测        ②确认         ③信号派生          ④组/items选择        ⑤展开为路径         ⑥加载文件
compact_     代理/用户      signal_from_      backend_reference_   code_reference_     agent按
repo_signals → 确认为      → _selection()    → items_for_signal()  → load_plan()      → referenceLoadPlan
              stack.tracks   关键词匹配          if 分支逐框架        catalog.resolve_    读取 .md
                             框架名             产出 items           entry()
```

其中 ①③ 是硬编码,④ 也是硬编码 if 分支,⑤ 已配置驱动(catalog.toml `[[routes.groups]]`),⑥ 已配置驱动(referenceLoadPlan 协议)。

本次重构将 ①③④ 三层全部改为配置驱动,实现端到端"新增框架只需改 catalog.toml + 加 .md 文件,不改 Rust"。

## 目标

1. 将 `compact_repo_signals` 中 5 个硬编码 `collect_*_signals` 重构为 `catalog.toml` 配置驱动的通用引擎,支持四种检测规则类型:manifest 文件存在性、文件夹命名规范、文件扩展名扫描、框架关键词匹配。
2. 将 `signal_from_selection`(`code_quality.rs:345`)的 if-else 语言识别链改为配置驱动。
3. 将 `backend_reference_items_for_signal`(`code_quality.rs:1126`)和 `frontend_reference_items_for_signal`(`code_quality.rs:1421`)的框架 if 分支改为配置驱动条件映射。
4. 复用 `reference-catalog` crate 已有的 catalog 加载/overlay 机制(vendor→enterprise→project)。
5. 输出结构(`RepoSignalSummary` JSON 与 `CodeReferenceSelection`)保持兼容,下游消费者无感知。

## 非目标

- 不修改 `code_reference_load_plan`/`catalog.resolve_entry` ⑤(已配置驱动)。
- 不修改 agent 侧 referenceLoadPlan 加载协议 ⑥。
- 不新增框架参考 `.md` 文件(如 `tech/backend/pyramid/*.md`)。本次仅扩展检测/选择链路;catalog.toml 中只声明已有 `.md` 文件的框架组,Pyramid/Sanic/Tornado 等在检测层可识别但参考文件待后续补充。
- 不修改 `TechnicalBaselineContract.stack` 的数据结构。
- 不修改 `RepositoryContext.technology_signals` 的数据结构。

## 架构概览

### 数据流(改后)

```
catalog.toml [repoSignals] ──load──▶ ReferenceCatalog.repo_signals
        │                                      │ resolved_catalog() 缓存 + overlay
        ▼                                      │
planning::RepoSignalEngine               reference-catalog::merge_catalogs
  collect(project_root, &repo_signals)         │
        │                                      │
        ▼ (输出不变)                           │
  RepoSignalSummary                            │
        │                                      │
        ▼                                      │
  repoEvidence.signals (JSON)                  │
        │                                      │
        ▼ 代理确认 stack.tracks                 │
  contracts::code_quality                      │
  code_stack_signals_from_baseline             │
        │                                      │
        ▼ ③配置驱动                             │
  signal_from_selection (引擎遍历              │
    catalog.repo_signals.languages.selection)  │
        │                                      │
        ▼ ④配置驱动                             │
  backend/frontend_reference_items_for_signal  │
  (引擎遍历 frameworkReferences,               │
    condition.evaluate(ctx))                   │
        │                                      │
        ▼ ⑤不变                                │
  code_reference_load_plan                     │
  → catalog.resolve_entry                      │
        │                                      │
        ▼ ⑥不变                                │
  referenceLoadPlan → agent 加载 .md           │
```

### 改动范围

| 文件 | 操作 | 内容 |
|------|------|------|
| `src/rust/contracts/condition.rs` | 新增 | `Condition` 枚举 + `ConditionContext` + `evaluate()` + 谓词分发表 |
| `src/rust/contracts/mod.rs` | 改 | `pub mod condition;` |
| `src/rust/contracts/code_quality.rs` | 大改 | `signal_from_selection`/`backend_reference_items_for_signal`/`frontend_reference_items_for_signal` 改为配置驱动 |
| `src/rust/reference-catalog/schema.rs` | 改 | 新增 `RepoSignalsConfig` 及子类型;`ReferenceCatalog` 增加 `repo_signals` 字段 |
| `src/rust/reference-catalog/merge.rs` | 改 | `merge_catalogs` 增加 `repo_signals` overlay 合并 |
| `src/rust/planning/technical_baseline.rs` | 改 | 删除 5 个 `collect_*_signals` + `compact_repo_signals` → `RepoSignalEngine` |
| `plugins/shared/loom/references/catalog.toml` | 大改 | 新增 `[repoSignals]` 段(迁移全部 5 语言检测规则 + ③④ 映射) |

### 不改动

- `RepoSignalSummary` 的输出 JSON 结构(`manifests`/`packageManagers`/`languages`/`frameworks`/`sourceRoots`)。
- `code_reference_load_plan` → `catalog.resolve_entry` 路径展开逻辑。
- `CodeReferenceSelection`/`CodeStackSignal` 的数据结构。
- agent 侧 `plugins/opencode/.opencode/commands/loom.md` 引用加载协议。

## catalog.toml `[repoSignals]` 配置结构

在 `catalog.toml` 顶层新增 `[repoSignals]` 段。结构如下(以 Python 为例展示全部四种规则类型):

```toml
# ════════════════════════════════════════════════════════════════════════
# Repo Signals — 仓库技术栈检测规则
# 由 planning crate 的 RepoSignalEngine 消费,产出 repoEvidence.signals。
# 由 contracts crate 的 code_quality 消费,驱动信号派生与参考选择。
# 支持四种检测规则:manifests / extensions / folderHints / frameworks
# ════════════════════════════════════════════════════════════════════════

[repoSignals]
# 通用跳过目录(扩展名扫描时递归跳过)
skipDirs = [".git", ".loom", "node_modules", "target", "dist", "build", "coverage", ".venv", "venv"]
# 扩展名扫描最大深度
maxScanDepth = 6

# ── 通用 source_roots 规则(替代硬编码 src/app/web/...)──
[repoSignals.sourceRoots]
paths = ["src", "app", "web", "service", "backend", "frontend", "tests"]

# ── 语言规则 ──
[[repoSignals.languages]]
id = "python"                          # 内部标识,用于日志/调试
label = "Python"                       # 写入 signals.languages 的显示名

  # 规则1: manifest 文件存在性
  [[repoSignals.languages.manifests]]
  path = "pyproject.toml"
  packageManager = "pip"               # 可选,命中时写入 packageManagers

  [[repoSignals.languages.manifests]]
  path = "requirements.txt"
  packageManager = "pip"

  [[repoSignals.languages.manifests]]
  path = "setup.py"
  packageManager = "pip"

  [[repoSignals.languages.manifests]]
  path = "setup.cfg"
  packageManager = "pip"

  [[repoSignals.languages.manifests]]
  path = "Pipfile"
  packageManager = "pipenv"

  [[repoSignals.languages.manifests]]
  path = "poetry.lock"
  packageManager = "poetry"

  # 规则2: 文件夹命名规范(命中即补强语言信号)
  [repoSignals.languages.folderHints]
  dirs = ["python", "scripts"]
  requireExtensions = [".py"]          # 且目录内需含 .py 文件才生效

  # 规则3: 文件扩展名扫描(无 manifest 时的兜底)
  [repoSignals.languages.extensions]
  scan = [".py"]
  threshold = 1                        # 命中至少 N 个文件才生效(避免误报)

  # 规则4: 框架关键词匹配(在已命中的 manifest 文件内容中搜索)
  [[repoSignals.languages.frameworks]]
  needle = "fastapi"
  label = "FastAPI"

  [[repoSignals.languages.frameworks]]
  needle = "django"
  label = "Django"

  [[repoSignals.languages.frameworks]]
  needle = "flask"
  label = "Flask"

  [[repoSignals.languages.frameworks]]
  needle = "pyramid"
  label = "Pyramid"

  [[repoSignals.languages.frameworks]]
  needle = "sanic"
  label = "Sanic"

  [[repoSignals.languages.frameworks]]
  needle = "tornado"
  label = "Tornado"

  # ── ③ 信号派生:从 stack.tracks.selection 文本识别语言+框架 ──
  [repoSignals.languages.selection]
  keywords = ["python", "fastapi", "django", "flask", "sqlalchemy", "pydantic", "drf"]
  excludeKeywords = []
  roles = ["backend_unless_persistence"]

  [[repoSignals.languages.selection.frameworks]]
  id = "fastapi"
  aliases = ["fastapi"]

  [[repoSignals.languages.selection.frameworks]]
  id = "django"
  aliases = ["django"]

  [[repoSignals.languages.selection.frameworks]]
  id = "django_rest_framework"
  aliases = ["django rest framework", "drf"]

  [[repoSignals.languages.selection.frameworks]]
  id = "pydantic"
  aliases = ["pydantic"]

  [[repoSignals.languages.selection.frameworks]]
  id = "sqlalchemy"
  aliases = ["sqlalchemy", "sql alchemy"]

  [[repoSignals.languages.selection.frameworks]]
  id = "flask"
  aliases = ["flask"]

  # ── ④ 参考选择:框架 → group items 的条件映射 ──
  [[repoSignals.languages.frameworkReferences]]
  frameworkId = "fastapi"
  groupId = "fastapi"                  # 匹配 [[routes.groups]] id
  surface = "backend"                  # backend | frontend

  [[repoSignals.languages.frameworkReferences.items]]
  itemId = "testing"
  when = "task_owns:test_implementation"

  [[repoSignals.languages.frameworkReferences.items]]
  itemId = "routing"
  when = "task_owns:api_contract"

  [[repoSignals.languages.frameworkReferences.items]]
  itemId = "schemas"
  when = "task_owns:api_contract"

  [[repoSignals.languages.frameworkReferences.items]]
  itemId = "data"
  [items.when]
  allOf = ["task_owns:persistence", "stack_fw:sqlalchemy"]

  [[repoSignals.languages.frameworkReferences.items]]
  itemId = "security"
  [items.when]
  anyOf = ["context:security", "task_action:ImplementAuthenticationOrAuthorization"]

  [[repoSignals.languages.frameworkReferences.items]]
  itemId = "migration"
  when = "task_action:MigrateFrameworkImplementation"

  [[repoSignals.languages.frameworkReferences.items]]
  itemId = "logging"
  when = "task_owns:logging_infrastructure"
```

### 设计决策

1. **四种检测规则独立可选**:每种语言可只声明需要的规则类型,缺省段不触发该类检测。
2. **manifest 规则的 `path` 是相对项目根的文件名或路径**;命中即插入 `manifests` + `languages` + 可选 `packageManager`。
3. **frameworks 关键词匹配的文件范围** = 该语言已命中的 manifest 文件内容。引擎对每个 matched manifest:先尝试 `serde_json::from_str` 取 `dependencies`/`devDependencies`/`peerDependencies` 键名(Node 路径),若非 JSON 或无依赖段则回退到文本 `contains` 匹配(Python 路径)。统一为一条代码路径,无需特例分支。
4. **extensions 扫描复用 `skipDirs` + `maxScanDepth`**,与 `collect_package_manifests`(`execution/browser.rs:153`)的跳过目录模式对齐。
5. **`threshold` 防误报**:扩展名扫描要求至少 N 个文件才补强语言信号,避免仅有单个 `.py` 配置文件的误判。
6. **`selection.excludeKeywords`**:语言互斥规则。例如 Java 排除 Kotlin:`excludeKeywords = ["kotlin", "ktor", "android", "kmp", "mybatis flex"]`。

### Node/Java/Rust/Go 迁移

Node/Java/Rust/Go 同样迁移为 `[[repoSignals.languages]]` 条目,规则与现有硬编码逻辑一一对应:

- **Node**:manifests=[package.json],packageManagers=[npm/pnpm/yarn(按 lockfile)],框架关键词=[next, react, vue, svelte, vite, express, fastify, @nestjs/core]。Node 的框架匹配通过 JSON 依赖键名集合判断(引擎统一路径)。
- **Java**:manifests=[pom.xml, build.gradle, build.gradle.kts],框架关键词=[spring-boot→Spring Boot]。`excludeKeywords = ["kotlin", "ktor", "android", "kmp", "mybatis flex"]`。
- **Rust**:manifests=[Cargo.toml],无 frameworks 关键词检测。
- **Go**:manifests=[go.mod],无 frameworks 关键词检测。

## Rust Schema 与引擎实现

### `reference-catalog/schema.rs` 新增类型

```rust
/// 仓库技术栈检测规则配置。由 catalog.toml [repoSignals] 段填充。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RepoSignalsConfig {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skip_dirs: Vec<String>,

    #[serde(default)]
    pub max_scan_depth: u32,

    #[serde(default)]
    pub source_roots: SourceRootsConfig,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub languages: Vec<LanguageRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SourceRootsConfig {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LanguageRule {
    pub id: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub manifests: Vec<ManifestRule>,
    #[serde(default)]
    pub folder_hints: Option<FolderHintsRule>,
    #[serde(default)]
    pub extensions: Option<ExtensionsRule>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub frameworks: Vec<FrameworkRule>,
    #[serde(default)]
    pub selection: Option<SelectionRule>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub framework_references: Vec<FrameworkReference>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestRule {
    pub path: String,
    pub package_manager: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderHintsRule {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dirs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub require_extensions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionsRule {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scan: Vec<String>,
    #[serde(default)]
    pub threshold: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameworkRule {
    pub needle: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectionRule {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keywords: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exclude_keywords: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub frameworks: Vec<FrameworkSelectionRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameworkSelectionRule {
    pub id: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameworkReference {
    pub framework_id: String,
    pub group_id: String,
    pub surface: String, // "backend" | "frontend"
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<ReferenceItemRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceItemRule {
    pub item_id: String,
    #[serde(default)]
    pub when: Option<Condition>,
}
```

`ReferenceCatalog` 增加字段:

```rust
pub struct ReferenceCatalog {
    // ...现有字段...
    #[serde(default, skip_serializing_if = "is_repo_signals_empty")]
    pub repo_signals: RepoSignalsConfig,
}
```

### `reference-catalog/merge.rs` — overlay 合并

`merge_catalogs` 增加对 `repo_signals` 的合并语义:

- **`skip_dirs` / `max_scan_depth`**:overlay 非默认值时整体替换。
- **`source_roots.paths`**:overlay 追加去重。
- **`languages`**:以 `id` 为键,overlay 整体替换同 id 语言规则(与现有 route/group 替换语义一致);新 id 追加。

### `planning/technical_baseline.rs` — 检测引擎

删除:`collect_node_signals`、`collect_java_signals`、`collect_python_signals`、`collect_rust_signals`、`collect_go_signals`、`package_dependencies`。

保留:`RepoSignalSummary` 结构及其 `to_json` 输出格式。

新增 `RepoSignalEngine`:

```rust
struct RepoSignalEngine<'a> {
    config: &'a RepoSignalsConfig,
}

impl<'a> RepoSignalEngine<'a> {
    fn collect(&self, project_root: &Path) -> RepoSignalSummary {
        let mut signals = RepoSignalSummary::default();
        for lang in &self.config.languages {
            self.collect_language(project_root, lang, &mut signals);
        }
        self.collect_source_roots(project_root, &mut signals);
        signals
    }

    fn collect_language(&self, root: &Path, rule: &LanguageRule, signals: &mut RepoSignalSummary) {
        let mut matched_manifests: Vec<PathBuf> = Vec::new();
        let mut language_hit = false;

        // 规则1: manifest 存在性
        for m in &rule.manifests {
            let path = root.join(&m.path);
            if path.is_file() {
                signals.manifests.insert(m.path.clone());
                if let Some(pm) = &m.package_manager {
                    signals.package_managers.insert(pm.clone());
                }
                matched_manifests.push(path);
                language_hit = true;
            }
        }

        // 规则2: 文件夹命名规范
        if let Some(fh) = &rule.folder_hints {
            if self.folder_matches(root, fh) {
                language_hit = true;
            }
        }

        // 规则3: 文件扩展名扫描
        if let Some(ext) = &rule.extensions {
            if self.extension_scan_hits(root, ext) {
                language_hit = true;
            }
        }

        if language_hit {
            signals.languages.insert(rule.label.clone());
        }

        // 规则4: 框架关键词(仅在有 manifest 命中时扫描内容)
        if !matched_manifests.is_empty() && !rule.frameworks.is_empty() {
            self.detect_frameworks(&matched_manifests, root, rule, signals);
        }
    }
}
```

`compact_repo_signals` 改写:

```rust
fn compact_repo_signals(project_root: &Path) -> Value {
    let catalog = reference_catalog::resolved_catalog();
    let engine = RepoSignalEngine { config: &catalog.repo_signals };
    engine.collect(project_root).to_json()
}
```

### Node 框架检测统一处理

引擎的 `detect_frameworks` 对每个 matched manifest 统一处理:

1. 尝试 `serde_json::from_str` → 取 `dependencies`/`devDependencies`/`peerDependencies` 键名 → `keys.contains(needle)`
2. 若非 JSON 或无依赖段 → 回退 `content.to_lowercase().contains(needle)`

这复刻了现有 `collect_node_signals`(`technical_baseline.rs:892-909`)和 `collect_python_signals`(`technical_baseline.rs:957-971`)的行为,统一为一条路径,无需特例分支。

## 结构化条件模型(方案 A)

### 类型定义(`contracts/condition.rs`,~40 行核心)

```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Condition {
    /// 单谓词字符串:"task_owns:test_implementation"
    Predicate(String),
    /// AND:全部为真
    All { all_of: Vec<Condition> },
    /// OR:任一为真
    Any { any_of: Vec<Condition> },
    /// NOT:取反
    Not { not: Box<Condition> },
}
```

### `ConditionContext`

```rust
pub struct ConditionContext<'a> {
    pub task: &'a TaskDefinition,
    pub context: &'a CodeReferenceTaskContext,
    pub stack_frameworks: &'a BTreeSet<String>,
    pub focus_tags: &'a [String],
    pub signal: &'a CodeStackSignal,
    pub current_track: &'a str,
}
```

### 谓词格式与分发表

谓词统一为 `namespace:value` 字符串:

| namespace | 示例 | 映射到 |
|-----------|------|--------|
| `task_owns` | `task_owns:test_implementation` | `task_owns_test_implementation(task)` |
| `task_action` | `task_action:ImplementAuthenticationOrAuthorization` | `task_has_action(task, ImplementationAction::X)` |
| `task_kind` | `task_kind:ConfigurationSupport` | `task.task_kind == TaskKind::X` |
| `context` | `context:security` | `context.security` |
| `stack_fw` | `stack_fw:sqlalchemy` | `stack_frameworks.contains("X")` |
| `focus` | `focus:frontend` | `focus_tags.contains("X")` |
| `fw` | `fw:nextjs` | `signal.frameworks.contains("X")` |
| `lang` | `lang:typescript` | `signal.language == Some("X")` |

`task_owns` 的 value 参数映射(完整 20 个):

| value | 函数 |
|-------|------|
| `test_implementation` | `task_owns_test_implementation` |
| `frontend_implementation` | `task_owns_frontend_implementation` |
| `frontend_surface` | `task_owns_frontend_surface` |
| `api_contract` | `task_owns_api_contract` |
| `persistence` | `task_owns_persistence` |
| `logging_infrastructure` | `task_owns_logging_infrastructure` |
| `sql_schema` | `task_owns_sql_schema` |
| `sql_query` | `task_owns_sql_query` |
| `sql_transaction` | `task_owns_sql_transaction` |
| `sql_performance` | `task_owns_sql_performance` |
| `sql_analytics` | `task_owns_sql_analytics` |
| `sql_tests` | `task_owns_sql_tests` |
| `nest_service_boundary` | `task_owns_nest_service_boundary` |
| `typescript_type_modeling` | `task_owns_typescript_type_modeling` |
| `typescript_configuration` | `task_owns_typescript_configuration` |
| `typescript_pattern` | `task_owns_typescript_pattern` |

未知 namespace/value → 求值器返回 `false` 并 `log::warn!`(配置错误可见但不 panic)。

### `evaluate` 实现

```rust
impl Condition {
    pub fn evaluate(&self, ctx: &ConditionContext) -> bool {
        match self {
            Condition::Predicate(p) => evaluate_predicate(p, ctx),
            Condition::All { all_of } => all_of.iter().all(|c| c.evaluate(ctx)),
            Condition::Any { any_of } => any_of.iter().any(|c| c.evaluate(ctx)),
            Condition::Not { not } => !not.evaluate(ctx),
        }
    }
}
```

### TOML 可读性验证(现有最复杂条件)

aspnetcore `runtime`(`code_quality.rs:1229-1248`,7 个 OR 分支):

```toml
[[items]]
itemId = "runtime"
[items.when]
anyOf = [
  "task_kind:ConfigurationSupport",
  "task_action:AddOrUpdateConfig",
  "task_action:ImplementRuntimeDeliveryContract",
  "context:observability",
  "context:resilience",
  "context:request_tracing",
]
```

react `testing` 带互斥(`code_quality.rs:1483`):

```toml
[[items]]
itemId = "testing"
[items.when]
allOf = ["task_owns:test_implementation", { not = "fw:nextjs" }]
```

## 引擎调用链(改后)

```
code_reference_selection_for_task_with_context()
  │
  ├─ code_stack_signals_from_baseline(stack)          [保留]
  │    └─ signal_from_selection() → 配置驱动
  │         遍历 catalog.repo_signals.languages[].selection
  │         haystack contains keywords && !excludeKeywords → 派生 language + frameworks + roles
  │
  ├─ for each signal:
  │    ├─ reference_items_for_signal()                [保留,语言通用 items]
  │    ├─ backend_reference_items_for_signal() → 配置驱动
  │    │    遍历 catalog.repo_signals.languages[].frameworkReferences
  │    │    匹配 frameworkId && surface=="backend"
  │    │    → 遍历 items → condition.evaluate(ctx) → 收集命中 itemId
  │    └─ frontend_reference_items_for_signal() → 配置驱动
  │         同上,匹配 surface=="frontend" && focus:frontend
  │
  └─ code_reference_load_plan(reference_groups)       [保留,调用 catalog.resolve_entry]
```

### roles 特殊规则

`push_backend_unless_persistence_track` 逻辑迁移为 `roles = ["backend_unless_persistence"]` 命名规则。引擎识别 `backend_unless_persistence` 前缀,检查 `ConditionContext.current_track != "persistence"` 时 push "backend"。其他 roles 规则(`frontend`/`backend`)直接 push。

## 迁移映射表

### ①检测层

| 现有函数 | → | TOML |
|----------|---|------|
| `collect_python_signals` | → | `id="python"`,manifests=[pyproject.toml, requirements.txt, setup.py, setup.cfg, Pipfile, poetry.lock],frameworks=[fastapi, django, flask, pyramid, sanic, tornado] |
| `collect_node_signals` | → | `id="node"`,manifests=[package.json],框架=[next→Next.js, react→React, vue→Vue, svelte→Svelte, vite→Vite, express→Express, fastify→Fastify, @nestjs/core→NestJS] |
| `collect_java_signals` | → | `id="java"`,manifests=[pom.xml, build.gradle, build.gradle.kts],frameworks=[spring-boot→Spring Boot] |
| `collect_rust_signals` | → | `id="rust"`,manifests=[Cargo.toml] |
| `collect_go_signals` | → | `id="go"`,manifests=[go.mod] |
| `source_roots` 硬编码 | → | `[repoSignals.sourceRoots] paths=[src,app,web,service,backend,frontend,tests]` |

### ③信号派生

`signal_from_selection`(`code_quality.rs:345-544`)的 12 个 if-else 语言分支(typescript/javascript/java/csharp/go/python/rust/flutter/kotlin/php/swift/cpp)→ `[[repoSignals.languages.selection]]`,每语言声明 `keywords`/`excludeKeywords`/`roles` + `[[selection.frameworks]] id+aliases`。

### ④参考选择

`backend_reference_items_for_signal`(`code_quality.rs:1126-1290`)6 个框架(spring_boot/mybatis_plus/django/fastapi/aspnet_core/nestjs) + `frontend_reference_items_for_signal`(`code_quality.rs:1421+`)4 个框架(nextjs/react/vue/svelte)→ `[[repoSignals.languages.frameworkReferences]]` + `[[items]] itemId+when`。

## 测试策略

### 新增测试文件

| 测试文件 | 覆盖 |
|----------|------|
| `tests/rust/contracts/condition.rs` | `Condition::evaluate` 单元测试:单谓词/allOf/anyOf/not/嵌套/未知谓词→false+warn |
| `tests/rust/reference-catalog/repo_signals.rs` | TOML 反序列化 + overlay 合并 + 等价性验证 |
| `tests/rust/planning/repo_signal_engine.rs` | `RepoSignalEngine` 各语言检测:manifest/folder/extension/framework |
| `tests/rust/mcp-server/submit_tools.rs`(增补) | 端到端:setup.py fixture → repoEvidence.signals 含 Python |

### 行为等价性回归测试

构造代表性 fixture 项目目录,验证迁移不引入行为变化:

1. `python-pyproject-fastapi/`(pyproject.toml + fastapi)
2. `python-setup-py/`(仅 setup.py)— 新增能力
3. `python-no-manifest-with-py-files/`(仅 .py 文件)— 新增能力
4. `node-react-ts/`(package.json + react + tsconfig.json)
5. `java-spring-maven/`(pom.xml + spring-boot)
6. `rust-cargo/`(Cargo.toml)
7. `go-mod/`(go.mod)
8. `multi-stack/`(package.json + requirements.txt)

### 验证命令

```bash
cargo test --manifest-path src/rust/Cargo.toml -p contracts --test condition
cargo test --manifest-path src/rust/Cargo.toml -p reference-catalog --test repo_signals
cargo test --manifest-path src/rust/Cargo.toml -p planning --test repo_signal_engine
cargo test --manifest-path src/rust/Cargo.toml -p mcp-server --test submit_tools
npm run rust:test
cargo fmt --manifest-path src/rust/Cargo.toml --all --check
cargo build --manifest-path src/rust/Cargo.toml -p mcp-server -p setup
```

## 风险与缓解

| 风险 | 缓解 |
|------|------|
| 迁移遗漏导致行为不等价 | parity 测试覆盖 8 个代表性 fixture |
| 条件 DSL 谓词映射表不完整 | 未知谓词 warn 日志 + 单元测试覆盖全部 20 个 task_owns + 47 个 task_action |
| TOML 配置体积增大 | 分语言组织,注释清晰;overlay 机制支持企业自定义 |
| Node JSON 依赖解析与现有行为细微差异 | parity 测试包含 node-react-ts fixture |
