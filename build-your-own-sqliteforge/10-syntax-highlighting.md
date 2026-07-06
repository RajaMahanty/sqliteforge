Previous: [Persistent History](09-persistent-history.md) | [Contents](00-index.md) | Next: [Autocomplete, Part 1](11-autocomplete-basics.md)

# Chapter 9: Syntax Highlighting

Everything typed so far has been plain white text. `reedline`'s
`Highlighter` trait gets called on every keystroke with the current buffer
and cursor position, and returns a `StyledText` — a list of `(Style, &str)`
fragments to draw instead of the raw string. We're building a small,
character-by-character state machine, not a real SQL tokenizer, and that
distinction matters for what it gets right and wrong.

## Step 9.1 — The state machine

**File:** `src/shell/highlighter.rs`
```diff
+use nu_ansi_term::{Color, Style};
+use reedline::{Highlighter, StyledText};
+
+/// SQL syntax highlighter for the interactive shell
+pub struct SqlHighlighter;
+
+impl SqlHighlighter {
+    const KEYWORDS: &'static [&'static str] = &[
+        "SELECT", "FROM", "WHERE", "INSERT", "INTO", "VALUES", "UPDATE", "SET",
+        "DELETE", "CREATE", "TABLE", "DROP", "ALTER", "INDEX", "VIEW", "JOIN",
+        "AND", "OR", "NOT", "NULL", "PRIMARY", "KEY", "FOREIGN", "REFERENCES",
+        /* ...about 170 more, covering DDL, joins, window functions, and
+           SQLite's built-in scalar/aggregate/date functions... */
+    ];
+
+    fn is_keyword(word: &str) -> bool {
+        Self::KEYWORDS.contains(&word.to_uppercase().as_str())
+    }
+}
+
+impl Highlighter for SqlHighlighter {
+    fn highlight(&self, line: &str, _cursor: usize) -> StyledText {
+        let mut styled = StyledText::new();
+
+        if line.trim_start().starts_with('.') {
+            styled.push((Style::new().bold().fg(Color::Magenta), line.to_string()));
+            return styled;
+        }
+
+        let mut chars = line.chars().peekable();
+        let mut current_word = String::new();
+        let mut in_string = false;
+        let mut string_char = '\'';
+
+        while let Some(ch) = chars.next() {
+            if in_string {
+                current_word.push(ch);
+                if ch == string_char {
+                    if chars.peek() == Some(&string_char) {
+                        current_word.push(chars.next().unwrap()); // escaped '' quote
+                    } else {
+                        styled.push((Style::new().fg(Color::Green), current_word.clone()));
+                        current_word.clear();
+                        in_string = false;
+                    }
+                }
+            } else if ch == '\'' || ch == '"' {
+                if !current_word.is_empty() { flush_word(&mut styled, &current_word); current_word.clear(); }
+                in_string = true; string_char = ch; current_word.push(ch);
+            } else if ch == '-' && chars.peek() == Some(&'-') {
+                if !current_word.is_empty() { flush_word(&mut styled, &current_word); current_word.clear(); }
+                let mut comment = String::from("--");
+                for c in chars.by_ref() { comment.push(c); }
+                styled.push((Style::new().fg(Color::DarkGray), comment));
+            } else if ch.is_alphanumeric() || ch == '_' {
+                current_word.push(ch);
+            } else {
+                if !current_word.is_empty() { flush_word(&mut styled, &current_word); current_word.clear(); }
+                let style = match ch {
+                    '(' | ')' => Style::new().fg(Color::Yellow),
+                    ';' => Style::new().bold().fg(Color::Cyan),
+                    '*' => Style::new().fg(Color::Cyan),
+                    _ => Style::new().fg(Color::White),
+                };
+                styled.push((style, ch.to_string()));
+            }
+        }
+        if !current_word.is_empty() {
+            if in_string { styled.push((Style::new().fg(Color::Green), current_word)); }
+            else { flush_word(&mut styled, &current_word); }
+        }
+        styled
+    }
+}
+
+fn flush_word(styled: &mut StyledText, word: &str) {
+    if SqlHighlighter::is_keyword(word) {
+        styled.push((Style::new().bold().fg(Color::Cyan), word.to_uppercase()));
+    } else if word.parse::<f64>().is_ok() {
+        styled.push((Style::new().fg(Color::Magenta), word.to_string()));
+    } else {
+        styled.push((Style::new().fg(Color::White), word.to_string()));
+    }
+}
```

**File:** `src/shell/mod.rs`
```diff
+pub mod highlighter;
+use self::highlighter::SqlHighlighter;
 ...
     let mut line_editor = Reedline::create()
         .with_history(reedline_history)
+        .with_highlighter(Box::new(SqlHighlighter))
         .with_validator(Box::new(SqlValidator));
```

**Verified** in a real terminal: typing `select 1` displays keywords
uppercased and bold-cyan as you type — `flush_word` calls
`word.to_uppercase()` on every recognized keyword, so what you *see* on
screen differs from what's actually in the editing buffer. That's purely
cosmetic: the submitted string still contains whatever case you actually
typed, since `StyledText` only controls rendering, not the buffer `Reedline`
hands back on `Signal::Success`.

Two implementation choices worth noting:

**It's a character scanner, not a parser.** `is_keyword` checks any bare
word against a flat list of ~180 strings — it has no idea whether `NAME` is
a column called `name` or the actual keyword-adjacent word `NAME`used in a
`PRAGMA`. That's an acceptable trade for a highlighter: getting a false
positive on a column that happens to share a name with a keyword just means
one word is colored slightly wrong, which is a much smaller failure mode
than, say, a parse error crashing the editor mid-keystroke.

**Escaped quotes get consumed two characters at a time.** The check for
`chars.peek() == Some(&string_char)` inside `in_string` is what lets
`'it''s'` highlight as one green string instead of ending after `it`. This
is the one place the scanner needs a full token of lookahead rather than
reacting to a single character — SQL's own escaping convention (doubling the
quote character) forces it.

The completer, next, is going to need to look at *far* more context than a
character at a time — not just "is this character a keyword," but "given
everywhere the cursor has been on this line, what kind of token belongs
here."

Next: [Chapter 10 — Autocomplete, Part 1](11-autocomplete-basics.md)
