# Reference Catalog 企业扩展配置案例

本目录是一个**纯配置**的 reference catalog 企业扩展案例。企业只需 TOML + `.md` 文件
+ 一个环境变量,无需编写任何 Rust 代码。

## 目录结构

```
examples/reference-catalog/
├── README.md                                  # 本文件(完整接入指南)
├── enterprise-overlay.toml                    # enterprise overlay 声明(code 路由 + repoSignals)
└── references/                                # 示例企业参考文件
    ├── tech/backend/goa/core.md               # 新增的 Goa 框架核心参考
    ├── tech/code/python/ruff.md               # 新增的 Ruff lint/格式化参考
    ├── tech/code/python/use_case.md           # 新增的测试用例规范参考
    ├── tech/frontend/acme_qt/                 # 新增的 AcmeQt 框架参考(6 个文件)
    │   ├── core.md
    │   ├── widgets.md
    │   ├── signals_slots.md
    │   ├── logging.md
    │   ├── exceptions.md
    │   └── testing.md
    ├── enterprise/java/core.md                # 改写的企业 Java 核心参考
    └── enterprise/python/core.md              # 改写的企业 Python 核心参考
```

## 案例场景

虚构企业 "Acme" 的技术栈策略,展示两类扩展:

### Code 路由组操作(参考文件路径解析)

| 改法 | 操作 | 说明 |
|------|------|------|
| 新增组 | `goa` 组 | 企业自研 Go web 框架,vendor 中没有 |
| 替换组 | `java` 组 | 不用 Spring Security/Reactive,改用 Quarkus |
| 替换组 + 显式条目 | `python` 组 | 去掉 packaging、新增 ruff + use_case、core 指向企业规范 |
| 新增组 | `acme_qt` 组 | 企业自研基于 Qt 的 Python 桌面 UI 框架 |

### repoSignals 扩展(检测/信号派生/框架引用选择)

| 扩展点 | 操作 | 说明 |
|--------|------|------|
| 检测规则 | 新增 `acme-qt` needle | 依赖清单中出现 `acme-qt` 时检测为 AcmeQt 框架 |
| 信号派生 | 新增 `acme_qt` selection framework | selection 文本匹配 AcmeQt 别名,推送 "frontend" role |
| 框架引用 | 新增 AcmeQt frameworkReferences | 6 个参考 item 按条件加载(testing/logging/...) |
| 编码规范注入 | `lang:python` 框架级条件模式 | 企业规范 item 无需改 Rust 代码注入 Python 组 |

Python 案例最典型——很多企业 Python 规范与 vendor 默认不同:
- 强制 Ruff 替代 black/isort/flake8
- 强制 strict 类型检查(`mypy --strict`)
- 不用 packaging(企业统一用 `uv`)
- `core` 改写为指向企业内部规范文件
- 新增测试用例规范(`use_case`)

## 企业接入清单(3 步,零代码)

### 1. 编写 enterprise overlay TOML

复制 `enterprise-overlay.toml`,按企业技术栈改写。包含两大段:

#### 第一部分:Code 路由组操作

**新增组** — 追加 `[[routes.groups]]`:

```toml
[[routes.groups]]
id = "acme_qt"
refIdTemplate = "fe.acme_qt.{item}"
pathTemplate = "tech/frontend/acme_qt/{item}.md"
label = "AcmeQt"
items = ["core", "widgets", "signals_slots", "logging", "exceptions", "testing"]
```

**替换组** — 与 vendor 同 `id` 的组,items 列表整体覆盖:

```toml
[[routes.groups]]
id = "python"
items = ["typing", "async", "testing", "ruff", "use_case"]
```

**替换组 + 显式条目** — items 走模板路径,部分 item 用 `itemEntries` 指向企业内部路径:

```toml
[[routes.groups]]
id = "python"
items = ["typing", "async", "testing", "ruff", "use_case"]

  [[routes.groups.itemEntries]]
  item = "core"
  refId = "tech.code.python.core"
  path = "enterprise/python/core.md"
  reason = "Acme 企业 Python 核心规范,强制 Ruff 与 strict 类型检查。"
  condition = "always"
```

> 一个组可以同时有 `items`(模板路径)和 `itemEntries`(显式路径)。
> 这让企业在不放弃模板便利的前提下,对个别 item 做精确路径覆盖。

#### 第二部分:[repoSignals] 扩展

`[repoSignals]` 控制三层行为:
- **检测**(manifests/folderHints/extensions/frameworks)— 扫描仓库识别语言和框架
- **信号派生**(selection)— 从 selection 文本推导语言/frameworks/roles
- **框架引用选择**(frameworkReferences)— 按条件加载框架参考 item

**重要约束**:`merge_repo_signals` 按 language ID 整体替换语言规则。overlay 必须包含
**完整** Python 语言规则(复制 vendor 全部 Python 检测/选择/框架引用,再叠加企业扩展)。
不能只写增量。

**新增内部框架检测** — 在 `[[repoSignals.languages.frameworks]]` 中追加 needle:

```toml
[[repoSignals.languages.frameworks]]
needle = "acme-qt"
label = "AcmeQt"
```

