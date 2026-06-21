use reedline::{Completer, Span, Suggestion};
use std::collections::HashMap;

use crate::config::CompletionConfig;

/// SQL and dot-command autocompletion engine
pub struct SqlCompleter {
    /// Dot commands
    dot_commands: Vec<String>,
    /// Table names (refreshed from database)
    tables: Vec<String>,
    /// View names
    views: Vec<String>,
    /// All column names (flat, for unqualified completion)
    all_columns: Vec<String>,
    /// Columns per table (for qualified "table"."col" completion)
    table_columns: HashMap<String, Vec<String>>,
    /// Index names
    indices: Vec<String>,
    /// Completion configuration
    config: CompletionConfig,
}

// ── Keyword categories ──────────────────────────────────────────────────────

/// Keywords valid in expressions (after SELECT, WHERE, ON, HAVING, etc.)
const EXPR_KEYWORDS: &[&str] = &[
    "ALL", "AND", "AS", "ASC", "BETWEEN", "CASE", "CAST", "CURRENT_DATE",
    "CURRENT_TIME", "CURRENT_TIMESTAMP", "DESC", "DISTINCT", "ELSE", "END",
    "ESCAPE", "EXCEPT", "EXISTS", "FILTER", "GLOB", "IN", "IS", "ISNULL",
    "LIKE", "LIMIT", "MATCH", "NOT", "NOTNULL", "NULL", "NULLS", "OFFSET",
    "OR", "ORDER", "OVER", "REGEXP", "THEN", "UNION", "WHEN", "WINDOW",
    // Functions
    "COUNT", "SUM", "AVG", "MIN", "MAX", "TOTAL", "GROUP_CONCAT",
    "ABS", "CHANGES", "COALESCE", "IFNULL", "IIF", "INSTR", "HEX", "LENGTH",
    "LIKELY", "LOAD_EXTENSION", "LOWER", "LTRIM", "NULLIF", "PRINTF", "QUOTE",
    "RANDOM", "RANDOMBLOB", "REPLACE", "ROUND", "RTRIM", "SOUNDEX",
    "SQLITE_VERSION", "SUBSTR", "SUBSTRING", "TOTAL_CHANGES", "TRIM",
    "TYPEOF", "UNICODE", "UNLIKELY", "UPPER", "ZEROBLOB",
    "DATE", "TIME", "DATETIME", "JULIANDAY", "STRFTIME", "UNIXEPOCH",
    // Types (for CAST)
    "INTEGER", "REAL", "TEXT", "BLOB", "NUMERIC", "BOOLEAN",
];

/// Keywords valid after a table/value (clause-level transitions)
const CLAUSE_KEYWORDS: &[&str] = &[
    "AND", "AS", "ASC", "BY", "CROSS", "DESC", "EXCEPT", "FROM", "FULL",
    "GROUP", "HAVING", "INNER", "INTERSECT", "JOIN", "LEFT", "LIMIT",
    "NATURAL", "OFFSET", "ON", "OR", "ORDER", "OUTER", "RETURNING", "RIGHT",
    "UNION", "USING", "WHERE",
];

/// Keywords to start a new statement (when nothing or general context)
const STMT_KEYWORDS: &[&str] = &[
    "ALTER", "ANALYZE", "ATTACH", "BEGIN", "COMMIT", "CREATE", "DELETE",
    "DETACH", "DROP", "EXPLAIN", "INSERT", "PRAGMA", "REINDEX", "RELEASE",
    "REPLACE", "ROLLBACK", "SAVEPOINT", "SELECT", "UPDATE", "VACUUM", "WITH",
];

/// DDL-specific keywords (after CREATE/ALTER/DROP)
const DDL_KEYWORDS: &[&str] = &[
    "TABLE", "INDEX", "VIEW", "TRIGGER", "VIRTUAL", "TEMP", "TEMPORARY",
    "IF", "EXISTS", "NOT", "PRIMARY", "KEY", "FOREIGN", "REFERENCES",
    "UNIQUE", "CHECK", "DEFAULT", "AUTOINCREMENT", "CONSTRAINT", "CONFLICT",
    "CASCADE", "RESTRICT", "NO", "ACTION", "ABORT", "FAIL", "IGNORE",
    "COLLATE", "DEFERRABLE", "INITIALLY", "DEFERRED", "IMMEDIATE",
    "COLUMN", "RENAME", "ADD", "GENERATED", "ALWAYS", "NULL",
    "INTEGER", "REAL", "TEXT", "BLOB", "NUMERIC", "BOOLEAN",
    "EXCLUSIVE", "WITHOUT",
];

