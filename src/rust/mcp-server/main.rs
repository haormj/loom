use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logging()?;
    mcp_server::run_stdio_server().await
}

fn init_logging() -> anyhow::Result<()> {
    let log_dir = loom_log_dir();
    std::fs::create_dir_all(&log_dir)?;
    let log_file = log_dir.join("loom-mcp.log");

    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)?;

    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("loom=info,knowledge=info"),
    )
    .format_timestamp_millis()
    .target(env_logger::Target::Pipe(Box::new(file)))
    .format(|buf, record| {
        writeln!(
            buf,
            "[{} {} {}] {}",
            buf.timestamp_millis(),
            record.level(),
            record.target(),
            record.args()
        )
    })
    .init();

    Ok(())
}

fn loom_log_dir() -> PathBuf {
    let loom_home = std::env::var("LOOM_HOME")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let home = std::env::var("HOME")
                .or_else(|_| std::env::var("USERPROFILE"))
                .unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join(".loom")
        });
    loom_home.join("log")
}
