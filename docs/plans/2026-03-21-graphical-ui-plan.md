# Graphical UI Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add an optional Slint-based GUI (`loom-browser`) alongside the existing TUI (`loom`), sharing `loom-core` for all LDAP, config, vault, and export logic.

**Architecture:** Two new crates (`loom-gui`, `loom-browser`) depend on `loom-core`. Config types move from `loom-tui` to `loom-core`. Slint runs the event loop on the main thread; LDAP operations run via `async-compat` + `slint::spawn_local`. No `#[tokio::main]`.

**Tech Stack:** Slint 1.15, async-compat, loom-core (ldap3, rustls, serde, toml)

**Important Slint patterns:**
- `build.rs` with `slint_build::compile()` compiles `.slint` files
- `slint::include_modules!()` in Rust imports generated types
- `slint::spawn_local(Compat::new(async { ... }))` for tokio async (ldap3)
- No built-in tree view — build custom with `ListView` + flat model with indent levels
- `StandardTableView` for attribute display
- `TabWidget` for connection tabs
- Globals for theming: `global AppTheme { in-out property <color> ... }`

---

## Phase 1: Foundation (Tasks 1-5)

### Task 1: Move config types from loom-tui to loom-core

**Files:**
- Move: `crates/loom-tui/src/config.rs` -> `crates/loom-core/src/config.rs`
- Modify: `crates/loom-core/src/lib.rs` (add `pub mod config`)
- Modify: `crates/loom-core/Cargo.toml` (already has `toml` and `serde` — no new deps)
- Modify: `crates/loom-tui/src/config.rs` (replace with re-export)
- Modify: all files in `loom-tui` that import from `crate::config`

**Step 1: Copy config.rs to loom-core**

Copy `crates/loom-tui/src/config.rs` to `crates/loom-core/src/config.rs`. Add `pub mod config;` to `crates/loom-core/src/lib.rs`.

**Step 2: Replace loom-tui config.rs with re-export**

Replace `crates/loom-tui/src/config.rs` contents with:

```rust
pub use loom_core::config::*;
```

**Step 3: Run tests to verify nothing broke**

Run: `cargo test --workspace`
Expected: All existing tests pass — this is a pure move + re-export.

**Step 4: Run clippy**

Run: `cargo clippy --workspace -- -D warnings`
Expected: No warnings.

**Step 5: Commit**

```bash
cargo fmt --all
git add -A
git commit -m "refactor: move config types from loom-tui to loom-core"
```

---

### Task 2: Rename TUI binary to "loom"

**Files:**
- Modify: `crates/loom-ldapbrowser/Cargo.toml` (change `name` and `[[bin]]`)

**Step 1: Update Cargo.toml**

In `crates/loom-ldapbrowser/Cargo.toml`, change:

```toml
[package]
name = "loom"

[[bin]]
name = "loom"
path = "src/main.rs"
```

**Step 2: Verify build**

Run: `cargo build -p loom`
Expected: Builds successfully, binary at `target/debug/loom`.

**Step 3: Verify existing tests pass**

Run: `cargo test --workspace`
Expected: All tests pass.

**Step 4: Commit**

```bash
cargo fmt --all
git add -A
git commit -m "refactor: rename TUI binary from loom-ldapbrowser to loom"
```

---

### Task 3: Create loom-gui crate (empty skeleton)

**Files:**
- Create: `crates/loom-gui/Cargo.toml`
- Create: `crates/loom-gui/src/lib.rs`
- Create: `crates/loom-gui/ui/main.slint`
- Create: `crates/loom-gui/build.rs`
- Modify: `Cargo.toml` (workspace members)

**Step 1: Create Cargo.toml**

```toml
[package]
name = "loom-gui"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
description = "Slint-based graphical UI for loom LDAP browser"
build = "build.rs"

[dependencies]
slint = "1.15"
loom-core = { path = "../loom-core" }
async-compat = "0.2"

[build-dependencies]
slint-build = "1.15"
```

**Step 2: Create build.rs**

```rust
fn main() {
    slint_build::compile("ui/main.slint").unwrap();
}
```

**Step 3: Create minimal main.slint**

```slint
import { VerticalBox, Text } from "std-widgets.slint";

export component MainWindow inherits Window {
    title: "Loom Browser";
    preferred-width: 1200px;
    preferred-height: 800px;

    VerticalBox {
        Text {
            text: "Loom Browser — coming soon";
            horizontal-alignment: center;
            vertical-alignment: center;
        }
    }
}
```

**Step 4: Create lib.rs**

