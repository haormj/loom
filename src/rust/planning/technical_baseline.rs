use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use contracts::{
    BrainstormContract, ClientTrustModel, ProjectKind, SecurityKeySource, SecurityMechanism,
    SecurityProfile, SecurityRequirement, SecurityRequirementApplicability, SecurityTransport,
    TechnicalBaselineApprovalType, TechnicalBaselineCandidateAgentWritable,
    TechnicalBaselineContract, TechnicalBaselineStatus,
};
use delivery_core::{
    read_selectors_value_from_paths, ArtifactKind, DeliveryIndex, DomainDispatcher,
    FileSubmitInput, LoomMcpActionResult, LoomMcpFailure, LoomMcpFailureResult,
    LoomMcpRepairableErrorResult, LoomMcpUserGateResult, OperationContext, ReadRequestFieldsInput,
    RouteAction, RouteActionKind, SubmitAcceptedEvent, TransitionEngine, TransitionStore,
};
use reference_catalog::schema::{ExtensionsRule, FolderHintsRule, LanguageRule, RepoSignalsConfig};
use schemars::schema_for;
use serde_json::{json, Value};
use state::{
    lifecycle_store::FileTransitionStore,
    paths::{from_project_relative, to_project_relative, DeliveryPhaseLocator},
    write_targets::AuthorizedWriteSet,
};

use crate::{
    paths::{
        repository_context_file, technical_baseline_candidate_file, technical_baseline_file,
        technical_baseline_request_file,
    },
    write_artifact_result,
};

// Bump this whenever the agent-facing write shape or its validation boundary changes.
// Existing requests are refreshed instead of handing an agent a stale producer contract.
const TECHNICAL_BASELINE_PROTOCOL_VERSION: &str = "2.1";

struct TechnicalBaselineSelectionProjections {
    recommendation_context: Value,
    user_confirmation_view: Value,
}

pub fn materialize_request(
    project_root: &str,
    delivery_id: &str,
    phase_id: &str,
) -> LoomMcpActionResult {
    match materialize_request_inner(project_root, delivery_id, phase_id) {
        Ok(result) => result,
        Err(error) => LoomMcpActionResult::Failed(LoomMcpFailureResult {
            project_root: project_root.to_string(),
            error: LoomMcpFailure {
                code: "TECHNICAL_BASELINE_REQUEST_FAILED".to_string(),
                message: error.to_string(),
                target_batch: Some(8),
                domain: Some("planning".to_string()),
                route_action: Some("technical_baseline_request".to_string()),
                recovery_tool: None,
            },
        }),
    }
}

fn materialize_request_inner(
    project_root: &str,
    delivery_id: &str,
    phase_id: &str,
) -> Result<LoomMcpActionResult, state::store::StateError> {
    let root = Path::new(project_root);
    let locator = DeliveryPhaseLocator {
        delivery_id: delivery_id.to_string(),
        phase_id: phase_id.to_string(),
    };
    let store = FileTransitionStore;
    let mut delivery = store
        .load_delivery_index(project_root, delivery_id)
        .map_err(to_state_error)?;
    let phase = delivery
        .phases
        .iter()
        .find(|phase| phase.phase_id == phase_id)
        .ok_or_else(|| {
            state::store::StateError::InvalidArgument(format!(
                "阶段 {} 在交付 {} 中不存在",
                phase_id, delivery_id
            ))
        })?;
    let brainstorm_ref = phase
        .latest_refs
        .get("brainstormContract")
        .ok_or_else(|| {
            state::store::StateError::InvalidArgument(
                "缺少最新的 brainstormContract 引用".to_string(),
            )
        })?
        .clone();
    let brainstorm = read_brainstorm_contract(root, &brainstorm_ref)?;
    let previous_baseline_file = technical_baseline_file(root, delivery_id);
    let previous_baseline = if previous_baseline_file.exists() {
        let previous_baseline_ref = to_project_relative(root, &previous_baseline_file)?;
        let previous: TechnicalBaselineContract = state::store::read_json(&previous_baseline_file)?;
        Some((previous_baseline_ref, previous))
    } else {
        None
    };
    let project_kind =
        infer_project_kind_for_baseline(root, &delivery, phase_id, previous_baseline.is_some());
    let selection_projections =
        technical_baseline_selection_projections(project_kind, previous_baseline.is_some());
    let protocol_fingerprint = technical_baseline_protocol_fingerprint(
        project_kind,
        previous_baseline.is_some(),
        &brainstorm,
        selection_projections.as_ref(),
    );

    if let Some(existing_request_ref) = phase
        .latest_refs
        .get("technicalBaselineRequestRef")
        .cloned()
    {
        let inspected = state::inspect_request(delivery_core::InspectRequestInput {
            project_root: project_root.to_string(),
            request_ref: existing_request_ref.clone(),
        });
        let request_is_current = phase
            .latest_refs
            .get("technicalBaselineRequestProtocolFingerprint")
            .is_some_and(|fingerprint| fingerprint == &protocol_fingerprint);
        if request_is_current
            && inspected
                .as_ref()
                .map(|request| request.request_kind == "technical_baseline_request")
                .unwrap_or(false)
        {
            if matches!(project_kind, ProjectKind::NewProject) && previous_baseline.is_none() {
                return Ok(technical_baseline_recommendation_gate(
                    project_root,
                    &existing_request_ref,
                    delivery_id,
                    phase_id,
                ));
            }
            return write_artifact_result(
                project_root,
                &existing_request_ref,
                ArtifactKind::TechnicalBaselineCandidate,
            );
        }
    }

    let request_id = format!("tbr_{}", state::store::now_millis());
    let candidate_file = to_project_relative(
        root,
        &technical_baseline_candidate_file(root, &locator, &request_id),
    )?;
    let request_file = to_project_relative(
        root,
        &technical_baseline_request_file(root, &locator, &request_id),
    )?;
    let request_root = build_request_root(
        root,
        &brainstorm,
        delivery_id,
        phase_id,
        &request_id,
        &candidate_file,
        project_kind,
        previous_baseline.as_ref(),
        selection_projections.as_ref(),
        &protocol_fingerprint,
    );
    let stored = state::write_native_request(
        project_root,
        state::NativeRequestInput {
            request_id: request_id.clone(),
            request_kind: "technical_baseline_request".to_string(),
            request_file: Some(request_file),
            delivery_id: Some(delivery_id.to_string()),
            phase_id: Some(phase_id.to_string()),
            root: request_root,
        },
    )?;
    if let Some(active_phase) = delivery
        .phases
        .iter_mut()
        .find(|phase| phase.phase_id == phase_id)
    {
        active_phase
            .latest_refs
            .insert("technicalBaselineRequestId".to_string(), request_id);
        active_phase.latest_refs.insert(
            "technicalBaselineRequestRef".to_string(),
            stored.request_ref.clone(),
        );
        active_phase.latest_refs.insert(
            "technicalBaselineRequestProtocolFingerprint".to_string(),
            protocol_fingerprint,
        );
    }
    delivery.updated_at = state::store::now_string();
    store
        .save_delivery_index(project_root, &delivery)
        .map_err(to_state_error)?;
    if matches!(project_kind, ProjectKind::NewProject) && previous_baseline.is_none() {
        Ok(technical_baseline_recommendation_gate(
            project_root,
            &stored.request_ref,
            delivery_id,
            phase_id,
        ))
    } else {
        write_artifact_result(
            project_root,
            &stored.request_ref,
            ArtifactKind::TechnicalBaselineCandidate,
        )
    }
}

fn build_request_root(
    project_root: &Path,
    brainstorm: &BrainstormContract,
    delivery_id: &str,
    phase_id: &str,
    request_id: &str,
    candidate_file: &str,
    project_kind: ProjectKind,
    previous_baseline: Option<&(String, TechnicalBaselineContract)>,
    selection_projections: Option<&TechnicalBaselineSelectionProjections>,
    protocol_fingerprint: &str,
) -> Value {
    let schema_shape = serde_json::to_value(schema_for!(TechnicalBaselineCandidateAgentWritable))
        .unwrap_or_else(|_| json!({ "type": "object" }));
    let baseline_exists = previous_baseline.is_some();
    let previous_baseline_context = previous_baseline.map(|(previous_ref, previous)| {
        json!({
            "previousBaselineRef": previous_ref,
            "technicalBaselineId": previous.technical_baseline_id,
            "status": previous.status,
            "projectKind": previous.project_kind,
            "scope": previous.scope,
            "stack": previous.stack,
            "constraints": previous.constraints,
            "confidence": previous.confidence,
            "securityProfiles": previous.security_profiles,
            "updatedAt": previous.updated_at
        })
    });
    let repo_evidence =
        technical_baseline_repo_evidence(project_root, project_kind, baseline_exists);
    let baseline_context_fields =
        technical_baseline_context_fields(brainstorm, previous_baseline.is_some());
    let repo_evidence_fields = technical_baseline_repo_evidence_fields(project_kind);
    let mut security_selection_fields = vec!["userConfirmationView"];
    if !matches!(
        brainstorm.security_requirement.applies,
        SecurityRequirementApplicability::NotApplicable
    ) {
        security_selection_fields.extend(["securityRequirement", "securityProfileGuidance"]);
    }
    json!({
        "schemaVersion": "1.0",
        "requestType": "technical_baseline_request",
        "deliveryId": delivery_id,
        "phaseId": phase_id,
        "requestId": request_id,
        "requestProtocol": {
            "version": TECHNICAL_BASELINE_PROTOCOL_VERSION,
            "fingerprint": protocol_fingerprint
        },
        "projectKind": project_kind,
        "operation": if matches!(project_kind, ProjectKind::ExistingProject) {
            "infer_existing_project_baseline"
        } else {
            "recommend_new_project_baseline"
        },
        "brainstormLens": {
            "summary": brainstorm.summary,
            "scopeIndex": {
                "includedIds": scope_item_ids(&brainstorm.scope.included),
                "includedLabels": scope_item_labels(&brainstorm.scope.included),
                "deferredIds": scope_item_ids(&brainstorm.scope.deferred),
                "deferredLabels": scope_item_labels(&brainstorm.scope.deferred),
                "excludedIds": scope_item_ids(&brainstorm.scope.excluded),
                "excludedLabels": scope_item_labels(&brainstorm.scope.excluded),
                "assumptionTexts": brainstorm.scope.assumptions.iter().map(|item| item.text.clone()).collect::<Vec<_>>()
            },
            "domainModel": brainstorm.domain_model.as_ref().map(|domain_model| json!({
                "capabilityNames": domain_model.capability_groups.iter().map(|item| item.name.clone()).collect::<Vec<_>>(),
                "businessFlowNames": domain_model.business_flows.iter().map(|item| item.name.clone()).collect::<Vec<_>>()
            })),
            "acceptanceIndex": brainstorm.acceptance.iter().map(|acceptance| json!({
                "id": acceptance.id,
                "priority": acceptance.priority,
                "capabilityRefs": acceptance.capability_refs,
                "sourceRefs": acceptance.source_refs
            })).collect::<Vec<_>>(),
            "frontendTarget": brainstorm.frontend_experience.as_ref().map(|frontend_experience| json!({
                "required": frontend_experience.required,
                "kind": frontend_experience.kind,
                "experienceLevel": frontend_experience.experience_level,
                "audienceNames": frontend_experience.audiences.iter().map(|item| item.name.clone()).collect::<Vec<_>>(),
                "surfaceNames": frontend_experience.surfaces.iter().map(|item| item.name.clone()).collect::<Vec<_>>(),
                "operationNames": frontend_experience.operation_paths.iter().map(|item| item.name.clone()).collect::<Vec<_>>(),
                "operationGoals": frontend_experience.operation_paths.iter().map(|item| item.user_goal.clone()).collect::<Vec<_>>()
            })),
            "userFacingLanguage": brainstorm.delivery_context.user_facing_language,
            "roadmapSignal": {
                "required": brainstorm.roadmap.required,
                "currentPhaseId": brainstorm.roadmap.current_phase_id,
                "phaseIds": brainstorm.roadmap.phases.iter().map(|phase| phase.phase_id.clone()).collect::<Vec<_>>(),
                "phaseTitles": brainstorm.roadmap.phases.iter().map(|phase| phase.title.clone().or_else(|| phase.name.clone()).unwrap_or_else(|| phase.phase_id.clone())).collect::<Vec<_>>(),
                "phaseGoals": brainstorm.roadmap.phases.iter().map(|phase| phase.goal.clone().unwrap_or_default()).collect::<Vec<_>>(),
                "nextPhasePreview": next_phase_preview_summary(&brainstorm.phase_plan.next_phase_preview)
            },
            "sourceRefs": brainstorm.sources.iter().map(|source| source.source_id.clone()).collect::<Vec<_>>()
        },
        "currentPhaseLens": {
            "phaseId": brainstorm.phase_plan.current.phase_id,
            "title": brainstorm.phase_plan.current.title,
            "goal": brainstorm.phase_plan.current.goal,
            "includedScopeRefs": brainstorm.phase_plan.current.scope_refs,
            "acceptanceRefs": brainstorm.phase_plan.current.acceptance_refs,
        },
        "securityRequirement": brainstorm.security_requirement,
        "securityProfileGuidance": security_profile_guidance(
            &brainstorm.security_requirement,
            baseline_exists,
            previous_baseline.map(|(_, baseline)| &baseline.stack),
        ),
        "decisionNeeds": technical_baseline_decision_needs(project_kind, baseline_exists),
        "previousBaselineContext": previous_baseline_context,
        "constraints": {
            "mustUse": [],
            "mustAvoid": [],
            "userPreferences": [],
            "deploymentPreference": "local_first"
        },
        "repoEvidence": repo_evidence,
        "recommendationContext": selection_projections
            .map(|projections| projections.recommendation_context.clone()),
        "userConfirmationView": selection_projections
            .map(|projections| projections.user_confirmation_view.clone()),
        "enumRefs": {
            "projectKind": ["new_project", "existing_project", "unknown"],
            "status": ["draft", "needs_user_confirmation", "auto_accepted", "confirmed", "blocked", "superseded"],
            "source": ["user_specified", "user_confirmed", "detected_from_repo", "agent_inferred_from_repo_signals", "agent_recommended_for_new_project"],
            "scope": ["project", "roadmap", "phase_override"],
            "approvalType": ["user_confirmed", "policy_auto_accept", "manual_override", "none"],
            "confidence": ["low", "medium", "high", "unknown"],
            "securityMechanism": ["none", "server_session", "bearer_jwt"],
            "securityAlgorithm": ["RS256", "ES256", "EdDSA", "HS256"],
            "securityKeySource": ["existing_idp", "environment_secret", "file_mounted_key", "kms", "user_specified", "not_applicable"],
            "securityTransport": ["bearer_header", "same_origin_cookie", "mutual_tls", "not_applicable"]
        },
        "rules": {
            "context": [
                "以确认的 Brainstorm 范围作为产品范围的权威。",
                "选择技术基线时，不得改写或削弱已确认的 Brainstorm 范围、验收标准或前端目标。"
            ],
            "candidatePolicy": [
                "仅写入 TechnicalBaseline 候选 JSON。",
                "不得直接写入已接受的基线文件。",
                "当基线仍需用户明确确认时，使用 needs_user_confirmation 加 approval.type=none。"
            ]
        },
        "outputContract": {
            "artifactKind": ArtifactKind::TechnicalBaselineCandidate,
            "writeMode": "single_json",
            "submitTool": "loom.technicalBaselineAcceptFile",
            "writeTargets": [{
                "targetId": "candidate",
                "path": candidate_file,
                "required": true,
                "description": "写入 TechnicalBaseline 候选 JSON。"
            }],
            "schemaShape": schema_shape,
            "schemaProjection": {
                "requiredTopLevelFields": [
                    "status",
                    "source",
                    "projectKind",
                    "scope",
                    "stack",
                    "securityProfiles",
                    "approval",
                    "confidence"
                ],
                "nestedShapeHints": {
                    "stack.tracks.externalServices.providers": {
                        "type": "array",
                        "items": {
                            "required": ["provider", "capabilities"],
                            "capabilities": {
                                "type": "array",
                                "itemRequired": [
                                    "purpose",
                                    "durability",
                                    "startupRequirement"
                                ]
                            }
                        }
                    }
                }
            }
        },
        "requestReadPlan": {
            "groups": [
                {
                    "groupId": "technical_baseline_context",
                    "required": true,
                    "purpose": "在起草基线之前，阅读已确认的 Brainstorm 范围、验收 ID、前端目标、当前阶段视角和基线决策需求。",
                    "whenToRead": "在产出任何 TechnicalBaseline 推荐之前阅读。",
                    "selectors": read_selectors_value_from_paths(baseline_context_fields)
                },
                {
                    "groupId": "technical_baseline_repo_evidence",
                    "required": false,
                    "purpose": "在推断现有项目基线或判断是否适用复用之前，阅读仓库证据。",
                    "whenToRead": "针对 existing_project 或仓库连续性相关时阅读。",
                    "selectors": read_selectors_value_from_paths(repo_evidence_fields)
                },
                {
                    "groupId": "technical_baseline_recommendation",
                    "required": selection_projections.is_some(),
                    "purpose": "在呈现基线推荐之前，阅读完整范围的推荐依据和轨道归属。",
                    "whenToRead": "在产出任何 TechnicalBaseline 推荐之前阅读。",
                    "selectors": read_selectors_value_from_paths(["recommendationContext"])
                },
                {
                    "groupId": "technical_baseline_user_confirmation",
                    "required": selection_projections.is_some()
                        || !matches!(
                            brainstorm.security_requirement.applies,
                            SecurityRequirementApplicability::NotApplicable
                        ),
                    "purpose": "在呈现基线之前，阅读精确的面向用户的选项矩阵和单消息确认协议。",
                    "whenToRead": "在呈现基线推荐或请求用户确认之前阅读。",
                    "selectors": read_selectors_value_from_paths(security_selection_fields)
                },
                {
                    "groupId": "technical_baseline_write_contract",
                    "required": true,
                    "purpose": "在写入 TechnicalBaseline 候选之前，阅读候选 schema 和写入目标。",
                    "whenToRead": "仅在准备写入候选文件时阅读。",
                    "selectors": read_selectors_value_from_paths([
                        "outputContract.writeTargets",
                        "outputContract.submitTool",
                        "outputContract.schemaProjection",
                        "enumRefs.projectKind",
                        "enumRefs.status",
                        "enumRefs.source",
                        "enumRefs.scope",
                        "enumRefs.approvalType",
                        "enumRefs.confidence",
                        "enumRefs.securityMechanism",
                        "enumRefs.securityAlgorithm",
                        "enumRefs.securityKeySource",
                        "enumRefs.securityTransport"
                    ])
                }
            ]
        }
    })
}

