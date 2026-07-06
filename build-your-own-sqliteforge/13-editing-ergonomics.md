Previous: [Autocomplete, Part 2](12-autocomplete-context-aware.md) | [Contents](00-index.md) | Next: [The Database Explorer](14-database-explorer.md)

# Chapter 12: Editing Ergonomics

None of what's left touches SQL semantics at all — it's a handful of
independent, optional editing conveniences (auto-closing brackets and
quotes, Shift-to-select, Ctrl+Arrow word jumps, a couple of fixed
navigation keys), each gated behind its own config flag, each added as a
few more `keybindings.add_binding(...)` calls. The interesting part isn't
the bindings themselves so much as *how* auto-close has to work — and, by
the end, a config section with more fields than the shell loop actually
reads.

## Step 12.1 — `KeybindingsConfig`

**File:** `src/config/mod.rs`
```diff
+/// Keybinding configuration
+#[derive(Debug, Clone, Serialize, Deserialize)]
+pub struct KeybindingsConfig {
+    /// Key to toggle the database explorer (default: "ctrl+e")
+    #[serde(default = "default_explorer_toggle")]
+    pub explorer_toggle: String,
+    /// Key to execute query / submit (default: "enter")
+    #[serde(default = "default_submit")]
+    pub submit: String,
+    /// Enable Shift+Arrow for text selection (default: true)
+    #[serde(default = "default_true")]
+    pub shift_select: bool,
+    /// Enable Ctrl+Arrow for word-jump navigation (default: true)
+    #[serde(default = "default_true")]
+    pub word_jump: bool,
+    /// Key to clear screen (default: "ctrl+l")
+    #[serde(default = "default_clear_screen")]
+    pub clear_screen: String,
+    /// Auto-close brackets and quotes: ( -> (), ' -> '' (default: true)
+    #[serde(default = "default_true")]
+    pub auto_pairs: bool,
+    /// Auto-indent continuation lines inside parentheses (default: true)
+    #[serde(default = "default_true")]
+    pub auto_indent: bool,
+}
+// ...default_explorer_toggle() -> "ctrl+e", default_submit() -> "enter",
+//    default_clear_screen() -> "ctrl+l", a Default impl, and a
+//    `keybindings: KeybindingsConfig` field added to `Config`
```

Four of these seven fields are read exactly once, by the code we're about
to write in this chapter (`auto_pairs`, `shift_select`, `word_jump`). Keep
the other three — `explorer_toggle`, `submit`, `clear_screen`, and
`auto_indent` — in mind as we go, because none of them end up doing what
their names promise, and we'll be honest about that as each one comes up.

## Step 12.2 — Auto-close brackets and quotes: insert a pair, then "type through" it

Typing `(` should insert `()` and place the cursor between them — that part
is one `ReedlineEvent::Edit` away. The harder part is what happens next:
if the user then types their own `)`, we don't want `(|)` to become `()|)`.
We want the typed `)` to just move the cursor past the one that's already
there. That decision — "is the character under the cursor already what I'm
about to type?" — can't be expressed as a static keybinding at all, because
it depends on runtime buffer state. It needs
`ReedlineEvent::ExecuteHostCommand`, which hands control back to *our* loop
for one keystroke. The real code does this for three bracket pairs and two
quote characters — we're building all five, since they're all the same
pattern repeated:

**File:** `src/shell/mod.rs`
```diff
+use reedline::{..., EditCommand, ...};
 ...
+    // Auto-close brackets and quotes
+    if config.keybindings.auto_pairs {
+        // Open brackets auto-close and move left
+        keybindings.add_binding(KeyModifiers::NONE, KeyCode::Char('('),
+            ReedlineEvent::Edit(vec![
+                EditCommand::InsertString("()".to_string()),
+                EditCommand::MoveLeft { select: false },
+            ]));
+        keybindings.add_binding(KeyModifiers::NONE, KeyCode::Char('['),
+            ReedlineEvent::Edit(vec![
+                EditCommand::InsertString("[]".to_string()),
+                EditCommand::MoveLeft { select: false },
+            ]));
+        keybindings.add_binding(KeyModifiers::NONE, KeyCode::Char('{'),
+            ReedlineEvent::Edit(vec![
+                EditCommand::InsertString("{}".to_string()),
+                EditCommand::MoveLeft { select: false },
+            ]));
+
+        // Closing characters skip or insert
+        keybindings.add_binding(KeyModifiers::NONE, KeyCode::Char(')'),
+            ReedlineEvent::ExecuteHostCommand("__char_close_paren__".to_string()));
+        keybindings.add_binding(KeyModifiers::NONE, KeyCode::Char(']'),
+            ReedlineEvent::ExecuteHostCommand("__char_close_bracket__".to_string()));
+        keybindings.add_binding(KeyModifiers::NONE, KeyCode::Char('}'),
+            ReedlineEvent::ExecuteHostCommand("__char_close_brace__".to_string()));
+
+        // Quotes skip or auto-close
+        keybindings.add_binding(KeyModifiers::NONE, KeyCode::Char('\''),
+            ReedlineEvent::ExecuteHostCommand("__char_quote__".to_string()));
+        keybindings.add_binding(KeyModifiers::NONE, KeyCode::Char('"'),
+            ReedlineEvent::ExecuteHostCommand("__char_double_quote__".to_string()));
+    }
```

