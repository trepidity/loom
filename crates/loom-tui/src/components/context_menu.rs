use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::action::Action;
use crate::theme::Theme;

/// A single item in the context menu.
pub struct MenuItem {
    pub label: String,
    /// Shortcut hint shown right-aligned (e.g. "a", "F4").
    pub hint: String,
    pub action: Action,
}

/// A context-sensitive popup menu triggered by Space or right-click.
pub struct ContextMenu {
    pub visible: bool,
    items: Vec<MenuItem>,
    selected: usize,
    anchor: Option<(u16, u16)>,
    theme: Theme,
}

impl ContextMenu {
    pub fn new(theme: Theme) -> Self {
        Self {
            visible: false,
            items: Vec::new(),
            selected: 0,
            anchor: None,
            theme,
        }
    }

    /// Show the menu for a tree node.
    pub fn show_for_tree(&mut self, dn: &str) {
        self.items = vec![
            MenuItem {
                label: "Copy DN".into(),
                hint: String::new(),
                action: Action::CopyToClipboard(dn.to_string()),
            },
            MenuItem {
                label: "Create Child Entry".into(),
                hint: "a".into(),
                action: Action::ShowCreateEntryDialog(dn.to_string()),
            },
            MenuItem {
                label: "Export Subtree".into(),
                hint: "F4".into(),
                action: Action::ShowExportDialog,
            },
            MenuItem {
                label: "Refresh".into(),
                hint: "r".into(),
                action: Action::EntryRefresh,
            },
            MenuItem {
                label: "Delete Entry".into(),
                hint: "d".into(),
                action: Action::ShowConfirm(
                    format!("Delete entry?\n{}", dn),
                    Box::new(Action::DeleteEntry(dn.to_string())),
                ),
            },
        ];
        self.selected = 0;
        self.anchor = None;
        self.visible = true;
    }

    /// Show the menu for a detail panel attribute.
    pub fn show_for_detail(&mut self, dn: &str, attr_name: &str, attr_value: &str) {
        self.items = vec![
            MenuItem {
                label: "Copy Attribute Name".into(),
                hint: String::new(),
                action: Action::CopyToClipboard(attr_name.to_string()),
            },
            MenuItem {
                label: "Copy Attribute Value".into(),
                hint: String::new(),
                action: Action::CopyToClipboard(attr_value.to_string()),
            },
            MenuItem {
                label: "Copy DN".into(),
                hint: String::new(),
                action: Action::CopyToClipboard(dn.to_string()),
            },
            MenuItem {
                label: "Edit Value".into(),
                hint: "e".into(),
                action: Action::EditAttribute(
                    dn.to_string(),
                    attr_name.to_string(),
                    attr_value.to_string(),
                ),
            },
            MenuItem {
                label: "Add Value".into(),
                hint: "+".into(),
                action: Action::AddAttribute(dn.to_string(), attr_name.to_string()),
            },
            MenuItem {
                label: "Delete Value".into(),
                hint: "d".into(),
                action: Action::ShowConfirm(
                    format!("Delete value '{}' from '{}'?", attr_value, attr_name),
                    Box::new(Action::DeleteAttributeValue(
                        dn.to_string(),
                        attr_name.to_string(),
                        attr_value.to_string(),
                    )),
                ),
            },
        ];
        self.selected = 0;
        self.anchor = None;
        self.visible = true;
    }

    pub fn hide(&mut self) {
        self.visible = false;
        self.items.clear();
        self.anchor = None;
    }

    /// Set pixel anchor for positional rendering (from mouse click).
    pub fn set_anchor(&mut self, col: u16, row: u16) {
        self.anchor = Some((col, row));
    }

    /// Number of items in the current menu.
    pub fn item_count(&self) -> usize {
        self.items.len()
    }

