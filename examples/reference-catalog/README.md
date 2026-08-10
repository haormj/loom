# Reference Catalog 企业扩展配置案例

本目录是一个**纯配置**的 reference catalog 企业扩展案例。企业只需 TOML + `.md` 文件
+ 一个环境变量,无需编写任何 Rust 代码。

## 目录结构

```
examples/reference-catalog/
├── README.md                                # 本文件(完整接入指南)
├── enterprise-overlay.toml                  # enterprise overlay 声明
└── references/                              # 示例企业参考文件
    ├── tech/backend/goa/core.md             # 新增的 Goa 框架核心参考
    ├── tech/code/python/ruff.md             # 新增的 Ruff lint/格式化参考
    ├── enterprise/java/core.md              # 改写的企业 Java 核心参考
    └── enterprise/python/core.md            # 改写的企业 Python 核心参考
```

## 案例场景

虚构企业 "Acme" 的技术栈策略(对 vendor 默认的三种改法):

| 改法 | 操作 | 说明 |
|------|------|------|
| 新增组 | `goa` 组 | 企业自研 Go web 框架,vendor 中没有 |
| 替换组 | `java` 组 | 不用 Spring Security/Reactive,改用 Quarkus |
| 替换组 + 显式条目 | `python` 组 | 去掉 packaging、新增 ruff、core 指向企业规范 |

Python 案例最典型——很多企业 Python 规范与 vendor 默认不同:
- 强制 Ruff 替代 black/isort/flake8
- 强制 strict 类型检查(`mypy --strict`)
- 不用 packaging(企业统一用 `uv`)
- `core` 改写为指向企业内部规范文件

## 企业接入清单(3 步,零代码)

### 1. 编写 enterprise overlay TOML

复制 `enterprise-overlay.toml`,按企业技术栈改写。三种操作:

**新增组** — 追加 `[[routes.groups]]`:

```toml
[[routes.groups]]
id = "goa"
refIdTemplate = "bk.goa.{item}"
pathTemplate = "tech/backend/goa/{item}.md"
items = ["core", "routing", "middleware", "testing"]
```

**替换组** — 与 vendor 同 `id` 的组,items 列表整体覆盖:

```toml
[[routes.groups]]
id = "java"
refIdTemplate = "tech.code.java.{item}"
pathTemplate = "tech/code/java/{item}.md"
items = ["core", "spring", "persistence", "testing", "quarkus"]
```

**替换组 + 显式条目** — items 走模板路径,部分 item 用 `itemEntries` 指向企业内部路径:

```toml
[[routes.groups]]
id = "python"
refIdTemplate = "tech.code.python.{item}"
pathTemplate = "tech/code/python/{item}.md"
items = ["typing", "async", "testing", "ruff"]   # core 移到 itemEntries

  [[routes.groups.itemEntries]]
  item = "core"
  refId = "tech.code.python.core"
  path = "enterprise/python/core.md"               # 指向企业内部规范
  reason = "Acme 企业 Python 核心规范,强制 Ruff 与 strict 类型检查。"
  condition = "always"
```

> 一个组可以同时有 `items`(模板路径)和 `itemEntries`(显式路径)。
> 这让企业在不放弃模板便利的前提下,对个别 item 做精确路径覆盖。

### 2. 创建企业参考 `.md` 文件

参考文件需遵循 section 契约(与 vendor 参考文件一致):

- `## When To Use` — 何时使用此参考
- `## Implementation Focus` — 实现关注点
- `## Verification Focus` — 验证关注点
- `## Evidence Focus` — 证据关注点

参考 `references/` 目录下的四个示例文件。

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

## 三种操作速查

| 操作 | 方式 | 效果 |
|------|------|------|
| 新增组 | overlay TOML 追加 `[[routes.groups]]` | 新组追加到路由末尾 |
| 替换组 | overlay TOML 同 `id` 的组 | items/模板/label 整体覆盖 vendor 组 |
| 改写单个 item | 组内 `[[routes.groups.itemEntries]]` | 指定 item 走显式路径,覆盖模板路径 |

> **裁剪组**(prune)是函数操作,不在 TOML 中表达。
> 如需使用,参考 `docs/reference-catalog-extension.md` 的「程序化操作」章节。

## 配套资源

- 契约与语义详解:`docs/reference-catalog-extension.md`
- 集成测试:`tests/rust/reference-catalog/resolved.rs`、`enterprise_overlay.rs`
- 目录 schema:`src/rust/reference-catalog/schema.rs`
- vendor 基线:`plugins/shared/loom/references/catalog.toml`
