use delivery_core::{InspectRequestInput, ReadRequestFieldsInput};
use mcp_server::LoomMcpServer;
use serde_json::{json, Value};
use state::{paths::from_project_relative, store::write_json_atomic};
use std::sync::{Mutex, MutexGuard};

#[test]
fn plan_returns_user_gate_and_creates_brainstorm_delivery() {
    let fixture = Fixture::new("plan-tool");
    let server = LoomMcpServer::default();

    let result = server
        .invoke_tool(
            "loom.plan",
            Some(args(json!({
                "projectRoot": fixture.root_str(),
                "requestText": "实现股票交易系统，请按模块依赖关系整理阶段优先级，每个阶段不要太大，优先按单模块能力闭环划分。"
            }))),
        )
        .expect("plan call");
    let value = structured(result);

    assert_eq!(value["state"], "user_gate");
    assert!(value.get("readGroups").is_none());
    assert_eq!(value["preResponseContract"]["required"], true);
    assert_eq!(
        value["preResponseContract"]["steps"][0]["kind"],
        "inspect_request"
    );
    assert_eq!(
        value["preResponseContract"]["steps"][1]["kind"],
        "read_required_request_groups"
    );
    assert_eq!(
        value["preResponseContract"]["steps"][2]["kind"],
        "run_knowledge_context_plan"
    );
    assert_eq!(
        value["preResponseContract"]["steps"][3]["kind"],
        "present_gate"
    );
    assert!(!value["prompt"]
        .as_str()
        .expect("prompt")
        .contains("phase_scope"));
    let prompt = value["prompt"].as_str().expect("prompt");
    assert!(prompt.contains("业务背景确认"));
    assert!(!prompt.contains("phase-1 boundary"));
    assert_eq!(value["gate"]["currentBlock"], "business_background");
    assert!(value["gate"].get("requestReadGroups").is_none());
    assert!(value["gate"].get("requiredBeforeResponse").is_none());
    let request_ref = value["requestRef"].as_str().expect("requestRef");
    assert_eq!(count_entries(&fixture.root.join(".loom/deliveries")), 1);
    let inspected = state::inspect_request(InspectRequestInput {
        project_root: fixture.root_str().to_string(),
        request_ref: request_ref.to_string(),
    })
    .expect("inspect request");
    let group_ids = inspected
        .read_groups
        .iter()
        .map(|group| group.group_id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        group_ids,
        vec![
            "conversation_protocol",
            "requirement_context",
            "requirement_full_text",
            "current_block_rules",
            "knowledge_context_plan",
            "block_confirmation_contract",
        ]
    );
    assert!(inspected.submit_tool.is_none());
    assert!(inspected.write_targets.is_empty());
    let selected = structured(
        server
            .invoke_tool(
                "readFieldGroup",
                Some(args(json!({
                    "projectRoot": fixture.root_str(),
                    "requestRef": request_ref,
                    "groupId": "knowledge_context_plan"
                }))),
            )
            .expect("read knowledge context plan"),
    );
    assert!(selected["fields"]["knowledgeQueryPlan"]["toolContract"].is_object());
    let knowledge_group = inspected
        .read_groups
        .iter()
        .find(|group| group.group_id == "knowledge_context_plan")
        .expect("knowledge_context_plan group");
    assert!(knowledge_group.required);
    let conversation_protocol = state::read_field_group(delivery_core::ReadFieldGroupInput {
        project_root: fixture.root_str().to_string(),
        request_ref: request_ref.to_string(),
        group_id: "conversation_protocol".to_string(),
    })
    .expect("read conversation protocol");
    let current_turn_rule = &conversation_protocol.fields
        ["clarificationConversationProtocol.currentTurnAnswerRule"]
        .value;
    assert_eq!(current_turn_rule["consumeCurrentUserMessage"], true);
    assert_eq!(current_turn_rule["explicitOnly"], true);
    assert!(
        current_turn_rule["meaning"]
            .as_str()
            .expect("meaning")
            .contains("而非再次询问"),
        "{current_turn_rule:#}"
    );
    assert!(
        current_turn_rule["blockSpecificRule"]
            .as_str()
            .expect("block specific rule")
            .contains("业务目标、参与者、领域上下文或约束"),
        "{current_turn_rule:#}"
    );
    let requirement_context = inspected
        .read_groups
        .iter()
        .find(|group| group.group_id == "requirement_context")
        .expect("requirement_context group");
    assert!(
        !requirement_context
            .expanded_fields()
            .contains(&"requirementContext.normalizedText".to_string()),
        "default requirement_context group must stay compact"
    );
    let full_text = inspected
        .read_groups
        .iter()
        .find(|group| group.group_id == "requirement_full_text")
        .expect("requirement_full_text group");
    assert!(!full_text.required);
    assert_eq!(
        full_text.expanded_fields(),
        vec!["requirementContext.normalizedText".to_string()]
    );
    let current_block_rules = state::read_field_group(delivery_core::ReadFieldGroupInput {
        project_root: fixture.root_str().to_string(),
        request_ref: request_ref.to_string(),
        group_id: "current_block_rules".to_string(),
    })
    .expect("read current block rules");
    let rules_text =
        serde_json::to_string(&current_block_rules.fields).expect("serialize current block rules");
    assert!(rules_text.contains("业务背景"));
    assert!(!rules_text.contains("active phase-1"));
    assert!(rules_text.contains("调用 loom.knowledgeBrainstormContext"));
    assert!(rules_text.contains("业务目标"));
    assert!(rules_text.contains("参与者或干系人"));
    assert!(rules_text.contains("领域与既有系统上下文"));
    assert!(rules_text.contains("业务或合规约束"));
    assert!(rules_text.contains("成功标准"));
    assert!(rules_text.contains("不要向用户展示内部名称，如 business_background"));
    assert!(!current_block_rules
        .fields
        .keys()
        .any(|field| field.contains("candidateWrite")));
    let knowledge_fields = state::read_request_fields(ReadRequestFieldsInput {
        project_root: fixture.root_str().to_string(),
        request_ref: request_ref.to_string(),
        fields: vec![
            "clarificationConversationProtocol.userVisibleBlockTitle".to_string(),
            "clarificationConversationProtocol.blockRule".to_string(),
            "blockConfirmationContract.tool".to_string(),
            "knowledgeQueryPlan.toolContract".to_string(),
            "knowledgeQueryPlan.sharedRules".to_string(),
            "knowledgeQueryPlan.blocks.business_background.executionOrder".to_string(),
        ],
    })
    .expect("knowledge query plan fields");
    assert_eq!(
        knowledge_fields.fields["clarificationConversationProtocol.userVisibleBlockTitle"].value,
        "业务背景确认"
    );
    assert_eq!(
        knowledge_fields.fields["blockConfirmationContract.tool"].value,
        "loom.brainstormConfirmBlock"
    );
    assert!(
        knowledge_fields.fields["clarificationConversationProtocol.blockRule"]
            .value
            .as_str()
            .unwrap_or_default()
            .contains("业务目标、参与者、领域上下文与约束")
    );
    assert_eq!(
        knowledge_fields.fields["knowledgeQueryPlan.toolContract"].value["contextTool"],
        "loom.knowledgeBrainstormContext"
    );
    assert!(knowledge_fields.fields["knowledgeQueryPlan.sharedRules"]
        .value
        .to_string()
        .contains("不要静默回退到无知识的答案"));
    assert!(knowledge_fields.fields["knowledgeQueryPlan.sharedRules"]
        .value
        .to_string()
        .contains("不要要求用户在 Brainstorm 澄清中选择、命名、启用或管理 knowledge source"));
    assert!(knowledge_fields.fields["knowledgeQueryPlan.sharedRules"]
        .value
        .to_string()
        .contains("允许空的 knowledge 结果"));
    assert!(knowledge_fields.fields["knowledgeQueryPlan.sharedRules"]
        .value
        .to_string()
        .contains("kind:text"));
    assert!(knowledge_fields.fields["knowledgeQueryPlan.sharedRules"]
        .value
        .to_string()
        .contains("object:核心业务对象"));
    assert!(!knowledge_fields.fields["knowledgeQueryPlan.sharedRules"]
        .value
        .to_string()
        .contains("object:证券账户"));
    assert!(knowledge_fields.fields
        ["knowledgeQueryPlan.blocks.business_background.executionOrder"]
        .value
        .to_string()
        .contains("业务领域、参与者、既有系统衔接与约束上下文"));
    assert_eq!(
        knowledge_fields.fields["knowledgeQueryPlan.blocks.business_background.executionOrder"]
            .value[0]["queryKind"],
        "business_background"
    );
    assert!(
        knowledge_fields.fields["knowledgeQueryPlan.toolContract"].value["conditionalInputFields"]
            ["queryId"]
            .as_str()
            .unwrap_or_default()
            .contains("per_candidate_phase_cut")
    );
}

