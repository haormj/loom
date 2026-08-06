//! Playbook selection-rule engine.
//!
//! Step 2 of the Playbook migration. This module owns the data-driven
//! evaluation of reference selection rules, replacing the hardcoded
//! `match`/`if` branches in `code_quality.rs` with a YAML rule table
//! embedded at compile time from `playbooks/default/rules.yaml`.
//!
//! The engine evaluates three kinds of rules:
//! - **Applicability**: per-language conditions determining whether a
//!   `CodeStackSignal` is relevant to the task (replaces
//!   `signal_applies_to_task`).
//! - **Language items**: per-language reference group selection (replaces
//!   `reference_items_for_signal`).
//! - **Framework items**: backend/frontend framework reference group
//!   selection (replaces `backend_reference_items_for_signal` and
//!   `frontend_reference_items_for_signal`).
//!
//! Named ownership predicates (`task_owns_persistence`, etc.) remain Rust
//! functions — they are the predicate *vocabulary* rules reference by name.
//! The resolver `crate::code_quality::resolve_task_predicate` maps a string
//! name to the corresponding Rust function call.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use serde::Deserialize;

use crate::{
    CodeReferenceTaskContext, CodeStackSignal, ImplementationAction, TaskDefinition, TaskKind,
};

// ---------------------------------------------------------------------------
// Condition model
// ---------------------------------------------------------------------------

/// A composable condition. All set fields are AND'd together; `all`, `any`,
/// and `not` provide sub-condition combinators for complex boolean logic.
///
/// In YAML, a condition with only primitive fields is a flat map:
/// ```yaml
/// when: { language: java, focus: [security] }
/// ```
///
/// For OR / nested AND, use combinators:
/// ```yaml
/// when:
///   language: java
///   any:
///     - focus: [reactive]
///     - all: [focus: [api], signal_frameworks: [spring_webflux]]
/// ```
#[derive(Debug, Clone, Deserialize, Default)]
pub struct Condition {
    /// `signal.language == Some(value)`
    #[serde(default)]
    pub language: Option<String>,

    /// `signal.language.is_none()`
    #[serde(default)]
    pub language_is_none: bool,

    /// `signal.language.is_some()`
    #[serde(default)]
    pub language_is_some: bool,

    /// `signal.language.is_some() && !value.contains(language)`
    #[serde(default)]
    pub language_not_in: Vec<String>,

    /// `signal.frameworks` contains any of these (OR)
    #[serde(default)]
    pub signal_frameworks: Vec<String>,

    /// `signal.frameworks` contains ALL of these (AND)
    #[serde(default)]
    pub all_signal_frameworks: Vec<String>,

    /// `signal.frameworks` contains none of these
    #[serde(default)]
    pub not_signal_frameworks: Vec<String>,

    /// `stack_frameworks` contains any of these (OR)
    #[serde(default)]
    pub stack_frameworks: Vec<String>,

    /// `stack_frameworks` contains ALL of these (AND)
    #[serde(default)]
    pub all_stack_frameworks: Vec<String>,

    /// `stack_frameworks` contains none of these
    #[serde(default)]
    pub not_stack_frameworks: Vec<String>,

    /// `focus_tags` contains any of these (OR)
    #[serde(default)]
    pub focus: Vec<String>,

    /// `signal.roles` contains any of these (OR)
    #[serde(default)]
    pub roles: Vec<String>,

    /// `signal.roles` is empty
    #[serde(default)]
    pub roles_empty: bool,

    /// `signal.dialects` contains any of these (OR)
    #[serde(default)]
    pub dialects: Vec<String>,

    /// `task.task_kind` matches any of these snake_case names (OR)
    #[serde(default)]
    pub task_kind: Vec<String>,

    /// `task.implementation_actions` contains any of these (OR)
    #[serde(default)]
    pub actions: Vec<String>,

    /// Any named ownership predicate is true (OR). Names map to Rust
    /// functions via `resolve_task_predicate`.
    #[serde(default)]
    pub owns: Vec<String>,

    /// None of the named ownership predicates are true
    #[serde(default)]
    pub not_owns: Vec<String>,