**File:** `src/shell/mod.rs`
```diff
             Ok(Signal::Success(input)) => {
                 let trimmed = input.trim();
+
+                if trimmed == "__char_close_paren__" {
+                    handle_skip_or_insert_char(&mut line_editor, ')');
+                    continue;
+                }
+                if trimmed == "__char_close_bracket__" {
+                    handle_skip_or_insert_char(&mut line_editor, ']');
+                    continue;
+                }
+                if trimmed == "__char_close_brace__" {
+                    handle_skip_or_insert_char(&mut line_editor, '}');
+                    continue;
+                }
+                if trimmed == "__char_quote__" {
+                    handle_quote_char(&mut line_editor, '\'');
+                    continue;
+                }
+                if trimmed == "__char_double_quote__" {
+                    handle_quote_char(&mut line_editor, '"');
+                    continue;
+                }
+
                 if trimmed.is_empty() { continue; }
                 ...
+
+/// Skip over the closing character if it already matches the cursor's
+/// target, or insert it normally.
+fn handle_skip_or_insert_char(line_editor: &mut Reedline, c: char) {
+    let buffer = line_editor.current_buffer_contents().to_string();
+    let pos = line_editor.current_insertion_point();
+    let has_char = if pos < buffer.len() { buffer.as_bytes()[pos] == c as u8 } else { false };
+    if has_char {
+        line_editor.run_edit_commands(&[EditCommand::MoveRight { select: false }]);
+    } else {
+        line_editor.run_edit_commands(&[EditCommand::InsertChar(c)]);
+    }
+}
+
+/// Skip if matched, otherwise insert a pair and move left.
+fn handle_quote_char(line_editor: &mut Reedline, q: char) {
+    let buffer = line_editor.current_buffer_contents().to_string();
+    let pos = line_editor.current_insertion_point();
+    let has_char = if pos < buffer.len() { buffer.as_bytes()[pos] == q as u8 } else { false };
+    if has_char {
+        line_editor.run_edit_commands(&[EditCommand::MoveRight { select: false }]);
+    } else {
+        let pair = format!("{}{}", q, q);
+        line_editor.run_edit_commands(&[
+            EditCommand::InsertString(pair),
+            EditCommand::MoveLeft { select: false },
+        ]);
+    }
+}
```

`handle_quote_char` needs its own function rather than reusing
`handle_skip_or_insert_char`, even though the "skip if it matches" half is
identical: the "otherwise" half is different. A bracket's open and close
characters are different keys (you press `(`, and only `)` needs the
skip-logic); a quote's open and close characters are the *same* key, so
pressing `'` needs to insert `''` and step back between them, the same way
`(` inserts `()` — `handle_quote_char` does both jobs for one character
instead of splitting them across two keybindings.

There's a subtlety here that isn't optional: `ExecuteHostCommand` makes
reedline's `read_line` *return*, with the host command's name as if it were
a submitted line — it doesn't call some separate callback while editing
continues. That's why every `__char_...__` check has to sit at the very
top of the `Signal::Success` arm, ahead of even the empty-line check, and
why each one `continue`s immediately rather than falling into any of the
SQL-or-dot-command logic below it. Two more pieces of cleanup keep these
strings from leaking into places they shouldn't: `SqlValidator` (Chapter 6)
already treats any `__..__`-wrapped string as automatically `Complete`, and
`Reedline::create()` gets
`.with_history_exclusion_prefix(Some("__".to_string()))` so none of them
pollute the Up-arrow history.

**Verified** in a real terminal: typing `SELECT COUNT(` shows `SELECT
COUNT()` with the cursor between the parens; typing `*` then `)` produces
`SELECT COUNT(*)` — not `SELECT COUNT(*))` — because the second `)` was
recognized as already present and skipped rather than inserted.

## Step 12.3 — Selection and word-jump

These don't need any host-command trickery — pure, static keybindings,
because "select while moving" and "jump by word" are both things
`reedline`'s own `EditCommand` enum already knows how to do:

