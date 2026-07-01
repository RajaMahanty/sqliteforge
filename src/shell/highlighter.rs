use nu_ansi_term::{Color, Style};
use reedline::{Highlighter, StyledText};

/// SQL syntax highlighter for the interactive shell
pub struct SqlHighlighter;

impl SqlHighlighter {
    // SQL keywords to highlight — comprehensive SQLite keyword set
    const KEYWORDS: &'static [&'static str] = &[
        // Core DML
        "SELECT",
        "FROM",
        "WHERE",
        "INSERT",
        "INTO",
        "VALUES",
        "UPDATE",
        "SET",
        "DELETE",
        "REPLACE",
        "RETURNING",
        "EXPLAIN",
        "PRAGMA",
        // DDL
        "CREATE",
        "TABLE",
        "DROP",
        "ALTER",
        "INDEX",
        "VIEW",
        "TRIGGER",
        "VIRTUAL",
        "TEMP",
        "TEMPORARY",
        "IF",
        "COLUMN",
        "RENAME",
        "ADD",
        // Transactions
        "BEGIN",
        "COMMIT",
        "ROLLBACK",
        "TRANSACTION",
        "SAVEPOINT",
        "RELEASE",
        "DEFERRED",
        "IMMEDIATE",
        "EXCLUSIVE",
        // Joins
        "JOIN",
        "LEFT",
        "RIGHT",
        "INNER",
        "OUTER",
        "CROSS",
        "NATURAL",
        "FULL",
        "ON",
        "USING",
        // Logic / Operators
        "AND",
        "OR",
        "NOT",
        "IN",
        "LIKE",
        "GLOB",
        "REGEXP",
        "BETWEEN",
        "IS",
        "ISNULL",
        "NOTNULL",
        "EXISTS",
        "MATCH",
        "ESCAPE",
        // Sorting / Grouping
        "ORDER",
        "BY",
        "ASC",
        "DESC",
        "GROUP",
        "HAVING",
        "LIMIT",
        "OFFSET",
        "NULLS",
        "FIRST",
        "LAST",
        // Set operations
        "UNION",
        "INTERSECT",
        "EXCEPT",
        "ALL",
        "DISTINCT",
        // CASE expression
        "CASE",
        "WHEN",
        "THEN",
        "ELSE",
        "END",
        // Subqueries / CTEs
        "WITH",
        "RECURSIVE",
        "AS",
        // Constraints
        "PRIMARY",
        "KEY",
        "FOREIGN",
        "REFERENCES",
        "UNIQUE",
        "CHECK",
        "DEFAULT",
        "AUTOINCREMENT",
        "CONSTRAINT",
        "CONFLICT",
        "CASCADE",
        "RESTRICT",
        "NO",
        "ACTION",
        "ABORT",
        "FAIL",
        "IGNORE",
        "COLLATE",
        "DEFERRABLE",
        "INITIALLY",
        "NOT",
        "NULL",
        // Modifiers
        "ATTACH",
        "DETACH",
        "DATABASE",
        "INDEXED",
        "REINDEX",
        "VACUUM",
        "ANALYZE",
        // Window functions
        "OVER",
        "PARTITION",
        "ROWS",
        "ROW",
        "RANGE",
        "GROUPS",
        "WINDOW",
        "UNBOUNDED",
        "PRECEDING",
        "FOLLOWING",
        "CURRENT",
        "EXCLUDE",
        "TIES",
        "OTHERS",
        "FILTER",
        // Misc
        "DO",
        "NOTHING",
        "INSTEAD",
        "EACH",
        "BEFORE",
        "AFTER",
        "FOR",
        "OF",
        "PLAN",
        "QUERY",
        "RAISE",
        "GENERATED",
        "ALWAYS",
        "MATERIALIZED",
        "WITHOUT",
        "CURRENT_DATE",
        "CURRENT_TIME",
        "CURRENT_TIMESTAMP",
        // Types
        "INTEGER",
        "TEXT",
        "REAL",
        "BLOB",
        "NUMERIC",
        "BOOLEAN",
        // Aggregate functions
        "COUNT",
        "SUM",
        "AVG",
        "MIN",
        "MAX",
        "TOTAL",
        "GROUP_CONCAT",
        // Scalar functions
        "ABS",
        "COALESCE",
        "IFNULL",
        "IIF",
        "NULLIF",
        "LENGTH",
        "LOWER",
        "UPPER",
        "TRIM",
        "LTRIM",
        "RTRIM",
        "SUBSTR",
        "SUBSTRING",
        "TYPEOF",
        "UNICODE",
        "HEX",
        "INSTR",
        "PRINTF",
        "QUOTE",
        "RANDOM",
        "RANDOMBLOB",
        "REPLACE",
        "ROUND",
        "SOUNDEX",
        "ZEROBLOB",
        "CHANGES",
        "TOTAL_CHANGES",
        "LIKELY",
        "UNLIKELY",
        "LOAD_EXTENSION",
        "SQLITE_VERSION",
        "SQLITE_SOURCE_ID",
        "SQLITE_OFFSET",
        "SQLITE_COMPILEOPTION_GET",
        "SQLITE_COMPILEOPTION_USED",
        // Date/time functions
        "DATE",
        "TIME",
        "DATETIME",
        "STRFTIME",
        "JULIANDAY",
        "UNIXEPOCH",
        // CAST
        "CAST",
        "GLOB",
    ];

    fn is_keyword(word: &str) -> bool {
        let upper = word.to_uppercase();
        Self::KEYWORDS.contains(&upper.as_str())
    }
}

