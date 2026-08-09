use std::{collections::BTreeSet, path::Path};

use contracts::{
    ApiContractRequirement, ArchitectureArtifactContract, ArchitectureQualityRequirement,
    BrowserVerificationProfile, EngineeringQualityRequirement, ImplementationAction,
    TaskAttemptState, TaskDefinition, TaskKind, TaskPlan, TaskPlanRun, TaskPlanRunNextAction,
    TaskPlanRunStatus, TaskResult, TaskRunStatus, VerificationEvidence,
};
use delivery_core::{
    apply_delivery_index, read_selectors_value_from_paths, DeliveryLifecycleStatus,
    LoomMcpActionResult, LoomMcpAutoRunnableResult, LoomMcpFailure, LoomMcpFailureResult,
    LoomMcpNextAction, RouteAction, RouteActionKind, RunLoomToolNext, TransitionStore,
};
use serde_json::{json, Value};
use state::{
    lifecycle_store::FileTransitionStore,
    paths::{from_project_relative, to_project_relative, DeliveryPhaseLocator},
};

use crate::{
    api_contract::{exposure_projection, interfaces_for_refs, load_project_api_contract},
    paths::{
        task_execution_request_file, task_execution_result_candidate_file, task_plan_file,
        task_plan_latest_file, task_plan_run_file, task_plan_run_latest_file, task_result_file,
    },
    task_plan::{
        execute_task_next_from_request, update_run_summary, UI_OWNERSHIP_DIMENSION_VALUES,
    },
    templates::{
        code_quality_execution_context, code_quality_requirements_for_task,
        frontend_quality_self_check_applies, frontend_self_check_applies,
        runtime_delivery_evidence_applies, task_result_contract, task_result_contract_read_fields,
    },
};

pub fn continue_execution(
    project_root: &str,
    delivery_id: &str,
    phase_id: &str,
) -> LoomMcpActionResult {
    match continue_execution_inner(project_root, delivery_id, phase_id) {
        Ok(result) => result,
        Err(error) => LoomMcpActionResult::Failed(LoomMcpFailureResult {
            project_root: project_root.to_string(),
            error: LoomMcpFailure {
                code: "TASK_EXECUTION_MATERIALIZE_FAILED".to_string(),
                message: error.to_string(),
                target_batch: Some(8),
                domain: Some("execution".to_string()),
                route_action: Some("continue_execution".to_string()),
                recovery_tool: Some("loom.continue".to_string()),
            },
        }),
    }
}

fn continue_execution_inner(
    project_root: &str,
    delivery_id: &str,
    phase_id: &str,
) -> Result<LoomMcpActionResult, state::store::StateError> {
    let root = Path::new(project_root);
    let locator = DeliveryPhaseLocator {
        delivery_id: delivery_id.to_string(),
        phase_id: phase_id.to_string(),
    };
    let (task_plan, mut run) = load_current_plan_and_run(root, &locator)?;
    let Some(task_id) = running_or_ready_task_id(&run) else {
        update_route_for_review(project_root, delivery_id, phase_id)?;
        return Ok(crate::review::materialize_review_request(
            project_root,
            delivery_id,
            phase_id,
        ));
    };
    let task = task_plan
        .tasks
        .iter()
        .find(|task| task.task_id == task_id)
        .cloned()
        .ok_or_else(|| {
            state::store::StateError::StateCorrupted(format!(
                "TaskPlanRun references missing task {task_id}"
            ))
        })?;
    if matches!(task.task_kind, TaskKind::BrowserQualityClosure)
        && crate::browser::browser_runtime_preparation_state(root)
            == crate::browser::BrowserRuntimePreparationState::Unavailable
    {
        return close_unavailable_browser_environment(
            project_root,
            &locator,
            &task_plan,
            &mut run,
            &task,
        );
    }
    if matches!(task.task_kind, TaskKind::BrowserQualityClosure)
        && crate::browser::browser_runtime_preparation_state(root)
            == crate::browser::BrowserRuntimePreparationState::NeedsPreparation
    {
        return materialize_browser_runtime_prepare_action(
            project_root,
            &locator,
            &task_plan,
            &task,
        );
    }
    if let Some(existing) =
        existing_execution_next_if_current(project_root, delivery_id, phase_id, &task)?
    {
        return Ok(existing);
    }

    let now = state::store::now_string();
    if let Some(state) = run
        .task_states
        .iter_mut()
        .find(|state| state.task_id == task.task_id)
    {
        if state.status == TaskRunStatus::Pending {
            state.status = TaskRunStatus::Running;
            state.started_at = Some(now.clone());
        }
    }
    if let Some(group) = run
        .group_states
        .iter_mut()
        .find(|group| group.group_id == task.group_id)
    {
        if group.status == TaskRunStatus::Pending {
            group.status = TaskRunStatus::Running;
            group.started_at = Some(now.clone());
        }
    }
    run.status = TaskPlanRunStatus::Running;
    run.scheduler.started_at.get_or_insert(now.clone());
    run.next_action = Some(TaskPlanRunNextAction {
        r#type: "continue_execution".to_string(),
        reason: "TASK_READY".to_string(),
        source_task_id: Some(task.task_id.clone()),
        target_node: "task_execution".to_string(),
    });
    run.updated_at = now;
    update_run_summary(&mut run);
    save_run(root, &locator, &run)?;

    let request_id = format!(
        "exec_{}_{}",
        safe_id(&task.task_id),
        state::store::now_millis()
    );
    let result_file = to_project_relative(
        root,
        &task_execution_result_candidate_file(root, &request_id),
    )?;
    let request_file = to_project_relative(
        root,
        &task_execution_request_file(root, &locator, &request_id),
    )?;
    let request_root = build_execution_request(
        project_root,
        &locator,
        &request_id,
        &result_file,
        &task_plan,
        &run,
        &task,
    )?;
    let stored = state::write_native_request(
        project_root,
        state::NativeRequestInput {
            request_id: request_id.clone(),
            request_kind: "task_execution_request".to_string(),
            request_file: Some(request_file),
            delivery_id: Some(delivery_id.to_string()),
            phase_id: Some(phase_id.to_string()),
            root: request_root,
        },
    )?;
    if let Some(parent) = from_project_relative(root, &result_file)?.parent() {
        state::store::ensure_dir(parent)?;
    }
    update_route_for_execution(
        project_root,
        delivery_id,
        phase_id,
        &stored.request_ref,
        &result_file,
        &task,
        &run,
    )?;
    execute_task_next_from_request(project_root, &stored.request_ref, &task, result_file)
}

fn materialize_browser_runtime_prepare_action(
    project_root: &str,
    locator: &DeliveryPhaseLocator,
    task_plan: &TaskPlan,
    task: &TaskDefinition,
) -> Result<LoomMcpActionResult, state::store::StateError> {
    let root = Path::new(project_root);
    let profile = task_plan
        .browser_verification_profiles
        .iter()
        .find(|profile| profile.task_id == task.task_id)
        .ok_or_else(|| {
            state::store::StateError::StateCorrupted(
                "browser quality closure is missing its verification profile".to_string(),
            )
        })?;
    let request_id = format!("browser_runtime_prepare_{}", state::store::now_millis());
    let request_file = to_project_relative(
        root,
        &task_execution_request_file(root, locator, &request_id),
    )?;
    let request_root = json!({
        "schemaVersion": "1.0",
        "requestType": "browser_runtime_prepare",
        "source": {
            "taskPlanId": task_plan.task_plan_id,
            "taskId": task.task_id,
            "profileId": profile.profile_id
        },
        "browserRuntimePreparation": {
            "projectTargets": crate::browser::browser_runtime_targets(root),
            "requestedBrowsers": ["chromium"],
            "policy": "解析确切的项目版本，尝试宿主启动，然后回退到托管容器。"
        },
        "requestReadPlan": {"groups": [{
            "groupId": "browser_runtime_prepare_context",
            "required": true,
            "purpose": "读取确切的项目目标和运行时回退策略。",
            "whenToRead": "在调用 loom.browserRuntimePrepare 之前读取。",
            "selectors": read_selectors_value_from_paths([
                "source.taskPlanId",
                "source.taskId",
                "source.profileId",
                "browserRuntimePreparation.projectTargets",
                "browserRuntimePreparation.requestedBrowsers",
                "browserRuntimePreparation.policy"
            ])
        }]}
    });
    let stored = state::write_native_request(
        project_root,
        state::NativeRequestInput {
            request_id,
            request_kind: "browser_runtime_prepare_request".to_string(),
            request_file: Some(request_file),
            delivery_id: Some(locator.delivery_id.clone()),
            phase_id: Some(locator.phase_id.clone()),
            root: request_root,
        },
    )?;
    update_route_for_browser_runtime_prepare(project_root, locator, &stored.request_ref, task)?;
    Ok(LoomMcpActionResult::AutoRunnable(
        LoomMcpAutoRunnableResult::new(
            project_root.to_string(),
            LoomMcpNextAction::RunLoomTool(RunLoomToolNext {
                tool_name: "loom.browserRuntimePrepare".to_string(),
                request_ref: stored.request_ref,
                read_groups: stored.read_groups,
                retry_tool: "loom.continue".to_string(),
            }),
        ),
    ))
}

fn close_unavailable_browser_environment(
    project_root: &str,
    locator: &DeliveryPhaseLocator,
    task_plan: &TaskPlan,
    run: &mut TaskPlanRun,
    task: &TaskDefinition,
) -> Result<LoomMcpActionResult, state::store::StateError> {
    let root = Path::new(project_root);
    let profile = task_plan
        .browser_verification_profiles
        .iter()
        .find(|profile| profile.task_id == task.task_id)
        .ok_or_else(|| {
            state::store::StateError::StateCorrupted(
                "browser quality closure is missing its MCP verification profile".to_string(),
            )
        })?;
    let runtime_state =
        state::store::read_json_value(&root.join(".loom/runtime/browser-automation/latest.json"))?;
    let diagnostic = runtime_state
        .pointer("/runtime/runtimes")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .flat_map(|runtime| {
            runtime
                .get("doctorChecks")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
        })
        .filter(|check| check.get("status").and_then(Value::as_str) == Some("failed"))
        .filter_map(|check| check.get("summary").and_then(Value::as_str))
        .take(4)
        .collect::<Vec<_>>()
        .join(" ");
    let blocked_reason = if diagnostic.is_empty() {
        "浏览器启动在宿主环境和 Loom 托管容器上均不可用。".to_string()
    } else {
        diagnostic
    };
    let verification_results = task
        .verification_intents
        .iter()
        .map(|intent| {
            json!({
                "verificationId": intent.verification_id,
                "status": "inconclusive",
                "evidenceType": "browser_automation",
                "summary": "浏览器证据无法运行，因为两个支持的执行环境均不可用。",
                "browserChecks": profile.checks.iter()
                    .filter(|check| check.verification_id == intent.verification_id)
                    .map(|check| json!({
                        "checkId": check.check_id,
                        "status": "blocked",
                        "command": "",
                        "attempts": 0,
                        "artifactRefs": [],
                        "observedOutcome": "",
                        "blockedReason": blocked_reason.clone()
                    }))
                    .collect::<Vec<_>>()
            })
        })
        .collect::<Vec<_>>();
    let now = state::store::now_string();
    let result_id = format!("system-browser-environment-{}", state::store::now_millis());
    let result: TaskResult = serde_json::from_value(json!({
        "schemaVersion": "1.0",
        "taskResultId": result_id,
        "taskId": task.task_id,
        "taskPlanId": task_plan.task_plan_id,
        "status": "completed_with_notes",
        "changedFiles": [],
        "noChangeReason": {
            "code": "ENVIRONMENT_CHECK_ONLY",
            "summary": "MCP 在未更改项目文件的情况下关闭了浏览器环境检查。"
        },
        "verificationResults": verification_results,
        "executionContinuity": {
            "taskResultSubmittedAfterVerification": true,
            "agentOwnedLongRunningWork": "none",
            "notes": ["浏览器环境故障已由 MCP 分类，未进入执行修复流程。"]
        },
        "notes": [blocked_reason],
        "createdAt": now.clone(),
        "updatedAt": now.clone()
    }))
    .map_err(state::store::StateError::Json)?;
    let result_path = task_result_file(
        root,
        locator,
        &run.run_id,
        &task.task_id,
        &result.task_result_id,
    );
    state::store::write_json_atomic(&result_path, &result)?;

    if let Some(state) = run
        .task_states
        .iter_mut()
        .find(|state| state.task_id == task.task_id)
    {
        state.status = TaskRunStatus::CompletedWithNotes;
        state.result_id = Some(result.task_result_id.clone());
        state.finished_at = Some(now.clone());
        state.attempts.push(TaskAttemptState {
            attempt: state.attempts.len() as u32 + 1,
            result_id: result.task_result_id.clone(),
            status: TaskRunStatus::CompletedWithNotes,
        });
    }
    if let Some(group) = run
        .group_states
        .iter_mut()
        .find(|group| group.group_id == task.group_id)
    {
        group.status = TaskRunStatus::CompletedWithNotes;
        group.finished_at = Some(now.clone());
    }
    update_run_summary(run);
    run.status = if run.summary.pending == 0 && run.summary.running == 0 {
        TaskPlanRunStatus::CompletedWithNotes
    } else {
        TaskPlanRunStatus::Running
    };
    run.next_action = Some(TaskPlanRunNextAction {
        r#type: "review".to_string(),
        reason: "BROWSER_ENVIRONMENT_REQUIRES_REVIEW".to_string(),
        source_task_id: Some(task.task_id.clone()),
        target_node: "review".to_string(),
    });
    run.updated_at = now;
    save_run(root, locator, run)?;
    update_route_for_review(project_root, &locator.delivery_id, &locator.phase_id)?;
    Ok(crate::review::materialize_review_request(
        project_root,
        &locator.delivery_id,
        &locator.phase_id,
    ))
}

fn existing_execution_next_if_current(
    project_root: &str,
    delivery_id: &str,
    phase_id: &str,
    task: &TaskDefinition,
) -> Result<Option<LoomMcpActionResult>, state::store::StateError> {
    let store = FileTransitionStore;
    let delivery = store
        .load_delivery_index(project_root, delivery_id)
        .map_err(to_state_error)?;
    let Some(phase) = delivery
        .phases
        .iter()
        .find(|phase| phase.phase_id == phase_id)
    else {
        return Ok(None);
    };
    let Some(action) = phase.next_action.as_ref() else {
        return Ok(None);
    };
    if action.kind != RouteActionKind::ContinueExecution
        || action.source != "task_execution_request"
    {
        return Ok(None);
    }
    if action
        .details
        .as_ref()
        .and_then(|details| details.get("taskId"))
        .and_then(Value::as_str)
        != Some(task.task_id.as_str())
    {
        return Ok(None);
    }
    let Some(request_ref) = phase
        .latest_refs
        .get("taskExecutionRequestRef")
        .or(action.request_ref.as_ref())
    else {
        return Ok(None);
    };
    let Some(result_file) = action
        .details
        .as_ref()
        .and_then(|details| details.get("resultFile"))
        .and_then(Value::as_str)
    else {
        return Ok(None);
    };
    execute_task_next_from_request(project_root, request_ref, task, result_file.to_string())
        .map(Some)
}