    /// Any context flag is true (OR). Names: security, async_processing,
    /// integration, resilience, observability, request_tracing,
    /// application_architecture.
    #[serde(default)]
    pub context_flags: Vec<String>,

    /// All sub-conditions must be true (AND)
    #[serde(default)]
    pub all: Vec<Condition>,

    /// At least one sub-condition must be true (OR)
    #[serde(default)]
    pub any: Vec<Condition>,

    /// Sub-condition must be false (NOT)
    #[serde(default)]
    pub not: Option<Box<Condition>>,
}

// ---------------------------------------------------------------------------
// Rule models
// ---------------------------------------------------------------------------

/// A framework-level selection: which `group_key` and `group` to add.
#[derive(Debug, Clone, Deserialize)]
pub struct FrameworkSelection {
    pub group_key: String,
    pub group: String,
}

/// A language-level rule: condition + list of groups to select.
/// The group_key is implicitly the signal's language.
#[derive(Debug, Clone, Deserialize)]
pub struct LanguageRule {
    pub when: Condition,
    pub select: Vec<String>,
}

/// A framework-level rule: condition + list of selections.
#[derive(Debug, Clone, Deserialize)]
pub struct FrameworkRule {
    pub when: Condition,
    pub select: Vec<FrameworkSelection>,
}

/// The complete builtin rules file, deserialized from `rules.yaml`.
#[derive(Debug, Clone, Deserialize)]
pub struct RulesFile {
    pub schema_version: u32,
    pub playbook: String,
    #[serde(default)]
    pub mode: String,
    /// Map from language name to applicability condition. Special keys:
    /// `"*"` = `Some(_)` catch-all, `"_none"` = `None`.
    #[serde(default)]
    pub applicability: BTreeMap<String, Condition>,
    #[serde(default)]
    pub language_rules: Vec<LanguageRule>,
    #[serde(default)]
    pub backend_rules: Vec<FrameworkRule>,
    #[serde(default)]
    pub frontend_rules: Vec<FrameworkRule>,
}

const BUILTIN_RULES_YAML: &str = include_str!("../../../playbooks/default/rules.yaml");

/// Returns the builtin default rules, parsed once and cached.
pub fn builtin_rules() -> &'static RulesFile {
    static RULES: OnceLock<RulesFile> = OnceLock::new();
    RULES.get_or_init(|| {
        serde_yaml::from_str(BUILTIN_RULES_YAML)
            .expect("builtin playbook rules must parse; check playbooks/default/rules.yaml")
    })
}

// ---------------------------------------------------------------------------
// Evaluation input
// ---------------------------------------------------------------------------

/// All inputs needed to evaluate a condition against the current task/signal.
pub struct EvaluationInput<'a> {
    pub signal: &'a CodeStackSignal,
    pub focus_tags: &'a [String],
    pub task: &'a TaskDefinition,
    pub stack_frameworks: &'a BTreeSet<String>,
    pub context: &'a CodeReferenceTaskContext,
}

// ---------------------------------------------------------------------------
// Core condition evaluation
// ---------------------------------------------------------------------------

