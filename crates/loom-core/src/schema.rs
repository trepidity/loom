use std::collections::BTreeMap;

use ldap3::{Scope, SearchEntry};
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use crate::connection::LdapConnection;
use crate::error::CoreError;

/// Known LDAP attribute syntaxes mapped to friendly types.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AttributeSyntax {
    String,
    DirectoryString,
    Integer,
    Boolean,
    Dn,
    OctetString,
    GeneralizedTime,
    TelephoneNumber,
    Oid,
    Other(String),
}

/// An LDAP attribute type definition from the schema.
#[derive(Debug, Clone)]
pub struct AttributeTypeInfo {
    pub oid: String,
    pub names: Vec<String>,
    pub description: Option<String>,
    pub syntax: AttributeSyntax,
    pub single_value: bool,
    pub no_user_modification: bool,
}

/// An LDAP object class definition.
#[derive(Debug, Clone)]
pub struct ObjectClassInfo {
    pub oid: String,
    pub names: Vec<String>,
    pub description: Option<String>,
    pub superior: Option<String>,
    pub kind: ObjectClassKind,
    pub must: Vec<String>,
    pub may: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ObjectClassKind {
    Abstract,
    Structural,
    Auxiliary,
}

/// Cached schema information for a connection.
#[derive(Debug, Clone)]
pub struct SchemaCache {
    pub attribute_types: BTreeMap<String, AttributeTypeInfo>,
    pub object_classes: BTreeMap<String, ObjectClassInfo>,
}

impl SchemaCache {
    pub fn new() -> Self {
        Self {
            attribute_types: BTreeMap::new(),
            object_classes: BTreeMap::new(),
        }
    }

    /// Lookup an attribute type by name (case-insensitive).
    pub fn get_attribute_type(&self, name: &str) -> Option<&AttributeTypeInfo> {
        let name_lower = name.to_lowercase();
        self.attribute_types.get(&name_lower)
    }

    /// Get the syntax for an attribute name.
    pub fn attribute_syntax(&self, name: &str) -> AttributeSyntax {
        self.get_attribute_type(name)
            .map(|at| at.syntax.clone())
            .unwrap_or(AttributeSyntax::String)
    }

    /// Check if an attribute is single-valued.
    pub fn is_single_valued(&self, name: &str) -> bool {
        self.get_attribute_type(name)
            .map(|at| at.single_value)
            .unwrap_or(false)
    }
}

impl Default for SchemaCache {
    fn default() -> Self {
        Self::new()
    }
}

impl LdapConnection {
    /// Discover and load the schema from the server.
    pub async fn load_schema(
        &mut self,
        subschema_dn: Option<&str>,
    ) -> Result<SchemaCache, CoreError> {
        // Determine schema DN
        let schema_dn = match subschema_dn {
            Some(dn) => dn.to_string(),
            None => "cn=Subschema".to_string(),
        };

        debug!("Loading schema from: {}", schema_dn);

        let result = self
            .ldap
            .search(
                &schema_dn,
                Scope::Base,
                "(objectClass=*)",
                vec!["attributeTypes", "objectClasses"],
            )
            .await
            .map_err(CoreError::Ldap)?;

        let (entries, _res) = result
            .success()
            .map_err(|e| CoreError::SchemaError(format!("Schema search failed: {}", e)))?;

        let mut cache = SchemaCache::new();

        if let Some(entry) = entries.into_iter().next().map(SearchEntry::construct) {
            let attrs: BTreeMap<String, Vec<String>> = entry.attrs.into_iter().collect();

            // Parse attributeTypes
            if let Some(attr_types) = find_values_ci(&attrs, "attributetypes") {
                for def in attr_types {
                    match parse_attribute_type(def) {
                        Some(at) => {
                            for name in &at.names {
                                cache
                                    .attribute_types
                                    .insert(name.to_lowercase(), at.clone());
                            }
                        }
                        None => {
                            debug!(
                                "Failed to parse attributeType: {}",
                                &def[..def.len().min(80)]
                            );
                        }
                    }
                }
            }

            // Parse objectClasses
            if let Some(obj_classes) = find_values_ci(&attrs, "objectclasses") {
                for def in obj_classes {
                    match parse_object_class(def) {
                        Some(oc) => {
                            for name in &oc.names {
                                cache.object_classes.insert(name.to_lowercase(), oc.clone());
                            }
                        }
                        None => {
                            debug!("Failed to parse objectClass: {}", &def[..def.len().min(80)]);
                        }
                    }
                }
            }

            debug!(
                "Loaded schema: {} attribute types, {} object classes",
                cache.attribute_types.len(),
                cache.object_classes.len()
            );
        } else {
            warn!("No schema entry returned from {}", schema_dn);
        }

        Ok(cache)
    }
}

/// Case-insensitive attribute lookup.
fn find_values_ci<'a>(
    attrs: &'a BTreeMap<String, Vec<String>>,
    key: &str,
) -> Option<&'a Vec<String>> {
    let key_lower = key.to_lowercase();
    for (k, v) in attrs {
        if k.to_lowercase() == key_lower {
            return Some(v);
        }
    }
    None
}

