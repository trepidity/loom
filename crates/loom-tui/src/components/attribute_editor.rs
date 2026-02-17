use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

use crate::action::Action;
use crate::theme::Theme;
use loom_core::entry::LdapEntry;

/// Edit mode for an attribute value.
#[derive(Debug, Clone)]
pub enum EditOp {
    Replace { attr: String, old_value: String },
    Add { attr: String },
    Delete { attr: String, value: String },
}

/// Result of a completed edit operation.
#[derive(Debug, Clone)]
pub struct EditResult {
    pub dn: String,
    pub op: EditOp,
    pub new_value: String,
}

/// Which part of the editor has focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EditorFocus {
    Input,
    Results,
}

/// A popup dialog for editing a single attribute value,
/// with optional DN search-as-you-type mode.
pub struct AttributeEditor {
    pub visible: bool,
    dn: String,
    op: Option<EditOp>,
    input_buffer: String,
    cursor_pos: usize,
    theme: Theme,

    // DN search mode
    is_dn_search: bool,
    multi_select: bool,
    focus: EditorFocus,
    search_results: Vec<(String, String)>, // (dn, display_label)
    result_state: ListState,
    selected_dns: Vec<String>,
    search_generation: u64,
    search_dirty: bool,
    last_search_text: String,
    searching: bool,
}

impl AttributeEditor {
    pub fn new(theme: Theme) -> Self {
        Self {
            visible: false,
            dn: String::new(),
            op: None,
            input_buffer: String::new(),
            cursor_pos: 0,
            theme,
            is_dn_search: false,
            multi_select: false,
            focus: EditorFocus::Input,
            search_results: Vec::new(),
            result_state: ListState::default(),
            selected_dns: Vec::new(),
            search_generation: 0,
            search_dirty: false,
            last_search_text: String::new(),
            searching: false,
        }
    }

    fn reset_dn_search_state(&mut self) {
        self.is_dn_search = false;
        self.multi_select = false;
        self.focus = EditorFocus::Input;
        self.search_results.clear();
        self.result_state = ListState::default();
        self.selected_dns.clear();
        self.search_generation = 0;
        self.search_dirty = false;
        self.last_search_text.clear();
        self.searching = false;
    }

    /// Open editor to replace an existing attribute value.
    pub fn edit_value(&mut self, dn: String, attr: String, current_value: String) {
        self.dn = dn;
        self.input_buffer = current_value.clone();
        self.cursor_pos = self.input_buffer.len();
        self.op = Some(EditOp::Replace {
            attr,
            old_value: current_value,
        });
        self.reset_dn_search_state();
        self.visible = true;
    }

    /// Open editor to replace an existing attribute value, with DN search options.
    pub fn edit_value_with_options(
        &mut self,
        dn: String,
        attr: String,
        current_value: String,
        is_dn: bool,
        _multi_valued: bool,
    ) {
        self.edit_value(dn, attr, current_value);
        if is_dn {
            self.is_dn_search = true;
            // Replace mode is always single-value
            self.multi_select = false;
        }
    }

    /// Open editor to add a new value to an attribute.
    pub fn add_value(&mut self, dn: String, attr: String) {
        self.dn = dn;
        self.input_buffer.clear();
        self.cursor_pos = 0;
        self.op = Some(EditOp::Add { attr });
        self.reset_dn_search_state();
        self.visible = true;
    }

    /// Open editor to add a new value, with DN search options.
    pub fn add_value_with_options(
        &mut self,
        dn: String,
        attr: String,
        is_dn: bool,
        multi_valued: bool,
    ) {
        self.add_value(dn, attr);
        if is_dn {
            self.is_dn_search = true;
            self.multi_select = multi_valued;
        }
    }

    pub fn hide(&mut self) {
        self.visible = false;
        self.op = None;
        self.reset_dn_search_state();
    }

    /// Get the label describing the current operation.
    fn op_label(&self) -> String {
        match &self.op {
            Some(EditOp::Replace { attr, .. }) => format!("Edit: {}", attr),
            Some(EditOp::Add { attr }) => format!("Add value to: {}", attr),
            Some(EditOp::Delete { attr, .. }) => format!("Delete from: {}", attr),
            None => "Edit".to_string(),
        }
    }

