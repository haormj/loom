//! Builtin Playbook registry.
//!
//! A Playbook is Loom's externalized "best practice" layer: the taxonomy of
//! reference ids, the file path each resolves to, and (in later steps) the
//! selection rules and stack matchers. This module owns the builtin/default
//! registry, embedded at compile time from `playbooks/default/registry.yaml`.
//!
//! Step 1 wires only the code-quality reference path table (`code_refs` and the
//! always-prepended `code_common`). The lookup is data-driven; the prior
//! hardcoded `format!`/literal mapping in `code_quality.rs` is retained as a
//! safety fallback so behavior stays byte-equivalent.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use serde::Deserialize;

/// A single registry entry: the stable reference id, its repository-relative
/// content path, and the human-readable selection reason.
#[derive(Debug, Clone, Deserialize)]
pub struct RefEntry {
    pub ref_id: String,
    pub path: String,
    pub reason: String,
}

/// The deserialized builtin Playbook registry.
///
/// `code_refs` is keyed by `(group_key, group)` — the same pair the prior
/// `reference_load_plan_item` matched on. Future steps add `api_refs`,
/// `ui_refs`, `test_refs`, `rules`, and `matchers`.
#[derive(Debug, Clone, Deserialize)]
pub struct PlaybookRegistry {
    pub schema_version: u32,
    pub playbook: String,
    #[serde(default)]
    pub mode: String,
    pub code_common: RefEntry,
    #[serde(default)]
    pub code_refs: BTreeMap<String, BTreeMap<String, RefEntry>>,
}

const BUILTIN_REGISTRY_YAML: &str = include_str!("../../../playbooks/default/registry.yaml");

/// Returns the builtin default Playbook registry, parsed once and cached for
/// the process lifetime. A malformed builtin registry is a build-time authoring
/// error and panics on first access rather than degrading silently.
pub fn builtin_registry() -> &'static PlaybookRegistry {
    static REGISTRY: OnceLock<PlaybookRegistry> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        serde_yaml::from_str(BUILTIN_REGISTRY_YAML)
            .expect("builtin playbook registry must parse; check playbooks/default/registry.yaml")
    })
}

/// Looks up a code-quality reference by its `(group_key, group)` pair.
///
/// Returns `None` for pairs absent from the registry; callers keep a fallback
/// so an incomplete builtin table never changes behavior.
pub fn code_ref(group_key: &str, group: &str) -> Option<&'static RefEntry> {
    builtin_registry()
        .code_refs
        .get(group_key)
        .and_then(|groups| groups.get(group))
}

/// Returns the always-prepended common code-quality reference entry.
pub fn code_common() -> &'static RefEntry {
    &builtin_registry().code_common
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_registry_parses_and_is_default_base() {
        let registry = builtin_registry();
        assert_eq!(registry.schema_version, 1);
        assert_eq!(registry.playbook, "default");
        assert_eq!(registry.mode, "base");
        assert_eq!(registry.code_common.ref_id, "tech.code.common");
        assert_eq!(registry.code_common.path, "tech/code/common.md");
    }

    #[test]
    fn code_ref_resolves_backend_frontend_and_special_entries() {
        assert_eq!(
            code_ref("springboot", "security").unwrap().ref_id,
            "bk.spring.security"
        );
        assert_eq!(
            code_ref("springboot", "security").unwrap().path,
            "tech/backend/springboot/security.md"
        );
        assert_eq!(
            code_ref("react", "core").unwrap().ref_id,
            "fe.react.core"
        );
        assert_eq!(
            code_ref("mybatisplus", "crud").unwrap().ref_id,
            "bk.spring.mybatisplus.crud"
        );
        assert_eq!(
            code_ref("sql", "mysql.transactions").unwrap().ref_id,
            "tech.code.sql.mysql.transactions"
        );
        assert_eq!(
            code_ref("redis", "cache").unwrap().ref_id,
            "tech.code.redis.cache"
        );
        assert_eq!(
            code_ref("common", "observability").unwrap().ref_id,
            "tech.code.observability"
        );
    }

    #[test]
    fn code_ref_returns_none_for_unknown_pair() {
        assert!(code_ref("nonexistent", "group").is_none());
    }

    #[test]
    fn code_common_matches_prior_hardcoded_value() {
        let common = code_common();
        assert_eq!(common.ref_id, "tech.code.common");
        assert_eq!(common.path, "tech/code/common.md");
        assert_eq!(
            common.reason,
            "Common Loom code quality rules for repository adaptation, delivery evidence, and verification."
        );
    }
}
