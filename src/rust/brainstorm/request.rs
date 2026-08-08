use std::path::Path;

use contracts::{
    BrainstormCandidateAgentWritable, ClarificationBlockName, UserFacingLanguageConstraint,
};
use delivery_core::{
    read_selectors_value_from_paths, ArtifactKind, RouteAction, RouteActionKind, WriteMode,
};
use schemars::schema_for;
use serde_json::{json, Map, Value};
use state::paths::to_project_relative;

use crate::{gate::required_blocks, paths::brainstorm_agent_candidate_file};

pub fn build_brainstorm_request_root(
    _project_root: &Path,
    request_id: &str,
    delivery_id: &str,
    phase_id: &str,
    brainstorm_run_id: &str,
    user_facing_language: &UserFacingLanguageConstraint,
    context_refs: Value,
) -> serde_json::Value {
    build_brainstorm_clarification_request_root(
        request_id,
        delivery_id,
        phase_id,
        brainstorm_run_id,
        user_facing_language,
        context_refs,
        ClarificationBlockName::PhaseScope,
    )
}

pub fn build_brainstorm_clarification_request_root(
    _request_id: &str,
    delivery_id: &str,
    phase_id: &str,
    brainstorm_run_id: &str,
    user_facing_language: &UserFacingLanguageConstraint,
    context_refs: Value,
    current_block: ClarificationBlockName,
) -> serde_json::Value {
    let (rule_key, rules, rule_group_fields) = block_rules(&current_block);
    let mut rules_object = Map::new();
    rules_object.insert(rule_key.to_string(), rules);
    if current_block != ClarificationBlockName::FinalSummary {
        rules_object.insert(
            "requirementSemanticGrounding".to_string(),
            json!({ "compactRules": requirement_semantic_compact_rules() }),
        );
    }
    let mut groups = vec![json!({
        "groupId": "conversation_protocol",
        "required": true,
        "purpose": "在向用户呈现任何内容之前，阅读当前 Brainstorm 块的协议。",
        "whenToRead": "在此 Brainstorm 块开始时阅读。",
        "selectors": read_selectors_value_from_paths([
            "userFacingLanguage",
            "clarificationConversationProtocol.currentBlock",
            "clarificationConversationProtocol.userVisibleBlockTitle",
            "clarificationConversationProtocol.userFacingLanguageRule",
            "clarificationConversationProtocol.currentTurnAnswerRule",
            "clarificationConversationProtocol.blockRule",
            "clarificationConversationProtocol.confirmToolRule"
        ])
    })];
    if current_block != ClarificationBlockName::FinalSummary {
        groups.push(json!({
            "groupId": "requirement_context",
            "required": true,
            "purpose": "为当前 Brainstorm 块阅读紧凑的源元数据和需求提示。",
            "whenToRead": "在形成当前块响应之前阅读。",
            "selectors": read_selectors_value_from_paths([
                "requirementContext.sourceItems",
                "keywordHints.compact"
            ])
        }));
        groups.push(json!({
            "groupId": "requirement_full_text",
            "required": false,
            "purpose": "仅在紧凑上下文和 request-scoped knowledge 不足以判断时，才阅读完整的规范化需求文本。",
            "whenToRead": "仅在当前块需要时按需阅读。",
            "selectors": read_selectors_value_from_paths([
                "requirementContext.normalizedText"
            ])
        }));
    }
    groups.push(json!({
        "groupId": "current_block_rules",
        "required": true,
        "purpose": "仅阅读当前 Brainstorm 确认块的规则。",
        "whenToRead": "在呈现当前块之前阅读。",
        "selectors": read_selectors_value_from_paths(rule_group_fields)
    }));
    if current_block != ClarificationBlockName::PhaseScope {
        groups.push(json!({
            "groupId": "confirmed_clarification_state",
            "required": true,
            "purpose": "将已由用户确认的 Brainstorm 块作为当前块的权威依据来阅读。",
            "whenToRead": "在形成当前块响应之前阅读。",
            "selectors": read_selectors_value_from_paths([
                "confirmedClarificationState.blocks",
                "confirmedClarificationState.finalSummaryConfirmed"
            ])
        }));
    }
    if current_block != ClarificationBlockName::FinalSummary {
        groups.push(json!({
            "groupId": "knowledge_context_plan",
            "required": true,
            "purpose": "阅读 request-scoped knowledge 查询计划，并在呈现当前 Brainstorm 块之前调用 loom.knowledgeBrainstormContext。",
            "whenToRead": "在形成当前块响应之前阅读；对每个列出的 executionOrder 步骤调用 loom.knowledgeBrainstormContext，对 repeatMode 步骤运行所需的查询集。",
            "selectors": read_selectors_value_from_paths(vec![
                "knowledgeQueryPlan.sharedRules".to_string(),
                "knowledgeQueryPlan.toolContract".to_string(),
                format!("knowledgeQueryPlan.blocks.{}.executionOrder", block_id(&current_block))
            ])
        }));
    }
    groups.push(json!({
        "groupId": "block_confirmation_contract",
        "required": true,
        "purpose": "在用户可见确认此块后，阅读当前块确认提交的形状。",
        "whenToRead": "仅在用户在聊天中确认当前块后阅读。",
        "selectors": read_selectors_value_from_paths([
            "blockConfirmationContract.tool",
            "blockConfirmationContract.currentBlock",
            "blockConfirmationContract.summary",
            "blockConfirmationContract.confirmedDataShape"
        ])
    }));

    json!({
        "schemaVersion": "1.0",
        "requestType": "brainstorm_clarification_block",
        "deliveryId": delivery_id,
        "phaseId": phase_id,
        "brainstormRunId": brainstorm_run_id,
        "userFacingLanguage": user_facing_language,
        "contextRefs": context_refs,
        "clarificationConversationProtocol": {
            "mode": "progressive_blocks",
            "currentBlock": current_block,
            "requiredBlocks": required_blocks(),
            "userVisibleBlockTitle": user_visible_block_title(&current_block),
            "userFacingLanguageRule": user_facing_language.rule,
            "currentTurnAnswerRule": current_turn_answer_rule(&current_block),
            "blockRule": block_rule(&current_block),
            "confirmToolRule": "在用户可见确认后，调用 loom.brainstormConfirmBlock，传入此 requestRef、currentBlock、简洁的用户可见 summary 和当前块的 confirmedData。不要在澄清块中写入最终的 Brainstorm candidate。"
        },
        "knowledgeQueryPlan": knowledge_query_plan_for_block(&current_block),
        "rules": Value::Object(rules_object),
        "blockConfirmationContract": {
            "tool": "loom.brainstormConfirmBlock",
            "currentBlock": current_block,
            "summary": "用户对此块确认内容的简洁用户可见摘要。",
            "confirmedDataShape": block_confirmed_data_shape(&current_block)
        },
        "requestReadPlan": {
            "groups": groups
        }
    })
}