```rust
slint::include_modules!();

pub fn run() -> Result<(), slint::PlatformError> {
    let main_window = MainWindow::new()?;
    main_window.run()
}
```

**Step 5: Add to workspace**

Add `"crates/loom-gui"` to `[workspace.members]` in the root `Cargo.toml`. Also add workspace deps:

```toml
# GUI
slint = "1.15"
slint-build = "1.15"
async-compat = "0.2"

# Internal
loom-gui = { path = "crates/loom-gui" }
```

**Step 6: Verify build**

Run: `cargo build -p loom-gui`
Expected: Compiles successfully.

**Step 7: Commit**

```bash
cargo fmt --all
git add -A
git commit -m "feat: add loom-gui crate skeleton with Slint"
```

---

### Task 4: Create loom-browser binary crate

**Files:**
- Create: `crates/loom-browser/Cargo.toml`
- Create: `crates/loom-browser/src/main.rs`
- Modify: `Cargo.toml` (workspace members)

**Step 1: Create Cargo.toml**

```toml
[package]
name = "loom-browser"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
description = "Graphical LDAP browser (Slint GUI)"

[[bin]]
name = "loom-browser"
path = "src/main.rs"

[dependencies]
loom-gui = { workspace = true }
loom-core = { workspace = true }
```

**Step 2: Create main.rs**

```rust
fn main() -> Result<(), slint::PlatformError> {
    loom_gui::run()
}
```

Wait — `main.rs` needs `slint` in scope for the error type. Add `slint` to dependencies:

```toml
[dependencies]
loom-gui = { workspace = true }
loom-core = { workspace = true }
slint = { workspace = true }
```

**Step 3: Add to workspace members**

Add `"crates/loom-browser"` to `[workspace.members]` in root `Cargo.toml`.

**Step 4: Verify it runs**

Run: `cargo run -p loom-browser`
Expected: A window appears with "Loom Browser — coming soon". Close it.

**Step 5: Commit**

```bash
cargo fmt --all
git add -A
git commit -m "feat: add loom-browser binary crate"
```

---

### Task 5: Theme global and dark theme

**Files:**
- Create: `crates/loom-gui/ui/theme.slint`
- Modify: `crates/loom-gui/ui/main.slint`
- Modify: `crates/loom-gui/src/lib.rs`

**Step 1: Create theme.slint**

```slint
export global AppTheme {
    // Background
    in-out property <color> bg-primary: #1a1a1a;
    in-out property <color> bg-secondary: #252525;
    in-out property <color> bg-tertiary: #2f2f2f;
    in-out property <color> bg-hover: #333333;
    in-out property <color> bg-selected: #3a3a3a;

    // Foreground
    in-out property <color> fg-primary: #e0e0e0;
    in-out property <color> fg-secondary: #a0a0a0;
    in-out property <color> fg-muted: #666666;

    // Accent
    in-out property <color> accent: #4a9eff;
    in-out property <color> accent-hover: #6ab0ff;

    // Semantic
    in-out property <color> error: #ff5555;
    in-out property <color> success: #50fa7b;
    in-out property <color> warning: #f1fa8c;

    // Border
    in-out property <color> border: #333333;
    in-out property <color> border-focus: #4a9eff;

    // Dimensions
    in-out property <length> border-radius: 4px;
    in-out property <length> spacing: 8px;
    in-out property <length> padding: 12px;
}
```

**Step 2: Update main.slint to use theme**

```slint
import { VerticalBox } from "std-widgets.slint";
import { AppTheme } from "theme.slint";

export { AppTheme }

export component MainWindow inherits Window {
    title: "Loom Browser";
    preferred-width: 1200px;
    preferred-height: 800px;
    background: AppTheme.bg-primary;

    VerticalBox {
        Text {
            text: "Loom Browser";
            color: AppTheme.fg-primary;
            horizontal-alignment: center;
            vertical-alignment: center;
            font-size: 24px;
        }
    }
}
```

**Step 3: Add theme loading from config in lib.rs**

