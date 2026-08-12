use contracts::{
    CodeReferenceTaskContext, CodeStackSignal, Condition, ConditionContext, ImplementationAction,
    TaskArtifactRefs, TaskDefinition, TaskKind, TaskWriteBoundary, VerificationIntent,
};
use std::collections::BTreeSet;

fn dummy_signal() -> CodeStackSignal {
    CodeStackSignal {
        source_track: "backend".to_string(),
        source_path: "stack.tracks.backend.selection".to_string(),
        raw_selection: "python fastapi".to_string(),
        language: Some("python".to_string()),
        frameworks: vec!["fastapi".to_string()],
        dialects: vec![],
        roles: vec!["backend".to_string()],
        confidence: "high".to_string(),
        reason: "test".to_string(),
    }
}

fn dummy_task() -> TaskDefinition {
    TaskDefinition {
        task_id: "task-1".to_string(),
        group_id: "group-1".to_string(),
        title: "Test task".to_string(),
        objective: "Test".to_string(),
        task_kind: TaskKind::FeatureIncrement,
        implementation_actions: vec![],
        implementation_obligations: vec![],
        depends_on: vec![],
        scope_refs: vec![],
        acceptance_refs: vec![],
        requirement_detail_refs: vec![],
        write_boundary: TaskWriteBoundary {
            forbidden_paths: vec![],
            artifact_refs: TaskArtifactRefs::default(),
        },
        verification_intents: vec![VerificationIntent {
            verification_id: "verify-1".to_string(),
            acceptance_refs: vec![],
            requirement_detail_refs: vec![],
            behavior: "test".to_string(),
            preferred_evidence: vec![],
            acceptable_evidence: vec![],
        }],
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

struct CtxBundle {
    task: TaskDefinition,
    context: CodeReferenceTaskContext,
    stack_frameworks: BTreeSet<String>,
    focus_tags: Vec<String>,
    signal: CodeStackSignal,
}

impl CtxBundle {
    fn new() -> CtxBundle {
        CtxBundle {
            task: dummy_task(),
            context: CodeReferenceTaskContext::default(),
            stack_frameworks: BTreeSet::from(["fastapi".to_string()]),
            focus_tags: vec!["testing".to_string()],
            signal: dummy_signal(),
        }
    }

    fn context(&self) -> ConditionContext<'_> {
        ConditionContext {
            task: &self.task,
            context: &self.context,
            stack_frameworks: &self.stack_frameworks,
            focus_tags: &self.focus_tags,
            signal: &self.signal,
            current_track: "backend",
        }
    }
}

#[test]
fn predicate_single_evaluates_true() {
    let ctx = CtxBundle::new();
    let cond = Condition::Predicate("lang:python".to_string());
    assert!(cond.evaluate(&ctx.context()));
}

#[test]
fn predicate_single_evaluates_false_for_unknown() {
    let ctx = CtxBundle::new();
    let cond = Condition::Predicate("lang:javascript".to_string());
    assert!(!cond.evaluate(&ctx.context()));
}

#[test]
fn all_of_all_true() {
    let ctx = CtxBundle::new();
    let cond = Condition::All {
        all_of: vec![
            Condition::Predicate("lang:python".to_string()),
            Condition::Predicate("fw:fastapi".to_string()),
        ],
    };
    assert!(cond.evaluate(&ctx.context()));
}

#[test]
fn all_of_one_false() {
    let ctx = CtxBundle::new();
    let cond = Condition::All {
        all_of: vec![
            Condition::Predicate("lang:python".to_string()),
            Condition::Predicate("fw:django".to_string()),
        ],
    };
    assert!(!cond.evaluate(&ctx.context()));
}

#[test]
fn any_of_one_true() {
    let ctx = CtxBundle::new();
    let cond = Condition::Any {
        any_of: vec![
            Condition::Predicate("lang:javascript".to_string()),
            Condition::Predicate("lang:python".to_string()),
        ],
    };
    assert!(cond.evaluate(&ctx.context()));
}

#[test]
fn any_of_all_false() {
    let ctx = CtxBundle::new();
    let cond = Condition::Any {
        any_of: vec![
            Condition::Predicate("lang:javascript".to_string()),
            Condition::Predicate("lang:rust".to_string()),
        ],
    };
    assert!(!cond.evaluate(&ctx.context()));
}

#[test]
fn not_negates_true() {
    let ctx = CtxBundle::new();
    let cond = Condition::Not {
        not: Box::new(Condition::Predicate("fw:django".to_string())),
    };
    assert!(cond.evaluate(&ctx.context()));
}

#[test]
fn nested_condition() {
    let ctx = CtxBundle::new();
    let cond = Condition::All {
        all_of: vec![
            Condition::Predicate("lang:python".to_string()),
            Condition::Any {
                any_of: vec![
                    Condition::Predicate("fw:django".to_string()),
                    Condition::Predicate("fw:fastapi".to_string()),
                ],
            },
            Condition::Not {
                not: Box::new(Condition::Predicate("fw:nextjs".to_string())),
            },
        ],
    };
    assert!(cond.evaluate(&ctx.context()));
}

#[test]
fn unknown_predicate_returns_false() {
    let ctx = CtxBundle::new();
    let cond = Condition::Predicate("unknown_namespace:foo".to_string());
    assert!(!cond.evaluate(&ctx.context()));
}

#[test]
fn task_action_predicate_matches_owned_action() {
    let mut bundle = CtxBundle::new();
    bundle.task.implementation_actions = vec![ImplementationAction::CreateOrUpdateEntity];
    let cond = Condition::Predicate("task_action:CreateOrUpdateEntity".to_string());
    assert!(cond.evaluate(&bundle.context()));
}

#[test]
fn task_action_predicate_false_for_unowned_action() {
    let mut bundle = CtxBundle::new();
    bundle.task.implementation_actions = vec![ImplementationAction::CreateOrUpdateEntity];
    let cond = Condition::Predicate("task_action:AddOrUpdateTests".to_string());
    assert!(!cond.evaluate(&bundle.context()));
}

#[test]
fn task_kind_predicate_matches_task_kind() {
    let ctx = CtxBundle::new();
    let cond = Condition::Predicate("task_kind:FeatureIncrement".to_string());
    assert!(cond.evaluate(&ctx.context()));
}

#[test]
fn task_kind_predicate_false_for_mismatched_kind() {
    let ctx = CtxBundle::new();
    let cond = Condition::Predicate("task_kind:VerificationIncrement".to_string());
    assert!(!cond.evaluate(&ctx.context()));
}

#[test]
fn stack_fw_predicate_matches_stack_framework() {
    let ctx = CtxBundle::new();
    let cond = Condition::Predicate("stack_fw:fastapi".to_string());
    assert!(cond.evaluate(&ctx.context()));
}

#[test]
fn focus_predicate_matches_focus_tag() {
    let ctx = CtxBundle::new();
    let cond = Condition::Predicate("focus:testing".to_string());
    assert!(cond.evaluate(&ctx.context()));
}

#[test]
fn context_predicate_reads_context_flag() {
    let mut bundle = CtxBundle::new();
    bundle.context.security = true;
    let cond = Condition::Predicate("context:security".to_string());
    assert!(cond.evaluate(&bundle.context()));
}

#[test]
fn task_owns_predicate_routes_to_code_quality_function() {
    let mut bundle = CtxBundle::new();
    bundle.task.task_kind = TaskKind::VerificationIncrement;
    let cond = Condition::Predicate("task_owns:test_implementation".to_string());
    assert!(cond.evaluate(&bundle.context()));
}
