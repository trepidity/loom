use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use crate::error::CoreError;

/// How to obtain credentials for a connection.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CredentialMethod {
    #[default]
    Prompt,
    Command,
    Keychain,
}

/// Resolve a password using the configured credential method.
pub struct CredentialProvider;

impl CredentialProvider {
    /// Get password from a shell command (stdout, trimmed).
    pub fn from_command(command: &str) -> Result<String, CoreError> {
        debug!("Running password command");
        let output = std::process::Command::new("sh")
            .arg("-c")
            .arg(command)
            .output()
            .map_err(|e| CoreError::CredentialError(format!("Failed to run command: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(CoreError::CredentialError(format!(
                "Password command failed ({}): {}",
                output.status, stderr
            )));
        }

        let password = String::from_utf8(output.stdout)
            .map_err(|e| CoreError::CredentialError(format!("Invalid UTF-8 in password: {}", e)))?
            .trim_end_matches('\n')
            .trim_end_matches('\r')
            .to_string();

        Ok(password)
    }

    /// Get password from the OS keychain.
    pub fn from_keychain(connection_name: &str) -> Result<String, CoreError> {
        let entry = keyring::Entry::new("loom", connection_name)
            .map_err(|e| CoreError::CredentialError(format!("Keychain access failed: {}", e)))?;

        entry
            .get_password()
            .map_err(|e| CoreError::CredentialError(format!("Keychain get failed: {}", e)))
    }

    /// Store a password in the OS keychain.
    pub fn store_in_keychain(connection_name: &str, password: &str) -> Result<(), CoreError> {
        let entry = keyring::Entry::new("loom", connection_name)
            .map_err(|e| CoreError::CredentialError(format!("Keychain access failed: {}", e)))?;

        entry
            .set_password(password)
            .map_err(|e| CoreError::CredentialError(format!("Keychain store failed: {}", e)))
    }

    /// Delete a password from the OS keychain.
    pub fn delete_from_keychain(connection_name: &str) -> Result<(), CoreError> {
        let entry = keyring::Entry::new("loom", connection_name)
            .map_err(|e| CoreError::CredentialError(format!("Keychain access failed: {}", e)))?;

        match entry.delete_credential() {
            Ok(()) => Ok(()),
            Err(e) => {
                warn!("Keychain delete warning: {}", e);
                Ok(())
            }
        }
    }
}
