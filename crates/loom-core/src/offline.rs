use crate::dn;
use crate::entry::LdapEntry;
use crate::error::CoreError;
use crate::import::ldif;
use crate::schema::{
    AttributeSyntax, AttributeTypeInfo, ObjectClassInfo, ObjectClassKind, SchemaCache,
};
use crate::tree::TreeNode;

const EXAMPLE_LDIF: &str = include_str!("../../../assets/example-directory.ldif");

/// An offline LDAP directory backed by an in-memory LDIF dataset.
/// Provides read-only browse, search, and schema lookups without a server.
#[derive(Debug, Clone)]
pub struct OfflineDirectory {
    entries: Vec<LdapEntry>,
    base_dn: String,
    schema: SchemaCache,
}

impl OfflineDirectory {
    /// Load the embedded example directory.
    pub fn load_embedded() -> Self {
        Self::from_ldif(EXAMPLE_LDIF).expect("embedded LDIF must parse")
    }

    /// Parse an LDIF string into an offline directory.
    pub fn from_ldif(content: &str) -> Result<Self, CoreError> {
        let entries = ldif::parse_ldif(content)?;
        let base_dn = entries.first().map(|e| e.dn.clone()).unwrap_or_default();
        let schema = build_example_schema();
        Ok(Self {
            entries,
            base_dn,
            schema,
        })
    }

    pub fn base_dn(&self) -> &str {
        &self.base_dn
    }

    pub fn schema(&self) -> &SchemaCache {
        &self.schema
    }

    /// Return immediate children of the given parent DN.
    pub fn children(&self, parent_dn: &str) -> Vec<TreeNode> {
        let parent_lower = parent_dn.to_lowercase();
        self.entries
            .iter()
            .filter(|e| {
                dn::parent_dn(&e.dn)
                    .map(|p| p.to_lowercase() == parent_lower)
                    .unwrap_or(false)
            })
            .map(|e| TreeNode::new(e.dn.clone()))
            .collect()
    }

    /// Look up an entry by exact DN (case-insensitive).
    pub fn entry(&self, dn: &str) -> Option<LdapEntry> {
        let dn_lower = dn.to_lowercase();
        self.entries
            .iter()
            .find(|e| e.dn.to_lowercase() == dn_lower)
            .cloned()
    }

    /// Search entries under base_dn matching a simple filter.
    /// Supports `(objectClass=*)` for all entries, or substring match
    /// across all attribute values for any other filter.
    pub fn search(&self, base_dn: &str, filter: &str) -> Vec<LdapEntry> {
        let base_lower = base_dn.to_lowercase();

        // Filter to entries under the base DN (or equal to it)
        let in_scope: Vec<&LdapEntry> = self
            .entries
            .iter()
            .filter(|e| {
                let dn_lower = e.dn.to_lowercase();
                dn_lower == base_lower || dn_lower.ends_with(&format!(",{}", base_lower))
            })
            .collect();

        if filter == "(objectClass=*)" || filter == "*" {
            return in_scope.into_iter().cloned().collect();
        }

        // Extract search term from simple filters like (cn=*value*) or (attr=value)
        let search_term = extract_filter_value(filter).to_lowercase();
        if search_term.is_empty() {
            return in_scope.into_iter().cloned().collect();
        }

        in_scope
            .into_iter()
            .filter(|e| {
                // Match against DN
                if e.dn.to_lowercase().contains(&search_term) {
                    return true;
                }
                // Match against any attribute value
                e.attributes
                    .values()
                    .any(|vals| vals.iter().any(|v| v.to_lowercase().contains(&search_term)))
            })
            .cloned()
            .collect()
    }

    /// Limited search returning at most `limit` results, matching against
    /// DN and common name attributes (for DN picker / fuzzy search).
    pub fn search_limited(&self, base_dn: &str, query: &str, limit: usize) -> Vec<LdapEntry> {
        let base_lower = base_dn.to_lowercase();
        let query_lower = query.to_lowercase();

        self.entries
            .iter()
            .filter(|e| {
                let dn_lower = e.dn.to_lowercase();
                (dn_lower == base_lower || dn_lower.ends_with(&format!(",{}", base_lower)))
                    && (dn_lower.contains(&query_lower)
                        || e.attributes.values().any(|vals| {
                            vals.iter().any(|v| v.to_lowercase().contains(&query_lower))
                        }))
            })
            .take(limit)
            .cloned()
            .collect()
    }
}