fn security_profile_guidance(
    requirement: &SecurityRequirement,
    has_previous_baseline: bool,
    baseline_stack: Option<&Value>,
) -> Value {
    let applicability = &requirement.applies;
    let profile_required = matches!(
        applicability,
        SecurityRequirementApplicability::Required | SecurityRequirementApplicability::Optional
    );
    let protected_scope = !matches!(
        applicability,
        SecurityRequirementApplicability::NotApplicable
    );
    let mut guidance = json!({
        "applies": protected_scope,
        "authority": "来自已接受 Brainstorm 合约的 securityRequirement",
        "profileRequired": profile_required,
        "capabilityState": "dormant",
        "activationRule": "bearer JWT 配置是显式选入能力。具有已接受服务器会话依赖的同源浏览器需求改用 server_session；不得从全新项目默认值、后端框架、仅 Redis 或一般性受保护需求中推荐 JWT。",
        "supportedProfiles": [
            {
                "mechanism": "server_session",
                "label": "服务器管理的浏览器会话",
                "transport": "same_origin_cookie",
                "when": "当已接受的基线包含 Redis 等服务器会话存储时，用于同源浏览器客户端。",
                "identityRule": "后端在应用接口权限之前从服务器端会话解析已认证用户和角色；这不是 bearer-token 合约。"
            },
            {
                "mechanism": "bearer_jwt",
                "label": "Bearer JWT",
                "transport": "bearer_header",
                "when": "仅在用户显式选择令牌授权场景及其完整配置后使用。",
                "identityRule": "后端在应用接口权限之前验证所选的令牌配置。"
            }
        ],
        "agentFields": []
    });
    guidance["serverSessionFactsAvailable"] =
        json!(baseline_stack.is_some_and(has_accepted_redis_session_capability));
    if profile_required {
        guidance["existingBaselineRule"] = json!(if has_previous_baseline {
            "当现有已接受的安全配置满足当前需求时予以复用；未经用户明确确认不得更改其算法。"
        } else {
            "当认证为必需或可选时，使用结构化信任模型和运行时能力事实。同源浏览器加上已接受的 Redis 会话能力使用 MCP 派生的 server_session；仅未解析的信任模型需要显式的用户配置选择。"
        });
        if has_previous_baseline
            && !baseline_stack.is_some_and(has_accepted_redis_session_capability)
        {
            guidance["algorithmPolicy"] = json!([
                "仅显式选择的 bearer JWT 配置可以声明算法。",
                "不得从传入的令牌头推导算法，也不得将其作为全新项目的默认值。",
                "将算法限定在显式选择的配置内；不得为灵活性添加第二个算法。"
            ]);
            guidance["agentFields"] = json!([
                "securityProfiles[].profileId",
                "securityProfiles[].name",
                "securityProfiles[].mechanism",
                "securityProfiles[].algorithm",
                "securityProfiles[].keySource",
                "securityProfiles[].transport",
                "securityProfiles[].issuer",
                "securityProfiles[].audiences",
                "securityProfiles[].claims",
                "securityProfiles[].sourceRefs",
                "securityProfiles[].rationale"
            ]);
        } else {
            guidance["selectionRule"] = json!(
                "对于同源浏览器加上已接受的 Redis 会话能力，使用 MCP 派生的 server_session 配置；代理不得要求用户选择 JWT 或写入令牌字段。否则在用户显式选择受支持的配置之前保持 securityProfiles 为空。如果用户选择 bearer_jwt，从该显式决策写入完整配置；不得臆造其场景、算法、签发方、受众、声明或传输方式。"
            );
            guidance["derivedProfileRule"] = json!({
                "mechanism": "server_session",
                "condition": "securityRequirement.applies 为 required 或 optional，clientTrustModels 包含 same_origin_browser，且 stack.tracks.externalServices.providers 包含 capabilities.purpose=session 的 Redis",
                "owner": "MCP 派生并持久化该配置；代理不得为此情况编写 securityProfiles。"
            });
        }
    } else if matches!(
        applicability,
        SecurityRequirementApplicability::DeferredWithRisk
    ) {
        guidance["profilePolicy"] = json!(
            "当前范围的安全已推迟。保持 securityProfiles 为空并记录推迟风险；不得在本阶段激活 JWT 或其他认证实现。"
        );
    } else {
        guidance["profilePolicy"] =
            json!("已接受范围不适用任何认证配置；securityProfiles 必须保持为空数组。");
    }
    if baseline_stack.is_some_and(has_accepted_redis_session_capability)
        && profile_required
        && same_origin_browser_is_required(requirement)
    {
        guidance["derivedProfileRule"] = json!({
            "mechanism": "server_session",
            "condition": "已接受的基线包含 capabilities.purpose=session 的 Redis，且已接受的安全需求包含 same_origin_browser。",
            "owner": "MCP 派生并持久化该配置；代理不得为此情况编写 securityProfiles。"
        });
        guidance["agentFields"] = json!([]);
    }
    guidance
}

fn has_accepted_redis_session_capability(stack: &Value) -> bool {
    let Some(track) = stack
        .pointer("/tracks/externalServices")
        .and_then(Value::as_object)
    else {
        return false;
    };
    if !matches!(
        track.get("status").and_then(Value::as_str),
        Some("selected" | "user_custom")
    ) {
        return false;
    }
    track
        .get("providers")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .any(|provider| {
            let is_redis = provider
                .get("provider")
                .and_then(Value::as_str)
                .is_some_and(|name| technology_matches_any(name, &["redis"]));
            is_redis
                && provider
                    .get("capabilities")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .any(|capability| {
                        capability
                            .get("purpose")
                            .and_then(Value::as_str)
                            .is_some_and(|purpose| normalize_stack_token(purpose) == "session")
                    })
        })
}

fn same_origin_browser_is_required(requirement: &SecurityRequirement) -> bool {
    matches!(
        requirement.applies,
        SecurityRequirementApplicability::Required | SecurityRequirementApplicability::Optional
    ) && requirement
        .client_trust_models
        .contains(&ClientTrustModel::SameOriginBrowser)
        && requirement
            .client_trust_models
            .iter()
            .all(|model| matches!(model, ClientTrustModel::SameOriginBrowser))
}

fn normalize_external_service_capability_shape(stack: &mut Value) {
    let Some(providers) = stack
        .pointer_mut("/tracks/externalServices/providers")
        .and_then(Value::as_array_mut)
    else {
        return;
    };
    for provider in providers {
        let Some(capabilities) = provider.get_mut("capabilities") else {
            continue;
        };
        let Some(role_map) = capabilities.as_object() else {
            continue;
        };
        let normalized = role_map
            .iter()
            .map(|(purpose, value)| {
                let mut capability = value.as_object()?.clone();
                capability.insert("purpose".to_string(), json!(purpose));
                Some(Value::Object(capability))
            })
            .collect::<Option<Vec<_>>>();
        if let Some(normalized) = normalized {
            *capabilities = Value::Array(normalized);
        }
    }
}

fn derive_server_session_profile(
    candidate: &mut TechnicalBaselineCandidateAgentWritable,
    requirement: &SecurityRequirement,
) -> bool {
    if !candidate.security_profiles.is_empty()
        || !same_origin_browser_is_required(requirement)
        || !has_accepted_redis_session_capability(&candidate.stack)
    {
        return false;
    }
    candidate.security_profiles.push(SecurityProfile {
        profile_id: "security_server_session".to_string(),
        name: "基于 Redis 的服务器会话".to_string(),
        mechanism: SecurityMechanism::ServerSession,
        algorithm: None,
        key_source: SecurityKeySource::NotApplicable,
        transport: SecurityTransport::SameOriginCookie,
        issuer: None,
        audiences: Vec::new(),
        claims: Vec::new(),
        source_refs: requirement.source_refs.clone(),
        rationale: "同源浏览器请求使用服务器管理的登录会话；后端在应用权限之前从 Redis 解析已认证用户和角色。".to_string(),
    });
    if candidate.approval.r#type == TechnicalBaselineApprovalType::UserConfirmed
        && candidate.status == TechnicalBaselineStatus::NeedsUserConfirmation
    {
        candidate.status = TechnicalBaselineStatus::Confirmed;
    }
    true
}

