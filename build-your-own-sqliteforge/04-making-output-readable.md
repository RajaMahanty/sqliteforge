Previous: [Talking to SQLite](03-talking-to-sqlite.md) | [Contents](00-index.md) | Next: [Configuration and the App](05-configuration-and-app.md)

# Chapter 3: Making Output Readable

`{:#?}` was never going to be the real output format. This chapter builds
`Renderer`, all seven of SQLiteForge's output modes, and fixes the
multi-statement bug from the last chapter — ending with a genuinely useful
command-line tool, before we've written any interactive code at all.

## Step 3.1 — One mode: `box`

**File:** `src/renderer/mod.rs`
```diff
+use crate::database::QueryResult;
+use unicode_width::UnicodeWidthStr;
+
+/// Render query results in different output modes
+pub struct Renderer;
+
+impl Renderer {
+    pub fn render(result: &QueryResult, mode: &str, headers: bool, nullvalue: &str) -> String {
+        if !result.is_select || result.columns.is_empty() {
+            return Self::render_modify_result(result);
+        }
+        match mode {
+            "box" => Self::render_box(result, headers, nullvalue),
+            _ => Self::render_box(result, headers, nullvalue),
+        }
+    }
+
+    fn render_modify_result(result: &QueryResult) -> String {
+        format!("Changes: {}\nExecution Time: {:.1} ms",
+            result.rows_affected, result.execution_time_ms)
+    }
+
+    fn column_widths(result: &QueryResult, _nullvalue: &str) -> Vec<usize> {
+        let mut widths: Vec<usize> = result.columns.iter()
+            .map(|c| UnicodeWidthStr::width(c.as_str())).collect();
+        for row in &result.rows {
+            for (i, val) in row.iter().enumerate() {
+                if i < widths.len() {
+                    let w = UnicodeWidthStr::width(val.as_str());
+                    if w > widths[i] { widths[i] = w; }
+                }
+            }
+        }
+        widths
+    }
+
+    fn render_box(result: &QueryResult, headers: bool, nullvalue: &str) -> String {
+        let widths = Self::column_widths(result, nullvalue);
+        let mut out = String::new();
+        out.push('┌');
+        for (i, w) in widths.iter().enumerate() {
+            out.push_str(&"─".repeat(w + 2));
+            if i < widths.len() - 1 { out.push('┬'); }
+        }
+        out.push_str("┐\n");
+        if headers {
+            out.push('│');
+            for (i, col) in result.columns.iter().enumerate() {
+                let pad = widths[i] - UnicodeWidthStr::width(col.as_str());
+                out.push(' '); out.push_str(col); out.push_str(&" ".repeat(pad)); out.push_str(" │");
+            }
+            out.push('\n');
+            out.push('├');
+            for (i, w) in widths.iter().enumerate() {
+                out.push_str(&"─".repeat(w + 2));
+                if i < widths.len() - 1 { out.push('┼'); }
+            }
+            out.push_str("┤\n");
+        }
+        for row in &result.rows {
+            out.push('│');
+            for (i, val) in row.iter().enumerate() {
+                let w = if i < widths.len() { widths[i] } else { 0 };
+                let display_width = UnicodeWidthStr::width(val.as_str());
+                let pad = if w >= display_width { w - display_width } else { 0 };
+                out.push(' '); out.push_str(val); out.push_str(&" ".repeat(pad)); out.push_str(" │");
+            }
+            out.push('\n');
+        }
+        out.push('└');
+        for (i, w) in widths.iter().enumerate() {
+            out.push_str(&"─".repeat(w + 2));
+            if i < widths.len() - 1 { out.push('┴'); }
+        }
+        out.push_str("┘\n");
+        out.push_str(&format!("Rows: {}  Execution Time: {:.1} ms",
+            result.rows.len(), result.execution_time_ms));
+        out
+    }
+}
```

Notice `render` and every renderer now also take a `nullvalue: &str`
parameter we don't have a real value for yet — there's no config to hold a
custom NULL string until next chapter. We're adding the parameter now
because it's already part of the real function signature we're rebuilding
toward, and passing a placeholder through today is cheaper than threading a
new parameter through seven renderer functions later. For now, every call
site just passes `""`. Keep an eye on `column_widths`, though: its
parameter is named `_nullvalue`, underscore and all — that's Rust telling
you it's declared but never read. This one's real, in the shipped
SQLiteForge, not just in our reconstruction: the parameter is threaded all
the way through every render function and never actually changes what gets
printed anywhere. We're reproducing that exactly, warning included, rather
than quietly "fixing" it into something that looks more useful than the
real code is.

