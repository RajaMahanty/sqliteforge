pub mod highlighter;
pub mod prompt;
pub mod validator;

use reedline::{
    default_emacs_keybindings, ColumnarMenu, EditCommand, Emacs, FileBackedHistory, KeyCode,
    KeyModifiers, MenuBuilder, Reedline, ReedlineEvent, ReedlineMenu, Signal,
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

    // Set up completer with schema info and config
    let mut completer = SqlCompleter::with_config(&config.completion);
    refresh_completer(&db, &mut completer);

    // Set up explorer with config
    let mut explorer = Explorer::with_config(&config.explorer);
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

    // ── Smart Enter: use reedline's native Enter which handles menu selection,
    // validator-based submission, and multiline newline insertion ────
    // Note: auto_indent is handled separately via the __auto_indent__ host command
    // which is no longer needed since reedline's Enter handles everything natively.

    // ── Auto-close brackets ─────────────────────────────────────────────
    if config.keybindings.auto_pairs {
        // Open brackets auto-close and move left
        keybindings.add_binding(
            KeyModifiers::NONE,
            KeyCode::Char('('),
            ReedlineEvent::Edit(vec![
                EditCommand::InsertString("()".to_string()),
                EditCommand::MoveLeft { select: false },
            ]),
        );
        keybindings.add_binding(
            KeyModifiers::NONE,
            KeyCode::Char('['),
            ReedlineEvent::Edit(vec![
                EditCommand::InsertString("[]".to_string()),
                EditCommand::MoveLeft { select: false },
            ]),
        );
        keybindings.add_binding(
            KeyModifiers::NONE,
            KeyCode::Char('{'),
            ReedlineEvent::Edit(vec![
                EditCommand::InsertString("{}".to_string()),
                EditCommand::MoveLeft { select: false },
            ]),
        );

        // Closing characters skip or insert
        keybindings.add_binding(
            KeyModifiers::NONE,
            KeyCode::Char(')'),
            ReedlineEvent::ExecuteHostCommand("__char_close_paren__".to_string()),
        );
        keybindings.add_binding(
            KeyModifiers::NONE,
            KeyCode::Char(']'),
            ReedlineEvent::ExecuteHostCommand("__char_close_bracket__".to_string()),
        );
        keybindings.add_binding(
            KeyModifiers::NONE,
            KeyCode::Char('}'),
            ReedlineEvent::ExecuteHostCommand("__char_close_brace__".to_string()),
        );

        // Quotes skip or auto-close
        keybindings.add_binding(
            KeyModifiers::NONE,
            KeyCode::Char('\''),
            ReedlineEvent::ExecuteHostCommand("__char_quote__".to_string()),
        );
        keybindings.add_binding(
            KeyModifiers::NONE,
            KeyCode::Char('"'),
            ReedlineEvent::ExecuteHostCommand("__char_double_quote__".to_string()),
        );
    }

    // ── Shift+Arrow keybindings for text selection ───────────────────────
    if config.keybindings.shift_select {
        keybindings.add_binding(
            KeyModifiers::SHIFT,
            KeyCode::Left,
            ReedlineEvent::Edit(vec![EditCommand::MoveLeft { select: true }]),
        );
        keybindings.add_binding(
            KeyModifiers::SHIFT,
            KeyCode::Right,
            ReedlineEvent::Edit(vec![EditCommand::MoveRight { select: true }]),
        );
        keybindings.add_binding(
            KeyModifiers::SHIFT,
            KeyCode::Up,
            ReedlineEvent::Edit(vec![EditCommand::MoveToLineStart { select: true }]),
        );
        keybindings.add_binding(
            KeyModifiers::SHIFT,
            KeyCode::Down,
            ReedlineEvent::Edit(vec![EditCommand::MoveToLineEnd { select: true }]),
        );
        keybindings.add_binding(
            KeyModifiers::SHIFT,
            KeyCode::Home,
            ReedlineEvent::Edit(vec![EditCommand::MoveToLineStart { select: true }]),
        );
        keybindings.add_binding(
            KeyModifiers::SHIFT,
            KeyCode::End,
            ReedlineEvent::Edit(vec![EditCommand::MoveToLineEnd { select: true }]),
        );
    }

    // ── Ctrl+Arrow for word-level navigation ────────────────────────────
    if config.keybindings.word_jump {
        keybindings.add_binding(
            KeyModifiers::CONTROL,
            KeyCode::Left,
            ReedlineEvent::Edit(vec![EditCommand::MoveWordLeft { select: false }]),
        );
        keybindings.add_binding(
            KeyModifiers::CONTROL,
            KeyCode::Right,
            ReedlineEvent::Edit(vec![EditCommand::MoveWordRight { select: false }]),
        );
        keybindings.add_binding(
            KeyModifiers::CONTROL | KeyModifiers::SHIFT,
            KeyCode::Left,
            ReedlineEvent::Edit(vec![EditCommand::MoveWordLeft { select: true }]),
        );
        keybindings.add_binding(
            KeyModifiers::CONTROL | KeyModifiers::SHIFT,
            KeyCode::Right,
            ReedlineEvent::Edit(vec![EditCommand::MoveWordRight { select: true }]),
        );
    }

    // Ctrl+A: move to beginning of line
    keybindings.add_binding(
        KeyModifiers::CONTROL,
        KeyCode::Char('a'),
        ReedlineEvent::Edit(vec![EditCommand::MoveToLineStart { select: false }]),
    );

    let edit_mode = Box::new(Emacs::new(keybindings));

    // Build completion menu
    let completion_menu = Box::new(
        ColumnarMenu::default()
            .with_name("completion_menu")
            .with_columns(config.completion.menu_columns)
            .with_column_padding(config.completion.menu_padding)
            .with_marker(""),
    );

    // Build reedline history
    let reedline_history = Box::new(
        FileBackedHistory::with_file(1000, History::history_path().with_extension("txt"))
            .expect("Failed to initialize reedline history"),
    );

    // Build reedline with validator for multi-line support
    let mut line_editor = Reedline::create()
        .with_history(reedline_history)
        .with_history_exclusion_prefix(Some("__".to_string()))
        .with_completer(Box::new(completer))
        .with_menu(ReedlineMenu::EngineCompleter(completion_menu))
        .with_highlighter(Box::new(SqlHighlighter))
        .with_validator(Box::new(SqlValidator))
        .with_edit_mode(edit_mode)
        .with_ansi_colors(true)
        .with_quick_completions(true)
        .with_partial_completions(true);

    // Print welcome banner
    print_banner(&db.path);

    // Main loop
    let mut should_exit = false;

    loop {
        if should_exit {
            break;
        }

        match line_editor.read_line(&prompt) {
            Ok(Signal::Success(input)) => {
                let trimmed = input.trim();

                // ── Auto-close / Skip handlers ──────────────────────────
                if trimmed == "__char_close_paren__" {
                    handle_skip_or_insert_char(&mut line_editor, ')');
                    continue;
                }
                if trimmed == "__char_close_bracket__" {
                    handle_skip_or_insert_char(&mut line_editor, ']');
                    continue;
                }
                if trimmed == "__char_close_brace__" {
                    handle_skip_or_insert_char(&mut line_editor, '}');
                    continue;
                }
                if trimmed == "__char_quote__" {
                    handle_quote_char(&mut line_editor, '\'');
                    continue;
                }
                if trimmed == "__char_double_quote__" {
                    handle_quote_char(&mut line_editor, '"');
                    continue;
                }

                // ── Explorer toggle ─────────────────────────────────────
                if trimmed == "__explorer_toggle__" {
                    explorer.toggle();
                    if explorer.visible {
                        println!("{}", explorer.render());
                    }
                    continue;
                }

                // ── Normal flow (auto_indent disabled, validator handles multiline)
                if trimmed.is_empty() {
                    continue;
                }

                // Save to history
                if let Some(ref hist) = history {
                    let _ = hist.add(trimmed);
                }

                // Dot commands
                if trimmed.starts_with('.') {
                    process_dot_command(
                        trimmed,
                        &db,
                        &mut config,
                        &mut output_file,
                        &mut line_editor,
                        &mut explorer,
                        &mut should_exit,
                    );
                    continue;
                }

                // SQL execution
                execute_sql(
                    trimmed,
                    &db,
                    &config,
                    &output_file,
                    &mut line_editor,
                    &mut explorer,
                );
            }
            Ok(Signal::CtrlC) => {
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

/// Process a dot command
fn process_dot_command(
    cmd: &str,
    db: &Database,
    config: &mut Config,
    output_file: &mut Option<String>,
    line_editor: &mut Reedline,
    explorer: &mut Explorer,
    should_exit: &mut bool,
) {
    match commands::execute_dot_command(cmd, db, config) {
        DotCommandResult::Output(text) => {
            output_text(&text, output_file);
        }
        DotCommandResult::Handled => {}
        DotCommandResult::Exit => {
            println!("Goodbye!");
            *should_exit = true;
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
            println!("Headers {}", if h { "enabled" } else { "disabled" });
        }
        DotCommandResult::NullvalueChanged(nv) => {
            config.nullvalue = nv.clone();
            println!("Nullvalue set to: \"{}\"", nv);
        }
        DotCommandResult::OutputChanged(file) => {
            match &file {
                Some(f) => println!("Output redirected to: {}", f),
                None => println!("Output restored to stdout"),
            }
            *output_file = file;
        }
    }

    // Refresh completer after schema-modifying commands
    if cmd.starts_with(".read") {
        let mut new_completer = SqlCompleter::with_config(&config.completion);
        refresh_completer(db, &mut new_completer);
        *line_editor = std::mem::replace(line_editor, Reedline::create())
            .with_completer(Box::new(new_completer));
        explorer.refresh(db);
    }
}

/// Execute a SQL statement and handle results
fn execute_sql(
    sql: &str,
    db: &Database,
    config: &Config,
    output_file: &Option<String>,
    line_editor: &mut Reedline,
    explorer: &mut Explorer,
) {
    match db.execute_query(sql) {
        Ok(result) => {
            let output = Renderer::render(&result, &config.mode, config.headers, &config.nullvalue);
            output_text(&output, output_file);

            // Refresh completer after DDL statements
            let upper = sql.to_uppercase();
            if upper.starts_with("CREATE")
                || upper.starts_with("DROP")
                || upper.starts_with("ALTER")
            {
                let mut new_completer = SqlCompleter::with_config(&config.completion);
                refresh_completer(db, &mut new_completer);
                *line_editor = std::mem::replace(line_editor, Reedline::create())
                    .with_completer(Box::new(new_completer));
                explorer.refresh(db);
            }
        }
        Err(e) => {
            eprintln!("\x1b[31mError: {}\x1b[0m", e);
        }
    }
}

/// Refresh the completer with current schema
fn refresh_completer(db: &Database, completer: &mut SqlCompleter) {
    let tables = db.get_tables();
    let views = db.get_views();
    let indices = db.get_indices();

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

/// Helper to either skip over a character if it already matches the cursor's target,
/// or insert it normally.
fn handle_skip_or_insert_char(line_editor: &mut Reedline, c: char) {
    let buffer = line_editor.current_buffer_contents().to_string();
    let pos = line_editor.current_insertion_point();
    let has_char = if pos < buffer.len() {
        buffer.as_bytes()[pos] == c as u8
    } else {
        false
    };
    if has_char {
        line_editor.run_edit_commands(&[EditCommand::MoveRight { select: false }]);
    } else {
        line_editor.run_edit_commands(&[EditCommand::InsertChar(c)]);
    }
}

/// Helper to handle quotes: skip if matched, otherwise insert pair and move left.
fn handle_quote_char(line_editor: &mut Reedline, q: char) {
    let buffer = line_editor.current_buffer_contents().to_string();
    let pos = line_editor.current_insertion_point();
    let has_char = if pos < buffer.len() {
        buffer.as_bytes()[pos] == q as u8
    } else {
        false
    };
    if has_char {
        line_editor.run_edit_commands(&[EditCommand::MoveRight { select: false }]);
    } else {
        let pair = format!("{}{}", q, q);
        line_editor.run_edit_commands(&[
            EditCommand::InsertString(pair),
            EditCommand::MoveLeft { select: false },
        ]);
    }
}