/// Column constraint keywords (inside CREATE TABLE parentheses)
const COLUMN_DEF_KEYWORDS: &[&str] = &[
    "PRIMARY", "KEY", "NOT", "NULL", "UNIQUE", "CHECK", "DEFAULT",
    "AUTOINCREMENT", "REFERENCES", "COLLATE", "CONSTRAINT",
    "FOREIGN", "ON", "DELETE", "UPDATE", "CASCADE", "RESTRICT",
    "SET", "NO", "ACTION", "ABORT", "FAIL", "IGNORE", "REPLACE",
    "DEFERRABLE", "INITIALLY", "DEFERRED", "IMMEDIATE",
    "GENERATED", "ALWAYS", "AS", "STORED", "VIRTUAL",
    // Data types
    "INTEGER", "TEXT", "REAL", "BLOB", "NUMERIC", "BOOLEAN",
    "VARCHAR", "CHAR", "INT", "BIGINT", "SMALLINT", "TINYINT",
    "FLOAT", "DOUBLE", "DECIMAL", "DATE", "DATETIME", "TIMESTAMP",
];

/// INSERT-specific keywords
const INSERT_KEYWORDS: &[&str] = &[
    "INTO", "VALUES", "DEFAULT", "SELECT", "OR", "REPLACE", "ABORT",
    "FAIL", "IGNORE", "ROLLBACK", "ON", "CONFLICT", "DO", "NOTHING",
    "UPDATE", "SET", "WHERE", "RETURNING",
];

/// Which keyword set to use
#[derive(Clone, Copy)]
enum KeywordScope {
    /// After SELECT, WHERE, ON, HAVING — expression keywords
    Expression,
    /// After a table/identifier — clause transition keywords
    Clause,
    /// Start of statement or general — statement + clause + expression
    General,
    /// After CREATE/ALTER/DROP — DDL keywords
    Ddl,
    /// Inside column definitions (CREATE TABLE body)
    ColumnDef,
    /// After INSERT
    Insert,
    /// No keywords (e.g. directly after FROM/JOIN)
    None,
}

/// What types of completions are allowed in this context
struct CompletionContext {
    keyword_scope: KeywordScope,
    allow_tables: bool,
    allow_columns: bool,
    allow_indices: bool,
    /// Allow suggestions even when the current word is empty
    eager: bool,
}

impl SqlCompleter {
    pub fn new() -> Self {
        Self {
            dot_commands: Self::dot_command_list(),
            tables: Vec::new(),
            views: Vec::new(),
            all_columns: Vec::new(),
            table_columns: HashMap::new(),
            indices: Vec::new(),
            config: CompletionConfig::default(),
        }
    }

    pub fn with_config(config: &CompletionConfig) -> Self {
        Self {
            dot_commands: Self::dot_command_list(),
            tables: Vec::new(),
            views: Vec::new(),
            all_columns: Vec::new(),
            table_columns: HashMap::new(),
            indices: Vec::new(),
            config: config.clone(),
        }
    }

    /// Update schema information from the database
    pub fn update_schema(
        &mut self,
        tables: Vec<String>,
        views: Vec<String>,
        all_columns: Vec<String>,
        table_columns: HashMap<String, Vec<String>>,
        indices: Vec<String>,
    ) {
        self.tables = tables;
        self.views = views;
        self.all_columns = all_columns;
        self.table_columns = table_columns;
        self.indices = indices;
    }

    fn dot_command_list() -> Vec<String> {
        vec![
            ".help", ".quit", ".exit", ".tables", ".schema", ".indices", ".mode", ".headers",
            ".read", ".output", ".dump", ".nullvalue", ".show", ".preview",
        ]
        .into_iter()
        .map(String::from)
        .collect()
    }

