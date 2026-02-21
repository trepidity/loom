use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Rect};
use ratatui::style::Modifier;
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, BorderType, Borders, Cell, Row, Table, TableState};
use ratatui::Frame;

use crate::action::{Action, ContextMenuSource};
use crate::component::Component;
use crate::theme::Theme;
use loom_core::entry::LdapEntry;
use loom_core::schema::SchemaCache;

/// Whether an attribute is user-editable or operational/system.
#[derive(Clone, Copy, PartialEq, Eq)]
enum AttrKind {
    Normal,
    Operational,
}

/// Flattened attribute row for table display.
struct AttrRow {
    attr_name: String,
    /// Original value (for editing, copying, deleting).
    raw_value: String,
    /// Sanitized value for display (control chars replaced).
    display_value: String,
    /// True for first value of an attribute (displays the attribute name).
    is_first: bool,
    kind: AttrKind,
}

/// The top-right panel: entry detail viewer.
pub struct DetailPanel {
    pub entry: Option<LdapEntry>,
    pub table_state: TableState,
    rows: Vec<AttrRow>,
    theme: Theme,
    area: Option<Rect>,
}

impl DetailPanel {
    pub fn new(theme: Theme) -> Self {
        Self {
            entry: None,
            table_state: TableState::default(),
            rows: Vec::new(),
            theme,
            area: None,
        }
    }

    pub fn set_entry(&mut self, entry: LdapEntry, schema: Option<&SchemaCache>) {
        self.rows = build_rows(&entry, schema);
        self.table_state
            .select(if self.rows.is_empty() { None } else { Some(0) });
        self.entry = Some(entry);
    }

    pub fn clear(&mut self) {
        self.entry = None;
        self.rows.clear();
        self.table_state.select(None);
    }

    /// Get the attribute name and raw value at the currently selected row.
    pub fn selected_attr_value(&self) -> Option<(&str, &str)> {
        let idx = self.table_state.selected()?;
        let row = self.rows.get(idx)?;
        Some((&row.attr_name, &row.raw_value))
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                let i = self.table_state.selected().unwrap_or(0);
                if i > 0 {
                    self.table_state.select(Some(i - 1));
                }
                Action::None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let i = self.table_state.selected().unwrap_or(0);
                if i + 1 < self.rows.len() {
                    self.table_state.select(Some(i + 1));
                }
                Action::None
            }
            KeyCode::PageUp => {
                let i = self.table_state.selected().unwrap_or(0);
                self.table_state.select(Some(i.saturating_sub(10)));
                Action::None
            }
            KeyCode::PageDown => {
                let i = self.table_state.selected().unwrap_or(0);
                let max = self.rows.len().saturating_sub(1);
                self.table_state.select(Some((i + 10).min(max)));
                Action::None
            }
            KeyCode::Char('e') => {
                // Edit the selected attribute value
                if let (Some(entry), Some((attr, val))) = (&self.entry, self.selected_attr_value())
                {
                    return Action::EditAttribute(
                        entry.dn.clone(),
                        attr.to_string(),
                        val.to_string(),
                    );
                }
                Action::None
            }
            KeyCode::Enter => {
                // Jump to the selected value as a DN
                if let Some((_attr, val)) = self.selected_attr_value() {
                    return Action::TreeSelect(val.to_string());
                }
                Action::None
            }
            KeyCode::Char('a') => {
                // Open attribute picker to add a new attribute
                if let Some(entry) = &self.entry {
                    return Action::ShowAddAttribute(entry.dn.clone());
                }
                Action::None
            }
            KeyCode::Char('+') => {
                // Add value to selected attribute (reuses existing attribute editor)
                if let (Some(entry), Some((attr, _val))) = (&self.entry, self.selected_attr_value())
                {
                    let row = self.rows.get(self.table_state.selected().unwrap_or(0));
                    if row.map(|r| r.kind) != Some(AttrKind::Operational) {
                        return Action::AddAttribute(entry.dn.clone(), attr.to_string());
                    }
                }
                Action::None
            }
            KeyCode::Char('d') | KeyCode::Delete => {
                // Delete selected attribute value (with confirmation)
                if let (Some(entry), Some((attr, val))) = (&self.entry, self.selected_attr_value())
                {
                    let row = self.rows.get(self.table_state.selected().unwrap_or(0));
                    if row.map(|r| r.kind) != Some(AttrKind::Operational) {
                        let msg = format!(
                            "Delete value '{}' from '{}'?",
                            flatten_for_message(val),
                            attr
                        );
                        return Action::ShowConfirm(
                            msg,
                            Box::new(Action::DeleteAttributeValue(
                                entry.dn.clone(),
                                attr.to_string(),
                                val.to_string(),
                            )),
                        );
                    }
                }
                Action::None
            }
            KeyCode::Char('r') => Action::EntryRefresh,
            KeyCode::Char(' ') => {
                if let (Some(entry), Some((attr, val))) = (&self.entry, self.selected_attr_value())
                {
                    Action::ShowContextMenu(ContextMenuSource::Detail {
                        dn: entry.dn.clone(),
                        attr_name: attr.to_string(),
                        attr_value: val.to_string(),
                    })
                } else {
                    Action::None
                }
            }
            _ => Action::None,
        }
    }
}