fn build_execution_request(
    project_root: &str,
    locator: &DeliveryPhaseLocator,
    request_id: &str,
    result_file: &str,
    task_plan: &TaskPlan,
    run: &TaskPlanRun,
    task: &TaskDefinition,
) -> Result<Value, state::store::StateError> {
    let root = Path::new(project_root);
    let baseline_ref = format!(
        ".loom/deliveries/{}/contracts/technical-baseline.json",
        locator.delivery_id
    );
    let planning_ref = format!(
        ".loom/deliveries/{}/contracts/planning/{}/pgc.json",
        locator.delivery_id, locator.phase_id
    );
    let architecture_ref = format!(
        ".loom/deliveries/{}/contracts/architecture/{}/aac.json",
        locator.delivery_id, locator.phase_id
    );
    let baseline: contracts::TechnicalBaselineContract = read_project_json(root, &baseline_ref)?;
    let pgc: contracts::PlanningGenerationContract = read_project_json(root, &planning_ref)?;
    let aac: ArchitectureArtifactContract = read_project_json(root, &architecture_ref)?;
    let project_api_contract = load_project_api_contract(root, &aac)?;
    let task_plan_ref = to_project_relative(
        root,
        &task_plan_file(root, locator, &task_plan.task_plan_id),
    )?;
    let run_ref = to_project_relative(root, &task_plan_run_file(root, locator, &run.run_id))?;
    let request_task = task_with_execution_guidance(
        task.clone(),
        &aac,
        &pgc.planning_inputs.user_facing_language,
    );
    let engineering_quality_requirements =
        task_scoped_engineering_quality_requirements(task_plan, &request_task);
    let architecture_quality_requirements =
        task_scoped_architecture_quality_requirements(task_plan, &request_task);
    let api_contract_requirements = task_scoped_api_contract_requirements(task_plan, &request_task);
    let code_quality_requirements = code_quality_requirements_for_task(task_plan, &request_task);
    let browser_verification_profile =
        browser_verification_profile_for_task(task_plan, &request_task);
    let browser_verification_context = browser_verification_profile
        .map(|profile| browser_verification_context(root, task_plan, profile));
    let architecture_projection =
        task_scoped_architecture_projection(&aac, project_api_contract.as_ref(), &request_task);
    let result_contract = task_result_contract(
        &request_task,
        &code_quality_requirements,
        browser_verification_profile,
    );
    let dependency_results = dependency_results(run, task);
    let read_groups = task_execution_read_groups(
        &request_task,
        !dependency_results.is_empty(),
        &architecture_projection,
        browser_verification_context.is_some(),
    );
    let user_facing_language = pgc.planning_inputs.user_facing_language.clone();
    let mut execution_rules =
        task_execution_rules(result_file, &request_task, user_facing_language.clone());
    if browser_verification_context.is_some() {
        execution_rules["browserVerificationRules"] = browser_verification_rules();
    }
    let result_rules = task_result_rules(&request_task, browser_verification_profile.is_some());
    let mut source_context = json!({
        "technicalBaseline": {
            "projectKind": baseline.project_kind,
            "stack": baseline.stack,
            "securityProfiles": baseline.security_profiles
        },
        "architectureArtifactProjection": architecture_projection,
        "acceptanceSnapshot": pgc.phase_scope.acceptance_candidates.iter()
            .filter(|acceptance| request_task.acceptance_refs.iter().any(|id| id == &acceptance.id))
            .collect::<Vec<_>>(),
        "requirementDetailSnapshot": pgc.requirement_details.items.iter()
            .filter(|detail| request_task.requirement_detail_refs.iter().any(|id| id == &detail.detail_id))
            .collect::<Vec<_>>(),
        "userFacingLanguage": user_facing_language
    });
    if !dependency_results.is_empty() {
        source_context["dependencyResults"] = json!(dependency_results);
    }
    if !engineering_quality_requirements.is_empty() {
        source_context["engineeringQualityRequirements"] = json!(engineering_quality_requirements);
    }
    if !architecture_quality_requirements.is_empty() {
        source_context["architectureQualityRequirements"] =
            json!(architecture_quality_requirements);
    }
    if !api_contract_requirements.is_empty() {
        source_context["apiContractRequirements"] = json!(api_contract_requirements);
    }
    if !code_quality_requirements.is_empty() {
        source_context["codeQualityExecutionContext"] =
            code_quality_execution_context(&code_quality_requirements);
    }
    if let Some(browser_verification_context) = browser_verification_context {
        source_context["browserVerificationContext"] = browser_verification_context;
    }
    let request = json!({
        "schemaVersion": "1.0",
        "requestType": "execute_task",
        "requestId": request_id,
        "artifactKind": delivery_core::ArtifactKind::TaskResult,
        "source": {
            "taskPlanId": task_plan.task_plan_id,
            "taskId": task.task_id,
            "groupId": task.group_id,
            "technicalBaselineId": baseline.technical_baseline_id,
            "architectureArtifactContractId": aac.architecture_artifact_contract_id,
            "taskPlanRunId": run.run_id
        },
        "sourceRefs": {
            "technicalBaselineRef": baseline_ref,
            "planningGenerationContractRef": planning_ref,
            "architectureArtifactContractRef": architecture_ref,
            "taskPlanRef": task_plan_ref,
            "taskPlanRunRef": run_ref,
            "phaseConceptGroundingRef": pgc.context_refs.phase_concept_grounding_ref
        },
        "task": &request_task,
        "sourceContext": source_context,
        "executionRules": execution_rules,
        "enumRefs": {
            "taskResultStatus": ["completed", "completed_with_notes", "blocked", "failed"],
            "verificationStatus": ["passed", "not_run", "failed", "inconclusive"],
            "verificationEvidence": ["automated_test", "browser_automation", "manual_command_output", "runtime_api_check", "static_check", "agent_review_explanation"],
            "selfRepairStopReason": ["not_attempted", "verification_passed", "blocked_condition_detected", "same_failure_repeated_without_progress", "hard_attempt_limit_reached", "repair_requires_contract_change", "repair_requires_scope_expansion"]
        },
        "outputContract": {
            "artifactKind": delivery_core::ArtifactKind::TaskResult,
            "writeMode": "single_json",
            "submitTool": "loom.recordTaskResultFile",
            "resultFile": result_file,
            "writeTargets": [{
                "targetId": "result",
                "path": result_file,
                "required": true,
                "description": "为此计划任务写入 TaskResult JSON。"
            }],
            "requiredTopLevelFields": result_contract["requiredTopLevelFields"].clone(),
            "blockedReasonOptions": [
                {"code": "DESIGN_INSUFFICIENT", "nextNode": "architecture_artifact_repair"},
                {"code": "TASKPLAN_INVALID", "nextNode": "taskplan_repair"},
                {"code": "DEPENDENCY_NOT_READY", "nextNode": "wait_dependency"}
            ],
            "schemaShape": result_contract["schemaShape"].clone(),
            "resultTemplate": result_contract["resultTemplate"].clone(),
            "resultRules": result_rules
        },
        "requestReadPlan": { "groups": read_groups }
    });
    validate_task_execution_request_coverage(&request)?;
    Ok(request)
}

fn validate_task_execution_request_coverage(
    request: &Value,
) -> Result<(), state::store::StateError> {
    let task = request.get("task").ok_or_else(|| {
        state::store::StateError::InvalidArgument(
            "task execution request is missing task context".to_string(),
        )
    })?;
    let groups = request
        .pointer("/requestReadPlan/groups")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            state::store::StateError::InvalidArgument(
                "task execution request is missing requestReadPlan.groups".to_string(),
            )
        })?;
    let covered = groups
        .iter()
        .filter_map(|group| group.get("selectors"))
        .filter_map(|selectors| {
            serde_json::from_value::<Vec<delivery_core::ReadSelector>>(selectors.clone()).ok()
        })
        .flat_map(|selectors| delivery_core::expand_read_selectors(&selectors))
        .collect::<BTreeSet<_>>();

    let mut required = vec![
        "task.objective".to_string(),
        "task.writeBoundary".to_string(),
    ];
    for field in [
        "implementationActions",
        "implementationObligations",
        "verificationIntents",
    ] {
        if task
            .get(field)
            .and_then(Value::as_array)
            .is_some_and(|items| !items.is_empty())
        {
            required.push(format!("task.{field}"));
        }
    }
    if let Some(projection) = request
        .pointer("/sourceContext/architectureArtifactProjection")
        .and_then(Value::as_object)
    {
        for (field, value) in projection {
            if field != "compaction" && json_value_has_content(value) {
                required.push(format!(
                    "sourceContext.architectureArtifactProjection.{field}"
                ));
            }
        }
    }
    for path in required {
        if !covered.iter().any(|covered_path| {
            covered_path == &path || covered_path.starts_with(&format!("{path}."))
        }) {
            return Err(state::store::StateError::InvalidArgument(format!(
                "task execution request read plan does not cover authoritative field {path}"
            )));
        }
    }
    Ok(())
}

fn json_value_has_content(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Array(items) => !items.is_empty(),
        Value::Object(fields) => fields.values().any(json_value_has_content),
        Value::String(value) => !value.trim().is_empty(),
        Value::Bool(_) | Value::Number(_) => true,
    }
}

pub(crate) fn task_execution_rules(
    result_file: &str,
    task: &TaskDefinition,
    user_facing_language: Option<contracts::UserFacingLanguageConstraint>,
) -> Value {
    let mut rules = json!({
        "sourceEditPreparationContract": source_edit_preparation_contract(result_file),
        "completionBarrier": {
            "resultFile": result_file,
            "submitTool": "loom.recordTaskResultFile",
            "rule": "在 TaskResult 存在于 outputContract.resultFile 且 loom.recordTaskResultFile 成功之前，任务未完成。"
        },
        "writeTargetRule": "当文件提交工具接受 writtenTargetIds 时，传入当前 outputContract.writeTargets[].targetId（对于 TaskResult 为 result），切勿传入目标路径或 resultFile 字符串。",
        "finalResponseGuard": {
            "mustNotReportProgressBeforeSubmit": true,
            "rule": "在提交 TaskResult 之前，不得以仅含进度摘要的方式停止。"
        },
        "completionContinuityRequirement": {
            "rule": "验证方法由代理选择，但必须在提交 TaskResult 之前返回控制权。",
            "forbiddenOutcome": "不要让此任务等待长时间运行的命令、浏览器会话、交互式工具、服务器、监视器、工作进程、进度摘要或交接说明。",
            "requiredCloseout": "除非达到声明的停止条件，否则在当前任务轮次中写入 TaskResult 并运行 loom.recordTaskResultFile。",
            "taskResultField": "executionContinuity",
            "statusRule": "如果代理拥有的长时间运行工作已启动且其释放状态未知，使用 completed_with_notes 并附带说明，除非存在独立的失败或阻塞条件。"
        },
        "implementationClosureContract": {
            "source": "task.implementationObligations",
            "ownership": "MCP",
            "agentRule": "在报告 completed 之前，实现此任务中的每个必需义务。按照提供的规范顺序为每个义务报告一个 implementationObligationResults 条目；仅填写 status、evidenceRefs 和 summary。不要编写 obligationId 或 verificationIds。",
            "completionRule": "只有当每个必需义务都有具体证据满足且没有义务处于 partial、blocked 或 not_verified 状态时，任务才算完成。",
            "repairRule": "当义务未完成时，提交执行修复，保持 obligationId 不变并补充缺失的实现或证据；不要更改任务合同。"
        },
        "verificationCommandSchedulingRules": verification_command_scheduling_rules(),
        "userFacingLanguage": {
            "constraint": user_facing_language,
            "rule": "在生成的 UI、验证、反馈、测试标签和 TaskResult 证据中，适当时保留已确认的用户面向语言。"
        },
        "boundaryRules": [
            "仅执行当前任务。",
            "不要修改 Brainstorm、TechnicalBaseline、PGC、AAC、TaskPlan 或其他受保护的 Loom 制品。",
            "不要实现延迟范围。",
            "适当时在用户可见的 UI、反馈、测试名称和 TaskResult 证据中使用已确认的业务语言。",
            "仅将 TaskResult JSON 写入 outputContract.resultFile。"
        ],
        "taskResponsibilityBoundary": task_responsibility_boundary(task)
    });
    let Some(object) = rules.as_object_mut() else {
        return rules;
    };
    if task_has_frontend_execution(task) {
        object.insert(
            "frontendImplementationOrganizationRules".to_string(),
            frontend_implementation_organization_rules(),
        );
        object.insert(
            "interactiveVerificationProbePolicy".to_string(),
            interactive_verification_probe_policy(),
        );
    }
    if task_needs_controlled_runtime_probe_rules(task) {
        object.insert(
            "controlledRuntimeProbeRules".to_string(),
            controlled_runtime_probe_rules(),
        );
    }
    if runtime_delivery_evidence_applies(task) {
        object.insert(
            "runtimeDeliveryExecutionRules".to_string(),
            runtime_delivery_execution_rules(),
        );
    }
    if !task.engineering_quality_requirement_refs.is_empty() {
        object.insert(
            "engineeringQualityExecutionRules".to_string(),
            engineering_quality_execution_rules(task),
        );
    }
    if !task.architecture_quality_requirement_refs.is_empty() {
        object.insert(
            "architectureQualityExecutionRules".to_string(),
            architecture_quality_execution_rules(task),
        );
    }
    if !task.api_contract_requirement_refs.is_empty() {
        object.insert(
            "apiContractExecutionRules".to_string(),
            api_contract_execution_rules(task),
        );
    }
    if !task.code_quality_requirement_refs.is_empty() {
        object.insert(
            "codeQualityExecutionRules".to_string(),
            code_quality_execution_rules(task),
        );
    }
    rules
}

fn task_responsibility_boundary(task: &TaskDefinition) -> Value {
    let owns_persistence = task_directly_owns_persistence(task);
    let owns_interface = !task.write_boundary.artifact_refs.interfaces.is_empty()
        || task.implementation_actions.iter().any(|action| {
            matches!(
                action,
                contracts::ImplementationAction::CreateOrUpdateInterface
                    | contracts::ImplementationAction::CreateEntityCrud
            )
        });
    let owns_frontend = task.frontend_experience_requirement.is_some()
        || matches!(
            task.task_kind,
            contracts::TaskKind::FrontendExperience | contracts::TaskKind::UiFlowIncrement
        );
    let owns_runtime = task
        .runtime_delivery_requirement
        .as_ref()
        .is_some_and(|requirement| requirement.applies_to_this_task);
    let mut rules = vec![
        "仅实现 ownedResponsibilities 和所提供 implementationObligations 中列出的职责。"
            .to_string(),
        "被消费的接口是集成输入，不是重写提供方持久化、领域或 API 实现的许可。".to_string(),
        "当另一个任务拥有某职责时，调用其已接受的边界并记录依赖关系；不要在此任务中重复其实现。"
            .to_string(),
    ];
    if !owns_persistence {
        rules.push(
            "此任务不拥有持久化映射、模式、迁移、仓库或事务实现。不要添加或修改这些制品。"
                .to_string(),
        );
    }
    if !owns_interface {
        rules.push(
            "此任务不拥有服务端 API 接口。不要添加控制器、路由、请求处理器或 API 合同变更。"
                .to_string(),
        );
    }
    if !owns_frontend {
        rules.push(
            "此任务不拥有前端界面。不要添加或修改客户端 UI、浏览器流程或仅前端的状态。".to_string(),
        );
    }
    if !owns_runtime {
        rules
            .push("此任务不拥有运行时交付资产或包启动命令。不要重写部署或运行时配置。".to_string());
    }
    json!({
        "ownedResponsibilities": task.implementation_actions,
        "ownedArtifactRefs": task.write_boundary.artifact_refs,
        "ownedObligationIds": task.implementation_obligations.iter().map(|item| item.obligation_id.clone()).collect::<Vec<_>>(),
        "rules": rules
    })
}

fn task_directly_owns_persistence(task: &TaskDefinition) -> bool {
    !task.write_boundary.artifact_refs.entities.is_empty()
        || matches!(task.task_kind, contracts::TaskKind::DataModelIncrement)
        || task.implementation_actions.iter().any(|action| {
            matches!(
                action,
                contracts::ImplementationAction::CreateOrUpdateEntity
                    | contracts::ImplementationAction::CreateOrUpdatePersistence
                    | contracts::ImplementationAction::CreateEntityCrud
                    | contracts::ImplementationAction::CreateEntityRepository
                    | contracts::ImplementationAction::CreateEntityMigration
                    | contracts::ImplementationAction::CreateOrUpdatePersistenceQuery
                    | contracts::ImplementationAction::ImplementEntityLifecycle
                    | contracts::ImplementationAction::ImplementPersistenceTransaction
                    | contracts::ImplementationAction::OptimizePersistenceQuery
                    | contracts::ImplementationAction::ImplementAnalyticalQuery
                    | contracts::ImplementationAction::AddOrUpdatePersistenceTests
            )
        })
}

fn source_edit_preparation_contract(result_file: &str) -> Value {
    json!({
        "schemaVersion": "1.0",
        "contractKind": "source_edit_preparation",
        "resultFile": result_file,
        "requiredWritePlanFields": {
            "targetPath": "用于创建、替换、编辑或写入作为结果制品的具体项目相对或绝对路径。",
            "writeKind": ["create", "replace", "edit", "multi_edit", "artifact_result"],
            "contentBasis": [
                "task.objective",
                "task.acceptanceRefs and sourceContext.acceptanceSnapshot",
                "task.requirementDetailRefs and sourceContext.requirementDetailSnapshot",
                "task.frontendExperienceRequirement when present",
                "task.runtimeDeliveryRequirement when present",
                "current source file contents for source edits"
            ],
            "writePayloadReady": "true 仅当在调用写入方法之前已形成完整文件内容或完整编辑集时"
        },
        "sequence": [
            "读取所需的 requestReadPlan 组和将要更改的当前源文件。",
            "形成包含 targetPath、writeKind、contentBasis 和 writePayloadReady=true 的内部写入计划。",
            "仅在路径和载荷完整后调用文件写入/编辑。",
            "如果写入工具拒绝缺失或无效的路径/内容/编辑参数，在重试前重建完整参数。",
            "如果 targetPath 或载荷无法在此任务边界内确定，写入失败或阻塞的 TaskResult 并提交。"
        ],
        "forbiddenOutcomes": [
            "不要以缺失路径、内容或编辑参数调用写入/编辑工具。",
            "不要重复格式错误的写入/编辑工具调用。",
            "不要在 writePayloadReady 为 false 时开始写入。",
            "当不确定性可以表示为失败或阻塞的 TaskResult 时，不要询问用户如何继续。",
            "源码编辑后不要以仅含进度的摘要停止。"
        ]
    })
}

pub(crate) fn verification_command_scheduling_rules() -> Value {
    json!([
        "默认串行运行验证命令；仅只读检查命令可以并行化。",
        "对于可能安装依赖、构建制品、运行测试、启动或探针运行时、清理输出、生成代码、格式化文件、变更缓存或写入文件的命令，不要在同一响应中发出多个工具调用。",
        "对于产生写入的验证命令，运行一个命令，等待其完成，检查结果，然后决定下一个命令。",
        "不确定时将命令视为产生写入，包括 install、build、clean、test、e2e、带 cache/fix 的 lint、带 write 的 format、codegen、dev/start/preview 服务器、运行时检查以及可能写入 node_modules、dist、build、coverage、cache、reports、logs 或 lockfiles 的命令。",
        "当临时运行时为有界探针运行时，仅对该运行时运行就绪、HTTP、API 或浏览器探针，直到清理完成。",
        "按实际完成顺序在 TaskResult 中记录验证命令。"
    ])
}

