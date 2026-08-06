//! Playbook pack loading and overlay.
//!
//! Step 4 of the Playbook migration. This module owns the enterprise pack
//! model: a `manifest.toml` declaring `mode: base | extend`, plus the ability
//! to load a pack from disk and merge it with the builtin default.
//!
//! In `extend` mode, registry/rules/matchers entries are key-level merged
//! (override same key, append to ordered lists). In `base` mode, the
//! enterprise pack replaces the builtin wholesale.
//!
//! The builtin pack is always embedded via `include_str!`; enterprise packs
//! are loaded from `~/.loom/playbooks/<name>/` or `<project>/.loom/playbook/`.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use crate::playbook::{PlaybookRegistry, RefEntry};

// ---------------------------------------------------------------------------
// Manifest
// ---------------------------------------------------------------------------

/// The pack manifest, parsed from `manifest.toml`.
#[derive(Debug, Clone)]
pub struct PackManifest {
    pub name: String,
    pub loom_version_range: Option<String>,
    pub mode: String,
}

impl PackManifest {
    pub fn is_base(&self) -> bool {
        self.mode == "base"
    }

    /// Parses a manifest from TOML text. Handles simple `key = "value"` lines;
    /// comments (`#`) and blank lines are skipped.
    pub fn parse(toml_str: &str) -> Result<Self, String> {
        let kv = parse_simple_toml(toml_str);
        let name = kv
            .get("name")
            .ok_or("manifest.toml missing 'name'")?
            .clone();
        let loom_version_range = kv.get("loom_version_range").cloned();
        let mode = kv
            .get("mode")
            .cloned()
            .unwrap_or_else(|| "extend".to_string());
        Ok(PackManifest {
            name,
            loom_version_range,
            mode,
        })
    }
}

// ---------------------------------------------------------------------------
// Active config (.loom/playbook.toml)
// ---------------------------------------------------------------------------

/// Project-level playbook config, parsed from `.loom/playbook.toml`.
#[derive(Debug, Clone)]
pub struct PlaybookConfig {
    pub active_playbook: Option<String>,
}

impl PlaybookConfig {
    /// Parses a playbook config from TOML text.
    pub fn parse(toml_str: &str) -> Result<Self, String> {
        let kv = parse_simple_toml(toml_str);
        Ok(PlaybookConfig {
            active_playbook: kv.get("active_playbook").cloned(),
        })
    }
}

// ---------------------------------------------------------------------------
// Merged registry (builtin + enterprise overlay)
// ---------------------------------------------------------------------------

/// A merged registry that combines the builtin default with an optional
/// enterprise overlay. In `extend` mode, enterprise entries override
/// builtin entries with the same `(group_key, group)` key. In `base` mode,
/// only the enterprise registry is used.
#[derive(Debug, Clone)]
pub struct MergedRegistry {
    pub code_common: RefEntry,
    pub code_refs: BTreeMap<String, BTreeMap<String, RefEntry>>,
}

/// Returns the effective registry. If no enterprise pack is loaded, this is
/// the builtin default. If an enterprise pack is loaded in `extend` mode,
/// entries are merged. In `base` mode, the enterprise registry replaces.
pub fn effective_registry() -> &'static MergedRegistry {
    static REGISTRY: OnceLock<MergedRegistry> = OnceLock::new();
    REGISTRY.get_or_init(|| MergedRegistry {
        code_common: builtin_registry().code_common.clone(),
        code_refs: builtin_registry().code_refs.clone(),
    })
}

