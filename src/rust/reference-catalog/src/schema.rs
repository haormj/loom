use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// 参考目录顶层模型。
///
/// Phase 0 中仅从 vendor `catalog.toml` 填充。
/// 后续阶段将在此基础上叠加 enterprise 和 project overlay。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceCatalog {
    pub schema_version: u32,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub routes: Vec<Route>,

    /// Phase 2 占位 — vendor 目录中当前为空。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stack_signals: Vec<StackSignalEntry>,

    /// Phase 2 占位 — vendor 目录中当前为空。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub focus_rules: Vec<FocusRuleEntry>,

    /// Phase 2 占位 — vendor 目录中当前为空。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub applicability: Vec<ApplicabilityEntry>,

    /// 后端生态系统定义(Phase 2)。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub backend_ecosystems: Vec<BackendEcosystemEntry>,
}

/// 一条参考路由(code、api、arch、uix、browser、deploy)。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Route {
    pub id: String,

    /// 该路由解析时对应的共享参考树。
    /// `"loom"`(默认)→ `plugins/shared/loom/references/`
    /// `"loom-deploy"` → `plugins/shared/loom-deploy/references/`
    #[serde(default)]
    pub reference_root: String,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub groups: Vec<Group>,

    /// 路由级前置项,始终添加到加载计划头部
    /// (如 code 路由前置 `tech/code/common.md`)。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prepend_items: Vec<PrependItem>,

    /// 架构 section → item 映射(Foundation → [core, patterns, system] 等)。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub section_groups: Vec<SectionGroupMapping>,
}

/// 路由内的一个参考组(如 `springboot`、`react`、`api`、`arch`)。
///
/// 遵循统一 refId/path 模式的组使用 `ref_id_template`、`path_template`
/// 和 `items`。需要逐项覆盖的组使用 `item_entries`。
/// 一个组可以同时使用两者(如 `sql` 既有通用 item 又有 provider overlay)。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    pub id: String,

    /// 模板如 `"bk.spring.{item}"` — `{item}` 替换为每个 item id。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ref_id_template: Option<String>,

    /// 模板如 `"tech/backend/springboot/{item}.md"`。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_template: Option<String>,

    /// 遵循上述模板的简单 item id 列表。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<String>,

    /// 显式逐项条目,带覆盖的 refId/path/reason。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub item_entries: Vec<ItemEntry>,

    /// 两级组(如 SQL 方言)的 provider overlay。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub providers: Vec<ProviderOverlay>,

    /// reason 字符串中使用的人类可读标签(如 "Spring Boot")。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,

    /// reason 生成模板,支持占位符 `{label}`、`{item}`、`{group_id}`。
    /// 如 `"Selected {label} {item} framework quality reference for this task."`。
    /// 未设置时 `expand_all()` 的 reason 为 `None`,由调用方自行生成。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason_template: Option<String>,

    /// 每个参考文件中必须包含的最少 `##` section。
    /// enterprise overlay 可以追加但不可移除 vendor 条目。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_sections: Vec<String>,
}

/// 显式逐项参考条目(用于非标准 refId/path 映射)。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemEntry {
    pub item: String,
    pub ref_id: String,
    pub path: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,

    /// 选择条件,用于文档说明(如 "always"、"topology_needed")。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
}

/// 两级组内的 provider overlay(如 SQL 下的 MySQL)。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderOverlay {
    pub id: String,
    pub ref_id_template: String,
    pub path_template: String,
    pub subjects: Vec<String>,
    pub label: String,

    /// reason 生成模板,支持占位符 `{label}`、`{subject}`。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason_template: Option<String>,
}

/// 路由级前置参考(如 `tech/code/common.md`)。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrependItem {
    pub ref_id: String,
    pub path: String,
    pub reason: String,

    /// 何时包含此项(如 "when_groups_non_empty")。
    pub condition: String,
}

/// 架构 section → items 映射。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SectionGroupMapping {
    pub id: String,
    pub items: Vec<String>,
}

