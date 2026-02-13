use std::collections::HashSet;

use ldap3::controls::RelaxRules;
use ldap3::Mod;
use tracing::{debug, info};

use crate::connection::LdapConnection;
use crate::error::CoreError;

impl LdapConnection {
    /// Modify an entry's attributes.
    /// When `relax_rules` is enabled in connection settings, sends the
    /// Relax Rules control to bypass server-side schema violations from
    /// operational attributes injected by directory plugins/overlays.
    pub async fn modify_entry(
        &mut self,
        dn: &str,
        mods: Vec<Mod<String>>,
    ) -> Result<(), CoreError> {
        debug!(
            "modify_entry dn={} relax_rules={}",
            dn, self.settings.relax_rules
        );
        for m in &mods {
            match m {
                Mod::Add(attr, vals) => debug!("  mod ADD attr={} vals={:?}", attr, vals),
                Mod::Delete(attr, vals) => debug!("  mod DELETE attr={} vals={:?}", attr, vals),
                Mod::Replace(attr, vals) => debug!("  mod REPLACE attr={} vals={:?}", attr, vals),
                Mod::Increment(attr, vals) => {
                    debug!("  mod INCREMENT attr={} vals={:?}", attr, vals)
                }
            }
        }

        let result = if self.settings.relax_rules {
            self.ldap
                .with_controls(vec![RelaxRules.into()])
                .modify(dn, mods)
                .await
                .map_err(CoreError::Ldap)?
        } else {
            self.ldap.modify(dn, mods).await.map_err(CoreError::Ldap)?
        };

        debug!("modify_entry result rc={} text={}", result.rc, result.text);

        if result.rc != 0 {
            return Err(CoreError::ModifyFailed(format!(
                "Modify {} failed rc={}: {}",
                dn, result.rc, result.text
            )));
        }

        info!("Modified entry: {}", dn);
        Ok(())
    }

    /// Replace a single attribute value.
    pub async fn replace_attribute_value(
        &mut self,
        dn: &str,
        attr: &str,
        _old_value: &str,
        new_value: &str,
    ) -> Result<(), CoreError> {
        debug!(
            "replace_attribute_value dn={} attr={} new_value={}",
            dn, attr, new_value
        );
        let mods = vec![Mod::Replace(
            attr.to_string(),
            HashSet::from([new_value.to_string()]),
        )];
        self.modify_entry(dn, mods).await
    }

    /// Add a value to an attribute.
    pub async fn add_attribute_value(
        &mut self,
        dn: &str,
        attr: &str,
        value: &str,
    ) -> Result<(), CoreError> {
        debug!(
            "add_attribute_value dn={} attr={} value={}",
            dn, attr, value
        );
        let mods = vec![Mod::Add(
            attr.to_string(),
            HashSet::from([value.to_string()]),
        )];
        self.modify_entry(dn, mods).await
    }

    /// Delete a specific value from an attribute.
    pub async fn delete_attribute_value(
        &mut self,
        dn: &str,
        attr: &str,
        value: &str,
    ) -> Result<(), CoreError> {
        debug!(
            "delete_attribute_value dn={} attr={} value={}",
            dn, attr, value
        );
        let mods = vec![Mod::Delete(
            attr.to_string(),
            HashSet::from([value.to_string()]),
        )];
        self.modify_entry(dn, mods).await
    }

    /// Add a new entry with the given DN and attributes.
    pub async fn add_entry(
        &mut self,
        dn: &str,
        attrs: Vec<(String, HashSet<String>)>,
    ) -> Result<(), CoreError> {
        debug!(
            "add_entry dn={} relax_rules={}",
            dn, self.settings.relax_rules
        );
        for (attr, vals) in &attrs {
            debug!("  attr={} vals={:?}", attr, vals);
        }

        let result = if self.settings.relax_rules {
            self.ldap
                .with_controls(vec![RelaxRules.into()])
                .add(dn, attrs)
                .await
                .map_err(CoreError::Ldap)?
        } else {
            self.ldap.add(dn, attrs).await.map_err(CoreError::Ldap)?
        };

        debug!("add_entry result rc={} text={}", result.rc, result.text);

        if result.rc != 0 {
            return Err(CoreError::ModifyFailed(format!(
                "Add {} failed rc={}: {}",
                dn, result.rc, result.text
            )));
        }

        info!("Added entry: {}", dn);
        Ok(())
    }

    /// Delete an entry by DN.
    pub async fn delete_entry(&mut self, dn: &str) -> Result<(), CoreError> {
        debug!(
            "delete_entry dn={} relax_rules={}",
            dn, self.settings.relax_rules
        );

        let result = if self.settings.relax_rules {
            self.ldap
                .with_controls(vec![RelaxRules.into()])
                .delete(dn)
                .await
                .map_err(CoreError::Ldap)?
        } else {
            self.ldap.delete(dn).await.map_err(CoreError::Ldap)?
        };

        debug!("delete_entry result rc={} text={}", result.rc, result.text);

        if result.rc != 0 {
            return Err(CoreError::ModifyFailed(format!(
                "Delete {} failed rc={}: {}",
                dn, result.rc, result.text
            )));
        }

        info!("Deleted entry: {}", dn);
        Ok(())
    }
}