#[test]
fn brainstorm_full_confirmation_flow_accepts_and_advances_to_technical_baseline() {
    let fixture = Fixture::new("brainstorm-full-flow");
    let server = LoomMcpServer::default();

    let planned = structured(
        server
            .invoke_tool(
                "loom.plan",
                Some(args(json!({
                    "projectRoot": fixture.root_str(),
                    "requestText": "基于股票交易系统实验大纲，第一阶段先确认证券账户模块闭环，并需要工作人员后台页面办理开户、挂失补办、销户。"
                }))),
            )
            .expect("plan call"),
    );
    assert_eq!(planned["state"], "user_gate");
    assert!(!planned["prompt"]
        .as_str()
        .expect("planned prompt")
        .contains("phase_scope"));
    let mut request_ref = planned["requestRef"]
        .as_str()
        .expect("requestRef")
        .to_string();

    structured(
        server
            .invoke_tool(
                "loom.readFieldGroup",
                Some(args(json!({
                    "projectRoot": fixture.root_str(),
                    "requestRef": request_ref,
                    "groupId": "conversation_protocol"
                }))),
            )
            .expect("read conversation protocol"),
    );
    assert!(
        server
            .invoke_tool(
                "loom.readFieldGroup",
                Some(args(json!({
                    "projectRoot": fixture.root_str(),
                    "requestRef": request_ref,
                    "groupId": "candidate_write_contract"
                }))),
            )
            .is_err(),
        "clarification request must not expose candidate_write_contract"
    );

    let business_background_confirmed = confirm_block(
        &server,
        &fixture,
        &request_ref,
        "business_background",
        "确认股票交易系统业务背景：证券账户为交易身份基础，工作人员后台办理。",
        json!({
            "businessGoal": "完成证券账户生命周期办理能力闭环。",
            "stakeholders": ["工作人员", "投资者"],
            "domainContext": "证券交易系统，证券账户是资金账户和交易链路的上游基础对象。",
            "constraints": ["开户需要资格校验", "销户前必须清空持仓"],
            "successCriteria": ["工作人员可办理开户、挂失补办、销户并看到状态回读"],
            "assumptions": []
        }),
    );
    assert_eq!(
        business_background_confirmed["gate"]["alreadyConfirmedBlocks"],
        json!(["business_background"])
    );
    request_ref = business_background_confirmed["requestRef"]
        .as_str()
        .expect("phase scope request ref")
        .to_string();

    let phase_scope_confirmed = confirm_block(
        &server,
        &fixture,
        &request_ref,
        "phase_scope",
        "确认第一阶段为证券账户模块闭环。",
        json!({
            "scope": {
                "included": ["证券账户开户", "证券账户挂失补办", "证券账户销户", "账户状态管理"],
                "deferred": ["资金账户", "交易客户端", "中央撮合"],
                "excluded": []
            },
            "recommendation": {
                "label": "证券账户模块闭环",
                "reason": "证券账户是交易身份和持仓归属的上游基础对象。"
            },
            "nextPhasePreview": "下一步确认证券账户规则和页面路径。}}"
        }),
    );
    assert_eq!(
        phase_scope_confirmed["gate"]["alreadyConfirmedBlocks"],
        json!(["business_background", "phase_scope"])
    );
    request_ref = phase_scope_confirmed["requestRef"]
        .as_str()
        .expect("concept request ref")
        .to_string();
    let clarification_state_ref = latest_ref_for_phase(
        fixture.root_str(),
        planned["deliveryId"].as_str().unwrap(),
        "brainstormClarificationState",
    );
    let clarification_state: Value = serde_json::from_str(
        &std::fs::read_to_string(fixture.root.join(clarification_state_ref))
            .expect("read clarification state"),
    )
    .expect("parse clarification state");
    assert_eq!(
        clarification_state["blocks"][0]["block"],
        "business_background"
    );
    assert_eq!(clarification_state["blocks"][1]["block"], "phase_scope");
    assert_eq!(
        clarification_state["blocks"][1]["confirmedData"]["nextPhasePreview"],
        "下一步确认证券账户规则和页面路径。"
    );
    assert_eq!(
        state::inspect_request(InspectRequestInput {
            project_root: fixture.root_str().to_string(),
            request_ref: request_ref.clone(),
        })
        .expect("inspect concept request")
        .request_kind,
        "brainstorm_clarification_block"
    );
    let concept_rules = read_block_rules_text(&server, &fixture, &request_ref);
    assert!(concept_rules.contains("业务场景"));
    assert!(concept_rules.contains("逐 scope 覆盖"));
    assert!(concept_rules.contains("决策影响排序"));
    assert!(concept_rules.contains("生命周期扫描"));
    assert!(concept_rules.contains("对象-操作摘要"));
    assert!(concept_rules.contains("不要向用户展示内部名称，如 concept_grounding"));
    assert!(concept_rules.contains("不要仅呈现名词定义"));
    assert!(concept_rules.contains("concept_grounding_scope_item"));

    request_ref = confirm_block(
        &server,
        &fixture,
        &request_ref,
        "concept_grounding",
        "确认证券账户业务规则、状态和边界。",
        json!({
            "objects": ["证券账户"],
            "operations": ["开户", "挂失补办", "销户"],
            "rules": ["开户需要资格校验", "挂失后冻结证券", "销户前必须清空持仓"],
            "boundaries": ["资金账户递延", "交易客户端递延"]
        }),
    )["requestRef"]
        .as_str()
        .expect("frontend request ref")
        .to_string();
    let frontend_rules = read_block_rules_text(&server, &fixture, &request_ref);
    assert!(frontend_rules.contains("页面操作路径"));
    assert!(frontend_rules.contains("分页/列表行为"));
    assert!(frontend_rules.contains("查询条件"));
    assert!(frontend_rules.contains("成功反馈"));
    assert!(frontend_rules.contains("业务阻断反馈"));
    assert!(frontend_rules.contains("刷新/回读"));
    assert!(frontend_rules.contains("不要向用户展示内部名称，如 frontend_experience"));
    assert!(frontend_rules.contains("不要使用硬编码的行业字段列表"));
    assert!(frontend_rules.contains("frontend_experience_page_operation_path"));

    request_ref = confirm_block(
        &server,
        &fixture,
        &request_ref,
        "frontend_experience",
        "确认工作人员后台证券账户管理页面路径。",
        json!({
            "required": true,
            "surfaces": ["证券账户管理页面"],
            "targetDiscovery": ["分页查询列表", "按账户号、姓名、证件号查询"],
            "operationPaths": ["开户从新建入口进入", "挂失补办和销户先查询并选择目标账户"],
            "mustNot": ["不能只靠内部主键触发办理动作"]
        }),
    )["requestRef"]
        .as_str()
        .expect("final summary request ref")
        .to_string();
    let final_request = state::inspect_request(InspectRequestInput {
        project_root: fixture.root_str().to_string(),
        request_ref: request_ref.to_string(),
    })
    .expect("inspect final summary request");
    let final_group_ids = final_request
        .read_groups
        .iter()
        .map(|group| group.group_id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        final_group_ids,
        vec![
            "conversation_protocol",
            "current_block_rules",
            "confirmed_clarification_state",
            "block_confirmation_contract",
        ]
    );
    assert!(!final_request
        .read_groups
        .iter()
        .any(|group| group.group_id == "knowledge_context_plan"));
    assert!(!final_request
        .read_groups
        .iter()
        .any(|group| group.group_id == "requirement_context"));
    assert!(!final_request
        .read_groups
        .iter()
        .any(|group| group.group_id == "requirement_full_text"));
    let final_rules = read_block_rules_text(&server, &fixture, &request_ref);
    assert!(final_rules.contains("提交前覆盖检查清单"));
    assert!(final_rules.contains("不要向用户展示内部名称，如 final_summary"));
    assert!(!final_rules.contains("requirementSemanticGrounding"));
    assert!(final_rules.contains("一个用户可见的覆盖检查清单，仅一个确认动作"));
    assert!(final_rules.contains("当前要提交的阶段"));
    assert!(final_rules.contains("已确认的业务规则"));
    assert!(final_rules.contains("已确认的页面操作路径"));
    assert!(final_rules.contains("不会缩窄、省略、覆盖或压缩"));
    assert!(final_rules.contains("先前确认的块细节通过结构化字段"));
    assert!(final_rules.contains("将修正纳入受影响的已有字段"));

    let write_action = confirm_block(
        &server,
        &fixture,
        &request_ref,
        "final_summary",
        "用户已确认阶段范围、业务理解、页面办理路径和提交前核对。",
        json!({
            "coverageChecklist": ["证券账户模块闭环", "开户/挂失补办/销户规则", "工作人员后台办理路径"],
            "readyToWriteCandidate": true
        }),
    );
    assert_eq!(write_action["state"], "auto_runnable", "{write_action:#}");
    let request_ref = write_action["next"]["requestRef"]
        .as_str()
        .expect("candidate write requestRef");
    let candidate_request = state::inspect_request(InspectRequestInput {
        project_root: fixture.root_str().to_string(),
        request_ref: request_ref.to_string(),
    })
    .expect("inspect candidate write request");
    let candidate_group_ids = candidate_request
        .read_groups
        .iter()
        .map(|group| group.group_id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        candidate_group_ids,
        vec![
            "confirmed_clarification_state",
            "source_ref_registry",
            "candidate_write_contract"
        ]
    );
    let confirmed_state = structured(
        server
            .invoke_tool(
                "loom.readFieldGroup",
                Some(args(json!({
                    "projectRoot": fixture.root_str(),
                    "requestRef": request_ref,
                    "groupId": "confirmed_clarification_state"
                }))),
            )
            .expect("read confirmed clarification state"),
    );
    assert!(confirmed_state["fields"]["confirmedClarificationState"].is_object());
    let source_ref_registry = structured(
        server
            .invoke_tool(
                "loom.readFieldGroup",
                Some(args(json!({
                    "projectRoot": fixture.root_str(),
                    "requestRef": request_ref,
                    "groupId": "source_ref_registry"
                }))),
            )
            .expect("read source ref registry"),
    );
    assert_eq!(
        source_ref_registry["fields"]["sourceRefRegistry"]["sources"][0]["sourceId"],
        "req-001"
    );
    assert_eq!(
        source_ref_registry["fields"]["sourceRefRegistry"]["sources"][0]["title"],
        "request_text"
    );
    assert!(
        source_ref_registry["fields"]["sourceRefRegistry"]["sources"][0]
            .get("textRef")
            .is_none()
    );
    assert!(source_ref_registry["fields"]
        .get("keywordHints.compact")
        .is_none());

    let write_contract = structured(
        server
            .invoke_tool(
                "loom.readFieldGroup",
                Some(args(json!({
                    "projectRoot": fixture.root_str(),
                    "requestRef": request_ref,
                    "groupId": "candidate_write_contract"
                }))),
            )
            .expect("read candidate write contract"),
    );
    let template = &write_contract["fields"]["outputContract"]["resultTemplate"];
    assert!(
        template.get("clarificationProgress").is_none(),
        "clarification progress is MCP-owned and must not be in the agent template"
    );
    assert!(
        template.get("userConfirmation").is_none(),
        "confirmation metadata is MCP-owned and must not be in the agent template"
    );
    assert!(template["scope"]["deferred"][0].is_object());
    assert!(template["scope"]["assumptions"][0].is_object());
    assert_eq!(
        template["phasePlan"]["nextPhasePreview"]["kind"],
        "candidate"
    );
    assert!(template["frontendExperience"]["audiences"][0]["audienceId"].is_string());
    assert!(template["frontendExperience"]["surfaces"][0]["surfaceId"].is_string());
    assert!(template["frontendExperience"]["dataViews"][0]["viewId"].is_string());
    assert!(template["frontendExperience"]["actions"][0]["actionId"].is_string());
    assert!(template["frontendExperience"]["operationPaths"][0]["pathId"].is_string());
    assert_eq!(
        write_contract["fields"]["enumRefs"]["conceptPhaseRelevance"][0],
        "current"
    );
    assert_eq!(
        write_contract["fields"]["enumRefs"]["conceptPriority"][0],
        "must_understand"
    );
    let enum_fields = &write_contract["fields"]["outputContract"]["schemaProjection"]["enumFields"];
    assert_eq!(
        enum_fields["requestSummary.complexity"],
        "enumRefs.complexity"
    );
    assert_eq!(
        enum_fields["scope.included[].source"],
        "enumRefs.scopeSource"
    );
    assert_eq!(
        enum_fields["acceptance[].priority"],
        "enumRefs.acceptancePriority"
    );
    assert_eq!(
        enum_fields["conceptGrounding.phaseConceptGrounding.concepts[].riskFactors[]"],
        "enumRefs.conceptRiskFactor"
    );
    assert_eq!(
        enum_fields["frontendExperience.actions[].resultObservation[]"],
        "enumRefs.frontendResultObservationMode"
    );
    assert_eq!(
        enum_fields["frontendExperience.operationPaths[].requiredStates[]"],
        "enumRefs.frontendInteractionState"
    );
    assert!(
        !write_contract["fields"]["enumRefs"]["frontendResultObservationMode"]
            .as_array()
            .expect("frontend result observation enum")
            .contains(&json!("empty"))
    );
    assert!(
        write_contract["fields"]["enumRefs"]["frontendInteractionState"]
            .as_array()
            .expect("frontend interaction state enum")
            .contains(&json!("empty"))
    );
    assert!(
        write_contract["fields"]["outputContract"]["schemaProjection"]["fieldContract"]
            ["properties"]["frontendExperience"]["properties"]["actions"]["items"]["properties"]
            ["resultObservation"]["constraints"][0]
            .as_str()
            .expect("result observation shape rule")
            .contains("empty 不是结果观察")
    );
    assert!(
        write_contract["fields"]["outputContract"]["schemaProjection"]["fieldContract"]
            ["properties"]["frontendExperience"]["properties"]["actions"]["items"]["properties"]
            ["resultObservation"]["constraints"][0]
            .as_str()
            .expect("result observation shape rule")
            .contains("不要在此处使用 frontendInteractionState 值")
    );
    assert!(
        write_contract["fields"]["outputContract"]["schemaProjection"]["fieldContract"]
            ["properties"]["frontendExperience"]["properties"]["operationPaths"]["items"]
            ["properties"]["requiredStates"]["constraints"][0]
            .as_str()
            .expect("required states shape rule")
            .contains("empty 仅在此处")
    );
    assert!(
        write_contract["fields"]["outputContract"]["schemaProjection"]["fieldContract"]
            ["properties"]["frontendExperience"]["properties"]["operationPaths"]["items"]
            ["properties"]["requiredStates"]["constraints"][0]
            .as_str()
            .expect("required states shape rule")
            .contains("不要在此处使用 frontendResultObservationMode 值")
    );
    assert!(write_contract["fields"]["enumRefs"]["conceptRiskFactor"]
        .as_array()
        .expect("concept risk factor enum")
        .contains(&json!("business_invariant")));
    assert!(write_contract["fields"]["rules"]["candidateWrite"]
        .to_string()
        .contains("绝不用字符串数组替换类型化对象数组"));
    assert!(write_contract["fields"]["rules"]["candidateWrite"]
        .to_string()
        .contains("scope.deferred 非空"));
    let candidate_rules = write_contract["fields"]["rules"]["candidateWrite"].to_string();
    assert!(candidate_rules.contains("而非仅从 final_summary 构建"));
    assert!(candidate_rules.contains("自检必须验证"));
    assert!(candidate_rules.contains("scope.included"));
    assert!(candidate_rules.contains("domainModel.businessFlows"));
    assert!(candidate_rules.contains("frontendExperience"));
    assert!(candidate_rules.contains("TaskPlan"));
    let mut candidate = write_contract["fields"]["outputContract"]["resultTemplate"].clone();
    populate_confirmed_brainstorm_candidate(&mut candidate);

    let inspected = candidate_request;
    assert_eq!(
        inspected.submit_tool.as_deref(),
        Some("loom.brainstormAcceptFile")
    );
    assert_eq!(inspected.write_targets.len(), 1);
    let compact_request = read_compact_request_root(&fixture, &inspected.request_id);
    for key in [
        "artifactKind",
        "submitTool",
        "writeTargets",
        "writeMode",
        "outputContract",
    ] {
        assert!(
            compact_request.get(key).is_none(),
            "candidate write compact root must not duplicate {key}: {compact_request:#}"
        );
    }
    let manifest = read_request_storage_manifest(&fixture, &inspected.request_id);
    assert!(manifest["refs"]["outputContract"].is_object());
    assert!(manifest["refs"]["rules"].is_object());
    assert!(manifest["refs"]["enumRefs"].is_object());
    let target_path = inspected.write_targets[0]["path"]
        .as_str()
        .expect("candidate target path");
    let target_file =
        from_project_relative(&fixture.root, target_path).expect("candidate target absolute path");
    write_json_atomic(&target_file, &candidate).expect("write candidate");

    let accepted = structured(
        server
            .invoke_tool(
                "loom.brainstormAcceptFile",
                Some(args(json!({
                    "projectRoot": fixture.root_str(),
                    "requestRef": request_ref,
                    "writtenTargetIds": ["candidate"]
                }))),
            )
            .expect("brainstorm accept"),
    );

    assert_eq!(accepted["state"], "user_gate", "{accepted:#}");
    assert_eq!(
        accepted["gate"]["gateId"],
        "new_project_baseline_confirmation"
    );
    assert_eq!(
        accepted["gate"]["responseContract"]["maxMessages"],
        json!(1)
    );
    let baseline_ref = accepted["requestRef"]
        .as_str()
        .expect("technical baseline request ref");
    let baseline = state::inspect_request(InspectRequestInput {
        project_root: fixture.root_str().to_string(),
        request_ref: baseline_ref.to_string(),
    })
    .expect("inspect technical baseline request");
    assert_eq!(baseline.request_kind, "technical_baseline_request");
}

