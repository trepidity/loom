use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tracing::warn;

use crate::action::{Action, FocusTarget};
use crate::config::KeybindingConfig;

/// Parse a key string like "Alt+t", "Ctrl+Shift+x", "q", "F2", "Tab" into (modifiers, code).
pub fn parse_key(s: &str) -> Result<(KeyModifiers, KeyCode), String> {
    let parts: Vec<&str> = s.split('+').collect();
    if parts.is_empty() || s.is_empty() {
        return Err("empty key string".to_string());
    }

    let key_name = parts.last().unwrap().trim();
    let modifier_parts = &parts[..parts.len() - 1];

    let mut modifiers = KeyModifiers::NONE;
    for part in modifier_parts {
        match part.trim().to_lowercase().as_str() {
            "alt" => modifiers |= KeyModifiers::ALT,
            "ctrl" | "control" => modifiers |= KeyModifiers::CONTROL,
            "shift" => modifiers |= KeyModifiers::SHIFT,
            other => return Err(format!("unknown modifier: {}", other)),
        }
    }

    let code = match key_name.to_lowercase().as_str() {
        "tab" => {
            if modifiers.contains(KeyModifiers::SHIFT) {
                modifiers -= KeyModifiers::SHIFT;
                modifiers |= KeyModifiers::SHIFT;
                KeyCode::BackTab
            } else {
                KeyCode::Tab
            }
        }
        "enter" | "return" => KeyCode::Enter,
        "esc" | "escape" => KeyCode::Esc,
        "space" => KeyCode::Char(' '),
        "backspace" => KeyCode::Backspace,
        "delete" | "del" => KeyCode::Delete,
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "home" => KeyCode::Home,
        "end" => KeyCode::End,
        "pageup" => KeyCode::PageUp,
        "pagedown" => KeyCode::PageDown,
        s if s.starts_with('f') && s.len() > 1 => {
            if let Ok(n) = s[1..].parse::<u8>() {
                KeyCode::F(n)
            } else {
                // single char 'f'
                KeyCode::Char('f')
            }
        }
        s if s.chars().count() == 1 => KeyCode::Char(s.chars().next().unwrap()),
        other => return Err(format!("unknown key: {}", other)),
    };

    Ok((modifiers, code))
}

/// Format a key binding for compact display in the status bar.
/// e.g. (ALT, Char('t')) -> "A-t", (NONE, Char('q')) -> "q"
pub fn display_key(modifiers: KeyModifiers, code: KeyCode) -> String {
    let mut prefix = String::new();
    if modifiers.contains(KeyModifiers::CONTROL) {
        prefix.push_str("C-");
    }
    if modifiers.contains(KeyModifiers::ALT) {
        prefix.push_str("A-");
    }
    if modifiers.contains(KeyModifiers::SHIFT) {
        prefix.push_str("S-");
    }

    let key_part = match code {
        KeyCode::Char(c) => c.to_string(),
        KeyCode::Tab => "Tab".to_string(),
        KeyCode::BackTab => {
            // BackTab already implies Shift, so avoid double "S-"
            if prefix.contains("S-") {
                prefix = prefix.replace("S-", "");
            }
            "S-Tab".to_string()
        }
        KeyCode::Enter => "Enter".to_string(),
        KeyCode::Esc => "Esc".to_string(),
        KeyCode::Backspace => "Bksp".to_string(),
        KeyCode::Delete => "Del".to_string(),
        KeyCode::Up => "Up".to_string(),
        KeyCode::Down => "Down".to_string(),
        KeyCode::Left => "Left".to_string(),
        KeyCode::Right => "Right".to_string(),
        KeyCode::Home => "Home".to_string(),
        KeyCode::End => "End".to_string(),
        KeyCode::PageUp => "PgUp".to_string(),
        KeyCode::PageDown => "PgDn".to_string(),
        KeyCode::F(n) => format!("F{}", n),
        _ => "?".to_string(),
    };

    format!("{}{}", prefix, key_part)
}

/// Maps configured key bindings to actions and provides display hints.
pub struct Keymap {
    global: HashMap<(KeyModifiers, KeyCode), Action>,
    hints: HashMap<&'static str, String>,
}

