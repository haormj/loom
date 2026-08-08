use thiserror::Error;

#[derive(Debug, Error)]
pub enum CatalogError {
    #[error("读取目录文件失败 {path}: {source}")]
    FileRead {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("解析目录 TOML 失败 {path}: {source}")]
    TomlParse {
        path: String,
        #[source]
        source: toml::de::Error,
    },

    #[error("目录校验失败: {0}")]
    Validation(String),

    #[error("目录合并失败: {0}")]
    Merge(String),
}

pub type CatalogResult<T> = Result<T, CatalogError>;