fn scope_item_ids(items: &[contracts::ScopeItem]) -> Vec<String> {
    items.iter().map(|item| item.id.clone()).collect()
}

fn scope_item_labels(items: &[contracts::ScopeItem]) -> Vec<String> {
    items.iter().map(|item| item.label.clone()).collect()
}

fn next_phase_preview_summary(preview: &contracts::NextPhasePreview) -> Value {
    match preview {
        contracts::NextPhasePreview::Candidate {
            suggested_phase_id,
            title,
            goal,
            scope_preview,
            reason,
        } => json!({
            "kind": "candidate",
            "suggestedPhaseId": suggested_phase_id,
            "title": title,
            "goal": goal,
            "scopePreview": scope_preview,
            "reason": reason
        }),
        contracts::NextPhasePreview::None { reason } => json!({
            "kind": "none",
            "suggestedPhaseId": Value::Null,
            "title": Value::Null,
            "goal": Value::Null,
            "scopePreview": [],
            "reason": reason
        }),
    }
}

fn technical_baseline_context_fields(
    brainstorm: &BrainstormContract,
    has_previous_baseline: bool,
) -> Vec<&'static str> {
    let mut fields = vec![
        "brainstormLens.summary.title",
        "brainstormLens.summary.oneLine",
        "brainstormLens.summary.complexity",
        "brainstormLens.scopeIndex.includedIds",
        "brainstormLens.scopeIndex.includedLabels",
        "brainstormLens.scopeIndex.deferredIds",
        "brainstormLens.scopeIndex.deferredLabels",
        "brainstormLens.scopeIndex.excludedIds",
        "brainstormLens.scopeIndex.excludedLabels",
        "brainstormLens.scopeIndex.assumptionTexts",
        "brainstormLens.acceptanceIndex",
        "brainstormLens.userFacingLanguage",
        "brainstormLens.roadmapSignal.required",
        "brainstormLens.roadmapSignal.currentPhaseId",
        "brainstormLens.roadmapSignal.phaseIds",
        "brainstormLens.roadmapSignal.phaseTitles",
        "brainstormLens.roadmapSignal.phaseGoals",
        "brainstormLens.roadmapSignal.nextPhasePreview.kind",
        "brainstormLens.roadmapSignal.nextPhasePreview.suggestedPhaseId",
        "brainstormLens.roadmapSignal.nextPhasePreview.title",
        "brainstormLens.roadmapSignal.nextPhasePreview.goal",
        "brainstormLens.roadmapSignal.nextPhasePreview.scopePreview",
        "brainstormLens.roadmapSignal.nextPhasePreview.reason",
        "currentPhaseLens.phaseId",
        "currentPhaseLens.title",
        "currentPhaseLens.goal",
        "currentPhaseLens.includedScopeRefs",
        "currentPhaseLens.acceptanceRefs",
        "decisionNeeds",
        "securityRequirement",
        "securityProfileGuidance",
        "constraints.mustUse",
        "constraints.mustAvoid",
        "constraints.userPreferences",
        "constraints.deploymentPreference",
    ];
    if brainstorm.summary.business_goal.is_some() {
        fields.insert(2, "brainstormLens.summary.businessGoal");
    }
    if brainstorm.domain_model.is_some() {
        fields.extend([
            "brainstormLens.domainModel.capabilityNames",
            "brainstormLens.domainModel.businessFlowNames",
        ]);
    }
    if brainstorm.frontend_experience.is_some() {
        fields.extend([
            "brainstormLens.frontendTarget.required",
            "brainstormLens.frontendTarget.kind",
            "brainstormLens.frontendTarget.experienceLevel",
            "brainstormLens.frontendTarget.audienceNames",
            "brainstormLens.frontendTarget.surfaceNames",
            "brainstormLens.frontendTarget.operationNames",
            "brainstormLens.frontendTarget.operationGoals",
        ]);
    }
    if has_previous_baseline {
        fields.extend([
            "previousBaselineContext.previousBaselineRef",
            "previousBaselineContext.technicalBaselineId",
            "previousBaselineContext.status",
            "previousBaselineContext.projectKind",
            "previousBaselineContext.scope",
            "previousBaselineContext.stack",
            "previousBaselineContext.constraints",
            "previousBaselineContext.confidence",
            "previousBaselineContext.securityProfiles",
        ]);
    }
    fields
}

fn technical_baseline_repo_evidence(
    project_root: &Path,
    project_kind: ProjectKind,
    baseline_exists: bool,
) -> Value {
    let mut evidence = json!({
        "detectedProjectKind": project_kind,
        "baselineExists": baseline_exists,
        "repositoryContextExists": false
    });
    if matches!(project_kind, ProjectKind::ExistingProject) {
        evidence["signals"] = compact_repo_signals(project_root);
    }
    evidence
}

fn technical_baseline_repo_evidence_fields(project_kind: ProjectKind) -> Vec<&'static str> {
    let mut fields = vec![
        "projectKind",
        "repoEvidence.detectedProjectKind",
        "repoEvidence.baselineExists",
        "repoEvidence.repositoryContextExists",
    ];
    if matches!(project_kind, ProjectKind::ExistingProject) {
        fields.extend([
            "repoEvidence.signals.manifests",
            "repoEvidence.signals.packageManagers",
            "repoEvidence.signals.languages",
            "repoEvidence.signals.frameworks",
            "repoEvidence.signals.sourceRoots",
        ]);
    }
    fields
}

fn compact_repo_signals(project_root: &Path) -> Value {
    let catalog = reference_catalog::resolved_catalog();
    let engine = RepoSignalEngine {
        config: &catalog.repo_signals,
    };
    engine.collect(project_root).to_json()
}

#[derive(Default)]
pub struct RepoSignalSummary {
    manifests: BTreeSet<String>,
    package_managers: BTreeSet<String>,
    languages: BTreeSet<String>,
    frameworks: BTreeSet<String>,
    source_roots: BTreeSet<String>,
}

impl RepoSignalSummary {
    pub fn to_json(&self) -> Value {
        json!({
            "manifests": sorted_values(&self.manifests),
            "packageManagers": sorted_values(&self.package_managers),
            "languages": sorted_values(&self.languages),
            "frameworks": sorted_values(&self.frameworks),
            "sourceRoots": sorted_values(&self.source_roots)
        })
    }
}

fn sorted_values(values: &BTreeSet<String>) -> Vec<String> {
    values.iter().cloned().collect()
}

pub struct RepoSignalEngine<'a> {
    pub config: &'a RepoSignalsConfig,
}

impl<'a> RepoSignalEngine<'a> {
    pub fn collect(&self, project_root: &Path) -> RepoSignalSummary {
        let mut signals = RepoSignalSummary::default();
        for lang in &self.config.languages {
            self.collect_language(project_root, lang, &mut signals);
        }
        self.collect_source_roots(project_root, &mut signals);
        signals
    }

    fn collect_source_roots(&self, root: &Path, signals: &mut RepoSignalSummary) {
        for path in &self.config.source_roots.paths {
            if root.join(path).exists() {
                signals.source_roots.insert(path.clone());
            }
        }
    }

    fn collect_language(&self, root: &Path, rule: &LanguageRule, signals: &mut RepoSignalSummary) {
        let mut matched_manifests: Vec<PathBuf> = Vec::new();
        let mut language_hit = false;

        for m in &rule.manifests {
            let path = root.join(&m.path);
            if path.is_file() {
                signals.manifests.insert(m.path.clone());
                if let Some(pm) = &m.package_manager {
                    signals.package_managers.insert(pm.clone());
                }
                matched_manifests.push(path);
                language_hit = true;
            }
        }

        if let Some(fh) = &rule.folder_hints {
            if self.folder_matches(root, fh) {
                language_hit = true;
            }
        }

        if let Some(ext) = &rule.extensions {
            if self.extension_scan_hits(root, ext) {
                language_hit = true;
            }
        }

        if language_hit {
            signals.languages.insert(rule.label.clone());
        }

        if !matched_manifests.is_empty() && !rule.frameworks.is_empty() {
            self.detect_frameworks(&matched_manifests, rule, signals);
        }
    }