    /// Get the keyword list for the given scope
    fn keywords_for_scope(scope: KeywordScope) -> Vec<&'static str> {
        match scope {
            KeywordScope::Expression => EXPR_KEYWORDS.to_vec(),
            KeywordScope::Clause => CLAUSE_KEYWORDS.to_vec(),
            KeywordScope::Ddl => DDL_KEYWORDS.to_vec(),
            KeywordScope::ColumnDef => COLUMN_DEF_KEYWORDS.to_vec(),
            KeywordScope::Insert => INSERT_KEYWORDS.to_vec(),
            KeywordScope::General => {
                let mut all = Vec::new();
                all.extend_from_slice(STMT_KEYWORDS);
                all.extend_from_slice(CLAUSE_KEYWORDS);
                all.extend_from_slice(EXPR_KEYWORDS);
                // Deduplicate
                all.sort_unstable();
                all.dedup();
                all
            }
            KeywordScope::None => Vec::new(),
        }
    }

    /// Try to detect a "table"."col" pattern and return table-qualified column suggestions.
    fn try_qualified_completion(&self, line_to_cursor: &str, pos: usize) -> Option<Vec<Suggestion>> {
        let before = line_to_cursor;
        let dot_pos = before.rfind('.')?;
        let after_dot = &before[dot_pos + 1..];

        let (col_prefix, col_has_quote) = if after_dot.starts_with('"') {
            (&after_dot[1..], true)
        } else {
            (after_dot, false)
        };

        let before_dot = &before[..dot_pos];
        if !before_dot.ends_with('"') {
            let alias_start = before_dot
                .rfind(|c: char| c.is_whitespace() || c == '(' || c == ',')
                .map(|i| i + 1)
                .unwrap_or(0);
            let alias = &before_dot[alias_start..];
            if alias.is_empty() {
                return None;
            }

            let span_start = dot_pos + 1;
            let alias_lower = alias.to_lowercase();
            for (table_name, cols) in &self.table_columns {
                if table_name.to_lowercase() == alias_lower {
                    return Some(self.build_column_suggestions(
                        cols, col_prefix, span_start, pos, col_has_quote,
                    ));
                }
            }
            return Some(self.build_column_suggestions(
                &self.all_columns, col_prefix, span_start, pos, col_has_quote,
            ));
        }

        let table_end = before_dot.len() - 1;
        let table_start = before_dot[..table_end].rfind('"')?;
        let table_name = &before_dot[table_start + 1..table_end];
        let span_start = dot_pos + 1;
        let cols = self.table_columns.get(table_name)?;

        Some(self.build_column_suggestions(cols, col_prefix, span_start, pos, col_has_quote))
    }

    fn build_column_suggestions(
        &self,
        columns: &[String],
        col_prefix: &str,
        span_start: usize,
        pos: usize,
        _has_leading_quote: bool,
    ) -> Vec<Suggestion> {
        let lower_prefix = col_prefix.to_lowercase();
        let quote = self.config.quote_identifiers;
        columns
            .iter()
            .filter(|col| col_prefix.is_empty() || col.to_lowercase().starts_with(&lower_prefix))
            .map(|col| Suggestion {
                value: if quote {
                    format!("\"{}\"", col)
                } else {
                    col.clone()
                },
                description: Some("column".to_string()),
                style: None,
                extra: None,
                span: Span::new(span_start, pos),
                append_whitespace: true,
            })
            .collect()
    }

    /// Check if we are inside CREATE TABLE parentheses
    fn is_inside_create_table_body(line_to_cursor: &str) -> bool {
        let upper = line_to_cursor.to_uppercase();
        // Find the last CREATE TABLE, then check if we have an unmatched open paren
        if let Some(ct_pos) = upper.rfind("CREATE TABLE") {
            let after_ct = &line_to_cursor[ct_pos..];
            let open_parens = after_ct.matches('(').count();
            let close_parens = after_ct.matches(')').count();
            return open_parens > close_parens;
        }
        // Also handle CREATE TEMP TABLE, CREATE TEMPORARY TABLE
        if let Some(ct_pos) = upper.rfind("CREATE TEMP TABLE")
            .or_else(|| upper.rfind("CREATE TEMPORARY TABLE"))
        {
            let after_ct = &line_to_cursor[ct_pos..];
            let open_parens = after_ct.matches('(').count();
            let close_parens = after_ct.matches(')').count();
            return open_parens > close_parens;
        }
        false
    }

    /// Detect the SQL context by looking at the keyword preceding the cursor
    fn detect_context(line_to_cursor: &str, word_start: usize) -> CompletionContext {
        // Inside CREATE TABLE body: suggest column definition keywords / data types
        if Self::is_inside_create_table_body(line_to_cursor) {
            return CompletionContext {
                keyword_scope: KeywordScope::ColumnDef,
                allow_tables: false,
                allow_columns: false,
                allow_indices: false,
                eager: false,
            };
        }

        let before = line_to_cursor[..word_start].trim_end();
        let before = before.strip_suffix(',').unwrap_or(before).trim_end();
        let prev_keyword = Self::find_prev_keyword(before);

        match prev_keyword.as_deref() {
            // After FROM/JOIN variants: only tables and views
            Some("FROM") | Some("JOIN") => {
                CompletionContext {
                    keyword_scope: KeywordScope::None,
                    allow_tables: true,
                    allow_columns: false,
                    allow_indices: false,
                    eager: true,
                }
            }
            // After INTO: tables (for INSERT INTO)
            Some("INTO") => CompletionContext {
                keyword_scope: KeywordScope::None,
                allow_tables: true,
                allow_columns: false,
                allow_indices: false,
                eager: true,
            },
            // After TABLE: tables (for DROP TABLE, ALTER TABLE) + DDL keywords
            Some("TABLE") => CompletionContext {
                keyword_scope: KeywordScope::None,
                allow_tables: true,
                allow_columns: false,
                allow_indices: false,
                eager: true,
            },
            // After UPDATE: tables
            Some("UPDATE") => CompletionContext {
                keyword_scope: KeywordScope::None,
                allow_tables: true,
                allow_columns: false,
                allow_indices: false,
                eager: true,
            },
            // After DELETE: suggest FROM
            Some("DELETE") => CompletionContext {
                keyword_scope: KeywordScope::Clause,
                allow_tables: false,
                allow_columns: false,
                allow_indices: false,
                eager: true,
            },
            // After INSERT: suggest INTO, OR
            Some("INSERT") => CompletionContext {
                keyword_scope: KeywordScope::Insert,
                allow_tables: false,
                allow_columns: false,
                allow_indices: false,
                eager: true,
            },
            // After SELECT: expression keywords + columns + tables
            Some("SELECT") | Some("HAVING") => CompletionContext {
                keyword_scope: KeywordScope::Expression,
                allow_tables: true,
                allow_columns: true,
                allow_indices: false,
                eager: true,
            },
            // After WHERE/ON/AND/OR/operators: expression keywords + columns
            Some("WHERE") | Some("ON") | Some("SET") | Some("AND") | Some("OR")
            | Some("BETWEEN") | Some("CASE") | Some("WHEN") | Some("THEN")
            | Some("ELSE") | Some("LIKE") | Some("IN") | Some("VALUES") => CompletionContext {
                keyword_scope: KeywordScope::Expression,
                allow_tables: true,
                allow_columns: true,
                allow_indices: false,
                eager: true,
            },
            // After CREATE/ALTER/DROP: DDL keywords
            Some("CREATE") | Some("ALTER") | Some("DROP") => CompletionContext {
                keyword_scope: KeywordScope::Ddl,
                allow_tables: false,
                allow_columns: false,
                allow_indices: false,
                eager: true,
            },
            // After INDEX: indices or tables (for CREATE INDEX ... ON <table>)
            Some("INDEX") => CompletionContext {
                keyword_scope: KeywordScope::None,
                allow_tables: false,
                allow_columns: false,
                allow_indices: true,
                eager: true,
            },
            // After ORDER/GROUP: suggest BY, then columns
            Some("ORDER") | Some("GROUP") => CompletionContext {
                keyword_scope: KeywordScope::Expression,
                allow_tables: false,
                allow_columns: true,
                allow_indices: false,
                eager: true,
            },
            // After BY: columns (ORDER BY x, GROUP BY x)
            Some("BY") => CompletionContext {
                keyword_scope: KeywordScope::Expression,
                allow_tables: true,
                allow_columns: true,
                allow_indices: false,
                eager: true,
            },
            // After LIMIT/OFFSET: expression keywords + columns
            Some("LIMIT") | Some("OFFSET") => CompletionContext {
                keyword_scope: KeywordScope::Expression,
                allow_tables: false,
                allow_columns: true,
                allow_indices: false,
                eager: true,
            },
            // After join modifiers: suggest JOIN + tables
            Some("INNER") | Some("LEFT") | Some("RIGHT") | Some("CROSS")
            | Some("NATURAL") | Some("OUTER") | Some("FULL") => CompletionContext {
                keyword_scope: KeywordScope::None,
                allow_tables: true,
                allow_columns: false,
                allow_indices: false,
                eager: true,
            },
            // After AS: no suggestions (user defines alias)
            Some("AS") => CompletionContext {
                keyword_scope: KeywordScope::None,
                allow_tables: false,
                allow_columns: false,
                allow_indices: false,
                eager: false,
            },
            // After PRAGMA: no tables/columns, just let user type
            Some("PRAGMA") => CompletionContext {
                keyword_scope: KeywordScope::None,
                allow_tables: false,
                allow_columns: false,
                allow_indices: false,
                eager: false,
            },
            // After IF: suggest EXISTS/NOT (for CREATE TABLE IF NOT EXISTS)
            Some("IF") => CompletionContext {
                keyword_scope: KeywordScope::Ddl,
                allow_tables: false,
                allow_columns: false,
                allow_indices: false,
                eager: true,
            },
            // After EXISTS/NOT: suggest table names (CREATE TABLE IF NOT EXISTS <name>)
            Some("EXISTS") => CompletionContext {
                keyword_scope: KeywordScope::None,
                allow_tables: true,
                allow_columns: false,
                allow_indices: false,
                eager: true,
            },
            // After ADD: suggest COLUMN (ALTER TABLE x ADD COLUMN)
            Some("ADD") => CompletionContext {
                keyword_scope: KeywordScope::ColumnDef,
                allow_tables: false,
                allow_columns: false,
                allow_indices: false,
                eager: true,
            },
            // After RENAME: suggest TO, COLUMN
            Some("RENAME") => CompletionContext {
                keyword_scope: KeywordScope::Ddl,
                allow_tables: false,
                allow_columns: true,
                allow_indices: false,
                eager: true,
            },
            // No previous keyword: at start of statement
            None => {
                // Check if the line is truly empty or at start
                let all_before = line_to_cursor[..word_start].trim();
                if all_before.is_empty() {
                    // Beginning of a new statement
                    CompletionContext {
                        keyword_scope: KeywordScope::General,
                        allow_tables: false,
                        allow_columns: false,
                        allow_indices: false,
                        eager: false,
                    }
                } else {
                    // After a non-keyword token (table name, column, etc.) — clause transitions
                    CompletionContext {
                        keyword_scope: KeywordScope::Clause,
                        allow_tables: true,
                        allow_columns: true,
                        allow_indices: false,
                        eager: false,
                    }
                }
            }
            // Default: general (statement starters + clause + expression)
            _ => CompletionContext {
                keyword_scope: KeywordScope::Clause,
                allow_tables: true,
                allow_columns: true,
                allow_indices: false,
                eager: false,
            },
        }
    }

    /// Check the immediately preceding token. If it's a SQL keyword, return it.
    /// If it's a quoted identifier or non-keyword, return None (general context).
    fn find_prev_keyword(text: &str) -> Option<String> {
        let text = text.trim_end();
        if text.is_empty() {
            return None;
        }

        let bytes = text.as_bytes();
        let mut i = bytes.len();

        while i > 0 && bytes[i - 1].is_ascii_whitespace() {
            i -= 1;
        }
        if i == 0 {
            return None;
        }

        let last = bytes[i - 1];

        // Quoted identifier → general context
        if last == b'"' {
            return None;
        }
        // Closing paren → general context
        if last == b')' {
            return None;
        }

        // Extract the unquoted token
        let end = i;
        while i > 0 && !bytes[i - 1].is_ascii_whitespace()
            && bytes[i - 1] != b'(' && bytes[i - 1] != b','
            && bytes[i - 1] != b')'
        {
            i -= 1;
        }
        let token = &text[i..end];
        let upper = token.to_uppercase();

        let context_keywords = [
            "SELECT", "FROM", "WHERE", "JOIN", "INNER", "LEFT", "RIGHT", "CROSS",
            "NATURAL", "OUTER", "FULL", "ON", "INTO", "TABLE", "UPDATE", "SET",
            "HAVING", "BY", "INDEX", "AND", "OR", "BETWEEN", "CASE", "WHEN",
            "THEN", "ELSE", "LIKE", "ORDER", "GROUP", "DELETE", "INSERT",
            "VALUES", "CREATE", "ALTER", "DROP", "IN", "AS", "PRAGMA",
            "LIMIT", "OFFSET", "IF", "EXISTS", "NOT", "ADD", "RENAME",
        ];

        if context_keywords.contains(&upper.as_str()) {
            return Some(upper);
        }

        None
    }
}

