Previous: [Architecture](01-architecture.md) | [Contents](00-index.md) | Next: [Talking to SQLite](03-talking-to-sqlite.md)

# Chapter 1: The Skeleton

Every chapter after this one adds a module. This one just gets a binary that
parses its own arguments — no database, no shell, nothing SQL-shaped yet.
It's the smallest possible thing we can compile and run, which means it's
also the smallest possible thing we can get *wrong*, so we start here.

## Step 1.1 — A crate that parses `clap` arguments

```diff
[package]
name = "sqliteforge"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "sqliteforge"
path = "src/main.rs"

[dependencies]
+clap = { version = "4", features = ["derive"] }
```

```diff
+use clap::Parser;
+
+/// SQLiteForge - A modern terminal-first SQLite client
+#[derive(Parser, Debug)]
+#[command(name = "sqliteforge", version = "0.1.0")]
+struct Cli {
+    /// Path to the SQLite database file (creates if not exists)
+    #[arg(value_name = "DATABASE")]
+    database: Option<String>,
+}
+
+fn main() {
+    let cli = Cli::parse();
+    println!("{:?}", cli);
+}
```

**Verified:** compiles and runs. `cargo run -- mydb.db` prints
`Cli { database: Some("mydb.db") }`; `cargo run -- --help` prints a
usage message clap generated entirely from the struct's doc comments and
attributes.

The interesting choice here is `database: Option<String>`, not `String`.
SQLiteForge can be started with no arguments at all (`sqliteforge`) and fall
back to an in-memory database — so "no database given" has to be a
representable state, not an error. `clap`'s derive macro reads that `Option`
straight off the field type: an `Option<T>` argument is optional, a bare `T`
is required. We didn't write any validation logic for that; the type *is*
the validation logic.

We're also deriving `Debug` on `Cli` purely so `println!("{:?}", cli)` has
something to print. That line won't survive past this chapter — it exists
only so we have something observable to run right now, before there's a
database to connect to. This is a pattern we'll repeat a few times early on:
temporary scaffolding that a later chapter deletes once something more
meaningful exists to show.

## Step 1.2 — Two more flags, still doing nothing

SQLiteForge's real CLI has five flags. We don't need all of them yet — the
`-m`/`--mode` flag doesn't mean anything until Chapter 3 has a renderer to
pass it to — but `-c`/`--command` is worth adding now, because it's about to
give us something to actually execute.

```diff
struct Cli {
    #[arg(value_name = "DATABASE")]
    database: Option<String>,

+    /// Execute SQL command and exit
+    #[arg(short = 'c', long = "command")]
+    command: Option<String>,
}
```

**Verified:** compiles; `cargo run -- -c "select 1"` parses `command` as
`Some("select 1")` (confirmed via the same debug print from Step 1.1 — not
shown again here since nothing about its behavior changed).

Notice `command` is *also* `Option<String>`, for the same reason as
`database`: most of the time you won't pass `-c` at all, because you want
the interactive shell, not a one-shot query. We'll reach for `Option` for
almost every CLI flag in this program — the presence or absence of a flag is
almost always meaningful on its own, before you even look at its value.

At this point we have a CLI parser and nothing to parse it *for*. Let's fix
that by giving `database` somewhere real to point.

Next: [Chapter 2 — Talking to SQLite](03-talking-to-sqlite.md)