/// Evaluates a condition against the given input. An empty condition
/// (all defaults) evaluates to `true`.
pub fn evaluate_condition(condition: &Condition, input: &EvaluationInput) -> bool {
    if let Some(ref lang) = condition.language {
        if input.signal.language.as_deref() != Some(lang.as_str()) {
            return false;
        }
    }
    if condition.language_is_none && input.signal.language.is_some() {
        return false;
    }
    if condition.language_is_some && input.signal.language.is_none() {
        return false;
    }
    if !condition.language_not_in.is_empty() {
        match &input.signal.language {
            Some(lang) if condition.language_not_in.iter().any(|l| l == lang) => return false,
            None => return false,
            _ => {}
        }
    }
    if !condition.signal_frameworks.is_empty()
        && !condition
            .signal_frameworks
            .iter()
            .any(|fw| input.signal.frameworks.contains(fw))
    {
        return false;
    }
    if !condition.all_signal_frameworks.is_empty()
        && !condition
            .all_signal_frameworks
            .iter()
            .all(|fw| input.signal.frameworks.contains(fw))
    {
        return false;
    }
    if !condition.not_signal_frameworks.is_empty()
        && condition
            .not_signal_frameworks
            .iter()
            .any(|fw| input.signal.frameworks.contains(fw))
    {
        return false;
    }
    if !condition.stack_frameworks.is_empty()
        && !condition
            .stack_frameworks
            .iter()
            .any(|fw| input.stack_frameworks.contains(fw))
    {
        return false;
    }
    if !condition.all_stack_frameworks.is_empty()
        && !condition
            .all_stack_frameworks
            .iter()
            .all(|fw| input.stack_frameworks.contains(fw))
    {
        return false;
    }
    if !condition.not_stack_frameworks.is_empty()
        && condition
            .not_stack_frameworks
            .iter()
            .any(|fw| input.stack_frameworks.contains(fw))
    {
        return false;
    }
    if !condition.focus.is_empty()
        && !condition
            .focus
            .iter()
            .any(|tag| input.focus_tags.iter().any(|t| t == tag))
    {
        return false;
    }
    if !condition.roles.is_empty()
        && !condition
            .roles
            .iter()
            .any(|role| input.signal.roles.contains(role))
    {
        return false;
    }
    if condition.roles_empty && !input.signal.roles.is_empty() {
        return false;
    }
    if !condition.dialects.is_empty()
        && !condition
            .dialects
            .iter()
            .any(|d| input.signal.dialects.contains(d))
    {
        return false;
    }
    if !condition.task_kind.is_empty()
        && !condition.task_kind.iter().any(|name| {
            parse_task_kind(name)
                .map(|kind| input.task.task_kind == kind)
                .unwrap_or(false)
        })
    {
        return false;
    }
    if !condition.actions.is_empty()
        && !condition.actions.iter().any(|name| {
            parse_implementation_action(name)
                .map(|action| input.task.implementation_actions.contains(&action))
                .unwrap_or(false)
        })
    {
        return false;
    }
    if !condition.owns.is_empty()
        && !condition.owns.iter().any(|name| {
            crate::code_quality::resolve_task_predicate(name, input.task, input.context)
        })
    {
        return false;
    }
    if !condition.not_owns.is_empty()
        && condition.not_owns.iter().any(|name| {
            crate::code_quality::resolve_task_predicate(name, input.task, input.context)
        })
    {
        return false;
    }
    if !condition.context_flags.is_empty()
        && !condition
            .context_flags
            .iter()
            .any(|name| context_flag_is_true(input.context, name))
    {
        return false;
    }
    if !condition.all.is_empty()
        && !condition.all.iter().all(|sub| evaluate_condition(sub, input))
    {
        return false;
    }
    if !condition.any.is_empty()
        && !condition.any.iter().any(|sub| evaluate_condition(sub, input))
    {
        return false;
    }
    if let Some(ref sub) = condition.not {
        if evaluate_condition(sub, input) {
            return false;
        }
    }
    true
}

// ---------------------------------------------------------------------------
// Section evaluation functions
// ---------------------------------------------------------------------------

/// Evaluates whether a signal applies to the task, based on the signal's
/// language and the applicability rules. Replaces `signal_applies_to_task`.
pub fn evaluate_signal_applicability(
    signal: &CodeStackSignal,
    focus_tags: &[String],
    task: &TaskDefinition,
    stack_frameworks: &BTreeSet<String>,
    context: &CodeReferenceTaskContext,
) -> bool {
    let rules = builtin_rules();
    let key = match signal.language.as_deref() {
        Some(lang) if rules.applicability.contains_key(lang) => lang.to_string(),
        Some(_) => "*".to_string(),
        None => "_none".to_string(),
    };
    let Some(condition) = rules.applicability.get(&key) else {
        return false;
    };
    let input = EvaluationInput {
        signal,
        focus_tags,
        task,
        stack_frameworks,
        context,
    };
    evaluate_condition(condition, &input)
}

