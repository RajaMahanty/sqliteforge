Previous: [Making Output Readable](04-making-output-readable.md) | [Contents](00-index.md) | Next: [A Real Interactive Shell](06-a-real-interactive-shell.md)

# Chapter 4: Configuration and the App

So far every setting (`mode`, `headers`) has come from a CLI flag with a
hardcoded fallback. Before we get to the interactive shell — which has a lot
more settings than `-m` covers — we need somewhere those defaults actually
live: a config file. This chapter also introduces `App`, the small struct
that will eventually decide whether we're headed for the non-interactive
path or the interactive one.

## Step 4.1 — A `Config` with five fields

```diff
+use serde::{Deserialize, Serialize};
+use std::fs;
+use std::path::PathBuf;
+
+/// Application configuration loaded from ~/.config/sqliteforge/config.toml
+#[derive(Debug, Clone, Serialize, Deserialize)]
+pub struct Config {
+    #[serde(default = "default_theme")]
+    pub theme: String,
+    #[serde(default = "default_mode")]
+    pub mode: String,
+    #[serde(default = "default_true")]
+    pub headers: bool,
+    #[serde(default = "default_true")]
+    pub history: bool,
+    #[serde(default = "default_nullvalue")]
+    pub nullvalue: String,
+}
+
+fn default_theme() -> String { "catppuccin".to_string() }
+fn default_mode() -> String { "box".to_string() }
+fn default_true() -> bool { true }
+fn default_nullvalue() -> String { String::new() }
+
+impl Default for Config {
+    fn default() -> Self {
+        Self {
+            theme: default_theme(), mode: default_mode(), headers: default_true(),
+            history: default_true(), nullvalue: default_nullvalue(),
+        }
+    }
+}
+
+impl Config {
+    pub fn config_path() -> PathBuf {
+        dirs::config_dir().unwrap_or_else(|| PathBuf::from("~/.config"))
+            .join("sqliteforge").join("config.toml")
+    }
+
+    /// Load configuration from file, falling back to defaults
+    pub fn load() -> Self {
+        let path = Self::config_path();
+        if path.exists() {
+            match fs::read_to_string(&path) {
+                Ok(content) => match toml::from_str(&content) {
+                    Ok(config) => return config,
+                    Err(e) => eprintln!("Warning: Failed to parse config: {}", e),
+                },
+                Err(e) => eprintln!("Warning: Failed to read config: {}", e),
+            }
+        }
+        let config = Self::default();
+        let _ = config.save();
+        config
+    }
+
+    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
+        let path = Self::config_path();
+        if let Some(parent) = path.parent() { fs::create_dir_all(parent)?; }
+        let content = toml::to_string_pretty(self)?;
+        fs::write(&path, content)?;
+        Ok(())
+    }
+}
```

```diff
+serde = { version = "1", features = ["derive"] }
+toml = "0.8"
+dirs = "6"
```

**Verified:** with `XDG_CONFIG_HOME=/tmp/tut-config`, running the program
once creates `/tmp/tut-config/sqliteforge/config.toml` containing:

```toml
theme = "catppuccin"
mode = "box"
headers = true
history = true
nullvalue = ""
```

`Config::load` writes a default file out to disk the *first* time it can't
find one — not because SQLiteForge insists on a config file existing, but so
a curious user who runs `sqliteforge` once and then goes looking at
`~/.config/sqliteforge/` finds something to edit, rather than nothing. Every
field also carries a `#[serde(default = ...)]`, which matters more than it
looks: it means a user can delete every line from their `config.toml` except
`mode = "table"`, and the other four fields silently fall back to their
Rust-side defaults instead of `toml::from_str` failing to parse the file at
all. We're about to add several more config sections in later chapters
(completion tuning, keybindings, the explorer panel) — the whole point of
this pattern is that adding a new section can never break someone's
existing, older config file.

`theme` is worth flagging immediately, honestly, rather than letting you
discover it later: it's read back out in `.show` (Chapter 7) and it's
written to every config file, but nothing in SQLiteForge ever *branches* on
its value — there's no code path anywhere that changes a color, a prompt, or
any rendering based on which theme string is set. It's a setting a user can
change that does nothing yet. We're building it in anyway, because it's
really there, but it's the first entry in what turns out to be a fairly long
list of "declared but not wired up" details we'll keep encountering (full
list in the [Appendix](appendix.md)).

