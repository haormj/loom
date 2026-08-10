use std::fmt;

#[derive(Debug)]
pub struct DashboardError(pub String);

impl fmt::Display for DashboardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for DashboardError {}

pub fn serve(project_root: &str, port: u16, open_browser: bool) -> Result<(), DashboardError> {
    Err(DashboardError(format!(
        "dashboard::serve not yet implemented (project={}, port={}, open={})",
        project_root, port, open_browser
    )))
}
