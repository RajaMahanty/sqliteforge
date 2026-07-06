Previous: [Syntax Highlighting](10-syntax-highlighting.md) | [Contents](00-index.md) | Next: [Autocomplete, Part 2](12-autocomplete-context-aware.md)

# Chapter 10: Autocomplete, Part 1

Autocompletion is the largest single subsystem in SQLiteForge — the finished
`completion/mod.rs` is nearly 1,000 lines. We're splitting it across two
chapters, and being upfront about something first: this is the one part of
the tutorial where showing every line of the real file, unabbreviated,
would mean six giant keyword arrays back-to-back with no narrative value.
Where that happens, we show the real first few entries of an array plus its
real total count, and say so explicitly — the *logic* below is the
unabridged, exact code; only the keyword lists themselves are excerpted.

This chapter builds the plumbing: a `Completer` that suggests dot commands,
then table/column names pulled live from the open database, all triggered
by Tab. The next chapter teaches it to pay attention to *where* in a
statement the cursor is, and to `table.column`-qualified names.

## Step 10.1 — Dot-command completion and the Tab keybinding

```diff
+use reedline::{Completer, Span, Suggestion};
+use std::collections::HashMap;
+
+use crate::config::CompletionConfig;
+
+/// SQL and dot-command autocompletion engine
+pub struct SqlCompleter {
+    /// Dot commands
+    dot_commands: Vec<String>,
+}
+
+impl SqlCompleter {
+    pub fn new() -> Self {
+        Self { dot_commands: Self::dot_command_list() }
+    }
+
+    fn dot_command_list() -> Vec<String> {
+        // Only the commands that exist so far -- .read, .output, .dump,
+        // .nullvalue, and .preview join this list in Chapter 14.
+        vec![".help", ".quit", ".exit", ".tables", ".schema", ".indices", ".mode", ".headers", ".show"]
+            .into_iter().map(String::from).collect()
+    }
+}
+
+impl Completer for SqlCompleter {
+    fn complete(&mut self, line: &str, pos: usize) -> Vec<Suggestion> {
+        let line_to_cursor = &line[..pos];
+        if line_to_cursor.trim_start().starts_with('.') {
+            let input = line_to_cursor.trim_start();
+            let start = pos - input.len();
+            return self.dot_commands.iter()
+                .filter(|cmd| cmd.starts_with(input))
+                .map(|cmd| Suggestion {
+                    value: cmd.clone(), description: None, style: None, extra: None,
+                    span: Span::new(start, pos), append_whitespace: true,
+                })
+                .collect();
+        }
+        Vec::new()
+    }
+}
```

Wiring a `Completer` into reedline needs three pieces working together: the
completer itself, a menu widget to display suggestions, and a keybinding to
open it — none of which reedline gives you for free out of the box.

```diff
+use reedline::{
+    default_emacs_keybindings, ColumnarMenu, Emacs, FileBackedHistory, KeyCode,
+    KeyModifiers, MenuBuilder, Reedline, ReedlineEvent, ReedlineMenu, Signal,
+};
+use crate::completion::SqlCompleter;
 ...
+    // Tab: if menu is open -> cycle next; otherwise -> open menu
+    let mut keybindings = default_emacs_keybindings();
+    keybindings.add_binding(
+        KeyModifiers::NONE, KeyCode::Tab,
+        ReedlineEvent::UntilFound(vec![
+            ReedlineEvent::MenuNext,
+            ReedlineEvent::Menu("completion_menu".to_string()),
+        ]),
+    );
+    keybindings.add_binding(
+        KeyModifiers::SHIFT, KeyCode::BackTab, ReedlineEvent::MenuPrevious,
+    );
+    let edit_mode = Box::new(Emacs::new(keybindings));
+    let completion_menu = Box::new(ColumnarMenu::default().with_name("completion_menu"));
+
     let mut line_editor = Reedline::create()
         .with_history(reedline_history)
+        .with_completer(Box::new(SqlCompleter::new()))
+        .with_menu(ReedlineMenu::EngineCompleter(completion_menu))
         .with_highlighter(Box::new(SqlHighlighter))
         .with_validator(Box::new(SqlValidator))
+        .with_edit_mode(edit_mode);
```

**Verified** in a real terminal: typing `.h` and pressing Tab opens a menu
showing `.help` and `.headers`. Shift+Tab (`BackTab`) cycles backward
through it once it's open — `ReedlineEvent::MenuPrevious` is the mirror
image of the `MenuNext` we already reach for inside Tab's own
`UntilFound`.

`ReedlineEvent::UntilFound` is doing something specific: Tab tries
`MenuNext` first (cycle to the next suggestion, if a menu is already open),
and only falls through to `Menu(...)` (open the menu fresh) if there wasn't
one open to cycle. One physical key, two different behaviors depending on
editor state — which is also exactly why it needs `UntilFound` rather than
one binding calling the other unconditionally.

## Step 10.2 — Suggesting real table and column names

A completer that only knows dot commands isn't very useful for actual SQL.
`SqlCompleter` needs the database's own schema, refreshed whenever it
changes — and, looking ahead to next chapter's qualified `table.column`
completion, it needs columns indexed *per table*, not just as one flat
list:

