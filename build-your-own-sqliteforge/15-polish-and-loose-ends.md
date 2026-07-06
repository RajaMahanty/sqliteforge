Previous: [The Database Explorer](14-database-explorer.md) | [Contents](00-index.md) | Next: [Appendix](appendix.md)

# Chapter 14: Polish and Loose Ends

Everything that makes SQLiteForge feel finished, rather than merely
functional, is in this last chapter: the remaining dot commands that touch
files and the renderer, output redirection, the startup banner, a
`--version-info` flag, and reconciling the non-interactive path (built back
in Chapter 3, before dot commands existed) with everything since.

## Step 14.1 — `.read`, `.output`, `.dump`, `.nullvalue`, `.preview`

**File:** `src/commands/mod.rs`
```diff
     pub enum DotCommandResult {
         Output(String), Exit, Error(String), ModeChanged(String), HeadersChanged(bool),
+        NullvalueChanged(String),
+        OutputChanged(Option<String>),
     }
 ...
         ".show" => { ... }
+        ".read" => {
+            if args.is_empty() { DotCommandResult::Error("Usage: .read FILENAME".to_string()) }
+            else {
+                match fs::read_to_string(args) {
+                    Ok(content) => match db.execute_script(&content) {
+                        Ok(_) => DotCommandResult::Output(format!("Executed script: {}", args)),
+                        Err(e) => DotCommandResult::Error(format!("Script error: {}", e)),
+                    },
+                    Err(e) => DotCommandResult::Error(format!("Cannot read file: {}", e)),
+                }
+            }
+        }
+        ".output" => {
+            if args.is_empty() || args == "stdout" { DotCommandResult::OutputChanged(None) }
+            else { DotCommandResult::OutputChanged(Some(args.to_string())) }
+        }
+        ".dump" => match db.dump() {
+            Ok(dump) => DotCommandResult::Output(dump),
+            Err(e) => DotCommandResult::Error(format!("Dump error: {}", e)),
+        },
+        ".nullvalue" => {
+            if args.is_empty() { DotCommandResult::Output(format!("Nullvalue: \"{}\"", config.nullvalue)) }
+            else { DotCommandResult::NullvalueChanged(args.to_string()) }
+        }
+        ".preview" => {
+            if args.is_empty() { DotCommandResult::Error("Usage: .preview TABLE_NAME".to_string()) }
+            else {
+                let name = args.trim_matches('"').trim_matches('\'');
+                let sql = format!("SELECT * FROM \"{}\" LIMIT 20", name.replace('"', "\"\""));
+                match db.execute_query(&sql) {
+                    Ok(result) => DotCommandResult::Output(
+                        crate::renderer::Renderer::render(&result, &config.mode, config.headers, &config.nullvalue)),
+                    Err(e) => DotCommandResult::Error(format!("Preview error: {}", e)),
+                }
+            }
+        }
```

And `.help`'s own text grows to list the five new commands, plus a full
keyboard-shortcut reference and the config file's path (`Config::config_path()`,
from Chapter 4, finally gets a second caller). We're not reproducing that
string here in full — it's the least interesting kind of diff, purely more
text — but it's in the scratch crate and the real source identically.

`db.dump()` (schema + every row, as `INSERT` statements, wrapped in a
transaction) and `db.execute_script()` (a thin wrapper over
`execute_batch`, unused since Chapter 2 until `.read` finally calls it) are
new `Database` methods, in the same style as `get_tables` and friends.

## Step 14.2 — Output redirection

`.output somefile.txt` should redirect every subsequent result to a file
instead of stdout. That means every `println!` for query results and dot
command output needs to become conditional:

**File:** `src/shell/mod.rs`
```diff
+    // When set via `.output FILE`, query results go to a file instead of stdout
+    let mut output_file: Option<String> = None;
 ...
                     match commands::execute_dot_command(trimmed, &db, &config) {
-                        DotCommandResult::Output(text) => println!("{}", text),
+                        DotCommandResult::Output(text) => output_text(&text, &output_file),
                         ...
+                        DotCommandResult::NullvalueChanged(nv) => {
+                            config.nullvalue = nv.clone();
+                            db.nullvalue = nv.clone();
+                            println!("Nullvalue set to: \"{}\"", nv);
+                        }
+                        DotCommandResult::OutputChanged(file) => {
+                            match &file {
+                                Some(f) => println!("Output redirected to: {}", f),
+                                None => println!("Output restored to stdout"),
+                            }
+                            output_file = file;
+                        }
                     }
                 ...
                 match db.execute_query(trimmed) {
                     Ok(result) => {
                         let output = Renderer::render(&result, &config.mode, config.headers, &config.nullvalue);
-                        println!("{}", output);
+                        output_text(&output, &output_file);
 ...
+/// Print to stdout, or write to the redirected output file if one is set.
+fn output_text(text: &str, output_file: &Option<String>) {
+    match output_file {
+        Some(path) => {
+            if let Err(e) = std::fs::write(path, text) { eprintln!("Error writing to {}: {}", path, e); }
+        }
+        None => println!("{}", text),
+    }
+}
```

`db.nullvalue = nv.clone()` here is the payoff of a field we quietly added
to `Database` and set once, back at the top of `shell::run`
(`db.nullvalue = config.nullvalue.clone()`), but never mentioned: it's
what `execute_select` substitutes in for SQL `NULL` values instead of an
empty string. `.nullvalue` needs to update *both* `config.nullvalue` (so
`.show` and a future config save reflect it) and `db.nullvalue` (so the
very next query actually uses it) — two copies of the same setting, kept in
sync by hand at every place either dot command or config lets you change it.

**Verified** in a real terminal:

```
:memory:> CREATE TABLE t(a INTEGER); INSERT INTO t VALUES (1);
:memory:> .dump
BEGIN TRANSACTION;
CREATE TABLE t(a INTEGER);
INSERT INTO "t" VALUES(1);
COMMIT;
```

## Step 14.3 — The real banner, and `--version-info`

Every banner we've shown so far in this tutorial has been a placeholder
three-liner. The real one is bigger:

**File:** `src/shell/mod.rs`
```diff
+    print_banner(&db.path);
     loop {
 ...
+/// Print the welcome banner
+fn print_banner(db_path: &str) {
+    println!(
+        "\x1b[36m\x1b[1m{}",
+        r#"
+  ███████╗ ██████╗ ██╗     ██╗████████╗███████╗███████╗ ██████╗ ██████╗  ██████╗ ███████╗
+  ██╔════╝██╔═══██╗██║     ██║╚══██╔══╝██╔════╝██╔════╝██╔═══██╗██╔══██╗██╔════╝ ██╔════╝
+  ███████╗██║   ██║██║     ██║   ██║   █████╗  █████╗  ██║   ██║██████╔╝██║  ███╗█████╗
+  ╚════██║██║▄▄ ██║██║     ██║   ██║   ██╔══╝  ██╔══╝  ██║   ██║██╔══██╗██║   ██║██╔══╝
+  ███████║╚██████╔╝███████╗██║   ██║   ███████╗██║     ╚██████╔╝██║  ██║╚██████╔╝███████╗
+  ╚══════╝ ╚══▀▀═╝ ╚══════╝╚═╝   ╚═╝   ╚══════╝╚═╝      ╚═════╝ ╚═╝  ╚═╝ ╚═════╝ ╚══════╝"#
+    );
+    println!("\x1b[0m");
+    println!(
+        "  \x1b[37m\x1b[1mv1.0\x1b[0m  \x1b[90m─  A modern terminal-first SQLite client\x1b[0m"
+    );
+    println!("  \x1b[90mConnected to: \x1b[33m{}\x1b[0m", db_path);
+    println!(
+        "  \x1b[90mType \x1b[36m.help\x1b[90m for commands, \x1b[36mCtrl+D\x1b[90m to exit\x1b[0m"
+    );
+    println!();
+}
```

