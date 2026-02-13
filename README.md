# Loom

A terminal-based LDAP browser built with Rust.

Browse, search, edit, and manage LDAP directories from the comfort of your terminal. Loom provides a full-featured TUI with vim-style navigation, multi-tab connections, and bulk operations.

![Main browsing view](docs/screenshots/browse.png)

## Features

- **Tree browser** -- Navigate the directory hierarchy with vim keybindings or arrow keys
- **Attribute viewer** -- Inspect and edit entry attributes in a detail panel
- **Multi-tab** -- Open multiple connections side by side
- **Search** -- LDAP filter search with attribute selection
- **Create & delete entries** -- Add children or remove entries directly from the tree
- **Bulk update** -- Apply modifications across entries matching a filter
- **Export / Import** -- LDIF, JSON, CSV, and XLSX formats
- **Schema viewer** -- Browse object classes and attribute type definitions
- **Themes** -- Dark, light, solarized, and nord built-in themes
- **Credentials** -- Interactive prompt, shell command (`pass`, `op`, etc.), or OS keychain
- **Server detection** -- Identifies OpenLDAP, Active Directory, and other vendors
- **TLS** -- Auto-negotiation, LDAPS, StartTLS, or plaintext
- **Mouse support** -- Click to focus panels
- **Reconnect** -- Automatic reconnection with cached credentials

## Screenshots

> **Note:** Replace these placeholders with actual screenshots.
> Start the test LDAP server (`docker compose -f tests/integration/docker-compose.yml up -d`),
> run `cargo run -p loom -- -H localhost -p 3389 -D cn=admin,dc=example,dc=com -b dc=example,dc=com`,
> and capture the terminal.

| View | Screenshot |
|------|-----------|
| Browsing tree + attributes | ![browse](docs/screenshots/browse.png) |
| Search results | ![search](docs/screenshots/search.png) |
| Create entry dialog | ![create](docs/screenshots/create-entry.png) |
| Delete confirmation | ![delete](docs/screenshots/delete-confirm.png) |
| Export dialog | ![export](docs/screenshots/export.png) |
| Schema viewer | ![schema](docs/screenshots/schema.png) |
| Connection dialog | ![connect](docs/screenshots/connect.png) |
| Nord theme | ![nord](docs/screenshots/theme-nord.png) |

## Installation

### From source

Requires Rust 1.80 or later.

```bash
cargo install --path crates/loom
```

### From release binaries