/// Merges an enterprise registry on top of the builtin, returning a new
/// `MergedRegistry`. In `extend` mode, enterprise entries override same-key
/// builtin entries. In `base` mode, the enterprise registry is used alone.
pub fn merge_registry(
    builtin: &PlaybookRegistry,
    enterprise: Option<&PlaybookRegistry>,
) -> MergedRegistry {
    let Some(enterprise) = enterprise else {
        return MergedRegistry {
            code_common: builtin.code_common.clone(),
            code_refs: builtin.code_refs.clone(),
        };
    };
    if enterprise.mode == "base" {
        return MergedRegistry {
            code_common: enterprise.code_common.clone(),
            code_refs: enterprise.code_refs.clone(),
        };
    }
    let mut merged: BTreeMap<String, BTreeMap<String, RefEntry>> = builtin.code_refs.clone();
    for (group_key, groups) in &enterprise.code_refs {
        let entry = merged.entry(group_key.clone()).or_default();
        for (group, ref_entry) in groups {
            entry.insert(group.clone(), ref_entry.clone());
        }
    }
    let code_common = if enterprise.code_common.ref_id.is_empty() {
        builtin.code_common.clone()
    } else {
        enterprise.code_common.clone()
    };
    MergedRegistry {
        code_common,
        code_refs: merged,
    }
}

fn builtin_registry() -> &'static PlaybookRegistry {
    crate::playbook::builtin_registry()
}

// ---------------------------------------------------------------------------
// File-list derivation
// ---------------------------------------------------------------------------

/// Returns the set of reference file paths (relative to
/// `plugins/shared/loom/references/`) that the builtin registry declares.
/// This is used by `setup` to derive the code-reference portion of
/// `REQUIRED_SHARED_REFERENCE_FILES` from the registry, so adding/removing
/// a ref in YAML automatically updates the install file list.
pub fn registry_reference_paths() -> Vec<String> {
    let registry = effective_registry();
    let mut paths = Vec::new();
    paths.push(registry.code_common.path.clone());
    for groups in registry.code_refs.values() {
        for entry in groups.values() {
            paths.push(entry.path.clone());
        }
    }
    paths.sort();
    paths.dedup();
    paths
}

// ---------------------------------------------------------------------------
// Enterprise pack loading
// ---------------------------------------------------------------------------

/// Attempts to load an enterprise playbook from a directory. Returns `None`
/// if the directory doesn't exist or doesn't contain a `manifest.toml`.
pub fn load_enterprise_pack(dir: &std::path::Path) -> Option<LoadedPack> {
    let manifest_path = dir.join("manifest.toml");
    if !manifest_path.is_file() {
        return None;
    }
    let manifest_content = std::fs::read_to_string(&manifest_path).ok()?;
    let manifest = PackManifest::parse(&manifest_content).ok()?;
    let registry = load_registry(dir);
    Some(LoadedPack {
        manifest,
        registry,
    })
}

/// A loaded enterprise pack.
#[derive(Debug, Clone)]
pub struct LoadedPack {
    pub manifest: PackManifest,
    pub registry: Option<PlaybookRegistry>,
}

fn load_registry(dir: &std::path::Path) -> Option<PlaybookRegistry> {
    let registry_path = dir.join("registry.yaml");
    if !registry_path.is_file() {
        return None;
    }
    let content = std::fs::read_to_string(&registry_path).ok()?;
    serde_yaml::from_str(&content).ok()
}

