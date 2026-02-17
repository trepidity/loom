# Context Menu Implementation Plan

## Overview

Add a context-sensitive options menu to Loom that appears when the user presses
**Space** (keyboard) or **right-clicks** (mouse) on a tree node or detail panel
attribute. The menu displays a list of actions relevant to the current selection,
with keyboard shortcut hints for discoverability.

---

## 1. Keyboard Shortcut: Space

### Rationale

| Candidate | Verdict | Notes |
|-----------|---------|-------|
| **Space** | **Winner** | Largest key, Helix editor precedent ("space mode"), free in both panels, instantly discoverable |
| Shift+F10 | Rejected | Windows-standard context menu key, but obscure and awkward in a terminal |
| F3 | Rejected | Available but no mnemonic connection to "context menu" |
| `o` / `m` | Rejected | Mnemonic but not an industry convention |
| Right-click only | Partial | Good as a secondary trigger, but keyboard-only users need a path |

**Space** is free in tree panel and detail panel. It does not conflict with any
global keybinding, any popup, or the command panel (which requires `/` or `:`
to activate input mode first).

**Right-click** (`MouseButton::Right`, `MouseEventKind::Down`) will serve as
the mouse trigger. It is currently unhandled in `handle_mouse()`.

### Configurable Keybinding

Add `context_menu` to `KeybindingConfig` with default `"Space"`, allowing users
to rebind it in `config.toml`:

```toml
[keybindings]
context_menu = "Space"   # default
# context_menu = "F3"    # alternative
```

---

## 2. New Action Variants

Add to `Action` enum in `action.rs`:

```rust
// Context Menu
ShowContextMenu(ContextMenuSource),
CopyToClipboard(String),
```

New enum in `action.rs`:

```rust
/// Where the context menu was invoked from, carrying the relevant state.
#[derive(Debug, Clone)]
pub enum ContextMenuSource {
    Tree {
        dn: String,
    },
    Detail {
        dn: String,
        attr_name: String,
        attr_value: String,
    },
}
```

---

## 3. New Component: `ContextMenu`

### File: `crates/loom-tui/src/components/context_menu.rs`

```rust
pub struct MenuItem {
    pub label: String,
    pub hint: String,     // shortcut hint shown right-aligned, e.g. "a"
    pub action: Action,
}

pub struct ContextMenu {
    pub visible: bool,
    items: Vec<MenuItem>,
    selected: usize,
    source: Option<ContextMenuSource>,
    anchor: Option<(u16, u16)>,  // optional (col, row) for positional rendering
    theme: Theme,
}
```

### Construction & Show Logic