    /// Currently selected index.
    pub fn selected(&self) -> usize {
        self.selected
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> Action {
        if !self.visible {
            return Action::None;
        }

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
                Action::None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected + 1 < self.items.len() {
                    self.selected += 1;
                }
                Action::None
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                let action = self
                    .items
                    .get(self.selected)
                    .map(|item| item.action.clone())
                    .unwrap_or(Action::None);
                self.hide();
                action
            }
            KeyCode::Esc | KeyCode::Char('q') => {
                self.hide();
                Action::ClosePopup
            }
            // First-letter jump: find first item whose label starts with pressed char
            KeyCode::Char(c) => {
                let upper = c.to_ascii_uppercase();
                if let Some(idx) = self.items.iter().position(|item| {
                    item.label
                        .chars()
                        .next()
                        .map(|ch| ch.to_ascii_uppercase()) == Some(upper)
                }) {
                    self.selected = idx;
                }
                Action::None
            }
            _ => Action::None,
        }
    }

    pub fn render(&self, frame: &mut Frame, full: Rect) {
        if !self.visible || self.items.is_empty() {
            return;
        }

        // Calculate menu dimensions
        let max_label = self.items.iter().map(|i| i.label.len()).max().unwrap_or(10);
        let max_hint = self.items.iter().map(|i| i.hint.len()).max().unwrap_or(0);
        let content_width = max_label + if max_hint > 0 { max_hint + 2 } else { 0 } + 2; // padding
        let width = (content_width.min(40) + 2) as u16; // + borders
        let height = (self.items.len() + 2) as u16; // items + borders

        // Position: near anchor if set, else center of terminal
        let (x, y) = match self.anchor {
            Some((col, row)) => {
                let x = col.min(full.width.saturating_sub(width));
                let y = (row + 1).min(full.height.saturating_sub(height));
                (x, y)
            }
            None => {
                let x = (full.width.saturating_sub(width)) / 2;
                let y = full.height / 3;
                (x, y)
            }
        };

        let area = Rect::new(x, y, width.min(full.width), height.min(full.height));
        frame.render_widget(Clear, area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(self.theme.popup_border)
            .border_type(BorderType::Rounded);

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Render items
        for (i, item) in self.items.iter().enumerate() {
            if i as u16 >= inner.height {
                break;
            }
            let row_area = Rect::new(inner.x, inner.y + i as u16, inner.width, 1);

            let is_selected = i == self.selected;
            let style = if is_selected {
                self.theme.selected.add_modifier(Modifier::BOLD)
            } else {
                self.theme.normal
            };

            if inner.width < 4 {
                continue;
            }

            let available = inner.width as usize;
            let hint_str = if item.hint.is_empty() {
                String::new()
            } else {
                format!(" {}", item.hint)
            };
            let hint_len = hint_str.len();
            let label_space = available.saturating_sub(hint_len + 1); // 1 for leading space

            let line = Line::from(vec![
                Span::styled(
                    format!(" {:<width$}", item.label, width = label_space),
                    style,
                ),
                Span::styled(
                    hint_str,
                    if is_selected {
                        style
                    } else {
                        self.theme.dimmed
                    },
                ),
            ]);
            frame.render_widget(Paragraph::new(line), row_area);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;

    fn make_menu() -> ContextMenu {
        ContextMenu::new(Theme::default())
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn test_show_for_tree_populates_items() {
        let mut menu = make_menu();
        assert!(!menu.visible);
        menu.show_for_tree("dc=example,dc=com");
        assert!(menu.visible);
        assert_eq!(menu.selected, 0);
        assert_eq!(menu.item_count(), 5);
        assert_eq!(menu.items[0].label, "Copy DN");
        assert_eq!(menu.items[1].label, "Create Child Entry");
        assert_eq!(menu.items[4].label, "Delete Entry");
    }

    #[test]
    fn test_show_for_detail_populates_items() {
        let mut menu = make_menu();
        menu.show_for_detail("dc=example,dc=com", "cn", "Test User");
        assert!(menu.visible);
        assert_eq!(menu.selected, 0);
        assert_eq!(menu.item_count(), 6);
        assert_eq!(menu.items[0].label, "Copy Attribute Name");
        assert_eq!(menu.items[1].label, "Copy Attribute Value");
        assert_eq!(menu.items[2].label, "Copy DN");
        assert_eq!(menu.items[3].label, "Edit Value");
    }

    #[test]
    fn test_hide_clears_state() {
        let mut menu = make_menu();
        menu.show_for_tree("dc=example,dc=com");
        assert!(menu.visible);
        menu.hide();
        assert!(!menu.visible);
        assert_eq!(menu.item_count(), 0);
        assert!(menu.anchor.is_none());
    }

    #[test]
    fn test_navigate_down() {
        let mut menu = make_menu();
        menu.show_for_tree("dc=example,dc=com");
        assert_eq!(menu.selected, 0);

        menu.handle_key_event(key(KeyCode::Down));
        assert_eq!(menu.selected, 1);

        menu.handle_key_event(key(KeyCode::Char('j')));
        assert_eq!(menu.selected, 2);
    }

    #[test]
    fn test_navigate_down_clamps() {
        let mut menu = make_menu();
        menu.show_for_tree("dc=example,dc=com");
        let count = menu.item_count();
        for _ in 0..count + 5 {
            menu.handle_key_event(key(KeyCode::Down));
        }
        assert_eq!(menu.selected, count - 1);
    }

    #[test]
    fn test_navigate_up() {
        let mut menu = make_menu();
        menu.show_for_tree("dc=example,dc=com");
        menu.selected = 3;

        menu.handle_key_event(key(KeyCode::Up));
        assert_eq!(menu.selected, 2);

        menu.handle_key_event(key(KeyCode::Char('k')));
        assert_eq!(menu.selected, 1);
    }

    #[test]
    fn test_navigate_up_clamps_at_zero() {
        let mut menu = make_menu();
        menu.show_for_tree("dc=example,dc=com");
        assert_eq!(menu.selected, 0);
        menu.handle_key_event(key(KeyCode::Up));
        assert_eq!(menu.selected, 0);
    }

    #[test]
    fn test_enter_returns_action_and_hides() {
        let mut menu = make_menu();
        menu.show_for_tree("dc=example,dc=com");
        // First item is "Copy DN"
        let action = menu.handle_key_event(key(KeyCode::Enter));
        assert!(matches!(action, Action::CopyToClipboard(_)));
        assert!(!menu.visible);
    }

    #[test]
    fn test_space_selects_item() {
        let mut menu = make_menu();
        menu.show_for_tree("dc=example,dc=com");
        menu.handle_key_event(key(KeyCode::Down)); // select "Create Child Entry"
        let action = menu.handle_key_event(key(KeyCode::Char(' ')));
        assert!(matches!(action, Action::ShowCreateEntryDialog(_)));
        assert!(!menu.visible);
    }

    #[test]
    fn test_esc_closes_menu() {
        let mut menu = make_menu();
        menu.show_for_tree("dc=example,dc=com");
        let action = menu.handle_key_event(key(KeyCode::Esc));
        assert!(matches!(action, Action::ClosePopup));
        assert!(!menu.visible);
    }

    #[test]
    fn test_q_closes_menu() {
        let mut menu = make_menu();
        menu.show_for_tree("dc=example,dc=com");
        let action = menu.handle_key_event(key(KeyCode::Char('q')));
        assert!(matches!(action, Action::ClosePopup));
        assert!(!menu.visible);
    }

    #[test]
    fn test_first_letter_jump() {
        let mut menu = make_menu();
        menu.show_for_tree("dc=example,dc=com");
        // 'e' should jump to "Export Subtree" (index 2)
        menu.handle_key_event(key(KeyCode::Char('e')));
        assert_eq!(menu.selected, 2);
        // 'r' should jump to "Refresh" (index 3)
        menu.handle_key_event(key(KeyCode::Char('r')));
        assert_eq!(menu.selected, 3);
        // 'd' should jump to "Delete Entry" (index 4)
        menu.handle_key_event(key(KeyCode::Char('d')));
        assert_eq!(menu.selected, 4);
    }

    #[test]
    fn test_anchor_position() {
        let mut menu = make_menu();
        menu.show_for_tree("dc=example,dc=com");
        assert!(menu.anchor.is_none());
        menu.set_anchor(10, 20);
        assert_eq!(menu.anchor, Some((10, 20)));
    }

    #[test]
    fn test_handle_key_when_not_visible() {
        let mut menu = make_menu();
        let action = menu.handle_key_event(key(KeyCode::Enter));
        assert!(matches!(action, Action::None));
    }
}
