use std::collections::HashSet;

use ldap3::Mod;
use tracing::{debug, info};

use crate::connection::LdapConnection;
use crate::error::CoreError;

/// A single bulk modification operation.
#[derive(Debug, Clone)]
pub enum BulkMod {
    /// Replace all values of an attribute with a new value.
    ReplaceAttribute { attr: String, value: String },
    /// Add a value to an attribute.
    AddValue { attr: String, value: String },
    /// Delete all values of an attribute.
    DeleteAttribute { attr: String },
    /// Delete a specific value from an attribute.
    DeleteValue { attr: String, value: String },
}

/// Result of a bulk update operation.
#[derive(Debug)]
pub struct BulkResult {
    pub total: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub errors: Vec<(String, String)>, // (dn, error_message)
}

impl LdapConnection {
    /// Execute a bulk update: search for entries matching the filter,
    /// then apply the modifications to each.
    pub async fn bulk_update(
        &mut self,
        filter: &str,
        modifications: &[BulkMod],
    ) -> Result<BulkResult, CoreError> {
        // First, find all matching entries
        let base_dn = self.base_dn.clone();
        let entries = self.search_subtree(&base_dn, filter, &["dn"]).await?;

        let total = entries.len();
        info!("Bulk update: {} entries match filter '{}'", total, filter);

        let mut succeeded = 0;
        let mut failed = 0;
        let mut errors = Vec::new();

        for entry in &entries {
            let mods = build_ldap_mods(modifications);

            match self.modify_entry(&entry.dn, mods).await {
                Ok(()) => {
                    succeeded += 1;
                    debug!("Bulk modified: {}", entry.dn);
                }
                Err(e) => {
                    failed += 1;
                    errors.push((entry.dn.clone(), e.to_string()));
                    debug!("Bulk modify failed for {}: {}", entry.dn, e);
                }
            }
        }

        info!(
            "Bulk update complete: {} succeeded, {} failed out of {}",
            succeeded, failed, total
        );

        Ok(BulkResult {
            total,
            succeeded,
            failed,
            errors,
        })
    }
}

/// Convert BulkMod operations to ldap3 Mod operations.
fn build_ldap_mods(modifications: &[BulkMod]) -> Vec<Mod<String>> {
    let mut mods = Vec::new();

    for m in modifications {
        match m {
            BulkMod::ReplaceAttribute { attr, value } => {
                mods.push(Mod::Replace(attr.clone(), HashSet::from([value.clone()])));
            }
            BulkMod::AddValue { attr, value } => {
                mods.push(Mod::Add(attr.clone(), HashSet::from([value.clone()])));
            }
            BulkMod::DeleteAttribute { attr } => {
                mods.push(Mod::Delete(attr.clone(), HashSet::new()));
            }
            BulkMod::DeleteValue { attr, value } => {
                mods.push(Mod::Delete(attr.clone(), HashSet::from([value.clone()])));
            }
        }
    }

    mods
}
