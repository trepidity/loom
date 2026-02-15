use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::action::Action;
use crate::components::popup::Popup;
use crate::keymap::Keymap;
use crate::theme::Theme;

struct HelpSection {
    title: String,
    entries: Vec<(String, String)>,
}

/// A scrollable popup displaying all keyboard shortcuts.
pub struct HelpPopup {
    pub visible: bool,
    popup: Popup,
    theme: Theme,
    sections: Vec<HelpSection>,
    scroll_offset: usize,
    total_lines: usize,
}

impl HelpPopup {
    pub fn new(theme: Theme) -> Self {
        Self {
            visible: false,
            popup: Popup::new("Help", theme.clone()).with_size(60, 80),
            theme,
            sections: Vec::new(),
            scroll_offset: 0,
            total_lines: 0,
        }
    }

    pub fn show(&mut self, keymap: &Keymap) {
        self.sections = build_sections(keymap);
        self.scroll_offset = 0;
        self.total_lines = self
            .sections
            .iter()
            .map(|s| 1 + s.entries.len() + 1) // title + entries + blank line
            .sum::<usize>()
            .saturating_sub(1); // no trailing blank
        self.visible = true;
        self.popup.show();
    }

    pub fn hide(&mut self) {
        self.visible = false;
        self.popup.hide();
    }

    fn clamp_scroll(&mut self) {
        if self.scroll_offset > self.total_lines {
            self.scroll_offset = self.total_lines;
        }
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') => {
                self.hide();
                Action::ClosePopup
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.scroll_offset = self.scroll_offset.saturating_sub(1);
                Action::None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.scroll_offset += 1;
                self.clamp_scroll();
                Action::None
            }
            KeyCode::Home | KeyCode::Char('g') => {
                self.scroll_offset = 0;
                Action::None
            }
            KeyCode::End | KeyCode::Char('G') => {
                self.scroll_offset = self.total_lines;
                Action::None
            }
            KeyCode::PageUp => {
                self.scroll_offset = self.scroll_offset.saturating_sub(20);
                Action::None
            }
            KeyCode::PageDown => {
                self.scroll_offset = self.scroll_offset.saturating_add(20);
                self.clamp_scroll();
                Action::None
            }
            _ => Action::None,
        }
    }

    pub fn render(&self, frame: &mut Frame, full: Rect) {
        if !self.visible {
            return;
        }

        let area = self.popup.centered_area(full);
        frame.render_widget(Clear, area);

        let block = Block::default()
            .title(" Help ")
            .borders(Borders::ALL)
            .border_style(self.theme.popup_border)
            .title_style(self.theme.popup_title);

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let layout = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(inner);

        // Build all lines
        let mut lines: Vec<Line> = Vec::new();
        for (i, section) in self.sections.iter().enumerate() {
            // Section header
            lines.push(Line::from(Span::styled(
                section.title.clone(),
                self.theme.popup_title,
            )));
            // Entries
            for (key, desc) in &section.entries {
                let padded_key = format!("  {:<16}", key);
                lines.push(Line::from(vec![
                    Span::styled(padded_key, self.theme.header),
                    Span::styled(desc.clone(), self.theme.normal),
                ]));
            }
            // Blank line between sections (except last)
            if i + 1 < self.sections.len() {
                lines.push(Line::from(""));
            }
        }

        // Apply scroll
        let visible_height = layout[0].height as usize;
        let max_scroll = lines.len().saturating_sub(visible_height);
        let offset = self.scroll_offset.min(max_scroll);
        let end = (offset + visible_height).min(lines.len());
        let visible_lines: Vec<Line> = lines[offset..end].to_vec();

        frame.render_widget(Paragraph::new(visible_lines), layout[0]);

        // Hints
        let hints = Line::from(Span::styled(
            "\u{2191}/\u{2193}:scroll  Home/End  PgUp/PgDn  q:close",
            self.theme.dimmed,
        ));
        frame.render_widget(Paragraph::new(hints), layout[1]);
    }
}