pub fn build_brainstorm_candidate_write_request_root(
    project_root: &Path,
    request_id: &str,
    delivery_id: &str,
    phase_id: &str,
    brainstorm_run_id: &str,
    user_facing_language: &UserFacingLanguageConstraint,
    context_refs: Value,
) -> serde_json::Value {
    let candidate_file = to_project_relative(
        project_root,
        &brainstorm_agent_candidate_file(project_root, request_id),
    )
    .unwrap_or_else(|_| format!(".loom/agent-writable/{request_id}/brainstorm-candidate.json"));
    let schema_shape = serde_json::to_value(schema_for!(BrainstormCandidateAgentWritable))
        .unwrap_or_else(|_| json!({ "type": "object" }));

    json!({
        "schemaVersion": "1.0",
        "requestType": "brainstorm_candidate_write",
        "deliveryId": delivery_id,
        "phaseId": phase_id,
        "brainstormRunId": brainstorm_run_id,
        "userFacingLanguage": user_facing_language,
        "contextRefs": context_refs,
        "rules": {
            "candidateWrite": candidate_write_rules(),
            "requirementSemanticGrounding": {
                "compactRules": requirement_semantic_compact_rules()
            }
        },
        "enumRefs": enum_refs(),
        "outputContract": {
            "artifactKind": ArtifactKind::BrainstormCandidate,
            "writeMode": WriteMode::SingleJson,
            "submitTool": "loom.brainstormAcceptFile",
            "writeTargets": [{
                "targetId": "candidate",
                "path": candidate_file,
                "required": true,
                "description": "在 final_summary 确认后写入 Brainstorm candidate JSON。"
            }],
            "resultTemplate": candidate_result_template(phase_id),
            "schemaShape": schema_shape,
            "schemaProjection": schema_projection()
        },
        "postSubmit": {
            "nextAction": RouteAction {
                kind: RouteActionKind::TechnicalBaselineRequest,
                source: "brainstorm_accept".to_string(),
                reason: "brainstorm_confirmed".to_string(),
                prompt: None,
                accepted_responses: vec![],
                request_ref: None,
                details: None,
                target_phase_id: None
            }
        },
        "requestReadPlan": {
            "groups": [
                {
                    "groupId": "confirmed_clarification_state",
                    "required": true,
                    "purpose": "阅读必须在 candidate 中结构化保留的已确认 Brainstorm 块。",
                    "whenToRead": "在写入 Brainstorm candidate 之前阅读。",
                    "selectors": read_selectors_value_from_paths([
                        "confirmedClarificationState.blocks",
                        "confirmedClarificationState.finalSummaryConfirmed"
                    ])
                },
                {
                    "groupId": "source_ref_registry",
                    "required": true,
                    "purpose": "仅阅读合法的 requirement source ids 用于 candidate sourceRefs；不要从此 registry 重新解读需求。",
                    "whenToRead": "在填充 candidate sourceRefs 之前阅读。",
                    "selectors": read_selectors_value_from_paths([
                        "sourceRefRegistry.sources"
                    ])
                },
                {
                    "groupId": "candidate_write_contract",
                    "required": true,
                    "purpose": "阅读最终 Brainstorm candidate 的紧凑写入契约。",
                    "whenToRead": "在写入 Brainstorm candidate 之前立即阅读。",
                    "selectors": read_selectors_value_from_paths([
                        "outputContract.writeTargets",
                        "outputContract.submitTool",
                        "outputContract.resultTemplate",
                        "outputContract.schemaProjection",
                        "enumRefs.complexity",
                        "enumRefs.scopeSource",
                        "enumRefs.acceptancePriority",
                        "enumRefs.phaseStatus",
                        "enumRefs.phasePlanCurrentStatus",
                        "enumRefs.nextPhasePreviewKind",
                        "enumRefs.conceptGroundingMode",
                        "enumRefs.conceptPhaseRelevance",
                        "enumRefs.conceptPriority",
                        "enumRefs.conceptRiskFactor",
                        "enumRefs.glossaryUpdateOperation",
                        "enumRefs.clarificationBlockName",
                        "enumRefs.clarificationMode",
                        "enumRefs.frontendExperienceLevel",
                        "enumRefs.frontendTargetSelectionMode",
                        "enumRefs.frontendActionEntryPoint",
                        "enumRefs.frontendResultObservationMode",
                        "enumRefs.frontendInteractionState",
                        "enumRefs.securityRequirementApplicability",
                        "enumRefs.clientTrustModel",
                        "rules.candidateWrite"
                    ])
                }
            ]
        }
    })
}

fn schema_projection() -> Value {
    json!({
        "requiredTopLevelFields": [
            "requestSummary",
            "scope",
            "roadmap",
            "phasePlan",
            "acceptance",
            "securityRequirement"
        ],
        "phaseScopeFields": [
            "scope.included",
            "scope.excluded",
            "scope.deferred",
            "scope.assumptions",
            "roadmap.phases",
            "phasePlan.current.title",
            "phasePlan.current.goal",
            "phasePlan.current.scopeRefs",
            "phasePlan.current.acceptanceRefs",
            "phasePlan.current.status",
            "phasePlan.nextPhasePreview"
        ],
        "securityRequirementFields": [
            "securityRequirement.applies",
            "securityRequirement.clientTrustModels",
            "securityRequirement.sourceRefs",
            "securityRequirement.rationale"
        ],
        "conceptGroundingFields": [
            "acceptance",
            "domainModel.businessFlows",
            "conceptGrounding",
            "conceptConfirmation"
        ],
        "frontendExperienceFields": [
            "frontendExperience.required",
            "frontendExperience.kind",
            "frontendExperience.experienceLevel",
            "frontendExperience.audiences",
            "frontendExperience.surfaces",
            "frontendExperience.dataViews",
            "frontendExperience.actions",
            "frontendExperience.operationPaths",
            "frontendExperience.mustNot",
            "frontendExperience.confirmationSummary"
        ],
        "enumFields": {
            "requestSummary.complexity": "enumRefs.complexity",
            "scope.included[].source": "enumRefs.scopeSource",
            "scope.excluded[].source": "enumRefs.scopeSource",
            "scope.deferred[].source": "enumRefs.scopeSource",
            "acceptance[].priority": "enumRefs.acceptancePriority",
            "roadmap.phases[].status": "enumRefs.phaseStatus",
            "phasePlan.current.status": "enumRefs.phasePlanCurrentStatus",
            "phasePlan.nextPhasePreview.kind": "enumRefs.nextPhasePreviewKind",
            "conceptGrounding.deliveryConceptGlossary.mode": "enumRefs.conceptGroundingMode",
            "conceptGrounding.deliveryConceptGlossary.concepts[].phaseRelevance": "enumRefs.conceptPhaseRelevance",
            "conceptGrounding.deliveryConceptGlossary.concepts[].priority": "enumRefs.conceptPriority",
            "conceptGrounding.deliveryConceptGlossary.concepts[].riskFactors[]": "enumRefs.conceptRiskFactor",
            "conceptGrounding.phaseConceptGrounding.mode": "enumRefs.conceptGroundingMode",
            "conceptGrounding.phaseConceptGrounding.concepts[].phaseRelevance": "enumRefs.conceptPhaseRelevance",
            "conceptGrounding.phaseConceptGrounding.concepts[].priority": "enumRefs.conceptPriority",
            "conceptGrounding.phaseConceptGrounding.concepts[].riskFactors[]": "enumRefs.conceptRiskFactor",
            "conceptGrounding.glossaryUpdates[].operation": "enumRefs.glossaryUpdateOperation",
            "frontendExperience.experienceLevel": "enumRefs.frontendExperienceLevel",
            "frontendExperience.dataViews[].selectionMode": "enumRefs.frontendTargetSelectionMode",
            "frontendExperience.actions[].entryPoint": "enumRefs.frontendActionEntryPoint",
            "frontendExperience.actions[].resultObservation[]": "enumRefs.frontendResultObservationMode",
            "frontendExperience.operationPaths[].selectionMode": "enumRefs.frontendTargetSelectionMode",
            "frontendExperience.operationPaths[].requiredStates[]": "enumRefs.frontendInteractionState"
        },
        "objectShapeRules": {
            "conceptGrounding.glossaryUpdates[]": "此数组是可选的。当 delivery glossary 无变更时使用 []。Loom 在 accept 时生成 updateId；不要写入 updateId。每个项写入 operation 和 reason。add 使用精确的 phaseConceptGrounding.concepts[] 对象形状写入 concept；replace 写入 conceptRef 加上相同的 concept 对象形状；remove 写入 conceptRef 且不写 concept。不要将 term、normalizedName 或 explanation 等 concept 字段扁平化到 update 项上。",
            "frontendExperience.actions[].resultObservation[]": "仅使用 frontendResultObservationMode 值：list_refresh、detail_refresh、inline_status_update、response_message、not_applicable。不要在此处使用 frontendInteractionState 值：loading、success、error、empty、business_blocking。empty 不是结果观察；它是页面/列表状态。",
            "frontendExperience.operationPaths[].requiredStates[]": "仅使用 frontendInteractionState 值：loading、success、error、empty、business_blocking。不要在此处使用 frontendResultObservationMode 值：list_refresh、detail_refresh、inline_status_update、response_message、not_applicable。empty 仅在此处对空列表/页面状态有效。"
        },
        "notes": [
            "机器拥有的 ids、request binding、确认元数据、accepted status 和 handoff routing 由 Loom 在 accept 时添加。",
            "final_summary 是写入前的 gate，不是需求细节的来源。"
        ]
    })
}

