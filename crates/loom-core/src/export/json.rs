use std::collections::BTreeMap;
use std::path::Path;

use crate::entry::LdapEntry;
use crate::error::CoreError;

use super::requested_attrs;

/// Filter entries to include only the requested attributes.
fn filter_entries(entries: &[LdapEntry], attributes: &[String]) -> Vec<LdapEntry> {
    if let Some(attrs) = requested_attrs(attributes) {
        entries
            .iter()
            .map(|entry| {
                let filtered: BTreeMap<String, Vec<String>> = attrs
                    .iter()
                    .filter_map(|a| entry.attributes.get(a).map(|v| (a.clone(), v.clone())))
                    .collect();
                LdapEntry::new(entry.dn.clone(), filtered)
            })
            .collect()
    } else {
        entries.to_vec()
    }
}

/// Export entries to JSON format (array of entry objects).
pub fn export(
    entries: &[LdapEntry],
    path: &Path,
    attributes: &[String],
) -> Result<usize, CoreError> {
    let filtered = filter_entries(entries, attributes);
    let json = serde_json::to_string_pretty(&filtered)
        .map_err(|e| CoreError::ExportError(format!("JSON serialization failed: {}", e)))?;

    std::fs::write(path, json)
        .map_err(|e| CoreError::ExportError(format!("Failed to write file: {}", e)))?;

    Ok(entries.len())
}

/// Serialize entries to a JSON string.
pub fn to_string(entries: &[LdapEntry], attributes: &[String]) -> Result<String, CoreError> {
    let filtered = filter_entries(entries, attributes);
    serde_json::to_string_pretty(&filtered)
        .map_err(|e| CoreError::ExportError(format!("JSON serialization failed: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn test_export_json_roundtrip() {
        let entries = vec![LdapEntry::new(
            "cn=Test,dc=example,dc=com".to_string(),
            BTreeMap::from([
                ("cn".to_string(), vec!["Test".to_string()]),
                ("sn".to_string(), vec!["User".to_string()]),
            ]),
        )];

        let star = vec!["*".to_string()];
        let json = to_string(&entries, &star).unwrap();
        assert!(json.contains("cn=Test,dc=example,dc=com"));

        let parsed: Vec<LdapEntry> = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].dn, "cn=Test,dc=example,dc=com");
    }
}
