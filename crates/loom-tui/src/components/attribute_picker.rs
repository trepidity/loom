use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

use crate::action::Action;
use crate::theme::Theme;
use crate::widgets::fuzzy_input::{FuzzyFilter, FuzzyMatch};

/// Focus within the picker: either the text input or the results list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PickerFocus {
    Input,
    List,
}

/// A popup for fuzzy-searching and selecting an attribute name.
pub struct AttributePicker {
    pub visible: bool,
    dn: String,
    input: String,
    cursor: usize,
    all_items: Vec<(String, String)>, // (attr_name, syntax_label)
    item_names: Vec<String>,          // just names for fuzzy matching
    filtered: Vec<FuzzyMatch>,
    selected_idx: usize,
    fuzzy: FuzzyFilter,
    focus: PickerFocus,
    theme: Theme,
}

impl AttributePicker {
    pub fn new(theme: Theme) -> Self {
        Self {
            visible: false,
            dn: String::new(),
            input: String::new(),
            cursor: 0,
            all_items: Vec::new(),
            item_names: Vec::new(),
            filtered: Vec::new(),
            selected_idx: 0,
            fuzzy: FuzzyFilter::new(),
            focus: PickerFocus::Input,
            theme,
        }
    }

    /// Open the picker with the given DN and candidate attributes.
    pub fn show(&mut self, dn: String, candidates: Vec<(String, String)>) {
        self.dn = dn;
        self.input.clear();
        self.cursor = 0;
        self.item_names = candidates.iter().map(|(name, _)| name.clone()).collect();
        self.all_items = candidates;
        self.selected_idx = 0;
        self.focus = PickerFocus::Input;
        self.refilter();
        self.visible = true;
    }

    pub fn hide(&mut self) {
        self.visible = false;
        self.all_items.clear();
        self.item_names.clear();
        self.filtered.clear();
    }

