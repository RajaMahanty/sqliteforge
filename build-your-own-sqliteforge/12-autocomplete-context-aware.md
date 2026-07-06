Previous: [Autocomplete, Part 1](11-autocomplete-basics.md) | [Contents](00-index.md) | Next: [Editing Ergonomics](13-editing-ergonomics.md)

# Chapter 11: Autocomplete, Part 2

Last chapter's completer can't tell `SELECT |` from `FROM |` — both just see
an empty word and (for now) return nothing. This chapter teaches it to look
at the token immediately before the cursor and use that to decide what kind
of thing belongs next: a keyword from one of six different keyword sets, a
table, a column, an index, or nothing at all. Then we add `table.column` and
quoted `"table"."col"` completion, and finally the config knobs that let a
user tune all of it. By the end of this chapter, `completion/mod.rs` matches
the real, shipped file exactly.

## Step 11.1 — Six keyword sets and a big `match`

SQLite's grammar wants a different vocabulary depending on where you are.
The real file defines six keyword arrays; we're showing the real first few
entries of each and their real approximate sizes rather than every string,
since there's nothing to explain in a keyword list beyond "these are SQL
keywords":

```diff
+/// Keywords valid in expressions (after SELECT, WHERE, ON, HAVING, etc.)
+const EXPR_KEYWORDS: &[&str] = &[
+    "ALL", "AND", "AS", "ASC", "BETWEEN", "CASE", "CAST", "CURRENT_DATE",
+    /* ...operators, CASE/WHEN, and every scalar/aggregate/date function
+       SQLite ships with -- COUNT, SUBSTR, STRFTIME, and so on.
+       ~90 entries in total... */
+    "INTEGER", "REAL", "TEXT", "BLOB", "NUMERIC", "BOOLEAN",
+];
+/// Keywords valid after a table/value (clause-level transitions)
+const CLAUSE_KEYWORDS: &[&str] = &[
+    "AND", "AS", "ASC", "BY", "CROSS", "DESC", "EXCEPT", "FROM", "FULL",
+    "GROUP", "HAVING", /* ...26 entries covering JOIN variants, ORDER/GROUP
+       BY, set operations... */ "WHERE",
+];
+/// Keywords to start a new statement (when nothing or general context)
+const STMT_KEYWORDS: &[&str] = &[
+    "ALTER", "ANALYZE", "ATTACH", "BEGIN", "COMMIT", "CREATE", "DELETE",
+    /* ...21 entries: every statement-starting keyword... */ "WITH",
+];
+/// DDL-specific keywords (after CREATE/ALTER/DROP)
+const DDL_KEYWORDS: &[&str] = &[
+    "TABLE", "INDEX", "VIEW", "TRIGGER", "VIRTUAL", "TEMP", "TEMPORARY",
+    /* ...about 40 entries: constraints, conflict clauses, column
+       modifiers... */ "EXCLUSIVE", "WITHOUT",
+];
+/// Column constraint keywords (inside CREATE TABLE parentheses)
+const COLUMN_DEF_KEYWORDS: &[&str] = &[
+    "PRIMARY", "KEY", "NOT", "NULL", "UNIQUE", "CHECK", "DEFAULT",
+    /* ...about 40 entries: constraint keywords plus every column type
+       name, including SQL-standard aliases like VARCHAR and BIGINT
+       that SQLite accepts but doesn't strictly need... */ "TIMESTAMP",
+];
+/// INSERT-specific keywords
+const INSERT_KEYWORDS: &[&str] = &[
+    "INTO", "VALUES", "DEFAULT", "SELECT", "OR", "REPLACE", "ABORT",
+    /* ...18 entries, including the ON CONFLICT clause... */ "RETURNING",
+];
+
+/// Which keyword set to use
+#[derive(Clone, Copy)]
+enum KeywordScope {
+    /// After SELECT, WHERE, ON, HAVING -- expression keywords
+    Expression,
+    /// After a table/identifier -- clause transition keywords
+    Clause,
+    /// Start of statement or general -- statement + clause + expression
+    General,
+    /// After CREATE/ALTER/DROP -- DDL keywords
+    Ddl,
+    /// Inside column definitions (CREATE TABLE body)
+    ColumnDef,
+    /// After INSERT
+    Insert,
+    /// No keywords (e.g. directly after FROM/JOIN)
+    None,
+}
+
+/// What types of completions are allowed in this context
+struct CompletionContext {
+    keyword_scope: KeywordScope,
+    allow_tables: bool,
+    allow_columns: bool,
+    allow_indices: bool,
+    /// Allow suggestions even when the current word is empty
+    eager: bool,
+}
```