/// Extract the search value from a simple LDAP filter string.
/// Handles patterns like `(cn=*value*)`, `(attr=value)`, or bare strings.
fn extract_filter_value(filter: &str) -> &str {
    let s = filter.trim();
    // Strip outer parens
    let s = if s.starts_with('(') && s.ends_with(')') {
        &s[1..s.len() - 1]
    } else {
        s
    };
    // Find '=' and take the value part
    if let Some(pos) = s.find('=') {
        let val = &s[pos + 1..];
        // Strip leading/trailing wildcards
        val.trim_matches('*')
    } else {
        s
    }
}

/// Build a hardcoded AD-like schema for the example directory.
fn build_example_schema() -> SchemaCache {
    let mut cache = SchemaCache::new();

    // Attribute types
    let attr_defs: Vec<(&str, &str, AttributeSyntax, bool, bool)> = vec![
        // (oid, name, syntax, single_value, no_user_modification)
        ("2.5.4.0", "objectClass", AttributeSyntax::Oid, false, false),
        (
            "2.5.4.3",
            "cn",
            AttributeSyntax::DirectoryString,
            false,
            false,
        ),
        (
            "2.5.4.4",
            "sn",
            AttributeSyntax::DirectoryString,
            true,
            false,
        ),
        (
            "2.5.4.42",
            "givenName",
            AttributeSyntax::DirectoryString,
            true,
            false,
        ),
        (
            "2.5.4.12",
            "title",
            AttributeSyntax::DirectoryString,
            true,
            false,
        ),
        (
            "2.5.4.11",
            "ou",
            AttributeSyntax::DirectoryString,
            false,
            false,
        ),
        (
            "2.5.4.10",
            "o",
            AttributeSyntax::DirectoryString,
            false,
            false,
        ),
        (
            "0.9.2342.19200300.100.1.25",
            "dc",
            AttributeSyntax::String,
            true,
            false,
        ),
        (
            "2.5.4.7",
            "l",
            AttributeSyntax::DirectoryString,
            true,
            false,
        ),
        (
            "2.16.840.1.113730.3.1.241",
            "displayName",
            AttributeSyntax::DirectoryString,
            true,
            false,
        ),
        (
            "0.9.2342.19200300.100.1.3",
            "mail",
            AttributeSyntax::String,
            true,
            false,
        ),
        (
            "2.5.4.20",
            "telephoneNumber",
            AttributeSyntax::TelephoneNumber,
            false,
            false,
        ),
        (
            "2.5.4.13",
            "description",
            AttributeSyntax::DirectoryString,
            false,
            false,
        ),
        ("2.5.4.31", "member", AttributeSyntax::Dn, false, false),
        ("2.5.4.34", "seeAlso", AttributeSyntax::Dn, false, false),
        (
            "2.5.4.35",
            "userPassword",
            AttributeSyntax::OctetString,
            true,
            false,
        ),
        (
            "2.5.4.49",
            "distinguishedName",
            AttributeSyntax::Dn,
            true,
            true,
        ),
        (
            "0.9.2342.19200300.100.1.1",
            "uid",
            AttributeSyntax::String,
            true,
            false,
        ),
        (
            "2.5.18.1",
            "createTimestamp",
            AttributeSyntax::GeneralizedTime,
            true,
            true,
        ),
        (
            "2.5.18.2",
            "modifyTimestamp",
            AttributeSyntax::GeneralizedTime,
            true,
            true,
        ),
        (
            "2.5.4.15",
            "businessCategory",
            AttributeSyntax::DirectoryString,
            false,
            false,
        ),
        (
            "2.5.4.16",
            "postalAddress",
            AttributeSyntax::DirectoryString,
            false,
            false,
        ),
        (
            "2.5.4.17",
            "postalCode",
            AttributeSyntax::DirectoryString,
            true,
            false,
        ),
        (
            "2.5.4.6",
            "c",
            AttributeSyntax::DirectoryString,
            true,
            false,
        ),
        (
            "2.5.4.8",
            "st",
            AttributeSyntax::DirectoryString,
            true,
            false,
        ),
        (
            "2.5.4.9",
            "street",
            AttributeSyntax::DirectoryString,
            false,
            false,
        ),
    ];

    for (oid, name, syntax, single_value, no_user_modification) in attr_defs {
        let at = AttributeTypeInfo {
            oid: oid.to_string(),
            names: vec![name.to_string()],
            description: None,
            syntax,
            single_value,
            no_user_modification,
        };
        cache.attribute_types.insert(name.to_lowercase(), at);
    }

    // Object classes
    #[allow(clippy::type_complexity)]
    let oc_defs: Vec<(&str, &str, Option<&str>, ObjectClassKind, &[&str], &[&str])> = vec![
        (
            "2.5.6.0",
            "top",
            None,
            ObjectClassKind::Abstract,
            &["objectClass"],
            &[],
        ),
        (
            "0.9.2342.19200300.100.4.13",
            "domain",
            Some("top"),
            ObjectClassKind::Structural,
            &["dc"],
            &[
                "description",
                "l",
                "o",
                "seeAlso",
                "businessCategory",
                "st",
                "street",
            ],
        ),
        (
            "2.5.6.5",
            "organizationalUnit",
            Some("top"),
            ObjectClassKind::Structural,
            &["ou"],
            &[
                "description",
                "l",
                "seeAlso",
                "businessCategory",
                "postalAddress",
                "postalCode",
                "st",
                "street",
                "telephoneNumber",
            ],
        ),
        (
            "2.5.6.6",
            "person",
            Some("top"),
            ObjectClassKind::Structural,
            &["sn", "cn"],
            &["userPassword", "telephoneNumber", "seeAlso", "description"],
        ),
        (
            "2.5.6.7",
            "organizationalPerson",
            Some("person"),
            ObjectClassKind::Structural,
            &[],
            &["title", "ou", "l", "st", "postalAddress", "postalCode"],
        ),
        (
            "2.16.840.1.113730.3.2.2",
            "inetOrgPerson",
            Some("organizationalPerson"),
            ObjectClassKind::Structural,
            &[],
            &[
                "mail",
                "uid",
                "givenName",
                "displayName",
                "businessCategory",
            ],
        ),
        (
            "2.5.6.9",
            "groupOfNames",
            Some("top"),
            ObjectClassKind::Structural,
            &["cn", "member"],
            &["description", "o", "ou", "seeAlso", "businessCategory"],
        ),
        (
            "2.5.6.14",
            "device",
            Some("top"),
            ObjectClassKind::Structural,
            &["cn"],
            &["description", "l", "o", "ou", "seeAlso"],
        ),
    ];

    for (oid, name, superior, kind, must, may) in oc_defs {
        let oc = ObjectClassInfo {
            oid: oid.to_string(),
            names: vec![name.to_string()],
            description: None,
            superior: superior.map(|s| s.to_string()),
            kind,
            must: must.iter().map(|s| s.to_string()).collect(),
            may: may.iter().map(|s| s.to_string()).collect(),
        };
        cache.object_classes.insert(name.to_lowercase(), oc);
    }

    cache
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_embedded() {
        let dir = OfflineDirectory::load_embedded();
        assert_eq!(dir.base_dn(), "dc=contoso,dc=com");
        // Should have a healthy number of entries
        assert!(
            dir.entries.len() > 100,
            "expected >100 entries, got {}",
            dir.entries.len()
        );
    }

    #[test]
    fn test_children() {
        let dir = OfflineDirectory::load_embedded();
        let root_children = dir.children("dc=contoso,dc=com");
        // Should have: Administrator, Domain Controllers, Corporate, Regional, Groups, Servers, Contacts
        assert!(
            root_children.len() >= 7,
            "expected >=7 root children, got {}",
            root_children.len()
        );
        let dns: Vec<&str> = root_children.iter().map(|n| n.dn.as_str()).collect();
        assert!(dns.contains(&"ou=Corporate,dc=contoso,dc=com"));
        assert!(dns.contains(&"ou=Groups,dc=contoso,dc=com"));
    }

    #[test]
    fn test_children_nested() {
        let dir = OfflineDirectory::load_embedded();
        let it_children = dir.children("ou=IT,ou=Corporate,dc=contoso,dc=com");
        // Should have: Infrastructure, Development, Security, Service Accounts
        assert_eq!(it_children.len(), 4, "IT should have 4 sub-OUs");
    }

    #[test]
    fn test_entry_lookup() {
        let dir = OfflineDirectory::load_embedded();
        let entry = dir.entry("cn=Administrator,dc=contoso,dc=com");
        assert!(entry.is_some());
        let entry = entry.unwrap();
        assert_eq!(entry.first_value("cn"), Some("Administrator"));
    }

    #[test]
    fn test_entry_lookup_case_insensitive() {
        let dir = OfflineDirectory::load_embedded();
        let entry = dir.entry("CN=Administrator,DC=contoso,DC=com");
        assert!(entry.is_some());
    }

    #[test]
    fn test_entry_not_found() {
        let dir = OfflineDirectory::load_embedded();
        assert!(dir.entry("cn=nobody,dc=contoso,dc=com").is_none());
    }

    #[test]
    fn test_search_all() {
        let dir = OfflineDirectory::load_embedded();
        let results = dir.search("dc=contoso,dc=com", "(objectClass=*)");
        assert_eq!(results.len(), dir.entries.len());
    }

    #[test]
    fn test_search_substring() {
        let dir = OfflineDirectory::load_embedded();
        let results = dir.search("dc=contoso,dc=com", "(cn=*Sarah*)");
        assert!(!results.is_empty(), "should find at least Sarah Chen");
        assert!(results.iter().any(|e| e.dn.contains("Sarah Chen")));
    }

    #[test]
    fn test_search_scoped() {
        let dir = OfflineDirectory::load_embedded();
        let results = dir.search("ou=Groups,dc=contoso,dc=com", "(objectClass=*)");
        // Should only return entries under Groups
        for entry in &results {
            let dn_lower = entry.dn.to_lowercase();
            assert!(
                dn_lower.ends_with(",ou=groups,dc=contoso,dc=com")
                    || dn_lower == "ou=groups,dc=contoso,dc=com",
                "entry {} should be under Groups",
                entry.dn
            );
        }
    }

    #[test]
    fn test_search_limited() {
        let dir = OfflineDirectory::load_embedded();
        let results = dir.search_limited("dc=contoso,dc=com", "contoso", 5);
        assert!(results.len() <= 5);
    }

    #[test]
    fn test_schema_has_attributes() {
        let dir = OfflineDirectory::load_embedded();
        let schema = dir.schema();
        assert!(schema.get_attribute_type("cn").is_some());
        assert!(schema.get_attribute_type("member").is_some());
        assert_eq!(schema.attribute_syntax("member"), AttributeSyntax::Dn);
    }

    #[test]
    fn test_schema_has_object_classes() {
        let dir = OfflineDirectory::load_embedded();
        let schema = dir.schema();
        assert!(schema.object_classes.contains_key("person"));
        assert!(schema.object_classes.contains_key("inetorgperson"));
        assert!(schema.object_classes.contains_key("groupofnames"));
    }

    #[test]
    fn test_extract_filter_value() {
        assert_eq!(extract_filter_value("(cn=*Sarah*)"), "Sarah");
        assert_eq!(
            extract_filter_value("(mail=test@example.com)"),
            "test@example.com"
        );
        assert_eq!(extract_filter_value("Sarah"), "Sarah");
        assert_eq!(extract_filter_value("(objectClass=*)"), "");
    }
}