impl Highlighter for SqlHighlighter {
    fn highlight(&self, line: &str, _cursor: usize) -> StyledText {
        let mut styled = StyledText::new();

        // Dot command highlighting
        if line.trim_start().starts_with('.') {
            styled.push((Style::new().bold().fg(Color::Magenta), line.to_string()));
            return styled;
        }

        let mut chars = line.chars().peekable();
        let mut current_word = String::new();
        let mut in_string = false;
        let mut string_char = '\'';

        while let Some(ch) = chars.next() {
            if in_string {
                current_word.push(ch);
                if ch == string_char {
                    // Check for escaped quote
                    if chars.peek() == Some(&string_char) {
                        current_word.push(chars.next().unwrap());
                    } else {
                        styled.push((Style::new().fg(Color::Green), current_word.clone()));
                        current_word.clear();
                        in_string = false;
                    }
                }
            } else if ch == '\'' || ch == '"' {
                // Flush current word
                if !current_word.is_empty() {
                    flush_word(&mut styled, &current_word);
                    current_word.clear();
                }
                in_string = true;
                string_char = ch;
                current_word.push(ch);
            } else if ch == '-' && chars.peek() == Some(&'-') {
                // Line comment
                if !current_word.is_empty() {
                    flush_word(&mut styled, &current_word);
                    current_word.clear();
                }
                let mut comment = String::from("--");
                for c in chars.by_ref() {
                    comment.push(c);
                }
                styled.push((Style::new().fg(Color::DarkGray), comment));
            } else if ch.is_alphanumeric() || ch == '_' {
                current_word.push(ch);
            } else {
                if !current_word.is_empty() {
                    flush_word(&mut styled, &current_word);
                    current_word.clear();
                }
                // Operators and punctuation
                let style = match ch {
                    '(' | ')' => Style::new().fg(Color::Yellow),
                    ';' => Style::new().bold().fg(Color::Cyan),
                    ',' => Style::new().fg(Color::White),
                    '*' => Style::new().fg(Color::Cyan),
                    _ => Style::new().fg(Color::White),
                };
                styled.push((style, ch.to_string()));
            }
        }

        // Flush remaining
        if !current_word.is_empty() {
            if in_string {
                styled.push((Style::new().fg(Color::Green), current_word));
            } else {
                flush_word(&mut styled, &current_word);
            }
        }

        styled
    }
}

fn flush_word(styled: &mut StyledText, word: &str) {
    if SqlHighlighter::is_keyword(word) {
        styled.push((Style::new().bold().fg(Color::Cyan), word.to_uppercase()));
    } else if word.parse::<f64>().is_ok() {
        styled.push((Style::new().fg(Color::Magenta), word.to_string()));
    } else {
        styled.push((Style::new().fg(Color::White), word.to_string()));
    }
}