Six scopes, not one flat keyword list, is the whole design: `Ddl` and
`ColumnDef` overlap heavily (both are about `CREATE TABLE` syntax) but are
kept separate because they apply in genuinely different places — `Ddl`
right after `CREATE`, `ColumnDef` once you're inside the table's own
parentheses. `General`, used at the very start of a statement, is the union
of `STMT_KEYWORDS`, `CLAUSE_KEYWORDS`, and `EXPR_KEYWORDS` — because at the
start of a line, a `SELECT`, an `AND` (inside a larger script pasted in),
and a bare column name are all still grammatically possible.

```diff
 impl SqlCompleter {
     ...
+    fn keywords_for_scope(scope: KeywordScope) -> Vec<&'static str> {
+        match scope {
+            KeywordScope::Expression => EXPR_KEYWORDS.to_vec(),
+            KeywordScope::Clause => CLAUSE_KEYWORDS.to_vec(),
+            KeywordScope::Ddl => DDL_KEYWORDS.to_vec(),
+            KeywordScope::ColumnDef => COLUMN_DEF_KEYWORDS.to_vec(),
+            KeywordScope::Insert => INSERT_KEYWORDS.to_vec(),
+            KeywordScope::General => {
+                let mut all = Vec::new();
+                all.extend_from_slice(STMT_KEYWORDS);
+                all.extend_from_slice(CLAUSE_KEYWORDS);
+                all.extend_from_slice(EXPR_KEYWORDS);
+                all.sort_unstable();
+                all.dedup();
+                all
+            }
+            KeywordScope::None => Vec::new(),
+        }
+    }
+
+    /// Detect the SQL context by looking at the keyword preceding the cursor
+    fn detect_context(line_to_cursor: &str, word_start: usize) -> CompletionContext {
+        if Self::is_inside_create_table_body(line_to_cursor) {
+            return CompletionContext {
+                keyword_scope: KeywordScope::ColumnDef, allow_tables: false,
+                allow_columns: false, allow_indices: false, eager: false,
+            };
+        }
+        let before = line_to_cursor[..word_start].trim_end();
+        let before = before.strip_suffix(',').unwrap_or(before).trim_end();
+        let prev_keyword = Self::find_prev_keyword(before);
+
+        match prev_keyword.as_deref() {
+            Some("FROM") | Some("JOIN") => CompletionContext {
+                keyword_scope: KeywordScope::None, allow_tables: true,
+                allow_columns: false, allow_indices: false, eager: true,
+            },
+            Some("INTO") | Some("TABLE") | Some("UPDATE") => CompletionContext {
+                keyword_scope: KeywordScope::None, allow_tables: true,
+                allow_columns: false, allow_indices: false, eager: true,
+            },
+            Some("DELETE") => CompletionContext {
+                keyword_scope: KeywordScope::Clause, allow_tables: false,
+                allow_columns: false, allow_indices: false, eager: true,
+            },
+            Some("INSERT") => CompletionContext {
+                keyword_scope: KeywordScope::Insert, allow_tables: false,
+                allow_columns: false, allow_indices: false, eager: true,
+            },
+            Some("SELECT") | Some("HAVING") => CompletionContext {
+                keyword_scope: KeywordScope::Expression, allow_tables: true,
+                allow_columns: true, allow_indices: false, eager: true,
+            },
+            Some("WHERE") | Some("ON") | Some("SET") | Some("AND") | Some("OR")
+            | Some("BETWEEN") | Some("CASE") | Some("WHEN") | Some("THEN") | Some("ELSE")
+            | Some("LIKE") | Some("IN") | Some("VALUES") => CompletionContext {
+                keyword_scope: KeywordScope::Expression, allow_tables: true,
+                allow_columns: true, allow_indices: false, eager: true,
+            },
+            Some("CREATE") | Some("ALTER") | Some("DROP") => CompletionContext {
+                keyword_scope: KeywordScope::Ddl, allow_tables: false,
+                allow_columns: false, allow_indices: false, eager: true,
+            },
+            Some("INDEX") => CompletionContext {
+                keyword_scope: KeywordScope::None, allow_tables: false,
+                allow_columns: false, allow_indices: true, eager: true,
+            },
+            Some("ORDER") | Some("GROUP") => CompletionContext {
+                keyword_scope: KeywordScope::Expression, allow_tables: false,
+                allow_columns: true, allow_indices: false, eager: true,
+            },
+            Some("BY") => CompletionContext {
+                keyword_scope: KeywordScope::Expression, allow_tables: true,
+                allow_columns: true, allow_indices: false, eager: true,
+            },
+            Some("LIMIT") | Some("OFFSET") => CompletionContext {
+                keyword_scope: KeywordScope::Expression, allow_tables: false,
+                allow_columns: true, allow_indices: false, eager: true,
+            },
+            Some("INNER") | Some("LEFT") | Some("RIGHT") | Some("CROSS") | Some("NATURAL")
+            | Some("OUTER") | Some("FULL") => CompletionContext {
+                keyword_scope: KeywordScope::None, allow_tables: true,
+                allow_columns: false, allow_indices: false, eager: true,
+            },
+            Some("AS") | Some("PRAGMA") => CompletionContext {
+                keyword_scope: KeywordScope::None, allow_tables: false,
+                allow_columns: false, allow_indices: false, eager: false,
+            },
+            Some("IF") => CompletionContext {
+                keyword_scope: KeywordScope::Ddl, allow_tables: false,
+                allow_columns: false, allow_indices: false, eager: true,
+            },
+            Some("EXISTS") => CompletionContext {
+                keyword_scope: KeywordScope::None, allow_tables: true,
+                allow_columns: false, allow_indices: false, eager: true,
+            },
+            Some("ADD") => CompletionContext {
+                keyword_scope: KeywordScope::ColumnDef, allow_tables: false,
+                allow_columns: false, allow_indices: false, eager: true,
+            },
+            Some("RENAME") => CompletionContext {
+                keyword_scope: KeywordScope::Ddl, allow_tables: false,
+                allow_columns: true, allow_indices: false, eager: true,
+            },
+            None => {
+                let all_before = line_to_cursor[..word_start].trim();
+                if all_before.is_empty() {
+                    CompletionContext {
+                        keyword_scope: KeywordScope::General, allow_tables: false,
+                        allow_columns: false, allow_indices: false, eager: false,
+                    }
+                } else {
+                    CompletionContext {
+                        keyword_scope: KeywordScope::Clause, allow_tables: true,
+                        allow_columns: true, allow_indices: false, eager: false,
+                    }
+                }
+            }
+            _ => CompletionContext {
+                keyword_scope: KeywordScope::Clause, allow_tables: true,
+                allow_columns: true, allow_indices: false, eager: false,
+            },
+        }
+    }
+
+    /// Check the immediately preceding token. If it's a SQL keyword, return it.
+    /// If it's a quoted identifier or non-keyword, return None (general context).
+    fn find_prev_keyword(text: &str) -> Option<String> {
+        let text = text.trim_end();
+        if text.is_empty() { return None; }
+        let bytes = text.as_bytes();
+        let mut i = bytes.len();
+        while i > 0 && bytes[i - 1].is_ascii_whitespace() { i -= 1; }
+        if i == 0 { return None; }
+        let last = bytes[i - 1];
+        if last == b'"' || last == b')' { return None; }
+        let end = i;
+        while i > 0 && !bytes[i - 1].is_ascii_whitespace()
+            && bytes[i - 1] != b'(' && bytes[i - 1] != b',' && bytes[i - 1] != b')' {
+            i -= 1;
+        }
+        let token = &text[i..end];
+        let upper = token.to_uppercase();
+        let context_keywords = [
+            "SELECT", "FROM", "WHERE", "JOIN", "INNER", "LEFT", "RIGHT", "CROSS", "NATURAL",
+            "OUTER", "FULL", "ON", "INTO", "TABLE", "UPDATE", "SET", "HAVING", "BY", "INDEX",
+            "AND", "OR", "BETWEEN", "CASE", "WHEN", "THEN", "ELSE", "LIKE", "ORDER", "GROUP",
+            "DELETE", "INSERT", "VALUES", "CREATE", "ALTER", "DROP", "IN", "AS", "PRAGMA", "LIMIT",
+            "OFFSET", "IF", "EXISTS", "NOT", "ADD", "RENAME",
+        ];
+        if context_keywords.contains(&upper.as_str()) { Some(upper) } else { None }
+    }
 }
```