pub(crate) fn controlled_runtime_probe_rules() -> Value {
    json!([
        "不要将长时间运行的运行时或服务器命令作为前台阻塞验证命令运行。",
        "这适用于监听端口、服务请求、监视文件、打开 preview/dev 服务器、启动工作进程、启动队列或保持进程存活的命令。",
        "如果需要运行时探针，仅在后台启动任务拥有的临时运行时，带有有界就绪窗口，可用时记录 pid、端口和命令，运行探针，然后在写入 TaskResult 之前停止该任务拥有的运行时。",
        "如果运行时报告就绪或正在监听，不要等待自然进程退出；探针就绪目标并完成收尾。",
        "如果环境无法安全启动、探针和清理临时运行时，跳过实时探针并记录静态/代码级证据加上 unverifiedItems 或 completed_with_notes。",
        "运行时探针清理失败、未知清理或不安全清理本身是非阻塞的；记录 runtimeProbeCleanup 并使用 completed_with_notes，除非存在独立的缺陷。"
    ])
}

fn frontend_implementation_organization_rules() -> Value {
    json!([
        "对于前端任务，按职责边界组织实现，而不是一个巨大的混合文件。",
        "存在时使用项目已有的前端结构。",
        "向现有应用添加前端模块时，将其添加为现有应用 shell 中可到达的入口、路由、标签页或导航项。",
        "不要替换、隐藏或移除已有的可到达模块入口，除非需求明确要求替换或移除。",
        "用户可见文案遵循 sourceContext.userFacingLanguage 或 executionRules.userFacingLanguage；不要翻译代码标识符、API 路径、数据库字段、枚举值、包名、框架术语或内部制品 ID。",
        "使 UI/视图、API 或服务交互、状态或反馈处理以及验证证据可区分。",
        "不要为小任务强制将每个职责拆分到单独文件，也不要将多个前端职责合并为不可维护的单个文件。"
    ])
}

fn interactive_verification_probe_policy() -> Value {
    json!({
        "appliesWhen": "任务使用浏览器、e2e、交互式 UI、运行时 UI 或 API 支持的 UI 验证。",
        "deriveProbePlanFrom": [
            "task.verificationIntents[].behavior",
            "task.frontendExperienceRequirement.executionGuidance.uiTaskScope",
            "task.runtimeDeliveryRequirement.requiredCodeLevelChecks"
        ],
        "requiredExecutionPattern": [
            "在运行浏览器、e2e 或交互式代码之前，从当前任务字段派生最小适用探针计划。",
            "仅选择与当前任务职责匹配的探针；不要为缺失的界面、工作流、操作、绑定或运行时检查运行探针。",
            "每个探针必须验证一个交互目标：一个验证意图、工作流步骤、用户操作、前端/后端绑定或运行时检查。",
            "每个探针必须有界，在下一个探针开始之前返回，并产生可观察结果，如可达页面、可见状态、响应状态、结果消息、列表/详情变更或状态转换。",
            "不要将多个业务工作流打包到一个浏览器/e2e 脚本中。"
        ],
        "failureProgressRule": [
            "探针失败时，下一次尝试必须更小、更具体、重置工具上下文或改变失败条件。",
            "当出现新的可观察证据或失败特征改变时继续。",
            "当相同失败特征重复且没有新的可观察证据时停止重试该验证方法。"
        ],
        "taskResultEvidence": [
            "在 verificationResults[].summary 中为匹配的 verificationId 记录成功探针事实，并保持 verificationResults[].provenance 与具体证据引用、更改的文件、测试用例、命令和退出码关联。",
            "当 runtimeDeliveryEvidence 适用时，在 runtimeDeliveryEvidence.commandsRun 中记录运行时命令或探针证据。",
            "根据 TaskResult 规则，在 notes 或 runtimeDeliveryEvidence.unverifiedItems 中记录剩余未验证的职责。"
        ]
    })
}

fn runtime_delivery_execution_rules() -> Value {
    json!({
        "readRuntimeDeliveryRequirement": true,
        "verificationBoundary": "code_level_only",
        "mustKeepContractAndCodeAligned": true,
        "mayEditApplicationCode": true,
        "mayEditPackageScripts": true,
        "mayEditDeployGeneratedFiles": false,
        "mayEditRuntimeDeliveryContract": false,
        "mustRecordRuntimeDeliveryEvidence": true,
        "mustRecordRuntimeProbeCleanupWhenTemporaryRuntimeStarted": true,
        "foregroundRuntimeCommandsForbidden": true,
        "controlledRuntimeProbeRulesField": "executionRules.controlledRuntimeProbeRules",
        "runtimeProbeCleanupFailureSeverity": "completed_with_notes_only",
        "selfRepairWhenCodeLevelCheckFails": true
    })
}

fn task_result_rules(task: &TaskDefinition, has_browser_verification: bool) -> Value {
    let mut rules = vec![
        "TaskResult 必须包含每个 requiredTopLevelFields 条目。".to_string(),
        "如果状态为 completed，每个验证意图都应有通过的证据。".to_string(),
        "如果状态为 failed，failure 是必需的。".to_string(),
        "TaskResult 必须包含 executionContinuity；如果代理拥有的长时间运行工作释放状态未知，状态不能为 completed。".to_string(),
        "对于每个 task.implementationObligations 条目，按提供的顺序提供恰好一个 implementationObligationResults 条目，仅填写 status、evidenceRefs 和 summary。Loom 派生 obligationId 和 verificationIds。必需义务在 status=completed 之前必须被满足；partial、blocked 或 not_verified 义务需要执行修复或非 completed 状态。".to_string(),
        "implementationObligationResults.evidenceRefs 必须指向具体的实现或验证证据。单独的构建、编译或引用读取结果不能满足持久化、安全、状态转换、API 行为或运行时义务，除非其声明的证据能力证明了该目标。".to_string(),
        "implementationObligationResults.evidenceRefs 必须引用任务拥有的源码或测试文件。不要引用 dist、target、build、coverage、report、log、cache 或 node_modules 输出；验证来源携带命令和生成的报告引用。".to_string(),
        "changedFiles 必须列出预期的交付文件，而非附带的依赖目录、缓存、日志或生成的构建输出。".to_string(),
        "当 changedFiles 非空时 noChangeReason 必须为 null；当 changedFiles 为空且需要原因时，noChangeReason 必须是包含 code 和 summary 的对象，绝不能是字符串或数组。".to_string(),
        "对于 completed 或 completed_with_notes 结果，为每个 requirementDetailEvidence 条目提供实质性的 status、evidenceRefs 和 summary；Loom 从任务合同派生 detailId 和 verificationIds。".to_string(),
        "结果模板是一个保守的起始形状：not_run、not_verified、partial、missing 和 not_applicable 条目不是完成证据。仅在相应的工作或验证实际发生后替换它们。".to_string(),
    ];
    if frontend_self_check_applies(task) {
        rules.push("对于前端任务，存在时使用 task.frontendExperienceRequirement.executionGuidance 和前端/后端绑定填写 frontendExperienceSelfCheck；Loom 从分配的闭包合同派生 closureRequirementIds。".to_string());
        rules.push("对于浏览器/e2e/交互式验证，遵循 executionRules.interactiveVerificationProbePolicy 并通过现有 TaskResult 字段记录证据。".to_string());
    }
    if frontend_quality_self_check_applies(task) {
        rules.push("对于 frontendQualitySelfCheck，为任务范围的 uiProductionBrief.surfaceDecisionContract 提供实质性的 status、files、evidence、contentBoundaryEvidence、referencePlanFilesChecked 和令牌证据。按合同顺序提交界面证据数组；Loom 派生 surfaceDecisionContractRef 和每个证据 ID。不要在提交的结果中保留 replace_with_* 值。".to_string());
    }
    if has_browser_verification {
        rules.push("在 verificationResults[].browserChecks 下按 profile 顺序记录每个 sourceContext.browserVerificationContext.profile.checks 结果。Loom 派生 verificationId 和 checkId；不要写入或复制那些链接字段，也不要将 trace、screenshot 或 report 内容粘贴到 TaskResult 正文中。".to_string());
        rules.push("通过的浏览器检查需要确切的命令、尝试次数和简明的观察结果。阻塞的检查需要具体的 blockedReason。保持重试成功可见，尝试次数大于一。".to_string());
    }
    if runtime_delivery_evidence_applies(task) {
        rules.push("对于 runtimeDeliveryRequirement 任务，包含 runtimeDeliveryEvidence 及 checkedFields、codeLevelChecks、运行命令时的 commandsRun 以及环境阻止检查时的 unverifiedItems。".to_string());
        rules.push("对于 runtimeDeliveryEvidence.codeLevelChecks，按 task.runtimeDeliveryRequirement.requiredCodeLevelChecks 顺序报告 status 和 evidence。每个适用检查必须为 passed 或显式 unverified；当检查适用时替换模板的 not_applicable 状态。Loom 派生 requirementRef、checkedFields、checkId 和 contractField。".to_string());
        rules.push("如果启动了临时运行时/探针/服务器/容器，包含 runtimeDeliveryEvidence.runtimeProbeCleanup；单独的清理失败应为 completed_with_notes，而非 failed 或 blocked。".to_string());
    }
    if !task.engineering_quality_requirement_refs.is_empty() {
        rules.push("对于引用的 engineeringQualityRequirements，verificationResults 摘要必须说明实现如何保持此任务的声明 alignmentTargets 对齐。".to_string());
        rules.push("对于 persistence_mapping 需求，证据必须覆盖领域模型、存储模式或迁移、数据访问映射、DTO/API 合同和同提供方持久化行为中更改的风险字段类型（当这些部分在任务范围内时）。".to_string());
    }
    if !task.architecture_quality_requirement_refs.is_empty() {
        rules.push("对于引用的 architectureQualityRequirements，按任务顺序为每个分配的需求提供一个 architectureQualityEvidence 条目；Loom 派生 requirementId 和 verificationIds。模板起始为 not_verified；仅当更改的文件和通过的验证证据证明了引用的决策、NFR 或风险缓解时才设置为 satisfied。".to_string());
    }
    if !task.api_contract_requirement_refs.is_empty() {
        rules.push("对于引用的 apiContractRequirements，按任务顺序为每个分配的需求提供一个 apiContractEvidence 条目；Loom 派生 requirementId、interfaceRefs 和 verificationIds。摘要必须说明更改的文件如何实现或保持了引用的 API 接口。".to_string());
        rules.push("对于受保护的 API 需求，仅使用引用的 securityProfileRefs 和匹配的 sourceContext.technicalBaseline.securityProfiles 条目。阅读需求 referenceLoadPlan 中的每个文件，包括选中时的 tech/api/jwt.md；不要在任务结果中选择算法、签发者、受众或令牌传输方式。".to_string());
        rules.push("apiContractEvidence 模板起始为 not_verified。仅在分配的 API 行为有具体通过的验证证据后才设置 status=satisfied；对于 completed 或 completed_with_notes 结果，保持 knownGaps 为空并在摘要中解释不适用的检查，而非记录为 gap。".to_string());
    }
    if !task.code_quality_requirement_refs.is_empty() {
        rules.push("对于引用的 codeQualityExecutionContext 条目，按任务顺序为每个分配的需求提供一个 codeQualityEvidence 条目；Loom 派生 requirementId 和 verificationIds。referenceFilesChecked 必须准确列出从 sourceContext.codeQualityExecutionContext[].referenceLoadPlan 读取的文件，摘要必须说明更改的文件如何遵循选定的语言、框架、SQL 方言和现有仓库引用。".to_string());
        rules.push("referenceLoadPlan 路径是 Loom 安装的参考路径，而非项目源码路径；在编辑或写入 codeQualityEvidence 之前，在当前 Loom 技能参考根目录下解析它们。".to_string());
        rules.push("将 task.implementationObligations 视为实现闭包合同。task.implementationActions 仅是规范化的操作分类；不要在 TaskResult 中发明或重复义务。".to_string());
        rules.push("codeQualityEvidence 模板起始为 not_verified。仅在每个选定的参考文件被读取且更改的代码被具体通过的验证证据覆盖后才设置 status=satisfied；对于 completed 或 completed_with_notes 结果，knownGaps 必须为空数组。".to_string());
    }
    json!(rules)
}

fn engineering_quality_execution_rules(task: &TaskDefinition) -> Value {
    json!({
        "appliesToRequirementRefs": task.engineering_quality_requirement_refs,
        "requirementSource": "sourceContext.engineeringQualityRequirements",
        "scopeRule": "仅应用 appliesToTaskIds 包含此任务的列出需求；不要在 TaskResult 中创建新需求。",
        "implementationRules": [
            "在编辑影响持久化的代码之前，将 sourceContext.engineeringQualityRequirements[].alignmentTargets 与此任务更改的实体、模式或迁移、仓库、DTO、API 载荷、查询字段和测试进行比较。",
            "保持字段类型语义在代码、存储、数据访问、序列化和测试之间对齐；不要为声明的 riskFieldKinds 依赖提供方默认值。",
            "使用请求中的实际 stackSignals 作为证据来选择提供方兼容的映射；不要硬编码来自不相关技术栈的假设。"
        ],
        "verificationRules": [
            "使用 task.verificationIntents 作为验证 ID 来源。",
            "当实现涉及持久化时，优先使用同提供方测试或运行时检查而非仅模拟证据。",
            "当选择了 MySQL 或 PostgreSQL 方言参考时，针对该提供方验证提供方特定行为或记录确切不可用的提供方行为；不要从 SQLite、H2 或其他数据库声称方言支持。",
            "在 verificationResults[].summary 和 requirementDetailEvidence[].summary 中记录简明的对齐证据。"
        ]
    })
}

fn architecture_quality_execution_rules(task: &TaskDefinition) -> Value {
    json!({
        "appliesToRequirementRefs": task.architecture_quality_requirement_refs,
        "requirementSource": "sourceContext.architectureQualityRequirements",
        "architectureSource": "sourceContext.architectureArtifactProjection.architectureQuality",
        "scopeRule": "仅应用 appliesToTaskIds 包含此任务的列出需求；不要在 TaskResult 中创建新架构需求。",
        "implementationRules": [
            "编辑前，将 sourceContext.architectureQualityRequirements 与任务拥有的模块、接口、数据模型、运行时界面和工作流进行比较。",
            "尊重引用的决策，在范围内时实现引用的风险缓解，并通过代码或验证证据保持引用的 NFR 可观察。",
            "不要为满足不相关的决策、NFR 或风险而将架构范围扩展到当前任务之外。"
        ],
        "verificationRules": [
            "使用 task.verificationIntents 作为验证 ID 来源。",
            "为每个引用的架构质量需求在 architectureQualityEvidence 中记录简明证据。",
            "验证摘要必须说明更改的文件如何尊重引用的决策、NFR 或风险缓解。"
        ]
    })
}

fn api_contract_execution_rules(task: &TaskDefinition) -> Value {
    json!({
        "appliesToRequirementRefs": task.api_contract_requirement_refs,
        "requirementSource": "sourceContext.apiContractRequirements",
        "interfaceSource": "sourceContext.architectureArtifactProjection.interfaces",
        "bindingSource": "sourceContext.architectureArtifactProjection.apiContract",
        "scopeRule": "仅应用 appliesToTaskIds 包含此任务的列出 API 合同需求；不要在 TaskResult 中创建新 API 需求。",
        "implementationRules": [
            "在编辑 API 或客户端绑定代码之前，将 sourceContext.apiContractRequirements 与任务拥有的 AAC 接口进行比较。",
            "保持方法、路径、请求模式、响应模式、状态码类别、错误模式、认证策略和分页策略与 AAC 接口对齐。",
            "对于受保护的接口，保持 securityProfileRef 和所选配置文件的机制、算法、密钥来源、传输、签发者、受众和声明对齐；不要扩大或替换规范配置文件。",
            "不要用通用 500 响应或静默成功替换业务错误。",
            "除非 AAC 接口或需求明确声明，不要添加版本化路径或 OpenAPI 文件。"
        ],
        "verificationRules": [
            "使用 task.verificationIntents 作为验证 ID 来源。",
            "为每个引用的 API 合同需求在 apiContractEvidence 中记录简明证据。",
            "对于写入或状态转换 API，可行时验证应覆盖一个成功路径和一个验证或业务阻塞错误路径。",
            "对于集合 API，存在时验证应覆盖声明的分页或过滤行为。"
        ]
    })
}

fn code_quality_execution_rules(task: &TaskDefinition) -> Value {
    json!({
        "appliesToRequirementRefs": task.code_quality_requirement_refs,
        "requirementSource": "sourceContext.codeQualityExecutionContext",
        "scopeRule": "仅应用 appliesToTaskIds 包含此任务的列出代码质量需求；不要在 TaskResult 中创建新语言或框架需求。",
        "referenceLoadRule": "仅加载 sourceContext.codeQualityExecutionContext[].referenceLoadPlan 中列出的文件。这些路径相对于已安装的 Loom 技能参考根目录，而非项目工作区。不要从 referenceGroups 派生路径、扫描 tech/code 或 tech/backend 树或加载外部语言/框架技能。",
        "referencePathResolution": {
            "pathMeaning": "Loom 安装的参考路径",
            "projectWorkspacePath": false,
            "opencodeHint": "从活动的 OpenCode loom 命令/插件文件解析为 ../references/loom/<path>。"
        },
        "implementationRules": [
            "将 task.implementationActions 和 task.objective 作为此任务写入边界内的源码编辑决策；代码质量参考约束实现选择但不创建第二个任务范围。",
            "编辑前，将选定的语言/框架参考与现有仓库模式进行比较，当两者都有效时优先使用现有项目约定。",
            "将 API、UI、架构、运行时和持久化义务保留在各自的专用合同中；代码质量需求仅用于语言/框架实现规范。",
            "当选定的参考计划包含 tech/backend/springboot/mybatis-plus 时，将该计划作为此任务唯一的 MyBatis-Plus 指导；不要加载 JPA、MyBatis-Flex、普通 MyBatis 或未列出的外部持久化参考。",
            "当代码质量需求包含 packageNamingPolicy 时，生产源码包声明必须遵循该策略；占位符包根不可接受为交付代码。",
            "当选定的语言或框架参考不适用于更改的文件但需求仍被满足时，在摘要中解释不适用性而不添加 knownGaps。",
            "如果选定的 Loom 参考无法从安装的参考根目录加载，不要将 codeQualityEvidence 标记为 satisfied；将未解析的参考报告为阻塞的合同问题，而非将项目工作区视为缺失源文件。"
        ],
        "verificationRules": [
            "使用 task.verificationIntents 作为验证 ID 来源。",
            "运行可证明更改代码的最小编译、类型、lint、单元或集成检查。",
            "在 codeQualityEvidence 中记录选定的参考组和参考文件。更改的文件和命令属于规范 TaskResult changedFiles 和 verificationResults[].provenance 字段；不要在 codeQualityEvidence 中重复它们。",
            "对于 completed 或 completed_with_notes 结果，codeQualityEvidence.status 必须为 satisfied 且 knownGaps 必须为空数组；与某个更改文件无关的参考在摘要中解释，不记录为 gap。",
            "当结果模板将 requirementId 或 verificationIds 标记为 MCP 派生时不要编写它们；按分配的需求顺序保持证据条目，让 Loom 规范化链接字段。"
        ]
    })
}

