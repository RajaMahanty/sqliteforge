Previous: [Polish and Loose Ends](15-polish-and-loose-ends.md) | [Contents](00-index.md)

# Appendix

Loose ends that didn't belong in the build order, but are worth knowing.

## Building for release

`Cargo.toml` sets a `[profile.release]` block:

```toml
[profile.release]
opt-level = 3
lto = true
strip = true
```

`opt-level = 3` is already `cargo build --release`'s default, so this line
is redundant but harmless — it just makes the intent explicit in the
manifest rather than relying on cargo's default. `lto = true` enables
link-time optimization, letting the optimizer see across crate boundaries
(including into `rusqlite`'s bundled C code) at the cost of a noticeably
slower release build. `strip = true` removes debug symbols from the final
binary, trading away symbolized backtraces for a smaller executable — a
reasonable trade for a CLI tool that isn't shipping its own crash reporter.

## Things that don't match the README

A couple of details are worth flagging precisely because a casual read of
this repository would miss them:

- **`README.md` documents an `install.sh`** (`chmod +x install.sh &&
  ./install.sh`, with claims of installing a man page and bash completion)
  **that does not exist anywhere in the current repository**, tracked or
  untracked. It was present in the initial commit according to `git show
  --stat`, but is gone by `HEAD` with no deletion visible in the visible
  commit history. If you're picking up this codebase, that's the first
  thing to either restore or remove the README section for.
- **Both `README.md` and `Cargo.toml` declare an MIT license, but there is
  no `LICENSE` file in the repository.** Worth fixing before anyone
  redistributes the binary.

Neither of these affects anything we built in the tutorial — they're
packaging and documentation gaps, not code — but "read the README, trust the
README" would have led you astray on both counts, which is exactly why we
checked instead of assuming.

## The vestigial parts, all in one place

Scattered through the chapters above, we flagged four places where the code
does more than anything currently uses:

| What | Where | What we found |
|---|---|---|
| `crossterm`, `ratatui`, `chrono`, `regex` | `Cargo.toml` | Declared as direct dependencies; never `use`d anywhere in `src/`. `crossterm` is a real transitive dependency of `reedline`; `ratatui`, `chrono`, and `regex` appear unused entirely. |
| `History::search`, `recent`, `all_entries` | `history/mod.rs` | Query SQLite's `history.db` table by pattern, most-recent-N, or in full — none are called from anywhere in the shell loop. Up/Down and Ctrl+R are powered entirely by reedline's separate `FileBackedHistory` text file. |
| The "database explorer panel" | `explorer/mod.rs` | Named and shaped like a TUI panel; implemented as a `String` of box-drawing characters printed with `println!`. No `ratatui` widgets, no persistent screen region. |
| `SqlCompleter::new()`, `Explorer::new()` | `completion/mod.rs`, `explorer/mod.rs` | Both replaced by `with_config(...)` constructors once `Config` grew sections for them; the plain `new()` versions are never called, and `cargo build` flags both as dead code. |

None of these are bugs — the program works correctly without any of them
being exercised. They're the normal residue of a codebase that grew
features incrementally: a config section gets added and a new constructor
takes over, but the old one isn't always deleted; a history table gets built
in anticipation of a `.history` command that never shipped. Calling them out
isn't a criticism so much as a habit worth building in general: `cargo
build`'s dead-code warnings are free, accurate information about what a
codebase actually does, as opposed to what it appears to do from reading the
module names.

## Where to go from here

SQLiteForge's own [`docs/REQUIREMENTS.md`](../docs/REQUIREMENTS.md) lists,
in its own words, what v1.0 deliberately leaves out: multiple database
connections, query tabs, a plugin system, PostgreSQL/MySQL support, query
execution plans, an import wizard, spreadsheet-style editing, visual schema
diagrams, and vim mode. Every one of those is a plausible next chapter for
someone who's followed this tutorial and wants to keep building:

- **Multiple connections / query tabs** would mean `App` holding a
  `Vec<Database>` instead of one, and the shell prompt/explorer needing to
  know which connection is "current" — a genuinely different shape than the
  single-`Database` assumption baked into `shell::run`'s signature
  throughout this tutorial.
- **Finishing the `.history` command** the dead-code warning in Chapter 8
  points at directly: `History::search` already exists, it just needs a dot
  command to call it and a `DotCommandResult::Output` to print the results.
  Genuinely the smallest gap in this whole list.
- **A real TUI explorer** would mean finally reaching for the `ratatui`
  dependency that's already sitting in `Cargo.toml` unused — replacing
  `Explorer::render() -> String` with an actual `ratatui` widget drawn into
  a persistent screen region, rather than a string reprinted on every
  toggle.
- **Vim mode** is a one-line change in `shell::run` in principle
  (`reedline` ships a `Vi` edit mode alongside the `Emacs` one we built
  throughout) — the real work is that every custom keybinding from Chapter
  12 onward (auto-pairs, Ctrl+E, Tab-completion) was wired against
  `Emacs::new(keybindings)` specifically, and would need re-wiring against
  whatever `reedline`'s Vi-mode keybinding API looks like.

That's the tour. The real source lives in `src/`, unchanged by anything in
this tutorial — everything here was built in a disposable scratch crate
purely to verify each step actually compiled and behaved as described.

[Back to Contents](00-index.md)