```rust
slint::include_modules!();

use loom_core::config::AppConfig;

pub fn run() -> Result<(), slint::PlatformError> {
    let config = AppConfig::load();
    let main_window = MainWindow::new()?;

    apply_theme(&main_window, &config.general.theme);

    main_window.run()
}

fn apply_theme(window: &MainWindow, theme_name: &str) {
    let theme = window.global::<AppTheme>();
    match theme_name {
        "light" => {
            theme.set_bg_primary(slint::Color::from_rgb_u8(0xfa, 0xfa, 0xfa));
            theme.set_bg_secondary(slint::Color::from_rgb_u8(0xf0, 0xf0, 0xf0));
            theme.set_bg_tertiary(slint::Color::from_rgb_u8(0xe8, 0xe8, 0xe8));
            theme.set_fg_primary(slint::Color::from_rgb_u8(0x1a, 0x1a, 0x1a));
            theme.set_fg_secondary(slint::Color::from_rgb_u8(0x55, 0x55, 0x55));
            theme.set_border(slint::Color::from_rgb_u8(0xd0, 0xd0, 0xd0));
        }
        "solarized" => {
            theme.set_bg_primary(slint::Color::from_rgb_u8(0x00, 0x2b, 0x36));
            theme.set_bg_secondary(slint::Color::from_rgb_u8(0x07, 0x36, 0x42));
            theme.set_fg_primary(slint::Color::from_rgb_u8(0x83, 0x94, 0x96));
            theme.set_accent(slint::Color::from_rgb_u8(0x26, 0x8b, 0xd2));
            theme.set_border(slint::Color::from_rgb_u8(0x58, 0x6e, 0x75));
        }
        "nord" => {
            theme.set_bg_primary(slint::Color::from_rgb_u8(0x2e, 0x34, 0x40));
            theme.set_bg_secondary(slint::Color::from_rgb_u8(0x3b, 0x42, 0x52));
            theme.set_fg_primary(slint::Color::from_rgb_u8(0xec, 0xef, 0xf4));
            theme.set_accent(slint::Color::from_rgb_u8(0x88, 0xc0, 0xd0));
            theme.set_border(slint::Color::from_rgb_u8(0x4c, 0x56, 0x6a));
        }
        "matrix" => {
            theme.set_bg_primary(slint::Color::from_rgb_u8(0x0a, 0x0a, 0x0a));
            theme.set_bg_secondary(slint::Color::from_rgb_u8(0x12, 0x12, 0x12));
            theme.set_fg_primary(slint::Color::from_rgb_u8(0x00, 0xff, 0x00));
            theme.set_accent(slint::Color::from_rgb_u8(0x00, 0xcc, 0x00));
            theme.set_border(slint::Color::from_rgb_u8(0x00, 0x33, 0x00));
        }
        _ => {} // "dark" is the default from theme.slint
    }
}
```

**Step 4: Verify**

Run: `cargo run -p loom-browser`
Expected: Dark-themed window with "Loom Browser" text.

**Step 5: Commit**

```bash
cargo fmt --all
git add -A
git commit -m "feat(gui): add theme system with 5 built-in themes"
```

---

## Phase 2: Layout Shell (Tasks 6-9)

### Task 6: Status bar component

**Files:**
- Create: `crates/loom-gui/ui/status-bar.slint`
- Modify: `crates/loom-gui/ui/main.slint`

**Step 1: Create status-bar.slint**

```slint
import { AppTheme } from "theme.slint";

export component StatusBar inherits Rectangle {
    in property <string> message: "Ready";
    in property <bool> is-error: false;

    height: 28px;
    background: AppTheme.bg-secondary;
    border-width: 1px;
    border-color: AppTheme.border;

    HorizontalLayout {
        padding-left: 8px;
        padding-right: 8px;
        alignment: start;

        Text {
            text: root.message;
            color: root.is-error ? AppTheme.error : AppTheme.fg-secondary;
            font-size: 12px;
            vertical-alignment: center;
        }
    }
}
```

**Step 2: Add to main.slint**

Update `main.slint` to include the status bar at the bottom:

```slint
import { VerticalBox } from "std-widgets.slint";
import { AppTheme } from "theme.slint";
import { StatusBar } from "status-bar.slint";

export { AppTheme }

export component MainWindow inherits Window {
    title: "Loom Browser";
    preferred-width: 1200px;
    preferred-height: 800px;
    background: AppTheme.bg-primary;

    in-out property <string> status-message: "Ready";
    in-out property <bool> status-is-error: false;

    VerticalLayout {
        // Main content area (placeholder)
        Rectangle {
            vertical-stretch: 1;
            background: AppTheme.bg-primary;

            Text {
                text: "No connections. Use Profiles > New Profile to get started.";
                color: AppTheme.fg-muted;
                horizontal-alignment: center;
                vertical-alignment: center;
            }
        }

        // Status bar
        StatusBar {
            message: root.status-message;
            is-error: root.status-is-error;
        }
    }
}
```

**Step 3: Wire status bar from Rust**

In `lib.rs`, after creating the window, set the initial status:

```rust
main_window.set_status_message("Ready".into());
```

**Step 4: Verify**

Run: `cargo run -p loom-browser`
Expected: Window with centered placeholder text and "Ready" status bar at bottom.

