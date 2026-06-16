pub use crate::config::Config;
pub use crate::database::Database;

/// Application state holder
pub struct App {
    pub config: Config,
    pub db: Database,
}

impl App {
    /// Create a new application instance
    pub fn new(db_path: Option<&str>) -> Result<Self, Box<dyn std::error::Error>> {
        let config = Config::load();

        let db = match db_path {
            Some(path) => Database::open(path).map_err(|e| {
                format!("Failed to open database '{}': {}", path, e)
            })?,
            None => Database::open_in_memory().map_err(|e| {
                format!("Failed to create in-memory database: {}", e)
            })?,
        };

        Ok(Self { config, db })
    }

    /// Run the application
    pub fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        crate::shell::run(self.db, self.config)
    }
}
