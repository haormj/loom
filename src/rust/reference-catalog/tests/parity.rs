//! Parity 测试:验证 vendor catalog.toml 忠实复现了
//! contracts、architecture 和 deploy crate 中的硬编码 enum 函数及
//! match 分支。
//!
//! 这些测试是 Phase 1 的安全网:当选择代码路径切换为从目录读取时,
//! 这些测试证明目录是一次忠实的导出。Phase 1 之后可以移除或反转。

use reference_catalog::{
    load_vendor_catalog, validate_file_existence, validate_structure, vendor_catalog_path,
};
use std::collections::BTreeSet;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

/// 从 `code_quality_enum_refs()` 返回的 JSON 中提取
/// `knownReferenceGroups.code` 映射。
fn extract_code_groups(
    json: &serde_json::Value,
) -> std::collections::BTreeMap<String, BTreeSet<String>> {
    let groups = json
        .get("knownReferenceGroups")
        .and_then(|v| v.get("code"))
        .and_then(|v| v.as_object())
        .expect("knownReferenceGroups.code should be an object");

    groups
        .iter()
        .map(|(key, value)| {
            let items: BTreeSet<String> = value
                .as_array()
                .expect("group items should be an array")
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
            (key.clone(), items)
        })
        .collect()
}

// ════════════════════════════════════════════════════════════════════════
// 目录加载与结构校验
// ════════════════════════════════════════════════════════════════════════

#[test]
fn vendor_catalog_loads_successfully() {
    let path = vendor_catalog_path();
    assert!(
        path.exists(),
        "vendor catalog not found at {}",
        path.display()
    );
    let catalog = load_vendor_catalog().expect("vendor catalog should parse");
    assert_eq!(catalog.schema_version, 1);
    assert!(catalog.routes.len() >= 6, "expected at least 6 routes");
}

#[test]
fn vendor_catalog_has_all_expected_routes() {
    let catalog = load_vendor_catalog().expect("parse");
    let route_ids: BTreeSet<String> = catalog.routes.iter().map(|r| r.id.clone()).collect();
    assert!(route_ids.contains("code"));
    assert!(route_ids.contains("api"));
    assert!(route_ids.contains("arch"));
    assert!(route_ids.contains("uix"));
    assert!(route_ids.contains("browser"));
    assert!(route_ids.contains("deploy"));
}

#[test]
fn vendor_catalog_passes_structural_validation() {
    let catalog = load_vendor_catalog().expect("parse");
    validate_structure(&catalog).expect("structural validation should pass");
}

#[test]
fn vendor_catalog_all_reference_files_exist() {
    let catalog = load_vendor_catalog().expect("parse");
    let root = repo_root();
    validate_file_existence(&catalog, &root).expect("all referenced files should exist on disk");
}

// ════════════════════════════════════════════════════════════════════════
// Code 路由 parity — 对照 code_quality_enum_refs()
// ════════════════════════════════════════════════════════════════════════

#[test]
fn code_route_groups_match_hardcoded_enum() {
    let catalog = load_vendor_catalog().expect("parse");
    let catalog_groups = catalog.known_reference_groups("code");

    let enum_json = contracts::code_quality_enum_refs();
    let enum_groups = extract_code_groups(&enum_json);

    assert_eq!(
        catalog_groups.keys().collect::<BTreeSet<_>>(),
        enum_groups.keys().collect::<BTreeSet<_>>(),
        "code route group keys must match knownReferenceGroups.code keys"
    );

    for (group_id, enum_items) in &enum_groups {
        let catalog_items: BTreeSet<String> = catalog_groups
            .get(group_id)
            .unwrap_or_else(|| panic!("group {} missing from catalog", group_id))
            .iter()
            .cloned()
            .collect();
        assert_eq!(
            catalog_items, *enum_items,
            "code route group '{}' items mismatch",
            group_id
        );
    }
}

