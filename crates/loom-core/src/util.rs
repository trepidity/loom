use std::collections::BTreeMap;

/// Get all values for an attribute (case-insensitive key lookup).
pub fn get_values(attrs: &BTreeMap<String, Vec<String>>, key: &str) -> Vec<String> {
    let key_lower = key.to_lowercase();
    for (k, v) in attrs {
        if k.to_lowercase() == key_lower {
            return v.clone();
        }
    }
    Vec::new()
}

/// Get the first value for an attribute (case-insensitive).
pub fn get_first(attrs: &BTreeMap<String, Vec<String>>, key: &str) -> Option<String> {
    get_values(attrs, key).into_iter().next()
}

/// Check if an attribute exists (case-insensitive).
pub fn has_attr(attrs: &BTreeMap<String, Vec<String>>, key: &str) -> bool {
    let key_lower = key.to_lowercase();
    attrs.keys().any(|k| k.to_lowercase() == key_lower)
}

/// Find values for an attribute (case-insensitive), returning a reference.
pub fn find_values_ci<'a>(
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