    /// Get the current attribute name being edited, if any.
    fn current_attr(&self) -> Option<&str> {
        match &self.op {
            Some(EditOp::Replace { attr, .. }) => Some(attr),
            Some(EditOp::Add { attr }) => Some(attr),
            Some(EditOp::Delete { attr, .. }) => Some(attr),
            None => None,
        }
    }

    /// Tick-based debounce: called by App on Action::Tick when editor is visible.
    /// Returns a DnSearchRequest action if the input has changed and meets threshold.
    pub fn tick(&mut self, base_dn: &str) -> Action {
        if !self.visible || !self.is_dn_search || !self.search_dirty {
            return Action::None;
        }

        if self.input_buffer.len() >= 2 && self.input_buffer != self.last_search_text {
            self.search_dirty = false;
            self.last_search_text = self.input_buffer.clone();
            self.search_generation += 1;
            self.searching = true;

            let filter = build_dn_search_filter(&self.input_buffer);
            Action::DnSearchRequest {
                generation: self.search_generation,
                query: filter,
                base_dn: base_dn.to_string(),
            }
        } else {
            Action::None
        }
    }

    /// Receive search results from a background task.
    /// Ignores stale results (generation mismatch).
    pub fn receive_results(&mut self, generation: u64, entries: Vec<LdapEntry>) {
        if generation != self.search_generation {
            return; // stale
        }
        self.searching = false;
        self.search_results = entries
            .into_iter()
            .map(|e| {
                let label = e
                    .first_value("cn")
                    .or_else(|| e.first_value("uid"))
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| e.rdn().to_string());
                (e.dn.clone(), label)
            })
            .collect();
        if !self.search_results.is_empty() {
            self.result_state.select(Some(0));
        } else {
            self.result_state.select(None);
        }
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> Action {
        if !self.visible {
            return Action::None;
        }

        // Ctrl+Space: force-activate DN search mode
        if key.code == KeyCode::Char(' ') && key.modifiers.contains(KeyModifiers::CONTROL) {
            if !self.is_dn_search {
                self.is_dn_search = true;
                self.multi_select = matches!(&self.op, Some(EditOp::Add { .. }));
                self.search_dirty = true;
            }
            return Action::None;
        }

        if self.is_dn_search {
            match self.focus {
                EditorFocus::Input => self.handle_input_key_dn(key),
                EditorFocus::Results => self.handle_results_key(key),
            }
        } else {
            self.handle_input_key_plain(key)
        }
    }

