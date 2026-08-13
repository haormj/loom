use reference_catalog::{merge_catalogs, parse_catalog};

#[test]
fn repo_signals_deserialize_from_toml() {
    let toml = r#"
schemaVersion = 1

[repoSignals]
skipDirs = [".git", "node_modules"]
maxScanDepth = 3

[repoSignals.sourceRoots]
paths = ["src", "tests"]

[[repoSignals.languages]]
id = "python"
label = "Python"

  [[repoSignals.languages.manifests]]
  path = "pyproject.toml"
  packageManager = "pip"

  [[repoSignals.languages.manifests]]
  path = "setup.py"
  packageManager = "pip"

  [[repoSignals.languages.frameworks]]
  needle = "fastapi"
  label = "FastAPI"
"#;
    let catalog = parse_catalog(toml, "test").expect("parse");
    assert!(!catalog.repo_signals.languages.is_empty());
    let python = &catalog.repo_signals.languages[0];
    assert_eq!(python.id, "python");
    assert_eq!(python.label, "Python");
    assert_eq!(python.manifests.len(), 2);
    assert_eq!(python.manifests[0].path, "pyproject.toml");
    assert_eq!(python.manifests[1].path, "setup.py");
    assert_eq!(python.frameworks.len(), 1);
    assert_eq!(python.frameworks[0].needle, "fastapi");
}

#[test]
fn repo_signals_overlay_replaces_language() {
    let base_toml = r#"
schemaVersion = 1

[repoSignals]

[[repoSignals.languages]]
id = "python"
label = "Python"

  [[repoSignals.languages.manifests]]
  path = "pyproject.toml"
  packageManager = "pip"
"#;
    let overlay_toml = r#"
schemaVersion = 1

[[repoSignals.languages]]
id = "python"
label = "Python"

  [[repoSignals.languages.manifests]]
  path = "setup.py"
  packageManager = "pip"
"#;
    let base = parse_catalog(base_toml, "base").expect("parse base");
    let overlay = parse_catalog(overlay_toml, "overlay").expect("parse overlay");
    let merged = merge_catalogs(&base, &overlay).expect("merge");
    let python = merged
        .repo_signals
        .languages
        .iter()
        .find(|l| l.id == "python")
        .expect("python found");
    assert_eq!(python.manifests.len(), 1);
    assert_eq!(python.manifests[0].path, "setup.py");
}

#[test]
fn repo_signals_overlay_adds_new_language() {
    let base_toml = r#"
schemaVersion = 1
[repoSignals]
[[repoSignals.languages]]
id = "python"
label = "Python"
"#;
    let overlay_toml = r#"
schemaVersion = 1
[[repoSignals.languages]]
id = "ruby"
label = "Ruby"
"#;
    let base = parse_catalog(base_toml, "base").expect("parse base");
    let overlay = parse_catalog(overlay_toml, "overlay").expect("parse overlay");
    let merged = merge_catalogs(&base, &overlay).expect("merge");
    assert_eq!(merged.repo_signals.languages.len(), 2);
    assert!(merged.repo_signals.languages.iter().any(|l| l.id == "ruby"));
}

#[test]
fn repo_signals_default_when_absent() {
    let toml = "schemaVersion = 1\n";
    let catalog = parse_catalog(toml, "test").expect("parse");
    assert!(catalog.repo_signals.languages.is_empty());
    assert_eq!(catalog.repo_signals.max_scan_depth, 0);
}

#[test]
fn repo_signals_language_references_deserialize_from_toml() {
    let toml = r#"
schemaVersion = 1

[repoSignals]

[[repoSignals.languages]]
id = "python"
label = "Python"

  [[repoSignals.languages.languageReferences]]
  groupId = "python"

    [[repoSignals.languages.languageReferences.items]]
    itemId = "core"

    [[repoSignals.languages.languageReferences.items]]
    itemId = "packaging"
    when = { any_of = ["task_kind:ConfigurationSupport", "task_action:ImplementLanguageVersionFeature"] }
"#;
    let catalog = parse_catalog(toml, "test").expect("parse");
    let python = &catalog.repo_signals.languages[0];
    assert_eq!(python.language_references.len(), 1);
    let lang_ref = &python.language_references[0];
    assert_eq!(lang_ref.group_id, "python");
    assert!(lang_ref.when.is_none());
    assert_eq!(lang_ref.items.len(), 2);
    assert_eq!(lang_ref.items[0].item_id, "core");
    assert!(lang_ref.items[0].when.is_none());
    assert_eq!(lang_ref.items[1].item_id, "packaging");
    assert!(lang_ref.items[1].when.is_some());
}

#[test]
fn repo_signals_overlay_replaces_language_references() {
    let base_toml = r#"
schemaVersion = 1
[repoSignals]
[[repoSignals.languages]]
id = "python"
label = "Python"

  [[repoSignals.languages.languageReferences]]
  groupId = "python"

    [[repoSignals.languages.languageReferences.items]]
    itemId = "core"
"#;
    let overlay_toml = r#"
schemaVersion = 1
[[repoSignals.languages]]
id = "python"
label = "Python"

  [[repoSignals.languages.languageReferences]]
  groupId = "python"

    [[repoSignals.languages.languageReferences.items]]
    itemId = "core"

    [[repoSignals.languages.languageReferences.items]]
    itemId = "typing"

    [[repoSignals.languages.languageReferences.items]]
    itemId = "async"
    when = "task_action:ImplementAsyncProcessing"
"#;
    let base = parse_catalog(base_toml, "base").expect("parse base");
    let overlay = parse_catalog(overlay_toml, "overlay").expect("parse overlay");
    let merged = merge_catalogs(&base, &overlay).expect("merge");
    let python = merged
        .repo_signals
        .languages
        .iter()
        .find(|l| l.id == "python")
        .expect("python found");
    assert_eq!(python.language_references.len(), 1);
    let lang_ref = &python.language_references[0];
    assert_eq!(lang_ref.items.len(), 3);
    assert!(lang_ref.items.iter().any(|i| i.item_id == "core"));
    assert!(lang_ref.items.iter().any(|i| i.item_id == "typing"));
    assert!(lang_ref
        .items
        .iter()
        .any(|i| i.item_id == "async" && i.when.is_some()));
}