**Verified** in a real terminal, immediately after `SELECT<space><Tab>`:

```
AND OR NOT IN LIKE BETWEEN IS NULL CASE WHEN THEN ELSE END DISTINCT ...
id                                     (column)
name                                   (column)
users                                  (table)
```

`find_prev_keyword` is deliberately conservative: it only recognizes tokens
from its own fixed `context_keywords` list, and returns `None` for anything
else — including a quoted identifier or a `)`. That `None` doesn't mean "no
context"; look at `detect_context`'s own `None` arm: it means "the token
before the cursor was a plain identifier, not a keyword," which is the
signal for `Clause`-scope, table-and-column suggestions — the state you're
in right after typing a table name and a space, deciding whether to type
`WHERE`, a join, or nothing else at all.

`CompletionContext.eager` is what makes `SELECT |` (nothing typed yet after
the keyword) show suggestions unprompted, while a bare `|` at the very
start of a fresh line stays silent until you type a character. Reedline
calls `complete()` on *every* cursor move, not just on Tab — if every
context showed every possible completion the moment the word was empty, the
menu would be popping up constantly, including places where it's just
noise (mid-alias, say). `eager` is the one bit that separates "SQL grammar
makes it obvious what goes here" from "there are too many valid next tokens
to guess usefully."

## Step 11.2 — Recognizing "we're inside `CREATE TABLE (...)`"

