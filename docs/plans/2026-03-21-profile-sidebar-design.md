# Profile Sidebar Design

## Overview

Replace the tab bar and connect dialog with a permanent left sidebar for managing and switching between LDAP connection profiles. Supports nested folders, labels, multi-connection, and instant switching.

## Config Changes

One new optional field on `ConnectionProfile`:

```toml
[[connections]]
name = "AD Prod"
host = "ldap.acme.com"
folder = "Acme Corp/East Region"   # slash-separated = nested folders
labels = ["prod", "active-directory"]
```

- `labels`: `Vec<String>`, optional, defaults to empty. User-defined tags shown as colored badges.
- `folder`: already exists. Slash-separated paths create nested groups in the sidebar. `"Acme Corp/East Region"` → Acme Corp > East Region.

## Layout

```
+-------------------------------------------------------------+
|  Profiles  Edit  View  Help                     (OS menu)   |
+-------------+-----------------------------------------------+
| Filter      |                                               |
+-------------|  +-------------+--------------------------+   |
|             |  | DIT Tree    | Attribute Detail          |   |
| > Acme Corp |  |             |                           |   |
|  *AD Prod < |  | dc=acme     | dn: cn=user1,...          |   |
|  *AD Test   |  |  +- ou=usr  | cn: user1                |   |
|  > East     |  |  +- ou=grp  | mail: u@acme.com         |   |
|    RDL Dev  |  |             |                           |   |
|             |  |             |                           |   |
| > Contoso   |  |             |                           |   |
|   LDAP Prod |  |             |                           |   |
|             |  +-------------+--------------------------+   |
|             |                                               |
| [+ New]     |                                               |
+-------------+-----------------------------------------------+
|  Connected to ldap.acme.com                                 |
+-------------------------------------------------------------+
```

- `*` = green dot (open connection)
- `<` = currently viewed connection
- Three-column split: sidebar | DIT tree | attribute detail
- All dividers are draggable
- Sidebar default width ~200px

## Sidebar Component

### Structure

- **Filter bar** at top: LineEdit, filters across name, folder, and labels as you type
- **Profile tree**: nested folders (collapsible) with profiles as leaves
- **Badges**: labels shown as small colored tags next to profile name. Colors auto-assigned by hashing label text to a small palette.
- **Indicators**: green dot = open connection, highlight/arrow = currently viewed
- **"+ New" button** at bottom: opens new profile dialog

### Interactions

- **Double-click** closed profile: connect, then show
- **Single-click** open profile: switch view to that connection (instant, no reconnect)
- **Single-click** closed profile: connect, then show
- **Right-click** any profile: context menu (Connect, Edit, Duplicate, Delete, Disconnect if open)
- **Folder collapse**: click folder arrow to expand/collapse

### Filter

Matches against profile name, folder path, and labels. Case-insensitive substring match. Instant — filters as you type. When filtering, folder structure is flattened to show only matching profiles.

## Multi-Connection Model

Multiple profiles can be connected simultaneously. Each maintains independent state.

### Connection State

```rust
struct ConnectionState {
    conn: LdapConnection,
    tree_meta: Vec<TreeNodeMeta>,
    tree_nodes: Vec<TreeNode>,      // preserved tree state
    attributes: Vec<AttributeRow>,  // preserved detail state
    entry_dn: String,               // preserved selected DN
    selected_index: i32,            // preserved tree selection
}

// Map of profile index to connection state
connections: HashMap<usize, ConnectionState>
active_profile: Option<usize>
```

### Switching

When user clicks a different open profile:
1. Save current tree/detail state into the map
2. Load the target profile's state
3. Update the Slint models (tree, attributes, entry-dn)
4. Update sidebar indicators

This is instant — no network calls, just swapping UI state.

### Connecting

When user clicks a closed profile:
1. Start async connection (same flow as today)
2. On success: store ConnectionState in map, set as active, update sidebar
3. On failure: show error in status bar, no state change

### Disconnecting

Right-click > Disconnect:
1. Drop the LdapConnection
2. Remove from map
3. If it was the active profile: switch to another open profile, or show empty state
4. Update sidebar indicator (remove green dot)

## Menu Bar Changes

**Profiles menu:**
- New Profile...
- Disconnect
- (no Connect..., no Quit)

**View menu:**
- Theme...
- Toggle Sidebar

## Components Removed

- `tab-bar.slint` — deleted
- `connect-dialog.slint` — deleted
- TabBar component from main.slint
- Tab-related properties and callbacks
- `tabs` model, `active-tab`, `tab-clicked`, `close-tab`

## Components Added

- `profile-sidebar.slint` — new sidebar component with filter, tree, labels, indicators

## Components Modified

- `main.slint` — three-column layout (sidebar | tree | detail), no tab bar
- `lib.rs` — HashMap connection state, sidebar callbacks, switching logic
- `config.rs` — add `labels: Vec<String>` field to ConnectionProfile
