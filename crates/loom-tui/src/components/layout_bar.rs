use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::action::ActiveLayout;
use crate::theme::Theme;

/// Top-level layout bar: [Browser]  [Connections]
pub struct LayoutBar {
    pub active: ActiveLayout,
    theme: Theme,
}

impl LayoutBar {
    pub fn new(theme: Theme) -> Self {
        Self {
            active: ActiveLayout::Browser,
            theme,
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let browser_style = if self.active == ActiveLayout::Browser {
            self.theme.tab_active
        } else {
            self.theme.tab_inactive
        };
        let conns_style = if self.active == ActiveLayout::Connections {
            self.theme.tab_active
        } else {
            self.theme.tab_inactive
        };

        let mut spans = vec![Span::styled(" ", self.theme.status_bar)];

        if self.active == ActiveLayout::Browser {
            spans.push(Span::styled("[Browser]", browser_style));
        } else {
            spans.push(Span::styled(" Browser ", browser_style));
        }

        spans.push(Span::styled("  ", self.theme.status_bar));

        if self.active == ActiveLayout::Connections {
            spans.push(Span::styled("[Connections]", conns_style));
        } else {
            spans.push(Span::styled(" Connections ", conns_style));
        }

        // Pad remaining width
        let content_len: usize = spans.iter().map(|s| s.content.len()).sum();
        let padding = " ".repeat(area.width as usize - content_len.min(area.width as usize));
        spans.push(Span::styled(padding, self.theme.status_bar));

        let line = Line::from(spans);
        let bar = Paragraph::new(line);
        frame.render_widget(bar, area);
    }
}