/// Minimal TOML parser for flat `key = "value"` files (manifest.toml,
/// playbook.toml). Strips comments and handles quoted strings. This avoids
/// adding a toml dependency to the contracts crate.
fn parse_simple_toml(toml_str: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for line in toml_str.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(eq_pos) = line.find('=') {
            let key = line[..eq_pos].trim().to_string();
            let mut value = line[eq_pos + 1..].trim().to_string();
            if value.starts_with('#') {
                continue;
            }
            if let Some(comment_pos) = value.find(" #") {
                value = value[..comment_pos].trim().to_string();
            }
            if (value.starts_with('"') && value.ends_with('"') && value.len() >= 2)
                || (value.starts_with('\'') && value.ends_with('\'') && value.len() >= 2)
            {
                value = value[1..value.len() - 1].to_string();
            }
            map.insert(key, value);
        }
    }
    map
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_parses_extend_mode() {
        let toml_str = r#"
name = "acme"
loom_version_range = ">=0.2.7"
mode = "extend"
"#;
        let manifest = PackManifest::parse(toml_str).unwrap();
        assert_eq!(manifest.name, "acme");
        assert_eq!(manifest.mode, "extend");
        assert!(!manifest.is_base());
    }

    #[test]
    fn manifest_defaults_to_extend() {
        let toml_str = r#"
name = "test"
"#;
        let manifest = PackManifest::parse(toml_str).unwrap();
        assert_eq!(manifest.mode, "extend");
    }

    #[test]
    fn manifest_base_mode_detected() {
        let toml_str = r#"
name = "minimal"
mode = "base"
"#;
        let manifest = PackManifest::parse(toml_str).unwrap();
        assert!(manifest.is_base());
    }

    #[test]
    fn registry_reference_paths_includes_common_and_code_refs() {
        let paths = registry_reference_paths();
        assert!(paths.contains(&"tech/code/common.md".to_string()));
        assert!(paths.contains(&"tech/backend/springboot/security.md".to_string()));
        assert!(paths.contains(&"tech/frontend/react/core.md".to_string()));
        assert!(paths.contains(&"tech/code/java/core.md".to_string()));
    }

    #[test]
    fn merge_registry_extend_overrides_same_key() {
        let builtin = builtin_registry();
        let mut enterprise = PlaybookRegistry {
            schema_version: 1,
            playbook: "acme".to_string(),
            mode: "extend".to_string(),
            code_common: builtin.code_common.clone(),
            code_refs: BTreeMap::new(),
        };
        let mut springboot = BTreeMap::new();
        springboot.insert(
            "security".to_string(),
            RefEntry {
                ref_id: "acme.spring.security".to_string(),
                path: "acme/spring/security.md".to_string(),
                reason: "Enterprise security override".to_string(),
            },
        );
        enterprise.code_refs.insert("springboot".to_string(), springboot);

        let merged = merge_registry(builtin, Some(&enterprise));
        let sec = merged
            .code_refs
            .get("springboot")
            .and_then(|g| g.get("security"))
            .unwrap();
        assert_eq!(sec.ref_id, "acme.spring.security");
        assert_eq!(sec.path, "acme/spring/security.md");

        let web = merged
            .code_refs
            .get("springboot")
            .and_then(|g| g.get("web"))
            .unwrap();
        assert_eq!(web.ref_id, "bk.spring.web");
    }

    #[test]
    fn merge_registry_base_replaces_entirely() {
        let builtin = builtin_registry();
        let enterprise = PlaybookRegistry {
            schema_version: 1,
            playbook: "minimal".to_string(),
            mode: "base".to_string(),
            code_common: RefEntry {
                ref_id: "acme.common".to_string(),
                path: "acme/common.md".to_string(),
                reason: "Enterprise common".to_string(),
            },
            code_refs: BTreeMap::new(),
        };

        let merged = merge_registry(builtin, Some(&enterprise));
        assert_eq!(merged.code_common.ref_id, "acme.common");
        assert!(merged.code_refs.is_empty());
    }

    #[test]
    fn merge_registry_no_enterprise_returns_builtin() {
        let builtin = builtin_registry();
        let merged = merge_registry(builtin, None);
        assert_eq!(merged.code_common.ref_id, builtin.code_common.ref_id);
        assert_eq!(merged.code_refs.len(), builtin.code_refs.len());
    }

    #[test]
    fn playbook_config_parses() {
        let toml_str = r#"
active_playbook = "acme"
"#;
        let config = PlaybookConfig::parse(toml_str).unwrap();
        assert_eq!(config.active_playbook.as_deref(), Some("acme"));
    }

    #[test]
    fn playbook_config_empty_is_ok() {
        let toml_str = "";
        let config = PlaybookConfig::parse(toml_str).unwrap();
        assert!(config.active_playbook.is_none());
    }
}
