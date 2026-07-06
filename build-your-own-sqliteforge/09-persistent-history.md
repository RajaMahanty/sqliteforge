Previous: [Dot Commands](08-dot-commands.md) | [Contents](00-index.md) | Next: [Syntax Highlighting](10-syntax-highlighting.md)

# Chapter 8: Persistent History

Right now, closing SQLiteForge forgets every query you ran. This chapter
adds two independent, parallel history mechanisms — deliberately two, not
one, because they solve different problems and one of them, as we'll
confirm with a real `cargo build`, doesn't actually get used by anything yet.

## Step 8.1 — A SQLite-backed query log

**File:** `src/history/mod.rs`
```diff
+use rusqlite::Connection;
+use std::path::PathBuf;
+
+/// Persistent query history backed by SQLite
+pub struct History {
+    conn: Connection,
+}
+
+impl History {
+    pub fn open() -> Result<Self, String> {
+        let path = Self::history_path();
+        if let Some(parent) = path.parent() {
+            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
+        }
+        let conn = Connection::open(&path).map_err(|e| e.to_string())?;
+        conn.execute_batch(
+            "CREATE TABLE IF NOT EXISTS history (
+                id INTEGER PRIMARY KEY AUTOINCREMENT,
+                query TEXT NOT NULL,
+                timestamp TEXT NOT NULL DEFAULT (datetime('now'))
+            );
+            CREATE INDEX IF NOT EXISTS idx_history_timestamp ON history(timestamp DESC);",
+        ).map_err(|e| e.to_string())?;
+        Ok(Self { conn })
+    }
+
+    pub fn history_path() -> PathBuf {
+        dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("~/.local/share"))
+            .join("sqliteforge").join("history.db")
+    }
+
+    pub fn add(&self, query: &str) -> Result<(), String> {
+        let trimmed = query.trim();
+        if trimmed.is_empty() { return Ok(()); }
+        self.conn.execute("INSERT INTO history (query) VALUES (?1)", [trimmed])
+            .map_err(|e| e.to_string())?;
+        Ok(())
+    }
+
+    /// Search history with a pattern (fuzzy)
+    pub fn search(&self, pattern: &str) -> Vec<String> {
+        let like_pattern = format!("%{}%", pattern);
+        self.conn.prepare(
+            "SELECT DISTINCT query FROM history WHERE query LIKE ?1 ORDER BY id DESC LIMIT 50",
+        ).ok()
+            .map(|mut stmt| stmt.query_map([&like_pattern], |row| row.get(0)).ok()
+                .map(|rows| rows.filter_map(|r| r.ok()).collect())
+                .unwrap_or_default())
+            .unwrap_or_default()
+    }
+
+    /// Get recent history entries
+    pub fn recent(&self, limit: usize) -> Vec<String> {
+        self.conn.prepare("SELECT query FROM history ORDER BY id DESC LIMIT ?1").ok()
+            .map(|mut stmt| stmt.query_map([limit as i64], |row| row.get(0)).ok()
+                .map(|rows| rows.filter_map(|r| r.ok()).collect())
+                .unwrap_or_default())
+            .unwrap_or_default()
+    }
+
+    /// Get all history for reedline integration
+    pub fn all_entries(&self) -> Vec<String> {
+        self.conn.prepare("SELECT query FROM history ORDER BY id ASC").ok()
+            .map(|mut stmt| stmt.query_map([], |row| row.get(0)).ok()
+                .map(|rows| rows.filter_map(|r| r.ok()).collect())
+                .unwrap_or_default())
+            .unwrap_or_default()
+    }
+}
```

The index on `timestamp` is a small piece of forward-looking schema design:
nothing we've built yet queries `history` by timestamp (`search` and `add`
both go through `id`), but if a future `.history --since` ever gets built,
the index is already there waiting for it — cheap to maintain, and exactly
the kind of thing worth doing when you're already defining the table rather
than retrofitting it once the table has real data in it.

`recent` and `all_entries` round out `search` from a moment ago: three
different ways to read the same table back (by pattern, most-recent-N, or
in full), all built at once, none of them called from anywhere yet. We'll
confirm exactly that with `cargo build` in a moment.

**File:** `src/history/mod.rs`
```diff
+mod history;
```

**Verified:** compiles — with a warning we'll come back to in a moment.

## Step 8.2 — reedline's own history, wired into the shell

`History` alone doesn't give you Up-arrow recall or Ctrl+R search — those
are features of `Reedline` itself, driven by whatever implements its own
`History` trait. `FileBackedHistory` is reedline's built-in implementation,
backed by a plain text file, and it needs to be handed to the line editor
directly:

**File:** `src/shell/mod.rs`
```diff
+use reedline::{FileBackedHistory, Reedline, Signal};
+use crate::history::History;
 ...
 pub fn run(db: Database, mut config: Config) -> ... {
+    // SQLite-backed history: logs every query for later inspection
+    let history = if config.history { History::open().ok() } else { None };
     ...
     let prompt = SqlPrompt::new(&db_name);
+
+    // reedline's own plain-text history: this is what actually powers
+    // Up/Down arrow navigation and Ctrl+R reverse search.
+    let reedline_history = Box::new(
+        FileBackedHistory::with_file(1000, History::history_path().with_extension("txt"))
+            .expect("Failed to initialize reedline history"),
+    );
+
     let mut line_editor = Reedline::create()
+        .with_history(reedline_history)
         .with_validator(Box::new(SqlValidator));
     ...
             Ok(Signal::Success(input)) => {
                 let trimmed = input.trim();
                 if trimmed.is_empty() { continue; }
+
+                if let Some(ref hist) = history {
+                    let _ = hist.add(trimmed);
+                }
+
                 if trimmed.starts_with('.') {
```

**Verified** in a real terminal: after running `.tables`, pressing Up-arrow
recalls `.tables` into the prompt. This works entirely through
`FileBackedHistory` writing to `history.txt` — a completely different file
from the SQLite `history.db` we just built in Step 8.1.

Now the promised warning:

```
warning: methods `search`, `recent`, and `all_entries` are never used
  --> src/history/mod.rs:53:12
```

This is worth sitting with rather than silencing. `History::add` *is*
called — every query gets logged into `history.db` — but none of `search`,
`recent`, or `all_entries` are called by anything, anywhere in the program
we've built so far (and, we can confirm reading the real SQLiteForge source,
anywhere in the shipped program either). The SQLite-backed log and
reedline's text-file history aren't two views onto the same data; they're
two entirely separate systems that happen to both be called "history,"
where only one of them currently does anything a user can observe.
`history.db` reads like the start of a `.history` command, or a smarter
Ctrl+R that fuzzy-matches instead of substring-matches — a feature that got
scaffolded, three different ways to read it back and all, and never
finished. We're building it anyway, because it's what's really there, but
we're not going to pretend the warning doesn't mean what it says.

Next: [Chapter 9 — Syntax Highlighting](10-syntax-highlighting.md)
