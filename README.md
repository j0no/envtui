# envtui

A terminal UI application for viewing environment variables from .env files and system environment.

## Features

- View `.env` files from the current directory
- View system environment variables
- Dual-panel layout with sidebar and content area
- Navigate with keyboard controls

## Controls

| Key | Action |
|-----|--------|
| `Tab` | Switch between sidebar and content panel |
| `↑` / `↓` | Navigate (move up/down in focused panel) |
| `PageUp` / `PageDown` | Fast scroll |
| `Ctrl+C` | Quit |

## Building

```bash
cargo build --release
```

## Running

```bash
cargo run
```

Or run the built binary:

```bash
./target/debug/envtui
```

## Dependencies

- [opentui_rust](https://crates.io/crates/opentui_rust) - Terminal rendering
- [crossterm](https://crates.io/crates/crossterm) - Terminal input handling