    /// Handle key events in plain (non-DN-search) mode.
    fn handle_input_key_plain(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Enter => self.commit_plain(),
            KeyCode::Esc => {
                self.hide();
                Action::ClosePopup
            }
            _ => {
                self.edit_text(key);
                // Auto-detect DN pattern: input matches ^\w+=
                if !self.is_dn_search && looks_like_dn_input(&self.input_buffer) {
                    self.is_dn_search = true;
                    self.multi_select = matches!(&self.op, Some(EditOp::Add { .. }));
                    self.search_dirty = true;
                }
                Action::None
            }
        }
    }

    /// Handle key events when input is focused in DN search mode.
    fn handle_input_key_dn(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Enter => {
                // If multi-select has selections, commit those
                if self.multi_select && !self.selected_dns.is_empty() {
                    return self.commit_multi_select();
                }
                // Otherwise save input as-is
                self.commit_plain()
            }
            KeyCode::Esc => {
                self.hide();
                Action::ClosePopup
            }
            KeyCode::Tab | KeyCode::Down => {
                if !self.search_results.is_empty() {
                    self.focus = EditorFocus::Results;
                    if self.result_state.selected().is_none() {
                        self.result_state.select(Some(0));
                    }
                }
                Action::None
            }
            _ => {
                self.edit_text(key);
                self.search_dirty = true;
                Action::None
            }
        }
    }

    /// Handle key events when results list is focused.
    fn handle_results_key(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Esc | KeyCode::Tab => {
                self.focus = EditorFocus::Input;
                Action::None
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if let Some(selected) = self.result_state.selected() {
                    if selected == 0 {
                        // Move back to input
                        self.focus = EditorFocus::Input;
                    } else {
                        self.result_state.select(Some(selected - 1));
                    }
                }
                Action::None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let Some(selected) = self.result_state.selected() {
                    if selected + 1 < self.search_results.len() {
                        self.result_state.select(Some(selected + 1));
                    }
                }
                Action::None
            }
            KeyCode::Char(' ') if self.multi_select => {
                // Toggle selection on highlighted result
                if let Some(idx) = self.result_state.selected() {
                    if let Some((dn, _)) = self.search_results.get(idx) {
                        let dn = dn.clone();
                        if let Some(pos) = self.selected_dns.iter().position(|d| *d == dn) {
                            self.selected_dns.remove(pos);
                        } else {
                            self.selected_dns.push(dn);
                        }
                    }
                }
                Action::None
            }
            KeyCode::Enter => {
                if self.multi_select && !self.selected_dns.is_empty() {
                    return self.commit_multi_select();
                }
                // Single selection: pick highlighted DN
                if let Some(idx) = self.result_state.selected() {
                    if let Some((dn, _)) = self.search_results.get(idx) {
                        self.input_buffer = dn.clone();
                        self.cursor_pos = self.input_buffer.len();
                        return self.commit_plain();
                    }
                }
                Action::None
            }
            _ => Action::None,
        }
    }

    /// Commit the current input buffer as a plain edit result.
    fn commit_plain(&mut self) -> Action {
        if let Some(op) = self.op.take() {
            let result = EditResult {
                dn: self.dn.clone(),
                op,
                new_value: self.input_buffer.clone(),
            };
            self.visible = false;
            self.reset_dn_search_state();
            return Action::SaveAttribute(result);
        }
        Action::None
    }

    /// Commit multiple selected DNs.
    fn commit_multi_select(&mut self) -> Action {
        let attr = match self.current_attr() {
            Some(a) => a.to_string(),
            None => return Action::None,
        };
        let values = std::mem::take(&mut self.selected_dns);
        let dn = self.dn.clone();
        self.visible = false;
        self.op = None;
        self.reset_dn_search_state();
        Action::AddMultipleValues { dn, attr, values }
    }

    /// Apply text editing key to input buffer.
    fn edit_text(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Backspace => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                    self.input_buffer.remove(self.cursor_pos);
                }
            }
            KeyCode::Delete => {
                if self.cursor_pos < self.input_buffer.len() {
                    self.input_buffer.remove(self.cursor_pos);
                }
            }
            KeyCode::Left => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                }
            }
            KeyCode::Right => {
                if self.cursor_pos < self.input_buffer.len() {
                    self.cursor_pos += 1;
                }
            }
            KeyCode::Home => {
                self.cursor_pos = 0;
            }
            KeyCode::End => {
                self.cursor_pos = self.input_buffer.len();
            }
            KeyCode::Char(c) => {
                self.input_buffer.insert(self.cursor_pos, c);
                self.cursor_pos += 1;
            }
            _ => {}
        }
    }

    pub fn render(&mut self, frame: &mut Frame, full: Rect) {
        if !self.visible {
            return;
        }

        if self.is_dn_search {
            self.render_dn_search(frame, full);
        } else {
            self.render_plain(frame, full);
        }
    }

    fn render_plain(&self, frame: &mut Frame, full: Rect) {
        let popup_width = (full.width as u32 * 60 / 100).min(80) as u16;
        let popup_height = 7u16.min(full.height);

        let x = full.x + (full.width.saturating_sub(popup_width)) / 2;
        let y = full.y + (full.height.saturating_sub(popup_height)) / 2;
        let area = Rect::new(x, y, popup_width, popup_height);

        frame.render_widget(Clear, area);

        let title = format!(" {} ", self.op_label());
        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(self.theme.popup_border)
            .title_style(self.theme.popup_title);

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let layout = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(1),
        ])
        .split(inner);

        // DN line
        let dn_line = Line::from(vec![
            Span::styled("DN: ", self.theme.header),
            Span::styled(&self.dn, self.theme.dimmed),
        ]);
        frame.render_widget(Paragraph::new(dn_line), layout[0]);

        // Input with cursor
        self.render_input_line(frame, layout[1]);

        // Hint
        let hint = Line::from(Span::styled(
            "Enter: save  Esc: cancel  Ctrl+Space: DN search",
            self.theme.dimmed,
        ));
        frame.render_widget(Paragraph::new(hint), layout[2]);
    }

    fn render_dn_search(&mut self, frame: &mut Frame, full: Rect) {
        let popup_width = (full.width as u32 * 70 / 100).min(100) as u16;
        // Dynamic height: 4 (border+dn+input+hint) + results
        let results_height = (self.search_results.len() as u16).clamp(3, 10);
        let popup_height = (4 + results_height + 2).min(full.height); // +2 for border

        let x = full.x + (full.width.saturating_sub(popup_width)) / 2;
        let y = full.y + (full.height.saturating_sub(popup_height)) / 2;
        let area = Rect::new(x, y, popup_width, popup_height);

        frame.render_widget(Clear, area);

        let title = format!(" {} ", self.op_label());
        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(self.theme.popup_border)
            .title_style(self.theme.popup_title);

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let layout = Layout::vertical([
            Constraint::Length(1), // DN
            Constraint::Length(1), // Input
            Constraint::Min(1),    // Results
            Constraint::Length(1), // Hint
        ])
        .split(inner);

        // DN line
        let dn_line = Line::from(vec![
            Span::styled("DN: ", self.theme.header),
            Span::styled(&self.dn, self.theme.dimmed),
        ]);
        frame.render_widget(Paragraph::new(dn_line), layout[0]);

        // Input line with optional "Searching..." indicator
        let input_area = layout[1];
        let input_parts =
            Layout::horizontal([Constraint::Min(1), Constraint::Length(14)]).split(input_area);

        self.render_input_line(frame, input_parts[0]);

        if self.searching {
            let spinner = Span::styled("Searching...", self.theme.dimmed);
            frame.render_widget(Paragraph::new(Line::from(spinner)), input_parts[1]);
        }

        // Results list
        let results_area = layout[2];
        if self.search_results.is_empty() {
            let empty_msg = if self.searching {
                ""
            } else if self.input_buffer.len() < 2 {
                "Type at least 2 characters to search"
            } else {
                "No results"
            };
            let empty = Paragraph::new(Line::from(Span::styled(empty_msg, self.theme.dimmed)));
            frame.render_widget(empty, results_area);
        } else {
            let items: Vec<ListItem> = self
                .search_results
                .iter()
                .enumerate()
                .map(|(idx, (dn, label))| {
                    let is_selected = self.selected_dns.contains(dn);
                    let checkbox = if self.multi_select {
                        if is_selected {
                            "[x] "
                        } else {
                            "[ ] "
                        }
                    } else {
                        ""
                    };

                    // Truncate DN for display
                    let max_dn_len =
                        (popup_width as usize).saturating_sub(label.len() + checkbox.len() + 6);
                    let dn_display = if dn.len() > max_dn_len {
                        format!("{}..)", &dn[..max_dn_len.saturating_sub(3)])
                    } else {
                        format!("({})", dn)
                    };

                    let style = if Some(idx) == self.result_state.selected()
                        && self.focus == EditorFocus::Results
                    {
                        self.theme.selected
                    } else if is_selected {
                        self.theme.header
                    } else {
                        self.theme.normal
                    };

                    ListItem::new(Line::from(Span::styled(
                        format!("{}{}  {}", checkbox, label, dn_display),
                        style,
                    )))
                })
                .collect();

            let list = List::new(items);
            frame.render_stateful_widget(list, results_area, &mut self.result_state);
        }

        // Hint line
        let hint_text = if self.multi_select && !self.selected_dns.is_empty() {
            format!(
                "Space: toggle  Enter: add {}  Esc: cancel",
                self.selected_dns.len()
            )
        } else if self.multi_select {
            "Space: toggle  Enter: save  Tab: results  Esc: cancel".to_string()
        } else {
            "Enter: save  Tab: results  Esc: cancel".to_string()
        };
        let hint = Line::from(Span::styled(hint_text, self.theme.dimmed));
        frame.render_widget(Paragraph::new(hint), layout[3]);
    }

    fn render_input_line(&self, frame: &mut Frame, area: Rect) {
        let (before_cursor, after_cursor) = self.input_buffer.split_at(self.cursor_pos);
        let cursor_style = if self.focus == EditorFocus::Input {
            self.theme.selected
        } else {
            self.theme.normal
        };
        let input_line = Line::from(vec![
            Span::styled(before_cursor, self.theme.normal),
            Span::styled(
                if after_cursor.is_empty() {
                    "_"
                } else {
                    &after_cursor[..1]
                },
                cursor_style,
            ),
            Span::styled(
                if after_cursor.len() > 1 {
                    &after_cursor[1..]
                } else {
                    ""
                },
                self.theme.normal,
            ),
        ]);
        frame.render_widget(Paragraph::new(input_line), area);
    }
}

