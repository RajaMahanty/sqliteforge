Previous: [A Real Interactive Shell](06-a-real-interactive-shell.md) | [Contents](00-index.md) | Next: [Dot Commands](08-dot-commands.md)

# Chapter 6: Multi-line Queries

Right now, pressing Enter always submits — there's no way to write a
`CREATE TABLE` across several lines. `reedline` has a `Validator` trait for
exactly this: given the buffer, return `Complete` (submit) or `Incomplete`
(insert a newline and keep editing instead).

## Step 6.1 — Complete means "ends with a semicolon"

**File:** `src/shell/validator.rs`
```diff
+use reedline::{ValidationResult, Validator};
+
+/// SQL input validator - determines if the input is complete.
+/// A SQL statement is complete when it ends with a semicolon
+/// (ignoring trailing whitespace), or is a dot command.
+/// Also handles internal host commands (__explorer_toggle__, etc.).
+pub struct SqlValidator;
+
+impl Validator for SqlValidator {
+    fn validate(&self, line: &str) -> ValidationResult {
+        let trimmed = line.trim();
+
+        // Empty input is valid (just produces nothing)
+        if trimmed.is_empty() {
+            return ValidationResult::Complete;
+        }
+
+        // Internal host commands are always complete
+        if trimmed.starts_with("__") && trimmed.ends_with("__") {
+            return ValidationResult::Complete;
+        }
+
+        // Dot commands are always complete
+        if trimmed.starts_with('.') {
+            return ValidationResult::Complete;
+        }
+
+        // SQL statements must end with semicolon
+        if trimmed.ends_with(';') {
+            ValidationResult::Complete
+        } else {
+            ValidationResult::Incomplete
+        }
+    }
+}
```

Notice the doc comment already mentions "internal host commands" and the
code already checks for a `__..__`-wrapped string, even though nothing in
the program produces one yet — that's a feature from Chapter 12 (auto-pairs)
that this function is quietly written to already support. We're building it
in now, matching the real file exactly, rather than adding it retroactively
when Chapter 12 needs it; it's harmless today because no real SQL statement
or dot command happens to start and end with double underscores.

**File:** `src/shell/mod.rs`
```diff
 pub mod prompt;
+pub mod validator;
 use reedline::{Reedline, Signal};
+use self::validator::SqlValidator;
 ...
-    let mut line_editor = Reedline::create();
+    let mut line_editor = Reedline::create().with_validator(Box::new(SqlValidator));
```

**Verified** in a real terminal: typing `SELECT 1` and pressing Enter
now does nothing visible except switch the prompt to `...>` (the
multiline indicator from `SqlPrompt`, wired up back in Chapter 5 but unused
until now) — the statement doesn't run. Continuing with `as x;` and
pressing Enter again submits the *whole* two-line buffer and prints the
result table.

This is also where we ran into the one real surprise of building this
tutorial: **automating a semicolon keystroke through `tmux send-keys`
sometimes silently drops it** when `;` is the very last character sent in a
literal batch — reproducible even piping into plain `cat`, with no
`reedline` involved. It's a quirk of the test-automation tool, not of the
code we wrote (a human pressing `;` on a real keyboard never triggers it).
We worked around it during testing by always sending a trailing space after
any semicolon, which the `.trim()` at the top of `validate` strips anyway.
It's the kind of thing that's worth mentioning exactly because a tutorial
that hid it would leave you thinking multi-line submission was flaky, when
the flakiness was entirely on our end, not the shell's.

`SqlValidator` deliberately treats a bare `.` command as instantly complete
— there's no `commands` module yet for it to hand off to, so right now a
line like `.tables` will fall through to `db.execute_query(".tables")` and
fail as invalid SQL. That's fine for one more chapter; it's about to become
real.

## Step 6.2 — A function that never gets called

One more piece belongs in this file, and it's worth building even though
(spoiler) nothing ever calls it:

**File:** `src/shell/validator.rs`
```diff
+/// Calculate the proper indentation for a new line based on the buffer content.
+/// Returns the number of spaces to indent.
+pub fn calculate_indent(buffer: &str) -> usize {
+    let indent_width: usize = 4;
+    let mut depth: i32 = 0;
+    let mut in_single_quote = false;
+    let mut in_double_quote = false;
+    let mut prev_char = '\0';
+
+    for ch in buffer.chars() {
+        match ch {
+            '\'' if !in_double_quote && prev_char != '\'' => { in_single_quote = !in_single_quote; }
+            '"' if !in_single_quote => { in_double_quote = !in_double_quote; }
+            '(' if !in_single_quote && !in_double_quote => { depth += 1; }
+            ')' if !in_single_quote && !in_double_quote => {
+                depth -= 1;
+                if depth < 0 { depth = 0; }
+            }
+            _ => {}
+        }
+        prev_char = ch;
+    }
+    (depth as usize) * indent_width
+}
```

**Verified:** compiles — and `cargo build` immediately flags it with
`warning: function 'calculate_indent' is never used`. This isn't a mistake
on our part; the real, shipped SQLiteForge has the exact same warning. The
function counts unmatched open parens in a buffer (skipping over anything
inside quotes) to work out how many levels deep a continuation line should
be indented — clearly written to back a real auto-indent feature. Chapter 12
adds a comment, still in the real source, that explains what happened to
it: *"auto_indent is handled separately via the `__auto_indent__` host
command which is no longer needed since reedline's Enter handles everything
natively."* In other words: this function was the first implementation of
auto-indent, a second implementation replaced it, and this one was never
deleted. We're keeping it, warning and all, because pretending it doesn't
exist would make Chapter 12 harder to follow, not easier, once you notice
`config.keybindings.auto_indent` still sitting in the config file with
nothing left reading it.

Next: [Chapter 7 — Dot Commands](08-dot-commands.md)
