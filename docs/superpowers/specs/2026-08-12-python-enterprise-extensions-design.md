# Python 企业扩展示例设计

## 概述

扩展现有 `examples/reference-catalog/` 示例,完整演示企业如何通过纯配置(TOML + `.md` 文件)
扩展 Python 的三层能力:

1. **Code 路由组操作**(编码规范 .md 文件)— 现有能力,已在示例中展示
2. **repoSignals 检测 + 信号派生**(内部框架检测)— 新增能力展示
3. **repoSignals 框架引用选择**(框架参考 item 条件加载)— 新增能力展示

## 场景

虚构企业 "Acme" 拥有内部自研的基于 Qt 的 Python 桌面 UI 框架 "AcmeQt",集成了日志和异常
处理。企业还有内部 Python 编码规范(强制 Ruff、测试用例规范)。

### 需要展示的扩展操作

| 扩展点 | 操作 | 说明 |
|--------|------|------|
| 检测规则 | 新增框架 needle | `acme-qt` 出现在依赖清单时检测为 AcmeQt |
| 信号派生 | 新增 selection framework | `acme_qt` 别名匹配,推送 "frontend" role |
| 框架引用 | 新增 frameworkReferences | AcmeQt 的 6 个参考 item 按条件加载 |
| 编码规范注入 | `lang:python` 框架级条件模式 | 企业规范 item 无需改 Rust 代码注入 Python 组 |
| Code 路由组 | 新增 `acme_qt` 组 | 6 个框架参考 .md 文件的路径模板 |
| Code 路由组 | 更新 `python` 组 | 新增 `ruff` + `use_case` item,`core` 走 itemEntries |

## 设计

### 1. AcmeQt 框架检测 + 信号派生

#### 检测层

在 Python 的 `[[repoSignals.languages.frameworks]]` 中新增:

```toml
[[repoSignals.languages.frameworks]]
needle = "acme-qt"
label = "AcmeQt"
```

当 `requirements.txt`/`pyproject.toml` 中出现 `acme-qt` 依赖时,`RepoSignalEngine` 检测到
AcmeQt 框架,写入 `signal.frameworks`。

#### 信号派生层

在 Python 的 `selection` 中新增:

```toml
[[repoSignals.languages.selection.frameworks]]
id = "acme_qt"
aliases = ["acme-qt", "acmeqt", "acme ui"]
```

Python 的 `roles` 从 `["backend_unless_persistence"]` 改为
`["backend_unless_persistence", "frontend_conditional"]`。当 selection 文本中出现 AcmeQt
别名时,引擎推送 "frontend" role(通过 `selection_mentions_frontend_framework` 检查)。

### 2. AcmeQt 框架参考引用

```toml
[[repoSignals.languages.frameworkReferences]]
frameworkId = "acme_qt"
groupId = "acme_qt"
surface = "frontend"

  [[repoSignals.languages.frameworkReferences.items]]
  itemId = "core"
  when = "task_owns:frontend_implementation"

  [[repoSignals.languages.frameworkReferences.items]]
  itemId = "widgets"
  when = "task_owns:frontend_surface"

  [[repoSignals.languages.frameworkReferences.items]]
  itemId = "signals_slots"
  when = "task_owns:frontend_implementation"

  [[repoSignals.languages.frameworkReferences.items]]
  itemId = "logging"
  when = "task_owns:logging_infrastructure"

  [[repoSignals.languages.frameworkReferences.items]]
  itemId = "exceptions"
  when = "task_owns:frontend_implementation"

  [[repoSignals.languages.frameworkReferences.items]]
  itemId = "testing"
  when = "task_owns:test_implementation"
```

6 个参考项。`surface = "frontend"` 使其走 `frontend_reference_items_for_signal` 路径,
需要 `focus:frontend` tag 才会加载。

### 3. 企业编码规范注入(lang:python 模式)

这是关键设计——用 `FrameworkReference.when = "lang:python"` 让编码规范 item 对所有
Python 任务生效,无需改 Rust 代码:

```toml
[[repoSignals.languages.frameworkReferences]]
frameworkId = "acme_python_standards"
groupId = "python"
surface = "backend"
when = "lang:python"

  [[repoSignals.languages.frameworkReferences.items]]
  itemId = "ruff"

  [[repoSignals.languages.frameworkReferences.items]]
  itemId = "use_case"
  when = "task_owns:test_implementation"
```

