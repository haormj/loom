# Reference Catalog 业务扩展指南

## 概述

Loom 参考目录(reference catalog)是参考路由(route)、组(group)、条目(item)、
focus 关键词规则、适用性规则与后端生态系统定义的声明式清单。选择代码路径
(contracts、architecture、deploy、planning)通过此目录查询应加载哪些参考文件。

目录以 TOML 表达,顶层模型见 `src/rust/reference-catalog/schema.rs`。

### 三层模型

| 层 | 来源 | 作用 |
|----|------|------|
| vendor | `plugins/shared/loom/references/catalog.toml` | 仓库内置基线,随 Loom 发布 |
| enterprise | 企业级 overlay(后续接入) | 在 vendor 之上叠加企业私有组、裁剪不用的组、改写 item 路径 |
| project | 项目级 overlay(后续接入) | 在 enterprise 之上再做项目级微调 |

叠加通过 `merge_catalogs` 完成;`prune_groups`、`replace_item_entry` 提供更细粒度的
overlay 操作。三者均为 `src/rust/reference-catalog/merge.rs` 导出的纯函数。

### 当前状态

- `merge_catalogs`、`prune_groups`、`replace_item_entry` **已实现并具备测试**
  (`tests/rust/reference-catalog/merge.rs`、`enterprise_overlay.rs`)。
- **运行时已接入**:所有选择代码路径(contracts、architecture、deploy、planning)
  调用 `reference_catalog::resolved_catalog()`,它自动发现环境变量
  `LOOM_CATALOG_OVERLAY` 指向的 enterprise overlay 并与 vendor 基线合并。
- 企业只需设置环境变量 + 放置 overlay TOML,即可让扩展自动生效,无需改 Loom 代码。
- `vendor_catalog()` 仍保留,但仅返回 vendor 基线,供测试与对比使用。
- prune / replace item 操作不在 TOML 中表达,需在合并后调用函数完成
  (见下文"程序化操作")。

## 目录契约速览

完整模型见 `schema.rs`,此处只列扩展时常用字段。

```toml
schemaVersion = 1

[[routes]]
id = "code"                      # 路由 id:code/api/arch/uix/browser/deploy
referenceRoot = "loom"            # loom → plugins/shared/loom/references/
                                  # loom-deploy → plugins/shared/loom-deploy/references/

  # 路由级前置项:选中任意组时加载(受 condition 控制)
  [[routes.prependItems]]
  refId = "tech.code.common"
  path = "tech/code/common.md"
  reason = "..."
  condition = "when_groups_non_empty"

  # 基于模板的组:{item} 替换为每个 item id
  [[routes.groups]]
  id = "java"
  refIdTemplate = "tech.code.java.{item}"
  pathTemplate = "tech/code/java/{item}.md"
  reasonTemplate = "已选择 {group_id}.{item} 的实现质量参考,用于本任务。"
  items = ["core", "spring", "persistence", "security", "reactive", "testing"]

  # 显式逐项条目:非标准 refId/path 映射
  [[routes.groups]]
  id = "common"
    [[routes.groups.itemEntries]]
    item = "observability"
    refId = "tech.code.observability"
    path = "tech/code/observability.md"

  # 两级组(provider overlay,如 SQL 方言)
  [[routes.groups]]
  id = "sql"
  refIdTemplate = "tech.code.sql.{item}"
  pathTemplate = "tech/code/sql/{item}.md"
  items = ["dialects", "optimization", "queries", "schema", "windows"]
    [[routes.groups.providers]]
    id = "mysql"
    refIdTemplate = "tech.code.sql.mysql.{subject}"
    pathTemplate = "tech/code/sql/mysql/{subject}.md"
    subjects = ["schema", "queries", "transactions"]
    label = "MySQL"
    reasonTemplate = "已选择 {label} {subject} 的方言参考,用于本持久化任务。"

  # 架构 section → items 映射(仅 arch 路由)
  [[routes.sectionGroups]]
  id = "Foundation"
  items = ["core", "patterns", "system"]
```

`reasonTemplate` 支持占位符 `{label}`、`{item}`、`{group_id}`、`{subject}`。

focus 关键词规则(`focusRules`)、适用性规则(`applicability`)、后端生态系统
(`backendEcosystems`)也同属目录顶层,扩展方式与 routes 一致(overlay 中新增即追加,
同 id 路由内的组按下列规则合并)。注意:`merge_catalogs` 当前对 `focus_rules`、
`applicability`、`backend_ecosystems` 采取 **overlay 整体替换 base** 的策略
(见 `merge.rs:32-34`,`merged = base.clone()` 后仅 `routes` 被重算),因此在这三类
上做增量扩展时,overlay 必须包含完整列表。后续将按字段做细粒度合并。

## 运行时接入:enterprise overlay 自动生效

### 接入步骤

1. 编写 enterprise overlay TOML(参考下文案例或 `examples/reference-catalog/enterprise-overlay.toml`)。
2. 创建企业参考 `.md` 文件(放在企业自管目录)。
3. 设置环境变量 `LOOM_CATALOG_OVERLAY` 指向 overlay 文件路径:
   ```bash
   export LOOM_CATALOG_OVERLAY=/opt/acme/loom/enterprise-overlay.toml
   ```