```diff
 pub struct SqlCompleter {
     dot_commands: Vec<String>,
+    /// Table names (refreshed from database)
+    tables: Vec<String>,
+    /// View names
+    views: Vec<String>,
+    /// All column names (flat, for unqualified completion)
+    all_columns: Vec<String>,
+    /// Columns per table (for qualified "table"."col" completion)
+    table_columns: HashMap<String, Vec<String>>,
+    /// Index names
+    indices: Vec<String>,
 }
 impl SqlCompleter {
     pub fn new() -> Self {
-        Self { dot_commands: Self::dot_command_list() }
+        Self {
+            dot_commands: Self::dot_command_list(),
+            tables: Vec::new(), views: Vec::new(),
+            all_columns: Vec::new(), table_columns: HashMap::new(),
+            indices: Vec::new(),
+        }
     }
+
+    /// Update schema information from the database
+    pub fn update_schema(
+        &mut self, tables: Vec<String>, views: Vec<String>, all_columns: Vec<String>,
+        table_columns: HashMap<String, Vec<String>>, indices: Vec<String>,
+    ) {
+        self.tables = tables; self.views = views;
+        self.all_columns = all_columns; self.table_columns = table_columns;
+        self.indices = indices;
+    }
     ...
 impl Completer for SqlCompleter {
     fn complete(&mut self, line: &str, pos: usize) -> Vec<Suggestion> {
         { ... dot command completion, unchanged ... }
-        Vec::new()
+        let word_start = line_to_cursor
+            .rfind(|c: char| c.is_whitespace() || c == '(' || c == ',')
+            .map(|i| i + 1).unwrap_or(0);
+        let current_word = &line_to_cursor[word_start..];
+        if current_word.is_empty() {
+            return Vec::new(); // no eager suggestions yet -- next chapter
+        }
+        let lower_word = current_word.to_lowercase();
+        let mut suggestions = Vec::new();
+        for name in self.tables.iter().chain(self.views.iter())
+            .chain(self.all_columns.iter()).chain(self.indices.iter())
+        {
+            if name.to_lowercase().starts_with(&lower_word) {
+                suggestions.push(Suggestion {
+                    value: name.clone(), description: None, style: None, extra: None,
+                    span: Span::new(word_start, pos), append_whitespace: true,
+                });
+            }
+        }
+        suggestions
     }
 }
```

`Database` needs two more introspection methods to feed it — `get_views`
and `get_columns` (a `PRAGMA table_info(...)` query) — added the same way
as `get_tables` back in Chapter 7. And `shell::run` needs a function to pull
that schema out and hand it to the completer, both at startup and again
whenever the schema changes:

```diff
+    let mut completer = SqlCompleter::new();
+    refresh_completer(&db, &mut completer);
+
     let mut line_editor = Reedline::create()
         .with_history(reedline_history)
-        .with_completer(Box::new(SqlCompleter::new()))
+        .with_completer(Box::new(completer))
     ...
                 match db.execute_query(trimmed) {
                     Ok(result) => {
                         let output = Renderer::render(&result, &config.mode, config.headers, &config.nullvalue);
                         println!("{}", output);
+
+                        // Schema-changing statements invalidate our cached
+                        // table/column names, so rebuild the completer.
+                        let upper = trimmed.to_uppercase();
+                        if upper.starts_with("CREATE") || upper.starts_with("DROP")
+                            || upper.starts_with("ALTER") {
+                            let mut new_completer = SqlCompleter::new();
+                            refresh_completer(&db, &mut new_completer);
+                            line_editor = std::mem::replace(&mut line_editor, Reedline::create())
+                                .with_completer(Box::new(new_completer));
+                        }
+                    }
+                    Err(e) => eprintln!("\x1b[31mError: {}\x1b[0m", e),
+                }
+            }
+            ...
+
+/// Pull fresh table/view/column/index names out of the database and hand them to the completer.
+fn refresh_completer(db: &Database, completer: &mut SqlCompleter) {
+    let tables = db.get_tables();
+    let views = db.get_views();
+    let indices = db.get_indices();
+    let mut all_columns = Vec::new();
+    let mut table_columns = HashMap::new();
+    for table in &tables {
+        let cols = db.get_columns(table);
+        table_columns.insert(table.clone(), cols.clone());
+        for col in cols {
+            if !all_columns.contains(&col) { all_columns.push(col); }
+        }
+    }
+    completer.update_schema(tables, views, all_columns, table_columns, indices);
+}
```

**Verified** in a real terminal:

```
:memory:> CREATE TABLE users(id INTEGER, name TEXT);
Changes: 0
:memory:> SELECT * FROM us<Tab>
users
```

The `std::mem::replace(&mut line_editor, Reedline::create())` line is an
awkward-looking necessity, not a stylistic choice: `Reedline` owns its
completer as a boxed trait object with no public "swap the completer" method
— the only way to give it a *different* one is to move the whole editor out,
build a fresh one from it (`with_completer` consumes and returns `self`,
like the rest of the builder chain), and move that back in.
`Reedline::create()` as the placeholder works because it's cheap and
instantly discarded; the real `line_editor` is what comes back out of
`.with_completer(...)`.

There's no notion yet of *context* — typing `us` after `FROM` and typing
`us` after `SELECT` return the exact same suggestions, because every branch
of `complete` just filters the same flat lists by prefix. Teaching the
completer the difference between those two positions — and the difference
between a bare column and a `table.column` — is the whole subject of the
next chapter, and it's where this file grows from the ~120 lines we've
written so far to the ~970 in the real `completion/mod.rs`.

Next: [Chapter 11 — Autocomplete, Part 2](12-autocomplete-context-aware.md)
