# SQLiteForge v1.0 Requirements

## Overview

SQLiteForge is a modern terminal-first SQLite client focused exclusively on SQLite.

The goal of v1.0 is to provide a superior SQLite shell experience while maintaining compatibility with existing SQLite workflows.

SQLiteForge is not intended to be a database IDE in v1.0.

Primary focus:

* Fast startup
* Keyboard-first workflow
* SQLite compatibility
* Intelligent autocompletion
* Rich output formatting
* Persistent history
* Terminal-native experience

---

# Scope

## Included in v1.0

* Interactive SQL shell
* SQLite-compatible dot commands
* SQL autocompletion
* Dot-command autocompletion
* Persistent query history
* Reverse history search
* Multi-line query editing
* Multiple output modes
* Schema inspection
* Table preview
* Database explorer panel
* Configuration system
* Query execution statistics

## Excluded from v1.0

* Multiple database connections
* Query tabs
* Plugin system
* AI features
* PostgreSQL support
* MySQL support
* Query execution plans
* Import wizard
* Spreadsheet-style editing
* Visual schema diagrams
* Vim mode

---

# SQL Compatibility

SQLiteForge shall execute standard SQLite SQL statements.

Examples:

```sql
SELECT * FROM "books";

INSERT INTO "books" ("title")
VALUES ('CS50 SQL');

UPDATE "books"
SET "title" = 'SQLite'
WHERE "id" = 1;
```

## Identifier Rules

SQLiteForge shall generate and display identifiers using SQL-standard double quotes.

Examples:

```sql
SELECT * FROM "books";

SELECT "id", "title"
FROM "books";
```

Backticks and square brackets may be accepted as input because SQLite supports them, but generated SQL should always use double quotes.

---

# Interactive Shell

SQLiteForge shall provide an interactive shell.

Example:

```bash
sqliteforge library.db
```

Users shall be able to:

* Execute SQL statements
* Navigate history
* Use autocompletion
* Run dot commands
* Execute scripts

---

# Dot Command Support

SQLiteForge shall support SQLite-style dot commands.

Minimum required commands:

```text
.help
.quit
.exit
.tables
.schema
.indices
.mode
.headers
.read
.output
.dump
.nullvalue
.show
```

Additional commands may be added later.

---

# Output Modes

SQLiteForge shall support:

```text
.mode box
.mode table
.mode column
.mode markdown
.mode csv
.mode json
.mode list
```

Default mode:

```text
.mode box
```

Example:

```text
┌────┬─────────────┐
│ id │ title       │
├────┼─────────────┤
│ 1  │ SQLite      │
│ 2  │ CS50 SQL    │
└────┴─────────────┘
```

---

# SQL Autocompletion

Autocomplete shall be available using Tab.

## Keywords

Input:

```sql
SEL
```

Output:

```sql
SELECT
```

## Tables

Input:

```sql
SELECT * FROM "bo
```

Suggestions:

```text
books
book_reviews
bookmarks
```

## Columns

Input:

```sql
SELECT "tit
```

Suggestions:

```text
title
```

---

# Dot Command Autocompletion

Input:

```text
.sch
```

Output:

```text
.schema
```

---

# Completion Menu

When multiple matches exist, a selection menu shall be displayed.

Example:

```text
books
book_reviews
bookmarks
```

Users shall be able to:

* Navigate with arrow keys
* Select with Enter
* Cancel with Escape

Completion behavior:

* SQL keywords shall be inserted as plain text.
* Standard SQL identifiers such as table names, view names, column names, and index names shall be inserted using double quotes.
* Dot commands shall be inserted without quotes.

Example:

Selecting `books` from the completion menu while typing:

```sql
SELECT * FROM 
```

shall produce:

```sql
SELECT * FROM "books"
```

---

# Multi-Line Query Editing

Users shall be able to write multi-line SQL.

Example:

```sql
SELECT
    "title",
    "author"
FROM "books"
WHERE "year" > 2020;
```

Execution shall occur only after statement completion.

The editor shall provide automatic indentation for multi-line input.

Examples:

When typing:

```sql
CREATE TABLE "books" (
```

and pressing Enter, the cursor shall move to the next line with increased indentation:

```sql
CREATE TABLE "books" (
    |
```

When closing a block:

```sql
CREATE TABLE "books" (
    "id" INTEGER PRIMARY KEY
)
```

indentation shall automatically decrease as appropriate.

The editor should handle common SQL structures such as:

* Parentheses
* Nested queries
* CREATE TABLE statements
* CASE expressions

without requiring manual indentation adjustments for typical usage.

---

# Query History

Executed queries shall be stored locally.

Features:

* Persistent storage
* Searchable history
* Previous query navigation
* History replay

Storage location:

```text
~/.local/share/sqliteforge/history.db
```

---

# Reverse Search

Shortcut:

```text
Ctrl+R
```

Features:

* Fuzzy search
* Incremental filtering
* Execute selected query

---

# Database Explorer

Optional explorer panel.

Shortcut:

```text
Ctrl+E
```

Displays:

```text
Tables
├── books
├── authors

Views
├── active_books

Indexes
├── idx_books_title
```

Users shall be able to inspect objects directly from the explorer.

---

# Schema Inspection

Command:

```text
.schema "books"
```

Output:

```sql
CREATE TABLE "books" (
    "id" INTEGER PRIMARY KEY,
    "title" TEXT NOT NULL
);
```

---

# Table Preview

Command:

```text
.preview "books"
```

Equivalent to:

```sql
SELECT * FROM "books" LIMIT 20;
```

---

# Query Statistics

After execution SQLiteForge shall display:

* Rows returned
* Execution duration

Example:

```text
Rows: 42
Execution Time: 2.4 ms
```

---

# Configuration

Configuration file:

```text
~/.config/sqliteforge/config.toml
```

Example:

```toml
theme = "catppuccin"
mode = "box"
headers = true
history = true
```

---

# Keyboard Shortcuts

## Navigation

```text
Up Arrow      Previous History
Down Arrow    Next History
Ctrl+R        Reverse Search
Tab           Autocomplete
Shift+Tab     Previous Suggestion
Ctrl+E        Toggle Explorer
Ctrl+L        Clear Screen
Ctrl+C        Cancel Input
Ctrl+D        Exit
```

---

# Technical Requirements

## Language

Rust

## Database Driver

rusqlite

## Terminal

crossterm

## UI

ratatui

## Line Editing

reedline

## Configuration

serde + toml

## Storage

SQLite

---

# Project Structure

```text
src/
├── app/
├── shell/
├── commands/
├── completion/
├── database/
├── renderer/
├── history/
├── config/
├── explorer/
└── main.rs
```

---

# Success Criteria

A user shall be able to:

1. Open a SQLite database.
2. Execute SQL queries.
3. Use autocompletion.
4. Browse tables.
5. Inspect schemas.
6. Search query history.
7. Use SQLite-style dot commands.
8. Export results through output modes.
9. Work entirely from the terminal.

Without requiring a mouse or external GUI application.
