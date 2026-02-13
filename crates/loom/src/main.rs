use anyhow::Result;
use clap::Parser;
use tracing::info;
use tracing_subscriber::EnvFilter;

use loom_tui::app::App;
use loom_tui::config::AppConfig;

#[derive(Parser, Debug)]
#[command(name = "loom", version, about = "A terminal-based LDAP browser")]
struct Cli {
    /// Path to config file (default: ~/.config/loom/config.toml)
    #[arg(short, long)]
    config: Option<String>,

    /// LDAP host to connect to (overrides config)
    #[arg(short = 'H', long)]
    host: Option<String>,

    /// LDAP port (overrides config)
    #[arg(short, long)]
    port: Option<u16>,

    /// Bind DN (overrides config)
    #[arg(short = 'D', long)]
    bind_dn: Option<String>,

    /// Base DN (overrides config)
    #[arg(short, long)]
    base_dn: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging to ./logs/ directory at debug level
    let log_dir = std::path::PathBuf::from("./logs");
    std::fs::create_dir_all(&log_dir)?;
    let log_file = std::fs::File::create(log_dir.join("loom.log"))?;

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("loom=debug".parse()?))
        .with_writer(log_file)
        .with_ansi(false)
        .init();

    info!("Loom starting");

    // Load config
    let mut config = AppConfig::load();

    // Apply CLI overrides
    if let Some(host) = cli.host {
        // Create/override first connection from CLI args
        let profile = loom_tui::config::ConnectionProfile {
            name: host.clone(),
            host,
            port: cli.port.unwrap_or(389),
            tls_mode: loom_core::connection::TlsMode::Auto,
            bind_dn: cli.bind_dn,
            base_dn: cli.base_dn,
            credential_method: loom_core::credentials::CredentialMethod::Prompt,
            password_command: None,
            page_size: 500,
            timeout_secs: 30,
            relax_rules: false,
            folder: None,
        };
        config.connections.insert(0, profile);
    }

    // Create and run the app
    let mut app = App::new(config);
    app.connect_first_profile().await;
    app.run().await?;

    info!("Loom exiting");
    Ok(())
}
