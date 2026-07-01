use crate::config::Config;
use crate::database::Database;
use std::fs;

/// Result of executing a dot command
pub enum DotCommandResult {
    /// Output to display
    Output(String),
    /// Command was handled, no output needed
    Handled,
    /// Exit the shell
    Exit,
    /// Error message
    Error(String),
    /// Mode changed
    ModeChanged(String),
    /// Headers setting changed
    HeadersChanged(bool),
    /// Nullvalue changed
    NullvalueChanged(String),
    /// Output file changed
    OutputChanged(Option<String>),
}

/// Process dot commands
pub fn execute_dot_command(input: &str, db: &Database, config: &Config) -> DotCommandResult {
    let parts: Vec<&str> = input.trim().splitn(2, char::is_whitespace).collect();
    let command = parts[0].to_lowercase();
    let args = if parts.len() > 1 { parts[1].trim() } else { "" };

    match command.as_str() {
        ".help" => DotCommandResult::Output(help_text()),
        ".quit" | ".exit" => DotCommandResult::Exit,
        ".tables" => {
            let tables = db.get_tables();
            if tables.is_empty() {
                DotCommandResult::Output("No tables found.".to_string())
            } else {
                DotCommandResult::Output(tables.join("\n"))
            }
        }
        ".schema" => {
            if args.is_empty() {
                let schemas = db.get_all_schemas();
                if schemas.is_empty() {
                    DotCommandResult::Output("No Schema Found".to_string())
                } else {
                    DotCommandResult::Output(schemas.join(";\n\n") + ";")
                }
            } else {
                let name = args.trim_matches('"').trim_matches('\'');
                match db.get_schema(name) {
                    Some(schema) => DotCommandResult::Output(schema + ";"),
                    None => DotCommandResult::Error(format!("No such object: {}", name)),
                }
            }
        }
        ".indices" => {
            let indices = db.get_indices();
            if indices.is_empty() {
                DotCommandResult::Output("No indices found.".to_string())
            } else {
                DotCommandResult::Output(indices.join("\n"))
            }
        }
        ".mode" => {
            if args.is_empty() {
                DotCommandResult::Output(format!("Current mode: {}", config.mode))
            } else {
                let mode = args.to_lowercase();
                match mode.as_str() {
                    "box" | "table" | "column" | "markdown" | "csv" | "json" | "list" => {
                        DotCommandResult::ModeChanged(mode)
                    }
                    _ => DotCommandResult::Error(format!(
                        "Unknown mode: {}. Available: box, table, column, markdown, csv, json, list",
                        mode
                    )),
                }
            }
        }
        ".headers" => {
            if args.is_empty() {
                DotCommandResult::Output(format!(
                    "Headers: {}",
                    if config.headers { "on" } else { "off" }
                ))
            } else {
                match args.to_lowercase().as_str() {
                    "on" | "yes" | "true" | "1" => DotCommandResult::HeadersChanged(true),
                    "off" | "no" | "false" | "0" => DotCommandResult::HeadersChanged(false),
                    _ => DotCommandResult::Error("Usage: .headers on|off".to_string()),
                }
            }
        }
        ".read" => {
            if args.is_empty() {
                DotCommandResult::Error("Usage: .read FILENAME".to_string())
            } else {
                match fs::read_to_string(args) {
                    Ok(content) => match db.execute_script(&content) {
                        Ok(_) => DotCommandResult::Output(format!("Executed script: {}", args)),
                        Err(e) => DotCommandResult::Error(format!("Script error: {}", e)),
                    },
                    Err(e) => DotCommandResult::Error(format!("Cannot read file: {}", e)),
                }
            }
        }
        ".output" => {
            if args.is_empty() || args == "stdout" {
                DotCommandResult::OutputChanged(None)
            } else {
                DotCommandResult::OutputChanged(Some(args.to_string()))
            }
        }
        ".dump" => match db.dump() {
            Ok(dump) => DotCommandResult::Output(dump),
            Err(e) => DotCommandResult::Error(format!("Dump error: {}", e)),
        },
        ".nullvalue" => {
            if args.is_empty() {
                DotCommandResult::Output(format!("Nullvalue: \"{}\"", config.nullvalue))
            } else {
                DotCommandResult::NullvalueChanged(args.to_string())
            }
        }
        ".show" => {
            let mut out = String::new();
            out.push_str(&format!("    database: {}\n", db.path));
            out.push_str(&format!("        mode: {}\n", config.mode));
            out.push_str(&format!(
                "     headers: {}\n",
                if config.headers { "on" } else { "off" }
            ));
            out.push_str(&format!("   nullvalue: \"{}\"\n", config.nullvalue));
            out.push_str(&format!(
                "     history: {}\n",
                if config.history { "on" } else { "off" }
            ));
            out.push_str(&format!("       theme: {}\n", config.theme));
            DotCommandResult::Output(out)
        }
        ".preview" => {
            if args.is_empty() {
                DotCommandResult::Error("Usage: .preview TABLE_NAME".to_string())
            } else {
                let name = args.trim_matches('"').trim_matches('\'');
                let sql = format!("SELECT * FROM \"{}\" LIMIT 20", name.replace('"', "\"\""));
                match db.execute_query(&sql) {
                    Ok(result) => {
                        let output = crate::renderer::Renderer::render(
                            &result,
                            &config.mode,
                            config.headers,
                            &config.nullvalue,
                        );
                        DotCommandResult::Output(output)
                    }
                    Err(e) => DotCommandResult::Error(format!("Preview error: {}", e)),
                }
            }
        }
        _ => DotCommandResult::Error(format!(
            "Unknown command: {}. Use .help for a list.",
            command
        )),
    }
}

fn help_text() -> String {
    let config_path = crate::config::Config::config_path();
    format!(
        r#"SQLiteForge v1.0 - Available Commands:

  .help                   Show this help message
  .quit / .exit           Exit SQLiteForge
  .tables                 List all tables
  .schema [TABLE]         Show CREATE statements
  .indices                List all indices
  .mode [MODE]            Set output mode (box|table|column|markdown|csv|json|list)
  .headers [on|off]       Toggle column headers
  .read FILENAME          Execute a SQL script file
  .output [FILENAME]      Redirect output to file (no args = stdout)
  .dump                   Dump entire database as SQL
  .nullvalue [STRING]     Set string to display for NULL values
  .show                   Show current settings
  .preview TABLE          Preview first 20 rows of a table

  Keyboard Shortcuts:
    Tab                 Autocomplete
    Shift+Tab           Previous suggestion
    Up/Down Arrow       Navigate history
    Shift+Left/Right    Select text
    Shift+Up/Down       Select to line start/end
    Ctrl+Left/Right     Jump by word
    Ctrl+Shift+L/R      Select by word
    Ctrl+R              Reverse search history
    Ctrl+E              Toggle database explorer
    Ctrl+A              Move to line start
    Ctrl+L              Clear screen
    Ctrl+C              Cancel input
    Ctrl+D              Exit

  Config: {}"#,
        config_path.display()
    )
}
