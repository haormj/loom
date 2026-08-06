//! Playbook stack-matchers engine.
//!
//! Step 3 of the Playbook migration. This module owns the data-driven stack
//! identification logic, replacing the hardcoded keyword `if/else-if` chain
//! in `signal_from_selection` (code_quality.rs:397) with a YAML matcher table
//! embedded at compile time from `playbooks/default/matchers.yaml`.
//!
//! The engine normalizes the raw selection text (lowercase, replace delimiters
//! with spaces, pad), then evaluates an ordered language chain (first match
//! wins). Within each matched language, framework and role rules fire. After
//! the chain, cross-cutting rules may add roles regardless of language.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use serde::Deserialize;

use crate::CodeStackSignal;

// ---------------------------------------------------------------------------
// Model
// ---------------------------------------------------------------------------

/// A track-based role rule. Replaces `role_from_track`.
#[derive(Debug, Clone, Deserialize)]
pub struct TrackRole {
    pub keywords: Vec<String>,
    pub role: String,
}

/// A framework push rule within a group or language entry.
///
/// In YAML, either `name` (push a single framework) or `group` (invoke a
/// named framework group) is set. Keyword conditions gate the push.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct FrameworkPush {
    /// Framework name to push (e.g. "spring_boot"). Absent when `group` is set.
    #[serde(default)]
    pub name: Option<String>,

    /// Reference to a framework group (e.g. "frontend", "spring"). Absent
    /// when `name` is set.
    #[serde(default)]
    pub group: Option<String>,

    /// Push if haystack contains any of these keywords.
    #[serde(default)]
    pub if_any: Vec<String>,

    /// Skip if haystack contains any of these keywords.
    #[serde(default)]
    pub unless_any: Vec<String>,

    /// Additionally require haystack to contain any of these (AND condition).
    #[serde(default)]
    pub requires_any: Vec<String>,
}

/// A dialect push rule (for SQL).
#[derive(Debug, Clone, Deserialize, Default)]
pub struct DialectPush {
    pub name: String,
    #[serde(default)]
    pub if_any: Vec<String>,
}

/// A role push rule within a language entry.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct RolePush {
    /// Role name to push (e.g. "backend"). Absent when
    /// `backend_unless_persistence` is set.
    #[serde(default)]
    pub name: Option<String>,

    /// Push if haystack contains any of these keywords.
    #[serde(default)]
    pub if_any: Vec<String>,

    /// Push "frontend" if any frontend framework keyword is mentioned.
    /// Uses `frontend_keywords` from the matchers file.
    #[serde(default)]
    pub if_frontend_mentioned: bool,

    /// Push "backend" unless "persistence" or "database" role already present.
    #[serde(default)]
    pub backend_unless_persistence: bool,

    /// Only push if any of these frameworks were already matched.
    #[serde(default)]
    pub if_frameworks_any: Vec<String>,
}

/// An ordered language matcher entry. First match wins.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct LanguageMatcher {
    /// Language to set on the signal. None for frontend-only/flutter-only
    /// branches.
    #[serde(default)]
    pub language: Option<String>,

    /// Match if haystack contains any of these keywords.
    #[serde(default)]
    pub any: Vec<String>,

    /// Skip if haystack contains any of these keywords.
    #[serde(default)]
    pub unless: Vec<String>,

    /// Shorthand: match if `selection_mentions_flutter_framework`.
    #[serde(default)]
    pub flutter: bool,

    /// Shorthand: match if `selection_mentions_frontend_framework`.
    #[serde(default)]
    pub frontend: bool,

    /// Framework push rules (may reference groups).
    #[serde(default)]
    pub frameworks: Vec<FrameworkPush>,

    /// Dialect push rules.
    #[serde(default)]
    pub dialects: Vec<DialectPush>,

    /// Role push rules.
    #[serde(default)]
    pub roles: Vec<RolePush>,
}

