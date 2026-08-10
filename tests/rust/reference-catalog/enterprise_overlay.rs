//! 企业 overlay 真实场景案例:在真实 vendor 目录之上叠加 enterprise overlay,
//! 演示四种扩展操作(新增组、替换组、prune 组、replace item)的组合用法与
//! 结构校验。
//!
//! 与 `merge.rs` 的区别:本测试加载真实 vendor 目录(`load_vendor_catalog`),
//! 而非合成 fixture,从而构成贴近生产的业务案例。配套文档见
//! `docs/reference-catalog-extension.md`。

use reference_catalog::{
    load_vendor_catalog, merge_catalogs, parse_catalog, prune_groups, replace_item_entry,
    validate_structure, ItemEntry,
};

/// Acme 企业 overlay:新增 Goa 框架组、替换 Java 组(改用 Quarkus)。
///
/// prune swift 与 replace java.core 不写在 TOML 中,它们是 overlay 操作,
/// 在合并后调用 `prune_groups` / `replace_item_entry` 完成。这是当前 API
/// 的约定:TOML 表达"新增/整组替换",函数表达"裁剪/单 item 改写"。
const ENTERPRISE_OVERLAY_TOML: &str = r#"
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
"#;

/// 合并真实 vendor 目录与 Acme enterprise overlay,并执行 prune。
/// 不含 `replace_item_entry`——该步会移动 `java.core`,因此"替换组"断言
/// 需在此阶段验证 items 完整性。
fn acme_after_merge_and_prune() -> reference_catalog::ReferenceCatalog {
    let base = load_vendor_catalog().expect("vendor catalog should load");
    let overlay =
        parse_catalog(ENTERPRISE_OVERLAY_TOML, "enterprise.toml").expect("overlay should parse");

    let mut merged = merge_catalogs(&base, &overlay).expect("merge should succeed");

    // (c) 裁剪 swift:企业不做 iOS
    prune_groups(&mut merged, "code", &["swift".to_string()]);

    merged
}

/// 在 `acme_after_merge_and_prune` 之上执行 `replace_item_entry`,
/// 构成完整的 Acme overlay 结果。
fn acme_merged_catalog() -> reference_catalog::ReferenceCatalog {
    let mut merged = acme_after_merge_and_prune();

    // (d) 改写 java.core 指向企业内部参考文件
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
    )
    .expect("replace_item_entry should succeed");

    merged
}

#[test]
fn adds_new_enterprise_group() {
    let merged = acme_merged_catalog();
    let code = merged
        .routes
        .iter()
        .find(|r| r.id == "code")
        .expect("code route exists");

    let goa = code
        .groups
        .iter()
        .find(|g| g.id == "goa")
        .expect("goa group should be added");

    assert_eq!(goa.items, vec!["core", "routing", "middleware", "testing"]);
    assert_eq!(goa.label.as_deref(), Some("Goa"));
    assert_eq!(
        goa.path_template.as_deref(),
        Some("tech/backend/goa/{item}.md")
    );
    assert_eq!(goa.ref_id_template.as_deref(), Some("bk.goa.{item}"));
}

#[test]
fn replaces_existing_group_items() {
    // 在 replace_item 之前验证:overlay 整组替换后 items 完整
    let merged = acme_after_merge_and_prune();
    let java = merged
        .routes
        .iter()
        .find(|r| r.id == "code")
        .unwrap()
        .groups
        .iter()
        .find(|g| g.id == "java")
        .expect("java group present");

    // security/reactive 已移除,quarkus 已加入
    assert_eq!(
        java.items,
        vec!["core", "spring", "persistence", "testing", "quarkus"]
    );
    assert!(!java.items.contains(&"security".to_string()));
    assert!(!java.items.contains(&"reactive".to_string()));
    assert!(java.items.contains(&"quarkus".to_string()));
}

#[test]
fn pruned_group_is_removed() {
    let merged = acme_merged_catalog();
    let code = merged
        .routes
        .iter()
        .find(|r| r.id == "code")
        .expect("code route exists");

    assert!(
        !code.groups.iter().any(|g| g.id == "swift"),
        "swift group should have been pruned"
    );
}

#[test]
fn replaced_item_moves_to_explicit_entry_with_enterprise_path() {
    let merged = acme_merged_catalog();
    let java = merged
        .routes
        .iter()
        .find(|r| r.id == "code")
        .unwrap()
        .groups
        .iter()
        .find(|g| g.id == "java")
        .expect("java group present");

    // core 已从模板 items 移除
    assert!(!java.items.contains(&"core".to_string()));

    // core 转为显式条目,指向企业内部路径
    let core_entry = java
        .item_entries
        .iter()
        .find(|e| e.item == "core")
        .expect("core should be in item_entries after replace");

    assert_eq!(core_entry.ref_id, "tech.code.java.core");
    assert_eq!(core_entry.path, "enterprise/java/core.md");
    assert_eq!(
        core_entry.reason.as_deref(),
        Some("Acme 企业 Java 核心规范,覆盖内部编码与分层约定。")
    );
}

#[test]
fn replaced_item_resolves_via_catalog_lookup() {
    let merged = acme_merged_catalog();
    let resolved = merged
        .resolve_entry("code", "java", "core")
        .expect("java.core should resolve after replace");

    assert_eq!(resolved.ref_id, "tech.code.java.core");
    assert_eq!(resolved.path, "enterprise/java/core.md");
    assert_eq!(
        resolved.reason.as_deref(),
        Some("Acme 企业 Java 核心规范,覆盖内部编码与分层约定。")
    );
}

#[test]
fn non_overlapping_vendor_groups_are_preserved() {
    let merged = acme_merged_catalog();
    let code = merged
        .routes
        .iter()
        .find(|r| r.id == "code")
        .expect("code route exists");

    // overlay 未触及的 vendor 组应原样保留
    for expected in ["typescript", "python", "go", "rust", "sql", "redis"] {
        assert!(
            code.groups.iter().any(|g| g.id == expected),
            "vendor group '{}' should be preserved",
            expected
        );
    }
}

#[test]
fn merged_catalog_is_structurally_valid() {
    let merged = acme_merged_catalog();
    validate_structure(&merged).expect("merged catalog should pass structural validation");
}

#[test]
fn goa_items_expand_with_template_paths() {
    let merged = acme_merged_catalog();
    let entries: std::collections::HashMap<String, reference_catalog::ExpandedEntry> = merged
        .expand_all()
        .into_iter()
        .filter(|e| e.route_id == "code" && e.group_id == "goa")
        .map(|e| (e.item.clone(), e))
        .collect();

    assert_eq!(entries.len(), 4, "goa should expand 4 items");
    let core = entries.get("core").expect("goa.core expands");
    assert_eq!(core.ref_id, "bk.goa.core");
    assert_eq!(core.path, "tech/backend/goa/core.md");
    assert!(core.reason.as_deref().unwrap().contains("Goa"));
}
