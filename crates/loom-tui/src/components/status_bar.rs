use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::component::Component;
use crate::keymap::Keymap;
use crate::theme::Theme;

/// Bottom status bar showing connection info and hints.
pub struct StatusBar {
    pub connection_info: String,
    pub entry_count: Option<usize>,
    theme: Theme,
    hints: String,
}

impl StatusBar {
    pub fn new(theme: Theme, keymap: &Keymap) -> Self {
        let hints = format!(
            " | {}:quit {}:focus {}:search {}:connect {}:export {}:schema {}:logs {}:conns ",
            keymap.hint("quit"),
            keymap.hint("focus_next"),
            keymap.hint("search"),
            keymap.hint("show_connect_dialog"),
            keymap.hint("show_export_dialog"),
            keymap.hint("show_schema_viewer"),
            keymap.hint("toggle_log_panel"),
            keymap.hint("toggle_layout"),
        );
        Self {
            connection_info: "Not connected".to_string(),
            entry_count: None,
            theme,
            hints,
        }
    }

    pub fn set_connected(&mut self, host: &str, server_type: &str) {
        self.connection_info = format!("Connected: {} | {}", host, server_type);
    }

    pub fn set_disconnected(&mut self) {
        self.connection_info = "Not connected".to_string();
        self.entry_count = None;
    }
}

impl Component for StatusBar {
    fn render(&self, frame: &mut Frame, area: Rect, _focused: bool) {
        let mut spans = vec![
            Span::styled(" ", self.theme.status_bar),
            Span::styled(&self.connection_info, self.theme.status_bar),
        ];

        if let Some(count) = self.entry_count {
            spans.push(Span::styled(
                format!(" | {} entries", count),
                self.theme.status_bar,
            ));
        }

        spans.push(Span::styled(&self.hints, self.theme.status_bar));

        // Pad to fill width
        let content_len: usize = spans.iter().map(|s| s.content.len()).sum();
        let padding = " ".repeat(area.width as usize - content_len.min(area.width as usize));
        spans.push(Span::styled(padding, self.theme.status_bar));

        let line = Line::from(spans);
        let bar = Paragraph::new(line);
        frame.render_widget(bar, area);
    }
}
