Previous: [The Skeleton](02-the-skeleton.md) | [Contents](00-index.md) | Next: [Making Output Readable](04-making-output-readable.md)

# Chapter 2: Talking to SQLite

Time to give `database: Option<String>` somewhere to point. This chapter
builds `Database`, a thin wrapper around a `rusqlite::Connection`, and gets
to the point where we can run one hardcoded query and see real rows come
back — before we've written a single line of rendering or shell code.

## Step 2.1 — Open a connection, print its path

```diff
[dependencies]
clap = { version = "4", features = ["derive"] }
+rusqlite = { version = "0.31", features = ["bundled"] }
```

```diff
+use rusqlite::{Connection, Result as SqliteResult};
+use std::path::Path;
+
+/// Database connection wrapper
+pub struct Database {
+    conn: Connection,
+    pub path: String,
+}
+
+impl Database {
+    /// Open a database file (creates it if it doesn't exist)
+    pub fn open<P: AsRef<Path>>(path: P) -> SqliteResult<Self> {
+        let path_str = path.as_ref().to_string_lossy().to_string();
+        let conn = Connection::open(&path)?;
+        // Enable WAL mode for better concurrent access
+        conn.execute_batch("PRAGMA journal_mode=WAL;")?;
+        Ok(Self { conn, path: path_str })
+    }
+
+    /// Open an in-memory database
+    pub fn open_in_memory() -> SqliteResult<Self> {
+        let conn = Connection::open_in_memory()?;
+        Ok(Self { conn, path: ":memory:".to_string() })
+    }
+}
```

```diff
+mod database;
+
 use clap::Parser;
+use database::Database;
 ...
 fn main() {
     let cli = Cli::parse();
-    println!("{:?}", cli);
+
+    let db = match &cli.database {
+        Some(path) => Database::open(path).expect("failed to open database"),
+        None => Database::open_in_memory().expect("failed to create in-memory database"),
+    };
+
+    println!("Connected to: {}", db.path);
 }
```

**Verified:** compiles (with an expected `field 'conn' is never read` warning
— we haven't run a query yet, so nothing touches it). `cargo run` (no
arguments) prints `Connected to: :memory:`; `cargo run -- /tmp/test.db`
prints `Connected to: /tmp/test.db` and leaves a real SQLite file on disk.

`rusqlite`'s `bundled` feature is doing a lot of quiet work here: it compiles
SQLite's own C source straight into our binary, rather than linking against
whatever `libsqlite3` (if any) happens to be installed on the machine this
runs on. The tradeoff shows up immediately — this is the slowest `cargo
build` in the whole tutorial, because it's the first one compiling C, not
Rust. Every build after this one is fast again.

One line in `open` is easy to skip past: `conn.execute_batch("PRAGMA journal_mode=WAL;")`. SQLite defaults to a rollback journal, which briefly locks the *entire* database file for any write. Write-Ahead Logging instead appends changes to a separate `-wal` file and lets readers keep reading the old data until a checkpoint merges it back — the practical effect is that one connection writing doesn't block another connection reading, which matters the moment SQLiteForge is running against a database something else might also be touching. It costs nothing to enable and there's no reason not to turn it on unconditionally, so we do it once, in the one place every connection gets created.

## Step 2.2 — Actually run a query

An open connection isn't useful until we can ask it something. Real SQL
results need to represent two very different shapes: rows-and-columns (from
`SELECT`), or just a count of how many rows changed (from `INSERT`,
`UPDATE`, `CREATE`, ...). One struct, `QueryResult`, covers both — with a
`Vec<Vec<String>>` for rows and empty vectors when there aren't any.