    fn folder_matches(&self, root: &Path, fh: &FolderHintsRule) -> bool {
        for dir in &fh.dirs {
            let dir_path = root.join(dir);
            if !dir_path.is_dir() {
                continue;
            }
            if fh.require_extensions.is_empty() {
                return true;
            }
            if let Ok(entries) = fs::read_dir(&dir_path) {
                for entry in entries.filter_map(|e| e.ok()) {
                    if let Some(name) = entry.file_name().to_str() {
                        if fh.require_extensions.iter().any(|ext| name.ends_with(ext)) {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    fn extension_scan_hits(&self, root: &Path, ext: &ExtensionsRule) -> bool {
        let mut count = 0u32;
        self.count_extensions(root, ext, 0, &mut count);
        count >= ext.threshold
    }

    fn count_extensions(&self, dir: &Path, ext: &ExtensionsRule, depth: u32, count: &mut u32) {
        let max_depth = if self.config.max_scan_depth == 0 {
            6
        } else {
            self.config.max_scan_depth
        };
        if depth > max_depth || *count >= ext.threshold {
            return;
        }
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if self.config.skip_dirs.iter().any(|skip| skip == name) {
                        continue;
                    }
                }
                self.count_extensions(&path, ext, depth + 1, count);
                if *count >= ext.threshold {
                    return;
                }
            } else if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if ext.scan.iter().any(|s| name.ends_with(s)) {
                    *count += 1;
                    if *count >= ext.threshold {
                        return;
                    }
                }
            }
        }
    }

    fn detect_frameworks(
        &self,
        matched_manifests: &[PathBuf],
        rule: &LanguageRule,
        signals: &mut RepoSignalSummary,
    ) {
        let contents: Vec<String> = matched_manifests
            .iter()
            .filter_map(|p| fs::read_to_string(p).ok())
            .collect();
        let combined = contents.join("\n").to_lowercase();

        let dependency_keys: BTreeSet<String> = matched_manifests
            .iter()
            .filter_map(|p| {
                let Ok(text) = fs::read_to_string(p) else {
                    return None;
                };
                let Ok(pkg) = serde_json::from_str::<Value>(&text) else {
                    return None;
                };
                let mut keys = BTreeSet::new();
                for field in ["dependencies", "devDependencies", "peerDependencies"] {
                    if let Some(obj) = pkg.get(field).and_then(Value::as_object) {
                        keys.extend(obj.keys().cloned());
                    }
                }
                Some(keys)
            })
            .flatten()
            .collect();

        for fw in &rule.frameworks {
            let needle_lower = fw.needle.to_lowercase();
            if !dependency_keys.is_empty() && dependency_keys.contains(&fw.needle) {
                signals.frameworks.insert(fw.label.clone());
                continue;
            }
            if combined.contains(&needle_lower) {
                signals.frameworks.insert(fw.label.clone());
            }
        }
    }
}

const PORTABLE_DATA_ACCESS_OPTIONS: &[&str] = &["Raw SQL / framework-native wrapper", "No ORM"];
const PORTABLE_DATA_ACCESS_MATCHERS: &[&str] = &[
    "raw sql",
    "framework native wrapper",
    "lightweight wrapper",
    "no orm",
];

fn backend_ecosystem_guidance() -> Value {
    let catalog = reference_catalog::resolved_catalog();
    json!({
        "sourceOfTruth": "此目录是 backend/dataAccess 推荐关系和已知运行时家族兼容性检查的唯一来源。",
        "renderingRule": "将 backend 和 dataAccess 作为一个分组选择来呈现。不得呈现独立的扁平 dataAccess 选项列表。",
        "optionEnumerationRule": "在可调整范围中呈现生态系统时，枚举该生态系统的每个 backendOptions 条目和每个 recommendedDataAccessOptions 条目。不得将列表折叠为单个默认值或仅展示代表性选项。",
        "coverageRule": "当 backend 选择开放且没有已确认的约束排除某个生态系统时，保持 TypeScript/Node、Python、JVM/Spring 和 .NET 之间可调整范围的多样性。不得按目录顺序截断；当需求或用户偏好涉及 Go 时应将其纳入。",
        "customTechnologyPolicy": "捆绑包是主流推荐，不是白名单。当用户指定的后端和数据访问技术的关系是刻意的时予以保留，并在最终确认摘要中解释该自定义配对。",
        "portableDataAccessOptions": PORTABLE_DATA_ACCESS_OPTIONS,
        "bundles": catalog.backend_ecosystems().iter().map(|ecosystem| json!({
            "ecosystemId": ecosystem.ecosystem_id,
            "label": ecosystem.label,
            "runtimeFamily": ecosystem.runtime_family,
            "backendOptions": ecosystem.backend_options,
            "recommendedDataAccessOptions": ecosystem.data_access_options
        })).collect::<Vec<_>>()
    })
}

fn technical_baseline_selection_guidance(
    project_kind: ProjectKind,
    has_previous_baseline: bool,
) -> Option<Value> {
    if !matches!(project_kind, ProjectKind::NewProject) && !has_previous_baseline {
        return None;
    }
    Some(json!({
        "schemaVersion": "1.0",
        "purpose": if matches!(project_kind, ProjectKind::NewProject) {
            "在 PGC 之前，引导空或未初始化工作区中新项目的代理-用户技术基线确认。"
        } else {
            "当先前基线存在且最终候选可能添加、替换或与稳定基线元素冲突时，引导代理-用户技术基线确认。"
        },
        "runtimeBoundary": {
            "role": "请求仅提供材料、常见示例、输出合约和确认规则。",
            "doesNotDo": [
                "请求不推断此需求的具体推荐技术栈。",
                "请求不解析用户的自然语言技术回复。",
                "请求不参与中间确认轮次。"
            ],
            "requiredAgentLoop": [
                "阅读请求引用并理解已确认的需求范围。",
                "自行生成具体推荐或基线变更摘要。",
                "与用户进行所需轮次的对话。",
                "仅在用户明确确认最终技术基线后写入并提交候选。"
            ]
        },
        "confirmationRules": confirmation_rules(has_previous_baseline),
        "trackModel": {
            "requiredFinalShape": "使用 stack.tracks 包含 web、app、backend、persistence、dataAccess 和 externalServices 键。当选中 web 时，还需包含 qualityAutomation。每个轨道应包含 status、selection、source 和 rationale。选中的 externalServices 轨道还须包含具有已确认能力角色的结构化 providers。",
            "trackStatusValues": ["selected", "not_needed", "not_applicable", "user_custom"],
            "sourceValues": ["agent_recommended_user_confirmed", "user_adjusted", "user_specified", "previous_baseline", "not_applicable"],
            "coreTracks": ["web", "app", "backend", "persistence", "dataAccess", "externalServices"],
            "conditionalTracks": {
                "qualityAutomation": "当 web.status 为 selected 或 user_custom 时必需；在同一已确认基线中选择浏览器自动化技术栈。"
            },
            "coupledTracks": {
                "backendDataAccess": "backend 和 dataAccess 保持为独立的最终轨道，但推荐和确认必须将它们作为一个兼容的生态系统选择来呈现。"
            },
            "customTechnologyPolicy": "生态系统目录和独立轨道选项是示例，不是白名单。允许超出这些示例的用户指定技术，但须将相关轨道的 source 标记为 user_specified 或 user_custom，并将其纳入最终确认摘要和 reasoningSummary。",
            "externalServicesProviderShape": {
                "allowedProviderFields": ["provider", "capabilities"],
                "capabilitiesType": "array",
                "capabilityItemShape": {
                    "purpose": "cache | session | queue | stream | lock_rate_limit",
                    "durability": "ephemeral | persistent",
                    "startupRequirement": "required | optional"
                },
                "canonicalExample": {
                    "provider": "Redis",
                    "capabilities": [{
                        "purpose": "session",
                        "durability": "persistent",
                        "startupRequirement": "required"
                    }]
                },
                "allowedCapabilityFields": ["purpose", "durability", "startupRequirement"],
                "capabilityRoles": ["cache", "session", "queue", "stream", "lock_rate_limit"],
                "ownership": "TechnicalBaseline 仅确认 provider、能力角色、durability 和 startup requirement。MCP 派生 dependencyId 和 kind。故障处理、恢复、消费者和可观测性在影响当前阶段时归属于 Architecture 质量模型。",
                "shapeRule": "在候选 JSON 中，providers[].capabilities 始终是能力对象的数组。面向用户的角色映射（如 session: {...}）在写入候选之前必须规范化为 [{purpose: session, ...}]；不得将以角色为键的对象作为 capabilities 写入。",
                "unknownFieldPolicy": "不得在此添加 dependencyId、kind、requiredFor、failureBehavior、recoveryStrategy、observability、TTL、键模式、队列确认或部署设置。MCP 派生依赖标识，后续阶段负责操作行为。"
            }
        },
        "recommendationBasis": {
            "authority": "将完整的 BrainstormContract 作为首次新项目 TechnicalBaseline 推荐的产品范围权威。",
            "mustRead": [
                "brainstormLens.summary",
                "brainstormLens.scopeIndex 的 included/deferred/excluded 标签与假设",
                "brainstormLens.domainModel 的 capabilityNames 和 businessFlowNames（如存在）",
                "brainstormLens.frontendTarget（如存在）",
                "brainstormLens.roadmapSignal 的阶段标题、阶段目标与下一阶段预览"
            ],
            "currentPhaseLensRole": "currentPhaseLens 仅标识首个实现切片。当完整需求或路线图暗示后续产品界面、持久化规模、应用客户端、服务、集成或运维需求时，不得仅从当前阶段范围选择初始技术基线。",
            "recommendationRule": "为完整已确认的交付/路线图周期推荐稳定基线；说明当前阶段可在该基线内从小处起步而不隐藏后续已知需求的情况。"
        },
        "userFacingConfirmationProtocol": {
            "responseContract": {
                "responseId": "technical_baseline_confirmation",
                "mode": "single_user_message",
                "maxMessages": 1,
                "requiredSections": [
                    "recommendation_basis",
                    "recommended_final_baseline",
                    "adjustable_technology_range",
                    "confirmation_or_adjustment_prompt"
                ],
                "backendDataAccessRendering": "呈现每个 backendEcosystems 捆绑包及其 label、每个 backendOptions 条目和每个 recommendedDataAccessOptions 条目。不得缩写、折叠或重复该矩阵。",
                "deduplicationRule": "仅发出一次确认响应。不得在评论和最终输出中重复同一推荐。"
            },
            "mandatorySections": [
                "推荐依据：总结所使用的完整需求/路线图信号，而不仅是当前阶段。",
                "推荐最终基线：列出每个核心轨道及其选择和简短理由，当选中 web 时还需包含 qualityAutomation。",
                "可调整技术范围：展示 web、app、persistence 和 externalServices 的独立选项，然后将 backend 和 dataAccess 作为 backendEcosystems 中的分组生态系统选择来展示。对于每个展示的生态系统，列出该捆绑包提供的每个兼容 backend 和 dataAccess 选项；JVM/Spring 捆绑包必须包含 Java + Spring Boot 以及 Spring Data JPA、MyBatis Plus 和 jOOQ。",
                "回复格式：对于 web 项目，使用 web、app、backend、persistence、dataAccess、externalServices 和 qualityAutomation 展示规范的 key=value 示例。",
                "当 securityRequirement 为 required 或 optional 时，使用结构化信任模型和已接受的运行时能力为同源浏览器会话选择 server_session；不得将 JWT 作为全新项目默认值提出或启用。仅当结构化事实无法确定机制时才请求显式配置选择。当安全推迟时，展示推迟风险并保持当前阶段不包含认证实现。",
                "最终确认规则：如果用户更改了任何内容，在提交前总结最终基线并请求明确确认。"
            ],
            "wordingRules": [
                "不得将推荐呈现为仅基于首个阶段或当前小型实现切片。",
                "不得省略可调整技术范围。",
                "当预期选择主流框架时，不得将 backend 选项呈现为仅语言的标签；在面向用户的示例中展示语言 + 框架组合。",
                "不得将 backend 和 dataAccess 呈现为不相关的选项列表。将每个展示的数据访问选择保持在其兼容的 backend 生态系统下。",
                "不得仅用最熟悉的默认值替换兼容的 dataAccess 列表。保留每个目录选项，包括 Java + Spring Boot 下的 MyBatis Plus 和 jOOQ。",
                "不得按目录顺序截断 backend 备选方案。当没有已确认的约束排除它们时，保留主流生态系统覆盖范围，包括 Java + Spring Boot。",
                "不得使用 db 或 orm 作为主要回复键；在主要示例中使用 persistence 和 dataAccess。",
                "当 externalServices 包含 Redis 时，用通俗语言解释业务角色，允许多个角色，并仅询问所选角色所需的持久化或启动问题。",
                "不得在 TechnicalBaseline 确认期间要求用户选择 Redis 数据结构、TTL 值、Lua 脚本或 Compose 设置；这些决策归属于 Architecture。",
                "不得在面向用户的文本中提及 Loom 内部机制、门控、提交权限、工作流阻塞，或类似 Loom allows、Loom requires、Loom is stuck、Loom will not continue 等措辞。",
                "当用户使用 db 和 orm 等别名时可以将其理解为 persistence 和 dataAccess，但须将最终候选规范化为 stack.tracks.persistence 和 stack.tracks.dataAccess。"
            ]
        },
        "independentTrackOptions": {
            "web": {
                "label": "Web 客户端",
                "examples": ["Next.js", "React + Vite", "Vue + Vite", "SvelteKit", "Astro", "无 Web 客户端"]
            },
            "app": {
                "label": "App 客户端",
                "examples": ["无 App 客户端", "React Native + Expo", "Flutter", "iOS Native (Swift / SwiftUI)", "Android Native (Kotlin / Jetpack Compose)", "Hybrid WebView (Capacitor / Ionic)", "PWA"]
            },
            "persistence": {
                "label": "数据库 / 持久化",
                "examples": ["SQLite", "PostgreSQL", "MySQL", "MongoDB", "文件存储 / 本地 JSON", "暂无持久化"]
            },
            "externalServices": {
                "label": "外部服务",
                "examples": ["无", "用于缓存、会话或后台任务的 Redis", "用户指定", "仅推荐已确认需求明确要求的服务"],
                "capabilityRoles": {
                    "redis": [
                        "登录状态与共享会话",
                        "后台任务与消息处理",
                        "查询结果加速",
                        "原子锁或限流"
                    ]
                },
                "confirmationRule": "仅展示已确认需求支持的能力角色。允许多个角色。不得仅凭关键词选择 Redis 角色；当角色不明确时询问用户。"
            },
            "qualityAutomation": {
                "label": "浏览器质量自动化",
                "examples": ["Playwright", "用户指定的现有浏览器测试技术栈"]
            },
            "securityProfile": {
                "label": "受保护时的认证配置",
                "examples": ["由 Redis 支持的同源浏览器的 MCP 派生服务器会话", "已确认外部客户端场景的显式选择 bearer JWT 配置", "未经更改复用的现有已接受安全配置"],
                "rule": "对于同源浏览器加上已接受的 Redis 会话能力，使用 server_session 并从登录会话解析身份/角色。JWT 处于休眠状态，不得从默认值或关键词中选择；仅从显式用户选择或现有已接受配置中激活。"
            }
        },
        "backendEcosystems": backend_ecosystem_guidance(),
        "shorthandNormalization": {
            "backend": [
                "如果用户写 backend=Java 但未指定框架，将其规范化为 Java + Spring Boot，除非用户显式指定了不同的 Java 后端技术栈。",
                "如果用户写 backend=Python 但未指定框架，对于服务/后端工作将其规范化为 Python + FastAPI，除非需求或用户显式指向 Django 风格的站点/管理/内容能力。",
                "如果用户写 backend=Node.js 但未指定框架，在最终确认之前请求或总结具体的 Node.js 框架选择，如 Fastify、Express 或 NestJS。",
                "如果用户写 backend=.NET 但未指定框架，将其规范化为 .NET + ASP.NET Core，除非用户显式指定了其他 .NET 后端技术栈。"
            ]
        },
        "recommendationPrinciples": [
            "优先选择主流、可维护、社区成熟的技术。",
            "优先选择与已确认的产品形态和实现工作量匹配的技术。",
            "对于 Web UI，除非用户另有选择，否则优先使用 TypeScript。",
            "对于中小型本地优先 CRUD/管理系统，除非用户需要生产级多用户数据库，否则 SQLite 是合理的默认选择。",
            "当集成式全栈选项能降低编排成本且仍满足产品需求时予以优先。",
            "即使超出常见示例，也尊重用户的显式技术选择。",
            "除非用户要求或需求明确需要，否则避免小众技术栈。"
        ],
        "replyProtocolForUser": {
            "acceptRecommendation": "确认推荐方案",
            "partialAdjustmentExample": "web=Vue+Vite, backend=Java+Spring Boot, persistence=PostgreSQL, dataAccess=Spring Data JPA, qualityAutomation=Playwright, app=不需要, externalServices=不需要",
            "fullCustomExample": "web=React+Vite, app=React Native+Expo, backend=Node.js+Fastify, persistence=SQLite, dataAccess=Prisma, qualityAutomation=Playwright, externalServices=不需要",
            "redisCapabilityExample": "externalServices=Redis，用于登录会话和后台任务；登录会话重启后保留，后台任务失败可重试",
            "finalConfirmationPrompt": "当用户未直接接受推荐时，呈现最终技术基线摘要并要求其回复 确认技术栈 或 修改: ..."
        }
    }))
}

fn technical_baseline_selection_projections(
    project_kind: ProjectKind,
    has_previous_baseline: bool,
) -> Option<TechnicalBaselineSelectionProjections> {
    let guidance = technical_baseline_selection_guidance(project_kind, has_previous_baseline)?;
    Some(TechnicalBaselineSelectionProjections {
        recommendation_context: json!({
            "schemaVersion": guidance["schemaVersion"],
            "purpose": guidance["purpose"],
            "runtimeBoundary": guidance["runtimeBoundary"],
            "confirmationRules": guidance["confirmationRules"],
            "trackModel": guidance["trackModel"],
            "recommendationBasis": guidance["recommendationBasis"],
            "recommendationPrinciples": guidance["recommendationPrinciples"],
            "shorthandNormalization": guidance["shorthandNormalization"]
        }),
        user_confirmation_view: json!({
            "schemaVersion": guidance["schemaVersion"],
            "userFacingConfirmationProtocol": guidance["userFacingConfirmationProtocol"],
            "independentTrackOptions": guidance["independentTrackOptions"],
            "backendEcosystems": guidance["backendEcosystems"],
            "replyProtocolForUser": guidance["replyProtocolForUser"]
        }),
    })
}

fn technical_baseline_protocol_fingerprint(
    project_kind: ProjectKind,
    has_previous_baseline: bool,
    brainstorm: &BrainstormContract,
    projections: Option<&TechnicalBaselineSelectionProjections>,
) -> String {
    delivery_core::contract_fingerprint(&json!({
        "version": TECHNICAL_BASELINE_PROTOCOL_VERSION,
        "projectKind": project_kind,
        "hasPreviousBaseline": has_previous_baseline,
        "securityApplicability": brainstorm.security_requirement.applies,
        "recommendationContext": projections.map(|item| &item.recommendation_context),
        "userConfirmationView": projections.map(|item| &item.user_confirmation_view)
    }))
}

fn confirmation_rules(has_previous_baseline: bool) -> Vec<&'static str> {
    let mut rules = vec![
        "用户需求确认不等于技术基线确认。",
        "如果用户直接接受推荐，该回复即可作为最终技术基线确认。",
        "如果用户调整了部分技术栈或指定了自定义技术栈，在写入候选之前总结最终基线并请求最终确认。",
        "当任何核心轨道不明确时不得提交已确认的候选。仅当需求或用户确认支持时才将轨道标记为 not_applicable/not_needed。",
        "将 externalServices.providers[].capabilities 写入为包含 purpose、durability 和 startupRequirement 的对象数组。不得使用以 session、cache、queue 或其他能力角色为键的对象。",
        "构建命令、本地运行命令和部署准备在后续派生。对于选中的 Web 客户端，在同一基线确认中包含 qualityAutomation；不得仅为更新测试命令或运行时细节而重新开启确认。",
        "JWT 保持休眠。对于具有已接受 Redis 会话能力的同源浏览器工作，使用 server_session 配置并从登录会话解析身份/角色。仅当没有确定性会话配置适用或用户请求其他信任模型时才请求显式安全配置。",
        "deferred_with_risk 安全需求可以在没有安全配置的情况下被接受，但当前阶段不得创建认证或 JWT 实现工作。",
    ];
    if has_previous_baseline {
        rules.extend([
            "当先前基线存在时，对于现有技术栈内的常规修复、修缮、优化或功能工作，默认复用未更改的基线。",
            "仅当当前已确认范围显式添加新技术界面或替换先前基线元素时才需要显式技术基线确认。",
            "当前仓库的脚本、测试命令、构建命令、启动命令、生成文件或框架实现细节属于实现事实；不得将其本身视为面向用户的技术基线变更。",
            "保留用户未确认更改的先前基线轨道。",
        ]);
    }
    rules
}

pub fn accept_technical_baseline_file<D>(
    input: &FileSubmitInput,
    authorized: &AuthorizedWriteSet,
    dispatcher: D,
) -> LoomMcpActionResult
where
    D: DomainDispatcher + Clone,
{
    match accept_technical_baseline_file_inner(input, authorized, dispatcher) {
        Ok(result) => result,
        Err(error) => LoomMcpActionResult::Failed(LoomMcpFailureResult {
            project_root: input.project_root.clone(),
            error: LoomMcpFailure {
                code: "TECHNICAL_BASELINE_ACCEPT_FAILED".to_string(),
                message: error.to_string(),
                target_batch: Some(8),
                domain: Some("planning".to_string()),
                route_action: Some("technical_baseline_accept".to_string()),
                recovery_tool: None,
            },
        }),
    }
}

fn accept_technical_baseline_file_inner<D>(
    input: &FileSubmitInput,
    authorized: &AuthorizedWriteSet,
    dispatcher: D,
) -> Result<LoomMcpActionResult, state::store::StateError>
where
    D: DomainDispatcher + Clone,
{
    let Some(target) = authorized.targets.first() else {
        return Ok(repairable(
            input,
            authorized,
            String::new(),
            vec![issue(
                "TARGET_MISSING",
                "candidate",
                "未写入已授权的 TechnicalBaseline 目标。",
            )],
        ));
    };
    let delivery_id = authorized.delivery_id.clone().ok_or_else(|| {
        state::store::StateError::InvalidArgument("authorized deliveryId is missing".to_string())
    })?;
    let phase_id = authorized.phase_id.clone().ok_or_else(|| {
        state::store::StateError::InvalidArgument("authorized phaseId is missing".to_string())
    })?;
    if let Some(result) = ensure_latest_request(
        &input.project_root,
        &delivery_id,
        &phase_id,
        &input.request_ref,
        "technicalBaselineRequestRef",
    )? {
        return Ok(result);
    }
    let project_root = Path::new(&input.project_root);
    let request_fields = state::read_request_fields(ReadRequestFieldsInput {
        project_root: input.project_root.clone(),
        request_ref: input.request_ref.clone(),
        fields: vec!["securityRequirement".to_string()],
    })?;
    let security_requirement = request_fields
        .fields
        .get("securityRequirement")
        .and_then(|field| serde_json::from_value::<SecurityRequirement>(field.value.clone()).ok())
        .unwrap_or_default();
    let candidate_file = from_project_relative(project_root, &target.path)?;
    let raw = state::store::read_json_value(&candidate_file)?;
    let mut candidate: TechnicalBaselineCandidateAgentWritable =
        match serde_json::from_value(raw.clone()) {
            Ok(candidate) => candidate,
            Err(error) => {
                return Ok(repairable(
                    input,
                    authorized,
                    target.path.clone(),
                    vec![issue(
                        "TECHNICAL_BASELINE_SCHEMA_INVALID",
                        "candidate",
                        &format!("TechnicalBaseline 候选 JSON 的 schema 无效：{error}"),
                    )],
                ));
            }
        };

    let now = state::store::now_string();
    normalize_user_confirmed_approval(&mut candidate, &now);
    normalize_external_service_capability_shape(&mut candidate.stack);
    derive_server_session_profile(&mut candidate, &security_requirement);
    let issues = validate_candidate(&candidate, &security_requirement);
    if !issues.is_empty() {
        return Ok(repairable(input, authorized, target.path.clone(), issues));
    }
    if matches!(candidate.project_kind, ProjectKind::Unknown) {
        return Ok(technical_baseline_user_gate(
            input,
            authorized,
            "询问用户本阶段是继续现有项目还是启动新项目，然后使用确认的 projectKind 重写同一候选。"
                .to_string(),
            "project_kind_confirmation".to_string(),
        ));
    }
    if matches!(candidate.project_kind, ProjectKind::NewProject)
        && candidate.approval.r#type != TechnicalBaselineApprovalType::UserConfirmed
    {
        return Ok(technical_baseline_user_gate(
            input,
            authorized,
            "新项目的技术基线在规划继续之前必须由用户明确确认。呈现推荐技术栈，捕获修正，然后使用 approval.type=user_confirmed 重写同一候选。".to_string(),
            "new_project_baseline_confirmation".to_string(),
        ));
    }
    let previous_baseline_file = technical_baseline_file(project_root, &delivery_id);
    let security_profile_required = matches!(
        security_requirement.applies,
        SecurityRequirementApplicability::Required | SecurityRequirementApplicability::Optional
    );
    if security_profile_required && candidate.security_profiles.is_empty() {
        return Ok(technical_baseline_user_gate(
            input,
            authorized,
            "认证在范围内，但结构化事实无法确定安全配置。JWT 保持休眠且不是默认值。请求用户选择认证场景或调整范围，然后在继续之前重写同一候选。".to_string(),
            "security_profile_confirmation".to_string(),
        ));
    }
    if matches!(
        candidate.status,
        TechnicalBaselineStatus::NeedsUserConfirmation
    ) {
        return Ok(technical_baseline_user_gate(
            input,
            authorized,
            "技术基线仍需用户明确确认。呈现基线变更或推荐，然后使用已确认的基线重写同一候选。"
                .to_string(),
            "technical_baseline_confirmation".to_string(),
        ));
    }
    if previous_baseline_file.exists() {
        let previous: TechnicalBaselineContract = state::store::read_json(&previous_baseline_file)?;
        let stack_or_baseline_changed = technical_baseline_conflicts(&previous, &candidate);
        let repo_signal_conflicts =
            repo_signals_conflict_with_previous(project_root, &input.request_ref, &previous)?;
        if (stack_or_baseline_changed || !repo_signal_conflicts.is_empty())
            && !matches!(
                candidate.approval.r#type,
                TechnicalBaselineApprovalType::UserConfirmed
                    | TechnicalBaselineApprovalType::ManualOverride
            )
        {
            return Ok(technical_baseline_user_gate(
                input,
                authorized,
                "提议的技术基线变更了现有基线。向用户呈现先前基线和提议变更，然后在明确确认后使用 approval.type=user_confirmed 重写同一候选。".to_string(),
                "previous_baseline_change_confirmation".to_string(),
            ));
        }
    }

    let persisted = TechnicalBaselineContract {
        schema_version: "1.0".to_string(),
        technical_baseline_id: format!("tb_{}_{}", phase_id, state::store::now_millis()),
        delivery_id: delivery_id.clone(),
        phase_id: phase_id.clone(),
        status: candidate.status,
        source: candidate.source,
        project_kind: candidate.project_kind,
        scope: candidate.scope,
        stack: candidate.stack,
        security_profiles: candidate.security_profiles,
        constraints: candidate.constraints,
        evidence: candidate.evidence,
        approval: candidate.approval,
        confidence: candidate.confidence,
        reasoning_summary: candidate.reasoning_summary,
        alternatives: candidate.alternatives,
        created_at: now.clone(),
        updated_at: now.clone(),
    };
    let baseline_file = technical_baseline_file(project_root, &delivery_id);
    state::store::write_json_atomic(&baseline_file, &persisted)?;
    state::lifecycle_store::finalize_agent_candidate(project_root, &target.path)?;
    let baseline_ref = to_project_relative(project_root, &baseline_file)?;

    let store = FileTransitionStore;
    let mut delivery = store
        .load_delivery_index(&input.project_root, &delivery_id)
        .map_err(to_state_error)?;
    if let Some(phase) = delivery
        .phases
        .iter_mut()
        .find(|phase| phase.phase_id == phase_id)
    {
        phase.latest_refs.insert(
            "technicalBaselineRequestRef".to_string(),
            input.request_ref.clone(),
        );
        phase
            .latest_refs
            .insert("technicalBaseline".to_string(), baseline_ref.clone());
    }
    delivery.updated_at = now;
    store
        .save_delivery_index(&input.project_root, &delivery)
        .map_err(to_state_error)?;

    let next_action = if matches!(persisted.project_kind, ProjectKind::ExistingProject)
        && !repository_context_file(
            project_root,
            &DeliveryPhaseLocator {
                delivery_id: delivery_id.clone(),
                phase_id: phase_id.clone(),
            },
        )
        .exists()
    {
        RouteAction {
            kind: RouteActionKind::RepositoryContextRequest,
            source: "technical_baseline_accept".to_string(),
            reason: "technical_baseline_ready_existing_project".to_string(),
            prompt: None,
            accepted_responses: vec![],
            request_ref: None,
            details: None,
            target_phase_id: None,
        }
    } else {
        RouteAction {
            kind: RouteActionKind::PlanningContractCreate,
            source: "technical_baseline_accept".to_string(),
            reason: "technical_baseline_ready".to_string(),
            prompt: None,
            accepted_responses: vec![],
            request_ref: None,
            details: None,
            target_phase_id: None,
        }
    };

    let engine = TransitionEngine {
        store: FileTransitionStore,
        dispatcher,
    };
    engine
        .advance_after_submit(
            OperationContext {
                project_root: input.project_root.clone(),
            },
            SubmitAcceptedEvent {
                delivery_id,
                phase_id,
                source_tool: "loom.technicalBaselineAcceptFile".to_string(),
                accepted_artifact_ref: format!(
                    "{}/targets/{}",
                    input.request_ref, target.target_id
                ),
                next_action: Some(next_action),
            },
        )
        .map_err(to_state_error)
}

fn normalize_user_confirmed_approval(
    candidate: &mut TechnicalBaselineCandidateAgentWritable,
    confirmed_at: &str,
) {
    if candidate.approval.r#type != TechnicalBaselineApprovalType::UserConfirmed {
        return;
    }
    if candidate
        .approval
        .confirmed_at
        .as_deref()
        .map(str::trim)
        .unwrap_or_default()
        .is_empty()
    {
        candidate.approval.confirmed_at = Some(confirmed_at.to_string());
    }
}