fn confirm_block(
    server: &LoomMcpServer,
    fixture: &Fixture,
    request_ref: &str,
    block: &str,
    summary: &str,
    confirmed_data: Value,
) -> Value {
    read_required_request_groups(server, fixture, request_ref);
    if block != "final_summary" {
        run_knowledge_context(server, fixture, request_ref, block);
    }
    structured(
        server
            .invoke_tool(
                "loom.brainstormConfirmBlock",
                Some(args(json!({
                    "projectRoot": fixture.root_str(),
                    "requestRef": request_ref,
                    "block": block,
                    "summary": summary,
                    "confirmedData": confirmed_data
                }))),
            )
            .expect("confirm brainstorm block"),
    )
}

fn read_required_request_groups(server: &LoomMcpServer, fixture: &Fixture, request_ref: &str) {
    let inspected = state::inspect_request(InspectRequestInput {
        project_root: fixture.root_str().to_string(),
        request_ref: request_ref.to_string(),
    })
    .expect("inspect request before confirmation");
    for group in inspected.read_groups.iter().filter(|group| group.required) {
        server
            .invoke_tool(
                "loom.readFieldGroup",
                Some(args(json!({
                    "projectRoot": fixture.root_str(),
                    "requestRef": request_ref,
                    "groupId": group.group_id
                }))),
            )
            .expect("read required request group");
    }
}