/// Parse an LDAP attributeType schema definition string.
/// Format: ( OID NAME 'name' DESC 'desc' SYNTAX oid SINGLE-VALUE ... )
fn parse_attribute_type(def: &str) -> Option<AttributeTypeInfo> {
    let def = def.trim();
    if !def.starts_with('(') || !def.ends_with(')') {
        return None;
    }
    let inner = &def[1..def.len() - 1].trim();

    let oid = inner.split_whitespace().next()?.to_string();
    let names = parse_names(inner);
    let description = parse_quoted_field(inner, "DESC");
    let syntax_oid = parse_unquoted_field(inner, "SYNTAX");
    let single_value = inner.contains("SINGLE-VALUE");
    let no_user_modification = inner.contains("NO-USER-MODIFICATION");

    let syntax = syntax_oid
        .as_deref()
        .map(map_syntax_oid)
        .unwrap_or(AttributeSyntax::String);

    Some(AttributeTypeInfo {
        oid,
        names,
        description,
        syntax,
        single_value,
        no_user_modification,
    })
}

/// Parse an LDAP objectClass schema definition string.
fn parse_object_class(def: &str) -> Option<ObjectClassInfo> {
    let def = def.trim();
    if !def.starts_with('(') || !def.ends_with(')') {
        return None;
    }
    let inner = &def[1..def.len() - 1].trim();

    let oid = inner.split_whitespace().next()?.to_string();
    let names = parse_names(inner);
    let description = parse_quoted_field(inner, "DESC");
    let superior = parse_unquoted_field(inner, "SUP");

    let kind = if inner.contains("ABSTRACT") {
        ObjectClassKind::Abstract
    } else if inner.contains("AUXILIARY") {
        ObjectClassKind::Auxiliary
    } else {
        ObjectClassKind::Structural
    };

    let must = parse_attr_list(inner, "MUST");
    let may = parse_attr_list(inner, "MAY");

    Some(ObjectClassInfo {
        oid,
        names,
        description,
        superior,
        kind,
        must,
        may,
    })
}

/// Parse NAME field — can be 'single' or ( 'multiple' 'names' ).
fn parse_names(s: &str) -> Vec<String> {
    if let Some(pos) = s.find("NAME") {
        let rest = &s[pos + 4..].trim_start();
        if rest.starts_with('(') {
            // Multiple names: ( 'name1' 'name2' )
            if let Some(end) = rest.find(')') {
                let names_str = &rest[1..end];
                return names_str
                    .split('\'')
                    .filter(|s| !s.trim().is_empty())
                    .map(|s| s.to_string())
                    .collect();
            }
        } else if let Some(rest) = rest.strip_prefix('\'') {
            // Single name: 'name'
            if let Some(end) = rest.find('\'') {
                return vec![rest[..end].to_string()];
            }
        }
    }
    Vec::new()
}

/// Parse a single-quoted field value: KEYWORD 'value'.
fn parse_quoted_field(s: &str, keyword: &str) -> Option<String> {
    let pattern = format!("{} '", keyword);
    if let Some(pos) = s.find(&pattern) {
        let rest = &s[pos + pattern.len()..];
        if let Some(end) = rest.find('\'') {
            return Some(rest[..end].to_string());
        }
    }
    None
}

/// Parse an unquoted field value: KEYWORD value (terminated by space or paren).
fn parse_unquoted_field(s: &str, keyword: &str) -> Option<String> {
    let pattern = format!("{} ", keyword);
    if let Some(pos) = s.find(&pattern) {
        let rest = &s[pos + pattern.len()..].trim_start();
        // Take until next space, or strip leading/trailing braces
        let val: String = rest
            .split_whitespace()
            .next()?
            .trim_matches(|c| c == '{' || c == '}')
            .to_string();
        if !val.is_empty() {
            return Some(val);
        }
    }
    None
}

