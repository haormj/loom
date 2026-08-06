use std::fs::OpenOptions;
use std::io::Write;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logging()?;
    mcp_server::run_stdio_server().await
}

fn init_logging() -> anyhow::Result<()> {
    let log_dir = mcp_server::trace::loom_log_dir();
    std::fs::create_dir_all(&log_dir)?;
    let log_file = log_dir.join("loom-mcp.log");

    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)?;

    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("loom=info,knowledge=info"),
    )
    .filter_module("ureq", log::LevelFilter::Warn)
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
