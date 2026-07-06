Previous: [Editing Ergonomics](13-editing-ergonomics.md) | [Contents](00-index.md) | Next: [Polish and Loose Ends](15-polish-and-loose-ends.md)

# Chapter 13: The Database Explorer

Ctrl+E in SQLiteForge opens a "database explorer" panel. It's worth being
precise about what that actually is, because the name suggests more than
the implementation delivers: as noted back in the architecture chapter,
`ratatui` — a real terminal-UI framework — sits in `Cargo.toml` unused. The
explorer isn't an overlay pane or a separate screen region; it's a
`String` built out of box-drawing characters, printed to stdout like
anything else, that happens to look like a panel. We're building exactly
that, because that's what's really there.

## Step 13.1 — `Explorer`: pull the schema, draw a tree

**File:** `src/explorer/mod.rs`
```diff
+use crate::config::ExplorerConfig;
+use crate::database::Database;
+use std::collections::HashMap;
+
+/// Database explorer panel data
+pub struct Explorer {
+    pub tables: Vec<String>,
+    pub views: Vec<String>,
+    pub indices: Vec<String>,
+    pub table_columns: HashMap<String, Vec<(String, String)>>,
+    pub table_row_counts: HashMap<String, usize>,
+    pub visible: bool,
+    pub config: ExplorerConfig,
+}
+
+impl Explorer {
+    pub fn with_config(config: &ExplorerConfig) -> Self {
+        Self {
+            tables: Vec::new(), views: Vec::new(), indices: Vec::new(),
+            table_columns: HashMap::new(), table_row_counts: HashMap::new(),
+            visible: false, config: config.clone(),
+        }
+    }
+
+    /// Refresh the explorer data from the database
+    pub fn refresh(&mut self, db: &Database) {
+        self.tables = db.get_tables();
+        self.views = db.get_views();
+        self.indices = db.get_indices();
+        self.table_columns.clear();
+        self.table_row_counts.clear();
+        for table in &self.tables {
+            if self.config.show_columns {
+                self.table_columns.insert(table.clone(), db.get_column_info(table));
+            }
+            if self.config.show_row_counts {
+                if let Some(count) = db.get_row_count(table) {
+                    self.table_row_counts.insert(table.clone(), count);
+                }
+            }
+        }
+    }
+
+    pub fn toggle(&mut self) { self.visible = !self.visible; }
+
+    /// Render the explorer panel as a string
+    pub fn render(&self) -> String {
+        let w = self.config.panel_width;
+        // Inner width is panel_width minus the two border chars ("║" on each side)
+        let inner = w.saturating_sub(2);
+        let mut out = String::new();
+        out.push('\n');
+
+        out.push('╔');
+        out.push_str(&"═".repeat(inner));
+        out.push_str("╗\n");
+
+        let title = "Database Explorer";
+        let title_pad = inner.saturating_sub(title.len());
+        let title_left = title_pad / 2;
+        let title_right = title_pad - title_left;
+        out.push_str(&format!("║{}{}{}║\n", " ".repeat(title_left), title, " ".repeat(title_right)));
+
+        out.push('╠');
+        out.push_str(&"═".repeat(inner));
+        out.push_str("╣\n");
+
+        if self.tables.is_empty() && self.views.is_empty() && self.indices.is_empty() {
+            let msg = "(empty database)";
+            let pad = inner.saturating_sub(msg.len() + 2);
+            out.push_str(&format!("║  {}{}║\n", msg, " ".repeat(pad)));
+        }
+
+        if !self.tables.is_empty() {
+            let section = "Tables";
+            let pad = inner.saturating_sub(section.len() + 2);
+            out.push_str(&format!("║  {}{}║\n", section, " ".repeat(pad)));
+
+            for (i, table) in self.tables.iter().enumerate() {
+                let is_last = i == self.tables.len() - 1 && self.views.is_empty() && self.indices.is_empty();
+                let connector = if is_last && !self.config.show_columns { "└── " } else { "├── " };
+
+                let table_display = if self.config.show_row_counts {
+                    if let Some(count) = self.table_row_counts.get(table) {
+                        format!("{} ({})", table, count)
+                    } else { table.clone() }
+                } else { table.clone() };
+
+                let prefix = format!("{}{}", connector, table_display);
+                let pad = inner.saturating_sub(prefix.len() + 2);
+                out.push_str(&format!("║  {}{}║\n", prefix, " ".repeat(pad)));
+
+                if self.config.show_columns {
+                    if let Some(columns) = self.table_columns.get(table) {
+                        for (ci, (col_name, col_type)) in columns.iter().enumerate() {
+                            let is_last_col = ci == columns.len() - 1;
+                            let tree_prefix = if is_last && !self.config.show_columns { "    " } else { "│   " };
+                            let col_connector = if is_last_col { "└─" } else { "├─" };
+                            let col_display = if self.config.show_column_types && !col_type.is_empty() {
+                                format!("{} {}", col_name, col_type)
+                            } else { col_name.clone() };
+                            let prefix = format!("{}{} {}", tree_prefix, col_connector, col_display);
+                            let pad = inner.saturating_sub(prefix.len() + 2);
+                            // Dim color for column lines
+                            out.push_str(&format!("║  \x1b[90m{}\x1b[0m{}║\n", prefix, " ".repeat(pad)));
+                        }
+                    }
+                }
+            }
+        }
+
+        // Views and Indexes sections follow the same shape as Tables, each
+        // preceded by a blank separator line, using "└──"/"├──" connectors
+        // the same way.
+
+        out.push('╚');
+        out.push_str(&"═".repeat(inner));
+        out.push_str("╝\n");
+        out
+    }
+}
```