#[test]
fn code_route_refid_and_path_patterns_match_hardcoded_logic() {
    // 验证特定 (group, item) → (refId, path) 映射,对照
    // reference_load_plan_item() 的 match 分支(code_quality.rs:2227-2327)。
    let catalog = load_vendor_catalog().expect("parse");
    let entries: std::collections::HashMap<String, reference_catalog::ExpandedEntry> = catalog
        .expand_all()
        .into_iter()
        .filter(|e| e.route_id == "code")
        .map(|e| (format!("{}.{}", e.group_id, e.item), e))
        .collect();

    // mybatisplus — 特殊前缀 + 嵌套路径
    let e = entries.get("mybatisplus.configuration").unwrap();
    assert_eq!(e.ref_id, "bk.spring.mybatisplus.configuration");
    assert_eq!(
        e.path,
        "tech/backend/springboot/mybatis-plus/configuration.md"
    );

    // springboot — 后端前缀
    let e = entries.get("springboot.web").unwrap();
    assert_eq!(e.ref_id, "bk.spring.web");
    assert_eq!(e.path, "tech/backend/springboot/web.md");

    // django — 后端前缀
    let e = entries.get("django.models").unwrap();
    assert_eq!(e.ref_id, "bk.django.models");
    assert_eq!(e.path, "tech/backend/django/models.md");

    // react — 前端前缀
    let e = entries.get("react.hooks").unwrap();
    assert_eq!(e.ref_id, "fe.react.hooks");
    assert_eq!(e.path, "tech/frontend/react/hooks.md");

    // reactnative — 前端前缀 + 连字符目录
    let e = entries.get("reactnative.core").unwrap();
    assert_eq!(e.ref_id, "fe.rn.core");
    assert_eq!(e.path, "tech/frontend/react-native/core.md");

    // sql provider overlay — 两级路径
    let e = entries.get("sql.mysql.schema").unwrap();
    assert_eq!(e.ref_id, "tech.code.sql.mysql.schema");
    assert_eq!(e.path, "tech/code/sql/mysql/schema.md");

    // sql 通用项 — 回退路径
    let e = entries.get("sql.dialects").unwrap();
    assert_eq!(e.ref_id, "tech.code.sql.dialects");
    assert_eq!(e.path, "tech/code/sql/dialects.md");

    // redis — 特殊前缀
    let e = entries.get("redis.cache").unwrap();
    assert_eq!(e.ref_id, "tech.code.redis.cache");
    assert_eq!(e.path, "tech/code/redis/cache.md");

    // common — 特殊覆盖(非 tech/code/common/observability.md)
    let e = entries.get("common.observability").unwrap();
    assert_eq!(e.ref_id, "tech.code.observability");
    assert_eq!(e.path, "tech/code/observability.md");

    // 语言回退 — tech/code/{group}/{item}.md
    let e = entries.get("rust.ownership").unwrap();
    assert_eq!(e.ref_id, "tech.code.rust.ownership");
    assert_eq!(e.path, "tech/code/rust/ownership.md");

    let e = entries.get("python.typing").unwrap();
    assert_eq!(e.ref_id, "tech.code.python.typing");
    assert_eq!(e.path, "tech/code/python/typing.md");
}

#[test]
fn code_route_has_common_prepend_item() {
    let catalog = load_vendor_catalog().expect("parse");
    let prepends = catalog.prepend_items_for_route("code");
    assert_eq!(prepends.len(), 1);
    assert_eq!(prepends[0].ref_id, "tech.code.common");
    assert_eq!(prepends[0].path, "tech/code/common.md");
    assert_eq!(prepends[0].condition, "when_groups_non_empty");
}

// ════════════════════════════════════════════════════════════════════════
// API 路由 parity — 对照 api_quality_enum_refs()
// ════════════════════════════════════════════════════════════════════════

#[test]
fn api_route_groups_match_hardcoded_enum() {
    let catalog = load_vendor_catalog().expect("parse");
    let catalog_items: BTreeSet<String> =
        catalog.items_for_group("api", "api").into_iter().collect();

    let enum_json = contracts::api_quality_enum_refs();
    let enum_items: BTreeSet<String> = enum_json
        .get("knownReferenceGroups")
        .and_then(|v| v.get("api"))
        .and_then(|v| v.as_array())
        .unwrap()
        .iter()
        .filter_map(|v| v.as_str().map(String::from))
        .collect();

    assert_eq!(
        catalog_items, enum_items,
        "API route items must match knownReferenceGroups.api"
    );
}

#[test]
fn api_route_refid_and_path_patterns_match() {
    let catalog = load_vendor_catalog().expect("parse");
    let entries: std::collections::HashMap<String, reference_catalog::ExpandedEntry> = catalog
        .expand_all()
        .into_iter()
        .filter(|e| e.route_id == "api")
        .map(|e| (e.item.clone(), e))
        .collect();

    let e = entries.get("core").unwrap();
    assert_eq!(e.ref_id, "tech.api.core");
    assert_eq!(e.path, "tech/api/core.md");

    let e = entries.get("jwt").unwrap();
    assert_eq!(e.ref_id, "tech.api.jwt");
    assert_eq!(e.path, "tech/api/jwt.md");
}