`ColumnDef` scope needs its own detection, separate from the keyword-lookup
above, because "am I inside the parentheses of a `CREATE TABLE`" isn't a
question about the *previous token* — it's a question about how many
parentheses have opened and closed since the last `CREATE TABLE` appeared
anywhere earlier on the line:

```diff
 impl SqlCompleter {
+    /// Check if we are inside CREATE TABLE parentheses
+    fn is_inside_create_table_body(line_to_cursor: &str) -> bool {
+        let upper = line_to_cursor.to_uppercase();
+        if let Some(ct_pos) = upper.rfind("CREATE TABLE") {
+            let after_ct = &line_to_cursor[ct_pos..];
+            let open_parens = after_ct.matches('(').count();
+            let close_parens = after_ct.matches(')').count();
+            return open_parens > close_parens;
+        }
+        if let Some(ct_pos) = upper.rfind("CREATE TEMP TABLE")
+            .or_else(|| upper.rfind("CREATE TEMPORARY TABLE")) {
+            let after_ct = &line_to_cursor[ct_pos..];
+            let open_parens = after_ct.matches('(').count();
+            let close_parens = after_ct.matches(')').count();
+            return open_parens > close_parens;
+        }
+        false
+    }
     ...
```

This is why `detect_context` checks it *first*, before looking at the
previous keyword at all: inside `CREATE TABLE users (id INTEGER, |`, the
token right before the cursor is a comma, which `find_prev_keyword` would
otherwise just shrug off as "not a keyword" and fall through to general
`Clause` suggestions — tables and columns, which is exactly wrong three
levels deep into a column list that hasn't been created yet.

