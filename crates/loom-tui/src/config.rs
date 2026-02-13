use serde::{Deserialize, Serialize};

use loom_core::connection::{ConnectionSettings, TlsMode};
use loom_core::credentials::CredentialMethod;

/// A saved connection profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionProfile {
    pub name: String,
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default)]
    pub tls_mode: TlsMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bind_dn: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_dn: Option<String>,
    #[serde(default)]
    pub credential_method: CredentialMethod,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password_command: Option<String>,
    #[serde(default = "default_page_size")]
    pub page_size: u32,
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    #[serde(default)]
    pub relax_rules: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub folder: Option<String>,
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

impl ConnectionProfile {
    /// Convert to ConnectionSettings for connecting.
    pub fn to_connection_settings(&self) -> ConnectionSettings {
        ConnectionSettings {
            host: self.host.clone(),
            port: self.port,
            tls_mode: self.tls_mode.clone(),
            bind_dn: self.bind_dn.clone(),
            base_dn: self.base_dn.clone(),
            page_size: self.page_size,
            timeout_secs: self.timeout_secs,
            relax_rules: self.relax_rules,
        }
    }
}

/// Configurable keybindings for global shortcuts.
/// Each field holds a key string like "Alt+t", "Ctrl+c", "q", "F2", etc.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct KeybindingConfig {
    pub quit: String,
    pub force_quit: String,
    pub focus_next: String,
    pub focus_prev: String,
    pub show_connect_dialog: String,
    pub next_tab: String,
    pub prev_tab: String,
    pub search: String,
    pub show_export_dialog: String,
    pub show_bulk_update: String,
    pub show_schema_viewer: String,
    pub toggle_log_panel: String,
    pub save_connection: String,
    pub toggle_layout: String,
}

impl Default for KeybindingConfig {
    fn default() -> Self {
        Self {
            quit: "q".to_string(),
            force_quit: "Ctrl+c".to_string(),
            focus_next: "Tab".to_string(),
            focus_prev: "Shift+Tab".to_string(),
            show_connect_dialog: "Ctrl+t".to_string(),
            next_tab: "Ctrl+n".to_string(),
            prev_tab: "Ctrl+p".to_string(),
            search: "/".to_string(),
            show_export_dialog: "Ctrl+e".to_string(),
            show_bulk_update: "Ctrl+b".to_string(),
            show_schema_viewer: "Ctrl+s".to_string(),
            toggle_log_panel: "Ctrl+l".to_string(),
            save_connection: "Ctrl+w".to_string(),
            toggle_layout: "Ctrl+g".to_string(),
        }
    }
}

/// Top-level application configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub keybindings: KeybindingConfig,
    #[serde(default)]
    pub connections: Vec<ConnectionProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_tick_rate")]
    pub tick_rate_ms: u64,
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

fn default_theme() -> String {
    "dark".to_string()
}
fn default_tick_rate() -> u64 {
    250
}
fn default_log_level() -> String {
    "info".to_string()
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            theme: default_theme(),
            tick_rate_ms: default_tick_rate(),
            log_level: default_log_level(),
        }
    }
}

