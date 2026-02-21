use std::collections::BTreeMap;

use ldap3::{Scope, SearchEntry};
use serde::{Deserialize, Serialize};
use strum::Display;
use tracing::debug;

use crate::connection::LdapConnection;
use crate::error::CoreError;
use crate::util::{get_first, get_values, has_attr};

/// Known LDAP server types.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Display)]
pub enum ServerType {
    #[strum(serialize = "Active Directory")]
    ActiveDirectory,
    #[strum(serialize = "OpenLDAP")]
    OpenLdap,
    #[strum(serialize = "eDirectory")]
    EDirectory,
    #[strum(serialize = "OpenDS/OpenDJ")]
    OpenDs,
    #[strum(serialize = "Radiant Logic")]
    RadiantLogic,
    #[strum(serialize = "389 Directory")]
    Directory389,
    #[strum(serialize = "Unknown")]
    Unknown(String),
}

/// Information gathered from the Root DSE.
#[derive(Debug, Clone)]
pub struct RootDse {
    pub naming_contexts: Vec<String>,
    pub subschema_subentry: Option<String>,
    pub vendor_name: Option<String>,
    pub vendor_version: Option<String>,
    pub supported_controls: Vec<String>,
    pub supported_extensions: Vec<String>,
    pub server_type: ServerType,
    pub raw: BTreeMap<String, Vec<String>>,
}

impl LdapConnection {
    /// Read the Root DSE and detect server type.
    pub async fn read_root_dse(&mut self) -> Result<RootDse, CoreError> {
        let result = self
            .ldap
            .search(
                "",
                Scope::Base,
                "(objectClass=*)",
                vec![
                    "*",
                    "+",
                    "namingContexts",
                    "subschemaSubentry",
                    "vendorName",
                    "vendorVersion",
                    "supportedControl",
                    "supportedExtension",
                    "supportedLDAPVersion",
                    "forestFunctionality",
                    "domainFunctionality",
                    "domainControllerFunctionality",
                    "isGlobalCatalogReady",
                    "schemaNamingContext",
                    "configurationNamingContext",
                    "rootDomainNamingContext",
                    "objectClass",
                ],
            )
            .await
            .map_err(CoreError::Ldap)?;

        let (entries, _res) = result
            .success()
            .map_err(|e| CoreError::SearchFailed(format!("RootDSE: {}", e)))?;

        let entry = entries
            .into_iter()
            .next()
            .map(SearchEntry::construct)
            .ok_or_else(|| CoreError::SearchFailed("No RootDSE entry returned".to_string()))?;

        let attrs: BTreeMap<String, Vec<String>> = entry.attrs.into_iter().collect();

        let naming_contexts = get_values(&attrs, "namingcontexts");
        let subschema_subentry = get_first(&attrs, "subschemasubentry");
        let vendor_name = get_first(&attrs, "vendorname");
        let vendor_version = get_first(&attrs, "vendorversion");
        let supported_controls = get_values(&attrs, "supportedcontrol");
        let supported_extensions = get_values(&attrs, "supportedextension");

        let server_type = detect_server_type(&attrs, &vendor_name, &supported_controls);
        debug!("Detected server type: {}", server_type);

        // Auto-discover base DN if not set
        if self.base_dn.is_empty() {
            if let Some(first_nc) = naming_contexts.first() {
                debug!("Auto-discovered base DN: {}", first_nc);
                self.base_dn = first_nc.clone();
            }
        }

        Ok(RootDse {
            naming_contexts,
            subschema_subentry,
            vendor_name,
            vendor_version,
            supported_controls,
            supported_extensions,
            server_type,
            raw: attrs,
        })
    }
}

/// Detect server type from RootDSE attributes.
fn detect_server_type(
    attrs: &BTreeMap<String, Vec<String>>,
    vendor_name: &Option<String>,
    supported_controls: &[String],
) -> ServerType {
    // Active Directory: has forestFunctionality or domainFunctionality
    if has_attr(attrs, "forestfunctionality")
        || has_attr(attrs, "domainfunctionality")
        || has_attr(attrs, "domaincontrollerfunctionality")
        || has_attr(attrs, "isglobalcatalogready")
    {
        return ServerType::ActiveDirectory;
    }

    // Check vendor name
    if let Some(ref vn) = vendor_name {
        let vn_lower = vn.to_lowercase();
        if vn_lower.contains("openldap") {
            return ServerType::OpenLdap;
        }
        if vn_lower.contains("novell")
            || vn_lower.contains("netiq")
            || vn_lower.contains("edirectory")
        {
            return ServerType::EDirectory;
        }
        if vn_lower.contains("sun")
            || vn_lower.contains("oracle")
            || vn_lower.contains("opends")
            || vn_lower.contains("opendj")
            || vn_lower.contains("forgerock")
        {
            return ServerType::OpenDs;
        }
        if vn_lower.contains("radiant") {
            return ServerType::RadiantLogic;
        }
        if vn_lower.contains("389") || vn_lower.contains("red hat") || vn_lower.contains("fedora") {
            return ServerType::Directory389;
        }
    }

    // Check objectClass for OpenLDAP rootDSE
    let object_classes = get_values(attrs, "objectclass");
    for oc in &object_classes {
        let oc_lower = oc.to_lowercase();
        if oc_lower.contains("openldaprootdse") {
            return ServerType::OpenLdap;
        }
    }

    // Check supported controls for vendor-specific OIDs
    for ctrl in supported_controls {
        // Microsoft-specific OIDs
        if ctrl.starts_with("1.2.840.113556.1.4.") {
            return ServerType::ActiveDirectory;
        }
    }

    // Check for OpenDS/OpenDJ specific attributes
    if has_attr(attrs, "ds-private-naming-contexts") {
        return ServerType::OpenDs;
    }

    ServerType::Unknown("Unknown LDAP server".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_ad() {
        let mut attrs = BTreeMap::new();
        attrs.insert("forestFunctionality".to_string(), vec!["7".to_string()]);
        let server = detect_server_type(&attrs, &None, &[]);
        assert_eq!(server, ServerType::ActiveDirectory);
    }

    #[test]
    fn test_detect_openldap_by_vendor() {
        let attrs = BTreeMap::new();
        let vendor = Some("OpenLDAP".to_string());
        let server = detect_server_type(&attrs, &vendor, &[]);
        assert_eq!(server, ServerType::OpenLdap);
    }

    #[test]
    fn test_detect_openldap_by_objectclass() {
        let mut attrs = BTreeMap::new();
        attrs.insert(
            "objectClass".to_string(),
            vec!["top".to_string(), "OpenLDAProotDSE".to_string()],
        );
        let server = detect_server_type(&attrs, &None, &[]);
        assert_eq!(server, ServerType::OpenLdap);
    }

    #[test]
    fn test_detect_unknown() {
        let attrs = BTreeMap::new();
        let server = detect_server_type(&attrs, &None, &[]);
        assert!(matches!(server, ServerType::Unknown(_)));
    }
}