```rust
impl ContextMenu {
    pub fn new(theme: Theme) -> Self { ... }

    /// Show the menu for a tree node.
    pub fn show_for_tree(&mut self, dn: &str) {
        self.items = vec![
            MenuItem { label: "Copy DN".into(),           hint: "".into(),  action: Action::CopyToClipboard(dn.to_string()) },
            MenuItem { label: "Create Child Entry".into(), hint: "a".into(), action: Action::ShowCreateEntryDialog(dn.to_string()) },
            MenuItem { label: "Export Subtree".into(),     hint: "F4".into(), action: Action::ShowExportDialog },
            MenuItem { label: "Refresh".into(),            hint: "r".into(), action: Action::EntryRefresh },
            MenuItem { label: "Delete Entry".into(),       hint: "d".into(), action: Action::ShowConfirm(
                format!("Delete entry?\n{}", dn),
                Box::new(Action::DeleteEntry(dn.to_string())),
            )},
        ];
        self.selected = 0;
        self.source = Some(ContextMenuSource::Tree { dn: dn.to_string() });
        self.visible = true;
    }

    /// Show the menu for a detail panel attribute.
    pub fn show_for_detail(&mut self, dn: &str, attr_name: &str, attr_value: &str) {
        self.items = vec![
            MenuItem { label: "Copy Attribute Name".into(),  hint: "".into(),  action: Action::CopyToClipboard(attr_name.to_string()) },
            MenuItem { label: "Copy Attribute Value".into(), hint: "".into(),  action: Action::CopyToClipboard(attr_value.to_string()) },
            MenuItem { label: "Copy DN".into(),              hint: "".into(),  action: Action::CopyToClipboard(dn.to_string()) },
            MenuItem { label: "Edit Value".into(),           hint: "e".into(), action: Action::EditAttribute(dn.to_string(), attr_name.to_string(), attr_value.to_string()) },
            MenuItem { label: "Add Value".into(),            hint: "+".into(), action: Action::AddAttribute(dn.to_string(), attr_name.to_string()) },
            MenuItem { label: "Delete Value".into(),         hint: "d".into(), action: Action::ShowConfirm(
                format!("Delete value '{}' from '{}'?", attr_value, attr_name),
                Box::new(Action::DeleteAttributeValue(dn.to_string(), attr_name.to_string(), attr_value.to_string())),
            )},
        ];
        self.selected = 0;
        self.source = Some(ContextMenuSource::Detail {
            dn: dn.to_string(),
            attr_name: attr_name.to_string(),
            attr_value: attr_value.to_string(),
        });
        self.visible = true;
    }

    pub fn hide(&mut self) {
        self.visible = false;
        self.items.clear();
        self.source = None;
        self.anchor = None;
    }

    /// Set pixel anchor for positional rendering (from mouse click).
    pub fn set_anchor(&mut self, col: u16, row: u16) {
        self.anchor = Some((col, row));
    }
}
```

### Key Event Handling

```rust
pub fn handle_key_event(&mut self, key: KeyEvent) -> Action {
    if !self.visible { return Action::None; }

    match key.code {
        KeyCode::Up | KeyCode::Char('k') => {
            if self.selected > 0 { self.selected -= 1; }
            Action::None
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if self.selected + 1 < self.items.len() { self.selected += 1; }
            Action::None
        }
        KeyCode::Enter | KeyCode::Char(' ') => {
            let action = self.items.get(self.selected)
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
                item.label.chars().next().map(|ch| ch.to_ascii_uppercase()) == Some(upper)
            }) {
                self.selected = idx;
            }
            Action::None
        }
        _ => Action::None,
    }
}
```

### Rendering

The context menu renders as a compact, borderless (or thin-bordered) popup
positioned near the selection, **not** centered like other dialogs. This gives
the visual feel of a right-click menu.

```rust
pub fn render(&self, frame: &mut Frame, full: Rect) {
    if !self.visible || self.items.is_empty() { return; }

    // Calculate menu dimensions
    let max_label = self.items.iter().map(|i| i.label.len()).max().unwrap_or(10);
    let max_hint = self.items.iter().map(|i| i.hint.len()).max().unwrap_or(0);
    let width = (max_label + max_hint + 6).min(40) as u16;  // padding + separator
    let height = (self.items.len() + 2) as u16;               // items + border

    // Position: near anchor if set, else near center of terminal
    let (x, y) = match self.anchor {
        Some((col, row)) => {
            let x = col.min(full.width.saturating_sub(width));
            let y = (row + 1).min(full.height.saturating_sub(height));
            (x, y)
        }
        None => {
            // Center horizontally, place in upper third
            let x = (full.width.saturating_sub(width)) / 2;
            let y = full.height / 3;
            (x, y)
        }
    };

    let area = Rect::new(x, y, width, height);
    frame.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(self.theme.popup_border)
        .border_type(BorderType::Rounded);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Render items
    for (i, item) in self.items.iter().enumerate() {
        if i as u16 >= inner.height { break; }
        let row_area = Rect::new(inner.x, inner.y + i as u16, inner.width, 1);

        let style = if i == self.selected {
            self.theme.selected
        } else {
            self.theme.normal
        };

        let hint_width = item.hint.len() as u16;
        let label_width = inner.width.saturating_sub(hint_width + 1);

        let line = Line::from(vec![
            Span::styled(
                format!(" {:<width$}", item.label, width = label_width as usize - 1),
                style,
            ),
            Span::styled(
                format!("{} ", item.hint),
                if i == self.selected { style } else { self.theme.dimmed },
            ),
        ]);
        frame.render_widget(Paragraph::new(line), row_area);
    }
}
```