pub(crate) fn browser_verification_profile_for_task<'a>(
    task_plan: &'a TaskPlan,
    task: &TaskDefinition,
) -> Option<&'a BrowserVerificationProfile> {
    task_plan
        .browser_verification_profiles
        .iter()
        .find(|profile| profile.task_id == task.task_id)
}

pub(crate) fn browser_verification_context(
    project_root: &Path,
    task_plan: &TaskPlan,
    profile: &BrowserVerificationProfile,
) -> Value {
    let installation = profile
        .installation_id
        .as_ref()
        .and_then(|installation_id| {
            task_plan
                .browser_automation_facts
                .installations
                .iter()
                .find(|installation| &installation.installation_id == installation_id)
        });
    let runtime = state::store::read_json_value(
        &project_root.join(".loom/runtime/browser-automation/latest.json"),
    )
    .ok()
    .filter(|value| {
        matches!(
            value.get("status").and_then(Value::as_str),
            Some("ready" | "partial")
        )
    })
    .map(|value| {
        json!({
            "status": value.get("status").cloned().unwrap_or(Value::Null),
            "projectTargets": value.get("projectTargets").cloned().unwrap_or_else(|| json!([])),
            "runtimeEnvironments": value.get("runtimeEnvironments").cloned().unwrap_or_else(|| json!([]))
        })
    });
    json!({
        "profile": profile,
        "projectRunner": installation,
        "baselineSelection": task_plan.browser_automation_facts.baseline_selection,
        "runtime": runtime
    })
}

pub(crate) fn browser_verification_rules() -> Value {
    json!({
        "profileAuthority": "sourceContext.browserVerificationContext.profile",
        "referenceLoadRule": "仅读取 sourceContext.browserVerificationContext.profile.referenceLoadPlan 中列出的文件。路径相对于已安装的 Loom 技能参考根目录；不要浏览同级测试参考或加载外部 Playwright 技能。",
        "scopeRule": "仅运行此 MCP 生成的浏览器质量闭包中的 profile.checks，并将每个检查限定在其源任务、源验证、视口、后端模式和强制级别。",
        "runnerRule": "存在时复用 sourceContext.browserVerificationContext.projectRunner。不要替换已有的项目运行器或安装第二个项目局部 Playwright 栈。",
        "runnerBootstrapRule": "当 projectRunner 缺失且 profile.runnerSource 为 baseline_selected 或 loom_managed 时，仅为此闭包创建首个项目拥有的 Playwright 依赖/配置，将 @playwright/test 固定到 runtime.runtimeEnvironments 提供的确切 resolvedVersion，并更新项目 lockfile。共享运行器仍是准备/诊断资产，从不复制到项目中。",
        "runtimeAuthority": "MCP 在创建此执行请求之前准备了 sourceContext.browserVerificationContext.runtime。不要从任务内部调用 loom.browserRuntimePrepare、临时安装浏览器或编辑共享缓存状态。",
        "runtimeExecutionRule": "选择请求/解析版本与项目运行器匹配的运行时环境。对于 host 后端，将 browserEnvironment 应用于项目局部运行器。对于 managed_container 后端，使用其 commandPrefix 和 browserEnvironment，不将共享资产复制到项目中；当被测服务运行在宿主上时，将回环 base URL 转换为 managedContainer.hostGateway，同时保留实际端口。",
        "environmentFailureRule": "仅当提供的宿主/容器浏览器环境无法启动或执行时使用 blocked，并包含确切的环境诊断；Loom 将其分类在通用执行修复之外。应用启动、API、选择器、断言和工作流失败是产品证据，必须保持 failed。",
        "resultRule": "通过 verificationResults[].browserChecks 记录浏览器结果；不要将 Playwright 报告、trace、screenshot 或控制台日志粘贴到正文字段中。MCP 将闭包检查关联到源 UI 任务。"
    })
}

fn task_execution_read_groups(
    task: &TaskDefinition,
    has_dependency_results: bool,
    architecture_projection: &Value,
    has_browser_verification: bool,
) -> Value {
    let has_frontend_execution = task_has_frontend_execution(task);
    let has_frontend_requirement = task.frontend_experience_requirement.is_some();
    let needs_runtime_probe_rules = task_needs_controlled_runtime_probe_rules(task);
    let core_fields = vec![
        "source.taskPlanId",
        "source.taskId",
        "source.groupId",
        "source.technicalBaselineId",
        "source.architectureArtifactContractId",
        "source.taskPlanRunId",
        "task.taskId",
        "task.groupId",
        "task.title",
        "task.taskKind",
        "task.implementationActions",
        "task.implementationObligations",
        "task.objective",
        "task.dependsOn",
        "task.scopeRefs",
        "task.acceptanceRefs",
        "task.requirementDetailRefs",
        "task.writeBoundary.forbiddenPaths",
        "task.writeBoundary.artifactRefs",
        "task.verificationIntents",
        "sourceContext.technicalBaseline.projectKind",
        "sourceContext.userFacingLanguage",
        "executionRules.sourceEditPreparationContract",
        "executionRules.boundaryRules",
        "executionRules.taskResponsibilityBoundary",
    ];
    let mut scope_fields = vec![
        "sourceContext.acceptanceSnapshot",
        "sourceContext.requirementDetailSnapshot",
        "executionRules.verificationCommandSchedulingRules",
        "executionRules.userFacingLanguage",
    ];
    if has_dependency_results {
        scope_fields.push("sourceContext.dependencyResults");
    }

    let mut architecture_fields = Vec::new();
    if projection_array_has_items(architecture_projection, "modules") {
        architecture_fields.push("sourceContext.architectureArtifactProjection.modules");
    }
    if projection_array_has_items(architecture_projection, "entities") {
        architecture_fields.push("sourceContext.architectureArtifactProjection.entities");
    }
    if projection_array_has_items(architecture_projection, "interfaces") {
        architecture_fields.push("sourceContext.architectureArtifactProjection.interfaces");
        architecture_fields.push("sourceContext.architectureArtifactProjection.apiContract");
    }
    if architecture_projection
        .get("apiContract")
        .is_some_and(|value| !value.is_null())
        && !architecture_fields
            .contains(&"sourceContext.architectureArtifactProjection.apiContract")
    {
        architecture_fields.push("sourceContext.architectureArtifactProjection.apiContract");
    }
    if projection_array_has_items(architecture_projection, "userFlows") {
        architecture_fields.push("sourceContext.architectureArtifactProjection.userFlows");
    }
    if projection_array_has_items(architecture_projection, "stateMachines") {
        architecture_fields.push("sourceContext.architectureArtifactProjection.stateMachines");
    }
    if projection_architecture_quality_has_items(architecture_projection) {
        architecture_fields
            .push("sourceContext.architectureArtifactProjection.architectureQuality");
    }

    let mut frontend_fields = Vec::new();
    if has_frontend_requirement {
        frontend_fields.extend([
            "task.frontendExperienceRequirement.executionGuidance.schemaVersion",
            "task.frontendExperienceRequirement.executionGuidance.purpose",
            "task.frontendExperienceRequirement.executionGuidance.userFacingLanguage",
            "task.frontendExperienceRequirement.executionGuidance.responsibility",
            "task.frontendExperienceRequirement.executionGuidance.uiTaskScope",
            "task.frontendExperienceRequirement.executionGuidance.dataBindingExpectation",
            "task.frontendExperienceRequirement.executionGuidance.closureRequirementRefs",
            "task.frontendExperienceRequirement.executionGuidance.workflowClosureDetailSource",
            "task.frontendExperienceRequirement.executionGuidance.guidanceWarnings",
            "task.frontendExperienceRequirement.executionGuidance.uiProductionBrief",
            "task.frontendExperienceRequirement.executionGuidance.styleAssetPlan",
            "task.frontendExperienceRequirement.uiSurfaceDecisionContractRef",
        ]);
    }
    if has_frontend_execution {
        frontend_fields.push("executionRules.frontendImplementationOrganizationRules");
        frontend_fields.push("executionRules.interactiveVerificationProbePolicy");
    }

    let mut runtime_fields = Vec::new();
    if needs_runtime_probe_rules {
        runtime_fields.push("executionRules.controlledRuntimeProbeRules");
    }
    if runtime_delivery_evidence_applies(task) {
        runtime_fields.extend(runtime_delivery_requirement_read_fields(task));
        runtime_fields.push("sourceContext.architectureArtifactProjection.runtimeDelivery");
        runtime_fields.push("executionRules.runtimeDeliveryExecutionRules");
    }

    let browser_fields = if has_browser_verification {
        vec![
            "sourceContext.browserVerificationContext",
            "executionRules.browserVerificationRules",
        ]
    } else {
        vec![]
    };

    let mut quality_fields = Vec::new();
    if !task.engineering_quality_requirement_refs.is_empty() {
        quality_fields.extend([
            "task.engineeringQualityRequirementRefs",
            "sourceContext.engineeringQualityRequirements",
            "executionRules.engineeringQualityExecutionRules",
        ]);
    }
    if !task.architecture_quality_requirement_refs.is_empty() {
        quality_fields.extend([
            "task.architectureQualityRequirementRefs",
            "sourceContext.architectureQualityRequirements",
            "executionRules.architectureQualityExecutionRules",
        ]);
    }
    if !task.api_contract_requirement_refs.is_empty() {
        quality_fields.extend([
            "task.apiContractRequirementRefs",
            "sourceContext.apiContractRequirements",
            "executionRules.apiContractExecutionRules",
        ]);
    }
    if !task.code_quality_requirement_refs.is_empty() {
        quality_fields.extend([
            "task.codeQualityRequirementRefs",
            "sourceContext.codeQualityExecutionContext",
            "executionRules.codeQualityExecutionRules",
        ]);
    }
    let mut concept_fields = Vec::new();
    if !task.concept_refs.is_empty() {
        concept_fields.push("task.conceptRefs");
    }
    if !task.concept_responsibilities.is_empty() {
        concept_fields.push("task.conceptResponsibilities");
    }
    if !task.concept_verification_intents.is_empty() {
        concept_fields.push("task.conceptVerificationIntents");
    }

    let mut result_fields = vec![
        "enumRefs.taskResultStatus",
        "enumRefs.verificationStatus",
        "enumRefs.verificationEvidence",
        "enumRefs.selfRepairStopReason",
        "executionRules.completionBarrier",
        "executionRules.finalResponseGuard",
        "executionRules.completionContinuityRequirement",
        "executionRules.implementationClosureContract",
        "executionRules.verificationCommandSchedulingRules",
    ];
    result_fields.extend(task_result_contract_read_fields(task));

    let mut groups = vec![
        json!({
            "groupId": "task_execution_core",
            "required": true,
            "purpose": "在编辑前读取任务标识、编辑边界和源码编辑规则。",
            "whenToRead": "在任何源码编辑之前读取。",
            "selectors": read_selectors_value_from_paths(core_fields)
        }),
        json!({
            "groupId": "task_execution_scope_context",
            "required": true,
            "purpose": "读取任务范围的验收、需求详情、依赖和语言上下文。",
            "whenToRead": "在决定实现范围之前读取。",
            "selectors": read_selectors_value_from_paths(scope_fields)
        }),
    ];
    if !architecture_fields.is_empty() {
        groups.push(json!({
            "groupId": "task_execution_architecture_context",
            "required": true,
            "purpose": "仅读取此任务所需的任务拥有架构制品。",
            "whenToRead": "在编辑架构拥有的代码之前读取。",
            "selectors": read_selectors_value_from_paths(architecture_fields)
        }));
    }
    if !frontend_fields.is_empty() {
        groups.push(json!({
            "groupId": "task_execution_frontend_context",
            "required": true,
            "purpose": "读取任务拥有的前端指导和 UI 质量合同。",
            "whenToRead": "在编辑前端界面之前读取。",
            "selectors": read_selectors_value_from_paths(frontend_fields)
        }));
    }
    if !runtime_fields.is_empty() {
        groups.push(json!({
            "groupId": "task_execution_runtime_context",
            "required": true,
            "purpose": "仅在此任务需要运行时证据时读取运行时交付和受控探针规则。",
            "whenToRead": "在运行时影响的编辑或探针之前读取。",
            "selectors": read_selectors_value_from_paths(runtime_fields)
        }));
    }
    if !browser_fields.is_empty() {
        groups.push(json!({
            "groupId": "task_execution_browser_verification",
            "required": true,
            "purpose": "读取 MCP 派生的浏览器检查、选定的运行器事实和任务范围的 Playwright 参考计划。",
            "whenToRead": "在创建、更改或运行浏览器验证之前读取。",
            "selectors": read_selectors_value_from_paths(browser_fields)
        }));
    }
    if !quality_fields.is_empty() {
        groups.push(json!({
            "groupId": "task_execution_quality_context",
            "required": true,
            "purpose": "仅读取分配给此任务的质量合同。",
            "whenToRead": "在应用工程、架构、API 或代码质量义务之前读取。",
            "selectors": read_selectors_value_from_paths(quality_fields)
        }));
    }
    if !concept_fields.is_empty() {
        groups.push(json!({
            "groupId": "task_execution_concept_context",
            "required": true,
            "purpose": "读取分配给此任务的概念职责。",
            "whenToRead": "在实现概念敏感行为之前读取。",
            "selectors": read_selectors_value_from_paths(concept_fields)
        }));
    }
    groups.push(json!({
        "groupId": "task_execution_result_contract",
        "required": true,
        "purpose": "读取 TaskResult 输出文件、模式字段、枚举值和完成屏障。",
        "whenToRead": "在写入 TaskResult 之前读取。",
        "selectors": read_selectors_value_from_paths(result_fields)
    }));
    Value::Array(groups)
}

fn projection_array_has_items(projection: &Value, key: &str) -> bool {
    projection
        .get(key)
        .and_then(Value::as_array)
        .is_some_and(|items| !items.is_empty())
}

fn projection_architecture_quality_has_items(projection: &Value) -> bool {
    let Some(quality) = projection.get("architectureQuality") else {
        return false;
    };
    ["decisions", "nfrs", "risks"].iter().any(|key| {
        quality
            .get(*key)
            .and_then(Value::as_array)
            .is_some_and(|items| !items.is_empty())
    })
}

fn task_scoped_engineering_quality_requirements(
    task_plan: &TaskPlan,
    task: &TaskDefinition,
) -> Vec<EngineeringQualityRequirement> {
    if task.engineering_quality_requirement_refs.is_empty() {
        return vec![];
    }
    let refs = task
        .engineering_quality_requirement_refs
        .iter()
        .collect::<BTreeSet<_>>();
    task_plan
        .engineering_quality_requirements
        .iter()
        .filter(|requirement| refs.contains(&requirement.requirement_id))
        .cloned()
        .collect()
}

fn task_scoped_architecture_quality_requirements(
    task_plan: &TaskPlan,
    task: &TaskDefinition,
) -> Vec<ArchitectureQualityRequirement> {
    if task.architecture_quality_requirement_refs.is_empty() {
        return vec![];
    }
    let refs = task
        .architecture_quality_requirement_refs
        .iter()
        .collect::<BTreeSet<_>>();
    task_plan
        .architecture_quality_requirements
        .iter()
        .filter(|requirement| refs.contains(&requirement.requirement_id))
        .cloned()
        .collect()
}

fn task_scoped_api_contract_requirements(
    task_plan: &TaskPlan,
    task: &TaskDefinition,
) -> Vec<ApiContractRequirement> {
    if task.api_contract_requirement_refs.is_empty() {
        return vec![];
    }
    let refs = task
        .api_contract_requirement_refs
        .iter()
        .collect::<BTreeSet<_>>();
    task_plan
        .api_contract_requirements
        .iter()
        .filter(|requirement| refs.contains(&requirement.requirement_id))
        .cloned()
        .collect()
}

fn task_has_frontend_execution(task: &TaskDefinition) -> bool {
    task.frontend_experience_requirement.is_some()
        || matches!(
            task.task_kind,
            TaskKind::UiFlowIncrement | TaskKind::FrontendExperience
        )
        || task.implementation_actions.iter().any(|action| {
            matches!(
                action,
                ImplementationAction::CreateOrUpdateUiFlow
                    | ImplementationAction::CreateOrUpdateFrontendNavigation
                    | ImplementationAction::ImplementReactiveClientFlow
                    | ImplementationAction::ImplementSharedClientState
                    | ImplementationAction::OptimizeFrontendPerformance
                    | ImplementationAction::ImplementServerRenderedComponent
                    | ImplementationAction::ImplementServerMutation
                    | ImplementationAction::ImplementFrontendFrameworkVersionFeature
                    | ImplementationAction::ImplementFrontendExperienceContract
            )
        })
}