/// Evaluates language-level reference items for a signal. Replaces
/// `reference_items_for_signal`. Returns a set of group names; the
/// caller keys them under the signal's language.
pub fn evaluate_language_items(
    signal: &CodeStackSignal,
    focus_tags: &[String],
    task: &TaskDefinition,
    stack_frameworks: &BTreeSet<String>,
    context: &CodeReferenceTaskContext,
) -> BTreeSet<String> {
    let rules = builtin_rules();
    let input = EvaluationInput {
        signal,
        focus_tags,
        task,
        stack_frameworks,
        context,
    };
    let mut items = BTreeSet::new();
    for rule in &rules.language_rules {
        if evaluate_condition(&rule.when, &input) {
            items.extend(rule.select.iter().cloned());
        }
    }
    items
}

/// Evaluates backend framework reference items for a signal. Replaces
/// `backend_reference_items_for_signal`. Returns a map from group_key
/// (e.g. "springboot") to a set of groups.
pub fn evaluate_backend_items(
    signal: &CodeStackSignal,
    focus_tags: &[String],
    task: &TaskDefinition,
    stack_frameworks: &BTreeSet<String>,
    context: &CodeReferenceTaskContext,
) -> BTreeMap<String, BTreeSet<String>> {
    let rules = builtin_rules();
    let input = EvaluationInput {
        signal,
        focus_tags,
        task,
        stack_frameworks,
        context,
    };
    let mut groups: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for rule in &rules.backend_rules {
        if evaluate_condition(&rule.when, &input) {
            for sel in &rule.select {
                groups
                    .entry(sel.group_key.clone())
                    .or_default()
                    .insert(sel.group.clone());
            }
        }
    }
    groups
}

