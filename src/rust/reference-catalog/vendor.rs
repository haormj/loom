use crate::error::{CatalogError, CatalogResult};
use crate::merge::merge_catalogs;
use crate::schema::ReferenceCatalog;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// 进程级缓存的 vendor 目录实例。
///
/// 首次调用时从磁盘加载并解析,后续调用直接返回缓存的引用。
static VENDOR_CATALOG: OnceLock<ReferenceCatalog> = OnceLock::new();

/// 进程级缓存的 resolved 目录实例(vendor + enterprise overlay)。
///
/// 首次调用 `resolved_catalog()` 时构建:加载 vendor 基线,若环境变量
/// `LOOM_CATALOG_OVERLAY` 指向的 overlay 文件存在则合并。后续调用返回同一引用。
static RESOLVED_CATALOG: OnceLock<ReferenceCatalog> = OnceLock::new();

/// 环境变量名:指向 enterprise overlay TOML 文件的路径。
pub const CATALOG_OVERLAY_ENV: &str = "LOOM_CATALOG_OVERLAY";

/// 返回进程级缓存的 vendor 目录。
///
/// 首次调用时从 `catalog.toml` 加载;后续调用返回同一引用。
/// 如果加载失败会 panic(目录文件是构建时固定的,不应在运行时失败)。
///
/// 注意:此函数**只返回 vendor 基线**,不会合并 enterprise overlay。
/// 选择代码路径应使用 `resolved_catalog()` 获取合并后的目录。
pub fn vendor_catalog() -> &'static ReferenceCatalog {
    VENDOR_CATALOG
        .get_or_init(|| load_vendor_catalog().expect("vendor catalog must be loadable at runtime"))
}

/// 返回进程级缓存的 resolved 目录(vendor + enterprise overlay)。
///
/// 构建逻辑:
/// 1. 加载 vendor 基线目录。
/// 2. 读取环境变量 `LOOM_CATALOG_OVERLAY`。
///    - 未设置或路径不存在 → 静默回退到 vendor 基线。
///    - 路径存在 → 解析 overlay 并通过 `merge_catalogs` 合并到 vendor 之上。
/// 3. 合并结果缓存到进程级 `OnceLock`,后续调用返回同一引用。
///
/// overlay 解析或合并失败会 panic(企业配置错误应尽早暴露,而非静默降级)。
/// 企业可在此前用 `validate_structure` 自行校验 overlay 完整性。
///
/// 所有选择代码路径(contracts、architecture、deploy、planning)应通过此函数
/// 访问目录数据,以使 enterprise overlay 自动生效。
pub fn resolved_catalog() -> &'static ReferenceCatalog {
    RESOLVED_CATALOG.get_or_init(|| {
        let vendor = load_vendor_catalog().expect("vendor catalog must be loadable at runtime");
        let overlay_path = std::env::var(CATALOG_OVERLAY_ENV)
            .ok()
            .filter(|s| !s.trim().is_empty())
            .map(PathBuf::from);
        build_resolved_catalog(&vendor, overlay_path.as_deref())
    })
}

/// 构建 resolved 目录的纯函数核心逻辑,可独立测试。
///
/// - `vendor`:已加载的 vendor 基线目录。
/// - `overlay_path`:enterprise overlay 文件路径。
///   - `None` → 返回 vendor 副本。
///   - 路径不存在 → 返回 vendor 副本(静默回退)。
///   - 路径存在但解析/合并失败 → 返回 `CatalogError`(供测试断言)。
///
/// `resolved_catalog()` 在运行时对错误做 panic 包装;此函数暴露 Result
/// 以便测试验证错误路径。
pub fn build_resolved_catalog(
    vendor: &ReferenceCatalog,
    overlay_path: Option<&Path>,
) -> ReferenceCatalog {
    let Some(path) = overlay_path else {
        log::debug!(
            "catalog overlay: env var {} not set, using vendor baseline only (routes={})",
            CATALOG_OVERLAY_ENV,
            vendor.routes.len()
        );
        return vendor.clone();
    };
    if !path.exists() {
        log::warn!(
            "catalog overlay: path {} does not exist, falling back to vendor baseline (routes={})",
            path.display(),
            vendor.routes.len()
        );
        return vendor.clone();
    }
    log::info!(
        "catalog overlay: loading enterprise overlay from {} (vendor routes={})",
        path.display(),
        vendor.routes.len()
    );
    let overlay = match load_catalog(path) {
        Ok(c) => {
            log::info!(
                "catalog overlay: parsed overlay successfully (routes={} groups_total={})",
                c.routes.len(),
                c.routes.iter().map(|r| r.groups.len()).sum::<usize>()
            );
            c
        }
        Err(e) => {
            log::error!(
                "catalog overlay: failed to load overlay from {}: {}",
                path.display(),
                e
            );
            panic!(
                "enterprise catalog overlay at {} failed to load: {}",
                path.display(),
                e
            );
        }
    };
    let vendor_route_count = vendor.routes.len();
    let vendor_group_count: usize = vendor.routes.iter().map(|r| r.groups.len()).sum();
    let merged = merge_catalogs(vendor, &overlay).unwrap_or_else(|err| {
        log::error!(
            "catalog overlay: failed to merge overlay at {}: {}",
            path.display(),
            err
        );
        panic!(
            "enterprise catalog overlay at {} failed to merge: {}",
            path.display(),
            err
        )
    });
    log::info!(
        "catalog overlay: merge complete — vendor routes={} groups={} → resolved routes={} groups={}",
        vendor_route_count,
        vendor_group_count,
        merged.routes.len(),
        merged.routes.iter().map(|r| r.groups.len()).sum::<usize>()
    );
    merged
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