fn candidate_result_template(phase_id: &str) -> Value {
    json!({
        "requestSummary": {
            "title": "",
            "oneLine": "",
            "businessGoal": "",
            "complexity": "medium"
        },
        "scope": {
            "included": [{
                "id": "scope_1",
                "label": "",
                "items": [],
                "reason": "",
                "source": "user_confirmed"
            }],
            "excluded": [],
            "deferred": [{
                "id": "deferred_1",
                "label": "",
                "items": [],
                "reason": "",
                "source": "user_confirmed"
            }],
            "assumptions": [{
                "id": "assumption_1",
                "text": "",
                "requiresConfirmation": false
            }]
        },
        "roadmap": {
            "required": true,
            "phases": [{
                "phaseId": phase_id,
                "title": "",
                "name": "",
                "status": "scope_confirmed",
                "goal": ""
            }]
        },
        "phasePlan": {
            "current": {
                "title": "",
                "goal": "",
                "scopeRefs": ["scope_1"],
                "acceptanceRefs": ["acc_1"],
                "status": "scope_confirmed"
            },
            "nextPhasePreview": {
                "kind": "candidate",
                "suggestedPhaseId": "phase-next",
                "title": "",
                "goal": "",
                "scopePreview": [],
                "reason": ""
            }
        },
        "acceptance": [{
            "id": "acc_1",
            "statement": "",
            "capabilityRefs": [],
            "sourceRefs": [],
            "priority": "must"
        }],
        "domainModel": {
            "actors": [{
                "id": "actor_1",
                "name": "",
                "description": ""
            }],
            "capabilityGroups": [{
                "id": "capability_group_1",
                "name": "",
                "description": ""
            }],
            "businessFlows": [{
                "id": "flow_1",
                "name": "",
                "actors": ["actor_1"],
                "capabilityRefs": ["scope_1"],
                "summary": ""
            }]
        },
        "conceptGrounding": {
            "phaseConceptGrounding": {
                "mode": "concepts_present",
                "reason": "",
                "concepts": [{
                    "conceptId": "concept_1",
                    "term": "",
                    "normalizedName": "",
                    "explanation": "",
                    "mustNotMisinterpretAs": [],
                    "phaseRelevance": "current",
                    "priority": "must_understand",
                    "attentionRank": 1,
                    "riskFactors": [],
                    "scopeRefs": ["scope_1"],
                    "acceptanceRefs": ["acc_1"],
                    "humanReadableReason": ""
                }]
            },
            "glossaryUpdates": []
        },
        "conceptConfirmation": {
            "shownToUser": true,
            "confirmedConceptRefs": [],
            "confirmationSummary": ""
        },
        "securityRequirement": {
            "applies": "not_applicable | required | optional | deferred_with_risk",
            "clientTrustModels": ["same_origin_browser | external_api | service_to_service"],
            "sourceRefs": ["source id"],
            "rationale": "why security is or is not in scope"
        },
        "frontendExperience": {
            "required": true,
            "kind": "",
            "experienceLevel": "usable_internal_product",
            "audiences": [{
                "audienceId": "audience_1",
                "name": "",
                "primaryJobs": []
            }],
            "surfaces": [{
                "surfaceId": "surface_1",
                "name": "",
                "audienceRefs": ["audience_1"],
                "primaryJobs": []
            }],
            "dataViews": [{
                "viewId": "view_1",
                "name": "",
                "purpose": "",
                "targetObject": "",
                "selectionMode": "query_and_select",
                "paginationRequired": true,
                "defaultLoadsFirstPage": true,
                "searchCriteria": [{
                    "criterionId": "criterion_1",
                    "label": "",
                    "fieldRef": "",
                    "reason": "",
                    "sourceRefs": []
                }],
                "sourceRefs": []
            }],
            "actions": [{
                "actionId": "action_1",
                "label": "",
                "targetObject": "",
                "entryPoint": "navigation_entry",
                "inputFields": [],
                "resultObservation": ["response_message"],
                "refreshPolicy": "",
                "successFeedback": [],
                "blockingOrErrorFeedback": [],
                "sourceRefs": []
            }],
            "operationPaths": [{
                "pathId": "path_1",
                "name": "",
                "userGoal": "",
                "surfaceRef": "surface_1",
                "targetObject": "",
                "selectionMode": "query_and_select",
                "selectionSummary": "",
                "dataViewRefs": ["view_1"],
                "actionRefs": ["action_1"],
                "requiredStates": ["success", "business_blocking", "error"],
                "sourceRefs": []
            }],
            "mustNot": [],
            "confirmationSummary": ""
        }
    })
}

fn enum_refs() -> Value {
    json!({
        "complexity": ["small", "medium", "large", "unknown"],
        "scopeSource": ["source_explicit", "user_confirmed", "user_overridden", "model_recommended", "derived"],
        "acceptancePriority": ["must", "should", "could"],
        "phaseStatus": ["scope_confirmed", "proposed", "delivered", "paused", "skipped", "revised"],
        "phasePlanCurrentStatus": ["scope_confirmed"],
        "nextPhasePreviewKind": ["candidate", "none"],
        "conceptGroundingMode": ["concepts_present", "none_required", "not_applicable"],
        "conceptPhaseRelevance": ["current", "current_adjacent", "future", "deferred", "excluded"],
        "conceptPriority": ["must_understand", "should_understand", "nice_to_understand"],
        "conceptRiskFactor": ["business_invariant", "state_transition", "resource_consistency", "permission_boundary", "external_contract", "scope_confusion_risk", "user_visible_flow", "runtime_or_delivery_semantics", "frontend_experience_semantics"],
        "glossaryUpdateOperation": ["add", "replace", "remove"],
        "clarificationBlockName": ["phase_scope", "concept_grounding", "frontend_experience", "final_summary"],
        "clarificationMode": ["progressive_blocks"],
        "frontendExperienceLevel": ["none", "technical_demo", "usable_internal_product", "polished_product"],
        "frontendTargetSelectionMode": ["query_and_select", "direct_id_lookup", "preselected_context", "not_applicable"],
        "frontendActionEntryPoint": ["result_row_action", "detail_button", "form_submit", "bulk_action", "inline_action", "navigation_entry"],
        "frontendResultObservationMode": ["list_refresh", "detail_refresh", "inline_status_update", "response_message", "not_applicable"],
        "frontendInteractionState": ["loading", "success", "error", "empty", "business_blocking"],
        "securityRequirementApplicability": ["not_applicable", "required", "optional", "deferred_with_risk"],
        "clientTrustModel": ["same_origin_browser", "external_api", "service_to_service"]
    })
}