**Step 5: Commit**

```bash
cargo fmt --all
git add -A
git commit -m "feat(gui): add status bar component"
```

---

### Task 7: Tab bar component

**Files:**
- Create: `crates/loom-gui/ui/tab-bar.slint`
- Modify: `crates/loom-gui/ui/main.slint`

**Step 1: Create tab-bar.slint**

```slint
import { AppTheme } from "theme.slint";

export struct TabInfo {
    id: int,
    title: string,
}

export component TabBar inherits Rectangle {
    in property <[TabInfo]> tabs: [];
    in-out property <int> active-tab: -1;
    callback tab-clicked(int);
    callback close-tab(int);

    height: 36px;
    background: AppTheme.bg-secondary;
    border-width: 1px;
    border-color: AppTheme.border;

    HorizontalLayout {
        padding-left: 4px;
        alignment: start;
        spacing: 2px;

        for tab[index] in root.tabs: Rectangle {
            width: 180px;
            height: 32px;
            border-radius: AppTheme.border-radius;
            background: index == root.active-tab ? AppTheme.bg-primary : AppTheme.bg-tertiary;

            HorizontalLayout {
                padding-left: 12px;
                padding-right: 4px;

                Text {
                    text: tab.title;
                    color: index == root.active-tab ? AppTheme.fg-primary : AppTheme.fg-secondary;
                    font-size: 13px;
                    vertical-alignment: center;
                    overflow: elide;
                    horizontal-stretch: 1;
                }

                // Close button
                Rectangle {
                    width: 20px;
                    height: 20px;
                    border-radius: 3px;
                    background: close-touch.has-hover ? AppTheme.bg-hover : transparent;
                    vertical-alignment: center;

                    Text {
                        text: "x";
                        color: AppTheme.fg-muted;
                        font-size: 12px;
                        horizontal-alignment: center;
                        vertical-alignment: center;
                    }

                    close-touch := TouchArea {
                        clicked => {
                            root.close-tab(index);
                        }
                    }
                }
            }

            TouchArea {
                clicked => {
                    root.tab-clicked(index);
                }
            }
        }
    }
}
```

**Step 2: Add to main.slint**

Insert the tab bar between the menu area and the main content:

```slint
import { VerticalBox } from "std-widgets.slint";
import { AppTheme } from "theme.slint";
import { StatusBar } from "status-bar.slint";
import { TabBar, TabInfo } from "tab-bar.slint";

export { AppTheme }

export component MainWindow inherits Window {
    title: "Loom Browser";
    preferred-width: 1200px;
    preferred-height: 800px;
    background: AppTheme.bg-primary;

    in-out property <string> status-message: "Ready";
    in-out property <bool> status-is-error: false;
    in-out property <[TabInfo]> tabs: [];
    in-out property <int> active-tab: -1;

    callback tab-clicked(int);
    callback close-tab(int);

    VerticalLayout {
        // Tab bar
        TabBar {
            tabs: root.tabs;
            active-tab: root.active-tab;
            tab-clicked(index) => { root.tab-clicked(index); }
            close-tab(index) => { root.close-tab(index); }
        }

        // Main content area (placeholder)
        Rectangle {
            vertical-stretch: 1;
            background: AppTheme.bg-primary;

            Text {
                text: root.tabs.length == 0 ?
                    "No connections. Use Profiles > New Profile to get started." :
                    "";
                color: AppTheme.fg-muted;
                horizontal-alignment: center;
                vertical-alignment: center;
            }
        }

        StatusBar {
            message: root.status-message;
            is-error: root.status-is-error;
        }
    }
}
```

**Step 3: Verify**

Run: `cargo run -p loom-browser`
Expected: Empty tab bar visible above the placeholder content.

**Step 4: Commit**

```bash
cargo fmt --all
git add -A
git commit -m "feat(gui): add tab bar component"
```

---

### Task 8: Custom tree view component

This is the most complex widget. Slint has no built-in tree view, so we build one with a flat `ListView` and indent levels.

**Files:**
- Create: `crates/loom-gui/ui/tree-view.slint`

**Step 1: Create tree-view.slint**

