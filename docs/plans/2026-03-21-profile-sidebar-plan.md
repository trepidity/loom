# Profile Sidebar Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace the tab bar and connect dialog with a permanent left sidebar for managing LDAP connection profiles, supporting nested folders, labels, multi-connection, and instant switching.

**Architecture:** Add `labels` field to ConnectionProfile. Create a profile-sidebar.slint component with filter, folder tree, and indicators. Remove TabBar and ConnectDialog. Restructure main.slint to a three-column layout (sidebar | tree | detail). Change connection state from single `Option<ConnectionState>` to `HashMap<usize, ConnectionState>` for multi-connection.

**Tech Stack:** Slint 1.14, loom-core config/connection APIs

---

## Task 1: Add labels field to ConnectionProfile

**Files:**
- Modify: `crates/loom-core/src/config.rs`
- Modify: `crates/loom-gui/tests/integration.rs`

**Step 1: Add the field**

In `ConnectionProfile` struct, add:

```rust
#[serde(default, skip_serializing_if = "Vec::is_empty")]
pub labels: Vec<String>,
```

**Step 2: Run tests**

Run: `cargo test --workspace`
Expected: All tests pass — `#[serde(default)]` means existing configs work unchanged.

**Step 3: Commit**

```bash
cargo fmt --all
git add -A
git commit -m "feat: add labels field to ConnectionProfile"
```

---

## Task 2: Create profile-sidebar.slint

**Files:**
- Create: `crates/loom-gui/ui/profile-sidebar.slint`

The sidebar component. This is the largest new component.

**Step 1: Define data structs**

```slint
export struct SidebarProfile {
    name: string,
    host: string,
    index: int,
    labels: string,         // comma-separated for display
    is-connected: bool,
    is-active: bool,
    indent-level: int,      // 0 = top-level, 1+ = nested in folder
    is-folder: bool,
    is-expanded: bool,
}
```

The sidebar uses a flat model (same pattern as the tree view) where folders and profiles are interleaved with indent levels.

**Step 2: Build the component**

Structure:
- Filter LineEdit at top
- ListView with SidebarProfile items
- Each item renders differently based on `is-folder`:
  - Folder: expand arrow + bold name
  - Profile: green dot (if connected), name, host (muted), label badges, highlight if active
- "+ New" button at bottom

Properties:
- `in property <[SidebarProfile]> model: [];`
- `in-out property <string> filter-text: "";`
- `callback profile-clicked(int);` — fires with profile index
- `callback profile-right-clicked(int);` — for context menu
- `callback folder-toggled(int);` — expand/collapse folder
- `callback filter-changed(string);`
- `callback new-profile-clicked();`

Label badges: parse comma-separated labels string, render each as a small colored rectangle with text. Color is derived from label text hash:

```slint
// Simple color assignment based on first character
// a-e = blue, f-j = green, k-o = orange, p-t = red, u-z = purple
```

**Step 3: Verify**

Run: `cargo build -p loom-gui`
Expected: Compiles.

**Step 4: Commit**

```bash
cargo fmt --all
git add -A
git commit -m "feat(gui): add profile sidebar component"
```

---

## Task 3: Remove TabBar and ConnectDialog from main.slint

**Files:**
- Modify: `crates/loom-gui/ui/main.slint`
- Delete: `crates/loom-gui/ui/connect-dialog.slint` (optional — can just stop importing)

**Step 1: Remove imports and properties**

Remove from main.slint:
- `import { TabBar, TabInfo } from "tab-bar.slint";`
- `import { ConnectDialog, ConnectProfile } from "connect-dialog.slint";`
- All tab-related properties: `tabs`, `active-tab`, `tab-clicked`, `close-tab`
- All connect-dialog properties: `connect-dialog-visible`, `connect-profiles`, `connect-dialog-selected`, `connect-dialog-cancel`
- `show-connect-dialog` callback
- The `TabBar { ... }` element from the VerticalLayout
- The `ConnectDialog { ... }` overlay
- Remove "Connect..." MenuItem from the Profiles menu
- Remove "Quit" MenuItem from the Profiles menu

**Step 2: Remove conditional on tabs.length**

The current layout shows empty state when `tabs.length == 0` and tree+detail when `tabs.length > 0`. Replace with a new property:

```slint
in-out property <bool> has-active-connection: false;
```

Change conditionals from `root.tabs.length == 0` to `!root.has-active-connection` and `root.tabs.length > 0` to `root.has-active-connection`.

**Step 3: Verify**