fn build_sections(keymap: &Keymap) -> Vec<HelpSection> {
    vec![
        HelpSection {
            title: "GLOBAL SHORTCUTS (configurable)".to_string(),
            entries: vec![
                (
                    keymap.hint("switch_to_browser").to_string(),
                    "Browser layout".to_string(),
                ),
                (
                    keymap.hint("switch_to_profiles").to_string(),
                    "Profiles layout".to_string(),
                ),
                (
                    keymap.hint("show_connect_dialog").to_string(),
                    "Connect dialog".to_string(),
                ),
                (
                    keymap.hint("show_export_dialog").to_string(),
                    "Export dialog".to_string(),
                ),
                (
                    format!("{}/?", keymap.hint("show_help")),
                    "Help".to_string(),
                ),
                (
                    keymap.hint("show_schema_viewer").to_string(),
                    "Schema viewer".to_string(),
                ),
                (
                    keymap.hint("toggle_log_panel").to_string(),
                    "Log panel".to_string(),
                ),
                (
                    keymap.hint("show_bulk_update").to_string(),
                    "Bulk update".to_string(),
                ),
                (
                    keymap.hint("search").to_string(),
                    "Focus search input".to_string(),
                ),
                (
                    keymap.hint("save_connection").to_string(),
                    "Save connection".to_string(),
                ),
                (
                    keymap.hint("focus_next").to_string(),
                    "Next panel".to_string(),
                ),
                (
                    keymap.hint("focus_prev").to_string(),
                    "Previous panel".to_string(),
                ),
                (keymap.hint("quit").to_string(), "Quit".to_string()),
                (
                    keymap.hint("force_quit").to_string(),
                    "Force quit".to_string(),
                ),
            ],
        },
        HelpSection {
            title: "TREE PANEL".to_string(),
            entries: vec![
                (
                    "j/k \u{2191}/\u{2193}".to_string(),
                    "Navigate up/down".to_string(),
                ),
                (
                    "l/\u{2192}/Enter".to_string(),
                    "Expand / toggle node".to_string(),
                ),
                ("h/\u{2190}".to_string(), "Collapse node".to_string()),
                ("a".to_string(), "Create child entry".to_string()),
                ("d/Delete".to_string(), "Delete entry".to_string()),
            ],
        },
        HelpSection {
            title: "DETAIL PANEL".to_string(),
            entries: vec![
                (
                    "j/k \u{2191}/\u{2193}".to_string(),
                    "Navigate attributes".to_string(),
                ),
                ("e/Enter".to_string(), "Edit attribute value".to_string()),
                ("a".to_string(), "Add new attribute".to_string()),
                ("+".to_string(), "Add value to attribute".to_string()),
                ("d/Delete".to_string(), "Delete attribute value".to_string()),
                ("n".to_string(), "Create child entry".to_string()),
                ("x".to_string(), "Delete entry".to_string()),
                ("r".to_string(), "Refresh entry".to_string()),
            ],
        },
        HelpSection {
            title: "COMMAND / SEARCH".to_string(),
            entries: vec![
                ("/ or :".to_string(), "Activate search input".to_string()),
                ("Enter".to_string(), "Execute search filter".to_string()),
                ("Esc".to_string(), "Cancel / deactivate input".to_string()),
            ],
        },
        HelpSection {
            title: "PROFILES TREE".to_string(),
            entries: vec![
                (
                    "j/k \u{2191}/\u{2193}".to_string(),
                    "Navigate profiles".to_string(),
                ),
                ("l/\u{2192}".to_string(), "Expand folder / view".to_string()),
                ("h/\u{2190}".to_string(), "Collapse folder".to_string()),
                ("e".to_string(), "Edit / view profile".to_string()),
                ("c".to_string(), "Connect to profile".to_string()),
                ("n".to_string(), "New profile".to_string()),
                ("d/Delete".to_string(), "Delete profile".to_string()),
            ],
        },
        HelpSection {
            title: "CONNECTION FORM".to_string(),
            entries: vec![
                ("Tab/S-Tab".to_string(), "Next / previous field".to_string()),
                ("e".to_string(), "Enter edit mode (view)".to_string()),
                ("c".to_string(), "Connect (view mode)".to_string()),
                ("F2".to_string(), "Cycle TLS mode (edit)".to_string()),
                ("F3".to_string(), "Cycle credential method".to_string()),
                ("F10/C-Enter".to_string(), "Save profile (edit)".to_string()),
                ("Esc".to_string(), "Cancel editing".to_string()),
            ],
        },
        HelpSection {
            title: "SEARCH RESULTS".to_string(),
            entries: vec![
                (
                    "j/k \u{2191}/\u{2193}".to_string(),
                    "Navigate results".to_string(),
                ),
                ("Enter".to_string(), "Go to selected entry".to_string()),
                ("Esc/q".to_string(), "Close".to_string()),
            ],
        },
        HelpSection {
            title: "EXPORT DIALOG".to_string(),
            entries: vec![
                ("Tab/S-Tab".to_string(), "Next / previous field".to_string()),
                ("F2".to_string(), "Cycle export format".to_string()),
                ("Enter".to_string(), "Execute export".to_string()),
                ("Esc".to_string(), "Cancel".to_string()),
            ],
        },
        HelpSection {
            title: "BULK UPDATE DIALOG".to_string(),
            entries: vec![
                ("Tab/S-Tab".to_string(), "Next / previous field".to_string()),
                ("F2".to_string(), "Cycle operation type".to_string()),
                ("Enter".to_string(), "Execute bulk update".to_string()),
                ("Esc".to_string(), "Cancel".to_string()),
            ],
        },
        HelpSection {
            title: "CONFIRM DIALOG".to_string(),
            entries: vec![
                ("y".to_string(), "Confirm (Yes)".to_string()),
                ("n/Esc".to_string(), "Cancel (No)".to_string()),
                (
                    "h/l \u{2190}/\u{2192}".to_string(),
                    "Select Yes / No".to_string(),
                ),
                ("Enter".to_string(), "Execute selection".to_string()),
            ],
        },
        HelpSection {
            title: "SCHEMA VIEWER".to_string(),
            entries: vec![
                ("j/k \u{2191}/\u{2193}".to_string(), "Scroll".to_string()),
                ("Tab".to_string(), "Switch Attributes/Classes".to_string()),
                ("/".to_string(), "Filter".to_string()),
                ("Esc/q".to_string(), "Close".to_string()),
            ],
        },
        HelpSection {
            title: "LOG PANEL".to_string(),
            entries: vec![
                ("j/k \u{2191}/\u{2193}".to_string(), "Scroll".to_string()),
                ("g/G Home/End".to_string(), "Top / bottom".to_string()),
                ("Esc/q".to_string(), "Close".to_string()),
            ],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::KeybindingConfig;

    fn make_popup() -> HelpPopup {
        HelpPopup::new(Theme::load("dark"))
    }

    fn keymap() -> Keymap {
        Keymap::from_config(&KeybindingConfig::default())
    }

    #[test]
    fn test_show_sets_visible() {
        let mut popup = make_popup();
        assert!(!popup.visible);
        popup.show(&keymap());
        assert!(popup.visible);
        assert!(!popup.sections.is_empty());
        assert_eq!(popup.scroll_offset, 0);
    }

    #[test]
    fn test_hide_clears_visible() {
        let mut popup = make_popup();
        popup.show(&keymap());
        popup.hide();
        assert!(!popup.visible);
    }

    #[test]
    fn test_close_keys_return_close_popup() {
        let mut popup = make_popup();
        popup.show(&keymap());

        for code in [KeyCode::Esc, KeyCode::Char('q'), KeyCode::Char('?')] {
            popup.show(&keymap());
            let action = popup.handle_key_event(KeyEvent::from(code));
            assert!(matches!(action, Action::ClosePopup));
            assert!(!popup.visible);
        }
    }

    #[test]
    fn test_scroll_down_increments_offset() {
        let mut popup = make_popup();
        popup.show(&keymap());
        assert_eq!(popup.scroll_offset, 0);

        popup.handle_key_event(KeyEvent::from(KeyCode::Down));
        assert_eq!(popup.scroll_offset, 1);

        popup.handle_key_event(KeyEvent::from(KeyCode::Char('j')));
        assert_eq!(popup.scroll_offset, 2);
    }

    #[test]
    fn test_scroll_up_decrements_offset() {
        let mut popup = make_popup();
        popup.show(&keymap());
        popup.scroll_offset = 5;

        popup.handle_key_event(KeyEvent::from(KeyCode::Up));
        assert_eq!(popup.scroll_offset, 4);

        popup.handle_key_event(KeyEvent::from(KeyCode::Char('k')));
        assert_eq!(popup.scroll_offset, 3);
    }

    #[test]
    fn test_scroll_up_clamps_at_zero() {
        let mut popup = make_popup();
        popup.show(&keymap());
        popup.handle_key_event(KeyEvent::from(KeyCode::Up));
        assert_eq!(popup.scroll_offset, 0);
    }

    #[test]
    fn test_scroll_down_clamps_at_total_lines() {
        let mut popup = make_popup();
        popup.show(&keymap());
        let total = popup.total_lines;

        // Scroll way past the end
        for _ in 0..total + 50 {
            popup.handle_key_event(KeyEvent::from(KeyCode::Down));
        }
        assert_eq!(popup.scroll_offset, total);

        // Now scroll up should immediately work
        popup.handle_key_event(KeyEvent::from(KeyCode::Up));
        assert_eq!(popup.scroll_offset, total - 1);
    }

    #[test]
    fn test_home_end() {
        let mut popup = make_popup();
        popup.show(&keymap());

        popup.handle_key_event(KeyEvent::from(KeyCode::End));
        assert_eq!(popup.scroll_offset, popup.total_lines);

        popup.handle_key_event(KeyEvent::from(KeyCode::Home));
        assert_eq!(popup.scroll_offset, 0);
    }

    #[test]
    fn test_sections_populated_on_show() {
        let mut popup = make_popup();
        popup.show(&keymap());
        let titles: Vec<&str> = popup.sections.iter().map(|s| s.title.as_str()).collect();
        assert!(titles.contains(&"GLOBAL SHORTCUTS (configurable)"));
        assert!(titles.contains(&"TREE PANEL"));
        assert!(titles.contains(&"DETAIL PANEL"));
        assert!(titles.contains(&"COMMAND / SEARCH"));
        assert!(titles.contains(&"PROFILES TREE"));
        assert!(titles.contains(&"CONNECTION FORM"));
        assert!(titles.contains(&"EXPORT DIALOG"));
        assert!(titles.contains(&"SCHEMA VIEWER"));
        assert!(titles.contains(&"LOG PANEL"));
        assert!(popup.total_lines > 0);
    }

    #[test]
    fn test_global_section_has_fkeys() {
        let mut popup = make_popup();
        popup.show(&keymap());
        let global = &popup.sections[0];
        let keys: Vec<&str> = global.entries.iter().map(|(k, _)| k.as_str()).collect();
        // Check that F-key bindings are present
        assert!(keys.contains(&"F1"));
        assert!(keys.contains(&"F2"));
        assert!(keys.contains(&"F4"));
        assert!(keys.contains(&"F5/?"));
        assert!(keys.contains(&"F6"));
        assert!(keys.contains(&"F7"));
        assert!(keys.contains(&"F8"));
        assert!(keys.contains(&"F9"));
        assert!(keys.contains(&"F10"));
    }

    #[test]
    fn test_page_scroll_clamps() {
        let mut popup = make_popup();
        popup.show(&keymap());
        let total = popup.total_lines;

        // PageDown many times — should clamp at total_lines
        for _ in 0..20 {
            popup.handle_key_event(KeyEvent::from(KeyCode::PageDown));
        }
        assert_eq!(popup.scroll_offset, total);

        // PageUp should immediately work from the clamped position
        popup.handle_key_event(KeyEvent::from(KeyCode::PageUp));
        assert_eq!(popup.scroll_offset, total.saturating_sub(20));
    }
}