```slint
import { AppTheme } from "theme.slint";
import { ListView } from "std-widgets.slint";

export struct TreeNode {
    text: string,
    indent-level: int,
    expanded: bool,
    has-children: bool,
    is-loading: bool,
    is-selected: bool,
}

export component TreeView inherits Rectangle {
    in property <[TreeNode]> model: [];
    in-out property <int> selected-index: -1;
    callback toggle-expand(int);
    callback node-selected(int);

    background: AppTheme.bg-primary;

    ListView {
        for node[index] in root.model: Rectangle {
            height: 28px;
            background: node.is-selected ? AppTheme.bg-selected :
                        row-touch.has-hover ? AppTheme.bg-hover : transparent;

            HorizontalLayout {
                padding-left: node.indent-level * 20px + 4px;
                spacing: 4px;
                alignment: start;

                // Expand/collapse indicator
                Rectangle {
                    width: 16px;
                    height: 28px;

                    Text {
                        text: !node.has-children ? " " :
                              node.is-loading ? "..." :
                              node.expanded ? "v" : ">";
                        color: AppTheme.fg-muted;
                        font-size: 11px;
                        font-family: "monospace";
                        horizontal-alignment: center;
                        vertical-alignment: center;
                    }

                    TouchArea {
                        clicked => {
                            if node.has-children {
                                root.toggle-expand(index);
                            }
                        }
                    }
                }

                // Node text
                Text {
                    text: node.text;
                    color: node.is-selected ? AppTheme.accent : AppTheme.fg-primary;
                    font-size: 13px;
                    font-family: "monospace";
                    vertical-alignment: center;
                    overflow: elide;
                }
            }

            row-touch := TouchArea {
                clicked => {
                    root.node-selected(index);
                }
            }
        }
    }
}
```

**Step 2: Verify it compiles**

Add a temporary import in `main.slint`:

```slint
import { TreeView, TreeNode } from "tree-view.slint";
```

Run: `cargo build -p loom-gui`
Expected: Compiles.

**Step 3: Commit**

```bash
cargo fmt --all
git add -A
git commit -m "feat(gui): add custom tree view component"
```

---

### Task 9: Attribute detail panel and split layout

**Files:**
- Create: `crates/loom-gui/ui/detail-panel.slint`
- Modify: `crates/loom-gui/ui/main.slint` (compose the full layout)

**Step 1: Create detail-panel.slint**

```slint
import { AppTheme } from "theme.slint";
import { StandardTableView } from "std-widgets.slint";

export struct AttributeRow {
    name: string,
    value: string,
}

export component DetailPanel inherits Rectangle {
    in property <[AttributeRow]> attributes: [];
    in property <string> entry-dn: "";

    background: AppTheme.bg-primary;

    VerticalLayout {
        // DN header
        Rectangle {
            height: 32px;
            background: AppTheme.bg-secondary;
            border-width: 1px;
            border-color: AppTheme.border;

            HorizontalLayout {
                padding-left: 8px;

                Text {
                    text: root.entry-dn;
                    color: AppTheme.fg-primary;
                    font-size: 12px;
                    font-family: "monospace";
                    vertical-alignment: center;
                    overflow: elide;
                }
            }
        }

        // Attribute list
        ListView {
            for attr[index] in root.attributes: Rectangle {
                height: 28px;
                background: Math.mod(index, 2) == 0 ? AppTheme.bg-primary : AppTheme.bg-secondary;

                HorizontalLayout {
                    padding-left: 8px;
                    spacing: 16px;

                    Text {
                        text: attr.name;
                        color: AppTheme.fg-secondary;
                        font-size: 13px;
                        font-family: "monospace";
                        width: 200px;
                        vertical-alignment: center;
                    }

                    Text {
                        text: attr.value;
                        color: AppTheme.fg-primary;
                        font-size: 13px;
                        font-family: "monospace";
                        vertical-alignment: center;
                        overflow: elide;
                        horizontal-stretch: 1;
                    }
                }
            }
        }
    }
}
```

**Step 2: Compose the full layout in main.slint**