---

## 4. Clipboard Support

### Dependency: `arboard`

Add to workspace `Cargo.toml`:

```toml
arboard = "3"
```

Add to `crates/loom-tui/Cargo.toml`:

```toml
arboard = { workspace = true }
```

### Handler in `process_action`

```rust
Action::CopyToClipboard(text) => {
    match arboard::Clipboard::new() {
        Ok(mut clipboard) => {
            match clipboard.set_text(&text) {
                Ok(_) => {
                    let preview = if text.len() > 40 {
                        format!("{}...", &text[..40])
                    } else {
                        text.clone()
                    };
                    let _ = self.action_tx.send(Action::StatusMessage(
                        format!("Copied: {}", preview)
                    ));
                }
                Err(e) => {
                    let _ = self.action_tx.send(Action::ErrorMessage(
                        format!("Clipboard error: {}", e)
                    ));
                }
            }
        }
        Err(e) => {
            let _ = self.action_tx.send(Action::ErrorMessage(
                format!("Clipboard unavailable: {}", e)
            ));
        }
    }
}
```

> **Note**: `arboard` works on macOS (pasteboard), Linux (X11/Wayland), and
> Windows natively. No need for shelling out to `pbcopy`/`xclip`.

---

## 5. Integration into `app.rs`

### 5a. Add Field to `App`

```rust
// In App struct, alongside other popup/dialog fields:
context_menu: ContextMenu,
```

Initialize in `App::new()`:

```rust
context_menu: ContextMenu::new(theme.clone()),
```

### 5b. Key Event Dispatch

Insert context menu as the **first** popup check (highest priority), since it's
the most lightweight popup and should intercept before all others:

```rust
// In the key event match, before other popup checks:
let action = if self.context_menu.visible {
    self.context_menu.handle_key_event(key)
} else if self.attribute_editor.visible {
    // ... existing popup chain ...
```

### 5c. Space Key Trigger

In the browser layout panel dispatch (lines ~967-982 of `app.rs`), the Space
key will be handled in two places:

**Option A (Recommended)**: Handle Space inside `TreePanel::handle_key_event` and
`DetailPanel::handle_key_event` directly, returning a new `ShowContextMenu` action:

In `tree_panel.rs`, add a match arm:

```rust
KeyCode::Char(' ') => {
    if let Some(dn) = self.selected_dn().cloned() {
        Action::ShowContextMenu(ContextMenuSource::Tree { dn })
    } else {
        Action::None
    }
}
```

In `detail_panel.rs`, add a match arm:

```rust
KeyCode::Char(' ') => {
    if let (Some(entry), Some((attr, val))) = (&self.entry, self.selected_attr_value()) {
        Action::ShowContextMenu(ContextMenuSource::Detail {
            dn: entry.dn.clone(),
            attr_name: attr.to_string(),
            attr_value: val.to_string(),
        })
    } else {
        Action::None
    }
}
```

**Option B (Alternative)**: Handle via the global keymap system. Add `context_menu`
to `KeybindingConfig` and resolve it in `Keymap::resolve()` with context awareness.
This is more complex because the keymap doesn't have access to the currently
selected DN/attribute. Prefer Option A.

### 5d. Right-Click Mouse Trigger

In `handle_mouse()`, add a case for `MouseButton::Right`:

```rust
MouseEventKind::Down(crossterm::event::MouseButton::Right) => {
    // Block if popup is active (except context menu itself)
    if self.popup_active() && !self.context_menu.visible {
        return Action::None;
    }

    let pos = Rect::new(mouse.column, mouse.row, 1, 1);

    if self.active_layout == ActiveLayout::Browser {
        // Right-click on tree panel
        if let Some(tree) = self.tree_area {
            if tree.intersects(pos) {
                if let Some(dn) = self.tree_panel.selected_dn().cloned() {
                    self.context_menu.show_for_tree(&dn);
                    self.context_menu.set_anchor(mouse.column, mouse.row);
                    return Action::Render;
                }
            }
        }
        // Right-click on detail panel
        if let Some(detail) = self.detail_area {
            if detail.intersects(pos) {
                if let (Some(entry), Some((attr, val))) =
                    (&self.detail_panel.entry, self.detail_panel.selected_attr_value())
                {
                    self.context_menu.show_for_detail(&entry.dn, attr, val);
                    self.context_menu.set_anchor(mouse.column, mouse.row);
                    return Action::Render;
                }
            }
        }
    }
    Action::None
}
```

### 5e. Process Action Handler

In `process_action()`:

```rust
Action::ShowContextMenu(source) => {
    match &source {
        ContextMenuSource::Tree { dn } => {
            self.context_menu.show_for_tree(dn);
        }
        ContextMenuSource::Detail { dn, attr_name, attr_value } => {
            self.context_menu.show_for_detail(dn, attr_name, attr_value);
        }
    }
}
Action::CopyToClipboard(text) => {
    // clipboard logic (see section 4)
}
```

### 5f. Popup Active Check

Add to `popup_active()`:

```rust
fn popup_active(&self) -> bool {
    self.context_menu.visible
        || self.confirm_dialog.visible
        || self.connect_dialog.visible
        // ... rest unchanged ...
}
```

### 5g. Render Order

Render the context menu **after** other popups so it appears on top (since it's
lightweight and should always be visible when active):

```rust
// At the end of the popup render block, after log_panel:
if self.context_menu.visible {
    self.context_menu.render(frame, full);
}
```

### 5h. Module Registration

In `components/mod.rs`, add:

```rust
pub mod context_menu;
```

In `app.rs` imports, add:

```rust
use crate::components::context_menu::{ContextMenu, ContextMenuSource};
```

---

## 6. Help Popup Update

Add context menu to the help popup sections in `help_popup.rs`:

```rust
HelpSection {
    title: "CONTEXT MENU".to_string(),
    entries: vec![
        ("Space".to_string(), "Open context menu".to_string()),
        ("j/k \u{2191}/\u{2193}".to_string(), "Navigate items".to_string()),
        ("Enter/Space".to_string(), "Select item".to_string()),
        ("Esc/q".to_string(), "Close menu".to_string()),
    ],
},
```

Also add "Space" hint to the Tree Panel and Detail Panel sections:

```rust
// In TREE PANEL section:
("Space".to_string(), "Context menu".to_string()),

// In DETAIL PANEL section:
("Space".to_string(), "Context menu".to_string()),
```

---

## 7. Status Bar Hint Update

Update the detail panel's inline hint bar (rendered at bottom of detail area
when focused) to include Space:

```rust
// In detail_panel.rs render(), add to hint_line:
Span::styled("Space", self.theme.header),
Span::styled(":Menu  ", self.theme.dimmed),
```

Consider also adding a hint to the tree panel's status display if there is one,
or updating the global status bar to show "Space:Menu" when tree/detail is focused.

---

## 8. Configurable Keybinding (Optional Enhancement)

If desired, add `context_menu` to the keymap system so it can be rebound:

### config.rs

```rust
pub struct KeybindingConfig {
    // ... existing fields ...
    pub context_menu: String,
}

impl Default for KeybindingConfig {
    fn default() -> Self {
        Self {
            // ... existing ...
            context_menu: "Space".to_string(),
        }
    }
}
```

### keymap.rs

Add to the bindings vector in `Keymap::from_config()`:

```rust
("context_menu", &config.context_menu, &defaults.context_menu, Action::ShowContextMenu(ContextMenuSource::Tree { dn: String::new() })),
```