```diff
-use rusqlite::{Connection, Result as SqliteResult};
+use rusqlite::{types::Value, Connection, Result as SqliteResult};
 use std::path::Path;
+use std::time::Instant;
+
+/// Result of executing a SQL query
+#[derive(Debug)]
+pub struct QueryResult {
+    pub columns: Vec<String>,
+    pub rows: Vec<Vec<String>>,
+    pub rows_affected: usize,
+    pub execution_time_ms: f64,
+    pub is_select: bool,
+}

 impl Database {
     { ... open, open_in_memory ... }
+
+    /// Execute a SQL query and return results
+    pub fn execute_query(&self, sql: &str) -> Result<QueryResult, String> {
+        let trimmed = sql.trim();
+        if trimmed.is_empty() {
+            return Err("Empty query".to_string());
+        }
+
+        let start = Instant::now();
+        let upper = trimmed.to_uppercase();
+        let is_select = upper.starts_with("SELECT")
+            || upper.starts_with("PRAGMA")
+            || upper.starts_with("EXPLAIN")
+            || upper.starts_with("WITH");
+
+        if is_select {
+            self.execute_select(trimmed, start)
+        } else {
+            self.execute_modify(trimmed, start)
+        }
+    }
+
+    fn execute_select(&self, sql: &str, start: Instant) -> Result<QueryResult, String> {
+        let mut stmt = self.conn.prepare(sql).map_err(|e| e.to_string())?;
+        let columns: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
+
+        let rows: Vec<Vec<String>> = stmt
+            .query_map([], |row| {
+                let mut values = Vec::new();
+                for i in 0..columns.len() {
+                    let val: Value = row.get_unwrap(i);
+                    let s = match val {
+                        Value::Null => String::new(),
+                        Value::Integer(i) => i.to_string(),
+                        Value::Real(f) => f.to_string(),
+                        Value::Text(s) => s,
+                        Value::Blob(b) => format!("X'{}'", hex_encode(&b)),
+                    };
+                    values.push(s);
+                }
+                Ok(values)
+            })
+            .map_err(|e| e.to_string())?
+            .filter_map(|r| r.ok())
+            .collect();
+
+        let elapsed = start.elapsed();
+        Ok(QueryResult {
+            rows_affected: rows.len(), columns, rows,
+            execution_time_ms: elapsed.as_secs_f64() * 1000.0, is_select: true,
+        })
+    }
+
+    fn execute_modify(&self, sql: &str, start: Instant) -> Result<QueryResult, String> {
+        let rows_affected = self.conn.execute_batch(sql)
+            .map_err(|e| e.to_string())
+            .map(|_| self.conn.changes())?;
+        let elapsed = start.elapsed();
+        Ok(QueryResult {
+            columns: Vec::new(), rows: Vec::new(),
+            rows_affected: rows_affected as usize,
+            execution_time_ms: elapsed.as_secs_f64() * 1000.0, is_select: false,
+        })
+    }
+}
+
+fn hex_encode(bytes: &[u8]) -> String {
+    bytes.iter().map(|b| format!("{:02X}", b)).collect()
+}
```

```diff
 struct Cli {
     database: Option<String>,
+    /// Execute SQL command and exit
+    #[arg(short = 'c', long = "command")]
+    command: Option<String>,
 }
 ...
     println!("Connected to: {}", db.path);
+
+    if let Some(sql) = &cli.command {
+        match db.execute_query(sql) {
+            Ok(result) => println!("{:#?}", result),
+            Err(e) => eprintln!("Error: {}", e),
+        }
+    }
```

**Verified:**
`cargo run -- -c "SELECT 1 as one, 'hi' as two"` prints a real `QueryResult`
with `columns: ["one", "two"]`, `rows: [["1", "hi"]]`, `is_select: true`.

Two decisions worth pausing on:

**Every value becomes a `String`, even integers.** This looks wasteful —
why not keep a `1` as an `i64`? Because `QueryResult` doesn't know yet
whether it's headed for a box-drawing table, a CSV file, or a JSON blob, and
each of those wants a different textual representation of the same value.
Rather than pushing that decision down into every renderer, `Database`
makes it once, up front, and every consumer downstream just deals in
strings. We pay for this with a `parse::<f64>()` round-trip later when the
JSON renderer wants to tell "1" the number from "1" the string — a real
cost, in exchange for one simple, uniform result type.

**`is_select` is a heuristic, not a parser.** `execute_query` decides
whether to expect rows back by checking if the trimmed, uppercased SQL
*starts with* `SELECT`, `PRAGMA`, `EXPLAIN`, or `WITH` — no real SQL parsing
happens at all. This is deliberately cheap, and it's also exactly the kind
of shortcut that breaks on inputs its author didn't think of (a query
wrapped in a leading comment, for instance). We're noting it here because
it's about to bite us:

## Step 2.3 — The one-statement-at-a-time problem

Try feeding `execute_query` more than one statement separated by `;`:

```
$ cargo run -- -c "CREATE TABLE t(a INTEGER, b TEXT); INSERT INTO t VALUES (1, 'hi'); SELECT * FROM t"
QueryResult { columns: [], rows: [], rows_affected: 1, execution_time_ms: 0.28, is_select: false }
```

**Verified** — and the verified result is a bug, not a success. `is_select`
looked only at the *first* word (`CREATE`), so the whole three-statement
script went down the `execute_modify` path. `conn.execute_batch(sql)`
happily executes all three statements against SQLite — `execute_batch` is
built to run scripts — but we only ever report the change count, and never
see the `SELECT`'s rows, because our own `is_select` check never looked past
the first keyword.

This isn't a bug in isolation, it's a design pressure: `execute_query` is
built to run *one statement*, and needs something else to be in charge of
splitting a multi-statement script into individual calls first. That's
exactly what SQLiteForge's non-interactive mode does — and it's the next
thing we build.

Next: [Chapter 3 — Making Output Readable](04-making-output-readable.md)