/// A cross-cutting rule that fires after the language chain.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct CrossCuttingRule {
    pub push_roles: Vec<String>,
    #[serde(default)]
    pub if_any: Vec<String>,
}

/// The complete matchers file, deserialized from `matchers.yaml`.
#[derive(Debug, Clone, Deserialize)]
pub struct MatchersFile {
    pub schema_version: u32,
    pub playbook: String,
    #[serde(default)]
    pub mode: String,
    #[serde(default)]
    pub track_roles: Vec<TrackRole>,
    #[serde(default)]
    pub framework_groups: BTreeMap<String, Vec<FrameworkPush>>,
    #[serde(default)]
    pub frontend_keywords: Vec<String>,
    pub languages: Vec<LanguageMatcher>,
    #[serde(default)]
    pub cross_cutting: Vec<CrossCuttingRule>,
}

const BUILTIN_MATCHERS_YAML: &str = include_str!("../../../playbooks/default/matchers.yaml");

/// Returns the builtin default matchers, parsed once and cached.
pub fn builtin_matchers() -> &'static MatchersFile {
    static MATCHERS: OnceLock<MatchersFile> = OnceLock::new();
    MATCHERS.get_or_init(|| {
        serde_yaml::from_str(BUILTIN_MATCHERS_YAML)
            .expect("builtin playbook matchers must parse; check playbooks/default/matchers.yaml")
    })
}

// ---------------------------------------------------------------------------
// Text normalization (must match code_quality.rs::normalized exactly)
// ---------------------------------------------------------------------------

fn normalized(value: &str) -> String {
    format!(
        " {} ",
        value
            .to_ascii_lowercase()
            .replace(['/', ',', '-', '_'], " ")
    )
}

fn contains_any(haystack: &str, needles: &[String]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle.as_str()))
}

fn push_unique(output: &mut Vec<String>, value: &str) {
    if !output.iter().any(|item| item == value) {
        output.push(value.to_string());
    }
}

// ---------------------------------------------------------------------------
// Evaluation
// ---------------------------------------------------------------------------

/// Evaluates the matchers against a raw stack selection, producing a
/// `CodeStackSignal`. Replaces `signal_from_selection`.
pub fn evaluate_signal(track: &str, source_path: &str, raw_selection: &str) -> CodeStackSignal {
    let matchers = builtin_matchers();
    let haystack = normalized(raw_selection);

    let mut roles = initial_roles_from_track(matchers, track);
    let mut frameworks = Vec::new();
    let mut dialects = Vec::new();
    let mut language = None;

    // Language chain — first match wins
    for entry in &matchers.languages {
        if !language_matches(matchers, entry, &haystack) {
            continue;
        }
        language = entry.language.clone();
        push_frameworks(matchers, entry, &haystack, &mut frameworks);
        push_dialects(entry, &haystack, &mut dialects);
        push_roles(matchers, entry, &haystack, &frameworks, &mut roles);
        break;
    }

    // Cross-cutting rules
    for rule in &matchers.cross_cutting {
        if rule.if_any.is_empty() || contains_any(&haystack, &rule.if_any) {
            for role in &rule.push_roles {
                push_unique(&mut roles, role);
            }
        }
    }

    let mapped = language.is_some() || !frameworks.is_empty() || !dialects.is_empty();
    let confidence = if mapped { "high" } else { "low" }.to_string();
    let reason = if language.is_some() {
        "Mapped language from confirmed TechnicalBaseline stack selection.".to_string()
    } else if !frameworks.is_empty() {
        "Mapped framework from confirmed TechnicalBaseline stack selection.".to_string()
    } else if !dialects.is_empty() {
        "Mapped storage dialect from confirmed TechnicalBaseline stack selection.".to_string()
    } else {
        "No known Loom code reference profile matched this stack selection.".to_string()
    };

    CodeStackSignal {
        source_track: track.to_string(),
        source_path: source_path.to_string(),
        raw_selection: raw_selection.to_string(),
        language,
        frameworks,
        dialects,
        roles,
        confidence,
        reason,
    }
}