impl Component for DetailPanel {
    fn render(&self, frame: &mut Frame, area: Rect, focused: bool) {
        let border_style = if focused {
            self.theme.border_focused
        } else {
            self.theme.border
        };

        let mut block = Block::default()
            .title(" Details ")
            .borders(Borders::ALL)
            .border_style(border_style);
        if focused {
            block = block.border_type(BorderType::Double);
        }

        if let Some(ref entry) = self.entry {
            // Build header with DN
            let dn_line = Line::from(vec![
                Span::styled("DN: ", self.theme.header),
                Span::styled(&entry.dn, self.theme.normal),
            ]);

            // Build attribute rows
            let rows: Vec<Row> = self
                .rows
                .iter()
                .map(|r| {
                    let attr_style = match r.kind {
                        AttrKind::Operational => self.theme.attr_operational,
                        AttrKind::Normal => self.theme.header,
                    };
                    let value_style = match r.kind {
                        AttrKind::Operational => self.theme.attr_operational,
                        AttrKind::Normal => self.theme.normal,
                    };
                    let attr_display = if r.is_first { r.attr_name.as_str() } else { "" };

                    let value_lines: Vec<Line> = r
                        .display_value
                        .split('\n')
                        .map(|line| Line::from(Span::styled(line.to_string(), value_style)))
                        .collect();
                    let height = value_lines.len() as u16;

                    Row::new(vec![
                        Cell::from(Span::styled(attr_display, attr_style)),
                        Cell::from(Text::from(value_lines)),
                    ])
                    .height(height)
                })
                .collect();

            let widths = [Constraint::Percentage(30), Constraint::Percentage(70)];

            let table = Table::new(rows, widths)
                .header(
                    Row::new(vec![
                        Cell::from(Span::styled("Attribute", self.theme.header)),
                        Cell::from(Span::styled("Value", self.theme.header)),
                    ])
                    .style(self.theme.header),
                )
                .block(block)
                .highlight_style(self.theme.selected.add_modifier(Modifier::BOLD));

            frame.render_stateful_widget(table, area, &mut self.table_state.clone());

            // Render DN above the table (inside the block)
            let inner = area.inner(ratatui::layout::Margin {
                vertical: 1,
                horizontal: 1,
            });
            if inner.height > 0 {
                frame.render_widget(
                    ratatui::widgets::Paragraph::new(dn_line),
                    Rect {
                        x: inner.x,
                        y: inner.y,
                        width: inner.width,
                        height: 1,
                    },
                );
            }
        } else {
            let empty = ratatui::widgets::Paragraph::new("Select an entry from the tree")
                .style(self.theme.dimmed)
                .block(block);
            frame.render_widget(empty, area);
        }
    }

    fn last_area(&self) -> Option<Rect> {
        self.area
    }
}

/// Sanitize a value for multi-line display: normalize line endings and
/// replace non-printable control characters, but preserve newlines.
fn sanitize_for_display(value: &str) -> String {
    let normalized = value.replace("\r\n", "\n").replace('\r', "\n");
    let mut out = String::with_capacity(normalized.len());
    for ch in normalized.chars() {
        match ch {
            '\n' => out.push('\n'),
            '\t' => out.push_str("    "),
            c if c.is_control() => out.push('\u{FFFD}'),
            c => out.push(c),
        }
    }
    out
}

/// Flatten a value into a single line for use in confirmation dialogs etc.
fn flatten_for_message(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\n' | '\r' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {}
            c => out.push(c),
        }
    }
    out
}

fn build_rows(entry: &LdapEntry, schema: Option<&SchemaCache>) -> Vec<AttrRow> {
    let mut rows = Vec::new();
    for (name, values) in &entry.attributes {
        let kind = schema
            .and_then(|s| s.get_attribute_type(name))
            .map(|at| {
                if at.no_user_modification {
                    AttrKind::Operational
                } else {
                    AttrKind::Normal
                }
            })
            .unwrap_or(AttrKind::Normal);
        for (i, val) in values.iter().enumerate() {
            rows.push(AttrRow {
                attr_name: name.clone(),
                raw_value: val.clone(),
                display_value: sanitize_for_display(val),
                is_first: i == 0,
                kind,
            });
        }
    }
    rows
}