```slint
import { VerticalBox, HorizontalBox } from "std-widgets.slint";
import { AppTheme } from "theme.slint";
import { StatusBar } from "status-bar.slint";
import { TabBar, TabInfo } from "tab-bar.slint";
import { TreeView, TreeNode } from "tree-view.slint";
import { DetailPanel, AttributeRow } from "detail-panel.slint";

export { AppTheme }

export component MainWindow inherits Window {
    title: "Loom Browser";
    preferred-width: 1200px;
    preferred-height: 800px;
    background: AppTheme.bg-primary;

    // Status
    in-out property <string> status-message: "Ready";
    in-out property <bool> status-is-error: false;

    // Tabs
    in-out property <[TabInfo]> tabs: [];
    in-out property <int> active-tab: -1;
    callback tab-clicked(int);
    callback close-tab(int);

    // Tree
    in-out property <[TreeNode]> tree-model: [];
    in-out property <int> tree-selected-index: -1;
    callback tree-toggle-expand(int);
    callback tree-node-selected(int);

    // Detail
    in-out property <[AttributeRow]> attributes: [];
    in-out property <string> entry-dn: "";

    VerticalLayout {
        TabBar {
            tabs: root.tabs;
            active-tab: root.active-tab;
            tab-clicked(index) => { root.tab-clicked(index); }
            close-tab(index) => { root.close-tab(index); }
        }

        // Main content
        if root.tabs.length == 0: Rectangle {
            vertical-stretch: 1;
            background: AppTheme.bg-primary;

            Text {
                text: "No connections. Use Profiles > New Profile to get started.";
                color: AppTheme.fg-muted;
                horizontal-alignment: center;
                vertical-alignment: center;
                font-size: 14px;
            }
        }

        if root.tabs.length > 0: HorizontalLayout {
            vertical-stretch: 1;

            TreeView {
                model: root.tree-model;
                selected-index: root.tree-selected-index;
                horizontal-stretch: 1;
                toggle-expand(index) => { root.tree-toggle-expand(index); }
                node-selected(index) => { root.tree-node-selected(index); }
            }

            // Divider
            Rectangle {
                width: 1px;
                background: AppTheme.border;
            }

            DetailPanel {
                attributes: root.attributes;
                entry-dn: root.entry-dn;
                horizontal-stretch: 2;
            }
        }

        StatusBar {
            message: root.status-message;
            is-error: root.status-is-error;
        }
    }
}
```

**Step 3: Verify**

Run: `cargo run -p loom-browser`
Expected: Window with tab bar, empty state placeholder, status bar. Full layout shell.

**Step 4: Commit**

```bash
cargo fmt --all
git add -A
git commit -m "feat(gui): compose full layout with tree, detail panel, tabs, status bar"
```

---

## Phase 3: Core Integration (Tasks 10-13)

### Task 10: Async bridge — connect to LDAP from GUI

**Files:**
- Modify: `crates/loom-gui/src/lib.rs`

**Step 1: Add connection logic**

Restructure `lib.rs` to handle async LDAP connections via `spawn_local` + `Compat`:

```rust
slint::include_modules!();

use async_compat::Compat;
use loom_core::config::AppConfig;
use loom_core::connection::LdapConnection;
use loom_core::tls::TrustStore;
use std::sync::Arc;

pub fn run() -> Result<(), slint::PlatformError> {
    let config = AppConfig::load();
    let main_window = MainWindow::new()?;

    apply_theme(&main_window, &config.general.theme);

    // Populate initial state
    let weak = main_window.as_weak();
    let profiles = config.connections.clone();

    // Handle connect callback
    let config_clone = config.clone();
    main_window.on_connect_profile({
        let weak = weak.clone();
        move |profile_index| {
            let profile_index = profile_index as usize;
            if let Some(profile) = config_clone.connections.get(profile_index) {
                let profile = profile.clone();
                let weak = weak.clone();
                let trust_store = Arc::new(TrustStore::default());

                // Update status
                if let Some(win) = weak.upgrade() {
                    win.set_status_message(
                        format!("Connecting to {}...", profile.host).into(),
                    );
                }

                slint::spawn_local(Compat::new(async move {
                    let settings = profile.to_connection_settings();
                    match LdapConnection::connect(settings, Some(trust_store)).await {
                        Ok(_conn) => {
                            if let Some(win) = weak.upgrade() {
                                win.set_status_message("Connected".into());
                                win.set_status_is_error(false);
                            }
                        }
                        Err(e) => {
                            if let Some(win) = weak.upgrade() {
                                win.set_status_message(
                                    format!("Connection failed: {}", e).into(),
                                );
                                win.set_status_is_error(true);
                            }
                        }
                    }
                }))
                .unwrap();
            }
        }
    });

    main_window.run()
}

// apply_theme function from Task 5 (unchanged)
fn apply_theme(window: &MainWindow, theme_name: &str) {
    // ... same as Task 5
}
```

**Step 2: Add callback to main.slint**

Add to `MainWindow`:

```slint
callback connect-profile(int);
```

**Step 3: Verify build**

Run: `cargo build -p loom-browser`
Expected: Compiles. Connection won't be testable without an LDAP server, but the async bridge is wired.

**Step 4: Commit**

```bash
cargo fmt --all
git add -A
git commit -m "feat(gui): wire async LDAP connection via spawn_local + Compat"
```

---

### Task 11: Profile menu and connection flow

**Files:**
- Create: `crates/loom-gui/ui/profiles-menu.slint`
- Modify: `crates/loom-gui/ui/main.slint`
- Modify: `crates/loom-gui/src/lib.rs`

