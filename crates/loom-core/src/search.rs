use ldap3::{Scope, SearchEntry};
use tracing::debug;

use crate::connection::LdapConnection;
use crate::entry::LdapEntry;
use crate::error::CoreError;

impl LdapConnection {
    /// Search for immediate children of the given DN.
    pub async fn search_children(&mut self, parent_dn: &str) -> Result<Vec<LdapEntry>, CoreError> {
        self.search(
            parent_dn,
            Scope::OneLevel,
            "(objectClass=*)",
            vec!["*"],
        )
        .await
    }

    /// Search for a single entry by exact DN.
    /// Requests only user attributes ("*"). Operational attributes are excluded
    /// to avoid displaying non-modifiable server-internal attributes.
    pub async fn search_entry(&mut self, dn: &str) -> Result<Option<LdapEntry>, CoreError> {
        let results = self
            .search(dn, Scope::Base, "(objectClass=*)", vec!["*"])
            .await?;
        Ok(results.into_iter().next())
    }

    /// Search a subtree with the given filter.
    pub async fn search_subtree(
        &mut self,
        base_dn: &str,
        filter: &str,
        attrs: Vec<&str>,
    ) -> Result<Vec<LdapEntry>, CoreError> {
        self.search(base_dn, Scope::Subtree, filter, attrs).await
    }

    /// Perform a paged LDAP search.
    async fn search(
        &mut self,
        base_dn: &str,
        scope: Scope,
        filter: &str,
        attrs: Vec<&str>,
    ) -> Result<Vec<LdapEntry>, CoreError> {
        let page_size = self.settings.page_size;
        let mut all_entries = Vec::new();
        let mut cookie = Vec::new();

        loop {
            let controls = vec![ldap3::controls::RawControl {
                ctype: "1.2.840.113556.1.4.319".to_string(), // pagedResultsControl OID
                crit: false,
                val: Some(encode_paged_results_control(page_size, &cookie)),
            }];

            let result = self
                .ldap
                .with_controls(controls)
                .search(base_dn, scope, filter, attrs.clone())
                .await
                .map_err(CoreError::Ldap)?;

            let (entries, res) = result
                .success()
                .map_err(|e| CoreError::SearchFailed(format!("{}", e)))?;

            let count = entries.len();
            for entry in entries {
                all_entries.push(LdapEntry::from_search_entry(SearchEntry::construct(entry)));
            }

            debug!(
                "Paged search: got {} entries (total: {})",
                count,
                all_entries.len()
            );

            // Extract the cookie from the response control
            cookie = extract_paged_results_cookie(&res);
            if cookie.is_empty() {
                break;
            }
        }

        Ok(all_entries)
    }
}

/// Encode a Simple Paged Results control value (RFC 2696).
fn encode_paged_results_control(page_size: u32, cookie: &[u8]) -> Vec<u8> {
    // BER encoding: SEQUENCE { INTEGER size, OCTET STRING cookie }
    let size_bytes = ber_encode_integer(page_size as i64);
    let cookie_bytes = ber_encode_octet_string(cookie);

    let mut content = Vec::new();
    content.extend_from_slice(&size_bytes);
    content.extend_from_slice(&cookie_bytes);

    let mut result = Vec::new();
    result.push(0x30); // SEQUENCE tag
    ber_encode_length(&mut result, content.len());
    result.extend_from_slice(&content);
    result
}

/// Extract the cookie from a paged results response control.
fn extract_paged_results_cookie(res: &ldap3::LdapResult) -> Vec<u8> {
    for ctrl in &res.ctrls {
        if ctrl.1.ctype == "1.2.840.113556.1.4.319" {
            if let Some(ref val) = ctrl.1.val {
                return parse_paged_results_cookie(val);
            }
        }
    }
    Vec::new()
}

/// Parse the cookie from the BER-encoded paged results control value.
fn parse_paged_results_cookie(data: &[u8]) -> Vec<u8> {
    // SEQUENCE { INTEGER size, OCTET STRING cookie }
    if data.len() < 2 || data[0] != 0x30 {
        return Vec::new();
    }

    let (seq_len, offset) = ber_decode_length(&data[1..]);
    if 1 + offset + seq_len > data.len() {
        return Vec::new();
    }
    let seq_data = &data[1 + offset..1 + offset + seq_len];

    // Skip the INTEGER (size)
    if seq_data.is_empty() || seq_data[0] != 0x02 {
        return Vec::new();
    }
    let (int_len, int_offset) = ber_decode_length(&seq_data[1..]);
    let remaining = &seq_data[1 + int_offset + int_len..];

    // Parse the OCTET STRING (cookie)
    if remaining.is_empty() || remaining[0] != 0x04 {
        return Vec::new();
    }
    let (cookie_len, cookie_offset) = ber_decode_length(&remaining[1..]);
    if 1 + cookie_offset + cookie_len > remaining.len() {
        return Vec::new();
    }
    remaining[1 + cookie_offset..1 + cookie_offset + cookie_len].to_vec()
}

fn ber_encode_integer(val: i64) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut v = val;
    loop {
        bytes.push((v & 0xFF) as u8);
        v >>= 8;
        if v == 0 && (bytes.last().unwrap() & 0x80) == 0 {
            break;
        }
        if v == -1 && (bytes.last().unwrap() & 0x80) != 0 {
            break;
        }
    }
    bytes.reverse();

    let mut result = vec![0x02]; // INTEGER tag
    ber_encode_length(&mut result, bytes.len());
    result.extend_from_slice(&bytes);
    result
}

fn ber_encode_octet_string(data: &[u8]) -> Vec<u8> {
    let mut result = vec![0x04]; // OCTET STRING tag
    ber_encode_length(&mut result, data.len());
    result.extend_from_slice(data);
    result
}

fn ber_encode_length(buf: &mut Vec<u8>, len: usize) {
    if len < 128 {
        buf.push(len as u8);
    } else if len < 256 {
        buf.push(0x81);
        buf.push(len as u8);
    } else {
        buf.push(0x82);
        buf.push((len >> 8) as u8);
        buf.push((len & 0xFF) as u8);
    }
}

fn ber_decode_length(data: &[u8]) -> (usize, usize) {
    if data.is_empty() {
        return (0, 0);
    }
    if data[0] < 128 {
        (data[0] as usize, 1)
    } else {
        let num_bytes = (data[0] & 0x7F) as usize;
        let mut len = 0usize;
        for i in 0..num_bytes {
            if i + 1 < data.len() {
                len = (len << 8) | data[i + 1] as usize;
            }
        }
        (len, 1 + num_bytes)
    }
}