**Verified** in a real terminal: `CREATE TABLE t(a INTEGER, ` followed by
Tab now suggests `PRIMARY`, `NOT`, `DEFAULT`, and the rest of
`COLUMN_DEF_KEYWORDS` — not table or column names from elsewhere in the
database.

## Step 11.3 — `table.column` and quoted `"table"."col"`

Typing `users.` (or `u.` if aliased `AS u`) should suggest that specific
table's columns. The real version handles both a bare alias (`users.`) and
a quoted identifier (`"users".`) — the quoting matters because a table name
with spaces or reserved words in it has to be quoted in the SQL itself, and
the cursor could legitimately be sitting right after that closing quote:

```diff
 impl SqlCompleter {
+    /// Try to detect a "table"."col" pattern and return table-qualified column suggestions.
+    fn try_qualified_completion(&self, line_to_cursor: &str, pos: usize) -> Option<Vec<Suggestion>> {
+        let before = line_to_cursor;
+        let dot_pos = before.rfind('.')?;
+        let after_dot = &before[dot_pos + 1..];
+        let (col_prefix, col_has_quote) = if after_dot.starts_with('"') {
+            (&after_dot[1..], true)
+        } else { (after_dot, false) };
+
+        let before_dot = &before[..dot_pos];
+        if !before_dot.ends_with('"') {
+            let alias_start = before_dot
+                .rfind(|c: char| c.is_whitespace() || c == '(' || c == ',')
+                .map(|i| i + 1).unwrap_or(0);
+            let alias = &before_dot[alias_start..];
+            if alias.is_empty() { return None; }
+            let span_start = dot_pos + 1;
+            let alias_lower = alias.to_lowercase();
+            for (table_name, cols) in &self.table_columns {
+                if table_name.to_lowercase() == alias_lower {
+                    return Some(self.build_column_suggestions(cols, col_prefix, span_start, pos, col_has_quote));
+                }
+            }
+            return Some(self.build_column_suggestions(&self.all_columns, col_prefix, span_start, pos, col_has_quote));
+        }
+
+        let table_end = before_dot.len() - 1;
+        let table_start = before_dot[..table_end].rfind('"')?;
+        let table_name = &before_dot[table_start + 1..table_end];
+        let span_start = dot_pos + 1;
+        let cols = self.table_columns.get(table_name)?;
+        Some(self.build_column_suggestions(cols, col_prefix, span_start, pos, col_has_quote))
+    }
+
+    fn build_column_suggestions(
+        &self, columns: &[String], col_prefix: &str, span_start: usize, pos: usize,
+        _has_leading_quote: bool,
+    ) -> Vec<Suggestion> {
+        let lower_prefix = col_prefix.to_lowercase();
+        let quote = self.config.quote_identifiers;
+        columns.iter()
+            .filter(|col| col_prefix.is_empty() || col.to_lowercase().starts_with(&lower_prefix))
+            .map(|col| Suggestion {
+                value: if quote { format!("\"{}\"", col) } else { col.clone() },
+                description: Some("column".to_string()), style: None, extra: None,
+                span: Span::new(span_start, pos), append_whitespace: true,
+            })
+            .collect()
+    }
 }
 impl Completer for SqlCompleter {
     fn complete(&mut self, line: &str, pos: usize) -> Vec<Suggestion> {
         { ... dot command completion ... }
+
+        // Try table-qualified column completion first: "table"."col" or alias.col
+        if let Some(suggestions) = self.try_qualified_completion(line_to_cursor, pos) {
+            if !suggestions.is_empty() { return self.cap_suggestions(suggestions); }
+        }
+
         let word_start = ...
```