fn knowledge_query_plan() -> Value {
    json!({
        "sharedRules": [
            "仅对 phase_scope、concept_grounding 和 frontend_experience 使用 request-scoped knowledge context。",
            "不要将一个 Brainstorm 块的 knowledge chunks 带入另一个块，除非重新查询该块的步骤。",
            "对每个 executionOrder 步骤，调用 loom.knowledgeBrainstormContext，传入 projectRoot、requestRef、block、stepId、querySubject、naturalLanguageQuery 和 semanticFocus。",
            "当 executionOrder 步骤声明 repeatMode=per_candidate_phase_cut 时，在呈现选项前对每个候选阶段切割调用一次 loom.knowledgeBrainstormContext，使用不同的 queryId。",
            "如果 loom.knowledgeBrainstormContext 返回 status available，在使用前检查 readPlan 中列出的每个 chunk。",
            "如果 loom.knowledgeBrainstormContext 返回 status empty，继续使用源需求，仅在影响置信度时提及无知识匹配。",
            "如果任何 knowledge 工具返回 state failed 或 error 对象，停止澄清块并报告失败；不要静默回退到无知识的答案。",
            "不要要求用户在 Brainstorm 澄清中选择、命名、启用或管理 knowledge source。Loom 自动选择已启用的请求相关 knowledge，且在所需 knowledge 调用运行后允许空的 knowledge 结果。",
            "仅使用 knowledge 改善澄清质量。不要将 knowledge source ids、chunk ids、inspect output 或 knowledge paths 写入 Brainstorm candidate。",
            "semanticFocus 是一个紧凑的字符串数组。优先使用 kind:text 形式的类型化条目，如 object:核心业务对象、operation:提交请求、rule:完成前必须审批、page:管理工作台 或 flow:审批流程。仅在 kind 确实不明确时使用纯字符串。"
        ],
        "toolContract": {
            "contextTool": "loom.knowledgeBrainstormContext",
            "inspectTool": "loom.knowledgeInspectChunk",
            "doNotUseAsContextCheck": [
                "loom.knowledgeList",
                "loom.knowledgePending"
            ],
            "requiredInputFields": [
                "projectRoot",
                "requestRef",
                "block",
                "stepId",
                "querySubject",
                "naturalLanguageQuery",
                "semanticFocus"
            ],
            "conditionalInputFields": {
                "queryId": "当 executionOrder[].repeatMode 为 per_candidate_phase_cut 时必需。使用稳定值，如 capability_closure_A、capability_closure_B 或 atomic_scope。",
                "atomicScopeReason": "仅在 queryId 为 atomic_scope 时必需。"
            }
        },
        "blocks": {
            "phase_scope": {
                "executionOrder": [
                    {
                        "stepId": "phase_scope_dependency_order",
                        "queryKind": "dependency_order",
                        "querySubjectRule": "主题是用于比较当前阶段候选边界的依赖证据，不是完整的交付路线图。",
                        "queryConstructionRules": [
                            "仅使用 dependency_order 比较哪些属于当前阶段、哪些必须延后。",
                            "不要将整体依赖序列作为编号项目阶段输出或确认。",
                            "不要让宽泛的系统链查询单独决定当前阶段。"
                        ]
                    },
                    {
                        "stepId": "phase_scope_capability_closure",
                        "queryKind": "capability_closure",
                        "repeatMode": "per_candidate_phase_cut",
                        "minimumQueryCount": 2,
                        "queryIdRule": "对每个选项候选使用一个不同的 queryId，例如 capability_closure_A、capability_closure_B、capability_closure_C。当且仅当当前阶段确实是原子的，且不存在有意义的更窄、更宽、依赖、生命周期或 UI/runtime 边界时，使用 queryId=atomic_scope 并提供 atomicScopeReason。",
                        "querySubjectRule": "主题恰好是一个候选能力单元或一个闭合的当前阶段切片。",
                        "queryConstructionRules": [
                            "在组合选项前，从需求和 dependency-order 证据中识别候选能力单元。",
                            "对每个候选阶段切割运行一次 capability_closure 查询，使用不同的 queryId。",
                            "每个 capability_closure 查询恰好覆盖一个模块、对象生命周期、工作流、后端能力或页面操作集。",
                            "将 semanticFocus 保持在当前单元的 object、operation、rule、state、field 或 flow 锚点内。",
                            "不要在 semanticFocus 中包含兄弟、下游或下一阶段的能力单元。",
                            "对不同的操作使用独立的类型化 operation focus 条目；绝不将多个操作合并为一个 focus 条目。",
                            "对于连接的流程、生命周期转换、替换、恢复或有序流，包含主对象加上每个可识别的组件操作作为独立的 operation focus 条目；仅在整个流措辞明确时添加 flow focus。"
                        ]
                    }
                ]
            },
            "concept_grounding": {
                "executionOrder": [
                    {
                        "stepId": "concept_grounding_scope_item",
                        "queryKind": "scope_item_grounding",
                        "querySubjectRule": "主题是一个已确认的 scope item 或一组来自用户已确认阶段范围的紧密组合，共享相同的对象和操作流。",
                        "queryConstructionRules": [
                            "从每个已确认的当前阶段 included item 开始；不要只查询最简单或最高层的对象。",
                            "naturalLanguageQuery 应询问该主题的 objects、fields、rules、states、validations、blockers、outcomes、feedback 和 misunderstanding boundaries。",
                            "semanticFocus 必须保持在已确认 scope item 或紧密组合内，将 object focus 与相关的 operation、rule、state、field 或 flow 锚点配对（当这些锚点明确时）。",
                            "不要重新查询整个系统或每个延后的模块。",
                            "对不同的操作和生命周期步骤使用独立的类型化 operation focus 条目；不要使用单一复合 focus 替代生命周期或流程组件操作。"
                        ]
                    }
                ]
            },
            "frontend_experience": {
                "executionOrder": [
                    {
                        "stepId": "frontend_experience_page_operation_path",
                        "queryKind": "page_operation_path",
                        "querySubjectRule": "主题是一个已确认的用户/员工可见操作路径，或一组共享一个 surface、target discovery、action entry、feedback 和 readback 模式的紧密组合。",
                        "queryConstructionRules": [
                            "naturalLanguageQuery 应询问 entry surface、target discovery、query/list/detail selection、action entry、form inputs、success feedback、validation 或 business-blocking feedback、loading/empty/error states 和 refresh/readback。",
                            "从已确认的当前阶段操作和阻断条件出发，然后将它们转化为页面操作检索锚点，描述用户或员工如何找到目标、启动操作、输入数据、查看反馈并回读更新后的状态。",
                            "当 page 或 flow 标签明确时优先使用；否则使用已确认 scope 中的 operation、field、state 和 object 锚点。",
                            "当 knowledge source 没有明确的 page 标签时，在 naturalLanguageQuery 中使用页面操作意图，并使用已确认 scope 中的 operation、field、state 或 object semanticFocus 锚点。",
                            "对于多步页面路径或操作工作流，包含 page 或 flow focus 加上具体的 operation、field 或 state 锚点；不要仅依赖复合 operation focus。",
                            "不要为了满足 semanticFocus 而编造 page 标签。",
                            "当 operation、field 或 state 锚点可用时，不要仅使用业务对象名称查询。"
                        ]
                    }
                ]
            }
        }
    })
}

fn knowledge_query_plan_for_block(block: &ClarificationBlockName) -> Value {
    if *block == ClarificationBlockName::FinalSummary {
        return json!({
            "sharedRules": [
                "final_summary 不调用 knowledge context。"
            ],
            "toolContract": {
                "contextTool": "loom.knowledgeBrainstormContext",
                "inspectTool": "loom.knowledgeInspectChunk"
            },
            "blocks": {}
        });
    }
    let full = knowledge_query_plan();
    let block_name = block_id(block);
    let Some(block_plan) = full.pointer(&format!("/blocks/{block_name}")).cloned() else {
        return full;
    };
    let mut blocks = Map::new();
    blocks.insert(block_name.to_string(), block_plan);
    json!({
        "sharedRules": full.get("sharedRules").cloned().unwrap_or_else(|| json!([])),
        "toolContract": full.get("toolContract").cloned().unwrap_or_else(|| json!({})),
        "blocks": Value::Object(blocks)
    })
}

fn block_rules(block: &ClarificationBlockName) -> (&'static str, Value, Vec<&'static str>) {
    match block {
        ClarificationBlockName::PhaseScope => (
            "phaseScope",
            phase_scope_rules(),
            vec![
                "rules.phaseScope.blockMission",
                "rules.phaseScope.presentation",
                "rules.phaseScope.optionComparison",
                "rules.phaseScope.forbiddenOutput",
                "rules.phaseScope.selfCheck",
                "rules.phaseScope.confirmedDataShape",
            ],
        ),
        ClarificationBlockName::ConceptGrounding => (
            "conceptGrounding",
            concept_grounding_rules(),
            vec![
                "rules.conceptGrounding.presentation",
                "rules.conceptGrounding.selfCheck",
                "rules.conceptGrounding.scopeItemCoverage",
                "rules.conceptGrounding.objectOperation",
                "rules.conceptGrounding.businessScenario",
                "rules.conceptGrounding.decisionImpactOrdering",
                "rules.conceptGrounding.businessLifecycleScan",
                "rules.conceptGrounding.confirmedDataShape",
            ],
        ),
        ClarificationBlockName::FrontendExperience => (
            "frontendExperience",
            frontend_experience_rules(),
            vec![
                "rules.frontendExperience.presentation",
                "rules.frontendExperience.selfCheck",
                "rules.frontendExperience.operationPath",
                "rules.frontendExperience.candidateCarryForward",
                "rules.frontendExperience.confirmedDataShape",
            ],
        ),
        ClarificationBlockName::FinalSummary => (
            "finalSummary",
            final_summary_rules(),
            vec![
                "rules.finalSummary.reviewGate",
                "rules.finalSummary.presentation",
                "rules.finalSummary.checklistCoverage",
                "rules.finalSummary.requiredUserVisibleTopics",
                "rules.finalSummary.correctionWriteback",
                "rules.finalSummary.detailRetention",
                "rules.finalSummary.confirmedDataShape",
            ],
        ),
    }
}

