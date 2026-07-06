# SQLiteForge

> A modern terminal-first SQLite client built with Rust

<p align="center">
  <strong>Fast startup · Syntax highlighting · Autocompletion · Multiple output modes · Persistent history</strong>
</p>

---

## 📖 Tutorial: Build Your Own SQLiteForge

Curious how this was built? This repository includes a complete, step-by-step tutorial reconstructing the entire codebase from scratch.

[**Read the "Build Your Own SQLiteForge" Tutorial**](build-your-own-sqliteforge/00-index.md)

---

## Features

- 🎨 **SQL Syntax Highlighting** — Keywords, strings, numbers, comments are color-coded
- ⚡ **Tab Autocompletion** — SQL keywords, table names, column names, view names, indices, and dot commands
- 📝 **Multi-line Editing** — Write complex queries naturally with automatic continuation
- 📊 **7 Output Modes** — `box`, `table`, `column`, `markdown`, `csv`, `json`, `list`
- 🕐 **Persistent History** — All queries saved to `~/.local/share/sqliteforge/history.db`
- 🔍 **Reverse Search** — `Ctrl+R` to fuzzy search through history
- 🗂️ **Database Explorer** — `Ctrl+E` to browse tables, views, and indices
- 📋 **Dot Commands** — SQLite-compatible `.tables`, `.schema`, `.dump`, `.mode`, and more
- ⚙️ **TOML Config** — Customize via `~/.config/sqliteforge/config.toml`
- 📈 **Query Statistics** — Rows returned and execution time for every query

---

## Quick Start

### Running Without Installing

```bash
# Build in debug mode (faster compile)
cargo build

# Run directly
./target/debug/sqliteforge mydb.db

# Or use cargo run
cargo run -- mydb.db
```

### Running in Release Mode (Recommended)

```bash
# Build optimized binary
cargo build --release

# Run it
./target/release/sqliteforge mydb.db
```

---

## Installation

### System-wide Installation

```bash
# 1. Build release binary
cargo build --release

# 2. Copy to PATH
sudo install -m 755 target/release/sqliteforge /usr/local/bin/sqliteforge

# 3. Create config directory
mkdir -p ~/.config/sqliteforge

# 4. Create default config
cat > ~/.config/sqliteforge/config.toml <<EOF
theme = "catppuccin"
mode = "box"
headers = true
history = true
EOF

# 5. Verify
sqliteforge --version
```

### Cargo Install

```bash
cargo install --path .
```

This installs to `~/.cargo/bin/` (make sure it's in your `PATH`).

---

## Uninstallation

```bash
# Remove binary
sudo rm /usr/local/bin/sqliteforge

# Remove config (optional)
rm -rf ~/.config/sqliteforge

# Remove history (optional)
rm -rf ~/.local/share/sqliteforge
```

---

## Usage

### Interactive Mode

```bash
# Open or create a database
sqliteforge mydb.db

# In-memory database
sqliteforge

# With a specific output mode
sqliteforge mydb.db -m json
```

### Non-Interactive Mode

```bash
# Execute a query
sqliteforge mydb.db -c "SELECT * FROM users"

# Execute a SQL file
sqliteforge mydb.db -f schema.sql

# Pipe output
sqliteforge mydb.db -m csv -c "SELECT * FROM users" > export.csv
```

### Dot Commands

| Command | Description |
|---------|-------------|
| `.help` | Show all commands |
| `.tables` | List all tables |
| `.schema [TABLE]` | Show CREATE statements |
| `.indices` | List all indices |
| `.mode MODE` | Set output mode (box/table/column/markdown/csv/json/list) |
| `.headers on\|off` | Toggle column headers |
| `.preview TABLE` | Show first 20 rows |
| `.read FILE` | Execute SQL script |
| `.output [FILE]` | Redirect output to file |
| `.dump` | Dump entire database as SQL |
| `.nullvalue STRING` | Set NULL display string |
| `.show` | Show current settings |
| `.quit` / `.exit` | Exit |

### Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Tab` | Autocomplete |
| `↑` / `↓` | Navigate history |
| `Ctrl+R` | Reverse search history |
| `Ctrl+E` | Toggle database explorer |
| `Ctrl+L` | Clear screen |
| `Ctrl+C` | Cancel current input |
| `Ctrl+D` | Exit |

---

## Configuration

Edit `~/.config/sqliteforge/config.toml`:

```toml
# Color theme
theme = "catppuccin"

# Default output mode: box, table, column, markdown, csv, json, list
mode = "box"

# Show column headers
headers = true

# Enable persistent query history
history = true
```

---

## Project Structure

```
src/
├── main.rs           # Entry point, CLI argument parsing
├── app/mod.rs        # Application orchestrator
├── shell/
│   ├── mod.rs        # Interactive shell (reedline integration)
│   ├── highlighter.rs # SQL syntax highlighting
│   ├── validator.rs  # Multi-line input validation
│   └── prompt.rs     # Custom prompt rendering
├── commands/mod.rs   # Dot command handler
├── completion/mod.rs # SQL autocompletion engine
├── database/mod.rs   # SQLite connection wrapper
├── renderer/mod.rs   # Output formatting (7 modes)
├── history/mod.rs    # Persistent query history
├── config/mod.rs     # TOML configuration
└── explorer/mod.rs   # Database explorer panel
```

## Tech Stack

| Component | Library |
|-----------|---------|
| Language | Rust |
| Database | rusqlite (bundled SQLite) |
| Line Editor | reedline |
| Config | serde + toml |
| CLI Args | clap |

---

## License

MIT
