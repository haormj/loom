//! 参考目录:Loom 参考选择系统的声明式清单。
//!
//! `contracts`、`architecture`、`deploy`、`planning` 等 crate 中的
//! 选择代码路径通过此目录查询参考路由、focus tag 规则、适用性规则
//! 和后端生态系统定义。
//!
//! ## 架构
//!
//! - `schema.rs` — 目录 TOML 格式的 Serde 模型
//! - `vendor.rs` — vendor 目录的加载与校验
//! - `merge.rs` — 三层合并(vendor → enterprise → project)及 overlay 操作
//! - `error.rs` — 错误类型
//!
//! ## 目录文件
//!
//! vendor 目录位于 `plugins/shared/loom/references/catalog.toml`。

pub mod error;
pub mod merge;
pub mod schema;
pub mod vendor;

pub use error::{CatalogError, CatalogResult};
pub use merge::{merge_catalogs, prune_groups, replace_item_entry};
pub use schema::{
    ApplicabilityEntry, BackendEcosystemEntry, ExpandedEntry, FocusRuleEntry, Group, ItemEntry,
    PrependItem, ProviderOverlay, ReferenceCatalog, Route, SectionGroupMapping,
};
pub use vendor::{
    load_catalog, load_vendor_catalog, parse_catalog, validate_file_existence, validate_structure,
    vendor_catalog, vendor_catalog_path,
};