fn validate_candidate(
    candidate: &TechnicalBaselineCandidateAgentWritable,
    security_requirement: &SecurityRequirement,
) -> Vec<delivery_core::RepairIssue> {
    let mut issues = Vec::new();
    if !candidate.stack.is_object() {
        issues.push(issue(
            "TECHNICAL_BASELINE_STACK_INVALID",
            "stack",
            "stack 必须是描述所选技术基线的 JSON 对象。",
        ));
    }
    validate_external_services_track(&candidate.stack, &mut issues);
    validate_security_profiles(candidate, security_requirement, &mut issues);
    if candidate.approval.r#type == TechnicalBaselineApprovalType::None
        && matches!(candidate.status, TechnicalBaselineStatus::Confirmed)
    {
        issues.push(issue(
            "TECHNICAL_BASELINE_APPROVAL_INVALID",
            "approval.type",
            "已确认的 TechnicalBaseline 不得保留 approval.type=none。",
        ));
    }
    if matches!(candidate.project_kind, ProjectKind::NewProject) {
        validate_new_project_candidate(candidate, &mut issues);
    }
    issues
}

fn validate_security_profiles(
    candidate: &TechnicalBaselineCandidateAgentWritable,
    requirement: &SecurityRequirement,
    issues: &mut Vec<delivery_core::RepairIssue>,
) {
    let not_applicable = matches!(
        requirement.applies,
        SecurityRequirementApplicability::NotApplicable
    );
    let profile_required = matches!(
        requirement.applies,
        SecurityRequirementApplicability::Required | SecurityRequirementApplicability::Optional
    );
    if not_applicable && !candidate.security_profiles.is_empty() {
        issues.push(issue(
            "TECHNICAL_BASELINE_SECURITY_PROFILE_UNEXPECTED",
            "securityProfiles",
            "当已接受的安全需求为 not_applicable 时 securityProfiles 必须为空。",
        ));
        return;
    }
    if profile_required
        && !candidate.security_profiles.is_empty()
        && !candidate
            .security_profiles
            .iter()
            .any(|profile| !matches!(profile.mechanism, SecurityMechanism::None))
    {
        issues.push(issue(
            "TECHNICAL_BASELINE_PROTECTED_PROFILE_REQUIRED",
            "securityProfiles",
            "required 或 optional 的安全需求必须包含受支持的非 none 安全配置。由已接受 Redis 会话支持的同源浏览器会话使用 server_session；bearer_jwt 保持显式选入。",
        ));
    }
    let mut profile_ids = BTreeSet::new();
    for (index, profile) in candidate.security_profiles.iter().enumerate() {
        let path = format!("securityProfiles[{index}]");
        if profile.profile_id.trim().is_empty() || !profile_ids.insert(profile.profile_id.clone()) {
            issues.push(issue(
                "TECHNICAL_BASELINE_SECURITY_PROFILE_ID_INVALID",
                &format!("{path}.profileId"),
                "每个安全配置需要唯一且非空的 profileId。",
            ));
        }
        if profile.name.trim().is_empty() || profile.rationale.trim().is_empty() {
            issues.push(issue(
                "TECHNICAL_BASELINE_SECURITY_PROFILE_DESCRIPTION_REQUIRED",
                &path,
                "每个安全配置需要非空的 name 和 rationale。",
            ));
        }
        match profile.mechanism {
            SecurityMechanism::None => {
                if profile.algorithm.is_some()
                    || !matches!(profile.key_source, SecurityKeySource::NotApplicable)
                    || !matches!(profile.transport, SecurityTransport::NotApplicable)
                {
                    issues.push(issue(
                        "TECHNICAL_BASELINE_SECURITY_PROFILE_NONE_INVALID",
                        &path,
                        "none 安全配置不得声明算法、密钥材料或传输方式。",
                    ));
                }
            }
            SecurityMechanism::ServerSession => {
                if !same_origin_browser_is_required(requirement)
                    || !has_accepted_redis_session_capability(&candidate.stack)
                    || profile.algorithm.is_some()
                    || !matches!(profile.key_source, SecurityKeySource::NotApplicable)
                    || !matches!(profile.transport, SecurityTransport::SameOriginCookie)
                    || profile.issuer.is_some()
                    || !profile.audiences.is_empty()
                    || !profile.claims.is_empty()
                {
                    issues.push(issue(
                        "TECHNICAL_BASELINE_SERVER_SESSION_PROFILE_INVALID",
                        &path,
                        "server_session 配置仅对具有已接受 Redis 会话能力的同源浏览器需求有效；它必须使用 same_origin_cookie 且不得声明令牌算法或 issuer/audience/claim 字段。",
                    ));
                }
            }
            SecurityMechanism::BearerJwt => {
                if profile.algorithm.is_none() {
                    issues.push(issue(
                        "TECHNICAL_BASELINE_SECURITY_ALGORITHM_REQUIRED",
                        &format!("{path}.algorithm"),
                        "bearer JWT 配置必须显式选择一个签名算法。",
                    ));
                }
                if matches!(profile.key_source, SecurityKeySource::NotApplicable)
                    || matches!(profile.transport, SecurityTransport::NotApplicable)
                {
                    issues.push(issue(
                        "TECHNICAL_BASELINE_SECURITY_PROFILE_BOUNDARY_REQUIRED",
                        &path,
                        "bearer JWT 配置必须声明密钥来源和传输方式。",
                    ));
                }
                if profile
                    .issuer
                    .as_deref()
                    .is_none_or(|issuer| issuer.trim().is_empty())
                    || profile.audiences.is_empty()
                    || !profile.claims.iter().any(|claim| claim.trim() == "sub")
                {
                    issues.push(issue(
                        "TECHNICAL_BASELINE_JWT_CLAIMS_INCOMPLETE",
                        &path,
                        "bearer JWT 配置在被 API 合约使用之前必须声明 issuer、至少一个 audience 和 subject 声明。",
                    ));
                }
            }
        }
    }
}

