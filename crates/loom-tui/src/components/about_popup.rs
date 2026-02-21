use crossterm::event::KeyEvent;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::action::Action;
use crate::components::popup::Popup;
use crate::theme::Theme;

/// A popup displaying project information.
pub struct AboutPopup {
    pub visible: bool,
    popup: Popup,
    theme: Theme,
}

impl AboutPopup {
    pub fn new(theme: Theme) -> Self {
        Self {
            visible: false,
            popup: Popup::new("About", theme.clone()).with_size(50, 40),
            theme,
        }
    }

    pub fn show(&mut self) {
        self.visible = true;
        self.popup.show();
    }

    pub fn hide(&mut self) {
        self.visible = false;
        self.popup.hide();
    }

    pub fn handle_key_event(&mut self, _key: KeyEvent) -> Action {
        self.hide();
        Action::ClosePopup
    }

    pub fn render(&self, frame: &mut Frame, full: Rect) {
        if !self.visible {
            return;
        }

        let area = self.popup.centered_area(full);
        frame.render_widget(Clear, area);

        let block = Block::default()
            .title(" About ")
            .borders(Borders::ALL)
            .border_style(self.theme.popup_border)
            .title_style(self.theme.popup_title);

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let version = env!("CARGO_PKG_VERSION");

        let lines = vec![
            Line::from(""),
            Line::from(Span::styled("  Loom LDAP Browser", self.theme.popup_title)),
            Line::from(Span::styled(format!("  v{}", version), self.theme.dimmed)),
            Line::from(""),
            Line::from(Span::styled(
                "  A terminal-based LDAP browser",
                self.theme.normal,
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Author:  ", self.theme.dimmed),
                Span::styled("Jared Jennings", self.theme.normal),
            ]),
            Line::from(vec![
                Span::styled("  License: ", self.theme.dimmed),
                Span::styled("GPL-3.0", self.theme.normal),
            ]),
            Line::from(vec![
                Span::styled("  Repo:    ", self.theme.dimmed),
                Span::styled("github.com/trepidity/loom-ldapbrowser", self.theme.normal),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "  Built with Rust, ratatui, and ldap3",
                self.theme.dimmed,
            )),
            Line::from(""),
            Line::from(Span::styled("  Press any key to close", self.theme.dimmed)),
        ];

        frame.render_widget(Paragraph::new(lines), inner);
    }
}