Notice `try_qualified_completion` doesn't verify that `alias` is a real
table name, an alias defined earlier in the statement, or neither — if the
lookup in `self.table_columns` fails, it falls back to suggesting from
*every* column in the database (`self.all_columns`) rather than returning
nothing. That's a deliberate "fail open": a genuine alias like `u` in
`FROM users u` will never appear in `table_columns` (which is keyed by real
table names, not aliases), so without the fallback, `u.` would suggest
nothing at all. Suggesting too much is recoverable — you ignore the wrong
entries; suggesting nothing looks like the feature is broken.

`refresh_completer` in `shell/mod.rs` already builds the `table_columns`
map this needs — we added it back in Step 10.2, one chapter ahead of
having anything that used it.

**Verified** in a real terminal: after `CREATE TABLE users(id INTEGER, name
TEXT);`, typing `SELECT users.` and pressing Tab shows only `id` and `name`.

## Step 11.4 — Config-driven tuning, and capping suggestions

Everything so far is hardcoded: quote nothing, suggest everything, never
turn it off. `CompletionConfig` gives a user a TOML section for all of it —
more knobs than we need to explain individually, since each one is a single
`if` or ternary at its use site:

```diff
+/// Autocompletion configuration
+#[derive(Debug, Clone, Serialize, Deserialize)]
+pub struct CompletionConfig {
+    /// Enable autocompletion (default: true)
+    #[serde(default = "default_true")] pub enabled: bool,
+    /// Show completions eagerly after keywords like SELECT, FROM, etc. (default: true)
+    #[serde(default = "default_true")] pub eager_hint: bool,
+    /// Number of columns in the completion menu (default: 4)
+    #[serde(default = "default_menu_columns")] pub menu_columns: u16,
+    /// Column padding in completion menu (default: 2)
+    #[serde(default = "default_menu_padding")] pub menu_padding: usize,
+    /// Quote identifiers in completions (default: true)
+    #[serde(default = "default_true")] pub quote_identifiers: bool,
+    /// Include column suggestions in general context (default: true)
+    #[serde(default = "default_true")] pub suggest_columns: bool,
+    /// Include keyword suggestions (default: true)
+    #[serde(default = "default_true")] pub suggest_keywords: bool,
+    /// Maximum number of suggestions shown (default: 50)
+    #[serde(default = "default_max_suggestions")] pub max_suggestions: usize,
+}
+// ...default_menu_columns() -> 4, default_menu_padding() -> 2,
+//    default_max_suggestions() -> 50, and a Default impl assembling them all
```