    fn refilter(&mut self) {
        self.filtered = self.fuzzy.filter(&self.input, &self.item_names);
        self.selected_idx = 0;
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> Action {
        if !self.visible {
            return Action::None;
        }

        match self.focus {
            PickerFocus::Input => self.handle_input_key(key),
            PickerFocus::List => self.handle_list_key(key),
        }
    }

    fn handle_input_key(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Esc => {
                self.hide();
                Action::ClosePopup
            }
            KeyCode::Enter => self.pick_selected(),
            KeyCode::Down | KeyCode::Tab => {
                if !self.filtered.is_empty() {
                    self.focus = PickerFocus::List;
                    self.selected_idx = 0;
                }
                Action::None
            }
            KeyCode::Backspace => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                    self.input.remove(self.cursor);
                    self.refilter();
                }
                Action::None
            }
            KeyCode::Delete => {
                if self.cursor < self.input.len() {
                    self.input.remove(self.cursor);
                    self.refilter();
                }
                Action::None
            }
            KeyCode::Left => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                }
                Action::None
            }
            KeyCode::Right => {
                if self.cursor < self.input.len() {
                    self.cursor += 1;
                }
                Action::None
            }
            KeyCode::Home => {
                self.cursor = 0;
                Action::None
            }
            KeyCode::End => {
                self.cursor = self.input.len();
                Action::None
            }
            KeyCode::Char(c) => {
                self.input.insert(self.cursor, c);
                self.cursor += 1;
                self.refilter();
                Action::None
            }
            _ => Action::None,
        }
    }

    fn handle_list_key(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Esc => {
                self.hide();
                Action::ClosePopup
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected_idx == 0 {
                    self.focus = PickerFocus::Input;
                } else {
                    self.selected_idx -= 1;
                }
                Action::None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected_idx + 1 < self.filtered.len() {
                    self.selected_idx += 1;
                }
                Action::None
            }
            KeyCode::Tab => {
                self.focus = PickerFocus::Input;
                Action::None
            }
            KeyCode::Enter => self.pick_selected(),
            _ => Action::None,
        }
    }

    /// Pick the currently selected attribute and emit AddAttribute action.
    /// Falls back to the raw input as a freeform attribute name when no
    /// list match is available (e.g. schema not loaded).
    fn pick_selected(&mut self) -> Action {
        // Try the highlighted list item first
        if let Some(fm) = self.filtered.get(self.selected_idx) {
            if let Some((attr_name, _)) = self.all_items.get(fm.index) {
                let action =
                    Action::AddAttribute(self.dn.clone(), attr_name.clone());
                self.hide();
                return action;
            }
        }
        // Freeform fallback: use whatever the user typed
        let name = self.input.trim().to_string();
        if !name.is_empty() {
            let action = Action::AddAttribute(self.dn.clone(), name);
            self.hide();
            return action;
        }
        Action::None
    }

    pub fn render(&self, frame: &mut Frame, full: Rect) {
        if !self.visible {
            return;
        }

        let popup_width = (full.width as u32 * 50 / 100).clamp(40, 80) as u16;
        let popup_height = (full.height as u32 * 50 / 100).clamp(10, 30) as u16;

        let x = full.x + (full.width.saturating_sub(popup_width)) / 2;
        let y = full.y + (full.height.saturating_sub(popup_height)) / 2;
        let area = Rect::new(x, y, popup_width, popup_height);

        frame.render_widget(Clear, area);

        let block = Block::default()
            .title(" Add Attribute ")
            .borders(Borders::ALL)
            .border_style(self.theme.popup_border)
            .title_style(self.theme.popup_title);

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if inner.height < 4 {
            return;
        }

        let layout = Layout::vertical([
            Constraint::Length(1), // input
            Constraint::Length(1), // match count
            Constraint::Min(1),   // list
            Constraint::Length(1), // hints
        ])
        .split(inner);

        // Input line with cursor
        let (before, after) = self.input.split_at(self.cursor);
        let cursor_style = if self.focus == PickerFocus::Input {
            self.theme.selected
        } else {
            self.theme.normal
        };
        let input_line = Line::from(vec![
            Span::styled("> ", self.theme.header),
            Span::styled(before, self.theme.normal),
            Span::styled(
                if after.is_empty() { "_" } else { &after[..1] },
                cursor_style,
            ),
            Span::styled(
                if after.len() > 1 { &after[1..] } else { "" },
                self.theme.normal,
            ),
        ]);
        frame.render_widget(Paragraph::new(input_line), layout[0]);

        // Match count / freeform hint
        let count_line = if self.all_items.is_empty() {
            Line::from(Span::styled(
                "Type an attribute name",
                self.theme.dimmed,
            ))
        } else {
            Line::from(Span::styled(
                format!("{}/{} attributes", self.filtered.len(), self.all_items.len()),
                self.theme.dimmed,
            ))
        };
        frame.render_widget(Paragraph::new(count_line), layout[1]);

        // Filtered list
        let list_area = layout[2];
        let visible_count = list_area.height as usize;

        // Compute scroll offset to keep selected item visible
        let scroll_offset = if self.selected_idx >= visible_count {
            self.selected_idx - visible_count + 1
        } else {
            0
        };

        let items: Vec<ListItem> = self
            .filtered
            .iter()
            .skip(scroll_offset)
            .take(visible_count)
            .enumerate()
            .map(|(display_idx, fm)| {
                let actual_idx = display_idx + scroll_offset;
                let (name, syntax) = &self.all_items[fm.index];
                let is_highlighted = actual_idx == self.selected_idx
                    && self.focus == PickerFocus::List;
                let style = if is_highlighted {
                    self.theme.selected
                } else {
                    self.theme.normal
                };
                let syntax_style = if is_highlighted {
                    self.theme.selected
                } else {
                    self.theme.dimmed
                };
                ListItem::new(Line::from(vec![
                    Span::styled(name, style),
                    Span::styled(format!("  {}", syntax), syntax_style),
                ]))
            })
            .collect();

        let mut list_state = ListState::default();
        if self.focus == PickerFocus::List && !self.filtered.is_empty() {
            list_state.select(Some(self.selected_idx - scroll_offset));
        }
        let list = List::new(items);
        frame.render_stateful_widget(list, list_area, &mut list_state);

        // Hints
        let hint = Line::from(Span::styled(
            "Enter:Select  Esc:Cancel",
            self.theme.dimmed,
        ));
        frame.render_widget(Paragraph::new(hint), layout[3]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_candidates() -> Vec<(String, String)> {
        vec![
            ("cn".to_string(), "DirectoryString".to_string()),
            ("sn".to_string(), "DirectoryString".to_string()),
            ("mail".to_string(), "IA5String".to_string()),
            ("uid".to_string(), "DirectoryString".to_string()),
            ("telephoneNumber".to_string(), "TelephoneNumber".to_string()),
        ]
    }

    #[test]
    fn test_show_populates_items() {
        let theme = Theme::load("dark");
        let mut picker = AttributePicker::new(theme);
        picker.show("cn=Test,dc=example".to_string(), make_candidates());
        assert!(picker.visible);
        assert_eq!(picker.all_items.len(), 5);
        assert_eq!(picker.filtered.len(), 5); // empty query shows all
    }

    #[test]
    fn test_fuzzy_filter_narrows() {
        let theme = Theme::load("dark");
        let mut picker = AttributePicker::new(theme);
        picker.show("cn=Test,dc=example".to_string(), make_candidates());

        // Type "ma" to filter
        picker.input = "ma".to_string();
        picker.cursor = 2;
        picker.refilter();
        assert!(picker.filtered.len() < 5);
        // "mail" should match
        let matched_names: Vec<&str> = picker
            .filtered
            .iter()
            .map(|fm| picker.all_items[fm.index].0.as_str())
            .collect();
        assert!(matched_names.contains(&"mail"));
    }

    #[test]
    fn test_pick_emits_add_attribute() {
        let theme = Theme::load("dark");
        let mut picker = AttributePicker::new(theme);
        picker.show("cn=Test,dc=example".to_string(), make_candidates());
        picker.selected_idx = 0;

        let action = picker.pick_selected();
        assert!(!picker.visible);
        match action {
            Action::AddAttribute(dn, attr) => {
                assert_eq!(dn, "cn=Test,dc=example");
                assert_eq!(attr, "cn"); // first sorted item
            }
            _ => panic!("Expected AddAttribute action"),
        }
    }

    #[test]
    fn test_esc_closes() {
        let theme = Theme::load("dark");
        let mut picker = AttributePicker::new(theme);
        picker.show("cn=Test,dc=example".to_string(), make_candidates());

        let action = picker.handle_key_event(KeyEvent::from(KeyCode::Esc));
        assert!(!picker.visible);
        assert!(matches!(action, Action::ClosePopup));
    }

    #[test]
    fn test_freeform_input_no_candidates() {
        let theme = Theme::load("dark");
        let mut picker = AttributePicker::new(theme);
        // Open with empty candidates (no schema)
        picker.show("cn=Test,dc=example".to_string(), vec![]);
        assert!(picker.visible);
        assert!(picker.filtered.is_empty());

        // Type a custom attribute name
        picker.input = "description".to_string();
        picker.cursor = 11;

        let action = picker.pick_selected();
        assert!(!picker.visible);
        match action {
            Action::AddAttribute(dn, attr) => {
                assert_eq!(dn, "cn=Test,dc=example");
                assert_eq!(attr, "description");
            }
            _ => panic!("Expected AddAttribute from freeform input"),
        }
    }

    #[test]
    fn test_freeform_input_no_match() {
        let theme = Theme::load("dark");
        let mut picker = AttributePicker::new(theme);
        picker.show("cn=Test,dc=example".to_string(), make_candidates());

        // Type something that doesn't match any candidate
        picker.input = "zzzzNotAnAttr".to_string();
        picker.cursor = 13;
        picker.refilter();
        assert!(picker.filtered.is_empty());

        let action = picker.pick_selected();
        assert!(!picker.visible);
        match action {
            Action::AddAttribute(dn, attr) => {
                assert_eq!(dn, "cn=Test,dc=example");
                assert_eq!(attr, "zzzzNotAnAttr");
            }
            _ => panic!("Expected AddAttribute from freeform fallback"),
        }
    }

    #[test]
    fn test_freeform_empty_input_does_nothing() {
        let theme = Theme::load("dark");
        let mut picker = AttributePicker::new(theme);
        picker.show("cn=Test,dc=example".to_string(), vec![]);

        let action = picker.pick_selected();
        assert!(picker.visible, "should stay open with empty input");
        assert!(matches!(action, Action::None));
    }

    #[test]
    fn test_hide_clears_state() {
        let theme = Theme::load("dark");
        let mut picker = AttributePicker::new(theme);
        picker.show("cn=Test,dc=example".to_string(), make_candidates());
        picker.hide();
        assert!(!picker.visible);
        assert!(picker.all_items.is_empty());
        assert!(picker.filtered.is_empty());
    }
}