fn task_needs_controlled_runtime_probe_rules(task: &TaskDefinition) -> bool {
    runtime_delivery_evidence_applies(task)
        || task_has_frontend_execution(task)
        || task.verification_intents.iter().any(|intent| {
            intent
                .preferred_evidence
                .iter()
                .chain(intent.acceptable_evidence.iter())
                .any(|evidence| *evidence == VerificationEvidence::RuntimeApiCheck)
        })
}

pub(crate) fn runtime_delivery_requirement_read_fields(task: &TaskDefinition) -> Vec<&'static str> {
    runtime_delivery_requirement_read_fields_for_prefix(task, false)
}

pub(crate) fn task_projection_runtime_delivery_requirement_read_fields(
    task: &TaskDefinition,
) -> Vec<&'static str> {
    runtime_delivery_requirement_read_fields_for_prefix(task, true)
}

fn runtime_delivery_requirement_read_fields_for_prefix(
    task: &TaskDefinition,
    task_projection: bool,
) -> Vec<&'static str> {
    let Some(requirement) = task.runtime_delivery_requirement.as_ref() else {
        return vec![];
    };
    let (applies, reason, runtime_ref, affected, checks, expected, forbidden, source, failure_ref) =
        if task_projection {
            (
                "taskProjection.runtimeDeliveryRequirement.appliesToThisTask",
                "taskProjection.runtimeDeliveryRequirement.reason",
                "taskProjection.runtimeDeliveryRequirement.runtimeDeliveryRef",
                "taskProjection.runtimeDeliveryRequirement.affectedContractFields",
                "taskProjection.runtimeDeliveryRequirement.requiredCodeLevelChecks",
                "taskProjection.runtimeDeliveryRequirement.evidenceExpectedInTaskResult",
                "taskProjection.runtimeDeliveryRequirement.forbiddenActions",
                "taskProjection.runtimeDeliveryRequirement.source",
                "taskProjection.runtimeDeliveryRequirement.deploymentFailureRef",
            )
        } else {
            (
                "task.runtimeDeliveryRequirement.appliesToThisTask",
                "task.runtimeDeliveryRequirement.reason",
                "task.runtimeDeliveryRequirement.runtimeDeliveryRef",
                "task.runtimeDeliveryRequirement.affectedContractFields",
                "task.runtimeDeliveryRequirement.requiredCodeLevelChecks",
                "task.runtimeDeliveryRequirement.evidenceExpectedInTaskResult",
                "task.runtimeDeliveryRequirement.forbiddenActions",
                "task.runtimeDeliveryRequirement.source",
                "task.runtimeDeliveryRequirement.deploymentFailureRef",
            )
        };
    let mut fields = vec![applies, reason];
    if requirement.runtime_delivery_ref.is_some() {
        fields.push(runtime_ref);
    }
    if !requirement.affected_contract_fields.is_empty() {
        fields.push(affected);
    }
    if !requirement.required_code_level_checks.is_empty() {
        fields.push(checks);
    }
    if !requirement.evidence_expected_in_task_result.is_empty() {
        fields.push(expected);
    }
    if !requirement.forbidden_actions.is_empty() {
        fields.push(forbidden);
    }
    if requirement.source.is_some() {
        fields.push(source);
    }
    if requirement.deployment_failure_ref.is_some() {
        fields.push(failure_ref);
    }
    fields
}

pub(crate) fn task_with_phase_execution_guidance(
    project_root: &Path,
    locator: &DeliveryPhaseLocator,
    task: TaskDefinition,
) -> Result<TaskDefinition, state::store::StateError> {
    if task.frontend_experience_requirement.is_none() {
        return Ok(task);
    }
    let planning_ref = format!(
        ".loom/deliveries/{}/contracts/planning/{}/pgc.json",
        locator.delivery_id, locator.phase_id
    );
    let architecture_ref = format!(
        ".loom/deliveries/{}/contracts/architecture/{}/aac.json",
        locator.delivery_id, locator.phase_id
    );
    let pgc: contracts::PlanningGenerationContract =
        read_project_json(project_root, &planning_ref)?;
    let aac: ArchitectureArtifactContract = read_project_json(project_root, &architecture_ref)?;
    Ok(task_with_execution_guidance(
        task,
        &aac,
        &pgc.planning_inputs.user_facing_language,
    ))
}

pub(crate) fn task_with_execution_guidance(
    mut task: TaskDefinition,
    aac: &ArchitectureArtifactContract,
    user_facing_language: &Option<contracts::UserFacingLanguageConstraint>,
) -> TaskDefinition {
    if task.frontend_experience_requirement.is_none() {
        return task;
    }
    let guidance = build_frontend_execution_guidance(&task, aac, user_facing_language);
    let Some(requirement) = task.frontend_experience_requirement.as_mut() else {
        return task;
    };
    let Some(requirement_object) = requirement.as_object_mut() else {
        *requirement = json!({ "executionGuidance": guidance });
        return task;
    };
    requirement_object.insert("executionGuidance".to_string(), guidance);
    task
}

fn task_scoped_architecture_projection(
    aac: &ArchitectureArtifactContract,
    project_api_contract: Option<&Value>,
    task: &TaskDefinition,
) -> Value {
    let refs = &task.write_boundary.artifact_refs;
    let selected_user_flows =
        selected_values(&aac.user_flows, "flowId", &refs.user_flows, task, true);
    let (interface_refs_from_flows, state_machine_refs_from_flows) =
        behavior_refs_from_user_flows(&selected_user_flows);
    let interface_refs = unique_strings(
        refs.interfaces
            .iter()
            .cloned()
            .chain(interface_refs_from_flows)
            .collect(),
    );
    let state_machine_refs = unique_strings(
        refs.state_machines
            .iter()
            .cloned()
            .chain(state_machine_refs_from_flows)
            .collect(),
    );
    let mut selected_interfaces =
        selected_values(&aac.interfaces, "interfaceId", &interface_refs, task, true);
    for interface in interfaces_for_refs(project_api_contract, &interface_refs) {
        let interface_id = interface.get("interfaceId").and_then(Value::as_str);
        if !selected_interfaces
            .iter()
            .any(|existing| existing.get("interfaceId").and_then(Value::as_str) == interface_id)
        {
            selected_interfaces.push(interface);
        }
    }
    let mut projection = json!({
        "compaction": {
            "mode": "task_scoped_artifact_projection",
            "rule": "此投影仅包含由 task.writeBoundary.artifactRefs、直接关联的工作流引用或任务范围/验收引用选择的制品。"
        },
        "modules": selected_values(&aac.modules, "moduleId", &refs.modules, task, true),
        "entities": selected_entities(&aac.data_model, &refs.entities, task),
        "interfaces": selected_interfaces,
        "userFlows": selected_user_flows,
        "stateMachines": selected_values(&aac.state_machines, "machineId", &state_machine_refs, task, true),
        "architectureQuality": {
            "decisions": aac.architecture_quality.decisions.iter()
                .filter(|decision| refs.decisions.iter().any(|item| item == &decision.decision_id))
                .cloned()
                .collect::<Vec<_>>(),
            "nfrs": aac.architecture_quality.nfrs.iter()
                .filter(|nfr| refs.nfrs.iter().any(|item| item == &nfr.nfr_id))
                .cloned()
                .collect::<Vec<_>>(),
            "risks": aac.architecture_quality.risks.iter()
                .filter(|risk| refs.risks.iter().any(|item| item == &risk.risk_id))
                .cloned()
                .collect::<Vec<_>>()
        }
    });
    if !interface_refs.is_empty() {
        projection["apiContract"] =
            exposure_projection(aac.api_contract_ref.as_deref(), project_api_contract);
    }
    if runtime_delivery_evidence_applies(task) {
        projection["runtimeDelivery"] = aac.runtime_delivery.clone().unwrap_or(Value::Null);
    }
    projection
}

fn behavior_refs_from_user_flows(user_flows: &[Value]) -> (Vec<String>, Vec<String>) {
    let steps = user_flows
        .iter()
        .flat_map(|flow| array_at(flow, "happyPath"))
        .collect::<Vec<_>>();
    let interface_refs = unique_strings(
        steps
            .iter()
            .filter_map(|step| string_at(step, "interactionRef"))
            .collect(),
    );
    let state_machine_refs = unique_strings(
        steps
            .iter()
            .flat_map(|step| string_array_at(step, "stateMachineRefs"))
            .collect(),
    );
    (interface_refs, state_machine_refs)
}

fn build_frontend_execution_guidance(
    task: &TaskDefinition,
    aac: &ArchitectureArtifactContract,
    user_facing_language: &Option<contracts::UserFacingLanguageConstraint>,
) -> Value {
    let Some(frontend) = aac.frontend_experience.as_ref() else {
        return json!({
            "schemaVersion": "1.0",
            "purpose": "此任务没有 AAC frontendExperience。",
            "userFacingLanguage": user_facing_language,
            "responsibility": task.objective,
            "uiTaskScope": empty_ui_task_scope(),
            "dataBindingExpectation": {
                "allowedModes": ["wired", "mocked_with_reason", "static_only_with_reason", "not_applicable"]
            },
            "closureRequirementRefs": [],
            "workflowClosureDetailSource": {
                "closureRequirementIds": [],
                "derivationRule": "此任务没有 AAC frontendExperience。"
            },
            "uiProductionBrief": Value::Null,
            "styleAssetPlan": Value::Null,
            "guidanceWarnings": ["AAC frontendExperience 缺失。"]
        });
    };
    let closure_requirements = workflow_closure_requirements_for_task(task, aac);
    let task_scope = frontend_task_scope(task, aac, frontend, &closure_requirements);
    let surfaces = selected_frontend_surfaces(frontend, &task_scope.surface_refs);
    let operation_paths = selected_frontend_values(
        frontend,
        "operationPaths",
        "pathId",
        &task_scope.operation_path_refs,
    );
    let data_views =
        selected_frontend_values(frontend, "dataViews", "viewId", &task_scope.data_view_refs);
    let actions =
        selected_frontend_values(frontend, "actions", "actionId", &task_scope.action_refs);
    let mut frontend_backend_bindings = frontend_backend_bindings(&closure_requirements);
    if frontend_backend_bindings.is_empty() {
        frontend_backend_bindings = frontend_backend_bindings_from_scope(aac, &task_scope);
    }
    let closure_ids = closure_requirements
        .iter()
        .filter_map(|item| string_at(item, "closureId"))
        .collect::<Vec<_>>();
    let mut warnings = Vec::new();
    if closure_requirements.is_empty() {
        warnings.push("没有工作流闭包需求匹配此任务；UI 范围从 AAC uiSurfaceRegistry、详情覆盖、任务引用和前端操作路径派生。".to_string());
    }
    if surfaces.is_empty() && task_has_frontend_execution(task) {
        warnings.push("没有任务特定的 UI 界面匹配此任务；使用 uiProductionBrief 将实现保持在业务界面范围内，避免无关的 UI 扩展。".to_string());
    }
    json!({
        "schemaVersion": "1.0",
        "purpose": "从 AAC 和 TaskPlan 引用派生的任务范围前端执行指导。",
        "userFacingLanguage": user_facing_language,
        "responsibility": task.objective,
        "uiTaskScope": ui_task_scope_projection(
            frontend,
            &task_scope,
            &surfaces,
            &data_views,
            &actions,
            &operation_paths,
            &frontend_backend_bindings,
        ),
        "dataBindingExpectation": {
            "allowedModes": ["wired", "mocked_with_reason", "static_only_with_reason", "not_applicable"],
            "requiredModeForSatisfaction": if closure_requirements.is_empty() { Value::Null } else { json!("wired") },
            "closureRequirementIds": closure_ids,
            "staticModePolicy": if closure_requirements.is_empty() { Value::Null } else { json!("not_satisfied") },
            "knownGapPolicy": if closure_requirements.is_empty() { Value::Null } else { json!("not_satisfied_when_required_closure") }
        },
        "closureRequirementRefs": workflow_closure_requirement_execution_view(&closure_requirements),
        "workflowClosureDetailSource": {
            "closureRequirementIds": closure_requirements.iter().filter_map(|item| string_at(item, "closureId")).collect::<Vec<_>>(),
            "detailAuthority": "使用此请求中的 closureRequirementRefs、frontendBackendBindings 和 sourceContext.architectureArtifactProjection。",
            "derivationRule": "闭包引用从 AAC frontendExperience 界面或操作路径、任务 userFlows、结构化快乐路径步骤和可执行接口派生。"
        },
        "uiProductionBrief": ui_production_brief(task, frontend, &task_scope, user_facing_language),
        "styleAssetPlan": style_asset_plan(frontend),
        "guidanceWarnings": warnings
    })
}

fn empty_ui_task_scope() -> Value {
    json!({
        "source": "MCP 派生的 TaskPlan uiTaskScope 投影",
        "surfacesInScope": [],
        "dataViewsInScope": [],
        "actionsInScope": [],
        "operationPathsInScope": [],
        "frontendBackendBindings": [],
        "regionsInScope": [],
        "actionsInContract": [],
        "statesInContract": [],
        "qualityRulesInScope": [],
        "ownershipDimensions": []
    })
}

fn ui_task_scope_projection(
    frontend: &Value,
    scope: &FrontendTaskScope,
    surfaces: &[Value],
    data_views: &[Value],
    actions: &[Value],
    operation_paths: &[Value],
    bindings: &[Value],
) -> Value {
    let surface_contract = frontend
        .get("uiSurfaceDecisionContract")
        .unwrap_or(&Value::Null);
    json!({
        "source": "MCP 派生的 TaskPlan uiTaskScope 投影",
        "surfaceIds": scope.surface_refs,
        "surfacesInScope": surfaces,
        "dataViewsInScope": data_views,
        "actionsInScope": actions,
        "operationPathsInScope": operation_paths,
        "frontendBackendBindings": bindings,
        "regionsInScope": selected_surface_contract_values(surface_contract, "regionModel", "regionId", &scope.surface_region_refs),
        "actionsInContract": selected_surface_contract_values(surface_contract, "actionModel", "actionId", &scope.surface_action_refs),
        "statesInContract": selected_surface_contract_values(surface_contract, "stateModel", "state", &scope.state_refs),
        "qualityRulesInScope": selected_surface_contract_values(surface_contract, "qualityRules", "ruleId", &scope.quality_rule_refs),
        "ownershipDimensions": scope.ownership_dimensions,
        "layoutBaseline": surface_contract.get("layoutModel").cloned().unwrap_or(Value::Null),
        "informationModel": surface_contract.get("informationModel").cloned().unwrap_or(Value::Null),
        "contentBoundary": surface_contract.get("contentBoundary").cloned().unwrap_or(Value::Null)
    })
}

#[derive(Default)]
struct FrontendTaskScope {
    ownership_dimensions: Vec<String>,
    surface_refs: Vec<String>,
    surface_region_refs: Vec<String>,
    surface_action_refs: Vec<String>,
    data_view_refs: Vec<String>,
    action_refs: Vec<String>,
    operation_path_refs: Vec<String>,
    workflow_refs: Vec<String>,
    interface_refs: Vec<String>,
    state_refs: Vec<String>,
    quality_rule_refs: Vec<String>,
}