fn read_block_rules_text(server: &LoomMcpServer, fixture: &Fixture, request_ref: &str) -> String {
    let rules = structured(
        server
            .invoke_tool(
                "loom.readFieldGroup",
                Some(args(json!({
                    "projectRoot": fixture.root_str(),
                    "requestRef": request_ref,
                    "groupId": "current_block_rules"
                }))),
            )
            .expect("read current block rules"),
    );
    let knowledge = server
        .invoke_tool(
            "loom.readFieldGroup",
            Some(args(json!({
                "projectRoot": fixture.root_str(),
                "requestRef": request_ref,
                "groupId": "knowledge_context_plan"
            }))),
        )
        .ok()
        .map(structured);
    format!(
        "{}\n{}",
        serde_json::to_string(&rules["fields"]).expect("serialize rules"),
        knowledge
            .as_ref()
            .and_then(|value| serde_json::to_string(&value["fields"]).ok())
            .unwrap_or_default()
    )
}

fn run_knowledge_context(
    server: &LoomMcpServer,
    fixture: &Fixture,
    request_ref: &str,
    block: &str,
) {
    let knowledge_plan = structured(
        server
            .invoke_tool(
                "loom.readFieldGroup",
                Some(args(json!({
                    "projectRoot": fixture.root_str(),
                    "requestRef": request_ref,
                    "groupId": "knowledge_context_plan"
                }))),
            )
            .expect("read knowledge context plan"),
    );
    let steps = knowledge_plan["fields"]["knowledgeQueryPlan"]["blocks"][block]["executionOrder"]
        .as_array()
        .expect("knowledge executionOrder");
    for step in steps {
        let step_id = step["stepId"].as_str().expect("stepId");
        if step["repeatMode"].as_str() == Some("per_candidate_phase_cut") {
            for query_id in ["capability_closure_A", "capability_closure_B"] {
                let result = structured(
                    server
                        .invoke_tool(
                            "loom.knowledgeBrainstormContext",
                            Some(args(json!({
                                "projectRoot": fixture.root_str(),
                                "requestRef": request_ref,
                                "block": block,
                                "stepId": step_id,
                                "queryId": query_id,
                                "querySubject": format!("{block} {step_id} {query_id}"),
                                "naturalLanguageQuery": "证券账户 开户 挂失 补办 销户 资金账户 交易 依赖 闭环",
                                "semanticFocus": ["证券账户", "开户", "挂失", "补办", "销户"]
                            }))),
                        )
                        .expect("knowledge brainstorm context"),
                );
                let details = &result["details"];
                assert!(details.get("requestRef").is_none());
                assert!(details.get("stepId").is_none());
                assert!(details.get("querySubject").is_none());
                assert!(details.get("naturalLanguageQuery").is_none());
                assert!(details.get("semanticFocus").is_none());
                assert_no_key_recursive(details, "inspect");
            }
            continue;
        }
        let result = structured(
            server
                .invoke_tool(
                    "loom.knowledgeBrainstormContext",
                    Some(args(json!({
                        "projectRoot": fixture.root_str(),
                        "requestRef": request_ref,
                        "block": block,
                        "stepId": step_id,
                        "querySubject": format!("{block} {step_id}"),
                        "naturalLanguageQuery": "证券账户 开户 挂失 补办 销户 资金账户 交易 依赖 闭环",
                        "semanticFocus": ["证券账户", "开户", "挂失", "补办", "销户"]
                    }))),
                )
                .expect("knowledge brainstorm context"),
        );
        let details = &result["details"];
        assert!(details.get("requestRef").is_none());
        assert!(details.get("stepId").is_none());
        assert!(details.get("querySubject").is_none());
        assert!(details.get("naturalLanguageQuery").is_none());
        assert!(details.get("semanticFocus").is_none());
        assert_no_key_recursive(details, "inspect");
    }
}

