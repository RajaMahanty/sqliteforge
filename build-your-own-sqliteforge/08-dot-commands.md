Previous: [Multi-line Queries](07-multiline-queries.md) | [Contents](00-index.md) | Next: [Persistent History](09-persistent-history.md)

# Chapter 7: Dot Commands

`sqlite3`'s own CLI has `.tables`, `.schema`, and friends — commands that
aren't SQL, start with a dot, and never need a trailing semicolon (the
validator already special-cases them). This chapter gives them somewhere to
land: a `commands` module returning an enum the shell loop pattern-matches
on, rather than a module that prints directly.

## Step 7.1 — Introspection first: `.tables`, `.schema`, `.mode`, `.headers`

Dot commands need to read schema metadata SQLite itself tracks in
`sqlite_master`, so `Database` grows a handful of read-only queries first:

**File:** `src/database/mod.rs`
```diff
     /// Execute a SQL script file (may contain multiple statements)
+    pub fn get_tables(&self) -> Vec<String> {
+        let sql = "SELECT name FROM sqlite_master WHERE type='table' \
+                    AND name NOT LIKE 'sqlite_%' ORDER BY name";
+        self.conn.prepare(sql).ok()
+            .map(|mut stmt| stmt.query_map([], |row| row.get(0)).ok()
+                .map(|rows| rows.filter_map(|r| r.ok()).collect())
+                .unwrap_or_default())
+            .unwrap_or_default()
+    }
+    // get_views, get_indices: same shape, different `type=` filter
+    pub fn get_schema(&self, name: &str) -> Option<String> {
+        let sql = "SELECT sql FROM sqlite_master WHERE name = ?1";
+        self.conn.query_row(sql, [name], |row| row.get(0)).ok()
+    }
+    pub fn get_all_schemas(&self) -> Vec<String> {
+        let sql = "SELECT sql FROM sqlite_master WHERE sql IS NOT NULL ORDER BY type, name";
+        { ... same query_map + collect pattern ... }
+    }
```