> **Caveat**: The keymap system returns a fixed `Action`, but `ShowContextMenu`
> needs the current DN/attribute. There are two approaches:
>
> 1. **Sentinel approach**: The keymap returns a `ShowContextMenu` with empty
>    data, and `process_action` fills in the actual state from the focused panel.
>    This is simpler but slightly awkward.
>
> 2. **Panel-level approach** (recommended): Keep Space handling in the panel
>    `handle_key_event` methods (Option A from 5c), and do NOT put context_menu
>    in the global keymap. The configurable binding is only used to *recognize*
>    the key in the panel handlers by looking up the configured key.
>
> For simplicity, the initial implementation should use **Option A** (panel-level
> handling with hardcoded Space). The configurable keybinding can be added later
> if users request it.

---

## 9. File Change Summary

| File | Change |
|------|--------|
| `Cargo.toml` (workspace) | Add `arboard = "3"` to `[workspace.dependencies]` |
| `crates/loom-tui/Cargo.toml` | Add `arboard = { workspace = true }` |
| `crates/loom-tui/src/action.rs` | Add `ShowContextMenu(ContextMenuSource)`, `CopyToClipboard(String)`, `ContextMenuSource` enum |
| `crates/loom-tui/src/components/mod.rs` | Add `pub mod context_menu;` |
| **`crates/loom-tui/src/components/context_menu.rs`** | **New file** -- `ContextMenu` struct, `MenuItem`, show/hide/handle/render |
| `crates/loom-tui/src/components/tree_panel.rs` | Add `KeyCode::Char(' ')` match arm returning `ShowContextMenu` |
| `crates/loom-tui/src/components/detail_panel.rs` | Add `KeyCode::Char(' ')` match arm returning `ShowContextMenu`; add "Space:Menu" to hint bar |
| `crates/loom-tui/src/components/help_popup.rs` | Add CONTEXT MENU section; add Space hint to tree/detail sections |
| `crates/loom-tui/src/app.rs` | Add `context_menu` field; handle in key dispatch (first popup check), mouse dispatch (right-click), `process_action` (`ShowContextMenu`, `CopyToClipboard`), `popup_active()`, render order (last) |

---

## 10. Testing Plan

### Unit Tests (`context_menu.rs`)

1. **`test_show_for_tree_populates_items`** -- verify items, selected=0, visible=true
2. **`test_show_for_detail_populates_items`** -- verify items for detail source
3. **`test_hide_clears_state`** -- verify visible=false, items empty, source None
4. **`test_navigate_down`** -- j/Down increments selected, clamps at end
5. **`test_navigate_up`** -- k/Up decrements selected, clamps at 0
6. **`test_enter_returns_action_and_hides`** -- verify correct action returned, menu hidden
7. **`test_space_selects_item`** -- Space also confirms selection
8. **`test_esc_closes_menu`** -- returns ClosePopup, hides menu
9. **`test_first_letter_jump`** -- pressing 'c' jumps to first "Copy..." item
10. **`test_anchor_position`** -- set_anchor stores coordinates

### Integration Tests

11. **`test_space_in_tree_shows_menu`** -- simulate Space key with tree focused, verify context menu visible
12. **`test_space_in_detail_shows_menu`** -- simulate Space key with detail focused
13. **`test_context_menu_blocks_other_input`** -- verify popup_active returns true when context menu visible
14. **`test_copy_to_clipboard_action`** -- verify CopyToClipboard sends StatusMessage on success

---

## 11. Implementation Order

1. Add `ContextMenuSource` enum and new `Action` variants to `action.rs`
2. Create `context_menu.rs` with `ContextMenu` struct, key handling, rendering
3. Register module in `components/mod.rs`
4. Add `arboard` dependency to workspace and loom-tui Cargo.toml
5. Wire into `app.rs`: field, init, key dispatch, process_action, popup_active, render
6. Add Space handler to `tree_panel.rs` and `detail_panel.rs`
7. Handle right-click in `handle_mouse()`
8. Handle `CopyToClipboard` in `process_action`
9. Update help popup sections
10. Update detail panel hint bar
11. Write unit tests
12. Cargo check + clippy + test
