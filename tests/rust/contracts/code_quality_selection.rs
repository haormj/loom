use contracts::{
    code_reference_selection_for_task_with_context, code_stack_signals_from_baseline,
    CodeReferenceTaskContext, ImplementationAction, TaskArtifactRefs, TaskDefinition, TaskKind,
    TaskWriteBoundary, TechnicalBaselineApproval, TechnicalBaselineApprovalType,
    TechnicalBaselineContract, TechnicalBaselineScope, TechnicalBaselineSource,
    TechnicalBaselineStatus, VerificationIntent,
};
use serde_json::json;

fn baseline(stack: serde_json::Value) -> TechnicalBaselineContract {
    TechnicalBaselineContract {
        schema_version: "1.0".to_string(),
        technical_baseline_id: "tb-1".to_string(),
        delivery_id: "delivery-1".to_string(),
        phase_id: "phase-1".to_string(),
        status: TechnicalBaselineStatus::Confirmed,
        source: TechnicalBaselineSource::AgentRecommendedForNewProject,
        project_kind: contracts::ProjectKind::NewProject,
        scope: TechnicalBaselineScope::Project,
        stack,
        security_profiles: vec![],
        constraints: vec![],
        evidence: vec![],
        approval: TechnicalBaselineApproval {
            r#type: TechnicalBaselineApprovalType::UserConfirmed,
            confirmed_at: Some("2026-07-06T00:00:00Z".to_string()),
            reason: Some("confirmed".to_string()),
        },
        confidence: contracts::ConfidenceLevel::High,
        reasoning_summary: vec![],
        alternatives: vec![],
        created_at: "2026-07-06T00:00:00Z".to_string(),
        updated_at: "2026-07-06T00:00:00Z".to_string(),
    }
}

fn task_with_actions(actions: &[ImplementationAction]) -> TaskDefinition {
    TaskDefinition {
        task_id: "task-1".to_string(),
        group_id: "group-1".to_string(),
        title: "Implement API endpoint".to_string(),
        objective: "Add REST endpoint".to_string(),
        task_kind: TaskKind::InterfaceIncrement,
        implementation_actions: actions.to_vec(),
        implementation_obligations: vec![],
        depends_on: vec![],
        scope_refs: vec![],
        acceptance_refs: vec![],
        requirement_detail_refs: vec![],
        write_boundary: TaskWriteBoundary {
            forbidden_paths: vec![".loom".to_string()],
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

#[test]
fn signal_from_python_selection() {
    let stack = json!({
        "tracks": {
            "backend": {
                "status": "selected",
                "selection": "python fastapi"
            }
        }
    });
    let signals = code_stack_signals_from_baseline(&stack);
    assert_eq!(signals.len(), 1);
    let s = &signals[0];
    assert_eq!(s.language.as_deref(), Some("python"));
    assert!(s.frameworks.contains(&"fastapi".to_string()));
    assert!(s.roles.contains(&"backend".to_string()));
}

#[test]
fn signal_from_java_spring_selection() {
    let stack = json!({
        "tracks": {
            "backend": {
                "status": "selected",
                "selection": "java spring boot"
            }
        }
    });
    let signals = code_stack_signals_from_baseline(&stack);
    assert_eq!(signals.len(), 1);
    let s = &signals[0];
    assert_eq!(s.language.as_deref(), Some("java"));
    assert!(s.frameworks.iter().any(|f| f.starts_with("spring")));
}

#[test]
fn signal_from_typescript_react_selection() {
    let stack = json!({
        "tracks": {
            "web": {
                "status": "selected",
                "selection": "typescript react next.js"
            }
        }
    });
    let signals = code_stack_signals_from_baseline(&stack);
    assert_eq!(signals.len(), 1);
    let s = &signals[0];
    assert_eq!(s.language.as_deref(), Some("typescript"));
    assert!(s.frameworks.contains(&"react".to_string()));
}

#[test]
fn signal_from_rust_selection() {
    let stack = json!({
        "tracks": {
            "backend": {
                "status": "selected",
                "selection": "rust axum"
            }
        }
    });
    let signals = code_stack_signals_from_baseline(&stack);
    assert_eq!(signals.len(), 1);
    let s = &signals[0];
    assert_eq!(s.language.as_deref(), Some("rust"));
    assert!(s.frameworks.contains(&"axum".to_string()));
}

#[test]
fn signal_from_go_selection() {
    let stack = json!({
        "tracks": {
            "backend": {
                "status": "selected",
                "selection": "go gin"
            }
        }
    });
    let signals = code_stack_signals_from_baseline(&stack);
    let s = &signals[0];
    assert_eq!(s.language.as_deref(), Some("go"));
    assert!(s.frameworks.contains(&"gin".to_string()));
}

#[test]
fn java_excludes_kotlin() {
    let stack = json!({
        "tracks": {
            "backend": {
                "status": "selected",
                "selection": "java spring"
            }
        }
    });
    let kotlin_stack = json!({
        "tracks": {
            "backend": {
                "status": "selected",
                "selection": "kotlin ktor"
            }
        }
    });
    let java_signals = code_stack_signals_from_baseline(&stack);
    let kotlin_signals = code_stack_signals_from_baseline(&kotlin_stack);
    assert_eq!(java_signals[0].language.as_deref(), Some("java"));
    assert_eq!(kotlin_signals[0].language.as_deref(), Some("kotlin"));
}

#[test]
fn reference_selection_fastapi_testing() {
    let stack = json!({
        "tracks": {
            "backend": {
                "status": "selected",
                "selection": "python fastapi"
            }
        }
    });
    let baseline = TechnicalBaselineContract {
        stack,
        ..baseline(json!(null))
    };
    let task = task_with_actions(&[ImplementationAction::AddOrUpdateTests]);
    let ctx = CodeReferenceTaskContext::default();
    let selection = code_reference_selection_for_task_with_context(&baseline, &task, &ctx)
        .expect("selection exists");
    assert!(selection.reference_groups.contains_key("fastapi"));
    assert!(selection.reference_groups["fastapi"].contains(&"testing".to_string()));
}
