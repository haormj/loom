use crate::error::{CatalogError, CatalogResult};
use crate::schema::{Group, ItemEntry, ReferenceCatalog, Route};
use std::collections::BTreeMap;

/// 合并两个目录,`overlay` 优先于 `base`。
///
/// 合并语义:
/// - **路由**:以 `id` 为键。overlay 路由完全替换 base 路由
///   (后续将增加按组的细粒度 extend/replace/prune)。
/// - **路由内的组**:以 `id` 为键。overlay 组替换 base 组。
///
/// 当前仅加载 vendor 目录,因此该函数仅由单元测试用合成
/// fixture 验证。后续将用真实 enterprise/project overlay 调用。
pub fn merge_catalogs(
    base: &ReferenceCatalog,
    overlay: &ReferenceCatalog,
) -> CatalogResult<ReferenceCatalog> {
    let mut routes: BTreeMap<String, Route> = BTreeMap::new();

    for route in &base.routes {
        routes.insert(route.id.clone(), route.clone());
    }

    for route in &overlay.routes {
        if let Some(existing) = routes.get_mut(&route.id) {
            merge_route_groups(existing, route)?;
        } else {
            routes.insert(route.id.clone(), route.clone());
        }
    }

    let mut merged = base.clone();
    merged.routes = routes.into_values().collect();
    Ok(merged)
}

/// 将 `overlay_route` 的组合并到 `base_route`。
///
/// 相同 id 的 overlay 组完全替换 base 组。
/// 新组追加到末尾。
fn merge_route_groups(base_route: &mut Route, overlay_route: &Route) -> CatalogResult<()> {
    let mut group_map: BTreeMap<String, Group> = BTreeMap::new();

    for group in &base_route.groups {
        group_map.insert(group.id.clone(), group.clone());
    }

    for group in &overlay_route.groups {
        if let Some(_existing) = group_map.get(&group.id) {
            // 完全替换(后续将支持 extend/prune 语义)
            group_map.insert(group.id.clone(), group.clone());
        } else {
            group_map.insert(group.id.clone(), group.clone());
        }
    }

    // 合并前置项(overlay 追加新项,按 refId 替换已有项)
    let mut prepend_map: BTreeMap<String, crate::schema::PrependItem> = BTreeMap::new();
    for item in &base_route.prepend_items {
        prepend_map.insert(item.ref_id.clone(), item.clone());
    }
    for item in &overlay_route.prepend_items {
        prepend_map.insert(item.ref_id.clone(), item.clone());
    }

    base_route.groups = group_map.into_values().collect();
    base_route.prepend_items = prepend_map.into_values().collect();

    // section groups:overlay 非空时替换
    if !overlay_route.section_groups.is_empty() {
        base_route.section_groups = overlay_route.section_groups.clone();
    }

    Ok(())
}

/// 执行 prune 操作:从路由中移除指定的组。
///
/// 目前由单元测试验证。
pub fn prune_groups(catalog: &mut ReferenceCatalog, route_id: &str, group_ids: &[String]) {
    if let Some(route) = catalog.routes.iter_mut().find(|r| r.id == route_id) {
        route.groups.retain(|g| !group_ids.contains(&g.id));
    }
}