fn validate_external_services_track(stack: &Value, issues: &mut Vec<delivery_core::RepairIssue>) {
    let Some(track) = stack
        .pointer("/tracks/externalServices")
        .and_then(Value::as_object)
    else {
        return;
    };
    let status = track
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if matches!(status, "selected" | "user_custom")
        && !external_services_track_complete(track, status)
    {
        issues.push(issue(
            "TECHNICAL_BASELINE_EXTERNAL_SERVICES_UNSTRUCTURED",
            "stack.tracks.externalServices.providers",
            "选中的 externalServices 轨道必须包含至少一个具有一个或多个结构化能力角色的 provider，每个角色声明 purpose、durability 和 startupRequirement。",
        ));
    }
    if let Some(providers) = track.get("providers").and_then(Value::as_array) {
        let mut provider_ids = BTreeSet::new();
        for (provider_index, provider) in providers.iter().enumerate() {
            let Some(provider) = provider.as_object() else {
                continue;
            };
            let provider_path =
                format!("stack.tracks.externalServices.providers[{provider_index}]");
            if let Some(provider_name) = provider
                .get("provider")
                .and_then(Value::as_str)
                .map(normalize_stack_token)
                .filter(|value| !value.is_empty())
            {
                if !provider_ids.insert(provider_name) {
                    issues.push(issue(
                        "TECHNICAL_BASELINE_EXTERNAL_SERVICE_PROVIDER_DUPLICATE",
                        &format!("{provider_path}.provider"),
                        "每个外部服务 provider 只能出现一次；将其能力角色合并在一个 providers 条目中，以便 MCP 派生一个稳定的运行时依赖。",
                    ));
                }
            }
            for field in provider.keys() {
                if !matches!(field.as_str(), "provider" | "capabilities") {
                    issues.push(issue(
                        "TECHNICAL_BASELINE_EXTERNAL_SERVICE_FIELD_UNKNOWN",
                        &format!("{provider_path}.{field}"),
                        "TechnicalBaseline 外部服务 provider 只能声明 provider 和 capabilities；MCP 派生依赖标识，操作字段归属于 Architecture。",
                    ));
                }
            }
            if let Some(capabilities) = provider.get("capabilities").and_then(Value::as_array) {
                let mut purposes = BTreeSet::new();
                for (capability_index, capability) in capabilities.iter().enumerate() {
                    let Some(capability) = capability.as_object() else {
                        continue;
                    };
                    let capability_path =
                        format!("{provider_path}.capabilities[{capability_index}]");
                    if let Some(purpose) = capability
                        .get("purpose")
                        .and_then(Value::as_str)
                        .map(normalize_stack_token)
                        .filter(|value| !value.is_empty())
                    {
                        if !purposes.insert(purpose) {
                            issues.push(issue(
                                "TECHNICAL_BASELINE_EXTERNAL_SERVICE_PURPOSE_DUPLICATE",
                                &format!("{capability_path}.purpose"),
                                "每个 provider 只能声明一个能力 purpose 一次；在接受 TechnicalBaseline 之前合并重复的角色条目。",
                            ));
                        }
                    }
                    for field in capability.keys() {
                        if !matches!(
                            field.as_str(),
                            "purpose" | "durability" | "startupRequirement"
                        ) {
                            issues.push(issue(
                                "TECHNICAL_BASELINE_CAPABILITY_FIELD_UNKNOWN",
                                &format!("{capability_path}.{field}"),
                                "TechnicalBaseline 能力角色只能声明 purpose、durability 和 startupRequirement；consumer、failure、recovery 和 observability 字段不属于此合约。",
                            ));
                        }
                    }
                }
            }
        }
    }
}

const NEW_PROJECT_CORE_TRACKS: [&str; 6] = [
    "web",
    "app",
    "backend",
    "persistence",
    "dataAccess",
    "externalServices",
];
const NEW_PROJECT_TRACK_STATUSES: [&str; 4] =
    ["selected", "not_needed", "not_applicable", "user_custom"];

fn validate_new_project_candidate(
    candidate: &TechnicalBaselineCandidateAgentWritable,
    issues: &mut Vec<delivery_core::RepairIssue>,
) {
    if matches!(
        candidate.approval.r#type,
        TechnicalBaselineApprovalType::UserConfirmed
    ) && candidate
        .approval
        .confirmed_at
        .as_deref()
        .map(str::trim)
        .unwrap_or_default()
        .is_empty()
    {
        issues.push(issue(
            "NEW_PROJECT_BASELINE_CONFIRMATION_REQUIRED",
            "approval.confirmedAt",
            "具有 approval.type=user_confirmed 的新项目 TechnicalBaseline 必须包含实际的用户确认时间戳。",
        ));
    }
    if !matches!(
        candidate.status,
        TechnicalBaselineStatus::Confirmed | TechnicalBaselineStatus::NeedsUserConfirmation
    ) {
        issues.push(issue(
            "NEW_PROJECT_BASELINE_CONFIRMATION_REQUIRED",
            "status",
            "新项目 TechnicalBaseline 在规划之前必须被确认，或当用户确认仍待定时使用 status=needs_user_confirmation。",
        ));
    }
    if !new_project_stack_tracks_complete(&candidate.stack) {
        issues.push(issue(
            "NEW_PROJECT_BASELINE_TRACKS_INCOMPLETE",
            "stack.tracks",
            "新项目 TechnicalBaseline 的 stack.tracks 必须包含 web、app、backend、persistence、dataAccess 和 externalServices；每个轨道需要有效的 status 和非空 selection。选中的 externalServices 轨道必须提供结构化 providers 和能力角色。",
        ));
    }
    if new_project_web_selected(&candidate.stack)
        && !new_project_quality_automation_complete(&candidate.stack)
    {
        issues.push(issue(
            "NEW_PROJECT_QUALITY_AUTOMATION_INCOMPLETE",
            "stack.tracks.qualityAutomation",
            "新项目 Web 基线必须包含具有具体浏览器自动化技术栈的已选中 qualityAutomation 轨道。",
        ));
    }
    if let Some(compatibility_issue) = backend_data_access_compatibility_issue(&candidate.stack) {
        issues.push(compatibility_issue);
    }
}

fn backend_data_access_compatibility_issue(stack: &Value) -> Option<delivery_core::RepairIssue> {
    let backend = active_track_selection(stack, "backend")?;
    let data_access = active_track_selection(stack, "dataAccess")?;
    if technology_matches_any(data_access, PORTABLE_DATA_ACCESS_MATCHERS) {
        return None;
    }
    let backend_families = backend_runtime_families(backend);
    let data_access_families = data_access_runtime_families(data_access);
    if backend_families.is_empty()
        || data_access_families.is_empty()
        || !backend_families.is_disjoint(&data_access_families)
    {
        return None;
    }
    let catalog = reference_catalog::resolved_catalog();
    let compatible_options = catalog
        .backend_ecosystems()
        .iter()
        .filter(|ecosystem| backend_families.contains(&ecosystem.runtime_family))
        .flat_map(|ecosystem| ecosystem.data_access_options.iter().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>()
        .join(", ");
    Some(issue(
        "NEW_PROJECT_BACKEND_DATA_ACCESS_INCOMPATIBLE",
        "stack.tracks.dataAccess.selection",
        &format!(
            "backend 选择 '{backend}' 与 dataAccess 选择 '{data_access}' 属于不同的已知运行时生态系统。使用兼容选项如 {compatible_options}，或提供可在确认期间解释其关系的真正自定义数据访问选择。"
        ),
    ))
}

fn active_track_selection<'a>(stack: &'a Value, track: &str) -> Option<&'a str> {
    let track = stack.pointer(&format!("/tracks/{track}"))?.as_object()?;
    if !matches!(
        track.get("status").and_then(Value::as_str),
        Some("selected" | "user_custom")
    ) {
        return None;
    }
    track
        .get("selection")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|selection| !selection.is_empty())
}

fn backend_runtime_families(selection: &str) -> BTreeSet<String> {
    let catalog = reference_catalog::resolved_catalog();
    catalog
        .backend_ecosystems()
        .iter()
        .filter(|ecosystem| technology_matches_any(selection, &ecosystem.backend_matchers))
        .map(|ecosystem| ecosystem.runtime_family.clone())
        .collect()
}

fn data_access_runtime_families(selection: &str) -> BTreeSet<String> {
    let catalog = reference_catalog::resolved_catalog();
    catalog
        .backend_ecosystems()
        .iter()
        .filter(|ecosystem| technology_matches_any(selection, &ecosystem.data_access_matchers))
        .map(|ecosystem| ecosystem.runtime_family.clone())
        .collect()
}

