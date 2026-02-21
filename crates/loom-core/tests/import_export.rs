use std::path::Path;

use loom_core::export;
use loom_core::import;

#[test]
fn test_import_ldif_fixture() {
    let path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/fixtures/sample.ldif"
    ));
    let entries = import::ldif::import(path).unwrap();

    assert_eq!(entries.len(), 6);
    assert_eq!(entries[0].dn, "dc=example,dc=com");
    assert_eq!(entries[0].object_classes(), vec!["top", "domain"]);

    // Alice
    let alice = entries.iter().find(|e| e.dn.contains("Alice")).unwrap();
    assert_eq!(alice.first_value("cn"), Some("Alice Smith"));
    assert_eq!(alice.first_value("mail"), Some("alice@example.com"));

    // Bob has multi-valued telephoneNumber
    let bob = entries.iter().find(|e| e.dn.contains("Bob")).unwrap();
    let phones = bob.attributes.get("telephoneNumber").unwrap();
    assert_eq!(phones.len(), 2);
}

#[test]
fn test_import_json_fixture() {
    let path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/fixtures/sample.json"
    ));
    let entries = import::json::import(path).unwrap();

    assert_eq!(entries.len(), 4);
    assert_eq!(entries[0].dn, "dc=example,dc=com");

    let alice = entries.iter().find(|e| e.dn.contains("Alice")).unwrap();
    assert_eq!(alice.first_value("sn"), Some("Smith"));
}

#[test]
fn test_import_csv_fixture() {
    let path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/fixtures/sample.csv"
    ));
    let entries = import::csv::import(path).unwrap();

    assert_eq!(entries.len(), 4);
    assert_eq!(entries[0].dn, "dc=example,dc=com");

    let alice = entries.iter().find(|e| e.dn.contains("Alice")).unwrap();
    assert_eq!(alice.first_value("givenName"), Some("Alice"));
    assert!(alice.object_classes().contains(&"person"));
}

#[test]
fn test_ldif_roundtrip_with_fixtures() {
    let path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/fixtures/sample.ldif"
    ));
    let entries = import::ldif::import(path).unwrap();

    let star = vec!["*".to_string()];
    let mut buf = Vec::new();
    export::ldif::write_ldif(&mut buf, &entries, &star).unwrap();
    let ldif_str = String::from_utf8(buf).unwrap();

    let reimported = import::ldif::parse_ldif(&ldif_str).unwrap();
    assert_eq!(reimported.len(), entries.len());

    for (orig, re) in entries.iter().zip(reimported.iter()) {
        assert_eq!(orig.dn, re.dn);
        assert_eq!(orig.first_value("cn"), re.first_value("cn"));
    }
}

#[test]
fn test_json_roundtrip_with_fixtures() {
    let path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/fixtures/sample.json"
    ));
    let entries = import::json::import(path).unwrap();

    let star = vec!["*".to_string()];
    let json_str = export::json::to_string(&entries, &star).unwrap();

    let reimported = import::json::parse_json(&json_str).unwrap();
    assert_eq!(reimported.len(), entries.len());

    for (orig, re) in entries.iter().zip(reimported.iter()) {
        assert_eq!(orig.dn, re.dn);
    }
}

#[test]
fn test_csv_roundtrip_with_fixtures() {
    let path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/fixtures/sample.csv"
    ));
    let entries = import::csv::import(path).unwrap();

    let star = vec!["*".to_string()];
    let mut buf = Vec::new();
    export::csv::write_csv(&mut buf, &entries, &star).unwrap();
    let csv_str = String::from_utf8(buf).unwrap();

    let reimported = import::csv::parse_csv(&csv_str).unwrap();
    assert_eq!(reimported.len(), entries.len());

    for (orig, re) in entries.iter().zip(reimported.iter()) {
        assert_eq!(orig.dn, re.dn);
    }
}

#[test]
fn test_cross_format_ldif_to_json() {
    let path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/fixtures/sample.ldif"
    ));
    let entries = import::ldif::import(path).unwrap();

    let star = vec!["*".to_string()];
    let json_str = export::json::to_string(&entries, &star).unwrap();
    let reimported = import::json::parse_json(&json_str).unwrap();

    assert_eq!(reimported.len(), entries.len());
    for (orig, re) in entries.iter().zip(reimported.iter()) {
        assert_eq!(orig.dn, re.dn);
        assert_eq!(orig.attributes, re.attributes);
    }
}