/// Check if input looks like a DN fragment (matches `^\w+=`).
fn looks_like_dn_input(input: &str) -> bool {
    if input.len() < 3 {
        return false;
    }
    let bytes = input.as_bytes();
    let eq_pos = bytes.iter().position(|&b| b == b'=');
    match eq_pos {
        Some(pos) if pos > 0 => bytes[..pos]
            .iter()
            .all(|&b| b.is_ascii_alphanumeric() || b == b'-'),
        _ => false,
    }
}

/// Build an LDAP filter from user input for DN search.
/// - Bare text `john` → `(|(cn=*john*)(uid=*john*)(sn=*john*)(mail=*john*))`
/// - Pattern `cn=john` → `(cn=john*)`
pub fn build_dn_search_filter(input: &str) -> String {
    let input = input.trim();
    if input.is_empty() {
        return "(objectClass=*)".to_string();
    }

    // Check if input is in attr=value format
    if let Some(eq_pos) = input.find('=') {
        let attr_part = &input[..eq_pos];
        if !attr_part.is_empty()
            && attr_part
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        {
            let value_part = ldap_escape(&input[eq_pos + 1..]);
            return format!("({}={}*)", attr_part, value_part);
        }
    }

    // Bare text: search across common naming attributes
    let escaped = ldap_escape(input);
    format!(
        "(|(cn=*{}*)(uid=*{}*)(sn=*{}*)(mail=*{}*))",
        escaped, escaped, escaped, escaped
    )
}