4. 启动 Loom。所有选择代码路径通过 `resolved_catalog()` 自动获得合并后的目录。

### resolved_catalog() 行为

- 环境变量**未设置**或路径**不存在** → 静默回退到 vendor 基线(零配置兼容)。
- 环境变量指向的文件**存在且合法** → 加载 overlay,通过 `merge_catalogs` 合并到 vendor 之上,缓存到进程级 `OnceLock`。
- overlay **解析或合并失败** → panic(企业配置错误应尽早暴露,而非静默降级)。
- 企业可在部署前用 `validate_structure` 自行校验 overlay 完整性。

### 程序化操作:prune 与 replace item

TOML 只能表达"新增组"和"整组替换"。如需裁剪组(禁用某语言)或改写单个 item 的路径,
需在合并后调用函数完成。企业可在 CI 校验脚本或启动钩子中执行:

```rust
use reference_catalog::{
    load_vendor_catalog, merge_catalogs, parse_catalog, prune_groups,
    replace_item_entry, validate_structure, ItemEntry,
};

let base = load_vendor_catalog().expect("vendor catalog loads");
let overlay = parse_catalog(&overlay_toml, "enterprise.toml").expect("overlay parses");
let mut merged = merge_catalogs(&base, &overlay).expect("merge ok");

// 裁剪 swift 组
prune_groups(&mut merged, "code", &["swift".to_string()]);

// 改写 java.core 指向企业内部参考文件
replace_item_entry(
    &mut merged, "code", "java",
    ItemEntry {
        item: "core".to_string(),
        ref_id: "tech.code.java.core".to_string(),
        path: "enterprise/java/core.md".to_string(),
        reason: Some("Acme 企业 Java 核心规范".to_string()),
        condition: None,
    },
).expect("replace ok");

validate_structure(&merged).expect("structurally valid");
```

> **注意**:prune / replace 的结果不会自动写回 `resolved_catalog()` 的缓存。
> 若需让这些操作在运行时生效,需将合并后的目录注入到 `resolved_catalog` 的缓存中,
> 或在 overlay TOML 中用"整组替换"模拟 prune 效果(如把 swift 的 items 列表设为空)。
> 后续将支持在 TOML 中声明 prune 指令。

## 三种 overlay 操作语义

| 操作 | 函数 | 效果 |
|------|------|------|
| 合并 | `merge_catalogs(base, overlay)` | 同 id 路由内的组按下列规则合并;不同 id 路由追加 |
| 裁剪 | `prune_groups(catalog, route, groups)` | 从指定路由移除列出的组 |
| 替换 item | `replace_item_entry(catalog, route, group, entry)` | 把模板 item 转为显式 `itemEntries`,覆盖 refId/path/reason |

### merge_catalogs 路由内合并规则(merge.rs:41-75)

- **组**:以 `id` 为键。overlay 中同 id 的组**整体替换** base 组(含 items、itemEntries、
  providers、模板、label、reasonTemplate、requiredSections)。overlay 中的新组追加到末尾。
- **prependItems**:按 `refId` 去重,overlay 项覆盖同 refId 的 base 项。
- **sectionGroups**:overlay 非空时整体替换 base 的 sectionGroups。

### prune_groups

按 group id 从路由的 `groups` 列表中 `retain`(保留不在删除列表中的组)。不动
prependItems / sectionGroups。

### replace_item_entry(merge.rs:89-135)

在指定组内查找 item:
- 若组有同 item 的 `itemEntries` → 直接替换该条目。
- 否则若 item 在模板 `items` 列表中 → 从 `items` 移除,并 push 一条新的 `itemEntries`。
- 都找不到 → 返回 `CatalogError::Merge`。

效果:把一个原本走模板路径的 item 改成走显式路径,从而可指向企业内部的参考文件。

## 完整企业案例:Acme 的 code 路由 overlay

虚构企业 "Acme" 使用 Loom,其技术栈策略:
- 内部 Go web 服务使用自研框架 Goa,需要新增 `goa` 参考组(模板路径
  `tech/backend/goa/{item}.md`)。
- Java 后端不使用 Spring Security / Reactive,改用 Quarkus;`java` 组需改写为
  `["core", "spring", "persistence", "testing", "quarkus"]`。
- 不做 iOS,移除 `swift` 组。
- Java `core` 参考需指向企业自有的 `enterprise/java/core.md`,并附企业 reason。

### Enterprise overlay TOML

```toml
schemaVersion = 1

[[routes]]
id = "code"
referenceRoot = "loom"

  # (a) 新增组:Goa 框架
  [[routes.groups]]
  id = "goa"
  refIdTemplate = "bk.goa.{item}"
  pathTemplate = "tech/backend/goa/{item}.md"
  reasonTemplate = "已选择 {label} {item} 的框架质量参考,用于本任务。"
  label = "Goa"
  items = ["core", "routing", "middleware", "testing"]

  # (b) 替换组:Java 改用 Quarkus,去掉 security/reactive
  [[routes.groups]]
  id = "java"
  refIdTemplate = "tech.code.java.{item}"
  pathTemplate = "tech/code/java/{item}.md"
  reasonTemplate = "已选择 {group_id}.{item} 的实现质量参考,用于本任务。"
  items = ["core", "spring", "persistence", "testing", "quarkus"]
```