fn initial_roles_from_track(matchers: &MatchersFile, track: &str) -> Vec<String> {
    let normalized_track = normalized(track);
    let mut roles = Vec::new();
    for rule in &matchers.track_roles {
        if contains_any(&normalized_track, &rule.keywords) {
            push_unique(&mut roles, &rule.role);
        }
    }
    roles
}

fn language_matches(matchers: &MatchersFile, entry: &LanguageMatcher, haystack: &str) -> bool {
    if entry.flutter {
        let flutter_keywords = matchers
            .framework_groups
            .get("frontend")
            .map(|group| {
                group
                    .iter()
                    .find(|f| f.name.as_deref() == Some("flutter"))
                    .map(|f| f.if_any.clone())
                    .unwrap_or_default()
            })
            .unwrap_or_default();
        if !contains_any(haystack, &flutter_keywords) {
            return false;
        }
    }
    if entry.frontend {
        if !contains_any(haystack, &matchers.frontend_keywords) {
            return false;
        }
    }
    if !entry.any.is_empty() && !contains_any(haystack, &entry.any) {
        return false;
    }
    if !entry.unless.is_empty() && contains_any(haystack, &entry.unless) {
        return false;
    }
    if entry.any.is_empty() && !entry.flutter && !entry.frontend {
        return false;
    }
    true
}

fn push_frameworks(
    matchers: &MatchersFile,
    entry: &LanguageMatcher,
    haystack: &str,
    frameworks: &mut Vec<String>,
) {
    for push in &entry.frameworks {
        if let Some(group_name) = &push.group {
            if let Some(group) = matchers.framework_groups.get(group_name) {
                for member in group {
                    push_single_framework(member, haystack, frameworks);
                }
            }
        } else {
            push_single_framework(push, haystack, frameworks);
        }
    }
}

fn push_single_framework(
    push: &FrameworkPush,
    haystack: &str,
    frameworks: &mut Vec<String>,
) {
    let Some(ref name) = push.name else {
        return;
    };
    if !push.if_any.is_empty() && !contains_any(haystack, &push.if_any) {
        return;
    }
    if !push.unless_any.is_empty() && contains_any(haystack, &push.unless_any) {
        return;
    }
    if !push.requires_any.is_empty() && !contains_any(haystack, &push.requires_any) {
        return;
    }
    push_unique(frameworks, name);
}

fn push_dialects(entry: &LanguageMatcher, haystack: &str, dialects: &mut Vec<String>) {
    for push in &entry.dialects {
        if push.if_any.is_empty() || contains_any(haystack, &push.if_any) {
            push_unique(dialects, &push.name);
        }
    }
}

