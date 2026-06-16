use crate::database::QueryResult;
use unicode_width::UnicodeWidthStr;

/// Render query results in different output modes
pub struct Renderer;

impl Renderer {
    /// Render results based on the specified mode
    pub fn render(result: &QueryResult, mode: &str, headers: bool, nullvalue: &str) -> String {
        if !result.is_select || result.columns.is_empty() {
            return Self::render_modify_result(result);
        }

        match mode {
            "box" => Self::render_box(result, headers, nullvalue),
            "table" => Self::render_table(result, headers, nullvalue),
            "column" => Self::render_column(result, headers, nullvalue),
            "markdown" => Self::render_markdown(result, headers, nullvalue),
            "csv" => Self::render_csv(result, headers),
            "json" => Self::render_json(result),
            "list" => Self::render_list(result, headers),
            _ => Self::render_box(result, headers, nullvalue),
        }
    }

    /// Render non-SELECT result
    fn render_modify_result(result: &QueryResult) -> String {
        format!(
            "Changes: {}\nExecution Time: {:.1} ms",
            result.rows_affected, result.execution_time_ms
        )
    }

    /// Compute column widths
    fn column_widths(result: &QueryResult, _nullvalue: &str) -> Vec<usize> {
        let mut widths: Vec<usize> = result
            .columns
            .iter()
            .map(|c| UnicodeWidthStr::width(c.as_str()))
            .collect();

        for row in &result.rows {
            for (i, val) in row.iter().enumerate() {
                if i < widths.len() {
                    let w = UnicodeWidthStr::width(val.as_str());
                    if w > widths[i] {
                        widths[i] = w;
                    }
                }
            }
        }
        widths
    }

    /// Box mode (default) - Unicode box drawing
    fn render_box(result: &QueryResult, headers: bool, nullvalue: &str) -> String {
        let widths = Self::column_widths(result, nullvalue);
        let mut out = String::new();

        // Top border
        out.push('┌');
        for (i, w) in widths.iter().enumerate() {
            out.push_str(&"─".repeat(w + 2));
            if i < widths.len() - 1 {
                out.push('┬');
            }
        }
        out.push_str("┐\n");

        // Header
        if headers {
            out.push('│');
            for (i, col) in result.columns.iter().enumerate() {
                let pad = widths[i] - UnicodeWidthStr::width(col.as_str());
                out.push(' ');
                out.push_str(col);
                out.push_str(&" ".repeat(pad));
                out.push_str(" │");
            }
            out.push('\n');

            // Header separator
            out.push('├');
            for (i, w) in widths.iter().enumerate() {
                out.push_str(&"─".repeat(w + 2));
                if i < widths.len() - 1 {
                    out.push('┼');
                }
            }
            out.push_str("┤\n");
        }

        // Rows
        for row in &result.rows {
            out.push('│');
            for (i, val) in row.iter().enumerate() {
                let w = if i < widths.len() { widths[i] } else { 0 };
                let display_width = UnicodeWidthStr::width(val.as_str());
                let pad = if w >= display_width {
                    w - display_width
                } else {
                    0
                };
                out.push(' ');
                out.push_str(val);
                out.push_str(&" ".repeat(pad));
                out.push_str(" │");
            }
            out.push('\n');
        }

        // Bottom border
        out.push('└');
        for (i, w) in widths.iter().enumerate() {
            out.push_str(&"─".repeat(w + 2));
            if i < widths.len() - 1 {
                out.push('┴');
            }
        }
        out.push_str("┘\n");

        // Stats
        out.push_str(&format!(
            "Rows: {}  Execution Time: {:.1} ms",
            result.rows.len(),
            result.execution_time_ms
        ));

        out
    }

    /// Table mode - ASCII table
    fn render_table(result: &QueryResult, headers: bool, nullvalue: &str) -> String {
        let widths = Self::column_widths(result, nullvalue);
        let mut out = String::new();

        let separator = || {
            let mut s = String::from("+");
            for w in &widths {
                s.push_str(&"-".repeat(w + 2));
                s.push('+');
            }
            s.push('\n');
            s
        };

        out.push_str(&separator());

        if headers {
            out.push('|');
            for (i, col) in result.columns.iter().enumerate() {
                let pad = widths[i] - UnicodeWidthStr::width(col.as_str());
                out.push(' ');
                out.push_str(col);
                out.push_str(&" ".repeat(pad));
                out.push_str(" |");
            }
            out.push('\n');
            out.push_str(&separator());
        }

        for row in &result.rows {
            out.push('|');
            for (i, val) in row.iter().enumerate() {
                let w = if i < widths.len() { widths[i] } else { 0 };
                let display_width = UnicodeWidthStr::width(val.as_str());
                let pad = if w >= display_width {
                    w - display_width
                } else {
                    0
                };
                out.push(' ');
                out.push_str(val);
                out.push_str(&" ".repeat(pad));
                out.push_str(" |");
            }
            out.push('\n');
        }

        out.push_str(&separator());
        out.push_str(&format!(
            "Rows: {}  Execution Time: {:.1} ms",
            result.rows.len(),
            result.execution_time_ms
        ));

        out
    }