#[test]
fn brainstorm_confirm_block_requires_request_scoped_knowledge_context() {
    let fixture = Fixture::new("brainstorm-knowledge-required");
    let server = LoomMcpServer::default();
    let planned = structured(
        server
            .invoke_tool(
                "loom.plan",
                Some(args(json!({
                    "projectRoot": fixture.root_str(),
                    "requestText": "实现股票交易系统，第一阶段优先证券账户模块闭环。"
                }))),
            )
            .expect("plan call"),
    );
    let request_ref = planned["requestRef"].as_str().expect("requestRef");
    read_required_request_groups(&server, &fixture, request_ref);
    run_knowledge_context(&server, &fixture, request_ref, "business_background");
    let business_background_confirmed = structured(
        server
            .invoke_tool(
                "loom.brainstormConfirmBlock",
                Some(args(json!({
                    "projectRoot": fixture.root_str(),
                    "requestRef": request_ref,
                    "block": "business_background",
                    "summary": "确认股票交易系统业务背景：证券账户为交易身份基础。",
                    "confirmedData": {
                        "businessGoal": "完成证券账户生命周期办理能力闭环。",
                        "stakeholders": ["工作人员"],
                        "domainContext": "证券交易系统。",
                        "constraints": ["开户需要资格校验"],
                        "successCriteria": ["可办理开户并看到回读"],
                        "assumptions": []
                    }
                }))),
            )
            .expect("confirm business background block"),
    );
    assert_eq!(
        business_background_confirmed["state"], "user_gate",
        "{business_background_confirmed:#}"
    );
    let phase_scope_request_ref = business_background_confirmed["requestRef"]
        .as_str()
        .expect("phase scope request ref")
        .to_string();
    read_required_request_groups(&server, &fixture, &phase_scope_request_ref);
    let result = structured(
        server
            .invoke_tool(
                "loom.brainstormConfirmBlock",
                Some(args(json!({
                    "projectRoot": fixture.root_str(),
                    "requestRef": phase_scope_request_ref,
                    "block": "phase_scope",
                    "summary": "确认第一阶段为证券账户模块闭环。",
                    "confirmedData": {
                        "scope": {
                            "included": ["证券账户开户", "证券账户挂失补办", "证券账户销户"],
                            "deferred": ["资金账户", "交易客户端"],
                            "excluded": []
                        },
                        "recommendation": {
                            "label": "证券账户模块闭环",
                            "reason": "证券账户是交易身份基础。"
                        }
                    }
                }))),
            )
            .expect("confirm brainstorm block"),
    );
    assert_eq!(result["state"], "auto_runnable", "{result:#}");
    assert_eq!(result["next"]["kind"], "run_loom_tool");
    assert_eq!(
        result["next"]["toolName"],
        "loom.knowledgeBrainstormContext"
    );
    assert_eq!(result["next"]["retryTool"], "loom.brainstormConfirmBlock");
    assert!(result["agentInstruction"]
        .as_str()
        .unwrap_or_default()
        .contains("不要要求用户重新确认"));

    let dependency_empty = structured(
        server
            .invoke_tool(
                "loom.knowledgeBrainstormContext",
                Some(args(json!({
                    "projectRoot": fixture.root_str(),
                    "requestRef": phase_scope_request_ref,
                    "block": "phase_scope",
                    "stepId": "phase_scope_dependency_order",
                    "querySubject": "证券账户模块与后续资金账户、交易客户端的依赖边界",
                    "naturalLanguageQuery": "证券账户 资金账户 交易客户端 依赖 边界",
                    "semanticFocus": ["object:证券账户", "object:资金账户", "object:交易客户端"]
                }))),
            )
            .expect("dependency order knowledge context"),
    );
    assert_eq!(dependency_empty["details"]["status"], "empty");
    let closure_a_empty = structured(
        server
            .invoke_tool(
                "loom.knowledgeBrainstormContext",
                Some(args(json!({
                    "projectRoot": fixture.root_str(),
                    "requestRef": phase_scope_request_ref,
                    "block": "phase_scope",
                    "stepId": "phase_scope_capability_closure",
                    "queryId": "capability_closure_A",
                    "querySubject": "方案A：证券账户模块闭环",
                    "naturalLanguageQuery": "证券账户 开户 挂失补办 销户 闭环",
                    "semanticFocus": ["object:证券账户", "operation:开户", "operation:挂失补办", "operation:销户"]
                }))),
            )
            .expect("single capability closure knowledge context"),
    );
    assert_eq!(closure_a_empty["details"]["status"], "empty");
    let still_missing = structured(
        server
            .invoke_tool(
                "loom.brainstormConfirmBlock",
                Some(args(json!({
                    "projectRoot": fixture.root_str(),
                    "requestRef": phase_scope_request_ref,
                    "block": "phase_scope",
                    "summary": "确认第一阶段为证券账户模块闭环。",
                    "confirmedData": {
                        "scope": {
                            "included": ["证券账户开户", "证券账户挂失补办", "证券账户销户"],
                            "deferred": ["资金账户", "交易客户端"],
                            "excluded": []
                        },
                        "recommendation": {
                            "label": "证券账户模块闭环",
                            "reason": "证券账户是交易身份基础。"
                        }
                    }
                }))),
            )
            .expect("confirm brainstorm block with one closure"),
    );
    assert_eq!(still_missing["state"], "auto_runnable", "{still_missing:#}");
    assert_eq!(
        still_missing["next"]["toolName"],
        "loom.knowledgeBrainstormContext"
    );
    assert!(still_missing["agentInstruction"]
        .as_str()
        .unwrap_or_default()
        .contains("不同的 queryId"));

    let closure_b_empty = structured(
        server
            .invoke_tool(
                "loom.knowledgeBrainstormContext",
                Some(args(json!({
                    "projectRoot": fixture.root_str(),
                    "requestRef": phase_scope_request_ref,
                    "block": "phase_scope",
                    "stepId": "phase_scope_capability_closure",
                    "queryId": "capability_closure_B",
                    "querySubject": "方案B：证券账户加工作人员办理页面闭环",
                    "naturalLanguageQuery": "证券账户 工作人员界面 开户 挂失补办 销户",
                    "semanticFocus": ["object:证券账户", "page:工作人员界面", "operation:开户", "operation:销户"]
                }))),
            )
            .expect("second capability closure knowledge context"),
    );
    assert_eq!(closure_b_empty["details"]["status"], "empty");
    let confirmed = structured(
        server
            .invoke_tool(
                "loom.brainstormConfirmBlock",
                Some(args(json!({
                    "projectRoot": fixture.root_str(),
                    "requestRef": phase_scope_request_ref,
                    "block": "phase_scope",
                    "summary": "确认第一阶段为证券账户模块闭环。",
                    "confirmedData": {
                        "scope": {
                            "included": ["证券账户开户", "证券账户挂失补办", "证券账户销户"],
                            "deferred": ["资金账户", "交易客户端"],
                            "excluded": []
                        },
                        "recommendation": {
                            "label": "证券账户模块闭环",
                            "reason": "证券账户是交易身份基础。"
                        }
                    }
                }))),
            )
            .expect("confirm brainstorm block after per-candidate closure"),
    );
    assert_eq!(confirmed["state"], "user_gate", "{confirmed:#}");
    assert_eq!(confirmed["gate"]["currentBlock"], "concept_grounding");
}