This task wires saved profiles into the Profiles dropdown and creates tabs on connect. Implementation depends on Slint's `MenuBar` widget or a custom popup. Check Slint docs for `PopupWindow` if `MenuBar` is insufficient.

**Step 1: Define profile data structs in main.slint**

```slint
export struct ProfileInfo {
    name: string,
    host: string,
    folder: string,
    index: int,
}
```

**Step 2: Add profiles property and callbacks to MainWindow**

```slint
in-out property <[ProfileInfo]> profiles: [];
callback connect-profile(int);
callback new-profile-requested();
```

**Step 3: Populate profiles from Rust**

In `lib.rs`, after creating the window:

```rust
let profile_model: Vec<ProfileInfo> = config
    .connections
    .iter()
    .enumerate()
    .map(|(i, p)| ProfileInfo {
        name: p.name.clone().into(),
        host: p.host.clone().into(),
        folder: p.folder.clone().unwrap_or_default().into(),
        index: i as i32,
    })
    .collect();
let model = std::rc::Rc::new(slint::VecModel::from(profile_model));
main_window.set_profiles(model.into());
```

**Step 4: Handle tab creation on connect**

When `connect-profile` callback fires, add a tab, set it active, and begin async connection.

**Step 5: Verify**

Run: `cargo run -p loom-browser`
Expected: If `config.toml` has profiles, they are accessible. Clicking one triggers a connection attempt and creates a tab.

**Step 6: Commit**

```bash
cargo fmt --all
git add -A
git commit -m "feat(gui): wire profiles menu and tab creation on connect"
```

---

### Task 12: Tree population from LDAP search

**Files:**
- Modify: `crates/loom-gui/src/lib.rs`

**Step 1: On successful connect, search for base DN children**

After the connection succeeds in the `connect-profile` handler, issue a one-level search at the base DN and populate the tree model:

```rust
// After successful connect:
let base_dn = profile.base_dn.clone().unwrap_or_default();
let entries = conn.search_one_level(&base_dn).await?;

let mut tree_nodes = vec![TreeNode {
    text: base_dn.clone().into(),
    indent_level: 0,
    expanded: true,
    has_children: true,
    is_loading: false,
    is_selected: false,
}];

for entry in entries {
    tree_nodes.push(TreeNode {
        text: entry.dn.clone().into(),
        indent_level: 1,
        expanded: false,
        has_children: true, // assume children until proven otherwise
        is_loading: false,
        is_selected: false,
    });
}

if let Some(win) = weak.upgrade() {
    let model = std::rc::Rc::new(slint::VecModel::from(tree_nodes));
    win.set_tree_model(model.into());
}
```

**Step 2: Handle tree-toggle-expand callback**

When user clicks expand, issue a one-level search for that DN and insert children into the flat model.

**Step 3: Handle tree-node-selected callback**

When user clicks a node, fetch its attributes and populate the detail panel.

**Step 4: Verify with offline mode**

Test against the built-in offline example directory if no live LDAP server is available.

**Step 5: Commit**

```bash
cargo fmt --all
git add -A
git commit -m "feat(gui): populate tree from LDAP search results"
```

---

### Task 13: Attribute detail panel population

**Files:**
- Modify: `crates/loom-gui/src/lib.rs`

**Step 1: On node select, fetch entry attributes**

When `tree-node-selected` fires, read the DN from the tree model and search for the entry's attributes:

```rust
main_window.on_tree_node_selected({
    let weak = weak.clone();
    move |index| {
        let weak = weak.clone();
        slint::spawn_local(Compat::new(async move {
            // Get DN from tree model at index
            // Search LDAP for entry attributes
            // Map to AttributeRow vec
            // Set on window: win.set_attributes(...), win.set_entry_dn(...)
        })).unwrap();
    }
});
```

**Step 2: Map LDAP attributes to AttributeRow**

```rust
let rows: Vec<AttributeRow> = entry
    .attrs
    .iter()
    .flat_map(|(name, values)| {
        values.iter().map(move |v| AttributeRow {
            name: name.clone().into(),
            value: v.clone().into(),
        })
    })
    .collect();
```

**Step 3: Verify**

Click a tree node -> detail panel shows attributes.

**Step 4: Commit**

```bash
cargo fmt --all
git add -A
git commit -m "feat(gui): show entry attributes in detail panel"
```

---

## Phase 4: Dialogs (Tasks 14-16)

### Task 14: New Profile dialog

**Files:**
- Create: `crates/loom-gui/ui/profile-dialog.slint`
- Modify: `crates/loom-gui/ui/main.slint`
- Modify: `crates/loom-gui/src/lib.rs`