**新增信号派生规则** — 在 `selection` 中追加 framework 别名,更新 roles:

```toml
[repoSignals.languages.selection]
keywords = ["python", "fastapi", ..., "acmeqt"]   # 新增 acmeqt
roles = ["backend_unless_persistence", "frontend_conditional"]  # 新增 frontend_conditional

[[repoSignals.languages.selection.frameworks]]
id = "acme_qt"
aliases = ["acme-qt", "acmeqt", "acme ui"]
```

**新增框架引用选择** — 声明 AcmeQt 的参考 item 及加载条件:

```toml
[[repoSignals.languages.frameworkReferences]]
frameworkId = "acme_qt"
groupId = "acme_qt"
surface = "frontend"

  [[repoSignals.languages.frameworkReferences.items]]
  itemId = "core"
  when = "task_owns:frontend_implementation"

  [[repoSignals.languages.frameworkReferences.items]]
  itemId = "testing"
  when = "task_owns:test_implementation"
```

**`lang:python` 模式 — 注入企业编码规范**(无需改 Rust 代码):

```toml
[[repoSignals.languages.frameworkReferences]]
frameworkId = "acme_python_standards"
groupId = "python"           # 合并到 python 组,走 python 组路径解析
surface = "backend"          # 走 backend 路径,不需要 focus:frontend
when = "lang:python"         # 框架级条件:所有 Python 任务生效

  [[repoSignals.languages.frameworkReferences.items]]
  itemId = "ruff"             # when 省略 → 无条件加载

  [[repoSignals.languages.frameworkReferences.items]]
  itemId = "use_case"
  when = "task_owns:test_implementation"
```

这个模式让企业编码规范 item(如 `ruff`、`use_case`)自动合并到 `reference_groups["python"]`,
通过 `code` 路由 `python` 组的路径模板解析为 `.md` 文件路径,**无需修改
`reference_items_for_signal` 硬编码函数**。

### 2. 创建企业参考 `.md` 文件

参考文件需遵循 section 契约(与 vendor 参考文件一致):

- `## When To Use` — 何时使用此参考
- `## Implementation Focus` — 实现关注点
- `## Verification Focus` — 验证关注点
- `## Evidence Focus` — 证据关注点

参考 `references/` 目录下的示例文件。

### 3. 设置环境变量

```bash
export LOOM_CATALOG_OVERLAY=/opt/acme/loom/enterprise-overlay.toml
```

启动 Loom 即可。所有选择代码路径通过 `resolved_catalog()` 自动获得合并后的目录。

## resolved_catalog() 行为

| 环境变量状态 | 行为 |
|-------------|------|
| 未设置 | 静默回退 vendor 基线 |
| 路径不存在 | 静默回退 vendor 基线 |
| 路径存在且合法 | 加载 overlay + 合并 + 缓存 |
| overlay 解析/合并失败 | 启动时报错(配置错误尽早暴露) |

## 操作速查

### Code 路由组操作

| 操作 | 方式 | 效果 |
|------|------|------|
| 新增组 | overlay TOML 追加 `[[routes.groups]]` | 新组追加到路由末尾 |
| 替换组 | overlay TOML 同 `id` 的组 | items/模板/label 整体覆盖 vendor 组 |
| 改写单个 item | 组内 `[[routes.groups.itemEntries]]` | 指定 item 走显式路径,覆盖模板路径 |

> **裁剪组**(prune)是函数操作,不在 TOML 中表达。
> 如需使用,参考 `docs/reference-catalog-extension.md` 的「程序化操作」章节。

### repoSignals 扩展操作

| 操作 | 方式 | 效果 |
|------|------|------|
| 新增框架检测 | `[[repoSignals.languages.frameworks]]` 追加 needle | 依赖清单匹配时检测到框架 |
| 新增信号派生 | `selection.frameworks` 追加 id + aliases | selection 文本匹配时推送 framework |
| 新增框架引用 | `[[repoSignals.languages.frameworkReferences]]` | 框架匹配时按条件加载参考 item |
| 注入编码规范 | frameworkReferences + `when = "lang:{lang}"` | 所有目标语言任务自动加载规范 item |

**关键约束**:`merge_repo_signals` 按 language ID 整体替换。overlay 必须包含完整语言规则
(复制 vendor + 叠加扩展),不能只写增量字段。

**`lang:{lang}` 模式**:用框架级 `when = "lang:python"` 条件让 `frameworkId` 不需要真正
被检测到——只要 `signal.language == "python"` 就匹配。配合 `groupId = "python"` 和
`surface = "backend"`,编码规范 item 自动注入 Python 组,无需改 Rust 代码。

## 配套资源

- 契约与语义详解:`docs/reference-catalog-extension.md`
- 设计文档:`docs/superpowers/specs/2026-08-12-python-enterprise-extensions-design.md`
- 集成测试:`tests/rust/reference-catalog/resolved.rs`、`enterprise_overlay.rs`
- 目录 schema:`src/rust/reference-catalog/schema.rs`
- vendor 基线:`plugins/shared/loom/references/catalog.toml`