impl AppConfig {
    /// Load config from ~/.config/loom/config.toml, with fallback to defaults.
    pub fn load() -> Self {
        let config_path = dirs::config_dir()
            .map(|d| d.join("loom").join("config.toml"))
            .unwrap_or_default();

        if config_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&config_path) {
                if let Ok(config) = toml::from_str::<AppConfig>(&content) {
                    return config;
                }
            }
        }

        Self::default()
    }

    /// Parse config from a TOML string.
    pub fn from_toml(content: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(content)
    }

    /// Save the entire config to disk, overwriting the existing file.
    pub fn save(&self) -> Result<(), String> {
        let config_dir = dirs::config_dir()
            .map(|d| d.join("loom"))
            .ok_or_else(|| "Cannot determine config directory".to_string())?;

        std::fs::create_dir_all(&config_dir)
            .map_err(|e| format!("Failed to create config dir: {}", e))?;

        let content = toml::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;

        std::fs::write(config_dir.join("config.toml"), content)
            .map_err(|e| format!("Failed to write config: {}", e))?;

        Ok(())
    }

    /// Update a connection profile at the given index.
    pub fn update_connection(&mut self, index: usize, profile: ConnectionProfile) {
        if index < self.connections.len() {
            self.connections[index] = profile;
        }
    }

    /// Delete a connection profile at the given index.
    pub fn delete_connection(&mut self, index: usize) {
        if index < self.connections.len() {
            self.connections.remove(index);
        }
    }

    /// Append a connection profile to the config file on disk.
    /// Creates the config directory and file if they don't exist.
    /// The password is never written — only the profile metadata.
    pub fn append_connection(profile: &ConnectionProfile) -> Result<(), String> {
        let config_dir = dirs::config_dir()
            .map(|d| d.join("loom"))
            .ok_or_else(|| "Cannot determine config directory".to_string())?;

        std::fs::create_dir_all(&config_dir)
            .map_err(|e| format!("Failed to create config dir: {}", e))?;

        let config_path = config_dir.join("config.toml");

        // Read existing content (or start empty)
        let mut content = if config_path.exists() {
            std::fs::read_to_string(&config_path)
                .map_err(|e| format!("Failed to read config: {}", e))?
        } else {
            String::new()
        };

        // Serialize just the profile as a [[connections]] block
        let block =
            toml::to_string(profile).map_err(|e| format!("Failed to serialize profile: {}", e))?;

        if !content.is_empty() && !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str("\n[[connections]]\n");
        content.push_str(&block);

        std::fs::write(&config_path, content)
            .map_err(|e| format!("Failed to write config: {}", e))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.general.theme, "dark");
        assert_eq!(config.general.tick_rate_ms, 250);
        assert!(config.connections.is_empty());
    }

    #[test]
    fn test_parse_minimal_config() {
        let toml = r#"
[general]
theme = "nord"
"#;
        let config = AppConfig::from_toml(toml).unwrap();
        assert_eq!(config.general.theme, "nord");
        assert!(config.connections.is_empty());
    }

    #[test]
    fn test_parse_full_config() {
        let toml = r#"
[general]
theme = "solarized"
tick_rate_ms = 100
log_level = "debug"

[[connections]]
name = "Production"
host = "ldap.example.com"
port = 636
tls_mode = "ldaps"
bind_dn = "cn=admin,dc=example,dc=com"
base_dn = "dc=example,dc=com"
credential_method = "prompt"
page_size = 1000
timeout_secs = 60
"#;
        let config = AppConfig::from_toml(toml).unwrap();
        assert_eq!(config.general.theme, "solarized");
        assert_eq!(config.general.tick_rate_ms, 100);
        assert_eq!(config.connections.len(), 1);

        let conn = &config.connections[0];
        assert_eq!(conn.name, "Production");
        assert_eq!(conn.host, "ldap.example.com");
        assert_eq!(conn.port, 636);
        assert_eq!(conn.bind_dn, Some("cn=admin,dc=example,dc=com".to_string()));
        assert_eq!(conn.page_size, 1000);
    }

    #[test]
    fn test_connection_profile_to_settings() {
        let profile = ConnectionProfile {
            name: "Test".to_string(),
            host: "localhost".to_string(),
            port: 389,
            tls_mode: TlsMode::None,
            bind_dn: Some("cn=admin".to_string()),
            base_dn: Some("dc=test".to_string()),
            credential_method: CredentialMethod::Prompt,
            password_command: None,
            page_size: 500,
            timeout_secs: 30,
            relax_rules: false,
            folder: None,
        };

        let settings = profile.to_connection_settings();
        assert_eq!(settings.host, "localhost");
        assert_eq!(settings.port, 389);
        assert_eq!(settings.bind_dn, Some("cn=admin".to_string()));
        assert_eq!(settings.base_dn, Some("dc=test".to_string()));
    }

    #[test]
    fn test_keybindings_config_defaults() {
        let config = AppConfig::default();
        assert_eq!(config.keybindings.quit, "q");
        assert_eq!(config.keybindings.force_quit, "Ctrl+c");
        assert_eq!(config.keybindings.show_connect_dialog, "Ctrl+t");
        assert_eq!(config.keybindings.toggle_layout, "Ctrl+g");
    }

    #[test]
    fn test_parse_keybindings_section() {
        let toml = r#"
[keybindings]
quit = "Alt+q"
show_connect_dialog = "Alt+t"
toggle_layout = "F1"
"#;
        let config = AppConfig::from_toml(toml).unwrap();
        assert_eq!(config.keybindings.quit, "Alt+q");
        assert_eq!(config.keybindings.show_connect_dialog, "Alt+t");
        assert_eq!(config.keybindings.toggle_layout, "F1");
        // Non-specified fields keep defaults
        assert_eq!(config.keybindings.force_quit, "Ctrl+c");
        assert_eq!(config.keybindings.search, "/");
    }

    #[test]
    fn test_parse_defaults() {
        let toml = r#"
[[connections]]
name = "Minimal"
host = "localhost"
"#;
        let config = AppConfig::from_toml(toml).unwrap();
        let conn = &config.connections[0];
        assert_eq!(conn.port, 389); // default
        assert_eq!(conn.page_size, 500); // default
        assert_eq!(conn.timeout_secs, 30); // default
    }
}