fn technology_matches_any<S: AsRef<str>>(selection: &str, matchers: &[S]) -> bool {
    let selection = format!(" {} ", normalize_technology_phrase(selection));
    matchers.iter().any(|matcher| {
        let matcher = format!(" {} ", normalize_technology_phrase(matcher.as_ref()));
        selection.contains(&matcher)
    })
}

fn normalize_technology_phrase(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn new_project_stack_tracks_complete(stack: &Value) -> bool {
    let Some(tracks) = stack.get("tracks").and_then(Value::as_object) else {
        return false;
    };
    NEW_PROJECT_CORE_TRACKS.iter().all(|track| {
        let Some(value) = tracks.get(*track).and_then(Value::as_object) else {
            return false;
        };
        let status = value
            .get("status")
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or_default();
        let selection = value
            .get("selection")
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or_default();
        let track_valid = NEW_PROJECT_TRACK_STATUSES.contains(&status) && !selection.is_empty();
        track_valid
            && (*track != "externalServices" || external_services_track_complete(value, status))
    })
}

fn external_services_track_complete(track: &serde_json::Map<String, Value>, status: &str) -> bool {
    let Some(providers) = track.get("providers") else {
        return !matches!(status, "selected" | "user_custom");
    };
    let Some(providers) = providers.as_array() else {
        return false;
    };
    if !matches!(status, "selected" | "user_custom") {
        return providers.is_empty();
    }
    !providers.is_empty()
        && providers.iter().all(|provider| {
            let Some(provider) = provider.as_object() else {
                return false;
            };
            let provider_name = provider
                .get("provider")
                .and_then(Value::as_str)
                .map(str::trim)
                .unwrap_or_default();
            let Some(capabilities) = provider.get("capabilities").and_then(Value::as_array) else {
                return false;
            };
            !provider_name.is_empty()
                && !capabilities.is_empty()
                && capabilities.iter().all(|capability| {
                    let Some(capability) = capability.as_object() else {
                        return false;
                    };
                    let purpose = capability
                        .get("purpose")
                        .and_then(Value::as_str)
                        .unwrap_or_default();
                    let durability = capability
                        .get("durability")
                        .and_then(Value::as_str)
                        .unwrap_or_default();
                    let startup_requirement = capability
                        .get("startupRequirement")
                        .and_then(Value::as_str)
                        .unwrap_or_default();
                    matches!(
                        purpose,
                        "cache" | "session" | "queue" | "stream" | "lock_rate_limit"
                    ) && matches!(durability, "ephemeral" | "persistent")
                        && matches!(
                            startup_requirement,
                            "required" | "optional" | "not_applicable"
                        )
                })
        })
}

fn new_project_web_selected(stack: &Value) -> bool {
    stack
        .pointer("/tracks/web")
        .and_then(Value::as_object)
        .is_some_and(|track| {
            matches!(
                track.get("status").and_then(Value::as_str),
                Some("selected" | "user_custom")
            )
        })
}

fn new_project_quality_automation_complete(stack: &Value) -> bool {
    stack
        .pointer("/tracks/qualityAutomation")
        .and_then(Value::as_object)
        .is_some_and(|track| {
            matches!(
                track.get("status").and_then(Value::as_str),
                Some("selected" | "user_custom")
            ) && track
                .get("selection")
                .and_then(Value::as_str)
                .is_some_and(|selection| !selection.trim().is_empty())
        })
}

fn technical_baseline_decision_needs(
    project_kind: ProjectKind,
    baseline_exists: bool,
) -> Vec<String> {
    if matches!(project_kind, ProjectKind::NewProject) {
        return vec![
            "适用时的 web 客户端技术轨道".to_string(),
            "适用时的 app 客户端技术轨道".to_string(),
            "backend/服务技术轨道".to_string(),
            "数据库或持久化技术轨道".to_string(),
            "ORM 或数据访问技术轨道".to_string(),
            "仅当已确认需求需要时的外部服务".to_string(),
            "选中 Web 客户端时的浏览器质量自动化".to_string(),
        ];
    }
    if baseline_exists {
        return vec![
            "当前已确认范围是否显式添加新技术界面"
                .to_string(),
            "当前已确认范围是否显式替换先前的技术基线元素"
                .to_string(),
            "否则对于现有技术栈内的常规修复、修缮、优化或功能工作，原样复用先前的 TechnicalBaseline"
                .to_string(),
        ];
    }
    if matches!(project_kind, ProjectKind::Unknown) {
        return vec!["confirm_project_kind".to_string()];
    }
    vec![
        "当前仓库的运行时、语言、框架和包管理器证据".to_string(),
        "当前仓库的持久化或数据访问证据（如存在）".to_string(),
        "已确认需求的技术偏好（如果用户显式提供了）".to_string(),
    ]
}

fn technical_baseline_conflicts(
    previous: &TechnicalBaselineContract,
    candidate: &TechnicalBaselineCandidateAgentWritable,
) -> bool {
    previous.project_kind != candidate.project_kind
        || previous.scope != candidate.scope
        || !stable_stack_equivalent(&previous.stack, &candidate.stack)
        || previous.constraints != candidate.constraints
        || previous.security_profiles != candidate.security_profiles
}

fn stable_stack_equivalent(left: &Value, right: &Value) -> bool {
    normalize_stack_for_comparison(left) == normalize_stack_for_comparison(right)
}

#[derive(Debug, Default, PartialEq, Eq)]
struct StableStack {
    runtimes: BTreeSet<String>,
    languages: BTreeSet<String>,
    frameworks: BTreeSet<String>,
    package_managers: BTreeSet<String>,
    databases: BTreeSet<String>,
    external_services: BTreeSet<String>,
    tracks: BTreeSet<String>,
}

fn normalize_stack_for_comparison(stack: &Value) -> StableStack {
    StableStack {
        runtimes: values_for_keys(stack, &["runtime", "runtimes", "runtimeKind"]),
        languages: values_for_keys(stack, &["language", "languages"]),
        frameworks: values_for_keys(
            stack,
            &[
                "framework",
                "frameworks",
                "frontendFramework",
                "backendFramework",
            ],
        ),
        package_managers: values_for_keys(stack, &["packageManager", "packageManagers"]),
        databases: values_for_keys(stack, &["database", "databases", "databaseProvider"]),
        external_services: external_service_values(stack),
        tracks: stack_track_selections_for_comparison(stack),
    }
}

fn external_service_values(stack: &Value) -> BTreeSet<String> {
    let mut values = BTreeSet::new();
    let Some(track) = stack.pointer("/tracks/externalServices") else {
        return values;
    };
    if let Some(providers) = track.get("providers").and_then(Value::as_array) {
        for provider in providers {
            let provider_name = provider
                .get("provider")
                .and_then(Value::as_str)
                .map(normalize_stack_token)
                .unwrap_or_default();
            for capability in provider
                .get("capabilities")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                let purpose = capability
                    .get("purpose")
                    .and_then(Value::as_str)
                    .map(normalize_stack_token)
                    .unwrap_or_default();
                let durability = capability
                    .get("durability")
                    .and_then(Value::as_str)
                    .map(normalize_stack_token)
                    .unwrap_or_default();
                let startup = capability
                    .get("startupRequirement")
                    .and_then(Value::as_str)
                    .map(normalize_stack_token)
                    .unwrap_or_default();
                values.insert(format!("{provider_name}:{purpose}:{durability}:{startup}"));
            }
        }
    } else if let Some(selection) = track.get("selection").and_then(Value::as_str) {
        let normalized = normalize_stack_token(selection);
        if !normalized.is_empty() {
            values.insert(normalized);
        }
    }
    values
}

fn values_for_keys(value: &Value, keys: &[&str]) -> BTreeSet<String> {
    let mut values = BTreeSet::new();
    for key in keys {
        collect_stack_value(value.get(*key).unwrap_or(&Value::Null), &mut values);
    }
    values
}

fn collect_stack_value(value: &Value, output: &mut BTreeSet<String>) {
    if let Some(text) = value.as_str() {
        let normalized = normalize_stack_token(text);
        if !normalized.is_empty() {
            output.insert(normalized);
        }
        return;
    }
    if let Some(items) = value.as_array() {
        for item in items {
            collect_stack_value(item, output);
        }
    }
}

fn stack_track_selections_for_comparison(stack: &Value) -> BTreeSet<String> {
    let Some(tracks) = stack.get("tracks").and_then(Value::as_object) else {
        return BTreeSet::new();
    };
    tracks
        .iter()
        .filter_map(|(track_name, track)| {
            let status = track
                .get("status")
                .and_then(Value::as_str)
                .map(normalize_stack_token)
                .unwrap_or_default();
            let selection = track
                .get("selection")
                .and_then(Value::as_str)
                .map(normalize_stack_token)
                .unwrap_or_default();
            if status.is_empty() && selection.is_empty() {
                None
            } else {
                Some(format!(
                    "{}:{}:{}",
                    normalize_stack_token(track_name),
                    status,
                    selection
                ))
            }
        })
        .collect()
}

fn repo_signals_conflict_with_previous(
    project_root: &Path,
    request_ref: &str,
    previous: &TechnicalBaselineContract,
) -> Result<Vec<String>, state::store::StateError> {
    let fields = state::read_request_fields(ReadRequestFieldsInput {
        project_root: project_root.to_string_lossy().to_string(),
        request_ref: request_ref.to_string(),
        fields: vec![
            "repoEvidence.signals.packageManagers".to_string(),
            "repoEvidence.signals.frameworks".to_string(),
            "repoEvidence.signals.languages".to_string(),
        ],
    })?;
    let stack = normalize_stack_for_comparison(&previous.stack);
    let mut conflicts = Vec::new();
    push_signal_conflict(
        &mut conflicts,
        "packageManagers",
        &stack.package_managers,
        field_string_set(&fields.fields, "repoEvidence.signals.packageManagers"),
    );
    push_signal_conflict(
        &mut conflicts,
        "frameworks",
        &stack.frameworks,
        field_string_set(&fields.fields, "repoEvidence.signals.frameworks"),
    );
    push_signal_conflict(
        &mut conflicts,
        "languages",
        &stack.languages,
        field_string_set(&fields.fields, "repoEvidence.signals.languages"),
    );
    if stack.runtimes.contains("node") {
        let mut repo_tokens =
            field_string_set(&fields.fields, "repoEvidence.signals.packageManagers");
        repo_tokens.extend(field_string_set(
            &fields.fields,
            "repoEvidence.signals.languages",
        ));
        let has_node_signal = ["npm", "pnpm", "yarn", "bun", "typescript", "javascript"]
            .iter()
            .any(|token| repo_tokens.contains(*token));
        let has_other_runtime_signal = ["maven", "gradle", "java", "python", "go", "rust"]
            .iter()
            .any(|token| repo_tokens.contains(*token));
        if !has_node_signal && has_other_runtime_signal {
            conflicts.push("runtime".to_string());
        }
    }
    Ok(conflicts)
}

