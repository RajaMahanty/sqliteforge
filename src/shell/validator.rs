use reedline::{Validator, ValidationResult};

/// SQL input validator - determines if the input is complete.
/// A SQL statement is complete when it ends with a semicolon
/// (ignoring trailing whitespace), or is a dot command.
pub struct SqlValidator;

impl Validator for SqlValidator {
    fn validate(&self, line: &str) -> ValidationResult {
        let trimmed = line.trim();

        // Empty input is valid (just produces nothing)
        if trimmed.is_empty() {
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