/// Evaluates frontend framework reference items for a signal. Replaces
/// `frontend_reference_items_for_signal`. Returns a map from group_key
/// (e.g. "react") to a set of groups.
pub fn evaluate_frontend_items(
    signal: &CodeStackSignal,
    focus_tags: &[String],
    task: &TaskDefinition,
    stack_frameworks: &BTreeSet<String>,
    context: &CodeReferenceTaskContext,
) -> BTreeMap<String, BTreeSet<String>> {
    let rules = builtin_rules();
    let input = EvaluationInput {
        signal,
        focus_tags,
        task,
        stack_frameworks,
        context,
    };
    let mut groups: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for rule in &rules.frontend_rules {
        if evaluate_condition(&rule.when, &input) {
            for sel in &rule.select {
                groups
                    .entry(sel.group_key.clone())
                    .or_default()
                    .insert(sel.group.clone());
            }
        }
    }
    groups
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_task_kind(s: &str) -> Option<TaskKind> {
    serde_json::from_str(&format!("\"{s}\"")).ok()
}

fn parse_implementation_action(s: &str) -> Option<ImplementationAction> {
    serde_json::from_str(&format!("\"{s}\"")).ok()
}

fn context_flag_is_true(context: &CodeReferenceTaskContext, name: &str) -> bool {
    match name {
        "application_architecture" => context.application_architecture,
        "security" => context.security,
        "async_processing" => context.async_processing,
        "integration" => context.integration,
        "resilience" => context.resilience,
        "observability" => context.observability,
        "request_tracing" => context.request_tracing,
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{TaskArtifactRefs, TaskWriteBoundary};

    fn test_signal(language: &str, frameworks: &[&str]) -> CodeStackSignal {
        CodeStackSignal {
            source_track: "test".to_string(),
            source_path: "stack.test".to_string(),
            raw_selection: language.to_string(),
            language: Some(language.to_string()),
            frameworks: frameworks.iter().map(|s| s.to_string()).collect(),
            dialects: vec![],
            roles: vec![],
            confidence: "high".to_string(),
            reason: "test".to_string(),
        }
    }

    fn test_task() -> TaskDefinition {
        TaskDefinition {
            task_id: "test".to_string(),
            group_id: "test".to_string(),
            title: "test".to_string(),
            task_kind: TaskKind::FeatureIncrement,
            implementation_actions: vec![],
            implementation_obligations: vec![],
            objective: "test".to_string(),
            depends_on: vec![],
            scope_refs: vec![],
            acceptance_refs: vec![],
            requirement_detail_refs: vec![],
            write_boundary: TaskWriteBoundary {
                forbidden_paths: vec![],
                artifact_refs: TaskArtifactRefs::default(),
            },
            verification_intents: vec![],
            concept_refs: vec![],
            concept_responsibilities: vec![],
            concept_verification_intents: vec![],
            frontend_experience_requirement: None,
            runtime_delivery_requirement: None,
            engineering_quality_requirement_refs: vec![],
            architecture_quality_requirement_refs: vec![],
            api_contract_requirement_refs: vec![],
            code_quality_requirement_refs: vec![],
        }
    }

    #[test]
    fn builtin_rules_parse() {
        let rules = builtin_rules();
        assert_eq!(rules.schema_version, 1);
        assert_eq!(rules.playbook, "default");
        assert_eq!(rules.mode, "base");
        assert!(!rules.applicability.is_empty());
        assert!(!rules.language_rules.is_empty());
        assert!(!rules.backend_rules.is_empty());
        assert!(!rules.frontend_rules.is_empty());
    }

    #[test]
    fn parse_task_kind_covers_all_variants() {
        assert_eq!(
            parse_task_kind("feature_increment"),
            Some(TaskKind::FeatureIncrement)
        );
        assert_eq!(
            parse_task_kind("configuration_support"),
            Some(TaskKind::ConfigurationSupport)
        );
        assert_eq!(parse_task_kind("nonexistent"), None);
    }

    #[test]
    fn parse_implementation_action_covers_key_variants() {
        assert_eq!(
            parse_implementation_action("implement_async_processing"),
            Some(ImplementationAction::ImplementAsyncProcessing)
        );
        assert_eq!(
            parse_implementation_action("add_or_update_config"),
            Some(ImplementationAction::AddOrUpdateConfig)
        );
        assert_eq!(parse_implementation_action("nonexistent"), None);
    }

    #[test]
    fn empty_condition_is_true() {
        let signal = test_signal("java", &[]);
        let task = test_task();
        let stack_frameworks = BTreeSet::new();
        let context = CodeReferenceTaskContext::default();
        let focus_tags: Vec<String> = vec![];
        let input = EvaluationInput {
            signal: &signal,
            focus_tags: &focus_tags,
            task: &task,
            stack_frameworks: &stack_frameworks,
            context: &context,
        };
        assert!(evaluate_condition(&Condition::default(), &input));
    }

    #[test]
    fn language_condition_matches() {
        let signal = test_signal("java", &[]);
        let task = test_task();
        let stack_frameworks = BTreeSet::new();
        let context = CodeReferenceTaskContext::default();
        let focus_tags: Vec<String> = vec![];
        let input = EvaluationInput {
            signal: &signal,
            focus_tags: &focus_tags,
            task: &task,
            stack_frameworks: &stack_frameworks,
            context: &context,
        };
        let cond = Condition {
            language: Some("java".to_string()),
            ..Default::default()
        };
        assert!(evaluate_condition(&cond, &input));

        let cond = Condition {
            language: Some("python".to_string()),
            ..Default::default()
        };
        assert!(!evaluate_condition(&cond, &input));
    }

    #[test]
    fn any_all_not_combinators_work() {
        let signal = test_signal("java", &["spring_framework"]);
        let task = test_task();
        let stack_frameworks = BTreeSet::new();
        let context = CodeReferenceTaskContext::default();
        let focus_tags = vec!["api".to_string(), "security".to_string()];
        let input = EvaluationInput {
            signal: &signal,
            focus_tags: &focus_tags,
            task: &task,
            stack_frameworks: &stack_frameworks,
            context: &context,
        };

        // any: focus has api OR reactive → true (api present)
        let cond = Condition {
            any: vec![
                Condition {
                    focus: vec!["reactive".to_string()],
                    ..Default::default()
                },
                Condition {
                    focus: vec!["api".to_string()],
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        assert!(evaluate_condition(&cond, &input));

        // all: focus has api AND security → true (both present)
        let cond = Condition {
            all: vec![
                Condition {
                    focus: vec!["api".to_string()],
                    ..Default::default()
                },
                Condition {
                    focus: vec!["security".to_string()],
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        assert!(evaluate_condition(&cond, &input));

        // not: focus does NOT have reactive → true
        let cond = Condition {
            not: Some(Box::new(Condition {
                focus: vec!["reactive".to_string()],
                ..Default::default()
            })),
            ..Default::default()
        };
        assert!(evaluate_condition(&cond, &input));
    }
}