fn field_string_set(
    fields: &std::collections::BTreeMap<String, delivery_core::FieldReadResult>,
    field: &str,
) -> BTreeSet<String> {
    fields
        .get(field)
        .and_then(|result| result.value.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(normalize_stack_token)
                .filter(|value| !value.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

fn push_signal_conflict(
    conflicts: &mut Vec<String>,
    label: &str,
    baseline_values: &BTreeSet<String>,
    signal_values: BTreeSet<String>,
) {
    if baseline_values.is_empty() || signal_values.is_empty() {
        return;
    }
    if !baseline_values
        .iter()
        .any(|value| signal_values.contains(value))
    {
        conflicts.push(label.to_string());
    }
}

fn normalize_stack_token(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .trim_end_matches(".js")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
}

fn infer_project_kind_for_baseline(
    project_root: &Path,
    delivery: &DeliveryIndex,
    phase_id: &str,
    has_previous_baseline: bool,
) -> ProjectKind {
    if has_previous_baseline || is_after_first_delivery_phase(delivery, phase_id) {
        return ProjectKind::ExistingProject;
    }
    infer_project_kind_from_repo(project_root)
}

fn is_after_first_delivery_phase(delivery: &DeliveryIndex, phase_id: &str) -> bool {
    delivery
        .phases
        .iter()
        .position(|phase| phase.phase_id == phase_id)
        .map(|index| index > 0)
        .unwrap_or(false)
}

fn infer_project_kind_from_repo(project_root: &Path) -> ProjectKind {
    let markers = [
        "package.json",
        "tsconfig.json",
        "pom.xml",
        "build.gradle",
        "pyproject.toml",
        "go.mod",
        "Cargo.toml",
        "requirements.txt",
        "app",
        "src",
        "frontend",
        "backend",
    ];
    if markers
        .iter()
        .any(|marker| project_root.join(marker).exists())
    {
        ProjectKind::ExistingProject
    } else {
        ProjectKind::NewProject
    }
}

fn read_brainstorm_contract(
    project_root: &Path,
    relative_ref: &str,
) -> Result<BrainstormContract, state::store::StateError> {
    let absolute = from_project_relative(project_root, relative_ref)?;
    state::store::read_json(&absolute)
}

fn ensure_latest_request(
    project_root: &str,
    delivery_id: &str,
    phase_id: &str,
    request_ref: &str,
    latest_ref_key: &str,
) -> Result<Option<LoomMcpActionResult>, state::store::StateError> {
    let store = FileTransitionStore;
    let delivery = store
        .load_delivery_index(project_root, delivery_id)
        .map_err(to_state_error)?;
    if delivery.active_phase_id != phase_id {
        return Ok(Some(stale_failure(
            project_root,
            "TechnicalBaseline 提交必须绑定到活动阶段。".to_string(),
        )));
    }
    let Some(phase) = delivery
        .phases
        .iter()
        .find(|phase| phase.phase_id == phase_id)
    else {
        return Ok(Some(stale_failure(
            project_root,
            format!("交付 {} 缺少阶段 {}", delivery_id, phase_id),
        )));
    };
    if phase.latest_refs.get(latest_ref_key).map(String::as_str) != Some(request_ref) {
        return Ok(Some(stale_failure(
            project_root,
            "TechnicalBaseline 提交必须使用活动阶段的最新 requestRef。".to_string(),
        )));
    }
    Ok(None)
}

fn technical_baseline_user_gate(
    input: &FileSubmitInput,
    authorized: &AuthorizedWriteSet,
    prompt: String,
    gate_id: String,
) -> LoomMcpActionResult {
    technical_baseline_confirmation_gate(
        &input.project_root,
        &input.request_ref,
        authorized.delivery_id.as_deref().unwrap_or_default(),
        authorized.phase_id.as_deref().unwrap_or_default(),
        prompt,
        gate_id,
    )
}

fn technical_baseline_recommendation_gate(
    project_root: &str,
    request_ref: &str,
    delivery_id: &str,
    phase_id: &str,
) -> LoomMcpActionResult {
    technical_baseline_confirmation_gate(
        project_root,
        request_ref,
        delivery_id,
        phase_id,
        "向用户呈现推荐的技术基线和完整的可调整选项矩阵。等待明确确认或调整；确认后写入并提交同一 TechnicalBaseline 候选请求。".to_string(),
        "new_project_baseline_confirmation".to_string(),
    )
}

fn technical_baseline_confirmation_gate(
    project_root: &str,
    request_ref: &str,
    delivery_id: &str,
    phase_id: &str,
    prompt: String,
    gate_id: String,
) -> LoomMcpActionResult {
    LoomMcpActionResult::UserGate(LoomMcpUserGateResult::new(
        project_root.to_string(),
        prompt,
        vec!["reply_in_chat".to_string()],
        Some(request_ref.to_string()),
        Some(delivery_id.to_string()),
        Some(phase_id.to_string()),
        Some(technical_baseline_gate_details(&gate_id)),
    ))
}

fn technical_baseline_gate_details(gate_id: &str) -> Value {
    json!({
        "gateId": gate_id,
        "kind": "technical_baseline_confirmation",
        "responseContract": {
            "responseId": "technical_baseline_confirmation",
            "mode": "single_user_message",
            "maxMessages": 1,
            "doNotRepeatInFinal": true,
            "requiredSections": [
                "recommendation_basis",
                "recommended_final_baseline",
                "adjustable_technology_range",
                "confirmation_or_adjustment_prompt"
            ]
        }
    })
}

fn repairable(
    input: &FileSubmitInput,
    authorized: &AuthorizedWriteSet,
    target_file: String,
    issues: Vec<delivery_core::RepairIssue>,
) -> LoomMcpActionResult {
    LoomMcpActionResult::RepairableError(LoomMcpRepairableErrorResult {
        project_root: input.project_root.clone(),
        stop_allowed: false,
        target_file,
        target_ids: authorized
            .targets
            .iter()
            .map(|target| target.target_id.clone())
            .collect(),
        issues,
        resubmit_tool: "loom.technicalBaselineAcceptFile".to_string(),
        fix_scope: Some("technical_baseline_candidate_only".to_string()),
        read_groups: authorized.read_groups.clone(),
        agent_instruction: delivery_core::repairable_error_agent_instruction(
            "loom.technicalBaselineAcceptFile",
        ),
    })
}

fn stale_failure(project_root: &str, message: String) -> LoomMcpActionResult {
    LoomMcpActionResult::Failed(LoomMcpFailureResult {
        project_root: project_root.to_string(),
        error: LoomMcpFailure {
            code: "STALE_TECHNICAL_BASELINE_REQUEST".to_string(),
            message,
            target_batch: Some(8),
            domain: Some("planning".to_string()),
            route_action: Some("technical_baseline_accept".to_string()),
            recovery_tool: Some("loom.continue".to_string()),
        },
    })
}

fn issue(code: &str, field_path: &str, message: &str) -> delivery_core::RepairIssue {
    delivery_core::RepairIssue {
        code: code.to_string(),
        message: message.to_string(),
        target_id: Some("candidate".to_string()),
        field_path: Some(field_path.to_string()),
    }
}

fn to_state_error(error: delivery_core::LoomCoreError) -> state::store::StateError {
    state::store::StateError::StateCorrupted(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stack_with_backend_and_data_access(backend: &str, data_access: &str) -> Value {
        json!({
            "tracks": {
                "backend": {
                    "status": "selected",
                    "selection": backend
                },
                "dataAccess": {
                    "status": "selected",
                    "selection": data_access
                }
            }
        })
    }

    #[test]
    fn backend_ecosystem_guidance_groups_spring_with_data_access_options() {
        let guidance = backend_ecosystem_guidance();
        let spring = guidance["bundles"]
            .as_array()
            .unwrap()
            .iter()
            .find(|bundle| bundle["ecosystemId"] == "jvm_spring")
            .expect("Spring ecosystem bundle");

        assert!(spring["backendOptions"]
            .as_array()
            .unwrap()
            .contains(&json!("Java + Spring Boot")));
        assert!(spring["recommendedDataAccessOptions"]
            .as_array()
            .unwrap()
            .contains(&json!("Spring Data JPA")));
        assert!(spring["recommendedDataAccessOptions"]
            .as_array()
            .unwrap()
            .contains(&json!("MyBatis Plus")));
        assert!(guidance["renderingRule"]
            .as_str()
            .unwrap()
            .contains("不得呈现独立的扁平 dataAccess 选项列表"));
        assert!(guidance["optionEnumerationRule"]
            .as_str()
            .unwrap()
            .contains("每个 recommendedDataAccessOptions 条目"));
    }

    #[test]
    fn known_cross_runtime_data_access_is_rejected() {
        let stack = stack_with_backend_and_data_access("Java + Spring Boot", "Prisma");

        let issue = backend_data_access_compatibility_issue(&stack).expect("compatibility issue");

        assert_eq!(issue.code, "NEW_PROJECT_BACKEND_DATA_ACCESS_INCOMPATIBLE");
        assert_eq!(
            issue.field_path.as_deref(),
            Some("stack.tracks.dataAccess.selection")
        );
        assert!(issue.message.contains("Spring Data JPA"));
    }

    #[test]
    fn compatible_portable_and_custom_data_access_remain_open() {
        for (backend, data_access) in [
            ("Java + Spring Boot", "Spring Data JPA"),
            ("Python + Django", "SQLAlchemy"),
            ("Node.js + Fastify", "Raw SQL / lightweight wrapper"),
            ("Java + Spring Boot", "Custom JDBC Adapter"),
        ] {
            let stack = stack_with_backend_and_data_access(backend, data_access);
            assert!(
                backend_data_access_compatibility_issue(&stack).is_none(),
                "{backend} + {data_access} should remain valid"
            );
        }
    }

    fn candidate_without_security_profile(
        status: &str,
        approval_type: &str,
    ) -> TechnicalBaselineCandidateAgentWritable {
        serde_json::from_value(json!({
            "status": status,
            "source": "agent_recommended_for_new_project",
            "projectKind": "new_project",
            "scope": "project",
            "stack": {},
            "securityProfiles": [],
            "constraints": [],
            "evidence": [],
            "approval": {"type": approval_type},
            "confidence": "unknown",
            "reasoningSummary": [],
            "alternatives": []
        }))
        .expect("candidate shape")
    }

    fn candidate_with_redis_session() -> TechnicalBaselineCandidateAgentWritable {
        let mut candidate = candidate_without_security_profile("confirmed", "user_confirmed");
        candidate.stack = json!({
            "tracks": {
                "externalServices": {
                    "status": "selected",
                    "selection": "Redis",
                    "providers": [{
                        "provider": "Redis",
                        "capabilities": [{
                            "purpose": "session",
                            "durability": "persistent",
                            "startupRequirement": "required"
                        }]
                    }]
                }
            }
        });
        candidate
    }

    #[test]
    fn jwt_guidance_is_dormant_and_has_no_greenfield_default() {
        let guidance = security_profile_guidance(
            &SecurityRequirement {
                applies: SecurityRequirementApplicability::Required,
                client_trust_models: vec![],
                source_refs: vec![],
                rationale: "test".to_string(),
            },
            false,
            None,
        );

        assert_eq!(guidance["capabilityState"], json!("dormant"));
        assert!(guidance.get("newProjectDefault").is_none());
        assert!(guidance.get("algorithmPolicy").is_none());
        assert!(guidance["activationRule"]
            .as_str()
            .expect("activation rule")
            .contains("显式选入"));
    }

    #[test]
    fn pending_protected_baseline_can_wait_for_explicit_security_selection() {
        let candidate = candidate_without_security_profile("needs_user_confirmation", "none");
        let requirement = SecurityRequirement {
            applies: SecurityRequirementApplicability::Required,
            client_trust_models: vec![contracts::ClientTrustModel::ExternalApi],
            source_refs: vec![],
            rationale: "External clients need an explicit authentication decision.".to_string(),
        };
        let mut issues = Vec::new();

        validate_security_profiles(&candidate, &requirement, &mut issues);

        assert!(
            issues.is_empty(),
            "pending security selection should reach the user gate"
        );
    }

    #[test]
    fn redis_session_and_same_origin_browser_derive_server_session_without_jwt() {
        let mut candidate = candidate_with_redis_session();
        let requirement = SecurityRequirement {
            applies: SecurityRequirementApplicability::Required,
            client_trust_models: vec![ClientTrustModel::SameOriginBrowser],
            source_refs: vec!["req-001".to_string()],
            rationale: "Roles are resolved from the login session.".to_string(),
        };

        derive_server_session_profile(&mut candidate, &requirement);

        assert_eq!(candidate.security_profiles.len(), 1);
        let profile = &candidate.security_profiles[0];
        assert_eq!(profile.mechanism, SecurityMechanism::ServerSession);
        assert_eq!(profile.transport, SecurityTransport::SameOriginCookie);
        assert_eq!(profile.key_source, SecurityKeySource::NotApplicable);
        assert!(profile.algorithm.is_none());
        assert!(profile.claims.is_empty());
        assert_eq!(candidate.status, TechnicalBaselineStatus::Confirmed);

        let mut issues = Vec::new();
        validate_security_profiles(&candidate, &requirement, &mut issues);
        assert!(
            issues.is_empty(),
            "derived session profile is valid: {issues:?}"
        );
    }

    #[test]
    fn normalizes_capability_role_map_to_canonical_array() {
        let mut stack = json!({
            "tracks": {
                "externalServices": {
                    "status": "selected",
                    "providers": [{
                        "provider": "Redis",
                        "capabilities": {
                            "session": {
                                "durability": "persistent",
                                "startupRequirement": "required"
                            }
                        }
                    }]
                }
            }
        });

        normalize_external_service_capability_shape(&mut stack);

        assert_eq!(
            stack["tracks"]["externalServices"]["providers"][0]["capabilities"],
            json!([{
                "purpose": "session",
                "durability": "persistent",
                "startupRequirement": "required"
            }])
        );
    }

    #[test]
    fn redis_session_does_not_derive_for_external_api_clients() {
        let mut candidate = candidate_with_redis_session();
        let requirement = SecurityRequirement {
            applies: SecurityRequirementApplicability::Required,
            client_trust_models: vec![ClientTrustModel::ExternalApi],
            source_refs: vec![],
            rationale: "External clients need an explicit token contract.".to_string(),
        };

        derive_server_session_profile(&mut candidate, &requirement);

        assert!(candidate.security_profiles.is_empty());
    }

    #[test]
    fn confirmed_protected_baseline_without_profile_waits_for_security_gate() {
        let candidate = candidate_without_security_profile("confirmed", "user_confirmed");
        let requirement = SecurityRequirement {
            applies: SecurityRequirementApplicability::Required,
            client_trust_models: vec![contracts::ClientTrustModel::ServiceToService],
            source_refs: vec![],
            rationale: "Service clients need an explicit authentication decision.".to_string(),
        };
        let mut issues = Vec::new();

        validate_security_profiles(&candidate, &requirement, &mut issues);

        assert!(
            issues.is_empty(),
            "the submit route should return the security user gate"
        );
    }

    #[test]
    fn deferred_security_does_not_require_an_active_profile() {
        let candidate = candidate_without_security_profile("confirmed", "user_confirmed");
        let requirement = SecurityRequirement {
            applies: SecurityRequirementApplicability::DeferredWithRisk,
            client_trust_models: vec![contracts::ClientTrustModel::SameOriginBrowser],
            source_refs: vec![],
            rationale: "Authentication is deferred to a later phase with an explicit risk."
                .to_string(),
        };
        let mut issues = Vec::new();

        validate_security_profiles(&candidate, &requirement, &mut issues);

        assert!(
            issues.is_empty(),
            "deferred security should not activate JWT"
        );
    }
}