Wired into `main.rs` in place of the debug print:

**File:** `src/main.rs`
```diff
+mod renderer;
+use renderer::Renderer;
 ...
     if let Some(sql) = &cli.command {
         match db.execute_query(sql) {
-            Ok(result) => println!("{:#?}", result),
+            Ok(result) => println!("{}", Renderer::render(&result, "box", true, "")),
             Err(e) => eprintln!("Error: {}", e),
         }
     }
```

**Verified:** `cargo run -- -c "SELECT 1 as one, 'hi' as two"` now prints:

```
┌─────┬─────┐
│ one │ two │
├─────┼─────┤
│ 1   │ hi  │
└─────┴─────┘
Rows: 1  Execution Time: 0.0 ms
```

Every renderer computes column widths with `unicode_width::UnicodeWidthStr`,
not `.len()`. It's tempting to reach for `.len()` here since almost all test
data is ASCII and the two agree — but `.len()` counts *bytes*, not glyphs.
The day a table has a name column with "café" or "日本語" in it, `.len()`
and the on-screen width diverge, and every box border after that row would
be misaligned by exactly the difference. Handling it now, before we've ever
hit that bug, is cheaper than debugging misaligned tables from a bug report
later.

## Step 3.2 — The other six modes

The remaining modes are minor variations on the same column-width
computation: different border characters, or none at all. `table` is `box`
with ASCII `+-|` instead of Unicode box-drawing (for terminals or fonts that
don't render box-drawing well); `column` drops borders entirely; `markdown`
produces a GitHub-flavored table; `csv` and `list` are single-pass,
delimiter-joined output with no width computation at all; `json` tries to
distinguish numbers from strings by attempting to parse each value.

**File:** `src/renderer/mod.rs`
```diff
     pub fn render(result: &QueryResult, mode: &str, headers: bool, nullvalue: &str) -> String {
         ...
         match mode {
             "box" => Self::render_box(result, headers, nullvalue),
+            "table" => Self::render_table(result, headers, nullvalue),
+            "column" => Self::render_column(result, headers, nullvalue),
+            "markdown" => Self::render_markdown(result, headers, nullvalue),
+            "csv" => Self::render_csv(result, headers),
+            "json" => Self::render_json(result),
+            "list" => Self::render_list(result, headers),
             _ => Self::render_box(result, headers, nullvalue),
         }
     }
+    { ... render_table, render_column, render_markdown: take the same
+          nullvalue parameter and same column_widths call as render_box,
+          different border characters ... }
+    fn render_csv(result: &QueryResult, headers: bool) -> String {
+        let mut out = String::new();
+        if headers { out.push_str(&result.columns.join(",")); out.push('\n'); }
+        for row in &result.rows {
+            let escaped: Vec<String> = row.iter().map(|v| {
+                if v.contains(',') || v.contains('"') || v.contains('\n') {
+                    format!("\"{}\"", v.replace('"', "\"\""))
+                } else { v.clone() }
+            }).collect();
+            out.push_str(&escaped.join(",")); out.push('\n');
+        }
+        out
+    }
+    fn render_json(result: &QueryResult) -> String {
+        let mut out = String::from("[\n");
+        for (row_idx, row) in result.rows.iter().enumerate() {
+            out.push_str("  {");
+            for (i, val) in row.iter().enumerate() {
+                if i > 0 { out.push_str(", "); }
+                let col = &result.columns[i];
+                if val.is_empty() {
+                    out.push_str(&format!("\"{}\": null", col));
+                } else if let Ok(n) = val.parse::<i64>() {
+                    out.push_str(&format!("\"{}\": {}", col, n));
+                } else if let Ok(f) = val.parse::<f64>() {
+                    out.push_str(&format!("\"{}\": {}", col, f));
+                } else {
+                    let escaped = val.replace('\\', "\\\\").replace('"', "\\\"")
+                        .replace('\n', "\\n").replace('\r', "\\r").replace('\t', "\\t");
+                    out.push_str(&format!("\"{}\": \"{}\"", col, escaped));
+                }
+            }
+            out.push('}');
+            if row_idx < result.rows.len() - 1 { out.push(','); }
+            out.push('\n');
+        }
+        out.push(']'); out
+    }
+    fn render_list(result: &QueryResult, headers: bool) -> String {
+        let mut out = String::new();
+        if headers { out.push_str(&result.columns.join("|")); out.push('\n'); }
+        for row in &result.rows { out.push_str(&row.join("|")); out.push('\n'); }
+        out
+    }
```

**Verified:** compiles; each mode produces visibly different, correctly
formatted output against the same `SELECT` result.

`render_json`'s number detection is a small, honest compromise: since every
value already arrived as a `String` (Chapter 2's decision), there's no way
to tell "the number `1`" from "the string `\"1\"`" except by trying to parse
it back. `val.parse::<i64>()` succeeding means it's *probably* a number —
but a genuine text column containing the digits `"1"` would be emitted as an
unquoted JSON number too. This is the cost of the uniform-string design from
Chapter 2 finally showing up on-screen, rather than staying an abstract
tradeoff.

## Step 3.3 — Splitting scripts by semicolon, and `-m`/`-f`

This is the fix for the multi-statement bug from the end of the last
chapter: split on `;` *before* calling `execute_query`, so each call only
ever sees one statement.

**File:** `src/main.rs`
```diff
 struct Cli {
     database: Option<String>,
     command: Option<String>,
+    /// Read and execute SQL from file
+    #[arg(short = 'f', long = "file")]
+    file: Option<String>,
+    /// Output mode (box, table, column, markdown, csv, json, list)
+    #[arg(short = 'm', long = "mode")]
+    mode: Option<String>,
 }
 ...
     println!("Connected to: {}", db.path);
-    if let Some(sql) = &cli.command {
-        match db.execute_query(sql) {
-            Ok(result) => println!("{}", Renderer::render(&result, "box", true, "")),
-            Err(e) => eprintln!("Error: {}", e),
-        }
-    }
+    let mode = cli.mode.as_deref().unwrap_or("box");
+    if let Some(sql) = &cli.command {
+        execute_noninteractive(&db, mode, sql);
+    } else if let Some(path) = &cli.file {
+        let content = std::fs::read_to_string(path).expect("failed to read file");
+        execute_noninteractive(&db, mode, &content);
+    }
 }
+
+/// Split a script into individual statements and run each one through the renderer.
+fn execute_noninteractive(db: &Database, mode: &str, sql: &str) {
+    // Split by semicolons for multiple statements
+    for statement in sql.split(';') {
+        let trimmed = statement.trim();
+        if trimmed.is_empty() { continue; }
+
+        let full_stmt = format!("{};", trimmed);
+        match db.execute_query(&full_stmt) {
+            Ok(result) => println!("{}", Renderer::render(&result, mode, true, "")),
+            Err(e) => eprintln!("Error: {}", e),
+        }
+    }
+}
```

**Verified:**

```
$ cargo run -- -c "CREATE TABLE t(a INTEGER, b TEXT); INSERT INTO t VALUES (1, 'hi'); SELECT * FROM t" -m json
Changes: 0
Execution Time: 0.3 ms
Changes: 1
Execution Time: 0.0 ms
[
  {"a": 1, "b": "hi"}
]
```

All three statements ran, each rendered independently, and `SELECT`'s rows
finally showed up. `sql.split(';')` is naive — a semicolon inside a string
literal (`INSERT INTO t VALUES('a;b')`) would split in the wrong place — but
it's the same shortcut the real SQLiteForge ships with, so we're keeping it
rather than "fixing" something that isn't actually broken in the original.

One more small, deliberate detail: `execute_noninteractive` reconstructs
`full_stmt` by gluing a semicolon back onto `trimmed` before calling
`execute_query`. Since `execute_query`'s own `is_select` check only looks at
the *start* of the string, a trailing `;` makes no difference to it —
rusqlite is equally happy with or without one. Re-adding it here doesn't
change behavior at all; it's a defensive habit (never hand a downstream SQL
engine a statement that looks truncated) more than a necessity, and we're
keeping it because it's what the real function does, not because we could
demonstrate a case where it matters.

At this point `sqliteforge mydb.db -c "..." -m csv` is a complete,
non-interactive SQL tool. Everything from here on is about turning it into
something worth typing queries into by hand — and, along the way, into
`execute_noninteractive`'s final, config-aware form, which we'll finish in
Chapter 14.

Next: [Chapter 4 — Configuration and the App](05-configuration-and-app.md)