Build a modal dialog with: Name, Host, Port, TLS Mode (ComboBox), Bind DN, Base DN, Credential Method (ComboBox), Folder. Save and Cancel buttons. On save, call `AppConfig::append_connection()` or `config.save()` and update the profiles model.

**Step 1: Create profile-dialog.slint**

Use Slint's `Dialog` or a `PopupWindow` with form fields using `LineEdit` and `ComboBox` from std-widgets.

**Step 2: Wire save callback to Rust**

On save, validate name + host are non-empty, construct a `ConnectionProfile`, save to config.

**Step 3: Verify**

New Profile -> fill form -> Save -> profile appears in menu.

**Step 4: Commit**

```bash
cargo fmt --all
git add -A
git commit -m "feat(gui): add new profile dialog"
```

---

### Task 15: Vault password dialog

**Files:**
- Create: `crates/loom-gui/ui/vault-dialog.slint`
- Modify: `crates/loom-gui/src/lib.rs`

On startup, if `vault_enabled && vault.dat exists`, show a modal dialog with password field + Unlock/Skip. On unlock, call `Vault::open()`. On skip, continue without vault.

**Step 1: Create vault-dialog.slint**

Modal with `LineEdit` (input-type: password), Unlock button, Skip button, error text.

**Step 2: Wire to Rust startup**

After `MainWindow::new()`, check config. If vault needed, show dialog and handle callbacks.

**Step 3: Commit**

```bash
cargo fmt --all
git add -A
git commit -m "feat(gui): add vault password dialog on startup"
```

---

### Task 16: Credential prompt dialog

**Files:**
- Create: `crates/loom-gui/ui/credential-dialog.slint`
- Modify: `crates/loom-gui/src/lib.rs`

When connecting and no password is available (credential method = Prompt, no vault entry), show a password dialog. On submit, retry the connection with the password.

**Step 1: Create credential-dialog.slint**

Modal with profile name display, password `LineEdit`, Connect/Cancel buttons.

**Step 2: Wire to connection flow**

In the connect handler, if `resolve_password` returns empty, show dialog instead of connecting.

**Step 3: Commit**

```bash
cargo fmt --all
git add -A
git commit -m "feat(gui): add credential prompt dialog"
```

---

## Phase 5: Polish (Tasks 17-19)

### Task 17: Search dialog

**Files:**
- Create: `crates/loom-gui/ui/search-dialog.slint`
- Modify: `crates/loom-gui/src/lib.rs`

Dialog with Base DN (pre-filled), Filter `LineEdit`, Scope `ComboBox` (subtree/one/base), Search/Cancel buttons. Results populate a list. Ctrl+F keyboard shortcut.

**Commit:** `feat(gui): add LDAP search dialog`

---

### Task 18: Export dialog

**Files:**
- Create: `crates/loom-gui/ui/export-dialog.slint`
- Modify: `crates/loom-gui/src/lib.rs`

Dialog with format `ComboBox` (LDIF/JSON/CSV/XLSX), file path `LineEdit`, Export/Cancel. Calls `loom-core` export functions.

**Commit:** `feat(gui): add export dialog`

---

### Task 19: Theme switching from View menu

**Files:**
- Modify: `crates/loom-gui/ui/main.slint`
- Modify: `crates/loom-gui/src/lib.rs`

Add theme selection to the View menu area. On select, call `apply_theme()` and save to config.

**Commit:** `feat(gui): add runtime theme switching`

---

## Phase 6: Final (Task 20)

### Task 20: Integration test — offline mode end-to-end

**Files:**
- Create: `crates/loom-gui/tests/integration.rs`

**Step 1: Write test**

```rust
#[test]
fn test_gui_launches_with_no_config() {
    // Verify MainWindow::new() succeeds with default config
    // Verify status message is "Ready"
    // Verify tabs are empty
}
```

**Step 2: Run tests**

Run: `cargo test -p loom-gui`
Expected: Pass.

**Step 3: Final commit**

```bash
cargo fmt --all
git add -A
git commit -m "test(gui): add integration tests for GUI launch"
```

---

## Task Summary

| Phase | Tasks | Description |
|-------|-------|-------------|
| 1. Foundation | 1-5 | Move config, rename binary, create crates, theme system |
| 2. Layout Shell | 6-9 | Status bar, tab bar, tree view, detail panel, full layout |
| 3. Core Integration | 10-13 | Async bridge, profiles, tree population, attribute display |
| 4. Dialogs | 14-16 | New profile, vault password, credential prompt |
| 5. Polish | 17-19 | Search, export, theme switching |
| 6. Final | 20 | Integration tests |