fn frontend_task_scope(
    task: &TaskDefinition,
    aac: &ArchitectureArtifactContract,
    frontend: &Value,
    closure_requirements: &[Value],
) -> FrontendTaskScope {
    let mut scope = FrontendTaskScope::default();
    push_unique_strings(
        &mut scope.workflow_refs,
        task.write_boundary.artifact_refs.user_flows.clone(),
    );
    push_unique_strings(
        &mut scope.interface_refs,
        task.write_boundary.artifact_refs.all_interfaces(),
    );
    for requirement in closure_requirements {
        push_unique(
            &mut scope.workflow_refs,
            string_at(requirement, "workflowRef"),
        );
        push_unique_strings(
            &mut scope.surface_refs,
            string_array_at(requirement, "surfaceRefs"),
        );
        push_unique_strings(
            &mut scope.operation_path_refs,
            string_array_at(requirement, "operationPathRefs"),
        );
        push_unique_strings(
            &mut scope.data_view_refs,
            string_array_at(requirement, "dataViewRefs"),
        );
        push_unique_strings(
            &mut scope.action_refs,
            string_array_at(requirement, "actionRefs"),
        );
        push_unique_strings(
            &mut scope.interface_refs,
            string_array_at(requirement, "interfaceRefs"),
        );
    }
    if let Some(requirement) = task.frontend_experience_requirement.as_ref() {
        push_unique_strings(
            &mut scope.surface_refs,
            scope_refs_from_requirement(
                requirement,
                "/uiTaskScope/surfacesInScope",
                &["surfaceId"],
            ),
        );
        push_unique_strings(
            &mut scope.data_view_refs,
            scope_refs_from_requirement(requirement, "/uiTaskScope/dataViewsInScope", &["viewId"]),
        );
        push_unique_strings(
            &mut scope.action_refs,
            scope_refs_from_requirement(requirement, "/uiTaskScope/actionsInScope", &["actionId"]),
        );
        push_unique_strings(
            &mut scope.operation_path_refs,
            scope_refs_from_requirement(
                requirement,
                "/uiTaskScope/operationPathsInScope",
                &["pathId"],
            ),
        );
        push_unique_strings(
            &mut scope.state_refs,
            scope_refs_from_requirement(requirement, "/uiTaskScope/stateExpectation", &["state"]),
        );
        push_unique_strings(
            &mut scope.surface_region_refs,
            scope_refs_from_requirement(requirement, "/uiTaskScope/regionsInScope", &["regionId"]),
        );
        push_unique_strings(
            &mut scope.surface_action_refs,
            scope_refs_from_requirement(
                requirement,
                "/uiTaskScope/actionsInContract",
                &["actionId"],
            ),
        );
        push_unique_strings(
            &mut scope.state_refs,
            scope_refs_from_requirement(requirement, "/uiTaskScope/statesInContract", &["state"]),
        );
        push_unique_strings(
            &mut scope.quality_rule_refs,
            scope_refs_from_requirement(
                requirement,
                "/uiTaskScope/qualityRulesInScope",
                &["ruleId"],
            ),
        );
        push_unique_strings(
            &mut scope.ownership_dimensions,
            ownership_dimensions_from_requirement(requirement),
        );

        // A TaskResult submit reconstructs the task from the execution request. Preserve the
        // previously derived task-scoped contract when the original TaskPlan projection did not
        // carry explicit UI scope arrays.
        let brief = requirement
            .pointer("/executionGuidance/uiProductionBrief")
            .unwrap_or(&Value::Null);
        push_unique_strings(
            &mut scope.surface_refs,
            string_array_at(brief.get("appliesTo").unwrap_or(&Value::Null), "surfaceIds"),
        );
        push_unique_strings(
            &mut scope.data_view_refs,
            string_array_at(
                brief.get("appliesTo").unwrap_or(&Value::Null),
                "dataViewIds",
            ),
        );
        push_unique_strings(
            &mut scope.action_refs,
            string_array_at(brief.get("appliesTo").unwrap_or(&Value::Null), "actionIds"),
        );
        push_unique_strings(
            &mut scope.operation_path_refs,
            string_array_at(
                brief.get("appliesTo").unwrap_or(&Value::Null),
                "operationPathIds",
            ),
        );
        let brief_contract = brief.get("surfaceDecisionContract").unwrap_or(&Value::Null);
        push_unique_strings(
            &mut scope.surface_region_refs,
            scope_refs_from_requirement(brief_contract, "/regionsInScope", &["regionId"]),
        );
        push_unique_strings(
            &mut scope.surface_action_refs,
            scope_refs_from_requirement(brief_contract, "/actionsInScope", &["actionId"]),
        );
        push_unique_strings(
            &mut scope.state_refs,
            scope_refs_from_requirement(brief_contract, "/statesInScope", &["state"]),
        );
        push_unique_strings(
            &mut scope.quality_rule_refs,
            scope_refs_from_requirement(brief_contract, "/qualityRulesInScope", &["ruleId"]),
        );
        for region in array_at(brief_contract, "regionsInScope") {
            push_unique_strings(&mut scope.state_refs, string_array_at(region, "stateRefs"));
        }
    }
    for detail in &aac.detail_coverage {
        if !task
            .requirement_detail_refs
            .iter()
            .any(|detail_id| detail_id == &detail.detail_id)
        {
            continue;
        }
        push_unique_strings(
            &mut scope.data_view_refs,
            detail.artifact_refs.frontend_data_views.clone(),
        );
        push_unique_strings(
            &mut scope.action_refs,
            detail.artifact_refs.frontend_actions.clone(),
        );
        push_unique_strings(
            &mut scope.operation_path_refs,
            detail.artifact_refs.frontend_operation_paths.clone(),
        );
        push_unique_strings(
            &mut scope.workflow_refs,
            detail.artifact_refs.user_flows.clone(),
        );
        push_unique_strings(
            &mut scope.interface_refs,
            detail.artifact_refs.interfaces.clone(),
        );
    }
    for _ in 0..2 {
        enrich_scope_from_operation_paths(frontend, &mut scope);
        enrich_scope_from_surfaces(frontend, &mut scope);
    }
    if let Some(surface_contract) = frontend.get("uiSurfaceDecisionContract") {
        if scope.surface_region_refs.is_empty()
            && (!scope.surface_refs.is_empty()
                || !scope.data_view_refs.is_empty()
                || !scope.action_refs.is_empty()
                || !scope.operation_path_refs.is_empty())
        {
            for region in array_at(surface_contract, "regionModel") {
                let refs = string_array_at(region, "surfaceRefs");
                if refs.is_empty()
                    || refs.iter().any(|reference| {
                        scope.surface_refs.contains(reference)
                            || scope.data_view_refs.contains(reference)
                            || scope.action_refs.contains(reference)
                            || scope.operation_path_refs.contains(reference)
                    })
                {
                    push_unique(
                        &mut scope.surface_region_refs,
                        string_at(region, "regionId"),
                    );
                }
            }
        }
        if scope.quality_rule_refs.is_empty()
            && (!scope.surface_refs.is_empty()
                || !scope.data_view_refs.is_empty()
                || !scope.action_refs.is_empty()
                || !scope.operation_path_refs.is_empty())
        {
            for rule in array_at(surface_contract, "qualityRules") {
                let refs = string_array_at(rule, "scopeRefs");
                if refs.is_empty()
                    || refs.iter().any(|reference| {
                        scope.surface_refs.contains(reference)
                            || scope.data_view_refs.contains(reference)
                            || scope.action_refs.contains(reference)
                            || scope.operation_path_refs.contains(reference)
                    })
                {
                    push_unique(&mut scope.quality_rule_refs, string_at(rule, "ruleId"));
                }
            }
        }
    }
    if scope.state_refs.is_empty() && !scope.surface_refs.is_empty() {
        for surface in selected_frontend_surfaces(frontend, &scope.surface_refs) {
            push_unique_strings(
                &mut scope.state_refs,
                string_array_at(&surface, "stateRefs"),
            );
            if let Some(model) = surface
                .get("statePlacementModel")
                .and_then(Value::as_object)
            {
                push_unique_strings(&mut scope.state_refs, model.keys().cloned().collect());
            }
        }
    }
    if scope.state_refs.is_empty() && !scope.surface_region_refs.is_empty() {
        if let Some(surface_contract) = frontend.get("uiSurfaceDecisionContract") {
            for region in array_at(surface_contract, "regionModel") {
                if string_at(region, "regionId")
                    .is_some_and(|region_id| scope.surface_region_refs.contains(&region_id))
                {
                    push_unique_strings(
                        &mut scope.state_refs,
                        string_array_at(region, "stateRefs"),
                    );
                }
            }
        }
    }
    if !scope.surface_region_refs.is_empty() {
        if let Some(surface_contract) = frontend.get("uiSurfaceDecisionContract") {
            for state in array_at(surface_contract, "stateModel") {
                push_unique(&mut scope.state_refs, string_at(state, "state"));
            }
        }
    }
    scope.surface_refs = unique_strings(scope.surface_refs);
    scope.surface_region_refs = unique_strings(scope.surface_region_refs);
    scope.surface_action_refs = unique_strings(scope.surface_action_refs);
    scope.data_view_refs = unique_strings(scope.data_view_refs);
    scope.action_refs = unique_strings(scope.action_refs);
    scope.operation_path_refs = unique_strings(scope.operation_path_refs);
    scope.workflow_refs = unique_strings(scope.workflow_refs);
    scope.interface_refs = unique_strings(scope.interface_refs);
    scope.state_refs = unique_strings(scope.state_refs);
    scope.quality_rule_refs = unique_strings(scope.quality_rule_refs);
    if scope.ownership_dimensions.is_empty() && task_has_frontend_execution(task) {
        scope.ownership_dimensions = derived_execution_ownership_dimensions(&scope);
    }
    scope.ownership_dimensions = unique_strings(scope.ownership_dimensions);
    scope
}