// ── Phase 2 占位类型(schema 已定义,数据在 Phase 2 填充)──

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StackSignalEntry {
    pub match_keywords: Vec<String>,
    pub track: String,
    pub language: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub frameworks: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FocusRuleEntry {
    pub focus_tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub task_kinds: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keywords: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicabilityEntry {
    pub signal: String,
    pub requires_any_focus: Vec<String>,
}

/// 后端生态系统定义(Phase 2 — 替代 planning/technical_baseline.rs 中的
/// `BACKEND_ECOSYSTEMS` 常量)。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackendEcosystemEntry {
    pub ecosystem_id: String,
    pub label: String,
    pub runtime_family: String,
    pub backend_options: Vec<String>,
    pub backend_matchers: Vec<String>,
    pub data_access_options: Vec<String>,
    pub data_access_matchers: Vec<String>,
}

// ── 展开辅助 ──

/// 根据 `reason_template` 模板和占位符值生成 reason 字符串。
///
/// 支持的占位符:`{label}`、`{item}`、`{group_id}`、`{subject}`。
/// `subject` 非空时同时替换 `{subject}`。
fn expand_reason(
    reason_template: &Option<String>,
    label: Option<&str>,
    group_id: &str,
    item: &str,
    subject: Option<&str>,
) -> Option<String> {
    reason_template.as_ref().map(|tmpl| {
        let mut result = tmpl
            .replace("{label}", label.unwrap_or(group_id))
            .replace("{item}", item)
            .replace("{group_id}", group_id);
        if let Some(sub) = subject {
            result = result.replace("{subject}", sub);
        }
        result
    })
}

/// 完全展开后的参考条目(不含模板占位符)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpandedEntry {
    pub route_id: String,
    pub group_id: String,
    pub item: String,
    pub ref_id: String,
    pub path: String,
    pub reason: Option<String>,
}

impl ReferenceCatalog {
    /// 将所有 routes/groups/items 展开为扁平的 `(route, group, item, refId, path)` 元组。
    pub fn expand_all(&self) -> Vec<ExpandedEntry> {
        let mut entries = Vec::new();
        for route in &self.routes {
            for group in &route.groups {
                // 基于模板的 item
                if let (Some(ref_tmpl), Some(path_tmpl)) =
                    (&group.ref_id_template, &group.path_template)
                {
                    for item in &group.items {
                        entries.push(ExpandedEntry {
                            route_id: route.id.clone(),
                            group_id: group.id.clone(),
                            item: item.clone(),
                            ref_id: ref_tmpl.replace("{item}", item),
                            path: path_tmpl.replace("{item}", item),
                            reason: expand_reason(
                                &group.reason_template,
                                group.label.as_deref(),
                                &group.id,
                                item,
                                None,
                            ),
                        });
                    }
                }

                // 显式 item 条目
                for entry in &group.item_entries {
                    let reason = entry.reason.clone().or_else(|| {
                        expand_reason(
                            &group.reason_template,
                            group.label.as_deref(),
                            &group.id,
                            &entry.item,
                            None,
                        )
                    });
                    entries.push(ExpandedEntry {
                        route_id: route.id.clone(),
                        group_id: group.id.clone(),
                        item: entry.item.clone(),
                        ref_id: entry.ref_id.clone(),
                        path: entry.path.clone(),
                        reason,
                    });
                }

                // Provider overlay(如 SQL 的两级组)
                for provider in &group.providers {
                    for subject in &provider.subjects {
                        entries.push(ExpandedEntry {
                            route_id: route.id.clone(),
                            group_id: group.id.clone(),
                            item: format!("{}.{}", provider.id, subject),
                            ref_id: provider.ref_id_template.replace("{subject}", subject),
                            path: provider.path_template.replace("{subject}", subject),
                            reason: expand_reason(
                                &provider.reason_template,
                                Some(&provider.label),
                                &provider.id,
                                subject,
                                Some(subject),
                            ),
                        });
                    }
                }
            }
        }
        entries
    }

    /// 收集指定路由的所有 group id。
    pub fn group_ids_for_route(&self, route_id: &str) -> Vec<String> {
        self.routes
            .iter()
            .find(|r| r.id == route_id)
            .map(|r| r.groups.iter().map(|g| g.id.clone()).collect())
            .unwrap_or_default()
    }

    /// 收集指定路由+group 的所有 item(包括 provider overlay)。
    pub fn items_for_group(&self, route_id: &str, group_id: &str) -> Vec<String> {
        let Some(route) = self.routes.iter().find(|r| r.id == route_id) else {
            return Vec::new();
        };
        let Some(group) = route.groups.iter().find(|g| g.id == group_id) else {
            return Vec::new();
        };
        let mut items = group.items.clone();
        for entry in &group.item_entries {
            if !items.contains(&entry.item) {
                items.push(entry.item.clone());
            }
        }
        for provider in &group.providers {
            for subject in &provider.subjects {
                items.push(format!("{}.{}", provider.id, subject));
            }
        }
        items
    }

