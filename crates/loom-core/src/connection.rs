use std::time::Duration;

use ldap3::{Ldap, LdapConnAsync, LdapConnSettings};
use tracing::{info, warn};

use crate::error::CoreError;

/// TLS mode for LDAP connections.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TlsMode {
    #[default]
    Auto,
    Ldaps,
    StartTls,
    None,
}

impl TlsMode {
    /// Cycle to the next TLS mode (for F2 toggling in UI).
    pub fn next(&self) -> Self {
        match self {
            TlsMode::Auto => TlsMode::Ldaps,
            TlsMode::Ldaps => TlsMode::StartTls,
            TlsMode::StartTls => TlsMode::None,
            TlsMode::None => TlsMode::Auto,
        }
    }

    /// Human-readable label for display.
    pub fn label(&self) -> &'static str {
        match self {
            TlsMode::Auto => "Auto",
            TlsMode::Ldaps => "LDAPS",
            TlsMode::StartTls => "StartTLS",
            TlsMode::None => "None",
        }
    }
}

/// Settings for an LDAP connection.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConnectionSettings {
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default)]
    pub tls_mode: TlsMode,
    pub bind_dn: Option<String>,
    pub base_dn: Option<String>,
    #[serde(default = "default_page_size")]
    pub page_size: u32,
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    /// Send the Relax Rules control with modify/delete operations
    /// to bypass server-side schema violations from directory plugins.
    #[serde(default)]
    pub relax_rules: bool,
}

fn default_port() -> u16 {
    389
}
fn default_page_size() -> u32 {
    500
}
fn default_timeout() -> u64 {
    30
}

/// An active LDAP connection with reconnect support.
pub struct LdapConnection {
    pub ldap: Ldap,
    pub settings: ConnectionSettings,
    pub base_dn: String,
    /// Credentials stored for reconnection.
    bind_credentials: Option<(String, String)>, // (bind_dn, password)
}

impl LdapConnection {
    /// Connect to an LDAP server using the given settings.
    pub async fn connect(settings: ConnectionSettings) -> Result<Self, CoreError> {
        let timeout = Duration::from_secs(settings.timeout_secs);

        let ldap = match settings.tls_mode {
            TlsMode::Auto => Self::auto_connect(&settings, timeout).await?,
            TlsMode::Ldaps => Self::connect_ldaps(&settings, timeout).await?,
            TlsMode::StartTls => Self::connect_starttls(&settings, timeout).await?,
            TlsMode::None => Self::connect_plain(&settings, timeout).await?,
        };

        let base_dn = settings.base_dn.clone().unwrap_or_default();

        Ok(Self {
            ldap,
            settings,
            base_dn,
            bind_credentials: None,
        })
    }

    async fn auto_connect(
        settings: &ConnectionSettings,
        timeout: Duration,
    ) -> Result<Ldap, CoreError> {
        // Try LDAPS first (port 636 or user-specified)
        let ldaps_port = if settings.port == 389 {
            636
        } else {
            settings.port
        };
        let ldaps_settings = ConnectionSettings {
            port: ldaps_port,
            ..settings.clone()
        };

        if let Ok(ldap) = Self::connect_ldaps(&ldaps_settings, timeout).await {
            info!("Connected via LDAPS on port {}", ldaps_port);
            return Ok(ldap);
        }
        warn!("LDAPS failed, trying StartTLS");

        // Try StartTLS on port 389
        if let Ok(ldap) = Self::connect_starttls(settings, timeout).await {
            info!("Connected via StartTLS on port {}", settings.port);
            return Ok(ldap);
        }
        warn!("StartTLS failed, trying plain LDAP");

        // Fall back to plain
        let ldap = Self::connect_plain(settings, timeout).await?;
        info!("Connected via plain LDAP on port {}", settings.port);
        Ok(ldap)
    }

    async fn connect_ldaps(
        settings: &ConnectionSettings,
        timeout: Duration,
    ) -> Result<Ldap, CoreError> {
        let url = format!("ldaps://{}:{}", settings.host, settings.port);
        let conn_settings = LdapConnSettings::new().set_conn_timeout(timeout);
        let (conn, ldap) = LdapConnAsync::with_settings(conn_settings, &url)
            .await
            .map_err(|e| CoreError::ConnectionFailed(format!("LDAPS: {e}")))?;
        ldap3::drive!(conn);
        Ok(ldap)
    }

    async fn connect_starttls(
        settings: &ConnectionSettings,
        timeout: Duration,
    ) -> Result<Ldap, CoreError> {
        let url = format!("ldap://{}:{}", settings.host, settings.port);
        let conn_settings = LdapConnSettings::new()
            .set_conn_timeout(timeout)
            .set_starttls(true);
        let (conn, ldap) = LdapConnAsync::with_settings(conn_settings, &url)
            .await
            .map_err(|e| CoreError::ConnectionFailed(format!("StartTLS: {e}")))?;
        ldap3::drive!(conn);
        Ok(ldap)
    }

    async fn connect_plain(
        settings: &ConnectionSettings,
        timeout: Duration,
    ) -> Result<Ldap, CoreError> {
        let url = format!("ldap://{}:{}", settings.host, settings.port);
        let conn_settings = LdapConnSettings::new().set_conn_timeout(timeout);
        let (conn, ldap) = LdapConnAsync::with_settings(conn_settings, &url)
            .await
            .map_err(|e| CoreError::ConnectionFailed(format!("plain LDAP: {e}")))?;
        ldap3::drive!(conn);
        Ok(ldap)
    }

    /// Store bind credentials for reconnection.
    pub fn store_credentials(&mut self, bind_dn: String, password: String) {
        self.bind_credentials = Some((bind_dn, password));
    }

    /// Attempt to reconnect using stored settings and credentials.
    /// Returns Ok(()) if reconnection and re-bind succeed.
    pub async fn reconnect(&mut self) -> Result<(), CoreError> {
        info!(
            "Attempting reconnect to {}:{}",
            self.settings.host, self.settings.port
        );

        let timeout = Duration::from_secs(self.settings.timeout_secs);

        let ldap = match self.settings.tls_mode {
            TlsMode::Auto => Self::auto_connect(&self.settings, timeout).await?,
            TlsMode::Ldaps => Self::connect_ldaps(&self.settings, timeout).await?,
            TlsMode::StartTls => Self::connect_starttls(&self.settings, timeout).await?,
            TlsMode::None => Self::connect_plain(&self.settings, timeout).await?,
        };

        self.ldap = ldap;

        // Re-bind with stored credentials
        if let Some((ref bind_dn, ref password)) = self.bind_credentials {
            let bind_dn = bind_dn.clone();
            let password = password.clone();
            self.simple_bind(&bind_dn, &password).await?;
        } else {
            self.anonymous_bind().await?;
        }

        info!("Reconnected successfully");
        Ok(())
    }

    /// Check if an error indicates a lost connection that may be recoverable.
    pub fn is_connection_error(err: &CoreError) -> bool {
        match err {
            CoreError::Ldap(ldap_err) => {
                let msg = ldap_err.to_string().to_lowercase();
                msg.contains("connection")
                    || msg.contains("broken pipe")
                    || msg.contains("reset")
                    || msg.contains("closed")
                    || msg.contains("eof")
                    || msg.contains("timed out")
            }
            CoreError::Timeout => true,
            CoreError::ConnectionFailed(_) => true,
            _ => false,
        }
    }

    /// Unbind and close the connection.
    pub async fn disconnect(&mut self) -> Result<(), CoreError> {
        self.ldap.unbind().await.map_err(CoreError::Ldap)
    }
}