fn push_roles(
    matchers: &MatchersFile,
    entry: &LanguageMatcher,
    haystack: &str,
    frameworks: &[String],
    roles: &mut Vec<String>,
) {
    for rule in &entry.roles {
        if rule.backend_unless_persistence {
            if !roles.iter().any(|r| r == "persistence" || r == "database") {
                if !rule.if_frameworks_any.is_empty() {
                    if !rule
                        .if_frameworks_any
                        .iter()
                        .any(|fw| frameworks.contains(fw))
                    {
                        continue;
                    }
                }
                push_unique(roles, "backend");
            }
            continue;
        }
        let Some(ref name) = rule.name else {
            continue;
        };
        if rule.if_frontend_mentioned {
            if contains_any(haystack, &matchers.frontend_keywords) {
                push_unique(roles, name);
            }
            continue;
        }
        if !rule.if_any.is_empty() {
            if contains_any(haystack, &rule.if_any) {
                push_unique(roles, name);
            }
            continue;
        }
        if !rule.if_frameworks_any.is_empty() {
            if rule
                .if_frameworks_any
                .iter()
                .any(|fw| frameworks.contains(fw))
            {
                push_unique(roles, name);
            }
            continue;
        }
        push_unique(roles, name);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_matchers_parse() {
        let m = builtin_matchers();
        assert_eq!(m.schema_version, 1);
        assert_eq!(m.playbook, "default");
        assert_eq!(m.mode, "base");
        assert!(!m.track_roles.is_empty());
        assert!(m.framework_groups.contains_key("frontend"));
        assert!(m.framework_groups.contains_key("spring"));
        assert!(!m.frontend_keywords.is_empty());
        assert!(!m.languages.is_empty());
        assert!(!m.cross_cutting.is_empty());
    }

    #[test]
    fn typescript_detection() {
        let signal = evaluate_signal("backend", "stack.language", "TypeScript");
        assert_eq!(signal.language.as_deref(), Some("typescript"));
        assert!(signal.confidence == "high");
    }

    #[test]
    fn java_with_spring_excludes_kotlin() {
        let signal = evaluate_signal("backend", "stack.language", "Java Spring Boot");
        assert_eq!(signal.language.as_deref(), Some("java"));
        assert!(signal.frameworks.contains(&"spring_boot".to_string()));
        assert!(signal.roles.contains(&"backend".to_string()));
    }

    #[test]
    fn kotlin_spring_triggers_backend() {
        let signal = evaluate_signal("backend", "stack.language", "Kotlin Spring Boot");
        assert_eq!(signal.language.as_deref(), Some("kotlin"));
        assert!(signal.frameworks.contains(&"spring_boot".to_string()));
        assert!(signal.roles.contains(&"backend".to_string()));
    }

    #[test]
    fn react_native_excludes_react() {
        let signal = evaluate_signal("web", "stack.framework", "React Native");
        assert!(signal.frameworks.contains(&"reactnative".to_string()));
        assert!(!signal.frameworks.contains(&"react".to_string()));
    }

    #[test]
    fn sql_dialects_detected() {
        let signal = evaluate_signal("database", "stack.database", "PostgreSQL");
        assert_eq!(signal.language.as_deref(), Some("sql"));
        assert!(signal.dialects.contains(&"postgresql".to_string()));
        assert!(signal.roles.contains(&"database".to_string()));
    }

    #[test]
    fn unmapped_signal_is_low_confidence() {
        let signal = evaluate_signal("backend", "stack.language", "brainfuck");
        assert!(signal.language.is_none());
        assert!(signal.frameworks.is_empty());
        assert_eq!(signal.confidence, "low");
    }

    #[test]
    fn cross_cutting_persistence_role() {
        let signal = evaluate_signal("backend", "stack.orm", "JPA Hibernate");
        assert!(signal.roles.contains(&"persistence".to_string()));
    }

    #[test]
    fn frontend_only_branch() {
        let signal = evaluate_signal("web", "stack.framework", "Vue");
        assert!(signal.language.is_none());
        assert!(signal.frameworks.contains(&"vue".to_string()));
        assert!(signal.roles.contains(&"frontend".to_string()));
    }

    #[test]
    fn flutter_only_branch() {
        let signal = evaluate_signal("app", "stack.framework", "Flutter Riverpod");
        assert!(signal.language.is_none());
        assert!(signal.frameworks.contains(&"flutter".to_string()));
        assert!(signal.frameworks.contains(&"riverpod".to_string()));
        assert!(signal.roles.contains(&"frontend".to_string()));
    }

    #[test]
    fn bloc_requires_flutter_family() {
        let signal = evaluate_signal("app", "stack.framework", "Flutter Bloc");
        assert!(signal.frameworks.contains(&"bloc".to_string()));
    }

    #[test]
    fn bloc_alone_not_pushed() {
        let signal = evaluate_signal("app", "stack.framework", "bloc");
        assert!(!signal.frameworks.contains(&"bloc".to_string()));
    }

    #[test]
    fn persistence_track_suppresses_backend() {
        let signal = evaluate_signal("persistence", "stack.orm", "Java JPA");
        assert_eq!(signal.language.as_deref(), Some("java"));
        assert!(!signal.roles.contains(&"backend".to_string()));
    }
}