fn ownership_dimensions_from_requirement(requirement: &Value) -> Vec<String> {
    requirement
        .pointer("/uiTaskScope/ownershipDimensions")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .filter(|dimension| UI_OWNERSHIP_DIMENSION_VALUES.contains(dimension))
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn derived_execution_ownership_dimensions(scope: &FrontendTaskScope) -> Vec<String> {
    let mut dimensions = Vec::new();
    if !scope.surface_refs.is_empty() || !scope.surface_region_refs.is_empty() {
        dimensions.push("surface".to_string());
        dimensions.push("layout".to_string());
    }
    if !scope.data_view_refs.is_empty() {
        dimensions.push("data_view".to_string());
    }
    if !scope.action_refs.is_empty()
        || !scope.surface_action_refs.is_empty()
        || !scope.operation_path_refs.is_empty()
    {
        dimensions.push("action".to_string());
        dimensions.push("integration_feedback".to_string());
    }
    if !scope.state_refs.is_empty() {
        dimensions.push("state".to_string());
    }
    dimensions.push("visual_system".to_string());
    dimensions.push("content_boundary".to_string());
    unique_strings(dimensions)
}

fn scope_refs_from_requirement(
    requirement: &Value,
    pointer: &str,
    id_keys: &[&str],
) -> Vec<String> {
    requirement
        .pointer(pointer)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    item.as_str().map(str::to_string).or_else(|| {
                        id_keys.iter().find_map(|key| {
                            item.get(*key).and_then(Value::as_str).map(str::to_string)
                        })
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn enrich_scope_from_operation_paths(frontend: &Value, scope: &mut FrontendTaskScope) {
    for path in array_at(frontend, "operationPaths") {
        let path_id = string_at(path, "pathId");
        let surface_ref = string_at(path, "surfaceRef");
        let workflow_ref = string_at(path, "workflowRef");
        let interface_refs = string_array_at(path, "interfaceRefs");
        let matches_scope = path_id
            .as_ref()
            .map(|id| scope.operation_path_refs.iter().any(|item| item == id))
            .unwrap_or(false)
            || surface_ref
                .as_ref()
                .map(|id| scope.surface_refs.iter().any(|item| item == id))
                .unwrap_or(false)
            || workflow_ref
                .as_ref()
                .map(|id| scope.workflow_refs.iter().any(|item| item == id))
                .unwrap_or(false)
            || interface_refs
                .iter()
                .any(|id| scope.interface_refs.iter().any(|item| item == id));
        if !matches_scope {
            continue;
        }
        push_unique(&mut scope.operation_path_refs, path_id);
        push_unique(&mut scope.surface_refs, surface_ref);
        push_unique(&mut scope.workflow_refs, workflow_ref);
        push_unique_strings(&mut scope.interface_refs, interface_refs);
        push_unique_strings(
            &mut scope.data_view_refs,
            string_array_at(path, "dataViewRefs"),
        );
        push_unique_strings(&mut scope.action_refs, string_array_at(path, "actionRefs"));
    }
}

fn enrich_scope_from_surfaces(frontend: &Value, scope: &mut FrontendTaskScope) {
    for surface in registry_surface_values(frontend)
        .into_iter()
        .chain(array_at(frontend, "surfaces"))
    {
        let surface_id = string_at(surface, "surfaceId");
        let workflow_refs = string_array_at(surface, "workflowRefs");
        let data_view_refs = string_array_at(surface, "dataViewRefs");
        let action_refs = string_array_at(surface, "actionRefs");
        let operation_path_refs = string_array_at(surface, "operationPathRefs");
        let interface_refs = string_array_at(surface, "interfaceRefs");
        let matches_scope = surface_id
            .as_ref()
            .map(|id| scope.surface_refs.iter().any(|item| item == id))
            .unwrap_or(false)
            || workflow_refs
                .iter()
                .any(|id| scope.workflow_refs.iter().any(|item| item == id))
            || data_view_refs
                .iter()
                .any(|id| scope.data_view_refs.iter().any(|item| item == id))
            || action_refs
                .iter()
                .any(|id| scope.action_refs.iter().any(|item| item == id))
            || operation_path_refs
                .iter()
                .any(|id| scope.operation_path_refs.iter().any(|item| item == id))
            || interface_refs
                .iter()
                .any(|id| scope.interface_refs.iter().any(|item| item == id));
        if !matches_scope {
            continue;
        }
        push_unique(&mut scope.surface_refs, surface_id);
        push_unique_strings(&mut scope.workflow_refs, workflow_refs);
        push_unique_strings(&mut scope.data_view_refs, data_view_refs);
        push_unique_strings(&mut scope.action_refs, action_refs);
        push_unique_strings(&mut scope.operation_path_refs, operation_path_refs);
        push_unique_strings(&mut scope.interface_refs, interface_refs);
        push_unique_strings(&mut scope.state_refs, string_array_at(surface, "stateRefs"));
    }
}

fn selected_frontend_surfaces(frontend: &Value, ids: &[String]) -> Vec<Value> {
    let registry = selected_from_values(registry_surface_values(frontend), "surfaceId", ids);
    if registry.is_empty() {
        selected_frontend_values(frontend, "surfaces", "surfaceId", ids)
    } else {
        registry
    }
}

fn selected_frontend_values(
    frontend: &Value,
    array_key: &str,
    id_key: &str,
    ids: &[String],
) -> Vec<Value> {
    selected_from_values(array_at(frontend, array_key), id_key, ids)
}

fn selected_from_values(values: Vec<&Value>, id_key: &str, ids: &[String]) -> Vec<Value> {
    if ids.is_empty() {
        return vec![];
    }
    values
        .into_iter()
        .filter(|value| {
            string_at(value, id_key)
                .map(|id| ids.iter().any(|item| item == &id))
                .unwrap_or(false)
        })
        .cloned()
        .collect()
}

fn registry_surface_values(frontend: &Value) -> Vec<&Value> {
    frontend
        .pointer("/uiSurfaceRegistry/surfaces")
        .and_then(Value::as_array)
        .map(|items| items.iter().collect())
        .unwrap_or_default()
}

fn frontend_backend_bindings_from_scope(
    aac: &ArchitectureArtifactContract,
    scope: &FrontendTaskScope,
) -> Vec<Value> {
    aac.interfaces
        .iter()
        .filter(|interface| {
            string_at(interface, "interfaceId")
                .map(|id| scope.interface_refs.iter().any(|item| item == &id))
                .unwrap_or(false)
        })
        .filter(|interface| is_executable_interface(interface))
        .map(|interface| {
            let interface_id = string_at(interface, "interfaceId").unwrap_or_default();
            json!({
                "bindingId": format!("ui-binding:{interface_id}"),
                "workflowRefs": scope.workflow_refs.clone(),
                "operationPathRefs": scope.operation_path_refs.clone(),
                "interfaces": [compact_interface_binding(interface)],
                "completionRule": "当任务拥有该交互时，将任务拥有的 UI 操作或界面连接到此 AAC 声明的接口。"
            })
        })
        .collect()
}

fn ui_production_brief(
    task: &TaskDefinition,
    frontend: &Value,
    scope: &FrontendTaskScope,
    user_facing_language: &Option<contracts::UserFacingLanguageConstraint>,
) -> Value {
    let surface_contract = frontend
        .get("uiSurfaceDecisionContract")
        .unwrap_or(&Value::Null);
    let surfaces = selected_frontend_surfaces(frontend, &scope.surface_refs);
    let data_views =
        selected_frontend_values(frontend, "dataViews", "viewId", &scope.data_view_refs);
    let ownership_dimensions = if scope.ownership_dimensions.is_empty() {
        derived_execution_ownership_dimensions(scope)
    } else {
        scope.ownership_dimensions.clone()
    };
    json!({
        "schemaVersion": "1.1",
        "briefKind": "task_ui_production_brief",
        "appliesTo": {
            "ownershipDimensions": ownership_dimensions,
            "surfaceIds": scope.surface_refs.clone(),
            "surfaceRoles": unique_strings(surfaces.iter().map(surface_role).collect()),
            "dataViewIds": scope.data_view_refs.clone(),
            "actionIds": scope.action_refs.clone(),
            "operationPathIds": scope.operation_path_refs.clone()
        },
        "productIntent": product_intent(task, &surfaces, &data_views),
        "surfaceDecisionContract": surface_decision_contract_projection(surface_contract, scope),
        "layoutContract": layout_contract(&surfaces, surface_contract),
        "informationContract": information_contract(surface_contract, &surfaces, &data_views),
        "actionContract": action_contract(surface_contract, scope, &surfaces),
        "stateContract": state_contract(surface_contract, &surfaces, scope),
        "visualContract": visual_contract(surface_contract, &surfaces),
        "contentBoundary": content_boundary(surface_contract, user_facing_language)
    })
}

fn product_intent(task: &TaskDefinition, surfaces: &[Value], data_views: &[Value]) -> Value {
    json!({
        "userRole": surface_model_string(surfaces, "/productIntent/userRole")
            .unwrap_or_else(|| "任务用户".to_string()),
        "businessObject": surface_model_string(surfaces, "/productIntent/businessObject")
            .unwrap_or_else(|| {
                compact_join(
                    unique_strings(data_views.iter().filter_map(value_display_name).collect()),
                    "任务拥有的业务对象",
                )
            }),
        "primaryJob": surface_model_string(surfaces, "/productIntent/primaryJob")
            .unwrap_or_else(|| task.objective.clone()),
        "successOutcome": surface_model_string(surfaces, "/productIntent/successOutcome")
            .unwrap_or_else(|| "用户可以完成任务拥有的工作流并看到更新后的业务状态。".to_string())
    })
}

fn surface_decision_contract_projection(contract: &Value, scope: &FrontendTaskScope) -> Value {
    if !contract.is_object() {
        return Value::Null;
    }
    let regions = selected_surface_contract_values(
        contract,
        "regionModel",
        "regionId",
        &scope.surface_region_refs,
    );
    let actions = selected_surface_contract_values(
        contract,
        "actionModel",
        "actionId",
        &scope.surface_action_refs,
    );
    let states =
        selected_surface_contract_values(contract, "stateModel", "state", &scope.state_refs);
    let rules = selected_surface_contract_values(
        contract,
        "qualityRules",
        "ruleId",
        &scope.quality_rule_refs,
    );
    json!({
        "contractRef": "sourceRefs.architectureArtifactContractRef#/frontendExperience/uiSurfaceDecisionContract",
        "selectionMode": "task_scope",
        "patternDecision": contract.get("patternDecision").cloned().unwrap_or(Value::Null),
        "semanticFacts": contract.get("semanticFacts").cloned().unwrap_or(Value::Null),
        "layoutModel": contract.get("layoutModel").cloned().unwrap_or(Value::Null),
        "regionsInScope": regions,
        "informationModel": contract.get("informationModel").cloned().unwrap_or(Value::Null),
        "actionsInScope": actions,
        "statesInScope": states,
        "compositionConstraints": contract.get("compositionConstraints").cloned().unwrap_or(Value::Null),
        "contentBoundary": contract.get("contentBoundary").cloned().unwrap_or(Value::Null),
        "qualityRulesInScope": rules
    })
}

fn selected_surface_contract_values(
    contract: &Value,
    array_key: &str,
    id_key: &str,
    ids: &[String],
) -> Vec<Value> {
    if ids.is_empty() {
        return Vec::new();
    }
    let values = array_at(contract, array_key);
    selected_from_values(values, id_key, ids)
}

fn layout_contract(surfaces: &[Value], surface_contract: &Value) -> Value {
    let layout_model = surface_contract.get("layoutModel").unwrap_or(&Value::Null);
    let composition = surface_contract
        .get("compositionConstraints")
        .unwrap_or(&Value::Null);
    let layout_baseline = string_at(layout_model, "layoutBaseline")
        .or_else(|| surface_model_string(surfaces, "/visualModel/layoutBaseline"))
        .unwrap_or_else(|| "custom_product_layout".to_string());
    let density = surface_model_string(surfaces, "/visualModel/density")
        .or_else(|| string_at(layout_model, "density"))
        .unwrap_or_else(|| "balanced".to_string());
    let required_regions = non_empty_or(
        unique_strings(
            string_array_at(composition, "requiredComposition")
                .into_iter()
                .chain(
                    surface_model_array(surfaces, "/compositionModel/requiredRegions")
                        .unwrap_or_default(),
                )
                .collect(),
        ),
        default_required_composition,
    );
    let forbidden_regions = non_empty_or(
        unique_strings(
            string_array_at(composition, "forbiddenComposition")
                .into_iter()
                .chain(
                    surface_model_array(surfaces, "/compositionModel/forbiddenRegions")
                        .unwrap_or_default(),
                )
                .collect(),
        ),
        default_forbidden_composition,
    );
    json!({
        "layoutBaseline": layout_baseline,
        "density": density,
        "requiredRegions": required_regions,
        "forbiddenRegions": forbidden_regions,
        "responsiveBehavior": surface_responsive_behavior(layout_model, surfaces),
        "primaryRegion": string_at(layout_model, "primaryWorkRegionId")
            .or_else(|| surface_model_string(surfaces, "/compositionModel/primaryRegion"))
            .unwrap_or_else(|| "任务相关的数据、表单、详情或操作区域".to_string()),
        "supportingRegions": surface_model_array(surfaces, "/compositionModel/supportingRegions")
            .unwrap_or_else(|| vec!["导航/上下文".to_string(), "反馈".to_string()])
    })
}

fn information_contract(
    surface_contract: &Value,
    surfaces: &[Value],
    data_views: &[Value],
) -> Value {
    let information_model = surface_contract
        .get("informationModel")
        .unwrap_or(&Value::Null);
    let data_view_names =
        unique_strings(data_views.iter().filter_map(value_display_name).collect());
    json!({
        "primaryObjects": string_array_at(information_model, "primaryObjects"),
        "mustShow": string_array_at(information_model, "fields")
            .into_iter()
            .chain(surface_model_array(surfaces, "/informationModel/mustShow").unwrap_or_default())
            .collect::<Vec<_>>(),
        "scanPriority": string_array_at(information_model, "scanOrder")
            .into_iter()
            .chain(surface_model_array(surfaces, "/informationModel/scanPriority").unwrap_or_default())
            .collect::<Vec<_>>(),
        "identityFields": string_array_at(information_model, "identityFields")
            .into_iter()
            .chain(surface_model_array(surfaces, "/informationModel/identityFields").unwrap_or_default())
            .collect::<Vec<_>>(),
        "statusFields": string_array_at(information_model, "statusFields")
            .into_iter()
            .chain(surface_model_array(surfaces, "/informationModel/statusFields").unwrap_or_default())
            .collect::<Vec<_>>(),
        "longContentPolicy": string_at(information_model, "longContentPolicy")
            .or_else(|| surface_model_string(surfaces, "/informationModel/longContentPolicy"))
            .unwrap_or_else(|| "长标签、备注和标识符必须换行、截断并提供访问完整值的途径，或移入详情视图，不得破坏布局。".to_string()),
        "dataViews": data_view_names
    })
}

fn action_contract(
    surface_contract: &Value,
    scope: &FrontendTaskScope,
    surfaces: &[Value],
) -> Value {
    let contract_actions = selected_surface_contract_values(
        surface_contract,
        "actionModel",
        "actionId",
        &scope.surface_action_refs,
    );
    json!({
        "actionsInScope": contract_actions,
        "primaryActions": unique_strings(
            selected_surface_contract_values(surface_contract, "actionModel", "actionId", &scope.surface_action_refs)
                .iter()
                .filter_map(|action| string_at(action, "label"))
                .chain(surface_model_array(surfaces, "/actionModel/primaryActions").unwrap_or_default())
                .collect()
        ),
        "contextualActions": surface_model_array(surfaces, "/actionModel/contextualActions")
            .unwrap_or_default(),
        "dangerousActions": surface_model_array(surfaces, "/actionModel/dangerousActions")
            .unwrap_or_default(),
        "placementRule": surface_model_string(surfaces, "/actionModel/placementRule")
            .unwrap_or_else(|| "将操作放置在用户做出决策的位置，保持受影响的对象标识可见。".to_string()),
        "postSuccessUpdate": selected_surface_contract_values(surface_contract, "actionModel", "actionId", &scope.surface_action_refs)
            .iter()
            .find_map(|action| string_at(action, "postSuccessUpdate"))
            .or_else(|| surface_model_string(surfaces, "/actionModel/postSuccessUpdate"))
            .unwrap_or_else(|| "更新受影响的行、详情、计数、状态或路由；不要仅依赖提示。".to_string())
    })
}

fn state_contract(
    surface_contract: &Value,
    surfaces: &[Value],
    scope: &FrontendTaskScope,
) -> Value {
    let state_refs = scope.state_refs.clone();
    let states_in_scope =
        selected_surface_contract_values(surface_contract, "stateModel", "state", &state_refs);
    json!({
        "statesInScope": state_refs,
        "stateModelsInScope": states_in_scope,
        "loading": state_rule(surfaces, "loading", "在等待数据或变更的区域或控件附近。"),
        "empty": state_rule(surfaces, "empty", "在数据/表单区域中，适用时显示过滤器和业务下一步操作。"),
        "error": state_rule(surfaces, "error", "在受影响区域附近，显示恢复路径，不显示堆栈跟踪。"),
        "success": state_rule(surfaces, "success", "内联对象更新加上有用的简短确认。"),
        "business_blocking": state_rule(surfaces, "business_blocking", "在受阻字段、行、详情或操作附近，使用产品语言说明原因。"),
        "validation": state_rule(surfaces, "validation", "在字段附近，较长表单时在摘要中显示。"),
        "disabled": state_rule(surfaces, "disabled", "在或靠近禁用控件处，可操作时显示解锁原因。")
    })
}

fn visual_contract(surface_contract: &Value, surfaces: &[Value]) -> Value {
    let composition = surface_contract
        .get("compositionConstraints")
        .unwrap_or(&Value::Null);
    let density = surface_model_string(surfaces, "/visualModel/density")
        .or_else(|| {
            surface_contract
                .pointer("/layoutModel/density")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .unwrap_or_else(|| "balanced".to_string());
    json!({
        "tokenPolicy": surface_model_string(surfaces, "/visualModel/tokenPolicy")
            .unwrap_or_else(|| "在页面局部样式之前使用已有或计划中的语义令牌；不要在已有令牌系统旁创建第二个令牌系统。".to_string()),
        "componentPolicy": surface_model_string(surfaces, "/visualModel/componentPolicy")
            .unwrap_or_else(|| "为数据、表单、详情、操作、反馈和导航使用任务适配的组件，而非装饰性能力卡片。".to_string()),
        "densityRule": format!("一致地使用 {density} 密度进行间距、行高、控件尺寸和信息分组。"),
        "antiDemoRules": string_array_at(composition, "antiDemoRules")
            .into_iter()
            .chain(surface_model_array(surfaces, "/visualModel/antiDemoRules").unwrap_or_default())
            .collect::<Vec<_>>()
    })
}

fn content_boundary(
    surface_contract: &Value,
    user_facing_language: &Option<contracts::UserFacingLanguageConstraint>,
) -> Value {
    let surface_boundary = surface_contract
        .get("contentBoundary")
        .unwrap_or(&Value::Null);
    json!({
        "userFacingLanguage": user_facing_language
            .as_ref()
            .map(|constraint| constraint.rule.clone())
            .unwrap_or_else(|| "使用项目已确认的用户面向语言和产品词汇。".to_string()),
        "allowedUserVisibleContent": surface_boundary
            .get("allowedUserVisibleContent")
            .cloned()
            .unwrap_or_else(|| json!([])),
        "forbiddenUserVisibleContent": surface_boundary
            .get("forbiddenUserVisibleContent")
            .cloned()
            .unwrap_or_else(|| json!([])),
        "copyRule": surface_boundary
            .get("copyRule")
            .and_then(Value::as_str)
            .unwrap_or("为用户的业务任务编写产品文案。不要暴露运行时命令、技术栈说明、交付进度、验证指令、内部工作流术语、生成的制品 ID 或验证器语言，除非产品本身是开发者/运行时工具。")
    })
}

fn surface_model_string(surfaces: &[Value], pointer: &str) -> Option<String> {
    surfaces
        .iter()
        .find_map(|surface| surface.pointer(pointer).and_then(Value::as_str))
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
}

fn surface_model_array(surfaces: &[Value], pointer: &str) -> Option<Vec<String>> {
    let values = unique_strings(
        surfaces
            .iter()
            .flat_map(|surface| {
                surface
                    .pointer(pointer)
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
            })
            .collect(),
    );
    if values.is_empty() {
        None
    } else {
        Some(values)
    }
}

fn responsive_behavior(surfaces: &[Value]) -> String {
    let mut parts = Vec::new();
    if let Some(value) = surface_model_string(surfaces, "/responsiveModel/desktop") {
        parts.push(format!("desktop: {value}"));
    }
    if let Some(value) = surface_model_string(surfaces, "/responsiveModel/tablet") {
        parts.push(format!("tablet: {value}"));
    }
    if let Some(value) = surface_model_string(surfaces, "/responsiveModel/mobile") {
        parts.push(format!("mobile: {value}"));
    }
    if parts.is_empty() {
        "在所需视口中保持任务顺序、对象标识、主要操作和范围反馈可用；仅在保持任务的前提下使用卡片、下钻、堆叠或横向溢出。".to_string()
    } else {
        parts.join("; ")
    }
}

fn surface_responsive_behavior(layout_model: &Value, surfaces: &[Value]) -> String {
    let mut parts = Vec::new();
    for posture in ["desktop", "tablet", "mobile"] {
        if let Some(intent) = layout_model
            .pointer(&format!("/{posture}/layoutIntent"))
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
        {
            parts.push(format!("{posture}: {intent}"));
        }
    }
    if parts.is_empty() {
        responsive_behavior(surfaces)
    } else {
        parts.join("; ")
    }
}

fn state_rule(surfaces: &[Value], state: &str, fallback: &str) -> String {
    surface_model_string(surfaces, &format!("/statePlacementModel/{state}"))
        .unwrap_or_else(|| fallback.to_string())
}

fn style_asset_plan(frontend: &Value) -> Value {
    let surface_contract = frontend
        .get("uiSurfaceDecisionContract")
        .unwrap_or(&Value::Null);
    json!({
        "designTokenAssetPlan": surface_contract.get("designTokenAssetPlan").cloned().unwrap_or(Value::Null),
        "semanticTokenPolicy": surface_contract.get("semanticTokenPolicy").cloned().unwrap_or(Value::Null),
        "referencePlan": surface_contract.get("referencePlan").cloned().unwrap_or_else(|| json!([])),
        "implementationRule": "当此任务更改用户可见前端代码时，仅加载 referencePlan 中列出的 UIX 文件。将 designTokenAssetPlan 作为此任务的单个令牌资产权威。"
    })
}

fn surface_role(surface: &Value) -> String {
    string_at(surface, "surfaceRole")
        .or_else(|| string_at(surface, "role"))
        .unwrap_or_else(|| "page".to_string())
}

fn value_display_name(value: &Value) -> Option<String> {
    string_at(value, "label")
        .or_else(|| string_at(value, "name"))
        .or_else(|| string_at(value, "title"))
}

fn default_required_composition() -> Vec<String> {
    vec![
        "业务导航或局部上下文".to_string(),
        "任务相关的数据视图、表单、表格、详情或操作区域".to_string(),
        "适用时局部加载、空、错误、成功和业务阻塞状态".to_string(),
    ]
}

fn default_forbidden_composition() -> Vec<String> {
    vec![
        "与任务拥有的业务工作流无关的界面组合".to_string(),
        "取代所需数据、操作、状态或反馈的装饰性或解释性区域".to_string(),
    ]
}

fn compact_join(values: Vec<String>, fallback: &str) -> String {
    if values.is_empty() {
        fallback.to_string()
    } else {
        values.join("; ")
    }
}

fn push_unique(values: &mut Vec<String>, value: Option<String>) {
    if let Some(value) = value {
        if !value.trim().is_empty() && !values.iter().any(|item| item == &value) {
            values.push(value);
        }
    }
}

fn push_unique_strings(values: &mut Vec<String>, next: Vec<String>) {
    for value in next {
        push_unique(values, Some(value));
    }
}

fn workflow_closure_requirements_for_task(
    task: &TaskDefinition,
    aac: &ArchitectureArtifactContract,
) -> Vec<Value> {
    crate::task_plan::workflow_closure_requirements(aac)
        .into_iter()
        .filter(|requirement| task_matches_workflow_closure(task, requirement))
        .collect()
}

fn task_matches_workflow_closure(task: &TaskDefinition, requirement: &Value) -> bool {
    let refs = &task.write_boundary.artifact_refs;
    let workflow_ref = string_at(requirement, "workflowRef");
    let workflow_matches = workflow_ref
        .as_ref()
        .map(|workflow_ref| refs.user_flows.iter().any(|item| item == workflow_ref))
        .unwrap_or(false);
    let interface_refs = string_array_at(requirement, "interfaceRefs");
    let task_interfaces = refs.all_interfaces();
    let interface_matches = !interface_refs.is_empty()
        && interface_refs
            .iter()
            .any(|interface_ref| task_interfaces.iter().any(|item| item == interface_ref));
    let acceptance_refs = string_array_at(requirement, "acceptanceRefs");
    let acceptance_matches = acceptance_refs.is_empty()
        || acceptance_refs.iter().any(|acceptance_ref| {
            task.acceptance_refs
                .iter()
                .any(|item| item == acceptance_ref)
        });
    task.frontend_experience_requirement.is_some()
        && acceptance_matches
        && (workflow_matches || interface_matches)
}

fn workflow_closure_requirement_execution_view(requirements: &[Value]) -> Vec<Value> {
    requirements
        .iter()
        .map(|requirement| {
            json!({
                "closureId": string_at(requirement, "closureId").unwrap_or_default(),
                "workflowRef": string_at(requirement, "workflowRef").unwrap_or_default(),
                "workflowName": string_at(requirement, "workflowName").unwrap_or_default(),
                "surfaceRefs": string_array_at(requirement, "surfaceRefs"),
                "operationPathRefs": string_array_at(requirement, "operationPathRefs"),
                "dataViewRefs": string_array_at(requirement, "dataViewRefs"),
                "actionRefs": string_array_at(requirement, "actionRefs"),
                "acceptanceRefs": string_array_at(requirement, "acceptanceRefs"),
                "interfaceRefs": string_array_at(requirement, "interfaceRefs"),
                "stateMachineRefs": string_array_at(requirement, "stateMachineRefs"),
                "requiredDataBindingMode": "wired",
                "requiredEvidence": requirement.get("requiredEvidence").cloned().unwrap_or(Value::Array(vec![])),
                "evidenceRule": "证据必须覆盖用户操作、声明的接口调用、状态或持久化变更以及成功或阻塞反馈。"
            })
        })
        .collect()
}

fn frontend_backend_bindings(requirements: &[Value]) -> Vec<Value> {
    requirements
        .iter()
        .flat_map(|requirement| {
            let workflow_ref = string_at(requirement, "workflowRef").unwrap_or_default();
            let workflow_name = string_at(requirement, "workflowName").unwrap_or_default();
            let step_ref = string_array_at(requirement, "stepRefs")
                .into_iter()
                .next()
                .unwrap_or_default();
            array_at(requirement, "interfaces")
                .into_iter()
                .map(move |interface| {
                    json!({
                        "bindingId": format!("{workflow_ref}:{step_ref}"),
                        "workflowRef": workflow_ref,
                        "workflowName": workflow_name,
                        "stepRef": step_ref,
                        "interfaces": [interface.clone()],
                        "completionRule": "将用户操作连接到此 AAC 声明的接口并验证回读或反馈。"
                    })
                })
        })
        .collect()
}

fn compact_interface_binding(interface: &Value) -> Value {
    json!({
        "interfaceId": string_at(interface, "interfaceId").unwrap_or_default(),
        "name": string_at(interface, "name").unwrap_or_default(),
        "type": string_at(interface, "type").unwrap_or_default(),
        "role": string_at(interface, "role"),
        "method": string_at(interface, "method"),
        "path": string_at(interface, "path"),
        "requestSchema": interface.get("requestSchema").cloned().unwrap_or(Value::Array(vec![])),
        "responseSchema": interface.get("responseSchema").cloned().unwrap_or(Value::Array(vec![])),
        "errorSchema": interface.get("errorSchema").cloned().unwrap_or(Value::Array(vec![]))
    })
}

fn selected_values(
    values: &[Value],
    id_key: &str,
    explicit_refs: &[String],
    task: &TaskDefinition,
    include_scope_acceptance_match: bool,
) -> Vec<Value> {
    values
        .iter()
        .filter(|value| {
            string_at(value, id_key)
                .map(|id| explicit_refs.iter().any(|item| item == &id))
                .unwrap_or(false)
                || (include_scope_acceptance_match && artifact_matches_task_scope(value, task))
        })
        .cloned()
        .collect()
}

fn selected_entities(
    data_model: &Value,
    explicit_refs: &[String],
    task: &TaskDefinition,
) -> Vec<Value> {
    data_model
        .pointer("/entities")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|entity| {
            string_at(entity, "entityId")
                .map(|id| explicit_refs.iter().any(|item| item == &id))
                .unwrap_or(false)
                || artifact_matches_task_scope(entity, task)
        })
        .cloned()
        .collect()
}

fn artifact_matches_task_scope(value: &Value, task: &TaskDefinition) -> bool {
    let scope_match = string_array_at(value, "scopeRefs")
        .iter()
        .any(|scope_ref| task.scope_refs.iter().any(|item| item == scope_ref));
    let acceptance_match = string_array_at(value, "acceptanceRefs")
        .iter()
        .any(|acceptance_ref| {
            task.acceptance_refs
                .iter()
                .any(|item| item == acceptance_ref)
        });
    scope_match || acceptance_match
}

fn array_at<'a>(value: &'a Value, key: &str) -> Vec<&'a Value> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(|items| items.iter().collect())
        .unwrap_or_default()
}

fn string_at(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).map(str::to_string)
}

fn string_array_at(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn unique_strings(values: Vec<String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    values
        .into_iter()
        .filter(|value| !value.trim().is_empty())
        .filter(|value| seen.insert(value.clone()))
        .collect()
}

fn non_empty_or<F>(values: Vec<String>, fallback: F) -> Vec<String>
where
    F: FnOnce() -> Vec<String>,
{
    if values.is_empty() {
        fallback()
    } else {
        values
    }
}

fn is_executable_interface(interface: &Value) -> bool {
    matches!(
        string_at(interface, "type").as_deref(),
        Some("http_api" | "service_method" | "cli_command" | "event" | "job" | "external_adapter")
    )
}

fn running_or_ready_task_id(run: &TaskPlanRun) -> Option<String> {
    if let Some(running) = run
        .task_states
        .iter()
        .find(|state| state.status == TaskRunStatus::Running)
    {
        return Some(running.task_id.clone());
    }
    let completed = run
        .task_states
        .iter()
        .filter(|state| {
            matches!(
                state.status,
                TaskRunStatus::Completed | TaskRunStatus::CompletedWithNotes
            )
        })
        .map(|state| state.task_id.clone())
        .collect::<BTreeSet<_>>();
    run.task_states
        .iter()
        .find(|state| {
            state.status == TaskRunStatus::Pending
                && state.depends_on.iter().all(|dep| completed.contains(dep))
        })
        .map(|state| state.task_id.clone())
}

fn dependency_results(run: &TaskPlanRun, task: &TaskDefinition) -> Vec<Value> {
    run.task_states
        .iter()
        .filter(|state| task.depends_on.iter().any(|dep| dep == &state.task_id))
        .map(|state| {
            json!({
                "taskId": state.task_id,
                "status": state.status,
                "resultId": state.result_id
            })
        })
        .collect()
}

pub(crate) fn load_current_plan_and_run(
    project_root: &Path,
    locator: &DeliveryPhaseLocator,
) -> Result<(TaskPlan, TaskPlanRun), state::store::StateError> {
    let latest_plan: Value =
        state::store::read_json(&task_plan_latest_file(project_root, locator))?;
    let plan_ref = latest_plan
        .get("taskPlanRef")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            state::store::StateError::StateCorrupted(
                "taskplans/latest.json missing taskPlanRef".to_string(),
            )
        })?;
    let latest_run: Value =
        state::store::read_json(&task_plan_run_latest_file(project_root, locator))?;
    let run_ref = latest_run
        .get("runRef")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            state::store::StateError::StateCorrupted("runs/latest.json missing runRef".to_string())
        })?;
    Ok((
        read_project_json(project_root, plan_ref)?,
        read_project_json(project_root, run_ref)?,
    ))
}

