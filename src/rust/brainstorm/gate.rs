use contracts::ClarificationBlockName;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrainstormGate {
    pub gate_id: String,
    pub current_block: ClarificationBlockName,
    pub required_blocks: Vec<ClarificationBlockName>,
    pub already_confirmed_blocks: Vec<ClarificationBlockName>,
    pub skipped_blocks: Vec<SkippedBlockSummary>,
    pub user_message: String,
    pub response_rule: BrainstormResponseRule,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub issues: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkippedBlockSummary {
    pub block: ClarificationBlockName,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrainstormResponseRule {
    pub mode: String,
    pub final_summary_required_before_write: bool,
    pub user_visible_confirmation_required: bool,
}

pub fn required_blocks() -> Vec<ClarificationBlockName> {
    vec![
        ClarificationBlockName::BusinessBackground,
        ClarificationBlockName::PhaseScope,
        ClarificationBlockName::ConceptGrounding,
        ClarificationBlockName::FrontendExperience,
        ClarificationBlockName::FinalSummary,
    ]
}

pub fn gate_for_block(
    block: ClarificationBlockName,
    already_confirmed_blocks: Vec<ClarificationBlockName>,
    skipped_blocks: Vec<SkippedBlockSummary>,
) -> BrainstormGate {
    BrainstormGate {
        gate_id: format!("gate_{}", block_id(&block)),
        current_block: block.clone(),
        required_blocks: required_blocks(),
        already_confirmed_blocks,
        skipped_blocks,
        user_message: block_message(&block),
        response_rule: BrainstormResponseRule {
            mode: "progressive_brainstorm".to_string(),
            final_summary_required_before_write: true,
            user_visible_confirmation_required: true,
        },
        issues: vec![],
    }
}

pub fn block_id(block: &ClarificationBlockName) -> &'static str {
    match block {
        ClarificationBlockName::BusinessBackground => "business_background",
        ClarificationBlockName::PhaseScope => "phase_scope",
        ClarificationBlockName::ConceptGrounding => "concept_grounding",
        ClarificationBlockName::FrontendExperience => "frontend_experience",
        ClarificationBlockName::FinalSummary => "final_summary",
    }
}

pub fn to_value(gate: &BrainstormGate) -> Value {
    serde_json::to_value(gate).unwrap_or_else(|_| serde_json::json!({}))
}

pub fn required_knowledge_step_ids(block: &ClarificationBlockName) -> &'static [&'static str] {
    match block {
        ClarificationBlockName::BusinessBackground => &["business_background_context"],
        ClarificationBlockName::PhaseScope => &[
            "phase_scope_dependency_order",
            "phase_scope_capability_closure",
        ],
        ClarificationBlockName::ConceptGrounding => &["concept_grounding_scope_item"],
        ClarificationBlockName::FrontendExperience => &["frontend_experience_page_operation_path"],
        ClarificationBlockName::FinalSummary => &[],
    }
}

pub fn block_message(block: &ClarificationBlockName) -> String {
    match block {
        ClarificationBlockName::BusinessBackground => {
            "阅读当前块的知识计划，查询 request-scoped knowledge，然后用用户语言呈现业务目标、参与者、领域上下文与约束，等待用户可见确认后进入阶段范围确认。不要向用户展示内部 block ids。".to_string()
        }
        ClarificationBlockName::PhaseScope => {
            "阅读当前块的知识计划，查询 request-scoped knowledge，然后用用户语言呈现 2-3 个当前阶段边界选项，而非完整的多阶段项目路线图。等待用户可见确认后，继续进入业务理解与规则确认。不要向用户展示内部 block ids。".to_string()
        }
        ClarificationBlockName::ConceptGrounding => {
            "阅读当前块的知识计划，查询 request-scoped knowledge，然后确认用户已确认的当前范围内的业务对象、操作、规则、字段、阻断条件、结果和误解边界。使用用户可见标题，如\"业务理解与规则确认\"。".to_string()
        }
        ClarificationBlockName::FrontendExperience => {
            "阅读当前块的知识计划，查询 request-scoped knowledge，然后确认页面或工作空间的操作路径、目标发现、操作入口、反馈和回读，或明确记录 UI 不适用的原因。使用用户可见标题，如\"页面办理路径确认\"。".to_string()
        }
        ClarificationBlockName::FinalSummary => {
            "呈现提交前覆盖检查清单，将用户修正回写到结构化字段，然后在写入最终结构化需求结果前确认。不要向用户展示内部 block ids。".to_string()
        }
    }
}