fn block_id(block: &ClarificationBlockName) -> &'static str {
    match block {
        ClarificationBlockName::PhaseScope => "phase_scope",
        ClarificationBlockName::ConceptGrounding => "concept_grounding",
        ClarificationBlockName::FrontendExperience => "frontend_experience",
        ClarificationBlockName::FinalSummary => "final_summary",
    }
}

fn user_visible_block_title(block: &ClarificationBlockName) -> &'static str {
    match block {
        ClarificationBlockName::PhaseScope => "阶段范围确认",
        ClarificationBlockName::ConceptGrounding => "业务理解与规则确认",
        ClarificationBlockName::FrontendExperience => "页面办理路径确认",
        ClarificationBlockName::FinalSummary => "提交前确认",
    }
}

fn block_rule(block: &ClarificationBlockName) -> &'static str {
    match block {
        ClarificationBlockName::PhaseScope => {
            "仅确认当前阶段边界：先查询此块的 request-scoped knowledge，然后呈现 2-3 个当前阶段选项，而非完整的多阶段项目路线图，并等待用户明确确认。"
        }
        ClarificationBlockName::ConceptGrounding => {
            "仅使用已确认的当前阶段 scope 作为主题集；先查询此块的 request-scoped knowledge，然后等待用户明确确认。"
        }
        ClarificationBlockName::FrontendExperience => {
            "使用已确认的业务操作；先查询此块的 request-scoped knowledge，然后确认页面或工作空间路径，或记录具体的跳过原因。"
        }
        ClarificationBlockName::FinalSummary => {
            "汇总已确认的块进行最终确认；不要在此处引入新的需求细节。"
        }
    }
}

fn current_turn_answer_rule(block: &ClarificationBlockName) -> Value {
    json!({
        "consumeCurrentUserMessage": true,
        "meaning": "如果启动或继续此 Loom 轮次的同一条用户消息已经对此 Brainstorm 块给出了明确答案，则将该消息视为用户对此块的可见确认，而非再次询问。",
        "explicitOnly": true,
        "ifAmbiguousAskUser": true,
        "evidenceSource": "先使用紧凑的 requirement_context；仅在紧凑上下文不足以判断当前用户消息是否明确回答了此块时，才阅读 requirement_full_text。",
        "currentBlock": block_id(block),
        "blockSpecificRule": current_turn_answer_block_rule(block)
    })
}

fn current_turn_answer_block_rule(block: &ClarificationBlockName) -> &'static str {
    match block {
        ClarificationBlockName::PhaseScope => {
            "仅当当前消息明确选择了当前阶段边界时才消费该消息，而非仅仅要求 Loom 提出阶段选项。"
        }
        ClarificationBlockName::ConceptGrounding => {
            "仅当当前消息明确确认或修正了已确认阶段范围的业务对象、操作、规则、字段、状态、阻断条件和边界时才消费该消息。"
        }
        ClarificationBlockName::FrontendExperience => {
            "仅当当前消息明确确认或修正了用户可见的页面/工作空间操作路径，或明确表示 UI 不适用时才消费该消息。"
        }
        ClarificationBlockName::FinalSummary => {
            "仅当当前消息明确确认了提交前检查清单，或对先前确认的块给出了具体修正时才消费该消息。"
        }
    }
}

fn block_confirmed_data_shape(block: &ClarificationBlockName) -> Value {
    match block {
        ClarificationBlockName::PhaseScope => json!({
            "scope": {
                "included": ["用户确认的当前阶段能力项"],
                "deferred": ["当前阶段外的边界项"],
                "excluded": ["不在此交付中的项（适用时）"]
            },
            "recommendation": {
                "label": "已确认的当前阶段",
                "reason": "为何确认此阶段边界"
            },
            "nextPhasePreview": "有用时附简短的用户可见下一阶段预览"
        }),
        ClarificationBlockName::ConceptGrounding => json!({
            "scopeCoverage": ["每个已确认 scope item 的覆盖说明"],
            "objects": ["业务对象或主体"],
            "operations": ["业务操作或工作流"],
            "fields": ["重要字段或输入"],
            "states": ["重要状态"],
            "rules": ["校验、阻断、结果或不变规则"],
            "boundaries": ["误解边界或延后规则"]
        }),
        ClarificationBlockName::FrontendExperience => json!({
            "required": true,
            "surfaces": ["页面或工作空间名称"],
            "targetDiscovery": ["查询、列表、选择或预选上下文"],
            "operationPaths": ["入口、输入、成功反馈、阻断反馈和回读"],
            "mustNot": ["不可接受的页面交互形式"]
        }),
        ClarificationBlockName::FinalSummary => json!({
            "coverageChecklist": ["已确认的 scope、规则、页面路径和延后边界"],
            "corrections": ["用户修正回写到先前块数据"],
            "readyToWriteCandidate": true
        }),
    }
}

