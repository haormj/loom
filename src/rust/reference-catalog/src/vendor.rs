use crate::error::{CatalogError, CatalogResult};
use crate::schema::ReferenceCatalog;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// 进程级缓存的 vendor 目录实例。
///
/// 首次调用时从磁盘加载并解析,后续调用直接返回缓存的引用。
/// Phase 1 中所有选择代码路径通过此函数访问目录数据。
static VENDOR_CATALOG: OnceLock<ReferenceCatalog> = OnceLock::new();

/// 返回进程级缓存的 vendor 目录。
///
/// 首次调用时从 `catalog.toml` 加载;后续调用返回同一引用。
/// 如果加载失败会 panic(目录文件是构建时固定的,不应在运行时失败)。
pub fn vendor_catalog() -> &'static ReferenceCatalog {
    VENDOR_CATALOG
        .get_or_init(|| load_vendor_catalog().expect("vendor catalog must be loadable at runtime"))
}

/// 从 TOML 文件加载目录。
pub fn load_catalog(path: &Path) -> CatalogResult<ReferenceCatalog> {
    let contents = std::fs::read_to_string(path).map_err(|source| CatalogError::FileRead {
        path: path.display().to_string(),
        source,
    })?;
    parse_catalog(&contents, &path.display().to_string())
}

/// 从 TOML 字符串解析目录。
pub fn parse_catalog(contents: &str, source_path: &str) -> CatalogResult<ReferenceCatalog> {
    toml::from_str(contents).map_err(|source| CatalogError::TomlParse {
        path: source_path.to_string(),
        source,
    })
}

/// 相对于 crate 根定位 vendor 目录文件。
///
/// vendor 目录位于仓库源码树的
/// `plugins/shared/loom/references/catalog.toml`。
/// 作为 workspace 成员编译时,`CARGO_MANIFEST_DIR` 指向
/// `src/rust/reference_catalog`。
pub fn vendor_catalog_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../plugins/shared/loom/references/catalog.toml")
}

/// 加载 vendor 目录。
pub fn load_vendor_catalog() -> CatalogResult<ReferenceCatalog> {
    let path = vendor_catalog_path();
    load_catalog(&path)
}

/// 校验目录中每个展开路径在磁盘上对应的共享参考目录下有真实文件。
///
/// `repo_root` 应指向仓库根目录(包含 `plugins/shared/` 的目录)。
pub fn validate_file_existence(catalog: &ReferenceCatalog, repo_root: &Path) -> CatalogResult<()> {
    let mut missing: Vec<String> = Vec::new();

    for entry in catalog.expand_all() {
        let route = catalog
            .routes
            .iter()
            .find(|r| r.id == entry.route_id)
            .unwrap_or_else(|| panic!("route {} not found", entry.route_id));

        let shared_dir = match route.effective_reference_root() {
            "loom" => "plugins/shared/loom/references",
            "loom-deploy" => "plugins/shared/loom-deploy/references",
            other => {
                return Err(CatalogError::Validation(format!(
                    "route {}: unknown reference_root '{}'",
                    entry.route_id, other
                )))
            }
        };

        let file_path = repo_root.join(shared_dir).join(&entry.path);
        if !file_path.exists() {
            missing.push(format!(
                "route={} group={} item={} path={}",
                entry.route_id, entry.group_id, entry.item, entry.path
            ));
        }
    }

    // 同时校验前置项
    for route in &catalog.routes {
        let shared_dir = match route.effective_reference_root() {
            "loom" => "plugins/shared/loom/references",
            "loom-deploy" => "plugins/shared/loom-deploy/references",
            _ => continue,
        };
        for prepend in &route.prepend_items {
            let file_path = repo_root.join(shared_dir).join(&prepend.path);
            if !file_path.exists() {
                missing.push(format!(
                    "route={} prepend refId={} path={}",
                    route.id, prepend.ref_id, prepend.path
                ));
            }
        }
    }

    if missing.is_empty() {
        Ok(())
    } else {
        Err(CatalogError::Validation(format!(
            "{} reference file(s) missing:\n{}",
            missing.len(),
            missing.join("\n")
        )))
    }
}

/// 校验目录结构完整性:refId 唯一、组非空、模板格式正确。
pub fn validate_structure(catalog: &ReferenceCatalog) -> CatalogResult<()> {
    let mut ref_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut duplicates: Vec<String> = Vec::new();

    for entry in catalog.expand_all() {
        if !ref_ids.insert(entry.ref_id.clone()) {
            duplicates.push(entry.ref_id.clone());
        }
    }

    for route in &catalog.routes {
        for prepend in &route.prepend_items {
            if !ref_ids.insert(prepend.ref_id.clone()) {
                duplicates.push(prepend.ref_id.clone());
            }
        }
    }

    if !duplicates.is_empty() {
        return Err(CatalogError::Validation(format!(
            "duplicate refId(s): {}",
            duplicates.join(", ")
        )));
    }

    // 确保每条路由至少有一个组
    for route in &catalog.routes {
        if route.groups.is_empty() && route.prepend_items.is_empty() {
            return Err(CatalogError::Validation(format!(
                "route '{}' has no groups and no prepend items",
                route.id
            )));
        }
    }

    Ok(())
}
