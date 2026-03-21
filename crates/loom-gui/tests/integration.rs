//! Integration tests for the loom-gui crate.
//!
//! Most GUI logic requires a display server (X11/Wayland/macOS) to instantiate
//! Slint windows, so these tests focus on verifying the crate compiles and that
//! the underlying loom-core logic (re-exported via dependencies) works correctly.

use loom_core::config::{AppConfig, ConnectionProfile};
use loom_core::connection::TlsMode;
use loom_core::credentials::CredentialMethod;

/// Verify that the loom-gui crate compiles and its public API is accessible.
#[test]
fn test_loom_gui_compiles() {
    // The public entry point exists (we can't call it without a display server).
    let _run_fn: fn() -> Result<(), slint::PlatformError> = loom_gui::run;
}

/// AppConfig::default() returns sensible defaults with no connections.
#[test]
fn test_config_defaults() {
    let config = AppConfig::default();
    assert!(config.connections.is_empty());
    assert!(!config.general.theme.is_empty());
}

/// Verify ConnectionProfile can be constructed and its fields are accessible,
/// matching the types that the GUI crate relies on for populating the UI.
#[test]
fn test_connection_profile_construction() {
    let profile = ConnectionProfile {
        name: "Test Server".to_string(),
        host: "ldap.example.com".to_string(),
        port: 636,
        tls_mode: TlsMode::Ldaps,
        bind_dn: Some("cn=admin,dc=example,dc=com".to_string()),
        base_dn: Some("dc=example,dc=com".to_string()),
        credential_method: CredentialMethod::Prompt,
        password_command: None,
        page_size: 1000,
        timeout_secs: 30,
        relax_rules: false,
        folder: None,
        read_only: false,
        offline: false,
    };

    assert_eq!(profile.name, "Test Server");
    assert_eq!(profile.host, "ldap.example.com");
    assert_eq!(profile.port, 636);
}

/// Verify ConnectionProfile::to_connection_settings() maps fields correctly.
#[test]
fn test_connection_profile_to_settings() {
    let profile = ConnectionProfile {
        name: "My LDAP".to_string(),
        host: "ldap.test.org".to_string(),
        port: 389,
        tls_mode: TlsMode::StartTls,
        bind_dn: Some("cn=reader,dc=test,dc=org".to_string()),
        base_dn: Some("dc=test,dc=org".to_string()),
        credential_method: CredentialMethod::Prompt,
        password_command: None,
        page_size: 500,
        timeout_secs: 10,
        relax_rules: true,
        folder: None,
        read_only: false,
        offline: false,
    };

    let settings = profile.to_connection_settings();
    assert_eq!(settings.host, "ldap.test.org");
    assert_eq!(settings.port, 389);
    assert_eq!(settings.base_dn, Some("dc=test,dc=org".to_string()));
    assert_eq!(
        settings.bind_dn,
        Some("cn=reader,dc=test,dc=org".to_string())
    );
    assert!(matches!(settings.tls_mode, TlsMode::StartTls));
    assert_eq!(settings.page_size, 500);
    assert_eq!(settings.timeout_secs, 10);
    assert!(settings.relax_rules);
}

/// Verify all five theme names are valid strings that the GUI crate recognizes.
/// Since apply_theme is private and requires a MainWindow, we just verify the
/// theme name list is consistent with what the config stores.
#[test]
fn test_theme_names_are_valid() {
    let valid_themes = ["dark", "light", "solarized", "nord", "matrix"];
    let config = AppConfig::default();

    // Default theme should be one of the valid themes
    assert!(
        valid_themes.contains(&config.general.theme.as_str()),
        "Default theme '{}' is not in the valid theme list",
        config.general.theme
    );

    // All theme names should be non-empty lowercase ASCII
    for name in &valid_themes {
        assert!(!name.is_empty());
        assert!(name.chars().all(|c| c.is_ascii_lowercase()));
    }
}
