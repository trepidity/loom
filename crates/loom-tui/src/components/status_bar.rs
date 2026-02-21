use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::component::Component;
use crate::keymap::Keymap;
use crate::theme::Theme;

/// Bottom status bar showing connection info (left) and keybinding hints (right).
pub struct StatusBar {
    pub connection_info: String,
    pub entry_count: Option<usize>,
    theme: Theme,
    hints: String,
}

impl StatusBar {
    pub fn new(theme: Theme, keymap: &Keymap) -> Self {
        let hints = format!(
            "{}:browser {}:profiles {}:help F3:about {}:quit",
            keymap.hint("switch_to_browser"),
            keymap.hint("switch_to_profiles"),
            keymap.hint("show_help"),
            keymap.hint("quit"),
        );
        Self {
            connection_info: String::new(),
            entry_count: None,
            theme,
            hints,
        }
    }

    pub fn set_connected(&mut self, host: &str, server_type: &str) {
        self.connection_info = format!("{} ({})", host, server_type);
    }

    pub fn set_disconnected(&mut self) {
        self.connection_info = String::new();
        self.entry_count = None;
    }
}

impl Component for StatusBar {
    fn render(&self, frame: &mut Frame, area: Rect, _focused: bool) {
        let width = area.width as usize;

        // Build left side: connection info + entry count
        let left = if self.connection_info.is_empty() {
            String::new()
        } else {
            let mut s = format!(" {}", self.connection_info);
            if let Some(count) = self.entry_count {
                s.push_str(&format!(" | {} entries", count));
            }
            s
        };

        // Right side: keybinding hints (with trailing space)
        let right = format!("{} ", self.hints);

        let left_len = left.len();
        let right_len = right.len();
        let gap = width.saturating_sub(left_len + right_len);
        let padding = " ".repeat(gap);

        let line = Line::from(vec![
            Span::styled(left, self.theme.status_bar),
            Span::styled(padding, self.theme.status_bar),
            Span::styled(right, self.theme.status_bar),
        ]);
        let bar = Paragraph::new(line);
        frame.render_widget(bar, area);
    }
}