fn phase_scope_rules() -> Value {
    json!({
        "blockMission": [
            "此块仅确认当前阶段的实现边界。",
            "仅使用模块依赖顺序来比较当前阶段候选边界和延后项。",
            "在呈现选项前，调用 loom.knowledgeBrainstormContext 执行 dependency_order 和 knowledge_context_plan 中每个必需的 per-candidate capability_closure 查询。如果结果为空，继续使用源需求。",
            "即使源要求阶段优先级或分阶段交付，也不要在此块中要求用户确认完整的项目路线图。"
        ],
        "presentation": [
            "仅对当前阶段呈现 2-3 个备选方案，建议以 A/B/C 形式。",
            "每个备选方案必须是一个当前阶段候选切片，而非一系列阶段。",
            "每个备选方案应显示 included scope、deferred boundary、reason 和 tradeoff。",
            "以一个推荐方案结尾，并要求用户选择 A/B/C 或调整当前阶段边界。"
        ],
        "optionComparison": [
            "在 phase_scope 块中，默认在请求确认前呈现 2-3 个基于源的 phase scope 选项。将任何 next phase seed 或 dependency order 视为选项设计的非约束性种子，而非预选的用户答案。",
            "在编写 phase_scope 选项前，内部将基于源的当前阶段候选工作分解为 scope items，并在组合选项前对每个 item 进行分类。",
            "仅使用以下内部 scope item 类别：goal-essential item、flow-support item、current-object lifecycle item、experience or management extension item、cross-phase item、explicitly excluded item 和 unresolved classification item。",
            "goal-essential item 是当前阶段目标成立所必需的；不要在推荐选项中遗漏它。",
            "flow-support item 不是标题目标，但对于范围内操作的可使用、可选择、可提交、可校验、可回读或可验证是必需的；不要仅将其移到宽选项中。",
            "如果任何备选选项包含推荐选项使用、选择、操作、校验、回读或验证所需的 support-like item，则推荐选项必须明确包含该 support item，而非依赖隐含。",
            "current-object lifecycle item 属于当前阶段核心对象或主体的生命周期。如果阶段目标是闭环或生命周期闭合，推荐选项必须包含适用的 current-object lifecycle items，除非有基于源的理由要求用户缩减范围。",
            "experience or management extension item 改善体验、管理、可观测性、审批深度、报告或相邻便利性，但对当前阶段目标或流程支持不是必需的。",
            "cross-phase item 依赖于后续模块、其他产品表面、其他子系统或不同的核心对象生命周期；除非用户明确要求跨阶段边界，否则不要将其放入推荐选项。",
            "explicitly excluded item 不得出现在任何选项的 included scope 中。",
            "如果分类不确定，以自然语言标记该 item 为 unresolved，并提出聚焦的范围问题，而非静默地将其放入 A/B/C。",
            "不要向用户暴露这些内部类别名称。仅使用它们使选项连贯。",
            "从所有 goal-essential items 加上所有 flow-support items 生成推荐选项，并在当前阶段目标为闭环或生命周期闭合时包含 current-object lifecycle items。",
            "通过仅缩减 current-object lifecycle items 或 experience/management extension items 生成更窄选项；不要从窄选项中移除 goal-essential 或 flow-support items，除非将其标记为有意的范围缩减并要求用户确认该缩减。",
            "通过添加真实的 experience/management extension items 或明确标记的相邻能力生成更宽选项；不要使宽选项成为 goal-essential 或 flow-support items 唯一出现的地方。",
            "在呈现选项前，将每个备选选项的 included-scope 行与推荐选项进行比较。如果备选项包含使用、选择、校验、回读或验证推荐选项所需的 item，则将其添加到推荐选项中或提出聚焦问题。",
            "每个选项必须显示 included scope、deferred 或 not-this-phase boundary、reason 和 tradeoff。",
            "每个选项块必须使用稳定的多行模板：首行为选项字母和简短标题，然后是单独的标记行，分别为 included scope、not-this-phase 或 deferred scope、reason 和 tradeoff。在中文中使用等同于 包含、本阶段不做或延后、原因 和 取舍 的用户可见标签。",
            "推荐应在选项标题中显示，如 A（推荐）或 Recommended，而非埋在选项段落中。",
            "仅推荐一个 phase_scope 选项，并解释为何它是当前阶段最佳切割。推荐选项必须保留当前阶段的基于源的核心结果、模块闭合、生命周期覆盖和依赖目的。",
            "在组合选项时，不要重写基于源或先前确认的对象关系、操作、所有权或状态转换。不要仅为了创建选项而将一个对象的操作变成另一个对象的操作。",
            "对于高风险的对象关系工作，phase_scope 应以基于源的术语命名能力边界，并将详细的关系语义推迟到 concept_grounding，除非源或先前确认已固定这些语义。",
            "不要在 phase_scope 措辞中引入模糊或新的关系端点。避免暗示一个对象被替换、重链接、继承、转移、冻结或恢复，除非该确切的关系效果是基于源的或先前确认的。",
            "可以提供更窄选项作为备选，但当它延后明确的当前阶段 seed items、先前延后的当前阶段工作或定义模块闭合的已确认生命周期操作时，不要推荐它，除非用户要求缩减范围或源/仓库证据显示该阶段无法实现完全闭合。",
            "如果推荐选项排除或延后了任何明确的当前阶段 seed item，则将其标记为范围缩减，解释基于源的理由，并要求用户确认该缩减，而非将其作为默认推荐呈现。",
            "仅当原子阶段不存在基于源的更窄切割、更宽相邻工作流切割、依赖/顺序切割、延后生命周期操作或备选 UI/runtime 边界时，才使用单一 phase_scope。",
            "当存在多个具体操作、工作流、UI surfaces、生命周期变更或交付边界时，该阶段不是原子的，必须呈现为 2-3 个选项。"
        ],
        "forbiddenOutput": [
            "不要输出编号的完整项目阶段，如 1..N。",
            "不要要求用户确认整个依赖序列。",
            "不要在此处将下游模块拆分为各自确认的实现阶段。",
            "仅在 deferred boundary 或简短的 next-phase preview 中提及下游工作。"
        ],
        "selfCheck": [
            "在响应前，检查每个选项是否为当前阶段边界候选。",
            "在响应前，检查是否使用了当前块的 knowledge_context_plan；如果 knowledge 结果为空，可以从源需求继续。",
            "在响应前，检查消息不是完整的多阶段路线图。",
            "验证基于源的候选工作已分解为 scope items 并在编写选项前进行了内部分类。",
            "验证推荐选项包含每个 goal-essential item 和 flow-support item，并在当前阶段目标为闭环或生命周期闭合时包含 current-object lifecycle items。",
            "验证更窄选项不会静默移除 goal-essential 或 flow-support items；如果移除了，该选项必须标记为需要用户确认的有意范围缩减。",
            "验证更宽选项仅添加真实的 experience/management extension items、相邻能力或明确标记的 cross-phase items；更宽选项不得成为 goal-essential 或 flow-support items 唯一出现的地方。",
            "将备选 included-scope 行与推荐选项进行比较，拒绝任何推荐选项所需的 support item 仅出现在备选选项中的选项集。",
            "验证没有选项重写基于源或先前确认的对象关系、对象所有权、操作所有权或状态转换。",
            "验证 explicitly excluded items 不出现在任何选项的 included scope 中，且 unresolved classification items 作为聚焦问题呈现，而非被静默包含。",
            "验证 included、deferred 和 excluded scope 是明确的，且 included scope 在源中存在时命名具体的对象、操作、工作流、交付物或边界。",
            "如果显示的选项少于 2 个，在请求确认前记录为何不存在更窄切割、更宽相邻工作流切割、依赖/顺序切割、延后生命周期操作或备选 UI/runtime 边界。",
            "不要让相邻或下游工作占据当前阶段，除非用户明确要求更宽的边界。"
        ],
        "confirmedDataShape": block_confirmed_data_shape(&ClarificationBlockName::PhaseScope)
    })
}

