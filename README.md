# loom-ldapbrowser

A terminal-based LDAP browser built with Rust.

Browse, search, edit, and manage LDAP directories from the comfort of your terminal.

## Features

- Tree browser with vim-style navigation
- Attribute viewing and inline editing
- Multi-tab connections
- LDAP filter search
- Create, delete, and bulk-update entries
- Export/import in LDIF, JSON, CSV, and XLSX
- Schema viewer for object classes and attribute types
- Connection profiles with folder organization
- Credential support: interactive prompt, shell command, or OS keychain
- TLS: auto-negotiation, LDAPS, StartTLS, or plaintext
- Server detection: OpenLDAP, Active Directory, and others
- 5 built-in themes: dark, light, solarized, nord, matrix
- Mouse support and context menus
- Offline demo mode

## Installation

### From source

Requires Rust 1.80+.

```bash
cargo install --path crates/loom-ldapbrowser
```

### Prebuilt binaries

Download from [GitHub Releases](https://github.com/trepidity/loom-ldapbrowser/releases) for Linux (x86_64, aarch64), macOS (Intel, Apple Silicon), and Windows (x86_64).

## Quick Start

```bash
# Connect directly
loom-ldapbrowser -H ldap.example.com -D "cn=admin,dc=example,dc=com" -b "dc=example,dc=com"

# Or use saved profiles
loom-ldapbrowser
```

Press `Ctrl+T` to open the connection dialog. Press `F5` or `?` for help.

## Configuration

loom-ldapbrowser reads `~/.config/loom-ldapbrowser/config.toml`. See the [User Manual](USER_MANUAL.md) for full configuration reference.

```toml
[general]
theme = "dark"

[[connections]]
name = "Production"
host = "ldap.example.com"
port = 389
tls_mode = "auto"
bind_dn = "cn=admin,dc=example,dc=com"
base_dn = "dc=example,dc=com"
credential_method = "prompt"
```

## Documentation

See the **[User Manual](USER_MANUAL.md)** for complete documentation including keybindings, configuration options, theming, and all features.

## Development

```bash
cargo check --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo fmt --all
```

## License

Licensed under the [GNU General Public License v3.0](LICENSE.txt).