```diff
+use crate::config::CompletionConfig;
 pub struct SqlCompleter {
     ...
     indices: Vec<String>,
+    /// Completion configuration
+    config: CompletionConfig,
 }
 impl SqlCompleter {
-    pub fn new() -> Self { ... }
+    pub fn new() -> Self { Self::with_config(&CompletionConfig::default()) }
+    pub fn with_config(config: &CompletionConfig) -> Self {
+        Self { /* ...same fields as before... */ config: config.clone() }
+    }
     ...
 impl Completer for SqlCompleter {
     fn complete(&mut self, line: &str, pos: usize) -> Vec<Suggestion> {
+        if !self.config.enabled { return Vec::new(); }
         let line_to_cursor = &line[..pos];
         { ... dot command completion, then try_qualified_completion ... }
-        let word_start = line_to_cursor
-            .rfind(|c: char| c.is_whitespace() || c == '(' || c == ',')
-            .map(|i| i + 1).unwrap_or(0);
-        let current_word = &line_to_cursor[word_start..];
+        let word_start = line_to_cursor
+            .rfind(|c: char| c.is_whitespace() || c == '(' || c == ',')
+            .map(|i| i + 1).unwrap_or(0);
+        let raw_word = &line_to_cursor[word_start..];
+
+        // A word starting with a double quote needs the quote stripped
+        // before we compare it against table/column names.
+        let (has_leading_quote, current_word, span_start) = if raw_word.starts_with('"') {
+            (true, &raw_word[1..], word_start)
+        } else {
+            (false, raw_word, word_start)
+        };
+
         let context = Self::detect_context(line_to_cursor, word_start);
+
+        // If the word is empty and context is not eager, don't show suggestions
+        // (unless we're in an eager context like after FROM, SELECT, etc.)
+        if current_word.is_empty() && !has_leading_quote && !context.eager {
+            return Vec::new();
+        }
         ...
+        if !has_leading_quote && self.config.suggest_keywords {
+            for kw in Self::keywords_for_scope(context.keyword_scope) { /* ... */ }
+        }
         if context.allow_tables {
             for table in &self.tables {
                 { ... push Suggestion, value quoted per self.config.quote_identifiers ... }
             }
         }
+        if context.allow_columns && self.config.suggest_columns {
             for col in &self.all_columns { { ... same shape ... } }
+        }
+        if context.allow_indices {
+            for idx in &self.indices { { ... same shape, description "index" ... } }
+        }
-        suggestions
+        self.cap_suggestions(suggestions)
     }
 }
+impl SqlCompleter {
+    /// Cap the number of suggestions to the configured maximum
+    fn cap_suggestions(&self, mut suggestions: Vec<Suggestion>) -> Vec<Suggestion> {
+        suggestions.truncate(self.config.max_suggestions);
+        suggestions
+    }
+}
```

`shell::run` picks `with_config(&config.completion)` over `new()` wherever
it builds a completer, including the DDL-triggered rebuild from Step 10.2,
and passes `config.completion.menu_columns` / `menu_padding` into the
`ColumnarMenu` builder:

```diff
-    let completion_menu = Box::new(ColumnarMenu::default().with_name("completion_menu"));
+    let completion_menu = Box::new(
+        ColumnarMenu::default()
+            .with_name("completion_menu")
+            .with_columns(config.completion.menu_columns)
+            .with_column_padding(config.completion.menu_padding)
+            .with_marker(""),
+    );
```

And the reedline builder chain picks up a few more options now that the
completer is config-aware:

```diff
     let mut line_editor = Reedline::create()
         .with_history(reedline_history)
+        .with_history_exclusion_prefix(Some("__".to_string()))
         .with_completer(Box::new(completer))
         .with_menu(ReedlineMenu::EngineCompleter(completion_menu))
         .with_highlighter(Box::new(SqlHighlighter))
         .with_validator(Box::new(SqlValidator))
         .with_edit_mode(edit_mode)
+        .with_ansi_colors(true)
+        .with_quick_completions(true)
+        .with_partial_completions(true);
```

`.with_history_exclusion_prefix(Some("__".to_string()))` keeps every
internal host command (the `__char_close_paren__` strings we'll add in
Chapter 12) out of Up-arrow recall — without it, pressing Up after an
auto-close keystroke would recall the literal string `__char_close_paren__`
instead of your last real query. `.with_quick_completions(true)` and
`.with_partial_completions(true)` enable reedline's built-in inline
completion hints alongside our menu.

**Verified**: with `quote_identifiers = true` in `config.toml`, completing
`us` after `FROM` inserts `"users"` (quoted) instead of `users`.

That's the completer finished — six keyword scopes, `CREATE TABLE` body
detection, qualified and quoted column completion, and config-driven
tuning, in that order because each one only made sense once the piece
before it existed to build on. Next we turn to something orthogonal: making
the raw experience of *typing* SQL more pleasant, independent of what gets
suggested.

Next: [Chapter 12 — Editing Ergonomics](13-editing-ergonomics.md)