There's genuinely nothing more to say about this one than "it's a big raw
string literal, printed with an ANSI color code wrapped around it." We're
including it in full rather than eliding it, though, because eliding it is
exactly the kind of thing that quietly turns "we rebuilt this program" into
"we rebuilt an approximation of this program," and by this chapter we mean
the former literally.

**File:** `src/main.rs`
```diff
 /// SQLiteForge - A modern terminal-first SQLite client
 #[derive(Parser, Debug)]
-#[command(name = "sqliteforge", version = "1.0.1")]
+#[command(
+    name = "sqliteforge",
+    version = "1.0.1",
+    about = "A modern terminal-first SQLite client",
+    long_about = "SQLiteForge is a modern, feature-rich terminal client for SQLite databases.\nIt provides syntax highlighting, autocompletion, persistent history, and multiple output formats."
+)]
 struct Cli {
     ...
     mode: Option<String>,
+    /// Show version information
+    #[arg(long = "version-info")]
+    version_info: bool,
 }
 fn main() {
     let cli = Cli::parse();
+    if cli.version_info {
+        println!("SQLiteForge v1.0.1");
+        println!("Built with Rust + rusqlite");
+        println!("Terminal: crossterm");
+        println!("Editor: reedline");
+        return;
+    }
     let app = App::new(...)...;
```

"Terminal: crossterm" in that version banner is worth one last look back at
the architecture chapter: it's the one place in the entire codebase where
`crossterm` is mentioned by name in our own source, and it's a `println!`
string, not a `use crossterm::...`. The dependency is real and load-bearing
(reedline needs it), but SQLiteForge's own code never touches it directly
even here, where it's advertising it.

## Step 14.4 — Splitting the loop body into named functions

Everything we've added since Chapter 7 has grown the body of the `match
line_editor.read_line(&prompt)` arm for `Signal::Success` — dot commands,
SQL execution, completer refresh, explorer refresh, all inline. The real
source extracts the two largest branches into their own functions,
`process_dot_command` and `execute_sql`, taking `db`, `config`,
`output_file`, `line_editor`, and `explorer` as parameters instead of
capturing them as closures. That extraction forces one more change:
`process_dot_command` can't `break` out of a loop it isn't inside anymore,
so exiting on `.quit` becomes a `*should_exit = true` write-through a `&mut
bool`, checked at the top of the loop on every iteration instead:

**File:** `src/shell/mod.rs`
```diff
+    let mut should_exit = false;
     loop {
+        if should_exit { break; }
         match line_editor.read_line(&prompt) {
             Ok(Signal::Success(input)) => {
                 ...
                 if trimmed.starts_with('.') {
-                    match commands::execute_dot_command(trimmed, &db, &config) { ... }
-                    continue;
+                    process_dot_command(
+                        trimmed, &db, &mut config, &mut output_file,
+                        &mut line_editor, &mut explorer, &mut should_exit,
+                    );
+                    continue;
                 }
-                match db.execute_query(trimmed) { ... }
+                execute_sql(trimmed, &db, &config, &output_file, &mut line_editor, &mut explorer);
             }
             ...
         }
     }
