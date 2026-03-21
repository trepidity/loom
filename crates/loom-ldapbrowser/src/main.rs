use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

use loom_core::vault::Vault;
use loom_tui::app::App;
use loom_tui::config::AppConfig;

#[derive(Parser, Debug)]
#[command(
    name = "loom-ldapbrowser",
    version,
    about = "A terminal-based LDAP browser"
)]
struct Cli {
    /// Path to config file (default: ~/.config/loom-ldapbrowser/config.toml)
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

    /// Vault master password (avoids interactive prompt)
    #[arg(long)]
    vault_password: Option<String>,

    /// Path to vault file (default: ~/.config/loom-ldapbrowser/vault.dat)
    #[arg(long)]
    vault: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    let cli = Cli::parse();

    // Initialize logging to ./logs/ directory at debug level
    let log_dir = std::path::PathBuf::from("./logs");
    std::fs::create_dir_all(&log_dir)?;
    let log_file = std::fs::File::create(log_dir.join("loom-ldapbrowser.log"))?;

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive("loom_ldapbrowser=debug".parse()?),
        )
        .with_writer(log_file)
        .with_ansi(false)
        .init();

    info!("loom-ldapbrowser starting");

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
            read_only: false,
            offline: false,
            labels: vec![],
        };
        config.connections.insert(0, profile);
    }

    // Resolve vault
    let vault = load_vault(&cli.vault_password, &cli.vault, &config);

    // Create and run the app
    let mut app = App::new(config, vault);
    app.connect_first_profile().await;
    app.run().await?;

    info!("loom-ldapbrowser exiting");
    Ok(())
}

/// Attempt to open the vault if vault_enabled is set in config.
/// Returns None if vault is not enabled, doesn't exist yet, or can't be opened.
fn load_vault(
    cli_password: &Option<String>,
    cli_path: &Option<String>,
    config: &AppConfig,
) -> Option<Vault> {
    if !config.general.vault_enabled {
        return None;
    }

    let vault_path = cli_path
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(Vault::default_path);

    if !Vault::exists(&vault_path) {
        return None;
    }

    // Determine vault password: CLI arg > env var > interactive prompt
    let password = if let Some(pw) = cli_password.clone() {
        pw
    } else if let Ok(pw) = std::env::var("LOOM_VAULT_PASSWORD") {
        pw
    } else {
        // Interactive prompt before TUI starts
        match rpassword::read_password_from_tty(Some("Vault master password: ")) {
            Ok(pw) => pw,
            Err(e) => {
                eprintln!("Failed to read vault password: {}", e);
                return None;
            }
        }
    };

    match Vault::open(&vault_path, &password) {
        Ok(vault) => {
            info!("Vault loaded successfully");
            Some(vault)
        }
        Err(e) => {
            warn!("Failed to open vault: {}", e);
            eprintln!(
                "Warning: Could not open vault ({}). Continuing without vault.",
                e
            );
            None
        }
    }
}