Download a prebuilt binary from [GitHub Releases](https://github.com/your-org/loom/releases) for:

| Platform | Architecture |
|----------|-------------|
| Linux | x86_64, aarch64 |
| macOS | x86_64 (Intel), aarch64 (Apple Silicon) |
| Windows | x86_64 |

### Build from source

```bash
git clone https://github.com/your-org/loom.git
cd loom
cargo build --release
# Binary at target/release/loom
```

## Quick Start

```bash
# Connect with CLI arguments
loom -H ldap.example.com -D "cn=admin,dc=example,dc=com" -b "dc=example,dc=com"

# Or configure a profile and just run:
loom
```

On first launch with no config, Loom shows the connection dialog (`Ctrl+T`).

## Configuration

Loom reads `~/.config/loom/config.toml`. Create it manually or press `Ctrl+T` to connect ad-hoc, then `Ctrl+W` to save the connection.

```toml
[general]
theme = "dark"               # dark | light | solarized | nord
tick_rate_ms = 250
log_level = "info"

[defaults]
page_size = 500
timeout_secs = 30
tls_mode = "auto"            # auto | ldaps | starttls | none
referral_policy = "ignore"

[export]
csv_delimiter = ","
csv_multivalue_separator = ";"
json_pretty = true
ldif_line_length = 76

[[connections]]
name = "Production"
host = "ldap.example.com"
port = 389
tls_mode = "auto"
bind_dn = "cn=admin,dc=example,dc=com"
base_dn = "dc=example,dc=com"
credential_method = "prompt"    # prompt | command | keychain
# password_command = "pass show ldap/prod"  # for credential_method = "command"
page_size = 500
timeout_secs = 30
relax_rules = false

[[connections]]
name = "Staging"
host = "ldap-staging.internal"
port = 636
tls_mode = "ldaps"
bind_dn = "cn=readonly,dc=staging,dc=com"
base_dn = "dc=staging,dc=com"
credential_method = "keychain"
```

### Credential methods

| Method | Description |
|--------|------------|
| `prompt` | Password prompt in the TUI. Also reads `LOOM_PASSWORD` env var if set. |
| `command` | Runs `password_command` and reads stdout. Works with `pass`, `op`, `gpg`, etc. |
| `keychain` | Uses the OS keychain (macOS Keychain, Linux Secret Service, Windows Credential Manager) via the `keyring` crate. |

### TLS modes

| Mode | Behavior |
|------|----------|
| `auto` | Try LDAPS (636), fall back to StartTLS, then plaintext |
| `ldaps` | LDAPS on port 636 |
| `starttls` | StartTLS upgrade on port 389 |
| `none` | Plaintext (no encryption) |

## Keybindings

### Global

| Key | Action |
|-----|--------|
| `q` / `Ctrl+C` | Quit |
| `Tab` | Focus next panel |
| `Shift+Tab` | Focus previous panel |
| `/` | Focus search input |
| `Ctrl+T` | Open connection dialog |
| `Ctrl+N` | Next tab |
| `Ctrl+P` | Previous tab |
| `Ctrl+E` | Export dialog |
| `Ctrl+B` | Bulk update dialog |
| `Ctrl+S` | Schema viewer |
| `Ctrl+L` | Toggle log panel |
| `Ctrl+W` | Save ad-hoc connection to config |

### Tree panel

| Key | Action |
|-----|--------|
| `j` / `Up` | Move up |
| `k` / `Down` | Move down |
| `l` / `Right` / `Enter` | Expand / select entry |
| `h` / `Left` | Collapse |
| `a` | Create new entry (under selected node) |
| `d` / `Delete` | Delete selected entry (with confirmation) |

### Detail panel

| Key | Action |
|-----|--------|
| `r` | Refresh entry |

### Dialogs

| Key | Action |
|-----|--------|
| `Tab` / `Shift+Tab` | Cycle fields |
| `Enter` | Submit |
| `Esc` | Cancel / close |
| `y` / `n` | Confirm / deny (confirmation dialogs) |

## Export & Import

Loom supports four formats, auto-detected by file extension:

| Format | Extensions | Notes |
|--------|-----------|-------|
| LDIF | `.ldif`, `.ldf` | RFC 2849 compliant |
| JSON | `.json` | Array of entry objects |
| CSV | `.csv` | One row per entry, multivalues joined by separator |
| Excel | `.xlsx`, `.xls` | Spreadsheet with header row |

Open the export dialog with `Ctrl+E`, set a search filter and attributes, choose format and filename, then press Enter.

## Architecture

Loom is organized as a Cargo workspace with three crates:

```
crates/
  loom/          Binary -- CLI parsing + entry point
  loom-core/     Library -- LDAP operations, export/import, schema, DN utilities
  loom-tui/      Library -- TUI framework, components, themes, keybindings
```

All state changes flow through an `Action` enum dispatched via an async mpsc channel. LDAP operations run in background Tokio tasks and send results back as actions, keeping the UI responsive.

```
User Input --> KeyEvent --> Action --> process_action() --> spawn async task
                                                               |
                                                               v
                                                         LDAP operation
                                                               |
                                                               v
                                                     Action (result) --> UI update
```

## Development

```bash
# Check all crates
cargo check --workspace

# Run tests (90 tests: 57 core + 7 integration + 26 TUI)
cargo test --workspace

# Lint
cargo clippy --workspace -- -D warnings

# Format
cargo fmt --all
```

### Integration tests with Docker

Start a local OpenLDAP server, then run the ignored integration tests:

```bash
docker compose -f tests/integration/docker-compose.yml up -d
cargo test --test ldap_integration -- --ignored
```

The Docker container provides:
- **Host:** localhost:3389 (LDAP), localhost:3636 (LDAPS)
- **Admin:** `cn=admin,dc=example,dc=com` / `admin`
- **Sample data:** Users (Alice, Bob), Groups (admins), OUs

### Project structure

```
.
├── Cargo.toml                    Workspace root
├── crates/
│   ├── loom/                     Binary crate
│   ├── loom-core/                LDAP client library
│   │   └── src/
│   │       ├── connection.rs     Connect, TLS, bind
│   │       ├── search.rs         Search operations
│   │       ├── modify.rs         Add, modify, delete entries
│   │       ├── bulk.rs           Bulk modifications
│   │       ├── schema.rs         Schema parsing
│   │       ├── tree.rs           Directory tree model
│   │       ├── entry.rs          LDAP entry model
│   │       ├── dn.rs             DN parsing utilities
│   │       ├── filter.rs         LDAP filter validation
│   │       ├── export/           LDIF, JSON, CSV, XLSX writers
│   │       └── import/           LDIF, JSON, CSV, XLSX readers
│   └── loom-tui/                 TUI framework
│       └── src/
│           ├── app.rs            Main app loop + action dispatch
│           ├── keymap.rs         Keybinding resolution
│           ├── theme.rs          Theme system
│           ├── config.rs         Configuration loading
│           └── components/       UI components (panels, dialogs)
├── config/
│   ├── default.toml              Default settings
│   └── themes/                   Built-in theme files
├── tests/
│   ├── fixtures/                 Sample LDIF/JSON/CSV data
│   └── integration/              Docker compose for OpenLDAP
└── .github/workflows/
    ├── ci.yml                    Check, test, clippy, fmt, MSRV
    └── release.yml               Multi-platform release builds
```

## License

Licensed under either of:

- [Apache License, Version 2.0](http://www.apache.org/licenses/LICENSE-2.0)
- [MIT License](http://opensource.org/licenses/MIT)

at your option.