impl Completer for SqlCompleter {
    fn complete(&mut self, line: &str, pos: usize) -> Vec<Suggestion> {
        if !self.config.enabled {
            return Vec::new();
        }

        let line_to_cursor = &line[..pos];

        // Dot command completion
        if line_to_cursor.trim_start().starts_with('.') {
            let input = line_to_cursor.trim_start();
            let start = pos - input.len();
            return self
                .dot_commands
                .iter()
                .filter(|cmd| cmd.starts_with(input))
                .map(|cmd| Suggestion {
                    value: cmd.clone(),
                    description: None,
                    style: None,
                    extra: None,
                    span: Span::new(start, pos),
                    append_whitespace: true,
                })
                .collect();
        }

        // Try table-qualified column completion first: "table"."col" or alias.col
        if let Some(suggestions) = self.try_qualified_completion(line_to_cursor, pos) {
            if !suggestions.is_empty() {
                return self.cap_suggestions(suggestions);
            }
        }

        // Find the current word being typed
        let word_start = line_to_cursor
            .rfind(|c: char| c.is_whitespace() || c == '(' || c == ',')
            .map(|i| i + 1)
            .unwrap_or(0);

        let raw_word = &line_to_cursor[word_start..];

        // Check if the word starts with a double quote
        let (has_leading_quote, current_word, span_start) = if raw_word.starts_with('"') {
            (true, &raw_word[1..], word_start)
        } else {
            (false, raw_word, word_start)
        };

        // Determine SQL context
        let context = Self::detect_context(line_to_cursor, word_start);

        // If the word is empty and context is not eager, don't show suggestions
        // (unless we're in an eager context like after FROM, SELECT, etc.)
        if current_word.is_empty() && !has_leading_quote && !context.eager {
            return Vec::new();
        }

        let upper_word = current_word.to_uppercase();
        let lower_word = current_word.to_lowercase();
        let mut suggestions = Vec::new();
        let quote = self.config.quote_identifiers;

        // Keywords (filtered by context scope)
        if !has_leading_quote && self.config.suggest_keywords {
            let keywords = Self::keywords_for_scope(context.keyword_scope);
            for kw in keywords {
                if current_word.is_empty() || kw.starts_with(&upper_word) {
                    suggestions.push(Suggestion {
                        value: kw.to_string(),
                        description: Some("keyword".to_string()),
                        style: None,
                        extra: None,
                        span: Span::new(span_start, pos),
                        append_whitespace: true,
                    });
                }
            }
        }

        // Table names
        if context.allow_tables {
            for table in &self.tables {
                if current_word.is_empty() || table.to_lowercase().starts_with(&lower_word) {
                    suggestions.push(Suggestion {
                        value: if quote {
                            format!("\"{}\"", table)
                        } else {
                            table.clone()
                        },
                        description: Some("table".to_string()),
                        style: None,
                        extra: None,
                        span: Span::new(span_start, pos),
                        append_whitespace: true,
                    });
                }
            }

            for view in &self.views {
                if current_word.is_empty() || view.to_lowercase().starts_with(&lower_word) {
                    suggestions.push(Suggestion {
                        value: if quote {
                            format!("\"{}\"", view)
                        } else {
                            view.clone()
                        },
                        description: Some("view".to_string()),
                        style: None,
                        extra: None,
                        span: Span::new(span_start, pos),
                        append_whitespace: true,
                    });
                }
            }
        }

        // Column names
        if context.allow_columns && self.config.suggest_columns {
            for col in &self.all_columns {
                if current_word.is_empty() || col.to_lowercase().starts_with(&lower_word) {
                    suggestions.push(Suggestion {
                        value: if quote {
                            format!("\"{}\"", col)
                        } else {
                            col.clone()
                        },
                        description: Some("column".to_string()),
                        style: None,
                        extra: None,
                        span: Span::new(span_start, pos),
                        append_whitespace: true,
                    });
                }
            }
        }

        // Index names
        if context.allow_indices {
            for idx in &self.indices {
                if current_word.is_empty() || idx.to_lowercase().starts_with(&lower_word) {
                    suggestions.push(Suggestion {
                        value: if quote {
                            format!("\"{}\"", idx)
                        } else {
                            idx.clone()
                        },
                        description: Some("index".to_string()),
                        style: None,
                        extra: None,
                        span: Span::new(span_start, pos),
                        append_whitespace: true,
                    });
                }
            }
        }

        self.cap_suggestions(suggestions)
    }
}

impl SqlCompleter {
    /// Cap the number of suggestions to the configured maximum
    fn cap_suggestions(&self, mut suggestions: Vec<Suggestion>) -> Vec<Suggestion> {
        suggestions.truncate(self.config.max_suggestions);
        suggestions
    }
}
