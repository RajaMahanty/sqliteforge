mod app;
mod commands;
mod completion;
mod config;
mod database;
mod explorer;
mod history;
mod renderer;
mod shell;

use clap::Parser;

/// SQLiteForge - A modern terminal-first SQLite client
#[derive(Parser, Debug)]
#[command(
    name = "sqliteforge",
    version = "1.0.0",
    about = "A modern terminal-first SQLite client",
    long_about = "SQLiteForge is a modern, feature-rich terminal client for SQLite databases.\nIt provides syntax highlighting, autocompletion, persistent history, and multiple output formats."
)]
struct Cli {
    /// Path to the SQLite database file (creates if not exists)
    #[arg(value_name = "DATABASE")]
    database: Option<String>,

    /// Execute SQL command and exit
    #[arg(short = 'c', long = "command")]
    command: Option<String>,

    /// Read and execute SQL from file
    #[arg(short = 'f', long = "file")]
    file: Option<String>,

    /// Output mode (box, table, column, markdown, csv, json, list)
    #[arg(short = 'm', long = "mode")]
    mode: Option<String>,

    /// Show version information
    #[arg(long = "version-info")]
    version_info: bool,
}

fn main() {
    let cli = Cli::parse();

    if cli.version_info {
        println!("SQLiteForge v1.0.0");
        println!("Built with Rust + rusqlite");
        println!("Terminal: crossterm");
        println!("Editor: reedline");
        return;
    }

    // Create the application
    let app = match app::App::new(cli.database.as_deref()) {
        Ok(app) => app,
        Err(e) => {
            eprintln!("\x1b[31mError: {}\x1b[0m", e);
            std::process::exit(1);
        }
    };

    // Apply CLI overrides to config
    let mut config = app.config.clone();
    if let Some(ref mode) = cli.mode {
        config.mode = mode.clone();
    }

    // Non-interactive mode: execute command
    if let Some(ref cmd) = cli.command {
        execute_noninteractive(&app.db, &config, cmd);
        return;
    }

    // Non-interactive mode: execute file
    if let Some(ref file) = cli.file {
        match std::fs::read_to_string(file) {
            Ok(content) => {
                execute_noninteractive(&app.db, &config, &content);
            }
            Err(e) => {
                eprintln!("\x1b[31mError reading file '{}': {}\x1b[0m", file, e);
                std::process::exit(1);
            }
        }
        return;
    }

    // Interactive mode
    let app = app::App {
        config,
        db: app.db,
    };
    if let Err(e) = app.run() {
        eprintln!("\x1b[31mError: {}\x1b[0m", e);
        std::process::exit(1);
    }
}

fn execute_noninteractive(db: &database::Database, config: &config::Config, sql: &str) {
    // Split by semicolons for multiple statements
    for statement in sql.split(';') {
        let trimmed = statement.trim();
        if trimmed.is_empty() {
            continue;
        }

        let full_stmt = format!("{};", trimmed);

        if trimmed.starts_with('.') {
            match commands::execute_dot_command(trimmed, db, config) {
                commands::DotCommandResult::Output(text) => println!("{}", text),
                commands::DotCommandResult::Error(e) => eprintln!("Error: {}", e),
                commands::DotCommandResult::Exit => return,
                _ => {}
            }
        } else {
            match db.execute_query(&full_stmt) {
                Ok(result) => {
                    let output = renderer::Renderer::render(
                        &result,
                        &config.mode,
                        config.headers,
                        &config.nullvalue,
                    );
                    println!("{}", output);
                }
                Err(e) => {
                    eprintln!("\x1b[31mError: {}\x1b[0m", e);
                }
            }
        }
    }
}
