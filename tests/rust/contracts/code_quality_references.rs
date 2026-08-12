use contracts::{
    code_reference_selection_for_task_with_context, CodeReferenceTaskContext, ImplementationAction,
    TaskArtifactRefs, TaskDefinition, TaskKind, TaskWriteBoundary, TechnicalBaselineApproval,
    TechnicalBaselineApprovalType, TechnicalBaselineContract, TechnicalBaselineScope,
    TechnicalBaselineSource, TechnicalBaselineStatus, VerificationIntent,
};
use serde_json::json;

fn baseline_with(stack_selection: &str) -> TechnicalBaselineContract {
    let stack = json!({
        "tracks": {
            "backend": {
                "status": "selected",
                "selection": stack_selection
            }
        }
    });
    baseline(stack)
}

fn baseline_web_with(stack_selection: &str) -> TechnicalBaselineContract {
    let stack = json!({
        "tracks": {
            "web": {
                "status": "selected",
                "selection": stack_selection
            }
        }
    });
    baseline(stack)
}

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

fn task_with(kind: TaskKind, actions: &[ImplementationAction]) -> TaskDefinition {
    TaskDefinition {
        task_id: "task-1".to_string(),
        group_id: "group-1".to_string(),
        title: "Test".to_string(),
        objective: "Test".to_string(),
        task_kind: kind,
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
fn fastapi_testing_reference_selected() {
    let baseline = baseline_with("python fastapi");
    let task = task_with(
        TaskKind::VerificationIncrement,
        &[ImplementationAction::AddOrUpdateTests],
    );
    let ctx = CodeReferenceTaskContext::default();
    let sel =
        code_reference_selection_for_task_with_context(&baseline, &task, &ctx).expect("selection");
    assert!(sel.reference_groups.contains_key("fastapi"));
    assert!(sel.reference_groups["fastapi"].contains(&"testing".to_string()));
}

#[test]
fn fastapi_security_reference_selected() {
    let baseline = baseline_with("python fastapi");
    let task = task_with(
        TaskKind::FeatureIncrement,
        &[ImplementationAction::ImplementAuthenticationOrAuthorization],
    );
    let ctx = CodeReferenceTaskContext {
        security: true,
        ..Default::default()
    };
    let sel =
        code_reference_selection_for_task_with_context(&baseline, &task, &ctx).expect("selection");
    assert!(sel.reference_groups["fastapi"].contains(&"security".to_string()));
}

#[test]
fn django_models_reference_selected() {
    let baseline = baseline_with("python django");
    let task = task_with(
        TaskKind::DataModelIncrement,
        &[ImplementationAction::CreateOrUpdateEntity],
    );
    let ctx = CodeReferenceTaskContext::default();
    let sel =
        code_reference_selection_for_task_with_context(&baseline, &task, &ctx).expect("selection");
    assert!(sel.reference_groups.contains_key("django"));
    assert!(sel.reference_groups["django"].contains(&"models".to_string()));
}

#[test]
fn spring_boot_web_reference_selected() {
    let baseline = baseline_with("java spring boot");
    let task = task_with(
        TaskKind::InterfaceIncrement,
        &[ImplementationAction::CreateOrUpdateInterface],
    );
    let ctx = CodeReferenceTaskContext::default();
    let sel =
        code_reference_selection_for_task_with_context(&baseline, &task, &ctx).expect("selection");
    assert!(sel.reference_groups.contains_key("springboot"));
    assert!(sel.reference_groups["springboot"].contains(&"web".to_string()));
}

#[test]
fn nestjs_controllers_reference_selected() {
    let baseline = baseline_with("javascript nestjs");
    let task = task_with(
        TaskKind::InterfaceIncrement,
        &[ImplementationAction::CreateOrUpdateInterface],
    );
    let ctx = CodeReferenceTaskContext::default();
    let sel =
        code_reference_selection_for_task_with_context(&baseline, &task, &ctx).expect("selection");
    assert!(sel.reference_groups.contains_key("nestjs"));
    assert!(sel.reference_groups["nestjs"].contains(&"controllers".to_string()));
}

#[test]
fn react_core_reference_selected() {
    let baseline = baseline_web_with("typescript react");
    let task = task_with(
        TaskKind::FrontendExperience,
        &[ImplementationAction::CreateOrUpdateUiFlow],
    );
    let ctx = CodeReferenceTaskContext::default();
    let sel =
        code_reference_selection_for_task_with_context(&baseline, &task, &ctx).expect("selection");
    assert!(sel.reference_groups.contains_key("react"));
    assert!(sel.reference_groups["react"].contains(&"core".to_string()));
}