- `when = "lang:python"` — 框架级条件:当 `signal.language == "python"` 时匹配,无需检测
  到特定框架
- `groupId = "python"` — item 合并到 `reference_groups["python"]`,走 `code` 路由 `python`
  组的路径解析
- `surface = "backend"` — 走 `backend_reference_items_for_signal` 路径,不需要 `focus:frontend`
- `ruff` item 的 `when` 省略 — 无条件加载(企业强制 Ruff)
- `use_case` item 的 `when = "task_owns:test_implementation"` — 仅测试任务时加载

### 4. Code 路由组

```toml
# python 组:新增 ruff + use_case,core 走 itemEntries
[[routes.groups]]
id = "python"
refIdTemplate = "tech.code.python.{item}"
pathTemplate = "tech/code/python/{item}.md"
reasonTemplate = "已选择 {group_id}.{item} 的实现质量参考,用于本任务。"
items = ["typing", "async", "testing", "ruff", "use_case"]

  [[routes.groups.itemEntries]]
  item = "core"
  refId = "tech.code.python.core"
  path = "enterprise/python/core.md"
  reason = "Acme 企业 Python 核心规范,强制 Ruff 与 strict 类型检查。"
  condition = "always"

# acme_qt 组:新增前端框架组
[[routes.groups]]
id = "acme_qt"
refIdTemplate = "fe.acme_qt.{item}"
pathTemplate = "tech/frontend/acme_qt/{item}.md"
reasonTemplate = "已选择 {label} {item} 的前端框架质量参考,用于本任务。"
label = "AcmeQt"
items = ["core", "widgets", "signals_slots", "logging", "exceptions", "testing"]
```

### 5. 新增 .md 参考文件

| 文件 | 内容 |
|------|------|
| `tech/frontend/acme_qt/core.md` | AcmeQt 应用生命周期、主循环、窗口管理 |
| `tech/frontend/acme_qt/widgets.md` | Widget 创建、布局、样式 |
| `tech/frontend/acme_qt/signals_slots.md` | 信号/槽连接模式、线程安全 |
| `tech/frontend/acme_qt/logging.md` | 集成日志框架使用 |
| `tech/frontend/acme_qt/exceptions.md` | 异常处理模式、错误恢复 |
| `tech/frontend/acme_qt/testing.md` | UI 测试模式 |
| `tech/code/python/use_case.md` | 测试用例规范:命名、结构、覆盖要求 |

每个文件遵循 section 契约:`## When To Use` / `## Implementation Focus` /
`## Verification Focus` / `## Evidence Focus`。

### 6. Overlay 完整结构

overlay TOML 分为两大段:

1. **`[[routes]]`** — code 路由组操作(goa 新增、java 替换、python 替换+itemEntries、acme_qt 新增)
2. **`[repoSignals]`** — 完整 Python 语言规则(复制 vendor 全部 Python 检测/选择/框架引用 +
   新增 AcmeQt + acme_python_standards)

**重要约束**:`merge_repo_signals` 按 ID 整体替换语言规则,overlay 必须包含完整 Python 语言
规则(含所有 vendor 的 manifests/folderHints/extensions/frameworks/selection/frameworkReferences)。

### 7. README.md 更新

新增以下内容:
- `[repoSignals]` 扩展说明:如何在 overlay 中添加内部框架检测/选择/引用
- `lang:python` 模式文档:如何用框架级条件注入企业编码规范 item
- overlay 复制约束提示:`merge_repo_signals` 按 ID 整体替换,必须复制完整语言规则

## 验证

- overlay TOML 能被 `parse_catalog` 正确解析
- `merge_catalogs(vendor, overlay)` 合并后 Python 语言规则包含 AcmeQt + acme_python_standards
- `code` 路由包含 `acme_qt` 组
- `python` 组 items 包含 `ruff` + `use_case`
- 现有 reference-catalog 测试仍通过

## 不在范围内

- 不修改 vendor `catalog.toml` — 所有扩展只在 overlay 中
- 不修改 Rust 代码 — 纯配置 + .md 文件
- 不添加 `reference_items_for_signal` 中的新 item — 使用 `lang:python` frameworkReferences 模式绕过
- 不修改 `focus_rules`/`applicability`/`backend_ecosystems` — 这些字段 overlay 整体替换,不在本示例展示