// ════════════════════════════════════════════════════════════════════════
// UIX 路由 parity — 对照 UI_*_REFERENCE_ITEMS 常量
// ════════════════════════════════════════════════════════════════════════

#[test]
fn uix_route_groups_match_hardcoded_constants() {
    use contracts::{
        UI_CORE_REFERENCE_ITEMS, UI_FOCUS_REFERENCE_ITEMS, UI_REFERENCE_GROUP_KEYS,
        UI_SCENARIO_REFERENCE_ITEMS, UI_STACK_REFERENCE_ITEMS, UI_TOKEN_REFERENCE_ITEMS,
    };

    let catalog = load_vendor_catalog().expect("parse");
    let route = catalog.routes.iter().find(|r| r.id == "uix").unwrap();

    // 组键
    let catalog_keys: BTreeSet<String> = route.groups.iter().map(|g| g.id.clone()).collect();
    let const_keys: BTreeSet<String> = UI_REFERENCE_GROUP_KEYS
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert_eq!(catalog_keys, const_keys, "UIX group keys mismatch");

    // core items
    let catalog_core: BTreeSet<String> = route
        .groups
        .iter()
        .find(|g| g.id == "core")
        .unwrap()
        .items
        .iter()
        .cloned()
        .collect();
    let const_core: BTreeSet<String> = UI_CORE_REFERENCE_ITEMS
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert_eq!(catalog_core, const_core, "UIX core items mismatch");

    // focus items
    let catalog_focus: BTreeSet<String> = route
        .groups
        .iter()
        .find(|g| g.id == "focus")
        .unwrap()
        .items
        .iter()
        .cloned()
        .collect();
    let const_focus: BTreeSet<String> = UI_FOCUS_REFERENCE_ITEMS
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert_eq!(catalog_focus, const_focus, "UIX focus items mismatch");

    // tokens items
    let catalog_tokens: BTreeSet<String> = route
        .groups
        .iter()
        .find(|g| g.id == "tokens")
        .unwrap()
        .items
        .iter()
        .cloned()
        .collect();
    let const_tokens: BTreeSet<String> = UI_TOKEN_REFERENCE_ITEMS
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert_eq!(catalog_tokens, const_tokens, "UIX tokens items mismatch");

    // scenarios items
    let catalog_scenarios: BTreeSet<String> = route
        .groups
        .iter()
        .find(|g| g.id == "scenarios")
        .unwrap()
        .items
        .iter()
        .cloned()
        .collect();
    let const_scenarios: BTreeSet<String> = UI_SCENARIO_REFERENCE_ITEMS
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert_eq!(
        catalog_scenarios, const_scenarios,
        "UIX scenarios items mismatch"
    );

    // stacks items
    let catalog_stacks: BTreeSet<String> = route
        .groups
        .iter()
        .find(|g| g.id == "stacks")
        .unwrap()
        .items
        .iter()
        .cloned()
        .collect();
    let const_stacks: BTreeSet<String> = UI_STACK_REFERENCE_ITEMS
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert_eq!(catalog_stacks, const_stacks, "UIX stacks items mismatch");
}

#[test]
fn uix_route_path_patterns_match_ui_reference_path() {
    let catalog = load_vendor_catalog().expect("parse");
    let entries: std::collections::HashMap<String, reference_catalog::ExpandedEntry> = catalog
        .expand_all()
        .into_iter()
        .filter(|e| e.route_id == "uix")
        .map(|e| (format!("{}.{}", e.group_id, e.item), e))
        .collect();

    // core/focus → uix/{item}.md
    assert_eq!(entries.get("core.core").unwrap().path, "uix/core.md");
    assert_eq!(entries.get("focus.data").unwrap().path, "uix/data.md");

    // tokens → uix/tokens/{item}.md
    assert_eq!(
        entries.get("tokens.color-system").unwrap().path,
        "uix/tokens/color-system.md"
    );

    // scenarios → uix/scenarios/{item}.md
    assert_eq!(
        entries.get("scenarios.admin-dashboard").unwrap().path,
        "uix/scenarios/admin-dashboard.md"
    );

    // stacks → uix/stacks/{item}.md
    assert_eq!(
        entries.get("stacks.react").unwrap().path,
        "uix/stacks/react.md"
    );

    // templates → 显式路径
    assert_eq!(
        entries.get("templates.tokens-css").unwrap().path,
        "uix/templates/tokens.css.tpl"
    );
    assert_eq!(
        entries.get("templates.tokens-tailwind").unwrap().path,
        "uix/templates/tokens.tailwind.tpl"
    );
}

