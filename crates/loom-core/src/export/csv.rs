use std::collections::BTreeSet;
use std::path::Path;

use crate::entry::LdapEntry;
use crate::error::CoreError;

use super::requested_attrs;

/// Export entries to CSV format.
///
/// Columns: dn, then all unique attribute names sorted alphabetically.
/// Multi-valued attributes are joined with "; ".
pub fn export(
    entries: &[LdapEntry],
    path: &Path,
    attributes: &[String],
) -> Result<usize, CoreError> {
    let file = std::fs::File::create(path)
        .map_err(|e| CoreError::ExportError(format!("Failed to create file: {}", e)))?;
    let writer = std::io::BufWriter::new(file);

    write_csv(writer, entries, attributes)
}

/// Write entries in CSV format to any writer.
pub fn write_csv<W: std::io::Write>(
    writer: W,
    entries: &[LdapEntry],
    attributes: &[String],
) -> Result<usize, CoreError> {
    if entries.is_empty() {
        return Ok(0);
    }

    let attr_names: Vec<String> = if let Some(attrs) = requested_attrs(attributes) {
        attrs.to_vec()
    } else {
        let mut all_attrs: BTreeSet<String> = BTreeSet::new();
        for entry in entries {
            for key in entry.attributes.keys() {
                all_attrs.insert(key.clone());
            }
        }
        all_attrs.into_iter().collect()
    };

    let mut csv_writer = csv::Writer::from_writer(writer);

    // Header: dn + attribute names
    let mut header = vec!["dn".to_string()];
    header.extend(attr_names.iter().cloned());
    csv_writer
        .write_record(&header)
        .map_err(|e| CoreError::ExportError(format!("CSV write failed: {}", e)))?;

    // Data rows
    for entry in entries {
        let mut record = vec![entry.dn.clone()];
        for attr in &attr_names {
            let value = entry
                .attributes
                .get(attr)
                .map(|vals| vals.join("; "))
                .unwrap_or_default();
            record.push(value);
        }
        csv_writer
            .write_record(&record)
            .map_err(|e| CoreError::ExportError(format!("CSV write failed: {}", e)))?;
    }

    csv_writer
        .flush()
        .map_err(|e| CoreError::ExportError(format!("CSV flush failed: {}", e)))?;

    Ok(entries.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn test_export_csv() {
        let entries = vec![
            LdapEntry::new(
                "cn=Alice,dc=example,dc=com".to_string(),
                BTreeMap::from([
                    ("cn".to_string(), vec!["Alice".to_string()]),
                    ("mail".to_string(), vec!["alice@example.com".to_string()]),
                ]),
            ),
            LdapEntry::new(
                "cn=Bob,dc=example,dc=com".to_string(),
                BTreeMap::from([
                    ("cn".to_string(), vec!["Bob".to_string()]),
                    ("sn".to_string(), vec!["Jones".to_string()]),
                ]),
            ),
        ];

        let star = vec!["*".to_string()];
        let mut buf = Vec::new();
        let count = write_csv(&mut buf, &entries, &star).unwrap();
        assert_eq!(count, 2);

        let output = String::from_utf8(buf).unwrap();
        assert!(output.starts_with("dn,cn,mail,sn\n"));
        assert!(output.contains("Alice"));
        assert!(output.contains("alice@example.com"));
    }

    #[test]
    fn test_multi_valued() {
        let entries = vec![LdapEntry::new(
            "cn=Test,dc=example,dc=com".to_string(),
            BTreeMap::from([(
                "objectClass".to_string(),
                vec!["top".to_string(), "person".to_string()],
            )]),
        )];

        let star = vec!["*".to_string()];
        let mut buf = Vec::new();
        write_csv(&mut buf, &entries, &star).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("top; person"));
    }

    #[test]
    fn test_filtered_ordered_attributes() {
        let entries = vec![
            LdapEntry::new(
                "cn=Alice,dc=example,dc=com".to_string(),
                BTreeMap::from([
                    ("cn".to_string(), vec!["Alice".to_string()]),
                    ("mail".to_string(), vec!["alice@example.com".to_string()]),
                    ("sn".to_string(), vec!["Smith".to_string()]),
                    ("title".to_string(), vec!["Engineer".to_string()]),
                ]),
            ),
            LdapEntry::new(
                "cn=Bob,dc=example,dc=com".to_string(),
                BTreeMap::from([
                    ("cn".to_string(), vec!["Bob".to_string()]),
                    ("sn".to_string(), vec!["Jones".to_string()]),
                ]),
            ),
        ];

        // Request only mail, cn (in that order) — sn and title should be excluded
        let attrs = vec!["mail".to_string(), "cn".to_string()];
        let mut buf = Vec::new();
        let count = write_csv(&mut buf, &entries, &attrs).unwrap();
        assert_eq!(count, 2);

        let output = String::from_utf8(buf).unwrap();
        let lines: Vec<&str> = output.lines().collect();
        // Header should be dn,mail,cn (user-specified order)
        assert_eq!(lines[0], "dn,mail,cn");
        // Alice has both (DN contains commas so CSV quotes it)
        assert_eq!(
            lines[1],
            "\"cn=Alice,dc=example,dc=com\",alice@example.com,Alice"
        );
        // Bob has no mail → empty cell
        assert_eq!(lines[2], "\"cn=Bob,dc=example,dc=com\",,Bob");
    }
}