## Step 4.2 — `App`: config and database, in one place

```diff
+pub use crate::config::Config;
+pub use crate::database::Database;
+
+/// Application state holder
+pub struct App {
+    pub config: Config,
+    pub db: Database,
+}
+
+impl App {
+    pub fn new(db_path: Option<&str>) -> Result<Self, Box<dyn std::error::Error>> {
+        let config = Config::load();
+        let db = match db_path {
+            Some(path) => Database::open(path)
+                .map_err(|e| format!("Failed to open database '{}': {}", path, e))?,
+            None => Database::open_in_memory()
+                .map_err(|e| format!("Failed to create in-memory database: {}", e))?,
+        };
+        Ok(Self { config, db })
+    }
+}
```

```diff
+mod app;
+mod config;
+use app::App;
 ...
 fn main() {
     let cli = Cli::parse();
-    let db = match &cli.database {
-        Some(path) => Database::open(path).expect("failed to open database"),
-        None => Database::open_in_memory().expect("failed to create in-memory database"),
-    };
-    println!("Connected to: {}", db.path);
-    let mode = cli.mode.as_deref().unwrap_or("box");
+    let app = App::new(cli.database.as_deref()).unwrap_or_else(|e| {
+        eprintln!("Error: {}", e);
+        std::process::exit(1);
+    });
+    let mut config = app.config.clone();
+    if let Some(mode) = &cli.mode { config.mode = mode.clone(); }
+    println!("Connected to: {}", app.db.path);
     if let Some(sql) = &cli.command {
-        execute_noninteractive(&db, mode, sql);
+        execute_noninteractive(&app.db, &config, sql);
     } else if let Some(path) = &cli.file {
         let content = std::fs::read_to_string(path).expect("failed to read file");
-        execute_noninteractive(&db, mode, &content);
+        execute_noninteractive(&app.db, &config, &content);
     }
 }
 
-fn execute_noninteractive(db: &Database, mode: &str, sql: &str) {
+fn execute_noninteractive(db: &Database, config: &Config, sql: &str) {
     for statement in sql.split(';') {
         let trimmed = statement.trim();
         if trimmed.is_empty() { continue; }
         let full_stmt = format!("{};", trimmed);
         match db.execute_query(&full_stmt) {
-            Ok(result) => println!("{}", Renderer::render(&result, mode, true, "")),
+            Ok(result) => println!(
+                "{}",
+                Renderer::render(&result, &config.mode, config.headers, &config.nullvalue)
+            ),
             Err(e) => eprintln!("Error: {}", e),
         }
     }
 }
```

**Verified:** `cargo run -- -c "SELECT 1 as x" -m table` still prints an
ASCII table, and a `config.toml` gets written on first run, but the saved
file still says `mode = "box"` — the `-m table` override only ever touches
the *cloned* `config` inside `main`, never `app.config` itself, so a
one-shot `-m` flag can't accidentally rewrite the user's saved default. This
is a small but deliberate ordering: `App::new` loads the config from disk
once; everything after that (`config.mode = mode.clone()`) works on a copy.

This is also the point where `""`, the placeholder nullvalue from the last
chapter, gets replaced with something real: `&config.nullvalue`. Nothing
observable changes yet — we already established that parameter doesn't
affect rendering — but `execute_noninteractive` swapping two loose
parameters (`mode: &str`, plus the `headers`/`nullvalue` it didn't even have
yet) for one `config: &Config` is the shape every other entry point in this
program converges on. Every dot command handler and the interactive shell
itself, from here on, take `&Config` as a whole rather than picking
individual fields out of it — it's a small refactor here specifically
because `execute_noninteractive` is the first function old enough to need
it.

`App` itself doesn't do much yet — bundling `Config` and `Database` into one
struct only pays off once something (the interactive shell, still two
chapters away) needs both at once. Right now it's a container we're
building ahead of its first real use, because retrofitting it once `main.rs`
has grown a shell-launching branch is messier than building it now, while
`main.rs` is still small enough to read in one glance.

Speaking of that branch — we still don't have anywhere for the *no* `-c`,
*no* `-f` case to go. That's next.

Next: [Chapter 5 — A Real Interactive Shell](06-a-real-interactive-shell.md)
