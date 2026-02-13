use std::time::Duration;

use crossterm::event::{self, Event as CrosstermEvent, KeyEvent, KeyEventKind, MouseEvent};

/// Application events produced by the event loop.
#[derive(Debug)]
pub enum AppEvent {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Resize(u16, u16),
    Tick,
}

/// Poll for the next event with a timeout.
pub fn poll_event(tick_rate: Duration) -> Option<AppEvent> {
    if event::poll(tick_rate).ok()? {
        match event::read().ok()? {
            CrosstermEvent::Key(key) if key.kind == KeyEventKind::Press => Some(AppEvent::Key(key)),
            CrosstermEvent::Mouse(mouse) => Some(AppEvent::Mouse(mouse)),
            CrosstermEvent::Resize(w, h) => Some(AppEvent::Resize(w, h)),
            _ => None,
        }
    } else {
        Some(AppEvent::Tick)
    }
}