// ════════════════════════════════════════════════════════════════════════
// Architecture 路由 parity — 对照 architecture_reference_groups()
// (私有函数,值从 architecture/request.rs:825-838 转录)
// ════════════════════════════════════════════════════════════════════════

#[test]
fn arch_route_section_groups_match_hardcoded_mapping() {
    let catalog = load_vendor_catalog().expect("parse");
    let route = catalog.routes.iter().find(|r| r.id == "arch").unwrap();

    let section_map: std::collections::BTreeMap<String, BTreeSet<String>> = route
        .section_groups
        .iter()
        .map(|sg| (sg.id.clone(), sg.items.iter().cloned().collect()))
        .collect();

    // 从 architecture/request.rs:827-836 转录
    assert_eq!(
        section_map.get("Foundation").unwrap(),
        &["core", "patterns", "system"]
            .iter()
            .map(|s| s.to_string())
            .collect(),
    );
    assert_eq!(
        section_map.get("DomainContract").unwrap(),
        &["core", "data", "system"]
            .iter()
            .map(|s| s.to_string())
            .collect(),
    );
    assert_eq!(
        section_map.get("Behavior").unwrap(),
        &["core", "system", "failure"]
            .iter()
            .map(|s| s.to_string())
            .collect(),
    );
    assert_eq!(
        section_map.get("FrontendExperience").unwrap(),
        &["core"].iter().map(|s| s.to_string()).collect(),
    );
    assert_eq!(
        section_map.get("RuntimeDelivery").unwrap(),
        &["core", "system", "failure"]
            .iter()
            .map(|s| s.to_string())
            .collect(),
    );
    assert_eq!(
        section_map.get("Coverage").unwrap(),
        &["core", "patterns", "system", "data", "nfr", "adr", "failure"]
            .iter()
            .map(|s| s.to_string())
            .collect(),
    );
}