**File:** `src/shell/mod.rs`
```diff
+    // Shift+Arrow keybindings for text selection
+    if config.keybindings.shift_select {
+        keybindings.add_binding(KeyModifiers::SHIFT, KeyCode::Left,
+            ReedlineEvent::Edit(vec![EditCommand::MoveLeft { select: true }]));
+        keybindings.add_binding(KeyModifiers::SHIFT, KeyCode::Right,
+            ReedlineEvent::Edit(vec![EditCommand::MoveRight { select: true }]));
+        keybindings.add_binding(KeyModifiers::SHIFT, KeyCode::Up,
+            ReedlineEvent::Edit(vec![EditCommand::MoveToLineStart { select: true }]));
+        keybindings.add_binding(KeyModifiers::SHIFT, KeyCode::Down,
+            ReedlineEvent::Edit(vec![EditCommand::MoveToLineEnd { select: true }]));
+        keybindings.add_binding(KeyModifiers::SHIFT, KeyCode::Home,
+            ReedlineEvent::Edit(vec![EditCommand::MoveToLineStart { select: true }]));
+        keybindings.add_binding(KeyModifiers::SHIFT, KeyCode::End,
+            ReedlineEvent::Edit(vec![EditCommand::MoveToLineEnd { select: true }]));
+    }
+
+    // Ctrl+Arrow for word-level navigation
+    if config.keybindings.word_jump {
+        keybindings.add_binding(KeyModifiers::CONTROL, KeyCode::Left,
+            ReedlineEvent::Edit(vec![EditCommand::MoveWordLeft { select: false }]));
+        keybindings.add_binding(KeyModifiers::CONTROL, KeyCode::Right,
+            ReedlineEvent::Edit(vec![EditCommand::MoveWordRight { select: false }]));
+        keybindings.add_binding(KeyModifiers::CONTROL | KeyModifiers::SHIFT, KeyCode::Left,
+            ReedlineEvent::Edit(vec![EditCommand::MoveWordLeft { select: true }]));
+        keybindings.add_binding(KeyModifiers::CONTROL | KeyModifiers::SHIFT, KeyCode::Right,
+            ReedlineEvent::Edit(vec![EditCommand::MoveWordRight { select: true }]));
+    }
+
+    // Ctrl+A: move to beginning of line
+    keybindings.add_binding(KeyModifiers::CONTROL, KeyCode::Char('a'),
+        ReedlineEvent::Edit(vec![EditCommand::MoveToLineStart { select: false }]));
```

Reusing `MoveToLineStart`/`MoveToLineEnd` for Shift+Up/Down is a shortcut,
not a mistake: reedline doesn't have a "select to visual line above"
command the way a full text editor would, so Shift+Up here means "select to
the start of the current line," and Shift+Down means "select to its end" —
close enough to be genuinely useful in a mostly-single-line SQL prompt,
without pretending to be a real multi-line selection model.

**Verified:** compiles; each new binding is one more `EditCommand` variant
attached to one more key, no new state, no host commands.

## Step 12.4 — Three settings that don't do anything yet

Look back at `KeybindingsConfig` from Step 12.1. `shift_select`,
`word_jump`, and `auto_pairs` all gate real `if` statements above. The
other three don't:

- **`explorer_toggle`** (default `"ctrl+e"`) is never read. Ctrl+E's own
  binding, which we'll add in the next chapter, hardcodes
  `KeyCode::Char('e')` directly — changing this config value to, say,
  `"ctrl+x"` in your `config.toml` has no effect at all.
- **`submit`** (default `"enter"`) is never read anywhere. Enter is wired
  natively by reedline's own validator-driven submission (Chapter 6); there
  was never a separate keybinding for it to reconfigure.
- **`clear_screen`** (default `"ctrl+l"`) is never read. Ctrl+L's
  screen-clearing behavior comes from `reedline`'s own default keybindings,
  which we never removed or overrode.
- **`auto_indent`** is the most interesting of the four, because the real
  source code explains it in a comment rather than leaving you to
  infer it:

  ```rust
  // Smart Enter: use reedline's native Enter which handles menu selection,
  // validator-based submission, and multiline newline insertion.
  // Note: auto_indent is handled separately via the __auto_indent__ host command
  // which is no longer needed since reedline's Enter handles everything natively.
  ```

  This is `calculate_indent` from Chapter 6, finally explained: there used
  to be an `__auto_indent__` host command, following the exact same pattern
  as `__char_close_paren__` above, that called `calculate_indent` on Enter
  to indent continuation lines. At some point, reedline's own Enter
  handling grew good enough to make that host command redundant, and it was
  deleted — but the config flag controlling it, and the function it used to
  call, were not. `config.keybindings.auto_indent` is real, in the config
  file, in the struct, and inert.

We're building all seven fields, because they're all really in the config
file a user sees — but four out of seven are worth remembering next time
`.show` or a config file makes a setting look more powerful than it is.

None of this changes what you can *see* about the database itself — that's
next, with a panel that finally shows you the whole schema at a glance.

Next: [Chapter 13 — The Database Explorer](14-database-explorer.md)