fn concept_grounding_rules() -> Value {
    json!({
        "presentation": [
            "使用用户可见标题，如业务理解与规则确认、业务规则确认。不要向用户展示内部名称，如 concept_grounding、conceptGrounding、domainModel、businessFlows、riskFactor、semantic grounding、scope.included 或 acceptance。",
            "当领域行为适用时，使用稳定的用户可见段落顺序：当前业务场景、逐 scope 覆盖、关键对象与操作规则、未决或延后规则（如有）、以及一条确认指令。",
            "将当前业务场景保持为简短的通俗段落，然后使用列表或紧凑 mini-block 展示细节。不要将场景、概念、对象字段、操作、阻断规则和状态变更合并为一个长段落。",
            "对于逐 scope 覆盖，每个已确认的当前阶段 scope item 显示一个单独的列表或 mini-block。每项应仅命名适用细节：对象或主体、操作或行为、关键字段或输入、前置条件、校验或阻断原因、成功状态或可见反馈、以及未决或延后说明。",
            "对于关键对象与操作规则，每个重要对象或操作显示一个单独的列表或 mini-block。当细节适用时，使用等同于 对象/关系、关键字段、操作与前置条件、校验/阻断、状态/成功结果、和 未决/递延 的用户可见标签。",
            "如果阶段是纯技术的或某个细节类别不适用，以用户语言说明具体原因，而非用通用业务标签填充检查清单。",
            "以一条简洁的确认指令结尾。除非有具体的未决问题阻碍进度，否则不要要求用户逐节单独确认。"
        ],
        "selfCheck": [
            "在呈现 concept_grounding 供用户确认前，在块内运行 concept_grounding 自检。",
            "验证每个已确认的 scope.included item 已被覆盖、明确标记为未决、或明确标记为延后。",
            "验证当前阶段有领域行为或用户操作时，已考虑业务场景确认、决策影响排序和生命周期扫描。",
            "验证关键对象或主体包含适用的字段集、操作输入、前置条件、校验或阻断原因、成功状态、状态转换和可见或返回的反馈。",
            "如果相关细节不明确或缺失，在标记 concept_grounding 为已确认前提出聚焦的概念或业务规则问题。",
            "不要让 final_summary 成为业务规则首次出现的地方。"
        ],
        "scopeItemCoverage": [
            "在要求用户确认概念前，包含一个自然语言的 scope-item 覆盖摘要。",
            "对每个已确认的 scope.included item，仅使用适用维度说明覆盖了什么需求细节：对象或主体、用户/系统操作或行为、输入或字段、前置条件、校验或阻断条件及原因、成功状态/数据/UI/API/结果变更、可见或返回的反馈、source refs 和未决说明。",
            "不要强制每个维度作用于每个 scope item。如果某个维度不适用，省略它或给出简短的具体原因；如果适用但源信息不足，标记为未决或提出聚焦的澄清。",
            "在呈现覆盖摘要时，不要使用固定的能力分类法或测试场景类别。覆盖行应遵循已确认的 scope 措辞和源事实。",
            "如果已确认的 scope item 未出现在 scope-item 覆盖摘要中，不要进入 frontend_experience 或 final_summary；先覆盖或明确延后该 item。"
        ],
        "objectOperation": [
            "concept_grounding 块拥有领域阶段的对象-操作澄清。",
            "当当前阶段包含业务对象、用户操作、系统操作、表单、持久化、状态变更或校验/阻断规则时，在要求用户确认概念前呈现自然语言的对象-操作摘要。",
            "对于当前阶段的每个关键业务对象，列出该阶段依赖的关键字段集：identity fields、input fields、display fields、relationship fields、state fields 和 result 或 feedback fields。有源确认名称时使用；如果某个类别不明确，将缺失细节作为问题或未决说明提出，而非编造字段。",
            "对于关键对象上的每个操作，汇总操作输入、前置条件、校验规则、阻断条件、阻断原因、成功结果、状态变更和用户可见反馈，下游实现必须保留这些内容。",
            "concept_grounding 中展示的每个对象字段、操作规则、状态变更和阻断原因必须指向原始需求、已确认的用户决策、仓库事实或明确的未决澄清说明。Keyword hints 仅作参考。",
            "当业务操作在范围内时，不要仅呈现名词定义或宽泛的概念摘要。",
            "如果当前阶段是纯技术、基础设施、构建、部署或非领域工作，说明为何对象-操作澄清不适用，并将 concept grounding 限于真实的高风险技术概念。"
        ],
        "businessScenario": [
            "当当前阶段有领域行为或用户操作时，包含一个通俗的业务场景确认。",
            "场景确认必须命名参与者或系统、业务对象或主体、触发条件、操作目标、预期结果和（适用时）此阶段不会做的边界。",
            "如果当前阶段是纯技术的，说明业务场景确认不适用的具体原因，并改为汇总技术工作流。"
        ],
        "decisionImpactOrdering": [
            "在请求确认前，按下游影响排序识别重要的澄清决策。",
            "高影响决策是指改变阶段范围、数据模型、业务流、前端操作路径、接口契约、验收结果、runtime 或交付边界的决策。",
            "对每个高影响决策，以通俗语言说明它影响什么，以及它是已确认、未决、明确延后还是不适用。",
            "不要仅为了填充检查清单而编造决策。仅包含基于源需求、已确认用户答案、仓库事实或明确未决说明的决策。"
        ],
        "businessLifecycleScan": [
            "对每个关键当前阶段业务对象或主体，扫描对此阶段真正相关的生命周期操作。",
            "生命周期操作包括 create、query/select、view、update、approve/process、state change、terminate/cancel 和 blocking/exception handling；不要强制每个操作作用于每个对象。",
            "对每个相关的生命周期操作，适用时汇总输入或字段、前置条件、校验或阻断原因、成功状态变更和可见或返回的反馈。",
            "对于明确不在范围内或延后的生命周期操作，以用户语言说明该边界，并视情况保留在 deferred、excluded、assumptions 或 next phase preview 中。"
        ],
        "confirmedDataShape": block_confirmed_data_shape(&ClarificationBlockName::ConceptGrounding)
    })
}

fn frontend_experience_rules() -> Value {
    json!({
        "presentation": [
            "使用用户可见标题，如页面办理路径确认或页面操作路径确认。不要向用户展示内部名称，如 frontend_experience、frontendExperience、targetDiscovery、search_then_select、list_browse、query_and_select、direct_id_lookup、preselected_context、not_applicable、dataViews、actions 或 operationPaths。",
            "当 UI 适用时，使用稳定的用户可见段落顺序：页面或工作空间 surface、target discovery 或 query-selection path、操作入口和输入、结果反馈和刷新/回读、不可接受的页面形式（如有）、以及一条确认指令。",
            "如果用户必须查询或选择已有对象，将分页/列表行为和查询条件作为单独的标记行显示，如 查询条件。不要将查询条件埋在散文段落中。",
            "对于多个操作，每个操作或操作组显示一个单独的列表或 mini-block。每个操作项应命名起点、用户输入内容、成功表现、阻断/错误反馈表现、以及刷新/回读行为。",
            "如果 UI 不是必需的、延后的、无变更继承的、或由登录/会话/预选上下文驱动的，说明具体原因，并仍避免使用内部枚举标签。",
            "以一条简洁的确认指令结尾。除非有具体的未决路径阻碍进度，否则不要要求用户逐操作单独确认。"
        ],
        "selfCheck": [
            "在呈现 frontend_experience 供用户确认前，在块内运行 frontend_experience 自检。",
            "验证 UI 是否必需、不必需或延后，并以用户语言说明原因。",
            "当 UI 必需时，验证 target discovery 或 selection、分页/列表行为（相关时）、基于源的查询条件、操作入口、输入字段、成功反馈、错误反馈、业务阻断反馈和刷新/回读行为。",
            "验证查询条件来自已确认的字段、用户措辞、验收细节、业务流细节或仓库事实；不要使用硬编码的行业字段列表。",
            "如果操作路径不明确，在标记 frontend_experience 为已确认前提出聚焦的前端操作路径问题。",
            "当阶段是非 UI 工作时，不要编造页面路径。"
        ],
        "operationPath": [
            "frontend_experience 块拥有页面操作路径澄清；不要等到 final_summary 才首次询问用户如何找到目标、触发操作或观察结果。",
            "当当前阶段对已有业务对象有 UI 时，呈现自然语言默认方案——分页查询结果加从结果中选择/操作，除非用户已确认直接 id 输入、上游上下文、登录/会话上下文或无目标对象。",
            "当操作从先前页面、已认证会话、通知、外部链接或已选记录开始时，以用户语言描述该预选上下文，不要强制查询页面。",
            "当操作是仅创建、仅登录、静态内容、本地开发工具或非 UI 技术任务时，说明为何 target selection 不适用。",
            "如果提议搜索/查询路径，仅列出基于已确认对象字段、验收声明、业务流细节、仓库事实或用户原话的查询条件；不要使用硬编码的行业字段列表。",
            "如果已确认字段不足以构成有意义的筛选器，不要阻断阶段。确认一个无高级筛选器的基本分页结果列表，并将缺失的筛选细节记录为风险或说明。",
            "在对话中使用自然的用户可见措辞，如 分页查询结果中选择记录并操作 或 从登录上下文带入当前对象。不要向用户展示内部枚举值，如 query_and_select、direct_id_lookup、preselected_context、not_applicable、dataViews、actions 或 operationPaths。"
        ],
        "candidateCarryForward": [
            "当 frontendExperience 存在时，将已确认的页面操作路径存储在 dataViews、actions 和 operationPaths 中，而非仅存储在 confirmationSummary 中。",
            "对于 query-and-select 工作流，设置 dataViews 分页和首页加载预期。仅当无查询条件被确认时搜索条件才可选；当查询条件已确认时，保留它们。",
            "对于 direct id lookup 工作流，说明为何直接 id 输入已由用户确认或操作上合适；不要将其作为已有对象后台操作的默认方式。",
            "每个 action 必须命名其入口点、输入字段（适用时）、成功反馈、阻断/错误反馈和刷新策略，以便下游架构和任务执行继承已确认的用户体验目标。"
        ],
        "confirmedDataShape": block_confirmed_data_shape(&ClarificationBlockName::FrontendExperience)
    })
}