#[test]
fn arch_route_items_match() {
    let catalog = load_vendor_catalog().expect("parse");
    let items: BTreeSet<String> = catalog
        .items_for_group("arch", "arch")
        .into_iter()
        .collect();
    let expected: BTreeSet<String> = [
        "core", "patterns", "system", "data", "nfr", "adr", "failure",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    assert_eq!(items, expected);
}

// ════════════════════════════════════════════════════════════════════════
// Browser 路由 parity — 对照 playwright_reference_load_plan()
// ════════════════════════════════════════════════════════════════════════

#[test]
fn browser_route_refids_and_paths_match_playwright_load_plan() {
    use contracts::{playwright_reference_load_plan, BrowserRunnerSource, BrowserVerificationMode};

    // 使用能选中全部 7 项的参数调用(reliability 永不会被选中)
    let plan = playwright_reference_load_plan(
        BrowserVerificationMode::BusinessFlow,
        BrowserRunnerSource::LoomManaged,
        &["action.test".to_string()],
        &["submitting".to_string()],
        &[
            "verify.rendered_viewports".to_string(),
            "web.semantic_accessibility".to_string(),
        ],
    );

    // 从函数输出构建期望映射
    let mut expected: std::collections::BTreeMap<String, (String, String)> =
        std::collections::BTreeMap::new();
    for item in &plan {
        expected.insert(
            item.ref_id.clone(),
            (item.path.clone(), item.reason.clone()),
        );
    }

    // 加载目录并获取 browser_explicit 组条目
    let catalog = load_vendor_catalog().expect("parse");
    let entries: Vec<reference_catalog::ExpandedEntry> = catalog
        .expand_all()
        .into_iter()
        .filter(|e| e.route_id == "browser" && e.group_id == "playwright_explicit")
        .collect();

    for entry in &entries {
        let expected_vals = expected
            .get(&entry.ref_id)
            .unwrap_or_else(|| panic!("refId {} not in playwright plan", entry.ref_id));
        assert_eq!(
            entry.path, expected_vals.0,
            "path mismatch for refId {}",
            entry.ref_id
        );
    }

    // 同时验证计划中的每个条目在目录中都有对应项
    let catalog_ref_ids: BTreeSet<String> = entries.iter().map(|e| e.ref_id.clone()).collect();
    for item in &plan {
        assert!(
            catalog_ref_ids.contains(&item.ref_id),
            "catalog missing refId {} from playwright plan",
            item.ref_id
        );
    }
}

#[test]
fn browser_route_all_eight_files_present() {
    let catalog = load_vendor_catalog().expect("parse");
    let items: BTreeSet<String> = catalog
        .items_for_group("browser", "playwright")
        .into_iter()
        .collect();
    let expected: BTreeSet<String> = [
        "core",
        "locators",
        "configuration",
        "fixtures",
        "network",
        "visual",
        "accessibility",
        "reliability",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    assert_eq!(items, expected);
}

// ════════════════════════════════════════════════════════════════════════
// Deploy 路由 parity — 对照 reference_metadata()
// (私有函数,值从 deploy/references.rs:110-187 转录)
// ════════════════════════════════════════════════════════════════════════

#[test]
fn deploy_route_refids_paths_and_reasons_match_reference_metadata() {
    let catalog = load_vendor_catalog().expect("parse");
    let entries: std::collections::BTreeMap<String, reference_catalog::ExpandedEntry> = catalog
        .expand_all()
        .into_iter()
        .filter(|e| e.route_id == "deploy")
        .map(|e| (e.ref_id.clone(), e))
        .collect();

    // 从 deploy/references.rs:110-187 转录
    let expected = [
        (
            "deploy.providers",
            "providers.md",
            "Provider selection and generated/existing asset policy.",
        ),
        (
            "deploy.matrix",
            "matrix.md",
            "Deployment topology, runtime, layout, port, and dependency matrix.",
        ),
        (
            "deploy.source-model",
            "source-model.md",
            "Repository evidence to deployable service model guidance.",
        ),
        (
            "deploy.topology",
            "topology.md",
            "Public entry, proxy route, and validation topology guidance.",
        ),
        (
            "deploy.compose",
            "compose.md",
            "Compose service wiring, ports, dependencies, and health guidance.",
        ),
        (
            "deploy.dockerfile",
            "dockerfile.md",
            "Dockerfile context, workdir, copy, build, and runtime guidance.",
        ),
        (
            "deploy.environment",
            "environment.md",
            "Environment, local defaults, dependency URL, and state guidance.",
        ),
        (
            "deploy.workspaces",
            "workspaces.md",
            "Workspace app path, source root, and build context guidance.",
        ),
        (
            "deploy.bootstrap",
            "bootstrap.md",
            "Migration/bootstrap diagnostics and approval boundary guidance.",
        ),
        (
            "deploy.repair",
            "repair.md",
            "Deploy repair decision tree and editable asset boundary.",
        ),
        (
            "deploy.dependencies.redis",
            "redis.md",
            "Redis dependency capabilities, persistence, health, and generated asset boundary.",
        ),
        (
            "deploy.stacks.node",
            "node.md",
            "Node-family scanner, generated asset, and repair guidance.",
        ),
        (
            "deploy.stacks.python",
            "python.md",
            "Python scanner, generated asset, and repair guidance.",
        ),
        (
            "deploy.stacks.go",
            "go.md",
            "Go scanner, generated asset, and repair guidance.",
        ),
        (
            "deploy.stacks.java",
            "java.md",
            "Java scanner, generated asset, and repair guidance.",
        ),
        (
            "deploy.stacks.dotnet",
            "dotnet.md",
            ".NET scanner, generated asset, and repair guidance.",
        ),
        (
            "deploy.stacks.php",
            "php.md",
            "PHP scanner, generated asset, and repair guidance.",
        ),
        (
            "deploy.stacks.ruby",
            "ruby.md",
            "Ruby scanner, generated asset, and repair guidance.",
        ),
        (
            "deploy.stacks.static",
            "static.md",
            "Static site scanner, generated asset, and repair guidance.",
        ),
    ];

    for (ref_id, path, reason) in &expected {
        let entry = entries
            .get(*ref_id)
            .unwrap_or_else(|| panic!("catalog missing deploy refId {}", ref_id));
        assert_eq!(entry.path, *path, "path mismatch for {}", ref_id);
        assert_eq!(
            entry.reason.as_deref().unwrap_or(""),
            *reason,
            "reason mismatch for {}",
            ref_id
        );
    }
}

#[test]
fn deploy_route_uses_loom_deploy_reference_root() {
    let catalog = load_vendor_catalog().expect("parse");
    let route = catalog.routes.iter().find(|r| r.id == "deploy").unwrap();
    assert_eq!(route.effective_reference_root(), "loom-deploy");
}

// ─── Phase 2: Focus tag 文本关键词规则 parity 测试 ───────────────────────

/// 验证 vendor catalog 包含预期数量的 focus rules(25 条)。
#[test]
fn focus_rules_count_matches_expected() {
    let catalog = load_vendor_catalog().expect("parse");
    assert_eq!(
        catalog.focus_rules.len(),
        25,
        "focus_rules 应有 25 条规则(对应原 task_focus_tags 的 25 个文本关键词块)"
    );
}

/// 验证 focus rules 的 tag 集合与原硬编码逻辑完全一致。
#[test]
fn focus_rules_tags_match_hardcoded_set() {
    let catalog = load_vendor_catalog().expect("parse");
    let tags: BTreeSet<String> = catalog
        .focus_rules
        .iter()
        .flat_map(|r| r.focus_tags.iter())
        .cloned()
        .collect();

    let expected: BTreeSet<String> = [
        "state",
        "hooks",
        "server_components",
        "react19",
        "app_router",
        "server_actions",
        "data_fetching",
        "nuxt",
        "build_tooling",
        "mobile",
        "routing",
        "rxjs",
        "ngrx",
        "riverpod",
        "bloc",
        "list_performance",
        "storage",
        "async",
        "cache",
        "performance",
        "runtime",
        "integration",
        "migration",
        "architecture",
        "generics",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();

    assert_eq!(tags, expected, "focus rule tag 集合不匹配");
}

/// 验证 `cache` 规则拥有 `requires_backend = true` 守卫,
/// 且是唯一一条有此守卫的规则。
#[test]
fn cache_rule_is_the_only_backend_guarded_rule() {
    let catalog = load_vendor_catalog().expect("parse");
    let backend_guarded: Vec<_> = catalog
        .focus_rules
        .iter()
        .filter(|r| r.requires_backend)
        .collect();

    assert_eq!(
        backend_guarded.len(),
        1,
        "仅 cache 规则应有 requires_backend 守卫"
    );
    assert_eq!(
        backend_guarded[0].focus_tags,
        vec!["cache"],
        "requires_backend 守卫的规则应为 cache"
    );
}

/// 验证关键词匹配产出正确的 tag(正向用例)。
#[test]
fn focus_tags_from_text_produces_expected_tags() {
    let catalog = load_vendor_catalog().expect("parse");

    // 单关键词 → 单 tag
    let tags = catalog.focus_tags_from_text("使用 zustand 管理状态", false);
    assert!(
        tags.contains(&"state".to_string()),
        "zustand 应产出 state tag"
    );

    // 多关键词命中多条规则 → 多 tag
    let tags = catalog.focus_tags_from_text(" custom hook and deep link ", false);
    assert!(
        tags.contains(&"hooks".to_string()),
        "custom hook 应产出 hooks tag"
    );
    assert!(
        tags.contains(&"routing".to_string()),
        "deep link 应产出 routing tag"
    );

    // 整词匹配:" bloc " 不应匹配 "block"
    let tags = catalog.focus_tags_from_text(" block design ", false);
    assert!(
        !tags.contains(&"bloc".to_string()),
        "'block' 不应匹配 bloc 规则(需要 ' bloc ' 整词)"
    );
    let tags = catalog.focus_tags_from_text(" bloc pattern ", false);
    assert!(
        tags.contains(&"bloc".to_string()),
        "' bloc ' 整词应匹配 bloc 规则"
    );
}

/// 验证 `requires_backend` 守卫:非后端任务不产出 cache tag,
/// 后端任务产出 cache tag。
#[test]
fn cache_rule_respects_backend_guard() {
    let catalog = load_vendor_catalog().expect("parse");
    let cache_text = " 使用 spring cache 和 caffeine 缓存策略 ";

    // 非后端任务:不产出 cache tag
    let tags = catalog.focus_tags_from_text(cache_text, false);
    assert!(
        !tags.contains(&"cache".to_string()),
        "非后端任务不应产出 cache tag(requires_backend 守卫)"
    );

    // 后端任务:产出 cache tag
    let tags = catalog.focus_tags_from_text(cache_text, true);
    assert!(
        tags.contains(&"cache".to_string()),
        "后端任务应产出 cache tag"
    );
}

/// 验证去重:同一规则的多个关键词命中只添加一次 tag。
#[test]
fn focus_tags_deduplicate_within_same_rule() {
    let catalog = load_vendor_catalog().expect("parse");
    // "state" 和 "store" 都属于 state 规则
    let tags = catalog.focus_tags_from_text(" state and store ", false);
    let state_count = tags.iter().filter(|t| *t == "state").count();
    assert_eq!(state_count, 1, "state tag 应只出现一次(去重)");
}