/// 执行 replace 操作:替换指定组内的某个 item 条目。
///
/// 目前由单元测试验证。
pub fn replace_item_entry(
    catalog: &mut ReferenceCatalog,
    route_id: &str,
    group_id: &str,
    new_entry: ItemEntry,
) -> CatalogResult<()> {
    let route = catalog
        .routes
        .iter_mut()
        .find(|r| r.id == route_id)
        .ok_or_else(|| {
            CatalogError::Merge(format!("route '{}' not found for replace", route_id))
        })?;

    let group = route
        .groups
        .iter_mut()
        .find(|g| g.id == group_id)
        .ok_or_else(|| {
            CatalogError::Merge(format!(
                "group '{}' not found in route '{}'",
                group_id, route_id
            ))
        })?;

    // 尝试替换已有条目
    if let Some(pos) = group
        .item_entries
        .iter()
        .position(|e| e.item == new_entry.item)
    {
        group.item_entries[pos] = new_entry;
    } else {
        // 尝试从基于模板的 items 中替换
        if let Some(idx) = group.items.iter().position(|i| i == &new_entry.item) {
            group.items.remove(idx);
            group.item_entries.push(new_entry);
        } else {
            return Err(CatalogError::Merge(format!(
                "item '{}' not found in group '{}' of route '{}'",
                new_entry.item, group_id, route_id
            )));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{Group, ItemEntry, PrependItem, ReferenceCatalog, Route};

    fn make_catalog(route_id: &str, group_id: &str, items: &[&str]) -> ReferenceCatalog {
        ReferenceCatalog {
            schema_version: 1,
            routes: vec![Route {
                id: route_id.to_string(),
                reference_root: "loom".to_string(),
                groups: vec![Group {
                    id: group_id.to_string(),
                    ref_id_template: Some("tech.code.{group_id}.{item}".to_string()),
                    path_template: Some("tech/code/{group_id}/{item}.md".to_string()),
                    items: items.iter().map(|s| s.to_string()).collect(),
                    item_entries: vec![],
                    providers: vec![],
                    label: None,
                    reason_template: None,
                    required_sections: vec![],
                }],
                prepend_items: vec![],
                section_groups: vec![],
            }],
            focus_rules: vec![],
            applicability: vec![],
            backend_ecosystems: vec![],
        }
    }

    #[test]
    fn merge_adds_new_group() {
        let base = make_catalog("code", "java", &["core", "testing"]);
        let overlay = ReferenceCatalog {
            schema_version: 1,
            routes: vec![Route {
                id: "code".to_string(),
                reference_root: "loom".to_string(),
                groups: vec![Group {
                    id: "kotlin".to_string(),
                    ref_id_template: Some("tech.code.kotlin.{item}".to_string()),
                    path_template: Some("tech/code/kotlin/{item}.md".to_string()),
                    items: vec!["core".to_string(), "coroutines".to_string()],
                    item_entries: vec![],
                    providers: vec![],
                    label: None,
                    reason_template: None,
                    required_sections: vec![],
                }],
                prepend_items: vec![],
                section_groups: vec![],
            }],
            focus_rules: vec![],
            applicability: vec![],
            backend_ecosystems: vec![],
        };

        let merged = merge_catalogs(&base, &overlay).unwrap();
        let code_route = merged.routes.iter().find(|r| r.id == "code").unwrap();
        assert_eq!(code_route.groups.len(), 2);
        assert!(code_route.groups.iter().any(|g| g.id == "java"));
        assert!(code_route.groups.iter().any(|g| g.id == "kotlin"));
    }

    #[test]
    fn merge_replaces_existing_group() {
        let base = make_catalog("code", "java", &["core", "testing"]);
        let overlay = make_catalog("code", "java", &["core", "spring", "reactive"]);
        let merged = merge_catalogs(&base, &overlay).unwrap();
        let java_group = merged
            .routes
            .iter()
            .find(|r| r.id == "code")
            .unwrap()
            .groups
            .iter()
            .find(|g| g.id == "java")
            .unwrap();
        assert_eq!(java_group.items, vec!["core", "spring", "reactive"]);
    }

    #[test]
    fn prune_removes_groups() {
        let mut catalog = ReferenceCatalog {
            schema_version: 1,
            routes: vec![Route {
                id: "code".to_string(),
                reference_root: "loom".to_string(),
                groups: vec![
                    Group {
                        id: "java".to_string(),
                        ref_id_template: Some("x".to_string()),
                        path_template: Some("x".to_string()),
                        items: vec!["core".to_string()],
                        item_entries: vec![],
                        providers: vec![],
                        label: None,
                        reason_template: None,
                        required_sections: vec![],
                    },
                    Group {
                        id: "swift".to_string(),
                        ref_id_template: Some("x".to_string()),
                        path_template: Some("x".to_string()),
                        items: vec!["core".to_string()],
                        item_entries: vec![],
                        providers: vec![],
                        label: None,
                        reason_template: None,
                        required_sections: vec![],
                    },
                ],
                prepend_items: vec![],
                section_groups: vec![],
            }],
            focus_rules: vec![],
            applicability: vec![],
            backend_ecosystems: vec![],
        };

        prune_groups(&mut catalog, "code", &["swift".to_string()]);
        let code_route = catalog.routes.iter().find(|r| r.id == "code").unwrap();
        assert_eq!(code_route.groups.len(), 1);
        assert!(code_route.groups.iter().all(|g| g.id == "java"));
    }

    #[test]
    fn replace_item_entry_swaps_path() {
        let mut catalog = make_catalog("code", "java", &["core", "testing"]);
        let new_entry = ItemEntry {
            item: "core".to_string(),
            ref_id: "tech.code.java.core".to_string(),
            path: "tech/code/java/core.md".to_string(),
            reason: Some("Enterprise Java core".to_string()),
            condition: None,
        };
        replace_item_entry(&mut catalog, "code", "java", new_entry).unwrap();

        let java_group = catalog
            .routes
            .iter()
            .find(|r| r.id == "code")
            .unwrap()
            .groups
            .iter()
            .find(|g| g.id == "java")
            .unwrap();
        assert!(!java_group.items.contains(&"core".to_string()));
        assert_eq!(java_group.items, vec!["testing"]);
        assert_eq!(java_group.item_entries[0].item, "core");
    }

    #[test]
    fn merge_prepend_items_deduplicates_by_refid() {
        let mut base = make_catalog("code", "java", &["core"]);
        base.routes[0].prepend_items.push(PrependItem {
            ref_id: "tech.code.common".to_string(),
            path: "tech/code/common.md".to_string(),
            reason: "base".to_string(),
            condition: "when_groups_non_empty".to_string(),
        });

        let mut overlay = make_catalog("code", "java", &["core"]);
        overlay.routes[0].prepend_items.push(PrependItem {
            ref_id: "tech.code.common".to_string(),
            path: "tech/code/common.md".to_string(),
            reason: "enterprise override".to_string(),
            condition: "when_groups_non_empty".to_string(),
        });

        let merged = merge_catalogs(&base, &overlay).unwrap();
        let code_route = merged.routes.iter().find(|r| r.id == "code").unwrap();
        assert_eq!(code_route.prepend_items.len(), 1);
        assert_eq!(code_route.prepend_items[0].reason, "enterprise override");
    }
}
