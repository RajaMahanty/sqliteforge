pub mod validator;
pub mod highlighter;
pub mod prompt;

use reedline::{
    default_emacs_keybindings, ColumnarMenu, Emacs, KeyCode, KeyModifiers,
    MenuBuilder, Reedline, ReedlineEvent, ReedlineMenu, Signal,
};

use std::collections::HashMap;

use crate::commands::{self, DotCommandResult};
use crate::completion::SqlCompleter;
use crate::config::Config;
use crate::database::Database;
use crate::explorer::Explorer;
use crate::history::History;
use crate::renderer::Renderer;

use self::highlighter::SqlHighlighter;
use self::prompt::SqlPrompt;
use self::validator::SqlValidator;

use std::fs;

/// Run the interactive shell
pub fn run(db: Database, mut config: Config) -> Result<(), Box<dyn std::error::Error>> {
    // Set up history
    let history = if config.history {
        History::open().ok()
    } else {
        None
    };

    // Set up completer with schema info
    let mut completer = SqlCompleter::new();
    refresh_completer(&db, &mut completer);

    // Set up explorer
    let mut explorer = Explorer::new();
    explorer.refresh(&db);

    // Output file redirection
    let mut output_file: Option<String> = None;

    // Set the null value
    let mut db = db;
    db.nullvalue = config.nullvalue.clone();

    // Build the prompt
    let db_name = std::path::Path::new(&db.path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| db.path.clone());

    let prompt = SqlPrompt::new(&db_name);

    // Set up keybindings
    let mut keybindings = default_emacs_keybindings();

    // Tab: if menu is open → cycle next; otherwise → open menu
    keybindings.add_binding(
        KeyModifiers::NONE,
        KeyCode::Tab,
        ReedlineEvent::UntilFound(vec![
            ReedlineEvent::MenuNext,
            ReedlineEvent::Menu("completion_menu".to_string()),
        ]),
    );
    keybindings.add_binding(
        KeyModifiers::SHIFT,
        KeyCode::BackTab,
        ReedlineEvent::MenuPrevious,
    );

    // Ctrl+E: toggle database explorer
    keybindings.add_binding(
        KeyModifiers::CONTROL,
        KeyCode::Char('e'),
        ReedlineEvent::ExecuteHostCommand("__explorer_toggle__".to_string()),
    );

    let edit_mode = Box::new(Emacs::new(keybindings));

    // Build completion menu
    let completion_menu = Box::new(
        ColumnarMenu::default()
            .with_name("completion_menu")
            .with_columns(4)
            .with_column_padding(2)
            .with_marker(""),
    );

    // Build reedline
    let mut line_editor = Reedline::create()
        .with_completer(Box::new(completer))
        .with_menu(ReedlineMenu::EngineCompleter(completion_menu))
        .with_highlighter(Box::new(SqlHighlighter))
        .with_validator(Box::new(SqlValidator))
        .with_edit_mode(edit_mode)
        .with_ansi_colors(true)
        .with_quick_completions(true)
        .with_partial_completions(true);

    // Load existing history into reedline
    if let Some(ref hist) = history {
        for entry in hist.all_entries() {
            // We can't directly add to reedline's history without its own History trait,
            // but we store it separately
            let _ = entry;
        }
    }

    // Print welcome banner
    print_banner(&db.path);

    // Main loop
    loop {
        match line_editor.read_line(&prompt) {
            Ok(Signal::Success(input)) => {
                let trimmed = input.trim();
                if trimmed.is_empty() {
                    continue;
                }

                // Save to history
                if let Some(ref hist) = history {
                    let _ = hist.add(trimmed);
                }

                // Handle Ctrl+E explorer toggle (sent via ExecuteHostCommand)
                if trimmed == "__explorer_toggle__" {
                    explorer.toggle();
                    if explorer.visible {
                        println!("{}", explorer.render());
                    }
                    continue;
                }

                // Dot commands
                if trimmed.starts_with('.') {
                    match commands::execute_dot_command(trimmed, &db, &config) {
                        DotCommandResult::Output(text) => {
                            output_text(&text, &output_file);
                        }
                        DotCommandResult::Handled => {}
                        DotCommandResult::Exit => {
                            println!("Goodbye!");
                            break;
                        }
                        DotCommandResult::Error(e) => {
                            eprintln!("\x1b[31mError: {}\x1b[0m", e);
                        }
                        DotCommandResult::ModeChanged(mode) => {
                            config.mode = mode.clone();
                            println!("Output mode changed to: {}", mode);
                        }
                        DotCommandResult::HeadersChanged(h) => {
                            config.headers = h;
                            println!(
                                "Headers {}",
                                if h { "enabled" } else { "disabled" }
                            );
                        }
                        DotCommandResult::NullvalueChanged(nv) => {
                            config.nullvalue = nv.clone();
                            db.nullvalue = nv.clone();
                            println!("Nullvalue set to: \"{}\"", nv);
                        }
                        DotCommandResult::OutputChanged(file) => {
                            match &file {
                                Some(f) => println!("Output redirected to: {}", f),
                                None => println!("Output restored to stdout"),
                            }
                            output_file = file;
                        }
                    }

                    // Refresh completer after schema-modifying commands
                    if trimmed.starts_with(".read") {
                        let mut new_completer = SqlCompleter::new();
                        refresh_completer(&db, &mut new_completer);
                        line_editor = line_editor.with_completer(Box::new(new_completer));
                        explorer.refresh(&db);
                    }

                    continue;
                }

                // SQL execution
                match db.execute_query(trimmed) {
                    Ok(result) => {
                        let output = Renderer::render(
                            &result,
                            &config.mode,
                            config.headers,
                            &config.nullvalue,
                        );
                        output_text(&output, &output_file);

                        // Refresh completer after DDL statements
                        let upper = trimmed.to_uppercase();
                        if upper.starts_with("CREATE")
                            || upper.starts_with("DROP")
                            || upper.starts_with("ALTER")
                        {
                            let mut new_completer = SqlCompleter::new();
                            refresh_completer(&db, &mut new_completer);
                            line_editor = line_editor.with_completer(Box::new(new_completer));
                            explorer.refresh(&db);
                        }
                    }
                    Err(e) => {
                        eprintln!("\x1b[31mError: {}\x1b[0m", e);
                    }
                }
            }
            Ok(Signal::CtrlC) => {
                // Cancel current input
                continue;
            }
            Ok(Signal::CtrlD) => {
                println!("Goodbye!");
                break;
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                break;
            }
        }
    }

    Ok(())
}

