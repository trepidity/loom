use std::collections::BTreeMap;
use std::path::Path;

use crate::entry::LdapEntry;
use crate::error::CoreError;

/// Import entries from an LDIF file.
pub fn import(path: &Path) -> Result<Vec<LdapEntry>, CoreError> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| CoreError::ImportError(format!("Failed to read file: {}", e)))?;
    parse_ldif(&content)
}

/// Parse LDIF content string into entries.
pub fn parse_ldif(content: &str) -> Result<Vec<LdapEntry>, CoreError> {
    let mut entries = Vec::new();
    let mut current_dn: Option<String> = None;
    let mut current_attrs: BTreeMap<String, Vec<String>> = BTreeMap::new();

    // Handle line folding (lines starting with a single space are continuations)
    let unfolded = unfold_lines(content);

    for line in unfolded.lines() {
        let line = line.trim_end();

        // Empty line = end of entry
        if line.is_empty() {
            if let Some(dn) = current_dn.take() {
                entries.push(LdapEntry::new(dn, std::mem::take(&mut current_attrs)));
            }
            continue;
        }

        // Skip comments
        if line.starts_with('#') {
            continue;
        }

        // Skip version line
        if line.starts_with("version:") {
            continue;
        }

        // Parse attribute: value or attribute:: base64value
        if let Some((attr, value)) = parse_ldif_line(line) {
            if attr.eq_ignore_ascii_case("dn") {
                // Start a new entry
                if let Some(dn) = current_dn.take() {
                    entries.push(LdapEntry::new(dn, std::mem::take(&mut current_attrs)));
                }
                current_dn = Some(value);
            } else {
                current_attrs.entry(attr).or_default().push(value);
            }
        }
    }

    // Don't forget the last entry
    if let Some(dn) = current_dn {
        entries.push(LdapEntry::new(dn, current_attrs));
    }

    Ok(entries)
}

/// Parse a single LDIF line into (attribute, value).
fn parse_ldif_line(line: &str) -> Option<(String, String)> {
    // Check for base64: "attr:: base64value"
    if let Some(pos) = line.find(":: ") {
        let attr = line[..pos].to_string();
        let b64 = line[pos + 3..].trim();
        let value = base64_decode(b64).unwrap_or_else(|| b64.to_string());
        return Some((attr, value));
    }

    // Normal: "attr: value"
    if let Some(pos) = line.find(": ") {
        let attr = line[..pos].to_string();
        let value = line[pos + 2..].to_string();
        return Some((attr, value));
    }

    // "attr:" with empty value
    if let Some(attr) = line.strip_suffix(':') {
        if !attr.contains(' ') {
            return Some((attr.to_string(), String::new()));
        }
    }

    None
}

/// Unfold LDIF continuation lines (lines starting with a single space).
fn unfold_lines(content: &str) -> String {
    let mut result = String::with_capacity(content.len());
    for line in content.lines() {
        if line.starts_with(' ') && !result.is_empty() && !result.ends_with('\n') {
            // Continuation: append without the leading space
            result.push_str(&line[1..]);
        } else {
            if !result.is_empty() {
                result.push('\n');
            }
            result.push_str(line);
        }
    }
    result
}

fn base64_decode(s: &str) -> Option<String> {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD.decode(s).ok()?;
    String::from_utf8(bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ldif() {
        let ldif = r#"dn: cn=Alice,ou=Users,dc=example,dc=com
cn: Alice
sn: Smith
objectClass: top
objectClass: person

dn: cn=Bob,ou=Users,dc=example,dc=com
cn: Bob
sn: Jones
"#;

        let entries = parse_ldif(ldif).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].dn, "cn=Alice,ou=Users,dc=example,dc=com");
        assert_eq!(entries[0].first_value("cn"), Some("Alice"));
        assert_eq!(entries[0].object_classes(), vec!["top", "person"]);
        assert_eq!(entries[1].dn, "cn=Bob,ou=Users,dc=example,dc=com");
    }

    #[test]
    fn test_parse_ldif_base64() {
        let ldif = "dn: cn=Test,dc=example,dc=com\ncn:: VGVzdA==\n";
        let entries = parse_ldif(ldif).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].first_value("cn"), Some("Test"));
    }

    #[test]
    fn test_unfold_lines() {
        let input = "dn: cn=Very Long\n DN,dc=example,dc=com\ncn: Test\n";
        let unfolded = unfold_lines(input);
        assert!(unfolded.contains("dn: cn=Very LongDN,dc=example,dc=com"));
    }

    #[test]
    fn test_ldif_roundtrip() {
        let entries = vec![LdapEntry::new(
            "cn=Test,dc=example,dc=com".to_string(),
            BTreeMap::from([
                ("cn".to_string(), vec!["Test".to_string()]),
                ("sn".to_string(), vec!["User".to_string()]),
            ]),
        )];

        let star = vec!["*".to_string()];
        let mut buf = Vec::new();
        crate::export::ldif::write_ldif(&mut buf, &entries, &star).unwrap();
        let ldif_str = String::from_utf8(buf).unwrap();

        let reimported = parse_ldif(&ldif_str).unwrap();
        assert_eq!(reimported.len(), 1);
        assert_eq!(reimported[0].dn, entries[0].dn);
        assert_eq!(reimported[0].first_value("cn"), Some("Test"));
    }
}
