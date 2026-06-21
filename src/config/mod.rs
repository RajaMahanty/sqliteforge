use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Application configuration loaded from ~/.config/sqliteforge/config.toml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_theme")]
    pub theme: String,

    #[serde(default = "default_mode")]
    pub mode: String,

    #[serde(default = "default_true")]
    pub headers: bool,

    #[serde(default = "default_true")]
    pub history: bool,

    #[serde(default = "default_nullvalue")]
    pub nullvalue: String,

    #[serde(default)]
    pub keybindings: KeybindingsConfig,

    #[serde(default)]
    pub completion: CompletionConfig,

    #[serde(default)]
    pub explorer: ExplorerConfig,
}

/// Keybinding configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeybindingsConfig {
    /// Key to toggle the database explorer (default: "ctrl+e")
    #[serde(default = "default_explorer_toggle")]
    pub explorer_toggle: String,

    /// Key to execute query / submit (default: "enter")
    #[serde(default = "default_submit")]
    pub submit: String,

    /// Enable Shift+Arrow for text selection (default: true)
    #[serde(default = "default_true")]
    pub shift_select: bool,

    /// Enable Ctrl+Arrow for word-jump navigation (default: true)
    #[serde(default = "default_true")]
    pub word_jump: bool,

    /// Key to clear screen (default: "ctrl+l")
    #[serde(default = "default_clear_screen")]
    pub clear_screen: String,

    /// Auto-close brackets and quotes: ( → (), ' → '' (default: true)
    #[serde(default = "default_true")]
    pub auto_pairs: bool,

    /// Auto-indent continuation lines inside parentheses (default: true)
    #[serde(default = "default_true")]
    pub auto_indent: bool,
}

/// Autocompletion configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionConfig {
    /// Enable autocompletion (default: true)
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Show completions eagerly after keywords like SELECT, FROM, etc. (default: true)
    #[serde(default = "default_true")]
    pub eager_hint: bool,

    /// Number of columns in the completion menu (default: 4)
    #[serde(default = "default_menu_columns")]
    pub menu_columns: u16,

    /// Column padding in completion menu (default: 2)
    #[serde(default = "default_menu_padding")]
    pub menu_padding: usize,

    /// Quote identifiers in completions (default: true)
    #[serde(default = "default_true")]
    pub quote_identifiers: bool,

    /// Include column suggestions in general context (default: true)
    #[serde(default = "default_true")]
    pub suggest_columns: bool,

    /// Include keyword suggestions (default: true)
    #[serde(default = "default_true")]
    pub suggest_keywords: bool,

    /// Maximum number of suggestions shown (default: 50)
    #[serde(default = "default_max_suggestions")]
    pub max_suggestions: usize,
}

/// Database explorer panel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplorerConfig {
    /// Show column names under each table (default: true)
    #[serde(default = "default_true")]
    pub show_columns: bool,

    /// Show row counts next to table names (default: false)
    #[serde(default)]
    pub show_row_counts: bool,

    /// Show column types alongside column names (default: true)
    #[serde(default = "default_true")]
    pub show_column_types: bool,

    /// Panel width in characters (default: 40)
    #[serde(default = "default_panel_width")]
    pub panel_width: usize,
}

// ── Default value functions ─────────────────────────────────────────────────

fn default_theme() -> String {
    "catppuccin".to_string()
}

fn default_mode() -> String {
    "box".to_string()
}

fn default_true() -> bool {
    true
}

fn default_nullvalue() -> String {
    String::new()
}

fn default_explorer_toggle() -> String {
    "ctrl+e".to_string()
}

fn default_submit() -> String {
    "enter".to_string()
}

fn default_clear_screen() -> String {
    "ctrl+l".to_string()
}

fn default_menu_columns() -> u16 {
    4
}

fn default_menu_padding() -> usize {
    2
}

fn default_max_suggestions() -> usize {
    50
}

fn default_panel_width() -> usize {
    40
}

// ── Trait implementations ───────────────────────────────────────────────────

impl Default for KeybindingsConfig {
    fn default() -> Self {
        Self {
            explorer_toggle: default_explorer_toggle(),
            submit: default_submit(),
            shift_select: default_true(),
            word_jump: default_true(),
            clear_screen: default_clear_screen(),
            auto_pairs: default_true(),
            auto_indent: default_true(),
        }
    }
}

impl Default for CompletionConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            eager_hint: default_true(),
            menu_columns: default_menu_columns(),
            menu_padding: default_menu_padding(),
            quote_identifiers: default_true(),
            suggest_columns: default_true(),
            suggest_keywords: default_true(),
            max_suggestions: default_max_suggestions(),
        }
    }
}

impl Default for ExplorerConfig {
    fn default() -> Self {
        Self {
            show_columns: default_true(),
            show_row_counts: false,
            show_column_types: default_true(),
            panel_width: default_panel_width(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: default_theme(),
            mode: default_mode(),
            headers: default_true(),
            history: default_true(),
            nullvalue: default_nullvalue(),
            keybindings: KeybindingsConfig::default(),
            completion: CompletionConfig::default(),
            explorer: ExplorerConfig::default(),
        }
    }
}

impl Config {
    /// Returns the path to the configuration file
    pub fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("sqliteforge")
            .join("config.toml")
    }

    /// Load configuration from file, falling back to defaults
    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            match fs::read_to_string(&path) {
                Ok(content) => match toml::from_str(&content) {
                    Ok(config) => return config,
                    Err(e) => {
                        eprintln!("Warning: Failed to parse config: {}", e);
                    }
                },
                Err(e) => {
                    eprintln!("Warning: Failed to read config: {}", e);
                }
            }
        }
        let config = Self::default();
        // Auto-create default config if it doesn't exist
        let _ = config.save();
        config
    }

    /// Save configuration to file
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        fs::write(&path, content)?;
        Ok(())
    }
}