/// Escape special characters for LDAP filter values per RFC 4515.
fn ldap_escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '*' => out.push_str("\\2a"),
            '(' => out.push_str("\\28"),
            ')' => out.push_str("\\29"),
            '\\' => out.push_str("\\5c"),
            '\0' => out.push_str("\\00"),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_filter_bare_text() {
        let filter = build_dn_search_filter("john");
        assert_eq!(filter, "(|(cn=*john*)(uid=*john*)(sn=*john*)(mail=*john*))");
    }

    #[test]
    fn test_build_filter_attr_value() {
        let filter = build_dn_search_filter("cn=john");
        assert_eq!(filter, "(cn=john*)");
    }

    #[test]
    fn test_build_filter_empty() {
        let filter = build_dn_search_filter("");
        assert_eq!(filter, "(objectClass=*)");
    }

    #[test]
    fn test_build_filter_whitespace() {
        let filter = build_dn_search_filter("  john  ");
        assert_eq!(filter, "(|(cn=*john*)(uid=*john*)(sn=*john*)(mail=*john*))");
    }

    #[test]
    fn test_ldap_escape() {
        assert_eq!(ldap_escape("hello"), "hello");
        assert_eq!(ldap_escape("a*b"), "a\\2ab");
        assert_eq!(ldap_escape("a(b)c"), "a\\28b\\29c");
        assert_eq!(ldap_escape("a\\b"), "a\\5cb");
    }

    #[test]
    fn test_looks_like_dn_input() {
        assert!(looks_like_dn_input("cn=john"));
        assert!(looks_like_dn_input("uid=test"));
        assert!(looks_like_dn_input("ou=People"));
        assert!(!looks_like_dn_input("john"));
        assert!(!looks_like_dn_input("=john"));
        assert!(!looks_like_dn_input("ab"));
        assert!(!looks_like_dn_input(""));
    }
}