#[test]
fn continue_replays_current_brainstorm_gate_after_plan() {
    let fixture = Fixture::new("continue-after-plan");
    let server = LoomMcpServer::default();

    let planned = structured(
        server
            .invoke_tool(
                "loom.plan",
                Some(args(json!({
                    "projectRoot": fixture.root_str(),
                    "requestText": "实现证券账户开户流程"
                }))),
            )
            .expect("plan call"),
    );
    let continued = structured(
        server
            .invoke_tool(
                "loom.continue",
                Some(args(json!({ "projectRoot": fixture.root_str() }))),
            )
            .expect("continue call"),
    );

    assert_eq!(continued["state"], "user_gate");
    assert_eq!(continued["requestRef"], planned["requestRef"]);
    assert!(!continued["prompt"]
        .as_str()
        .expect("continued prompt")
        .contains("phase_scope"));
    assert_eq!(continued["gate"]["currentBlock"], "business_background");
}

#[test]
fn continue_blocks_after_init_when_no_active_delivery_exists() {
    let fixture = Fixture::new("continue-tool");
    let server = LoomMcpServer::default();

    server
        .invoke_tool(
            "loom.initProject",
            Some(args(json!({ "projectRoot": fixture.root_str() }))),
        )
        .expect("initProject");
    let result = server
        .invoke_tool(
            "loom.continue",
            Some(args(json!({ "projectRoot": fixture.root_str() }))),
        )
        .expect("continue");
    let value = structured(result);

    assert_eq!(value["state"], "blocked");
    assert_eq!(value["recommendedTool"], "loom.plan");
}

