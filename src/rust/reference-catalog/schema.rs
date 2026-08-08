use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// 参考目录顶层模型。
///
/// 当前仅从 vendor `catalog.toml` 填充。
/// 后续将在此基础上叠加 enterprise 和 project overlay。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceCatalog {
    pub schema_version: u32,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub routes: Vec<Route>,

    /// Focus tag 文本关键词规则。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub focus_rules: Vec<FocusRuleEntry>,

    /// 适用性规则(signal 是否适用于 task 的决策引擎)。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub applicability: Vec<ApplicabilityEntry>,

    /// 后端生态系统定义。
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

// ── 选择规则类型 ──

/// Focus tag 规则 — 替代 code_quality.rs 中
/// `task_focus_tags()` 函数的文本关键词匹配部分。
///
/// 每条规则描述:当 task 文本(title + objective + actions)包含
/// `keywords` 中任意一个关键词时,将 `focus_tags` 中的标签
/// 添加到 task 的 focus tag 列表。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FocusRuleEntry {
    /// 匹配成功时添加的 focus tag 列表。
    pub focus_tags: Vec<String>,

    /// 文本关键词列表。task 文本包含其中任意一个即匹配。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keywords: Vec<String>,

    /// 可选:仅当 task 为后端任务时才匹配(对应原 `cache` 规则的
    /// `task_is_backend_task(task)` 守卫)。
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub requires_backend: bool,
}

/// 适用性规则 — 替代 code_quality.rs 中
/// `signal_applies_to_task()` 函数。
///
/// 每条规则描述:当 signal 的 language 匹配 `language` 字段
/// (或 `language` 为 `"*"` 表示通配,`"none"` 表示 language 为 None),
/// 且 signal 的 roles 包含 `required_role`(如设置),
/// 则 signal 适用于拥有 `any_focus_tags` 中任意一个的 task。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicabilityEntry {
    /// 匹配的 language 值。`"*"` 表示匹配任意非 None 的 language,
    /// `"none"` 表示 language 为 None。
    pub language: String,

    /// 可选:signal roles 必须包含此角色才适用。
    /// 未设置时不对 roles 做门控。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_role: Option<String>,

    /// signal 适用于 task 的条件:task 的 focus tags 包含此列表中任意一个。
    /// 空列表表示无条件适用(但仍受 required_role 门控)。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub any_focus_tags: Vec<String>,

    /// 可选:当 required_role 未设置且 roles 为空时,
    /// signal 是否无条件适用(对应原逻辑中的 `roles.is_empty()` 分支)。
    /// 设为 true 时,any_focus_tags 仍需满足。
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub applies_when_roles_empty: bool,
}

/// 后端生态系统定义 — 替代 planning/technical_baseline.rs 中
/// `BACKEND_ECOSYSTEMS` 常量。
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

    /// 从 task 文本关键词评估 focus tags。
    ///
    /// 替代原 `task_focus_tags()` 函数的文本关键词匹配部分(Layer C)。
    /// 遍历 focus_rules,当文本包含关键词且满足守卫条件时,
    /// 将对应的 focus tags 添加到结果列表(去重)。
    ///
    /// - `text`: 已规范化的 task 文本(title + objective + actions)
    /// - `is_backend_task`: task 是否为后端任务(用于 `requires_backend` 守卫)
    pub fn focus_tags_from_text(&self, text: &str, is_backend_task: bool) -> Vec<String> {
        let mut tags = Vec::new();
        for rule in &self.focus_rules {
            if rule.requires_backend && !is_backend_task {
                continue;
            }
            if rule.keywords.iter().any(|kw| text.contains(kw)) {
                for tag in &rule.focus_tags {
                    if !tags.contains(tag) {
                        tags.push(tag.clone());
                    }
                }
            }
        }
        tags
    }

    /// 评估 signal 是否适用于给定 focus tags 的 task。
    ///
    /// 替代原 `signal_applies_to_task()` 的决策逻辑。
    /// 遍历 applicability 规则,任一匹配即返回 true。
    ///
    /// - `language`: signal 的 language(Some("sql") 等,None 表示无)
    /// - `roles`: signal 的 roles 列表
    /// - `focus_tags`: task 的 focus tags 列表
    pub fn signal_applies_to_task(
        &self,
        language: Option<&str>,
        roles: &[String],
        focus_tags: &[String],
    ) -> bool {
        let has_focus = |tag: &str| focus_tags.iter().any(|item| item == tag);
        let has_role = |role: &str| roles.iter().any(|item| item == role);
        let roles_empty = roles.is_empty();

        for rule in &self.applicability {
            // 语言匹配
            let lang_matches = match rule.language.as_str() {
                "*" => language.is_some(),
                "none" => language.is_none(),
                lang => language == Some(lang),
            };
            if !lang_matches {
                continue;
            }

            // required_role 门控
            if let Some(ref required) = rule.required_role {
                if !has_role(required) {
                    continue;
                }
            } else if rule.applies_when_roles_empty && !roles_empty {
                // applies_when_roles_empty 仅在 roles 为空时生效
                continue;
            } else if !rule.applies_when_roles_empty
                && roles_empty
                && rule.required_role.is_none()
                && rule.language != "none"
            {
                // 非 applies_when_roles_empty 且无 required_role 且 roles 为空:
                // 原逻辑中 Some(_) catch-all 的 frontend/persistence 分支需要 roles 非空
                continue;
            }

            // any_focus_tags 条件(空列表视为无条件通过)
            if rule.any_focus_tags.is_empty() || rule.any_focus_tags.iter().any(|t| has_focus(t)) {
                return true;
            }
        }
        false
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
