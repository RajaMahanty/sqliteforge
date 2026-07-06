Previous: [Configuration and the App](05-configuration-and-app.md) | [Contents](00-index.md) | Next: [Multi-line Queries](07-multiline-queries.md)

# Chapter 5: A Real Interactive Shell

Everything so far has been testable by piping a `-c` flag through
`cargo run`. That ends here. `reedline` takes over the terminal in raw mode
to manage its own line editing, so from this chapter on, verifying anything
requires an actual terminal — piped stdin won't behave sensibly, because
reedline can't put a pipe into raw mode.

*A verification note, since this matters for the rest of the tutorial:* we
tested every interactive step in this and later chapters inside a real
`tmux` pane, sending literal keystrokes and capturing the pane's contents —
the closest thing to "a person typing" we can automate. If you're following
along, just run `cargo run` in your own terminal.

## Step 5.1 — A prompt and a read loop

**File:** `src/shell/prompt.rs`
```diff
+use reedline::Prompt;
+use std::borrow::Cow;
+
+/// Custom prompt for SQLiteForge
+pub struct SqlPrompt {
+    pub db_name: String,
+}
+
+impl SqlPrompt {
+    pub fn new(db_name: &str) -> Self {
+        Self { db_name: db_name.to_string() }
+    }
+}
+
+impl Prompt for SqlPrompt {
+    fn render_prompt_left(&self) -> Cow<'_, str> {
+        Cow::Owned(format!("{}> ", self.db_name))
+    }
+    fn render_prompt_right(&self) -> Cow<'_, str> { Cow::Borrowed("") }
+    fn render_prompt_indicator(&self, _mode: reedline::PromptEditMode) -> Cow<'_, str> {
+        Cow::Borrowed("")
+    }
+    fn render_prompt_multiline_indicator(&self) -> Cow<'_, str> {
+        let pad = self.db_name.len().saturating_sub(3);
+        Cow::Owned(format!("{}...> ", " ".repeat(pad)))
+    }
+    fn render_prompt_history_search_indicator(
+        &self, _search: reedline::PromptHistorySearch,
+    ) -> Cow<'_, str> {
+        Cow::Borrowed("(search) ")
+    }
+}
```

**File:** `src/shell/mod.rs`
```diff
+pub mod prompt;
+use reedline::{Reedline, Signal};
+use crate::config::Config;
+use crate::database::Database;
+use crate::renderer::Renderer;
+use self::prompt::SqlPrompt;
+
+/// Run the interactive shell
+pub fn run(db: Database, config: Config) -> Result<(), Box<dyn std::error::Error>> {
+    let db_name = std::path::Path::new(&db.path)
+        .file_name().map(|n| n.to_string_lossy().to_string())
+        .unwrap_or_else(|| db.path.clone());
+
+    let prompt = SqlPrompt::new(&db_name);
+    let mut line_editor = Reedline::create();
+
+    loop {
+        match line_editor.read_line(&prompt) {
+            Ok(Signal::Success(input)) => {
+                let trimmed = input.trim();
+                if trimmed.is_empty() { continue; }
+                match db.execute_query(trimmed) {
+                    Ok(result) => {
+                        let output = Renderer::render(&result, &config.mode, config.headers, &config.nullvalue);
+                        println!("{}", output);
+                    }
+                    Err(e) => eprintln!("\x1b[31mError: {}\x1b[0m", e),
+                }
+            }
+            Ok(Signal::CtrlC) => continue,
+            Ok(Signal::CtrlD) => { println!("Goodbye!"); break; }
+            Err(e) => { eprintln!("Error: {}", e); break; }
+        }
+    }
+    Ok(())
+}
```

`App` gets a `run` method, and `main` finally grows the branch that's been
missing since Chapter 4:

**File:** `src/app/mod.rs`
```diff
 impl App {
     pub fn new(...) -> ... { ... }
+
+    /// Run the application
+    pub fn run(self) -> Result<(), Box<dyn std::error::Error>> {
+        crate::shell::run(self.db, self.config)
+    }
 }
```

**File:** `src/main.rs`
```diff
+mod shell;
 ...
     if let Some(sql) = &cli.command {
         execute_noninteractive(&app.db, &config, sql);
-    } else if let Some(path) = &cli.file {
+        return;
+    }
+    if let Some(path) = &cli.file {
         let content = std::fs::read_to_string(path).expect("failed to read file");
         execute_noninteractive(&app.db, &config, &content);
+        return;
+    }
+
+    // Interactive mode
+    let app = App { config, db: app.db };
+    if let Err(e) = app.run() {
+        eprintln!("Error: {}", e);
+        std::process::exit(1);
     }
```

**Verified** (real terminal, no `-c`/`-f` flags): the program prints
`:memory>` and waits. Typing `SELECT 1 as x` and pressing Enter immediately
runs it and prints a table. Ctrl+D prints `Goodbye!` and exits; Ctrl+C
returns to a fresh prompt without exiting.

Notice what's *not* here yet: any concept of "this line isn't finished."
`Reedline::create()` with no validator treats every Enter as a submission —
type half a `SELECT`, press Enter, and it runs (and fails) immediately.
Multi-line SQL doesn't work at all right now, and syntax isn't highlighted,
and Tab does nothing. Each of the next several chapters bolts one of those
onto this same loop by handing `Reedline::create()` another `.with_*()`
builder call — the loop's shape barely changes; what changes is how much
work `Reedline` is doing before it ever calls back into our code.

The most urgent gap is "every Enter submits immediately" — that's next.

Next: [Chapter 6 — Multi-line Queries](07-multiline-queries.md)
