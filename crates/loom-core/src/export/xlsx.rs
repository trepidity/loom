use std::collections::BTreeSet;
use std::path::Path;

use rust_xlsxwriter::{Format, Workbook};

use crate::entry::LdapEntry;
use crate::error::CoreError;

use super::requested_attrs;

/// Export entries to Excel (.xlsx) format.
pub fn export(
    entries: &[LdapEntry],
    path: &Path,
    attributes: &[String],
) -> Result<usize, CoreError> {
    if entries.is_empty() {
        return Ok(0);
    }

    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    worksheet
        .set_name("LDAP Entries")
        .map_err(|e| CoreError::ExportError(format!("Excel error: {}", e)))?;

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

    let header_format = Format::new().set_bold();

    // Header row
    worksheet
        .write_string_with_format(0, 0, "dn", &header_format)
        .map_err(|e| CoreError::ExportError(format!("Excel write error: {}", e)))?;

    for (col, attr) in attr_names.iter().enumerate() {
        worksheet
            .write_string_with_format(0, (col + 1) as u16, attr, &header_format)
            .map_err(|e| CoreError::ExportError(format!("Excel write error: {}", e)))?;
    }

    // Data rows
    for (row_idx, entry) in entries.iter().enumerate() {
        let row = (row_idx + 1) as u32;

        worksheet
            .write_string(row, 0, &entry.dn)
            .map_err(|e| CoreError::ExportError(format!("Excel write error: {}", e)))?;

        for (col_idx, attr) in attr_names.iter().enumerate() {
            let col = (col_idx + 1) as u16;
            let value = entry
                .attributes
                .get(attr)
                .map(|vals| vals.join("; "))
                .unwrap_or_default();
            if !value.is_empty() {
                worksheet
                    .write_string(row, col, &value)
                    .map_err(|e| CoreError::ExportError(format!("Excel write error: {}", e)))?;
            }
        }
    }

    worksheet
        .set_column_width(0, 50)
        .map_err(|e| CoreError::ExportError(format!("Excel error: {}", e)))?;

    workbook
        .save(path)
        .map_err(|e| CoreError::ExportError(format!("Excel save failed: {}", e)))?;

    Ok(entries.len())
}