impl Keymap {
    /// Build a Keymap from user configuration.
    /// On parse errors, falls back to the default binding for that action.
    pub fn from_config(config: &KeybindingConfig) -> Self {
        let defaults = KeybindingConfig::default();
        let mut global = HashMap::new();
        let mut hints = HashMap::new();

        let bindings: Vec<(&str, &str, &str, Action)> = vec![
            ("quit", &config.quit, &defaults.quit, Action::Quit),
            (
                "force_quit",
                &config.force_quit,
                &defaults.force_quit,
                Action::Quit,
            ),
            (
                "focus_next",
                &config.focus_next,
                &defaults.focus_next,
                Action::FocusNext,
            ),
            (
                "focus_prev",
                &config.focus_prev,
                &defaults.focus_prev,
                Action::FocusPrev,
            ),
            (
                "show_connect_dialog",
                &config.show_connect_dialog,
                &defaults.show_connect_dialog,
                Action::ShowConnectDialog,
            ),
            (
                "next_tab",
                &config.next_tab,
                &defaults.next_tab,
                Action::NextTab,
            ),
            (
                "prev_tab",
                &config.prev_tab,
                &defaults.prev_tab,
                Action::PrevTab,
            ),
            (
                "search",
                &config.search,
                &defaults.search,
                Action::SearchFocusInput,
            ),
            (
                "show_export_dialog",
                &config.show_export_dialog,
                &defaults.show_export_dialog,
                Action::ShowExportDialog,
            ),
            (
                "show_bulk_update",
                &config.show_bulk_update,
                &defaults.show_bulk_update,
                Action::ShowBulkUpdateDialog,
            ),
            (
                "show_schema_viewer",
                &config.show_schema_viewer,
                &defaults.show_schema_viewer,
                Action::ShowSchemaViewer,
            ),
            (
                "toggle_log_panel",
                &config.toggle_log_panel,
                &defaults.toggle_log_panel,
                Action::ToggleLogPanel,
            ),
            (
                "save_connection",
                &config.save_connection,
                &defaults.save_connection,
                Action::SaveCurrentConnection,
            ),
            (
                "toggle_layout",
                &config.toggle_layout,
                &defaults.toggle_layout,
                Action::ToggleLayout,
            ),
        ];

        for (name, user_str, default_str, action) in bindings {
            let (mods, code) = match parse_key(user_str) {
                Ok(parsed) => parsed,
                Err(e) => {
                    warn!(
                        "Invalid keybinding for '{}': '{}' ({}), using default '{}'",
                        name, user_str, e, default_str
                    );
                    parse_key(default_str).expect("default keybinding must parse")
                }
            };
            global.insert((mods, code), action);
            hints.insert(name, display_key(mods, code));
        }

        Self { global, hints }
    }

    /// Resolve a key event to an action based on global bindings and context.
    pub fn resolve(&self, key: KeyEvent, focus: FocusTarget) -> Action {
        // Check configured global bindings
        if let Some(action) = self.global.get(&(key.modifiers, key.code)) {
            return action.clone();
        }

        // Context-specific hardcoded bindings (vim nav, panel-internal)
        match focus {
            FocusTarget::TreePanel => resolve_tree(key),
            FocusTarget::DetailPanel => resolve_detail(key),
            FocusTarget::CommandPanel => resolve_command(key),
            FocusTarget::ConnectionsTree | FocusTarget::ConnectionForm => Action::None,
        }
    }

    /// Get the display string for a named action (for status bar hints).
    pub fn hint(&self, action: &str) -> &str {
        self.hints
            .get(action)
            .map(|s| s.as_str())
            .unwrap_or("???")
    }
}

impl Default for Keymap {
    fn default() -> Self {
        Self::from_config(&KeybindingConfig::default())
    }
}

fn resolve_tree(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => Action::TreeUp,
        KeyCode::Down | KeyCode::Char('j') => Action::TreeDown,
        KeyCode::Right | KeyCode::Char('l') | KeyCode::Enter => Action::TreeToggle,
        KeyCode::Left | KeyCode::Char('h') => Action::TreeCollapse(String::new()),
        _ => Action::None,
    }
}

fn resolve_detail(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Char('r') => Action::EntryRefresh,
        _ => Action::None,
    }
}