fn count_entries(path: &std::path::Path) -> usize {
    std::fs::read_dir(path)
        .expect("read dir")
        .filter_map(Result::ok)
        .count()
}

fn args(value: Value) -> rmcp::model::JsonObject {
    value.as_object().cloned().expect("json object args")
}

fn structured(result: rmcp::model::CallToolResult) -> Value {
    serde_json::to_value(result).expect("call result to value")["structuredContent"].clone()
}

fn assert_no_key_recursive(value: &Value, forbidden_key: &str) {
    match value {
        Value::Object(object) => {
            assert!(
                !object.contains_key(forbidden_key),
                "{forbidden_key} must not appear in {value}"
            );
            for child in object.values() {
                assert_no_key_recursive(child, forbidden_key);
            }
        }
        Value::Array(items) => {
            for item in items {
                assert_no_key_recursive(item, forbidden_key);
            }
        }
        _ => {}
    }
}

fn read_compact_request_root(fixture: &Fixture, request_id: &str) -> Value {
    serde_json::from_str(
        &std::fs::read_to_string(
            fixture
                .root
                .join(".loom/requests")
                .join(format!("{request_id}.json")),
        )
        .expect("read compact request root"),
    )
    .expect("parse compact request root")
}

fn read_request_storage_manifest(fixture: &Fixture, request_id: &str) -> Value {
    serde_json::from_str(
        &std::fs::read_to_string(
            fixture
                .root
                .join(".loom/requests")
                .join(format!("{request_id}.manifest.json")),
        )
        .expect("read request storage manifest"),
    )
    .expect("parse request storage manifest")
}