Every one of these methods returns `Vec::new()` or `None` on any SQLite
error, rather than propagating a `Result`. That's a real, deliberate
tradeoff: `.tables` on a database with no tables and `.tables` where the
underlying query somehow failed look identical to the user ("No tables
found."), and for a read-only introspection helper, that's an acceptable
loss of information in exchange for call sites that don't need `?` or
`match` sprinkled through every dot command.

Then the `commands` module itself:

**File:** `src/commands/mod.rs`
```diff
+use crate::config::Config;
+use crate::database::Database;
+
+/// Result of executing a dot command
+pub enum DotCommandResult {
+    /// Output to display
+    Output(String),
+    /// Command was handled, no output needed
+    Handled,
+    /// Exit the shell
+    Exit,
+    /// Error message
+    Error(String),
+    /// Mode changed
+    ModeChanged(String),
+    /// Headers setting changed
+    HeadersChanged(bool),
+}
+
+pub fn execute_dot_command(input: &str, db: &Database, config: &Config) -> DotCommandResult {
+    let parts: Vec<&str> = input.trim().splitn(2, char::is_whitespace).collect();
+    let command = parts[0].to_lowercase();
+    let args = if parts.len() > 1 { parts[1].trim() } else { "" };
+
+    match command.as_str() {
+        ".help" => DotCommandResult::Output(help_text()),
+        ".quit" | ".exit" => DotCommandResult::Exit,
+        ".tables" => {
+            let tables = db.get_tables();
+            if tables.is_empty() { DotCommandResult::Output("No tables found.".to_string()) }
+            else { DotCommandResult::Output(tables.join("\n")) }
+        }
+        ".schema" => {
+            if args.is_empty() {
+                let schemas = db.get_all_schemas();
+                if schemas.is_empty() { DotCommandResult::Output("No Schema Found".to_string()) }
+                else { DotCommandResult::Output(schemas.join(";\n\n") + ";") }
+            } else {
+                let name = args.trim_matches('"').trim_matches('\'');
+                match db.get_schema(name) {
+                    Some(schema) => DotCommandResult::Output(schema + ";"),
+                    None => DotCommandResult::Error(format!("No such object: {}", name)),
+                }
+            }
+        }
+        ".indices" => { /* same shape as .tables */ DotCommandResult::Output(String::new()) }
+        ".mode" => {
+            if args.is_empty() {
+                DotCommandResult::Output(format!("Current mode: {}", config.mode))
+            } else {
+                match args.to_lowercase().as_str() {
+                    "box" | "table" | "column" | "markdown" | "csv" | "json" | "list" =>
+                        DotCommandResult::ModeChanged(args.to_lowercase()),
+                    m => DotCommandResult::Error(format!("Unknown mode: {}", m)),
+                }
+            }
+        }
+        ".headers" => {
+            if args.is_empty() {
+                DotCommandResult::Output(format!("Headers: {}", if config.headers {"on"} else {"off"}))
+            } else {
+                match args.to_lowercase().as_str() {
+                    "on" | "yes" | "true" | "1" => DotCommandResult::HeadersChanged(true),
+                    "off" | "no" | "false" | "0" => DotCommandResult::HeadersChanged(false),
+                    _ => DotCommandResult::Error("Usage: .headers on|off".to_string()),
+                }
+            }
+        }
+        ".show" => {
+            let mut out = String::new();
+            out.push_str(&format!("    database: {}\n", db.path));
+            out.push_str(&format!("        mode: {}\n", config.mode));
+            out.push_str(&format!("     headers: {}\n", if config.headers { "on" } else { "off" }));
+            out.push_str(&format!("   nullvalue: \"{}\"\n", config.nullvalue));
+            out.push_str(&format!("     history: {}\n", if config.history { "on" } else { "off" }));
+            out.push_str(&format!("       theme: {}\n", config.theme));
+            DotCommandResult::Output(out)
+        }
+        _ => DotCommandResult::Error(format!("Unknown command: {}. Use .help for a list.", command)),
+    }
+}

+fn help_text() -> String {
+    r#"SQLiteForge v1.0 - Available Commands:
+
+  .help                   Show this help message
+  .quit / .exit           Exit SQLiteForge
+  .tables                 List all tables
+  .schema [TABLE]         Show CREATE statements
+  .indices                List all indices
+  .mode [MODE]            Set output mode (box|table|column|markdown|csv|json|list)
+  .headers [on|off]       Toggle column headers
+  .show                   Show current settings"#
+        .to_string()
+}
```

One variant is already dead on arrival: nothing in `execute_dot_command`
ever returns `DotCommandResult::Handled`. It's declared, and — as we'll see
in a moment — the shell loop is already required to handle it, but no dot
command constructs it. It reads like a placeholder for a future command that
wants to do something (write a file, toggle a flag) without printing
anything back, and never got used for one.

The design worth calling out is `DotCommandResult` itself. `.mode json`
doesn't mutate `Config` from inside `execute_dot_command` — it can't, since
`config: &Config` is an immutable borrow. Instead it returns
`ModeChanged("json".to_string())` and hands the *decision* about what to do
with that back to the shell loop, which owns a mutable `Config` it's free to
update. This indirection is what makes `execute_dot_command` a plain
function you could unit-test with no shell, no reedline, and no mutable
state involved at all — the actual side effects all happen one layer up.

## Step 7.2 — Wiring it into the loop

**File:** `src/shell/mod.rs`
```diff
+use crate::commands::{self, DotCommandResult};
 ...
-pub fn run(db: Database, config: Config) -> ... {
+pub fn run(db: Database, mut config: Config) -> ... {
     ...
             Ok(Signal::Success(input)) => {
                 let trimmed = input.trim();
                 if trimmed.is_empty() { continue; }
+
+                if trimmed.starts_with('.') {
+                    match commands::execute_dot_command(trimmed, &db, &config) {
+                        DotCommandResult::Output(text) => println!("{}", text),
+                        DotCommandResult::Handled => {}
+                        DotCommandResult::Exit => { println!("Goodbye!"); break; }
+                        DotCommandResult::Error(e) => eprintln!("\x1b[31mError: {}\x1b[0m", e),
+                        DotCommandResult::ModeChanged(mode) => {
+                            config.mode = mode.clone();
+                            println!("Output mode changed to: {}", mode);
+                        }
+                        DotCommandResult::HeadersChanged(h) => {
+                            config.headers = h;
+                            println!("Headers {}", if h {"enabled"} else {"disabled"});
+                        }
+                    }
+                    continue;
+                }
+
                 match db.execute_query(trimmed) {
                     { ... unchanged ... }
```

**Verified** in a real terminal:

```
:memory:> CREATE TABLE t(a INTEGER);
Changes: 0
Execution Time: 0.2 ms
:memory:> .tables
t
```

`config` needed to become `mut config: Config` for this to compile —
`ModeChanged` and `HeadersChanged` are useless if the loop can't act on
them. That single word is the whole reason `DotCommandResult` needed to
exist as an enum in the first place, rather than `execute_dot_command` just
printing directly: something in this program has to actually *own* the
mutable configuration, and it isn't going to be a free function that only
ever sees an immutable borrow of it.

`.tables`/`.schema`/`.indices`/`.mode`/`.headers`/`.show` cover reading and
tweaking state that's already in memory. The next few dot commands —
`.read`, `.output`, `.dump`, `.preview` — touch the filesystem and the
renderer, and we're deferring them to Chapter 14, once there's more of the
program (history, the explorer) for them to interact with sensibly.

Next: [Chapter 8 — Persistent History](09-persistent-history.md)