Two `Database` methods feed this: `get_column_info` (like `get_columns`
from Chapter 10, but also returning each column's declared type via
`PRAGMA table_info`'s third column) and `get_row_count` (a
`SELECT COUNT(*)`). `ExplorerConfig` in `config/mod.rs` follows the same
shape as `CompletionConfig` — `show_columns`, `show_row_counts`,
`show_column_types`, and `panel_width`, each with its own default.

## Step 13.2 — Ctrl+E

**File:** `src/shell/mod.rs`
```diff
+use crate::explorer::Explorer;
 ...
+    let mut explorer = Explorer::with_config(&config.explorer);
+    explorer.refresh(&db);
 ...
+    // Ctrl+E: toggle the database explorer panel
+    keybindings.add_binding(
+        KeyModifiers::CONTROL, KeyCode::Char('e'),
+        ReedlineEvent::ExecuteHostCommand("__explorer_toggle__".to_string()),
+    );
 ...
                 if trimmed == "__char_close_paren__" { ... continue; }
+
+                if trimmed == "__explorer_toggle__" {
+                    explorer.toggle();
+                    if explorer.visible { println!("{}", explorer.render()); }
+                    continue;
+                }
```

And, matching the pattern already established for the completer, the
explorer needs refreshing wherever the completer does — right after any
`CREATE`/`DROP`/`ALTER`:

**File:** `src/shell/mod.rs`
```diff
                             line_editor = std::mem::replace(&mut line_editor, Reedline::create())
                                 .with_completer(Box::new(new_completer));
+                            explorer.refresh(&db);
```

**Verified** in a real terminal, after `CREATE TABLE users(id INTEGER, name
TEXT);` and Ctrl+E — this is the exact, real output, copied verbatim from a
captured `tmux` pane, not a hand-typed approximation:

```
╔════════════════════════════════════╗
║          Database Explorer           ║
╠═════════════════════════════════════╣
║  Tables                              ║
║  ├── users                     ║
║  │   ├─ id INTEGER             ║
║  │   └─ name TEXT              ║
╚═════════════════════════════════════╝
```

Look closely at the right-hand border: it doesn't line up. The top,
separator, and bottom borders are all exactly 40 columns wide; the `users`,
`id INTEGER`, and `name TEXT` lines are visibly narrower. This isn't a typo
in our reconstruction — it's in the real, shipped `explorer/mod.rs`, and we
can now explain exactly why, because we just wrote the line that causes it:

```rust
let prefix = format!("{}{}", connector, table_display);
let pad = inner.saturating_sub(prefix.len() + 2);
```

`connector` is `"├── "` — three box-drawing characters that each occupy *one*
terminal column, but *three bytes* in UTF-8. `prefix.len()` counts bytes.
So `pad` comes out too small by roughly two columns for every tree
character in the prefix, and the line renders short. This is precisely the
bug Chapter 3 built `unicode_width::UnicodeWidthStr` to avoid in the
renderer's own box-drawing output — and the explorer, written later,
re-learns that lesson the hard way by reaching for `.len()` again. We're reproducing the
misalignment exactly rather than quietly fixing it, because a `cargo build`
of the real source produces this same crooked panel, and pretending
otherwise would make this the one dishonest diagram in an otherwise
verified tutorial.

Every keypress toggles the panel on or off and reprints it — there's no
persistent screen region, no redraw-in-place, nothing that would require
`ratatui` or even `crossterm`'s cursor-positioning APIs. Ctrl+E just runs
`explorer.render()` and `println!`s the result like any other command
output, which is the whole explanation for why this "panel" is really just
one more string the shell loop happens to print.

Next: [Chapter 14 — Polish and Loose Ends](15-polish-and-loose-ends.md)