fn final_summary_rules() -> Value {
    json!({
        "reviewGate": [
            "final_summary 是提交前覆盖检查清单的 gate，不是需求细节的来源。",
            "在呈现 final_summary 供用户确认前，验证 phase_scope、concept_grounding 和 frontend_experience 已被确认或以具体原因明确跳过。",
            "不要使用 final_summary 引入先前块中未确认的新需求。",
            "不要通过扩展 final_summary 来修复缺失的结构化细节。返回相关的 Brainstorm 块或修复对应的结构化 candidate 字段后再提交。"
        ],
        "presentation": [
            "使用用户可见标题，如提交前核对或提交前确认。不要向用户展示内部名称，如 final_summary、phase_scope、concept_grounding、frontend_experience、BrainstormCandidate、dataViews、actions 或 operationPaths。",
            "将已确认的先前块投影为一个用户可见的覆盖检查清单，仅一个确认动作，而非三个单独确认。",
            "使用稳定的用户可见段落顺序，等同于：当前要提交的阶段、当前阶段覆盖、已确认的业务规则、已确认的页面操作路径、本阶段不做的内容、下一阶段预览、确认指令。",
            "以一条确认指令结尾。不要要求用户逐个重新确认先前块。"
        ],
        "checklistCoverage": [
            "对于除单一当前阶段目标和下一阶段预览之外的每个适用检查清单段，当先前已确认两个或更多项时，至少包含两个具体的检查清单项。",
            "当前阶段覆盖段必须列出已确认阶段范围中的具体包含能力或操作；当多个能力已确认时，不要将范围折叠为单个抽象句子。",
            "业务规则段必须列出适用时已确认的具体业务对象、关系、操作名称、字段集摘要、状态变更、阻断规则、成功结果或高风险误解防护。",
            "页面操作路径段必须在 UI 适用时列出具体的 surface/entry、target discovery 或 query-selection path、分页/查询条件（已确认时）、操作入口、结果反馈和刷新/回读行为。",
            "如果某个段不适用，用具体的用户语言写明原因，而非编造检查清单项。",
            "如果无法从已确认的先前块中提取具体的检查清单项，不要编造，也不要提交；先返回相关的 Brainstorm 块或修复对应的结构化 candidate 字段。"
        ],
        "requiredUserVisibleTopics": [
            "当前阶段提交目标",
            "来自已确认阶段范围的覆盖检查清单，包含具体的包含工作和延后或不做的边界",
            "来自已确认业务理解的业务规则检查清单，包含适用时具体的对象、关系、操作、字段集摘要、状态变更、阻断规则、成功结果和高风险误解防护",
            "来自已确认前端路径的页面操作检查清单，包含适用时 surface 或 entry、target discovery 或 query selection、分页和查询条件（已确认时）、操作入口、反馈和刷新或回读",
            "必须回写到结构化字段的明确 final_summary 修正",
            "用用户语言表述的下一阶段预览"
        ],
        "correctionWriteback": [
            "如果用户修正了 final_summary，不要从过时的摘要提交 BrainstormCandidate。",
            "将修正纳入受影响的已有字段，并在设置 finalSummaryConfirmed=true 之前呈现更新的 final_summary。",
            "修正可以更新阶段范围、概念理解、前端体验、延后边界或下一阶段预览；不要仅将其存储在 final_summary 文本中。"
        ],
        "detailRetention": [
            "不要求 final_summary 重复先前块中已确认的每个对象、字段、操作、规则、状态变更、阻断原因、反馈路径或前端操作路径。",
            "检查清单式的 final_summary 不会缩窄、省略、覆盖或压缩已确认的 phase_scope、concept_grounding 或 frontend_experience 细节。",
            "先前确认的块细节通过结构化字段（而非 final_summary 文本）仍是 BrainstormCandidate 契约的一部分。",
            "即使 final_summary 简洁，也要将已确认的 phase_scope、concept_grounding 和 frontend_experience 细节保留在结构化字段中。"
        ],
        "confirmedDataShape": block_confirmed_data_shape(&ClarificationBlockName::FinalSummary)
    })
}

fn candidate_write_rules() -> Value {
    json!([
        "仅在用户明确确认 final_summary 后写入 Brainstorm candidate target。",
        "接受的 BrainstormCandidate 必须由所有用户已确认的 Brainstorm 块加上 final_summary 修正构建，而非仅从 final_summary 构建。",
        "写入 candidate 时使用 outputContract.resultTemplate 作为具体字段形状。",
        "outputContract.resultTemplate 中的数组展示对象形状。仅当已确认需求对该字段无项时才保持数组为空；绝不用字符串数组替换类型化对象数组。",
        "当 scope.deferred 非空时，phasePlan.nextPhasePreview 必须使用 kind=candidate 并包含 suggestedPhaseId、title、goal、scopePreview 和 reason。仅当 scope.deferred 为空时才使用 kind=none。",
        "在用户可见摘要和确认文本中，使用用户可见块名称而非内部 block ids。",
        "不要将 knowledge 元数据放入 candidate 的 sourceRefs 和 summary 字段中。",
        "将所有已确认的块细节保留在 scope、acceptance、domainModel.businessFlows、conceptGrounding 和 frontendExperience 中，而非依赖 final_summary 文本。",
        "对于 phase_scope，将已确认选项的 included scope、excluded scope、deferred scope、reasons、tradeoffs 和 nextPhasePreview 方向保留在 scope、roadmap、phasePlan、assumptions 或 acceptance 中（视情况而定）。",
        "对于 concept_grounding，将已确认的业务场景、高风险概念、业务对象或主体、关键字段集、支持的操作、操作输入、前置条件、校验或阻断原因、状态转换、成功结果、可见或返回的反馈、未决说明和不可误解边界保留在 scope、acceptance、domainModel.businessFlows 和 conceptGrounding 中。",
        "对于 frontend_experience，将已确认的 UI 需求或跳过原因、surfaces、data views、target discovery 或 selection path、分页、已确认的查询条件、操作入口点、输入字段、成功反馈、错误反馈、业务阻断反馈、空/加载状态、刷新/回读策略和不可接受的 UI 形式保留在 frontendExperience 中。",
        "对于 final_summary，保留用户修正和最终范围决策，但不要将检查清单式的 final_summary 视为丢弃先前块中已确认细节的许可。",
        "如果已确认的细节无法放入精确的结构化字段，将其保留在最近现有字段的自然语言 summary 或 notes 中，而非省略。",
        "在写入或提交 BrainstormCandidate 前，对每个已确认的 Brainstorm 块进行自检；final_summary 仅是最后的确认表面和修正来源。",
        "自检必须验证已确认的需求细节存储在已有 BrainstormCandidate 字段中而非仅存储在聊天中：scope.included[].items、acceptance[].statement、domainModel.businessFlows[].summary、conceptGrounding、frontendExperience 和 phasePlan.nextPhasePreview。",
        "自检必须验证每个已确认的 scope.included item 已在 concept_grounding scope-item 覆盖摘要中被考虑。如果某个 scope item 无适用细节，保留具体原因或未决说明，而非静默丢弃。",
        "自检必须检查用户可见的工作流阶段是否在 frontendExperience 中存储了页面操作路径：用户如何找到或接收目标对象、分页和查询条件（已确认时）、哪个 view/action 启动操作、输入字段、刷新/回读策略以及如何观察成功、空、加载、错误或业务阻断结果。",
        "如果自检发现某个必需的细节不明确或从已有字段中缺失，在提交前返回相关的 Brainstorm 块并询问用户；不要让 PGC、AAC、TaskPlan 或 TaskExecution 在后续重新发现该细节。",
        "不要为此自检创建单独的 Markdown 规范、commit 或并行需求 artifact；接受的 BrainstormCandidate 仍是需求契约。"
    ])
}

fn requirement_semantic_compact_rules() -> Value {
    json!([
        "在已有 Brainstorm candidate 字段中保留已确认的当前阶段语义；避免模糊标签。",
        "当业务细节适用时，在所属块中确认并写入 objects、operations、rules、fields、blockers、outcomes 和 page paths。",
        "final_summary 仅是最终的覆盖检查清单和修正来源；不要将其作为 candidate 的唯一细节来源。",
        "当业务细节不适用时，说明具体的非领域原因，而非编造领域规则。",
        "如果在阅读需求和检查的 knowledge 后某个必需的语义细节仍不明确，在 accept 前询问用户。"
    ])
}
