use std::io::Write;
use std::path::Path;

use crate::entry::LdapEntry;
use crate::error::CoreError;

use super::requested_attrs;

/// Export entries to LDIF format (RFC 2849).
pub fn export(
    entries: &[LdapEntry],
    path: &Path,
    attributes: &[String],
) -> Result<usize, CoreError> {
    let file = std::fs::File::create(path)
        .map_err(|e| CoreError::ExportError(format!("Failed to create file: {}", e)))?;
    let mut writer = std::io::BufWriter::new(file);

    write_ldif(&mut writer, entries, attributes)
}

/// Write entries in LDIF format to any writer.
pub fn write_ldif<W: Write>(
    writer: &mut W,
    entries: &[LdapEntry],
    attributes: &[String],
) -> Result<usize, CoreError> {
    let filtered = requested_attrs(attributes);
    let mut count = 0;

    for entry in entries {
        if count > 0 {
            writeln!(writer).map_err(|e| CoreError::ExportError(format!("Write failed: {}", e)))?;
        }

        // DN line
        if needs_base64(&entry.dn) {
            writeln!(writer, "dn:: {}", base64_encode(&entry.dn))
        } else {
            writeln!(writer, "dn: {}", entry.dn)
        }
        .map_err(|e| CoreError::ExportError(format!("Write failed: {}", e)))?;

        // Attributes
        if let Some(attrs) = filtered {
            for attr in attrs {
                if let Some(values) = entry.attributes.get(attr) {
                    for value in values {
                        if needs_base64(value) {
                            writeln!(writer, "{}:: {}", attr, base64_encode(value))
                        } else {
                            writeln!(writer, "{}: {}", attr, value)
                        }
                        .map_err(|e| CoreError::ExportError(format!("Write failed: {}", e)))?;
                    }
                }
            }
        } else {
            for (attr, values) in &entry.attributes {
                for value in values {
                    if needs_base64(value) {
                        writeln!(writer, "{}:: {}", attr, base64_encode(value))
                    } else {
                        writeln!(writer, "{}: {}", attr, value)
                    }
                    .map_err(|e| CoreError::ExportError(format!("Write failed: {}", e)))?;
                }
            }
        }

        count += 1;
    }

    writer
        .flush()
        .map_err(|e| CoreError::ExportError(format!("Flush failed: {}", e)))?;

    Ok(count)
}

/// Check if a value needs base64 encoding for LDIF.
fn needs_base64(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let first = s.as_bytes()[0];
    if first == b' ' || first == b':' || first == b'<' {
        return true;
    }
    s.bytes()
        .any(|b| b > 127 || (b < 32 && b != b'\n' && b != b'\r'))
}

fn base64_encode(s: &str) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(s.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn test_export_ldif() {
        let entries = vec![
            LdapEntry::new(
                "cn=Alice,ou=Users,dc=example,dc=com".to_string(),
                BTreeMap::from([
                    ("cn".to_string(), vec!["Alice".to_string()]),
                    ("sn".to_string(), vec!["Smith".to_string()]),
                    (
                        "objectClass".to_string(),
                        vec!["top".to_string(), "person".to_string()],
                    ),
                ]),
            ),
            LdapEntry::new(
                "cn=Bob,ou=Users,dc=example,dc=com".to_string(),
                BTreeMap::from([
                    ("cn".to_string(), vec!["Bob".to_string()]),
                    ("sn".to_string(), vec!["Jones".to_string()]),
                ]),
            ),
        ];

        let star = vec!["*".to_string()];
        let mut buf = Vec::new();
        let count = write_ldif(&mut buf, &entries, &star).unwrap();
        assert_eq!(count, 2);

        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("dn: cn=Alice,ou=Users,dc=example,dc=com"));
        assert!(output.contains("cn: Alice"));
        assert!(output.contains("objectClass: top"));
        assert!(output.contains("objectClass: person"));
        assert!(output.contains("dn: cn=Bob,ou=Users,dc=example,dc=com"));
    }

    #[test]
    fn test_base64_encoding() {
        assert!(!needs_base64("hello"));
        assert!(needs_base64(" leading space"));
        assert!(needs_base64(":colon"));
    }
}