/// Refresh the completer with current schema
fn refresh_completer(db: &Database, completer: &mut SqlCompleter) {
    let tables = db.get_tables();
    let views = db.get_views();
    let indices = db.get_indices();

    // Gather columns per table and all columns
    let mut all_columns = Vec::new();
    let mut table_columns = HashMap::new();
    for table in &tables {
        let cols = db.get_columns(table);
        table_columns.insert(table.clone(), cols.clone());
        for col in cols {
            if !all_columns.contains(&col) {
                all_columns.push(col);
            }
        }
    }

    completer.update_schema(tables, views, all_columns, table_columns, indices);
}

/// Output text to stdout or file
fn output_text(text: &str, output_file: &Option<String>) {
    match output_file {
        Some(path) => {
            if let Err(e) = fs::write(path, text) {
                eprintln!("Error writing to {}: {}", path, e);
            }
        }
        None => println!("{}", text),
    }
}

/// Print the welcome banner
fn print_banner(db_path: &str) {
    println!(
        "\x1b[36m\x1b[1m{}",
        r#"
  ███████╗ ██████╗ ██╗     ██╗████████╗███████╗███████╗ ██████╗ ██████╗  ██████╗ ███████╗
  ██╔════╝██╔═══██╗██║     ██║╚══██╔══╝██╔════╝██╔════╝██╔═══██╗██╔══██╗██╔════╝ ██╔════╝
  ███████╗██║   ██║██║     ██║   ██║   █████╗  █████╗  ██║   ██║██████╔╝██║  ███╗█████╗  
  ╚════██║██║▄▄ ██║██║     ██║   ██║   ██╔══╝  ██╔══╝  ██║   ██║██╔══██╗██║   ██║██╔══╝  
  ███████║╚██████╔╝███████╗██║   ██║   ███████╗██║     ╚██████╔╝██║  ██║╚██████╔╝███████╗
  ╚══════╝ ╚══▀▀═╝ ╚══════╝╚═╝   ╚═╝   ╚══════╝╚═╝      ╚═════╝ ╚═╝  ╚═╝ ╚═════╝ ╚══════╝"#
    );
    println!("\x1b[0m");
    println!(
        "  \x1b[37m\x1b[1mv1.0\x1b[0m  \x1b[90m─  A modern terminal-first SQLite client\x1b[0m"
    );
    println!("  \x1b[90mConnected to: \x1b[33m{}\x1b[0m", db_path);
    println!(
        "  \x1b[90mType \x1b[36m.help\x1b[90m for commands, \x1b[36mCtrl+D\x1b[90m to exit\x1b[0m"
    );
    println!();
}
