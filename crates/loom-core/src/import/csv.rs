use std::collections::BTreeMap;
use std::path::Path;

use crate::entry::LdapEntry;
use crate::error::CoreError;

/// Import entries from a CSV file.
///
/// Expects first column to be "dn", remaining columns are attribute names.
/// Multi-valued attributes should be separated by "; ".
pub fn import(path: &Path) -> Result<Vec<LdapEntry>, CoreError> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| CoreError::ImportError(format!("Failed to read file: {}", e)))?;
    parse_csv(&content)
}

/// Parse CSV content string into entries.
pub fn parse_csv(content: &str) -> Result<Vec<LdapEntry>, CoreError> {
    let mut reader = csv::Reader::from_reader(content.as_bytes());

    let headers: Vec<String> = reader
        .headers()
        .map_err(|e| CoreError::ImportError(format!("CSV header error: {}", e)))?
        .iter()
        .map(|s| s.to_string())
        .collect();

    if headers.is_empty() {
        return Ok(Vec::new());
    }

    // Find the DN column (should be first, but search by name)
    let dn_idx = headers
        .iter()
        .position(|h| h.eq_ignore_ascii_case("dn"))
        .ok_or_else(|| CoreError::ImportError("CSV missing 'dn' column".to_string()))?;

    let mut entries = Vec::new();

    for result in reader.records() {
        let record =
            result.map_err(|e| CoreError::ImportError(format!("CSV record error: {}", e)))?;

        let dn = record.get(dn_idx).unwrap_or("").to_string();

        if dn.is_empty() {
            continue;
        }

        let mut attributes = BTreeMap::new();

        for (idx, header) in headers.iter().enumerate() {
            if idx == dn_idx {
                continue;
            }
            if let Some(value) = record.get(idx) {
                let value = value.trim();
                if !value.is_empty() {
                    // Split multi-valued attributes on "; "
                    let values: Vec<String> = value.split("; ").map(|s| s.to_string()).collect();
                    attributes.insert(header.clone(), values);
                }
            }
        }

        entries.push(LdapEntry::new(dn, attributes));
    }

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_csv() {
        let csv_data =
            "dn,cn,sn,objectClass\n\"cn=Alice,dc=example\",Alice,Smith,\"top; person\"\n";

        let entries = parse_csv(csv_data).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].dn, "cn=Alice,dc=example");
        assert_eq!(entries[0].first_value("cn"), Some("Alice"));
        assert_eq!(
            entries[0].attributes.get("objectClass").unwrap(),
            &vec!["top", "person"]
        );
    }

    #[test]
    fn test_csv_roundtrip() {
        use std::collections::BTreeMap;

        let entries = vec![LdapEntry::new(
            "cn=Test,dc=example,dc=com".to_string(),
            BTreeMap::from([
                ("cn".to_string(), vec!["Test".to_string()]),
                ("sn".to_string(), vec!["User".to_string()]),
            ]),
        )];

        let star = vec!["*".to_string()];
        let mut buf = Vec::new();
        crate::export::csv::write_csv(&mut buf, &entries, &star).unwrap();
        let csv_str = String::from_utf8(buf).unwrap();

        let reimported = parse_csv(&csv_str).unwrap();
        assert_eq!(reimported.len(), 1);
        assert_eq!(reimported[0].dn, entries[0].dn);
        assert_eq!(reimported[0].first_value("cn"), Some("Test"));
    }
}
