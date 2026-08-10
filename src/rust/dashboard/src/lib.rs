use std::fmt;

pub mod embedded;
pub mod reader;
pub mod routes;
pub mod server;

#[derive(Debug)]
pub struct DashboardError(pub String);

impl fmt::Display for DashboardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for DashboardError {}

pub async fn serve(
    project_root: &str,
    port: u16,
    open_browser: bool,
) -> Result<(), DashboardError> {
    server::serve(project_root.to_string(), port, open_browser).await
}

pub fn build_router(project_root: String) -> axum::Router {
    server::build_router(project_root)
}