+
+/// Process a dot command
+fn process_dot_command(
+    cmd: &str, db: &Database, config: &mut Config, output_file: &mut Option<String>,
+    line_editor: &mut Reedline, explorer: &mut Explorer, should_exit: &mut bool,
+) {
+    match commands::execute_dot_command(cmd, db, config) {
+        DotCommandResult::Output(text) => output_text(&text, output_file),
+        DotCommandResult::Handled => {}
+        DotCommandResult::Exit => { println!("Goodbye!"); *should_exit = true; }
+        DotCommandResult::Error(e) => eprintln!("\x1b[31mError: {}\x1b[0m", e),
+        DotCommandResult::ModeChanged(mode) => { config.mode = mode.clone(); println!("Output mode changed to: {}", mode); }
+        DotCommandResult::HeadersChanged(h) => { config.headers = h; println!("Headers {}", if h {"enabled"} else {"disabled"}); }
+        DotCommandResult::NullvalueChanged(nv) => { config.nullvalue = nv.clone(); println!("Nullvalue set to: \"{}\"", nv); }
+        DotCommandResult::OutputChanged(file) => {
+            match &file {
+                Some(f) => println!("Output redirected to: {}", f),
+                None => println!("Output restored to stdout"),
+            }
+            *output_file = file;
+        }
+    }
+    if cmd.starts_with(".read") {
+        let mut new_completer = SqlCompleter::with_config(&config.completion);
+        refresh_completer(db, &mut new_completer);
+        *line_editor = std::mem::replace(line_editor, Reedline::create())
+            .with_completer(Box::new(new_completer));
+        explorer.refresh(db);
+    }
+}
+
+/// Execute a SQL statement and handle results
+fn execute_sql(
+    sql: &str, db: &Database, config: &Config, output_file: &Option<String>,
+    line_editor: &mut Reedline, explorer: &mut Explorer,
+) {
+    match db.execute_query(sql) {
+        Ok(result) => {
+            let output = Renderer::render(&result, &config.mode, config.headers, &config.nullvalue);
+            output_text(&output, output_file);
+            let upper = sql.to_uppercase();
+            if upper.starts_with("CREATE") || upper.starts_with("DROP") || upper.starts_with("ALTER") {
+                let mut new_completer = SqlCompleter::with_config(&config.completion);
+                refresh_completer(db, &mut new_completer);
+                *line_editor = std::mem::replace(line_editor, Reedline::create())
+                    .with_completer(Box::new(new_completer));
+                explorer.refresh(db);
+            }
+        }
+        Err(e) => eprintln!("\x1b[31mError: {}\x1b[0m", e),
+    }
 }
```

One find in this refactor is worth calling back to Chapter 7: `.read`'s
completer-and-explorer refresh now lives inside `process_dot_command`
itself rather than the loop that calls it — which is exactly the
`DotCommandResult::Handled` variant's moment to have mattered, if any dot
command actually needed to signal "I already did my own side effect,
don't print anything" instead of returning `Output`. None of them do.
`Handled` stays unconstructed even here, in the function that would be its
only caller.

And `main.rs`'s non-interactive path, unchanged in *shape* since Chapter 4
but still missing dot-command support, finally catches up. This is the
last piece needed for `sqliteforge db.sqlite -f setup.sql` to use `.read`
or `.mode` the same way an interactive session can:

**File:** `src/main.rs`
```diff
 fn execute_noninteractive(db: &Database, config: &Config, sql: &str) {
     for statement in sql.split(';') {
         let trimmed = statement.trim();
         if trimmed.is_empty() { continue; }
         let full_stmt = format!("{};", trimmed);
+        if trimmed.starts_with('.') {
+            match commands::execute_dot_command(trimmed, db, config) {
+                DotCommandResult::Output(text) => println!("{}", text),
+                DotCommandResult::Error(e) => eprintln!("Error: {}", e),
+                DotCommandResult::Exit => return,
+                _ => {}
+            }
+        } else {
-        match db.execute_query(&full_stmt) {
-            Ok(result) => println!("{}", Renderer::render(&result, &config.mode, config.headers, &config.nullvalue)),
-            Err(e) => eprintln!("Error: {}", e),
-        }
+            match db.execute_query(&full_stmt) {
+                Ok(result) => println!("{}", Renderer::render(&result, &config.mode, config.headers, &config.nullvalue)),
+                Err(e) => eprintln!("\x1b[31mError: {}\x1b[0m", e),
+            }
     }
 }
```

**Verified:** `cargo run -- --version-info` prints the three version lines
and exits without touching a database at all; interactive mode now opens
with a colored banner before the first prompt.

That closes the gap between "the CLI can run one statement" (Chapter 3) and
"the CLI understands everything the interactive shell does" (here) — the
two entry points share `Database`, `Config`, and `commands` completely, and
differ only in how they get a line of input in the first place: one from
`reedline`, one from splitting a string on `;`.

This is also, line for line, feature for feature, the real SQLiteForge.
Everything from here is what didn't fit the build order above.

Next: [Appendix](appendix.md)