pub(crate) fn save_run(
    project_root: &Path,
    locator: &DeliveryPhaseLocator,
    run: &TaskPlanRun,
) -> Result<(), state::store::StateError> {
    state::store::write_json_atomic(&task_plan_run_file(project_root, locator, &run.run_id), run)?;
    state::store::write_json_atomic(
        &task_plan_run_latest_file(project_root, locator),
        &json!({
            "schemaVersion": "1.0",
            "taskPlanRunId": run.run_id,
            "runRef": to_project_relative(project_root, &task_plan_run_file(project_root, locator, &run.run_id))?,
            "taskPlanId": run.task_plan_id,
            "updatedAt": run.updated_at
        }),
    )
}

fn update_route_for_execution(
    project_root: &str,
    delivery_id: &str,
    phase_id: &str,
    request_ref: &str,
    result_file: &str,
    task: &TaskDefinition,
    run: &TaskPlanRun,
) -> Result<(), state::store::StateError> {
    let store = FileTransitionStore;
    let mut status = store.load_status(project_root).map_err(to_state_error)?;
    let mut delivery = store
        .load_delivery_index(project_root, delivery_id)
        .map_err(to_state_error)?;
    if let Some(phase) = delivery
        .phases
        .iter_mut()
        .find(|phase| phase.phase_id == phase_id)
    {
        phase.latest_refs.insert(
            "taskExecutionRequestRef".to_string(),
            request_ref.to_string(),
        );
        phase.next_action = Some(RouteAction {
            kind: RouteActionKind::ContinueExecution,
            source: "task_execution_request".to_string(),
            reason: "task_execution_request_created".to_string(),
            prompt: None,
            accepted_responses: vec![],
            request_ref: Some(request_ref.to_string()),
            details: Some(json!({
                "taskId": task.task_id,
                "groupId": task.group_id,
                "taskPlanRunId": run.run_id,
                "resultFile": result_file
            })),
            target_phase_id: None,
        });
    }
    delivery.status = DeliveryLifecycleStatus::Executing;
    delivery.updated_at = state::store::now_string();
    store
        .save_delivery_index(project_root, &delivery)
        .map_err(to_state_error)?;
    apply_delivery_index(&mut status, &delivery);
    store
        .save_status(project_root, &status)
        .map_err(to_state_error)?;
    Ok(())
}

fn update_route_for_browser_runtime_prepare(
    project_root: &str,
    locator: &DeliveryPhaseLocator,
    request_ref: &str,
    task: &TaskDefinition,
) -> Result<(), state::store::StateError> {
    let store = FileTransitionStore;
    let mut status = store.load_status(project_root).map_err(to_state_error)?;
    let mut delivery = store
        .load_delivery_index(project_root, &locator.delivery_id)
        .map_err(to_state_error)?;
    if let Some(phase) = delivery
        .phases
        .iter_mut()
        .find(|phase| phase.phase_id == locator.phase_id)
    {
        phase.latest_refs.insert(
            "browserRuntimePrepareRequestRef".to_string(),
            request_ref.to_string(),
        );
        phase.next_action = Some(RouteAction {
            kind: RouteActionKind::ContinueExecution,
            source: "browser_runtime_prepare_request".to_string(),
            reason: "browser_runtime_prepare_required".to_string(),
            prompt: None,
            accepted_responses: vec![],
            request_ref: Some(request_ref.to_string()),
            details: Some(json!({
                "taskId": task.task_id,
                "groupId": task.group_id
            })),
            target_phase_id: None,
        });
    }
    delivery.status = DeliveryLifecycleStatus::Executing;
    delivery.updated_at = state::store::now_string();
    store
        .save_delivery_index(project_root, &delivery)
        .map_err(to_state_error)?;
    apply_delivery_index(&mut status, &delivery);
    store
        .save_status(project_root, &status)
        .map_err(to_state_error)
}

fn update_route_for_review(
    project_root: &str,
    delivery_id: &str,
    phase_id: &str,
) -> Result<(), state::store::StateError> {
    let store = FileTransitionStore;
    let mut status = store.load_status(project_root).map_err(to_state_error)?;
    let mut delivery = store
        .load_delivery_index(project_root, delivery_id)
        .map_err(to_state_error)?;
    if let Some(phase) = delivery
        .phases
        .iter_mut()
        .find(|phase| phase.phase_id == phase_id)
    {
        phase.next_action = Some(RouteAction {
            kind: RouteActionKind::Review,
            source: "task_plan_run".to_string(),
            reason: "taskplan_run_ready_for_review".to_string(),
            prompt: None,
            accepted_responses: vec![],
            request_ref: None,
            details: None,
            target_phase_id: None,
        });
    }
    delivery.status = DeliveryLifecycleStatus::Reviewing;
    delivery.updated_at = state::store::now_string();
    store
        .save_delivery_index(project_root, &delivery)
        .map_err(to_state_error)?;
    apply_delivery_index(&mut status, &delivery);
    store
        .save_status(project_root, &status)
        .map_err(to_state_error)?;
    Ok(())
}

fn safe_id(value: &str) -> String {
    value
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect()
}

fn read_project_json<T: serde::de::DeserializeOwned>(
    project_root: &Path,
    relative: &str,
) -> Result<T, state::store::StateError> {
    let path = from_project_relative(project_root, relative)?;
    state::store::read_json(&path)
}

fn to_state_error(error: delivery_core::LoomCoreError) -> state::store::StateError {
    state::store::StateError::StateCorrupted(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn code_quality_task() -> TaskDefinition {
        TaskDefinition {
            task_id: "task-observability".to_string(),
            group_id: "group-backend".to_string(),
            title: "Implement application observability".to_string(),
            task_kind: TaskKind::ConfigurationSupport,
            implementation_actions: vec![ImplementationAction::ImplementObservability],
            implementation_obligations: vec![],
            objective: "Configure logging and implement owned diagnostic boundaries.".to_string(),
            depends_on: vec![],
            scope_refs: vec![],
            acceptance_refs: vec![],
            requirement_detail_refs: vec![],
            write_boundary: contracts::TaskWriteBoundary {
                forbidden_paths: vec![".loom".to_string()],
                artifact_refs: contracts::TaskArtifactRefs::default(),
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
            code_quality_requirement_refs: vec!["code-quality-task-observability".to_string()],
        }
    }

    #[test]
    fn task_execute_uses_task_scope_as_the_implementation_contract() {
        let task = code_quality_task();
        let execution_rules = code_quality_execution_rules(&task);
        let result_rules = task_result_rules(&task, false);

        assert!(execution_rules["implementationRules"]
            .as_array()
            .unwrap()
            .iter()
            .any(|rule| rule
                .as_str()
                .is_some_and(|text| text.contains("task.implementationActions"))));
        assert!(result_rules.as_array().unwrap().iter().any(|rule| rule
            .as_str()
            .is_some_and(|text| text.contains("task.implementationActions"))));
    }

    #[test]
    fn task_execute_restricts_mybatis_plus_guidance_to_selected_reference_plan() {
        let rules = code_quality_execution_rules(&code_quality_task());

        assert!(rules["implementationRules"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item
                .as_str()
                .is_some_and(|text| text.contains("mybatis-plus"))));
    }

    #[test]
    fn task_execute_exposes_mcp_derived_responsibility_boundary() {
        let task = code_quality_task();
        let rules = task_execution_rules(".loom/result.json", &task, None);
        let boundary = &rules["taskResponsibilityBoundary"];

        assert!(boundary["ownedResponsibilities"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str() == Some("implement_observability")));
        assert!(boundary["rules"].as_array().unwrap().iter().any(|item| item
            .as_str()
            .is_some_and(|text| text.contains("不拥有持久化"))));
    }

    #[test]
    fn task_execution_request_requires_authoritative_fields_in_read_plan() {
        let request = json!({
            "task": {
                "objective": "Implement the task",
                "implementationActions": ["create_or_update_interface"],
                "writeBoundary": {"forbiddenPaths": []},
                "verificationIntents": [{"verificationId": "verify-1"}]
            },
            "sourceContext": {
                "architectureArtifactProjection": {
                    "modules": [{"moduleId": "module-1"}]
                }
            },
            "requestReadPlan": {"groups": [
                {"selectors": read_selectors_value_from_paths([
                    "task.objective",
                    "task.implementationActions",
                    "task.writeBoundary.forbiddenPaths",
                    "task.verificationIntents"
                ])},
                {"selectors": read_selectors_value_from_paths([
                    "sourceContext.architectureArtifactProjection.modules"
                ])}
            ]}
        });

        assert!(validate_task_execution_request_coverage(&request).is_ok());
    }

    #[test]
    fn task_execution_request_rejects_read_plan_that_drops_implementation_actions() {
        let request = json!({
            "task": {
                "objective": "Implement the task",
                "implementationActions": ["create_or_update_interface"],
                "writeBoundary": {"forbiddenPaths": []},
                "verificationIntents": []
            },
            "sourceContext": {"architectureArtifactProjection": {}},
            "requestReadPlan": {"groups": [{"selectors": read_selectors_value_from_paths([
                "task.objective",
                "task.writeBoundary.forbiddenPaths"
            ])}]}
        });

        let error = validate_task_execution_request_coverage(&request).unwrap_err();
        assert!(error.to_string().contains("task.implementationActions"));
    }

    #[test]
    fn architecture_projection_links_structured_happy_path_refs() {
        let user_flows = vec![json!({
            "flowId": "flow.submit-order",
            "happyPath": [
                {
                    "stepId": "step.submit",
                    "interactionRef": "api.orders.create",
                    "stateMachineRefs": ["machine.order"]
                },
                {
                    "stepId": "step.notify",
                    "interactionRef": "event.order-submitted",
                    "stateMachineRefs": []
                }
            ]
        })];

        let (interaction_refs, state_machine_refs) = behavior_refs_from_user_flows(&user_flows);

        assert_eq!(
            interaction_refs,
            vec!["api.orders.create", "event.order-submitted"]
        );
        assert_eq!(state_machine_refs, vec!["machine.order"]);
    }

    #[test]
    fn ui_production_brief_surface_projection_is_task_scoped() {
        let contract = json!({
            "patternDecision": {
                "mode": "known",
                "knownPattern": "collection_workbench"
            },
            "semanticFacts": {
                "userJobs": ["browse"]
            },
            "layoutModel": {
                "density": "workbench_dense"
            },
            "regionModel": [
                { "regionId": "region_primary", "purpose": "primary work" },
                { "regionId": "region_secondary", "purpose": "secondary work" }
            ],
            "informationModel": {
                "primaryObjects": ["request"]
            },
            "actionModel": [
                { "actionId": "action_create", "label": "Create" },
                { "actionId": "action_archive", "label": "Archive" }
            ],
            "stateModel": [
                { "state": "loading" },
                { "state": "error" }
            ],
            "compositionConstraints": {
                "antiDemoRules": ["no_internal_process_copy"]
            },
            "contentBoundary": {
                "forbiddenUserVisibleContent": ["runtime_commands"]
            },
            "qualityRules": [
                { "ruleId": "rule_primary", "expectation": "primary" },
                { "ruleId": "rule_secondary", "expectation": "secondary" }
            ]
        });
        let scope = FrontendTaskScope {
            surface_region_refs: vec!["region_primary".to_string()],
            surface_action_refs: vec!["action_create".to_string()],
            state_refs: vec!["loading".to_string()],
            quality_rule_refs: vec!["rule_primary".to_string()],
            ..FrontendTaskScope::default()
        };

        let projection = surface_decision_contract_projection(&contract, &scope);

        assert_eq!(
            projection.get("selectionMode").and_then(Value::as_str),
            Some("task_scope")
        );
        assert_eq!(
            projection
                .pointer("/regionsInScope/0/regionId")
                .and_then(Value::as_str),
            Some("region_primary")
        );
        assert_eq!(
            projection
                .pointer("/actionsInScope/0/actionId")
                .and_then(Value::as_str),
            Some("action_create")
        );
        assert_eq!(
            projection
                .pointer("/qualityRulesInScope/0/ruleId")
                .and_then(Value::as_str),
            Some("rule_primary")
        );
        assert_eq!(
            projection
                .get("regionsInScope")
                .and_then(Value::as_array)
                .map(Vec::len),
            Some(1),
            "projection must not copy unrelated regions when task scope is explicit"
        );
    }
}
