#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("loom=info,knowledge=info"),
    )
    .format_timestamp_millis()
    .target(env_logger::Target::Stderr)
    .init();
    mcp_server::run_stdio_server().await
}