fn populate_confirmed_brainstorm_candidate(candidate: &mut Value) {
    candidate["securityRequirement"] = json!({
        "applies": "not_applicable",
        "clientTrustModels": [],
        "sourceRefs": [],
        "rationale": "当前阶段没有受保护接口。"
    });
    candidate["requestSummary"]["title"] = json!("证券账户模块闭环");
    candidate["requestSummary"]["oneLine"] = json!("第一阶段完成证券账户生命周期办理路径。");
    candidate["requestSummary"]["businessGoal"] = json!("先完成交易身份和持仓归属账户的闭环。");
    candidate["businessBackground"] = json!({
        "businessGoal": "完成证券账户生命周期办理能力闭环。",
        "stakeholders": [{ "id": "stakeholder_1", "name": "工作人员", "role": "办理证券账户业务" }],
        "domainContext": "证券交易系统，证券账户是资金账户和交易链路的上游基础对象。",
        "constraints": ["开户需要资格校验", "销户前必须清空持仓"],
        "successCriteria": ["工作人员可办理开户、挂失补办、销户并看到状态回读"],
        "assumptions": []
    });
    candidate["scope"]["included"][0]["label"] = json!("证券账户模块闭环");
    candidate["scope"]["included"][0]["items"] = json!(["开户", "挂失补办", "销户", "状态管理"]);
    candidate["scope"]["included"][0]["reason"] =
        json!("证券账户是资金账户和交易链路的上游基础对象。");
    candidate["roadmap"]["phases"][0]["title"] = json!("证券账户模块闭环");
    candidate["roadmap"]["phases"][0]["name"] = json!("证券账户模块闭环");
    candidate["roadmap"]["phases"][0]["goal"] = json!("完成证券账户生命周期办理能力。");
    candidate["phasePlan"]["current"]["title"] = json!("证券账户模块闭环");
    candidate["phasePlan"]["current"]["goal"] =
        json!("工作人员可以办理开户、挂失补办、销户并看到状态回读。");
    candidate["acceptance"][0]["statement"] =
        json!("工作人员可以完成证券账户开户、挂失补办、销户，并看到中文反馈。");
    candidate["domainModel"]["businessFlows"] = json!([
        {
            "id": "flow_open",
            "name": "证券账户开户",
            "actors": ["工作人员"],
            "capabilityRefs": ["scope_1"],
            "summary": "录入个人或法人资料，校验开户资格，生成证券账户号。"
        },
        {
            "id": "flow_close",
            "name": "证券账户销户",
            "actors": ["工作人员"],
            "capabilityRefs": ["scope_1"],
            "summary": "销户前校验持仓清空，满足后关闭账户。"
        }
    ]);
    candidate["conceptGrounding"]["phaseConceptGrounding"]["reason"] =
        json!("用户确认证券账户与资金账户边界、挂失冻结、销户清仓规则。");
    candidate["conceptConfirmation"]["confirmationSummary"] =
        json!("用户已确认证券账户业务边界和关键阻断规则。");
    candidate["frontendExperience"]["kind"] = json!("staff_admin_workspace");
    candidate["frontendExperience"]["audiences"] = json!([{ "audienceId": "aud_staff", "name": "工作人员", "primaryJobs": ["办理证券账户业务"] }]);
    candidate["frontendExperience"]["surfaces"] = json!([{ "surfaceId": "surface_account_management", "name": "证券账户管理页面", "audienceRefs": ["aud_staff"], "primaryJobs": ["查询", "开户", "挂失补办", "销户"] }]);
    candidate["frontendExperience"]["operationPaths"] = json!([{
        "pathId": "path_manage_account",
        "name": "证券账户管理办理路径",
        "userGoal": "工作人员通过列表查询目标账户，并办理挂失补办或销户；开户从新建入口进入。",
        "surfaceRef": "surface_account_management",
        "targetObject": "证券账户",
        "selectionMode": "query_and_select",
        "selectionSummary": "开户不依赖查询；挂失补办和销户先查询并选择目标账户。",
        "dataViewRefs": [],
        "actionRefs": [],
        "requiredStates": ["success", "business_blocking", "error"],
        "sourceRefs": []
    }]);
    candidate["frontendExperience"]["mustNot"] = json!(["不能只靠内部主键触发办理动作"]);
    candidate["frontendExperience"]["confirmationSummary"] =
        json!("用户已确认工作人员后台证券账户管理页面路径。");
}

fn latest_ref_for_phase(project_root: &str, delivery_id: &str, key: &str) -> String {
    let index_path = std::path::Path::new(project_root)
        .join(".loom/deliveries")
        .join(delivery_id)
        .join("index.json");
    let index: Value =
        serde_json::from_str(&std::fs::read_to_string(index_path).expect("read delivery index"))
            .expect("parse delivery index");
    let active_phase = index["activePhaseId"].as_str().expect("active phase id");
    index["phases"]
        .as_array()
        .expect("phases")
        .iter()
        .find(|phase| phase["phaseId"] == active_phase)
        .and_then(|phase| phase["latestRefs"][key].as_str())
        .unwrap_or_else(|| panic!("missing latest ref {key}"))
        .to_string()
}

struct Fixture {
    root: std::path::PathBuf,
    _guard: MutexGuard<'static, ()>,
}

impl Fixture {
    fn new(name: &str) -> Self {
        static ENV_LOCK: Mutex<()> = Mutex::new(());
        let guard = ENV_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
        let root = std::env::temp_dir().join(format!(
            "loom-mcp-plan-{name}-{}-{}",
            std::process::id(),
            state::store::now_millis()
        ));
        std::fs::create_dir_all(&root).expect("create fixture root");
        std::env::set_var("LOOM_HOME", root.join(".loom-home"));
        Self {
            root,
            _guard: guard,
        }
    }

    fn root_str(&self) -> &str {
        self.root.to_str().expect("fixture path utf8")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