Run: `cargo build -p loom-gui`
Expected: Will fail because lib.rs still references removed types. That's OK — Task 5 fixes lib.rs.

**Step 4: Commit**

```bash
cargo fmt --all
git add -A
git commit -m "refactor(gui): remove tab bar and connect dialog from layout"
```

---

## Task 4: Restructure main.slint with sidebar layout

**Files:**
- Modify: `crates/loom-gui/ui/main.slint`

**Step 1: Add sidebar imports and properties**

```slint
import { ProfileSidebar, SidebarProfile } from "profile-sidebar.slint";
```

Add properties:
```slint
// Sidebar
in-out property <[SidebarProfile]> sidebar-model: [];
in-out property <string> sidebar-filter: "";
in-out property <bool> sidebar-visible: true;
callback sidebar-profile-clicked(int);
callback sidebar-profile-right-clicked(int);
callback sidebar-folder-toggled(int);
callback sidebar-filter-changed(string);
callback sidebar-new-profile();
```

**Step 2: Three-column layout**

Replace the current VerticalLayout content with:

```slint
VerticalLayout {
    HorizontalLayout {
        vertical-stretch: 1;

        // Sidebar
        if root.sidebar-visible: ProfileSidebar {
            width: 220px;
            model: root.sidebar-model;
            filter-text <=> root.sidebar-filter;
            profile-clicked(index) => { root.sidebar-profile-clicked(index); }
            profile-right-clicked(index) => { root.sidebar-profile-right-clicked(index); }
            folder-toggled(index) => { root.sidebar-folder-toggled(index); }
            filter-changed(text) => { root.sidebar-filter-changed(text); }
            new-profile-clicked => { root.sidebar-new-profile(); }
        }

        // Sidebar divider
        if root.sidebar-visible: Rectangle {
            width: 1px;
            background: AppTheme.border;
        }

        // Main content - empty state
        if !root.has-active-connection: Rectangle {
            horizontal-stretch: 1;
            background: AppTheme.bg-primary;
            Text {
                text: "Select a profile to connect.";
                color: AppTheme.fg-muted;
                horizontal-alignment: center;
                vertical-alignment: center;
                font-size: 14px;
            }
        }

        // Main content - tree + detail
        if root.has-active-connection: TreeView {
            horizontal-stretch: 1;
            model: root.tree-model;
            selected-index: root.tree-selected-index;
            toggle-expand(index) => { root.tree-toggle-expand(index); }
            node-selected(index) => { root.tree-node-selected(index); }
        }

        if root.has-active-connection: Rectangle {
            width: 1px;
            background: AppTheme.border;
        }

        if root.has-active-connection: DetailPanel {
            horizontal-stretch: 2;
            attributes: root.attributes;
            entry-dn: root.entry-dn;
        }
    }

    StatusBar {
        message: root.status-message;
        is-error: root.status-is-error;
    }
}
```

**Step 3: Update menu bar**

Remove "Connect..." and "Quit" from Profiles menu. Add "Toggle Sidebar" to View menu:

```slint
Menu {
    title: "View";
    MenuItem {
        title: "Toggle Sidebar";
        activated => { root.sidebar-visible = !root.sidebar-visible; }
    }
    MenuItem {
        title: "Theme...";
        activated => { root.show-theme-selector(); }
    }
}
```

**Step 4: Verify**

Run: `cargo build -p loom-gui`
Expected: May still fail due to lib.rs. That's OK.

**Step 5: Commit**

```bash
cargo fmt --all
git add -A
git commit -m "feat(gui): add three-column layout with profile sidebar"
```

---

## Task 5: Rewrite lib.rs for multi-connection sidebar

**Files:**
- Modify: `crates/loom-gui/src/lib.rs`

This is the largest task. The connection model changes from single to multi.

**Step 1: Update ConnectionState and add multi-connection map**

```rust
use std::collections::HashMap;

struct ConnectionState {
    conn: LdapConnection,
    tree_meta: Vec<TreeNodeMeta>,
    tree_nodes: Vec<TreeNode>,
    attributes: Vec<AttributeRow>,
    entry_dn: SharedString,
    selected_index: i32,
}

// In run():
let connections: Rc<RefCell<HashMap<usize, ConnectionState>>> = Rc::new(RefCell::new(HashMap::new()));
let active_profile: Rc<RefCell<Option<usize>>> = Rc::new(RefCell::new(None));
```

**Step 2: Build sidebar model from config**

On startup, build the sidebar model from config.connections. Parse folder paths into a flat list with indent levels:

```rust
fn build_sidebar_model(config: &AppConfig, connections: &HashMap<usize, ConnectionState>, active: Option<usize>) -> Vec<SidebarProfile> {
    // 1. Collect all unique folder paths, sort
    // 2. For each folder level, emit a folder entry with indent
    // 3. For each profile in that folder, emit a profile entry with indent+1
    // 4. Emit ungrouped profiles at indent 0
    // Mark is-connected and is-active based on connections map and active_profile
}
```

**Step 3: Wire sidebar callbacks**

- `on_sidebar_profile_clicked(index)`:
  1. Look up which profile this index refers to (skip folders)
  2. If already connected: save current state, load this profile's state, update UI
  3. If not connected: start async connection, on success add to map and show

- `on_sidebar_folder_toggled(index)`:
  1. Toggle expanded state for this folder
  2. Rebuild sidebar model (insert/remove children)

- `on_sidebar_filter_changed(text)`:
  1. Rebuild sidebar model filtered by text (match name, folder, labels)

- `on_sidebar_new_profile()`:
  1. Same as existing new-profile-requested handler

**Step 4: Wire menu disconnect**

- `on_menu_disconnect()`:
  1. If active profile exists: drop its ConnectionState from map
  2. Clear tree/detail UI
  3. Set active_profile to None or switch to another open connection
  4. Rebuild sidebar model

**Step 5: Remove all tab-related code**

Remove:
- `tabs_model`, `on_tab_clicked`, `on_close_tab`
- All references to `TabInfo`
- The `show-connect-dialog` handler
- The `connect-dialog-selected` and `connect-dialog-cancel` handlers

**Step 6: Update connect-profile handler**

The existing `on_connect_profile(index)` and `spawn_connect` need to:
- Store the new ConnectionState in the HashMap (not a single Option)
- Set this as the active profile
- Rebuild the sidebar model to show the green dot

**Step 7: Update tree/detail callbacks**

The `on_tree_toggle_expand`, `on_tree_node_selected` callbacks need to use the active profile's connection from the HashMap instead of the single Option.

**Step 8: Verify**

Run: `cargo build -p loom-browser`
Expected: Compiles.

Run: `cargo test --workspace`
Expected: All tests pass.

**Step 9: Commit**

```bash
cargo fmt --all
git add -A
git commit -m "feat(gui): wire multi-connection sidebar with instant switching"
```

---

## Task 6: Add labels to profile dialog

**Files:**
- Modify: `crates/loom-gui/ui/profile-dialog.slint`
- Modify: `crates/loom-gui/ui/main.slint` (add labels property to dialog)
- Modify: `crates/loom-gui/src/lib.rs` (include labels in save)

**Step 1: Add labels LineEdit to profile-dialog.slint**

Add a new field after Folder:
- Label: "Labels (comma-separated)"
- LineEdit for labels input
- Property: `in-out property <string> labels: "";`

**Step 2: Wire in main.slint**

Add `profile-dialog-labels` property, bind to dialog.

**Step 3: Wire in lib.rs**

When saving profile, split labels string by comma, trim whitespace, store as `Vec<String>`.

Update `save-profile` callback signature to include labels (9th argument), or parse from a property.

**Step 4: Verify and commit**

```bash
cargo fmt --all
git add -A
git commit -m "feat(gui): add labels field to profile dialog"
```

---

## Task 7: Clean up deleted files and verify

**Files:**
- Delete: `crates/loom-gui/ui/tab-bar.slint`
- Delete: `crates/loom-gui/ui/connect-dialog.slint`

**Step 1: Delete unused files**

Remove tab-bar.slint and connect-dialog.slint if they're no longer imported anywhere.

**Step 2: Full verification**

Run: `cargo build -p loom-browser`
Run: `cargo test --workspace`
Run: `cargo clippy --workspace -- -D warnings`
Expected: All clean.

**Step 3: Commit**

```bash
cargo fmt --all
git add -A
git commit -m "chore: remove unused tab-bar and connect-dialog components"
```

---

## Task Summary

| Task | Description | Complexity |
|------|-------------|------------|
| 1 | Add labels to ConnectionProfile | Small |
| 2 | Create profile-sidebar.slint | Large — main new component |
| 3 | Remove TabBar/ConnectDialog from main.slint | Medium |
| 4 | Restructure main.slint with sidebar layout | Medium |
| 5 | Rewrite lib.rs for multi-connection | Large — biggest change |
| 6 | Add labels to profile dialog | Small |
| 7 | Clean up and verify | Small |
