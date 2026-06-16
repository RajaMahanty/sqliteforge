use crate::database::Database;

/// Database explorer panel data
pub struct Explorer {
    pub tables: Vec<String>,
    pub views: Vec<String>,
    pub indices: Vec<String>,
    pub visible: bool,
}

impl Explorer {
    pub fn new() -> Self {
        Self {
            tables: Vec::new(),
            views: Vec::new(),
            indices: Vec::new(),
            visible: false,
        }
    }

    /// Refresh the explorer data from the database
    pub fn refresh(&mut self, db: &Database) {
        self.tables = db.get_tables();
        self.views = db.get_views();
        self.indices = db.get_indices();
    }

    /// Toggle visibility
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    /// Render the explorer panel as a string
    pub fn render(&self) -> String {
        let mut out = String::new();
        out.push_str("╔══════════════════════════════╗\n");
        out.push_str("║    Database Explorer         ║\n");
        out.push_str("╠══════════════════════════════╣\n");

        if !self.tables.is_empty() {
            out.push_str("║  Tables                      ║\n");
            for table in &self.tables {
                out.push_str(&format!("║  ├── {:<23}║\n", table));
            }
        }

        if !self.views.is_empty() {
            out.push_str("║                              ║\n");
            out.push_str("║  Views                       ║\n");
            for view in &self.views {
                out.push_str(&format!("║  ├── {:<23}║\n", view));
            }
        }

        if !self.indices.is_empty() {
            out.push_str("║                              ║\n");
            out.push_str("║  Indexes                     ║\n");
            for idx in &self.indices {
                out.push_str(&format!("║  ├── {:<23}║\n", idx));
            }
        }

        if self.tables.is_empty() && self.views.is_empty() && self.indices.is_empty() {
            out.push_str("║  (empty database)            ║\n");
        }

        out.push_str("╚══════════════════════════════╝\n");
        out.push_str("  Press Ctrl+E to close\n");
        out
    }
}
