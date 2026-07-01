use crate::config::ExplorerConfig;
use crate::database::Database;
use std::collections::HashMap;

/// Database explorer panel data
pub struct Explorer {
    pub tables: Vec<String>,
    pub views: Vec<String>,
    pub indices: Vec<String>,
    /// Column info per table: (column_name, column_type)
    pub table_columns: HashMap<String, Vec<(String, String)>>,
    /// Row counts per table (optional)
    pub table_row_counts: HashMap<String, usize>,
    pub visible: bool,
    pub config: ExplorerConfig,
}

impl Explorer {
    pub fn new() -> Self {
        Self {
            tables: Vec::new(),
            views: Vec::new(),
            indices: Vec::new(),
            table_columns: HashMap::new(),
            table_row_counts: HashMap::new(),
            visible: false,
            config: ExplorerConfig::default(),
        }
    }

    pub fn with_config(config: &ExplorerConfig) -> Self {
        Self {
            tables: Vec::new(),
            views: Vec::new(),
            indices: Vec::new(),
            table_columns: HashMap::new(),
            table_row_counts: HashMap::new(),
            visible: false,
            config: config.clone(),
        }
    }

    /// Refresh the explorer data from the database
    pub fn refresh(&mut self, db: &Database) {
        self.tables = db.get_tables();
        self.views = db.get_views();
        self.indices = db.get_indices();

        self.table_columns.clear();
        self.table_row_counts.clear();

        for table in &self.tables {
            if self.config.show_columns {
                let cols = db.get_column_info(table);
                self.table_columns.insert(table.clone(), cols);
            }
            if self.config.show_row_counts {
                if let Some(count) = db.get_row_count(table) {
                    self.table_row_counts.insert(table.clone(), count);
                }
            }
        }
    }

    /// Toggle visibility
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    /// Render the explorer panel as a string
    pub fn render(&self) -> String {
        let w = self.config.panel_width;
        // Inner width is panel_width minus the two border chars ("║" on each side)
        let inner = w.saturating_sub(2);
        let mut out = String::new();

        // Newline before the panel for visual separation
        out.push('\n');

        // Top border
        out.push('╔');
        out.push_str(&"═".repeat(inner));
        out.push_str("╗\n");

        // Title
        let title = "Database Explorer";
        let title_pad = inner.saturating_sub(title.len());
        let title_left = title_pad / 2;
        let title_right = title_pad - title_left;
        out.push_str(&format!(
            "║{}{}{}║\n",
            " ".repeat(title_left),
            title,
            " ".repeat(title_right),
        ));

        // Separator
        out.push('╠');
        out.push_str(&"═".repeat(inner));
        out.push_str("╣\n");

        if self.tables.is_empty() && self.views.is_empty() && self.indices.is_empty() {
            let msg = "(empty database)";
            let pad = inner.saturating_sub(msg.len() + 2);
            out.push_str(&format!("║  {}{}║\n", msg, " ".repeat(pad)));
        }

        // Tables section
        if !self.tables.is_empty() {
            let section = "Tables";
            let pad = inner.saturating_sub(section.len() + 2);
            out.push_str(&format!("║  {}{}║\n", section, " ".repeat(pad)));

            for (i, table) in self.tables.iter().enumerate() {
                let is_last =
                    i == self.tables.len() - 1 && self.views.is_empty() && self.indices.is_empty();
                let connector = if is_last && !self.config.show_columns {
                    "└── "
                } else {
                    "├── "
                };

                // Format table name with optional row count
                let table_display = if self.config.show_row_counts {
                    if let Some(count) = self.table_row_counts.get(table) {
                        format!("{} ({})", table, count)
                    } else {
                        table.clone()
                    }
                } else {
                    table.clone()
                };

                let prefix = format!("{}{}", connector, table_display);
                let pad = inner.saturating_sub(prefix.len() + 2);
                out.push_str(&format!("║  {}{}║\n", prefix, " ".repeat(pad)));

                // Show columns under this table
                if self.config.show_columns {
                    if let Some(columns) = self.table_columns.get(table) {
                        for (ci, (col_name, col_type)) in columns.iter().enumerate() {
                            let is_last_col = ci == columns.len() - 1;
                            let tree_prefix = if is_last && !self.config.show_columns {
                                "    "
                            } else {
                                "│   "
                            };
                            let col_connector = if is_last_col { "└─" } else { "├─" };

                            let col_display =
                                if self.config.show_column_types && !col_type.is_empty() {
                                    format!("{} {}", col_name, col_type)
                                } else {
                                    col_name.clone()
                                };

                            let prefix =
                                format!("{}{} {}", tree_prefix, col_connector, col_display);
                            let pad = inner.saturating_sub(prefix.len() + 2);
                            // Dim color for column lines
                            out.push_str(&format!(
                                "║  \x1b[90m{}\x1b[0m{}║\n",
                                prefix,
                                " ".repeat(pad)
                            ));
                        }
                    }
                }
            }
        }

        // Views section
        if !self.views.is_empty() {
            // Blank separator line
            let pad = inner;
            out.push_str(&format!("║{}║\n", " ".repeat(pad)));

            let section = "Views";
            let pad = inner.saturating_sub(section.len() + 2);
            out.push_str(&format!("║  {}{}║\n", section, " ".repeat(pad)));

            for (i, view) in self.views.iter().enumerate() {
                let is_last = i == self.views.len() - 1 && self.indices.is_empty();
                let connector = if is_last { "└── " } else { "├── " };
                let prefix = format!("{}{}", connector, view);
                let pad = inner.saturating_sub(prefix.len() + 2);
                out.push_str(&format!("║  {}{}║\n", prefix, " ".repeat(pad)));
            }
        }

        // Indexes section
        if !self.indices.is_empty() {
            let pad = inner;
            out.push_str(&format!("║{}║\n", " ".repeat(pad)));

            let section = "Indexes";
            let pad = inner.saturating_sub(section.len() + 2);
            out.push_str(&format!("║  {}{}║\n", section, " ".repeat(pad)));

            for (i, idx) in self.indices.iter().enumerate() {
                let is_last = i == self.indices.len() - 1;
                let connector = if is_last { "└── " } else { "├── " };
                let prefix = format!("{}{}", connector, idx);
                let pad = inner.saturating_sub(prefix.len() + 2);
                out.push_str(&format!("║  {}{}║\n", prefix, " ".repeat(pad)));
            }
        }

        // Bottom border
        out.push('╚');
        out.push_str(&"═".repeat(inner));
        out.push_str("╝\n");

        out.push_str("  Press Ctrl+E to close\n");
        out
    }
}
