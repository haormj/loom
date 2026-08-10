//! resolved_catalog 集成测试:验证 vendor + enterprise overlay 的运行时合并入口。
//!
//! 测试通过 `build_resolved_catalog` 纯函数核心验证 overlay 发现与合并逻辑,
//! 避免 `OnceLock` 不可重置的问题。`resolved_catalog()` 只是此函数的缓存包装。
//!
//! 测试场景:
//! - overlay 路径为 None → 回退 vendor
//! - overlay 路径不存在 → 回退 vendor
//! - overlay 路径存在且合法 → 合并成功
//! - overlay 路径存在但 TOML 非法 → panic(配置错误尽早暴露)

use reference_catalog::{
    build_resolved_catalog, load_vendor_catalog, validate_structure, CATALOG_OVERLAY_ENV,
};
use std::io::Write;

const VALID_OVERLAY_TOML: &str = r#"
schemaVersion = 1

[[routes]]
id = "code"
referenceRoot = "loom"

  [[routes.groups]]
  id = "goa"
  refIdTemplate = "bk.goa.{item}"
  pathTemplate = "tech/backend/goa/{item}.md"
  reasonTemplate = "已选择 {label} {item} 的框架质量参考,用于本任务。"
  label = "Goa"
  items = ["core", "routing", "middleware", "testing"]
"#;

fn write_temp_overlay(content: &str, suffix: &str) -> std::path::PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "loom-catalog-overlay-test-{}-{}",
        std::process::id(),
        suffix
    ));
    let mut file = std::fs::File::create(&path).expect("create temp overlay");
    file.write_all(content.as_bytes())
        .expect("write temp overlay");
    path
}

#[test]
fn no_overlay_path_returns_vendor() {
    let vendor = load_vendor_catalog().expect("vendor loads");
    let resolved = build_resolved_catalog(&vendor, None);

    // 无 overlay 时,resolved 与 vendor 的 code 路由组数一致
    let vendor_code = vendor
        .routes
        .iter()
        .find(|r| r.id == "code")
        .expect("code route exists");
    let resolved_code = resolved
        .routes
        .iter()
        .find(|r| r.id == "code")
        .expect("code route exists");
    assert_eq!(vendor_code.groups.len(), resolved_code.groups.len());
}

#[test]
fn nonexistent_overlay_path_returns_vendor() {
    let vendor = load_vendor_catalog().expect("vendor loads");
    let bogus = std::path::PathBuf::from("/nonexistent/overlay.toml");
    let resolved = build_resolved_catalog(&vendor, Some(&bogus));

    // 路径不存在时静默回退
    assert_eq!(
        vendor
            .routes
            .iter()
            .find(|r| r.id == "code")
            .unwrap()
            .groups
            .len(),
        resolved
            .routes
            .iter()
            .find(|r| r.id == "code")
            .unwrap()
            .groups
            .len()
    );
}

#[test]
fn valid_overlay_merges_new_group() {
    let vendor = load_vendor_catalog().expect("vendor loads");
    let overlay_path = write_temp_overlay(VALID_OVERLAY_TOML, "valid");
    let resolved = build_resolved_catalog(&vendor, Some(&overlay_path));

    let code = resolved
        .routes
        .iter()
        .find(|r| r.id == "code")
        .expect("code route exists");

    // goa 组应被新增
    assert!(
        code.groups.iter().any(|g| g.id == "goa"),
        "goa group should be added by overlay"
    );

    // vendor 原有组保留
    assert!(code.groups.iter().any(|g| g.id == "java"));
    assert!(code.groups.iter().any(|g| g.id == "typescript"));

    // 结构校验通过
    validate_structure(&resolved).expect("merged catalog should be structurally valid");

    let _ = std::fs::remove_file(overlay_path);
}

#[test]
fn valid_overlay_resolves_new_entries() {
    let vendor = load_vendor_catalog().expect("vendor loads");
    let overlay_path = write_temp_overlay(VALID_OVERLAY_TOML, "resolve");
    let resolved = build_resolved_catalog(&vendor, Some(&overlay_path));

    let goa_core = resolved
        .resolve_entry("code", "goa", "core")
        .expect("goa.core should resolve after overlay merge");

    assert_eq!(goa_core.ref_id, "bk.goa.core");
    assert_eq!(goa_core.path, "tech/backend/goa/core.md");

    let _ = std::fs::remove_file(overlay_path);
}

#[test]
#[should_panic(expected = "failed to load")]
fn malformed_overlay_panics() {
    let vendor = load_vendor_catalog().expect("vendor loads");
    let overlay_path = write_temp_overlay("this is not valid toml {{{", "malformed");
    let _ = build_resolved_catalog(&vendor, Some(&overlay_path));
}

#[test]
fn env_var_name_matches_contract() {
    assert_eq!(CATALOG_OVERLAY_ENV, "LOOM_CATALOG_OVERLAY");
}
