# Build Your Own SQLiteForge

[SQLiteForge](https://github.com/) is a terminal-first SQLite client written in
Rust: syntax highlighting, context-aware autocompletion, multi-line editing,
seven output formats, persistent history, and a little ASCII-art database
explorer, all wrapped around [rusqlite](https://github.com/rusqlite/rusqlite)
and [reedline](https://github.com/nushell/reedline).

This tutorial reconstructs it from an empty `cargo new`, one small, working
step at a time, in the style of
[Build Your Own Text Editor](https://viewsourcecode.org/snaptoken/kilo/) (the
"kilo" tutorial). Every step in every chapter was actually built and run in a
scratch crate while writing this — where something doesn't compile, or
doesn't do anything visible yet, we say so instead of pretending otherwise.

**A note on history.** SQLiteForge's real git history is a single ~4,800-line
"initial commit" followed by three cleanup commits — not a step-by-step
build. So instead of mining commits, this tutorial *reconstructs* a sane
build order: the one where each step is small, compiles (or we tell you why
it doesn't yet), and is motivated by the step before it. The end state
matches the real `src/` tree; the path there is our own.

**What you'll need.** Rust and Cargo (stable), and a real terminal for the
chapters from 6 onward — reedline takes over raw terminal input, so it won't
behave sensibly when piped or redirected. Every code listing is a diff
against the previous step; unchanged code is collapsed to `{ ... }`.

## Table of Contents

0. [Architecture](01-architecture.md) — the shape of the finished program before we build it: nine modules, their dependencies, and why each library was chosen.
1. [The Skeleton](02-the-skeleton.md) — `clap`, a `Cli` struct, and a program that parses its own arguments.
2. [Talking to SQLite](03-talking-to-sqlite.md) — the `Database` wrapper: opening a file, running a query, telling SELECT apart from everything else.
3. [Making Output Readable](04-making-output-readable.md) — the `Renderer`, all seven output modes, and a fully working non-interactive `-c` / `-f` mode.
4. [Configuration and the App](05-configuration-and-app.md) — a TOML config file with sane defaults, and the `App` struct that ties config to database.
5. [A Real Interactive Shell](06-a-real-interactive-shell.md) — swapping nothing for `reedline`: a prompt, a read loop, Ctrl+C and Ctrl+D.
6. [Multi-line Queries](07-multiline-queries.md) — a `Validator` that knows a SQL statement isn't finished until it sees a semicolon.
7. [Dot Commands](08-dot-commands.md) — `.tables`, `.schema`, `.mode`, and the enum that lets them talk back to the shell.
8. [Persistent History](09-persistent-history.md) — a SQLite-backed log of every query, and reedline's own history file for Up/Down and Ctrl+R.
9. [Syntax Highlighting](10-syntax-highlighting.md) — a hand-rolled SQL tokenizer that colors keywords, strings, comments, and punctuation as you type.
10. [Autocomplete, Part 1](11-autocomplete-basics.md) — dot-command completion, then schema-aware table/column completion wired to a Tab-triggered menu.
11. [Autocomplete, Part 2](12-autocomplete-context-aware.md) — teaching the completer *where* in a statement the cursor is, plus `alias.column` completion.
12. [Editing Ergonomics](13-editing-ergonomics.md) — auto-closing brackets and quotes, Shift-to-select, Ctrl+Arrow word jumps.
13. [The Database Explorer](14-database-explorer.md) — a Ctrl+E panel that trees out tables, columns, views, and indices.
14. [Polish and Loose Ends](15-polish-and-loose-ends.md) — `.dump`, `.read`, `.output` redirection, the banner, `--version-info`, and tying the non-interactive path back together.
15. [Appendix](appendix.md) — packaging, dead code we found along the way, and what SQLiteForge deliberately doesn't do.

Start with [Chapter 0: Architecture](01-architecture.md).
