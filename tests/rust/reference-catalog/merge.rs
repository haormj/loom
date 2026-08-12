//! 合并测试:使用合成 fixture 验证三层目录合并语义和 overlay
//! 操作(extend/replace/prune)。

use reference_catalog::{
    merge_catalogs, parse_catalog, prune_groups, replace_item_entry, validate_structure,
    ReferenceCatalog,
};

fn empty_catalog() -> ReferenceCatalog {
    ReferenceCatalog {
        schema_version: 1,
        routes: vec![],
        focus_rules: vec![],
        applicability: vec![],
        backend_ecosystems: vec![],
        repo_signals: Default::default(),
    }
}

const BASE_TOML: &str = r#"
schemaVersion = 1

[[routes]]
id = "code"
referenceRoot = "loom"

  [[routes.groups]]
  id = "java"
  refIdTemplate = "tech.code.java.{item}"
  pathTemplate = "tech/code/java/{item}.md"
  items = ["core", "testing"]

  [[routes.groups]]
  id = "python"
  refIdTemplate = "tech.code.python.{item}"
  pathTemplate = "tech/code/python/{item}.md"
  items = ["core", "async", "testing"]
"#;

const OVERLAY_EXTEND_TOML: &str = r#"
schemaVersion = 1

[[routes]]
id = "code"
referenceRoot = "loom"

  [[routes.groups]]
  id = "kotlin"
  refIdTemplate = "tech.code.kotlin.{item}"
  pathTemplate = "tech/code/kotlin/{item}.md"
  items = ["core", "coroutines"]
"#;

const OVERLAY_REPLACE_TOML: &str = r#"
schemaVersion = 1

[[routes]]
id = "code"
referenceRoot = "loom"

  [[routes.groups]]
  id = "java"
  refIdTemplate = "tech.code.java.{item}"
  pathTemplate = "tech/code/java/{item}.md"
  items = ["core", "spring", "reactive", "testing"]
"#;

#[test]
fn merge_extend_adds_new_group() {
    let base = parse_catalog(BASE_TOML, "base").unwrap();
    let overlay = parse_catalog(OVERLAY_EXTEND_TOML, "overlay").unwrap();
    let merged = merge_catalogs(&base, &overlay).unwrap();

    let code_route = merged.routes.iter().find(|r| r.id == "code").unwrap();
    let group_ids: Vec<&str> = code_route.groups.iter().map(|g| g.id.as_str()).collect();
    assert!(group_ids.contains(&"java"));
    assert!(group_ids.contains(&"python"));
    assert!(group_ids.contains(&"kotlin"));

    let kotlin = code_route.groups.iter().find(|g| g.id == "kotlin").unwrap();
    assert_eq!(kotlin.items, vec!["core", "coroutines"]);
}

#[test]
fn merge_replace_overwrites_existing_group_items() {
    let base = parse_catalog(BASE_TOML, "base").unwrap();
    let overlay = parse_catalog(OVERLAY_REPLACE_TOML, "overlay").unwrap();
    let merged = merge_catalogs(&base, &overlay).unwrap();

    let java = merged
        .routes
        .iter()
        .find(|r| r.id == "code")
        .unwrap()
        .groups
        .iter()
        .find(|g| g.id == "java")
        .unwrap();
    assert_eq!(java.items, vec!["core", "spring", "reactive", "testing"]);
}

#[test]
fn merge_preserves_non_overlapping_groups() {
    let base = parse_catalog(BASE_TOML, "base").unwrap();
    let overlay = parse_catalog(OVERLAY_EXTEND_TOML, "overlay").unwrap();
    let merged = merge_catalogs(&base, &overlay).unwrap();

    let python = merged
        .routes
        .iter()
        .find(|r| r.id == "code")
        .unwrap()
        .groups
        .iter()
        .find(|g| g.id == "python")
        .unwrap();
    assert_eq!(python.items, vec!["core", "async", "testing"]);
}

#[test]
fn prune_removes_specified_groups() {
    let mut catalog = parse_catalog(BASE_TOML, "base").unwrap();
    prune_groups(&mut catalog, "code", &["python".to_string()]);

    let code_route = catalog.routes.iter().find(|r| r.id == "code").unwrap();
    assert_eq!(code_route.groups.len(), 1);
    assert!(code_route.groups.iter().all(|g| g.id == "java"));
}

#[test]
fn replace_item_entry_moves_template_item_to_explicit() {
    let mut catalog = parse_catalog(BASE_TOML, "base").unwrap();
    let new_entry = reference_catalog::ItemEntry {
        item: "core".to_string(),
        ref_id: "tech.code.java.core".to_string(),
        path: "enterprise/java/core.md".to_string(),
        reason: Some("Enterprise Java core override".to_string()),
        condition: None,
    };
    replace_item_entry(&mut catalog, "code", "java", new_entry).unwrap();

    let java = catalog
        .routes
        .iter()
        .find(|r| r.id == "code")
        .unwrap()
        .groups
        .iter()
        .find(|g| g.id == "java")
        .unwrap();

    assert!(!java.items.contains(&"core".to_string()));
    assert_eq!(java.item_entries.len(), 1);
    assert_eq!(java.item_entries[0].path, "enterprise/java/core.md");
}

#[test]
fn merged_catalog_passes_structural_validation() {
    let base = parse_catalog(BASE_TOML, "base").unwrap();
    let overlay = parse_catalog(OVERLAY_EXTEND_TOML, "overlay").unwrap();
    let merged = merge_catalogs(&base, &overlay).unwrap();
    validate_structure(&merged).expect("merged catalog should be structurally valid");
}

#[test]
fn merge_with_empty_overlay_returns_base() {
    let base = parse_catalog(BASE_TOML, "base").unwrap();
    let overlay = empty_catalog();
    let merged = merge_catalogs(&base, &overlay).unwrap();

    assert_eq!(merged.routes.len(), base.routes.len());
}

#[test]
fn merge_with_empty_base_returns_overlay() {
    let base = empty_catalog();
    let overlay = parse_catalog(OVERLAY_EXTEND_TOML, "overlay").unwrap();
    let merged = merge_catalogs(&base, &overlay).unwrap();

    assert_eq!(merged.routes.len(), 1);
    let code_route = merged.routes.iter().find(|r| r.id == "code").unwrap();
    assert!(code_route.groups.iter().any(|g| g.id == "kotlin"));
}