fn resolve_command(_key: KeyEvent) -> Action {
    Action::None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn ctrl(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::CONTROL)
    }

    // --- parse_key tests ---

    #[test]
    fn test_parse_key_simple_char() {
        let (mods, code) = parse_key("q").unwrap();
        assert_eq!(mods, KeyModifiers::NONE);
        assert!(matches!(code, KeyCode::Char('q')));
    }

    #[test]
    fn test_parse_key_slash() {
        let (mods, code) = parse_key("/").unwrap();
        assert_eq!(mods, KeyModifiers::NONE);
        assert!(matches!(code, KeyCode::Char('/')));
    }

    #[test]
    fn test_parse_key_alt_char() {
        let (mods, code) = parse_key("Alt+t").unwrap();
        assert_eq!(mods, KeyModifiers::ALT);
        assert!(matches!(code, KeyCode::Char('t')));
    }

    #[test]
    fn test_parse_key_ctrl_char() {
        let (mods, code) = parse_key("Ctrl+c").unwrap();
        assert_eq!(mods, KeyModifiers::CONTROL);
        assert!(matches!(code, KeyCode::Char('c')));
    }

    #[test]
    fn test_parse_key_shift_tab() {
        let (mods, code) = parse_key("Shift+Tab").unwrap();
        assert_eq!(mods, KeyModifiers::SHIFT);
        assert!(matches!(code, KeyCode::BackTab));
    }

    #[test]
    fn test_parse_key_tab() {
        let (mods, code) = parse_key("Tab").unwrap();
        assert_eq!(mods, KeyModifiers::NONE);
        assert!(matches!(code, KeyCode::Tab));
    }

    #[test]
    fn test_parse_key_f2() {
        let (mods, code) = parse_key("F2").unwrap();
        assert_eq!(mods, KeyModifiers::NONE);
        assert!(matches!(code, KeyCode::F(2)));
    }

    #[test]
    fn test_parse_key_enter() {
        let (mods, code) = parse_key("Enter").unwrap();
        assert_eq!(mods, KeyModifiers::NONE);
        assert!(matches!(code, KeyCode::Enter));
    }

    #[test]
    fn test_parse_key_case_insensitive_modifier() {
        let (mods, _code) = parse_key("alt+x").unwrap();
        assert_eq!(mods, KeyModifiers::ALT);

        let (mods, _code) = parse_key("CTRL+x").unwrap();
        assert_eq!(mods, KeyModifiers::CONTROL);
    }

    #[test]
    fn test_parse_key_ctrl_shift_combo() {
        let (mods, code) = parse_key("Ctrl+Shift+x").unwrap();
        assert_eq!(mods, KeyModifiers::CONTROL | KeyModifiers::SHIFT);
        assert!(matches!(code, KeyCode::Char('x')));
    }

    #[test]
    fn test_parse_key_arrows() {
        assert!(matches!(parse_key("Up").unwrap().1, KeyCode::Up));
        assert!(matches!(parse_key("Down").unwrap().1, KeyCode::Down));
        assert!(matches!(parse_key("Left").unwrap().1, KeyCode::Left));
        assert!(matches!(parse_key("Right").unwrap().1, KeyCode::Right));
    }

    #[test]
    fn test_parse_key_esc() {
        let (mods, code) = parse_key("Esc").unwrap();
        assert_eq!(mods, KeyModifiers::NONE);
        assert!(matches!(code, KeyCode::Esc));
    }

    #[test]
    fn test_parse_key_delete() {
        let (mods, code) = parse_key("Delete").unwrap();
        assert_eq!(mods, KeyModifiers::NONE);
        assert!(matches!(code, KeyCode::Delete));
    }

    #[test]
    fn test_parse_key_error_empty() {
        assert!(parse_key("").is_err());
    }

    #[test]
    fn test_parse_key_error_bad_modifier() {
        assert!(parse_key("Meta+x").is_err());
    }

    // --- display_key tests ---

    #[test]
    fn test_display_key_plain_char() {
        assert_eq!(display_key(KeyModifiers::NONE, KeyCode::Char('q')), "q");
    }

    #[test]
    fn test_display_key_alt() {
        assert_eq!(display_key(KeyModifiers::ALT, KeyCode::Char('t')), "A-t");
    }

    #[test]
    fn test_display_key_ctrl() {
        assert_eq!(
            display_key(KeyModifiers::CONTROL, KeyCode::Char('c')),
            "C-c"
        );
    }

    #[test]
    fn test_display_key_shift_backtab() {
        assert_eq!(
            display_key(KeyModifiers::SHIFT, KeyCode::BackTab),
            "S-Tab"
        );
    }

    #[test]
    fn test_display_key_tab() {
        assert_eq!(display_key(KeyModifiers::NONE, KeyCode::Tab), "Tab");
    }

    #[test]
    fn test_display_key_f_key() {
        assert_eq!(display_key(KeyModifiers::NONE, KeyCode::F(2)), "F2");
    }

    #[test]
    fn test_display_key_enter() {
        assert_eq!(display_key(KeyModifiers::NONE, KeyCode::Enter), "Enter");
    }

    // --- Keymap tests ---

    #[test]
    fn test_default_quit() {
        let km = Keymap::default();
        let action = km.resolve(key(KeyCode::Char('q')), FocusTarget::TreePanel);
        assert!(matches!(action, Action::Quit));
    }

    #[test]
    fn test_default_ctrl_c_quit() {
        let km = Keymap::default();
        let action = km.resolve(ctrl(KeyCode::Char('c')), FocusTarget::TreePanel);
        assert!(matches!(action, Action::Quit));
    }

    #[test]
    fn test_default_tab_focus() {
        let km = Keymap::default();
        let action = km.resolve(key(KeyCode::Tab), FocusTarget::TreePanel);
        assert!(matches!(action, Action::FocusNext));
    }

    #[test]
    fn test_default_ctrl_t_connect() {
        let km = Keymap::default();
        let action = km.resolve(ctrl(KeyCode::Char('t')), FocusTarget::TreePanel);
        assert!(matches!(action, Action::ShowConnectDialog));
    }

    #[test]
    fn test_default_ctrl_e_export() {
        let km = Keymap::default();
        let action = km.resolve(ctrl(KeyCode::Char('e')), FocusTarget::TreePanel);
        assert!(matches!(action, Action::ShowExportDialog));
    }

    #[test]
    fn test_default_ctrl_s_schema() {
        let km = Keymap::default();
        let action = km.resolve(ctrl(KeyCode::Char('s')), FocusTarget::TreePanel);
        assert!(matches!(action, Action::ShowSchemaViewer));
    }

    #[test]
    fn test_default_ctrl_w_save_connection() {
        let km = Keymap::default();
        let action = km.resolve(ctrl(KeyCode::Char('w')), FocusTarget::TreePanel);
        assert!(matches!(action, Action::SaveCurrentConnection));
    }

    #[test]
    fn test_default_search_slash() {
        let km = Keymap::default();
        let action = km.resolve(key(KeyCode::Char('/')), FocusTarget::DetailPanel);
        assert!(matches!(action, Action::SearchFocusInput));
    }

    #[test]
    fn test_tree_navigation() {
        let km = Keymap::default();
        let action = km.resolve(key(KeyCode::Char('j')), FocusTarget::TreePanel);
        assert!(matches!(action, Action::TreeDown));

        let action = km.resolve(key(KeyCode::Char('k')), FocusTarget::TreePanel);
        assert!(matches!(action, Action::TreeUp));
    }

    #[test]
    fn test_tree_panel_a_no_action() {
        let km = Keymap::default();
        let action = km.resolve(key(KeyCode::Char('a')), FocusTarget::TreePanel);
        assert!(matches!(action, Action::None));
    }

    #[test]
    fn test_tree_panel_d_no_action() {
        let km = Keymap::default();
        let action = km.resolve(key(KeyCode::Char('d')), FocusTarget::TreePanel);
        assert!(matches!(action, Action::None));
    }

    #[test]
    fn test_custom_keybinding() {
        let mut config = KeybindingConfig::default();
        config.quit = "Alt+q".to_string();
        config.show_connect_dialog = "F1".to_string();

        let km = Keymap::from_config(&config);

        // Custom quit
        let action = km.resolve(
            KeyEvent::new(KeyCode::Char('q'), KeyModifiers::ALT),
            FocusTarget::TreePanel,
        );
        assert!(matches!(action, Action::Quit));

        // Custom connect dialog
        let action = km.resolve(key(KeyCode::F(1)), FocusTarget::TreePanel);
        assert!(matches!(action, Action::ShowConnectDialog));

        // Old 'q' no longer quits (overridden)
        let action = km.resolve(key(KeyCode::Char('q')), FocusTarget::TreePanel);
        assert!(matches!(action, Action::None));
    }

    #[test]
    fn test_invalid_key_string_falls_back() {
        let mut config = KeybindingConfig::default();
        config.quit = "BADKEY!!!".to_string();

        let km = Keymap::from_config(&config);

        // Should fall back to default "q"
        let action = km.resolve(key(KeyCode::Char('q')), FocusTarget::TreePanel);
        assert!(matches!(action, Action::Quit));
    }

    #[test]
    fn test_ctrl_defaults() {
        let km = Keymap::default();

        assert!(matches!(
            km.resolve(ctrl(KeyCode::Char('n')), FocusTarget::TreePanel),
            Action::NextTab
        ));
        assert!(matches!(
            km.resolve(ctrl(KeyCode::Char('p')), FocusTarget::TreePanel),
            Action::PrevTab
        ));
        assert!(matches!(
            km.resolve(ctrl(KeyCode::Char('b')), FocusTarget::TreePanel),
            Action::ShowBulkUpdateDialog
        ));
        assert!(matches!(
            km.resolve(ctrl(KeyCode::Char('l')), FocusTarget::TreePanel),
            Action::ToggleLogPanel
        ));
        assert!(matches!(
            km.resolve(ctrl(KeyCode::Char('g')), FocusTarget::TreePanel),
            Action::ToggleLayout
        ));
    }

    #[test]
    fn test_hint_returns_display_string() {
        let km = Keymap::default();
        assert_eq!(km.hint("quit"), "q");
        assert_eq!(km.hint("show_connect_dialog"), "C-t");
        assert_eq!(km.hint("toggle_layout"), "C-g");
        assert_eq!(km.hint("search"), "/");
        assert_eq!(km.hint("focus_next"), "Tab");
    }

    #[test]
    fn test_hint_unknown_action() {
        let km = Keymap::default();
        assert_eq!(km.hint("nonexistent"), "???");
    }
}