    /// Column mode - aligned columns without borders
    fn render_column(result: &QueryResult, headers: bool, nullvalue: &str) -> String {
        let widths = Self::column_widths(result, nullvalue);
        let mut out = String::new();

        if headers {
            for (i, col) in result.columns.iter().enumerate() {
                let pad = widths[i] - UnicodeWidthStr::width(col.as_str());
                out.push_str(col);
                out.push_str(&" ".repeat(pad + 2));
            }
            out.push('\n');

            for (i, w) in widths.iter().enumerate() {
                out.push_str(&"-".repeat(*w));
                if i < widths.len() - 1 {
                    out.push_str("  ");
                }
            }
            out.push('\n');
        }

        for row in &result.rows {
            for (i, val) in row.iter().enumerate() {
                let w = if i < widths.len() { widths[i] } else { 0 };
                let display_width = UnicodeWidthStr::width(val.as_str());
                let pad = if w >= display_width {
                    w - display_width
                } else {
                    0
                };
                out.push_str(val);
                out.push_str(&" ".repeat(pad + 2));
            }
            out.push('\n');
        }

        out.push_str(&format!(
            "Rows: {}  Execution Time: {:.1} ms",
            result.rows.len(),
            result.execution_time_ms
        ));

        out
    }

    /// Markdown mode
    fn render_markdown(result: &QueryResult, headers: bool, nullvalue: &str) -> String {
        let widths = Self::column_widths(result, nullvalue);
        let mut out = String::new();

        if headers {
            out.push('|');
            for (i, col) in result.columns.iter().enumerate() {
                let pad = widths[i] - UnicodeWidthStr::width(col.as_str());
                out.push(' ');
                out.push_str(col);
                out.push_str(&" ".repeat(pad));
                out.push_str(" |");
            }
            out.push('\n');

            out.push('|');
            for w in &widths {
                out.push(' ');
                out.push_str(&"-".repeat(*w));
                out.push_str(" |");
            }
            out.push('\n');
        }

        for row in &result.rows {
            out.push('|');
            for (i, val) in row.iter().enumerate() {
                let w = if i < widths.len() { widths[i] } else { 0 };
                let display_width = UnicodeWidthStr::width(val.as_str());
                let pad = if w >= display_width {
                    w - display_width
                } else {
                    0
                };
                out.push(' ');
                out.push_str(val);
                out.push_str(&" ".repeat(pad));
                out.push_str(" |");
            }
            out.push('\n');
        }

        out
    }

    /// CSV mode
    fn render_csv(result: &QueryResult, headers: bool) -> String {
        let mut out = String::new();

        if headers {
            out.push_str(&result.columns.join(","));
            out.push('\n');
        }

        for row in &result.rows {
            let escaped: Vec<String> = row
                .iter()
                .map(|v| {
                    if v.contains(',') || v.contains('"') || v.contains('\n') {
                        format!("\"{}\"", v.replace('"', "\"\""))
                    } else {
                        v.clone()
                    }
                })
                .collect();
            out.push_str(&escaped.join(","));
            out.push('\n');
        }

        out
    }

    /// JSON mode
    fn render_json(result: &QueryResult) -> String {
        let mut out = String::from("[\n");

        for (row_idx, row) in result.rows.iter().enumerate() {
            out.push_str("  {");
            for (i, val) in row.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                let col = &result.columns[i];
                // Try to detect numbers
                if val.is_empty() {
                    out.push_str(&format!("\"{}\": null", col));
                } else if let Ok(n) = val.parse::<i64>() {
                    out.push_str(&format!("\"{}\": {}", col, n));
                } else if let Ok(f) = val.parse::<f64>() {
                    out.push_str(&format!("\"{}\": {}", col, f));
                } else {
                    let escaped = val
                        .replace('\\', "\\\\")
                        .replace('"', "\\\"")
                        .replace('\n', "\\n")
                        .replace('\r', "\\r")
                        .replace('\t', "\\t");
                    out.push_str(&format!("\"{}\": \"{}\"", col, escaped));
                }
            }
            out.push('}');
            if row_idx < result.rows.len() - 1 {
                out.push(',');
            }
            out.push('\n');
        }

        out.push(']');
        out
    }

    /// List mode - pipe separated
    fn render_list(result: &QueryResult, headers: bool) -> String {
        let mut out = String::new();

        if headers {
            out.push_str(&result.columns.join("|"));
            out.push('\n');
        }

        for row in &result.rows {
            out.push_str(&row.join("|"));
            out.push('\n');
        }

        out
    }
}
