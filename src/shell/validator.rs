use reedline::{ValidationResult, Validator};

/// SQL input validator - determines if the input is complete.
/// A SQL statement is complete when it ends with a semicolon
/// (ignoring trailing whitespace), or is a dot command.
/// Also handles internal host commands (__explorer_toggle__, etc.).
pub struct SqlValidator;

impl Validator for SqlValidator {
    fn validate(&self, line: &str) -> ValidationResult {
        let trimmed = line.trim();

        // Empty input is valid (just produces nothing)
        if trimmed.is_empty() {
            return ValidationResult::Complete;
        }

        // Internal host commands are always complete
        if trimmed.starts_with("__") && trimmed.ends_with("__") {
            return ValidationResult::Complete;
        }

        // Dot commands are always complete
        if trimmed.starts_with('.') {
            return ValidationResult::Complete;
        }

        // SQL statements must end with semicolon
        if trimmed.ends_with(';') {
            ValidationResult::Complete
        } else {
            ValidationResult::Incomplete
        }
    }
}

/// Calculate the proper indentation for a new line based on the buffer content.
/// Returns the number of spaces to indent.
pub fn calculate_indent(buffer: &str) -> usize {
    let indent_width: usize = 4;
    let mut depth: i32 = 0;

    // Count unmatched open parens in the entire buffer
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut prev_char = '\0';

    for ch in buffer.chars() {
        match ch {
            '\'' if !in_double_quote && prev_char != '\'' => {
                in_single_quote = !in_single_quote;
            }
            '"' if !in_single_quote => {
                in_double_quote = !in_double_quote;
            }
            '(' if !in_single_quote && !in_double_quote => {
                depth += 1;
            }
            ')' if !in_single_quote && !in_double_quote => {
                depth -= 1;
                if depth < 0 {
                    depth = 0;
                }
            }
            _ => {}
        }
        prev_char = ch;
    }

    (depth as usize) * indent_width
}