> (c) prune `swift` 与 (d) replace `java.core` 不写在 TOML 里——它们是 overlay 操作,
> 在合并后对结果目录调用 `prune_groups` / `replace_item_entry` 完成。这是当前 API 的
> 约定:TOML 表达"新增/整组替换",函数表达"裁剪/单 item 改写"。

### Rust 调用顺序

```rust
use reference_catalog::{
    load_vendor_catalog, merge_catalogs, parse_catalog, prune_groups,
    replace_item_entry, validate_structure, ItemEntry,
};

let base = load_vendor_catalog().expect("vendor catalog loads");
let overlay = parse_catalog(ENTERPRISE_OVERLAY_TOML, "enterprise.toml")
    .expect("enterprise overlay parses");

// 1. vendor + enterprise 合并(新增 goa、替换 java)
let mut merged = merge_catalogs(&base, &overlay).expect("merge ok");

// 2. 裁剪 swift
prune_groups(&mut merged, "code", &["swift".to_string()]);

// 3. 改写 java.core 指向企业内部参考文件
replace_item_entry(
    &mut merged,
    "code",
    "java",
    ItemEntry {
        item: "core".to_string(),
        ref_id: "tech.code.java.core".to_string(),
        path: "enterprise/java/core.md".to_string(),
        reason: Some("Acme 企业 Java 核心规范,覆盖内部编码与分层约定。".to_string()),
        condition: None,
    },
).expect("replace ok");

// 4. 结构校验:refId 唯一、组非空
validate_structure(&merged).expect("merged catalog structurally valid");
```

### 预期结果

合并后的 `code` 路由:
- `goa` 组存在,items = `[core, routing, middleware, testing]`,展开后 path 为
  `tech/backend/goa/{item}.md`、refId 为 `bk.goa.{item}`。
- `java` 组 items = `[core, spring, persistence, testing, quarkus]`;其中 `core` 已从
  模板 items 移除并转入 `itemEntries`,path = `enterprise/java/core.md`、
  refId = `tech.code.java.core`、reason = Acme 自定义。其余 item 仍走模板路径
  `tech/code/java/{item}.md`。
- `swift` 组已移除。
- 其他 vendor 组(typescript/javascript/python/.../redis/sql 等)原样保留。
- `validate_structure` 通过:无重复 refId,所有路由至少有一个组或 prependItem。

可运行案例见 `tests/rust/reference-catalog/enterprise_overlay.rs`。

## 参考文件放置约定与限制

- vendor 参考文件位于 `plugins/shared/loom/references/`(referenceRoot=`loom`)或
  `plugins/shared/loom-deploy/references/`(referenceRoot=`loom-deploy`)。
  `validate_file_existence` 当前**只识别这两个 root**(vendor.rs:68-77、90-94)。
- enterprise 参考文件(如 `enterprise/java/core.md`)应放在企业自管的目录中。
  目前**没有**约定好的 enterprise root 与自动发现机制;`validate_file_existence` 不会
  校验 enterprise 路径,企业需自行保证文件存在。
- overlay TOML 的 `referenceRoot` 应与 base 一致(如 `code` 路由保持 `"loom"`),否则
  `validate_file_existence` 会因未知 root 报错。企业内部参考文件路径(如
  `enterprise/java/core.md`)通过 `itemEntries` 的 `path` 字段表达,不依赖 referenceRoot
  解析——但运行时加载器(后续接入)需要知道如何定位这类路径。
- `requiredSections`:enterprise overlay 可以在组的 `requiredSections` 中追加企业强制
  section,但不可移除 vendor 已声明的条目(当前合并是整组替换,故 overlay 需把 vendor
  的 requiredSections 一并写回)。

## 当前限制与后续路线

| 项 | 当前 | 后续 |
|----|------|------|
| vendor 目录加载 | 已接入(`vendor_catalog()` 单例) | 不变 |
| enterprise overlay 加载 | **已接入**;`resolved_catalog()` 通过 `LOOM_CATALOG_OVERLAY` 环境变量发现并合并 | 增加 project overlay 层(项目级微调) |
| prune / replace 运行时生效 | 需程序化调用;结果不写回 `resolved_catalog` 缓存 | 支持 TOML 声明 prune 指令,或运行时注入合并结果 |
| `focus_rules`/`applicability`/`backend_ecosystems` 合并 | overlay 整体替换 base | 按字段做细粒度追加/去重 |
| enterprise 参考文件存在性校验 | `validate_file_existence` 跳过 enterprise 路径(仅校验 loom/loom-deploy root) | 企业自行保证文件存在;后续可增加可配置 root |
| 参考文件 `.md` 内容契约 | 由各域 crate(contracts/architecture/deploy)在运行时校验 section | 不变 |