/// Parse an attribute list: KEYWORD ( attr1 $ attr2 ) or KEYWORD attr.
fn parse_attr_list(s: &str, keyword: &str) -> Vec<String> {
    let pattern = format!("{} ", keyword);
    if let Some(pos) = s.find(&pattern) {
        let rest = &s[pos + pattern.len()..].trim_start();
        if rest.starts_with('(') {
            if let Some(end) = rest.find(')') {
                return rest[1..end]
                    .split('$')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
        } else {
            // Single attribute
            if let Some(val) = rest.split_whitespace().next() {
                return vec![val.to_string()];
            }
        }
    }
    Vec::new()
}

/// Map LDAP syntax OID to our AttributeSyntax enum.
fn map_syntax_oid(oid: &str) -> AttributeSyntax {
    // Strip any length constraint like {128}
    let oid = oid.split('{').next().unwrap_or(oid);
    match oid {
        "1.3.6.1.4.1.1466.115.121.1.15" => AttributeSyntax::DirectoryString,
        "1.3.6.1.4.1.1466.115.121.1.26" => AttributeSyntax::String, // IA5String
        "1.3.6.1.4.1.1466.115.121.1.27" => AttributeSyntax::Integer,
        "1.3.6.1.4.1.1466.115.121.1.7" => AttributeSyntax::Boolean,
        "1.3.6.1.4.1.1466.115.121.1.12" => AttributeSyntax::Dn,
        "1.3.6.1.4.1.1466.115.121.1.40" => AttributeSyntax::OctetString,
        "1.3.6.1.4.1.1466.115.121.1.24" => AttributeSyntax::GeneralizedTime,
        "1.3.6.1.4.1.1466.115.121.1.50" => AttributeSyntax::TelephoneNumber,
        "1.3.6.1.4.1.1466.115.121.1.38" => AttributeSyntax::Oid,
        "1.3.6.1.4.1.1466.115.121.1.5" => AttributeSyntax::OctetString, // Binary
        "1.3.6.1.4.1.1466.115.121.1.44" => AttributeSyntax::String,     // PrintableString
        "1.3.6.1.4.1.1466.115.121.1.36" => AttributeSyntax::String,     // NumericString
        _ => AttributeSyntax::Other(oid.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_attribute_type_single_name() {
        let def =
            "( 2.5.4.3 NAME 'cn' DESC 'Common Name' SYNTAX 1.3.6.1.4.1.1466.115.121.1.15{64} )";
        let at = parse_attribute_type(def).unwrap();
        assert_eq!(at.oid, "2.5.4.3");
        assert_eq!(at.names, vec!["cn"]);
        assert_eq!(at.description, Some("Common Name".to_string()));
        assert_eq!(at.syntax, AttributeSyntax::DirectoryString);
        assert!(!at.single_value);
    }

    #[test]
    fn test_parse_attribute_type_multi_name() {
        let def =
            "( 2.5.4.4 NAME ( 'sn' 'surname' ) SYNTAX 1.3.6.1.4.1.1466.115.121.1.15 SINGLE-VALUE )";
        let at = parse_attribute_type(def).unwrap();
        assert_eq!(at.names, vec!["sn", "surname"]);
        assert!(at.single_value);
    }

    #[test]
    fn test_parse_attribute_type_boolean() {
        let def = "( 1.2.3.4 NAME 'enabled' SYNTAX 1.3.6.1.4.1.1466.115.121.1.7 SINGLE-VALUE )";
        let at = parse_attribute_type(def).unwrap();
        assert_eq!(at.syntax, AttributeSyntax::Boolean);
    }

    #[test]
    fn test_parse_object_class() {
        let def = "( 2.5.6.6 NAME 'person' DESC 'RFC2256: a person' SUP top STRUCTURAL MUST ( sn $ cn ) MAY ( userPassword $ telephoneNumber ) )";
        let oc = parse_object_class(def).unwrap();
        assert_eq!(oc.oid, "2.5.6.6");
        assert_eq!(oc.names, vec!["person"]);
        assert_eq!(oc.superior, Some("top".to_string()));
        assert_eq!(oc.kind, ObjectClassKind::Structural);
        assert_eq!(oc.must, vec!["sn", "cn"]);
        assert_eq!(oc.may, vec!["userPassword", "telephoneNumber"]);
    }

    #[test]
    fn test_map_syntax_oid() {
        assert_eq!(
            map_syntax_oid("1.3.6.1.4.1.1466.115.121.1.15"),
            AttributeSyntax::DirectoryString
        );
        assert_eq!(
            map_syntax_oid("1.3.6.1.4.1.1466.115.121.1.7"),
            AttributeSyntax::Boolean
        );
        assert_eq!(
            map_syntax_oid("1.3.6.1.4.1.1466.115.121.1.27"),
            AttributeSyntax::Integer
        );
    }
}