    /// 收集路由的前置项。
    pub fn prepend_items_for_route(&self, route_id: &str) -> &[PrependItem] {
        self.routes
            .iter()
            .find(|r| r.id == route_id)
            .map(|r| r.prepend_items.as_slice())
            .unwrap_or(&[])
    }

    /// 构建路由的 group_id → items 映射(镜像 `knownReferenceGroups`)。
    pub fn known_reference_groups(&self, route_id: &str) -> BTreeMap<String, Vec<String>> {
        let Some(route) = self.routes.iter().find(|r| r.id == route_id) else {
            return BTreeMap::new();
        };
        let mut map = BTreeMap::new();
        for group in &route.groups {
            map.insert(group.id.clone(), self.items_for_group(route_id, &group.id));
        }
        map
    }

    /// 查找指定 (route, group, item) 的展开条目。
    ///
    /// 依次检查:基于模板的 items → 显式 item_entries → provider overlay。
    /// 返回 `ExpandedEntry`(含 refId、path、reason)。
    pub fn resolve_entry(
        &self,
        route_id: &str,
        group_id: &str,
        item: &str,
    ) -> Option<ExpandedEntry> {
        let route = self.routes.iter().find(|r| r.id == route_id)?;
        let group = route.groups.iter().find(|g| g.id == group_id)?;

        // 基于模板的 item
        if let (Some(ref_tmpl), Some(path_tmpl)) = (&group.ref_id_template, &group.path_template) {
            if group.items.iter().any(|i| i == item) {
                return Some(ExpandedEntry {
                    route_id: route.id.clone(),
                    group_id: group.id.clone(),
                    item: item.to_string(),
                    ref_id: ref_tmpl.replace("{item}", item),
                    path: path_tmpl.replace("{item}", item),
                    reason: expand_reason(
                        &group.reason_template,
                        group.label.as_deref(),
                        &group.id,
                        item,
                        None,
                    ),
                });
            }
        }

        // 显式 item 条目
        if let Some(entry) = group.item_entries.iter().find(|e| e.item == item) {
            let reason = entry.reason.clone().or_else(|| {
                expand_reason(
                    &group.reason_template,
                    group.label.as_deref(),
                    &group.id,
                    &entry.item,
                    None,
                )
            });
            return Some(ExpandedEntry {
                route_id: route.id.clone(),
                group_id: group.id.clone(),
                item: entry.item.clone(),
                ref_id: entry.ref_id.clone(),
                path: entry.path.clone(),
                reason,
            });
        }

        // Provider overlay(如 `mysql.schema`)
        if let Some((provider_id, subject)) = item.split_once('.') {
            if let Some(provider) = group.providers.iter().find(|p| p.id == provider_id) {
                if provider.subjects.iter().any(|s| s == subject) {
                    return Some(ExpandedEntry {
                        route_id: route.id.clone(),
                        group_id: group.id.clone(),
                        item: item.to_string(),
                        ref_id: provider.ref_id_template.replace("{subject}", subject),
                        path: provider.path_template.replace("{subject}", subject),
                        reason: expand_reason(
                            &provider.reason_template,
                            Some(&provider.label),
                            &provider.id,
                            subject,
                            Some(subject),
                        ),
                    });
                }
            }
        }

        None
    }

    /// 查找路由的 section → items 映射(仅 arch 路由使用)。
    pub fn section_groups_for_route(&self, route_id: &str) -> Option<&[SectionGroupMapping]> {
        self.routes
            .iter()
            .find(|r| r.id == route_id)
            .map(|r| r.section_groups.as_slice())
    }

    /// 返回后端生态系统定义列表。
    pub fn backend_ecosystems(&self) -> &[BackendEcosystemEntry] {
        &self.backend_ecosystems
    }
}

impl Route {
    /// 未设置时返回 `"loom"`,否则返回配置的根。
    pub fn effective_reference_root(&self) -> &str {
        if self.reference_root.is_empty() {
            "loom"
        } else {
            &self.reference_root
        }
    }
}

impl Default for Route {
    fn default() -> Self {
        Self {
            id: String::new(),
            reference_root: String::new(),
            groups: Vec::new(),
            prepend_items: Vec::new(),
            section_groups: Vec::new(),
        }
    }
}
