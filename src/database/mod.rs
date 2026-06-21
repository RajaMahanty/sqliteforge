use rusqlite::{Connection, Result as SqliteResult, types::Value};
use std::path::Path;
use std::time::Instant;

/// Result of executing a SQL query
#[derive(Debug)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub rows_affected: usize,
    pub execution_time_ms: f64,
    pub is_select: bool,
}

/// Database connection wrapper
pub struct Database {
    conn: Connection,
    pub path: String,
    pub nullvalue: String,
}

impl Database {
    /// Open a database file (creates if it doesn't exist)
    pub fn open<P: AsRef<Path>>(path: P) -> SqliteResult<Self> {
        let path_str = path.as_ref().to_string_lossy().to_string();
        let conn = Connection::open(&path)?;
        // Enable WAL mode for better concurrent access
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        Ok(Self {
            conn,
            path: path_str,
            nullvalue: String::new(),
        })
    }

    /// Open an in-memory database
    pub fn open_in_memory() -> SqliteResult<Self> {
        let conn = Connection::open_in_memory()?;
        Ok(Self {
            conn,
            path: ":memory:".to_string(),
            nullvalue: String::new(),
        })
    }

    /// Execute a SQL query and return results
    pub fn execute_query(&self, sql: &str) -> Result<QueryResult, String> {
        let trimmed = sql.trim();
        if trimmed.is_empty() {
            return Err("Empty query".to_string());
        }

        let start = Instant::now();

        // Determine if it's a SELECT-like query (returns rows)
        let upper = trimmed.to_uppercase();
        let is_select = upper.starts_with("SELECT")
            || upper.starts_with("PRAGMA")
            || upper.starts_with("EXPLAIN")
            || upper.starts_with("WITH");

        if is_select {
            self.execute_select(trimmed, start)
        } else {
            self.execute_modify(trimmed, start)
        }
    }

    /// Execute a SELECT query
    fn execute_select(&self, sql: &str, start: Instant) -> Result<QueryResult, String> {
        let mut stmt = self.conn.prepare(sql).map_err(|e| e.to_string())?;

        let columns: Vec<String> = stmt
            .column_names()
            .iter()
            .map(|s| s.to_string())
            .collect();

        let rows: Vec<Vec<String>> = stmt
            .query_map([], |row| {
                let mut values = Vec::new();
                for i in 0..columns.len() {
                    let val: Value = row.get_unwrap(i);
                    let s = match val {
                        Value::Null => self.nullvalue.clone(),
                        Value::Integer(i) => i.to_string(),
                        Value::Real(f) => f.to_string(),
                        Value::Text(s) => s,
                        Value::Blob(b) => format!("X'{}'", hex_encode(&b)),
                    };
                    values.push(s);
                }
                Ok(values)
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();

        let elapsed = start.elapsed();
        Ok(QueryResult {
            rows_affected: rows.len(),
            columns,
            rows,
            execution_time_ms: elapsed.as_secs_f64() * 1000.0,
            is_select: true,
        })
    }

    /// Execute a non-SELECT query (INSERT, UPDATE, DELETE, etc.)
    fn execute_modify(&self, sql: &str, start: Instant) -> Result<QueryResult, String> {
        let rows_affected = self
            .conn
            .execute_batch(sql)
            .map_err(|e| e.to_string())
            .map(|_| self.conn.changes())?;

        let elapsed = start.elapsed();
        Ok(QueryResult {
            columns: Vec::new(),
            rows: Vec::new(),
            rows_affected: rows_affected as usize,
            execution_time_ms: elapsed.as_secs_f64() * 1000.0,
            is_select: false,
        })
    }

    /// Get all table names
    pub fn get_tables(&self) -> Vec<String> {
        let sql = "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name";
        self.conn
            .prepare(sql)
            .ok()
            .map(|mut stmt| {
                stmt.query_map([], |row| row.get(0))
                    .ok()
                    .map(|rows| rows.filter_map(|r| r.ok()).collect())
                    .unwrap_or_default()
            })
            .unwrap_or_default()
    }

    /// Get all view names
    pub fn get_views(&self) -> Vec<String> {
        let sql = "SELECT name FROM sqlite_master WHERE type='view' ORDER BY name";
        self.conn
            .prepare(sql)
            .ok()
            .map(|mut stmt| {
                stmt.query_map([], |row| row.get(0))
                    .ok()
                    .map(|rows| rows.filter_map(|r| r.ok()).collect())
                    .unwrap_or_default()
            })
            .unwrap_or_default()
    }

    /// Get all index names
    pub fn get_indices(&self) -> Vec<String> {
        let sql = "SELECT name FROM sqlite_master WHERE type='index' AND name NOT LIKE 'sqlite_%' ORDER BY name";
        self.conn
            .prepare(sql)
            .ok()
            .map(|mut stmt| {
                stmt.query_map([], |row| row.get(0))
                    .ok()
                    .map(|rows| rows.filter_map(|r| r.ok()).collect())
                    .unwrap_or_default()
            })
            .unwrap_or_default()
    }

    /// Get columns for a table
    pub fn get_columns(&self, table: &str) -> Vec<String> {
        let sql = format!("PRAGMA table_info(\"{}\")", table.replace('"', "\"\""));
        self.conn
            .prepare(&sql)
            .ok()
            .map(|mut stmt| {
                stmt.query_map([], |row| row.get::<_, String>(1))
                    .ok()
                    .map(|rows| rows.filter_map(|r| r.ok()).collect())
                    .unwrap_or_default()
            })
            .unwrap_or_default()
    }

    /// Get columns with their types for a table (name, type)
    pub fn get_column_info(&self, table: &str) -> Vec<(String, String)> {
        let sql = format!("PRAGMA table_info(\"{}\")", table.replace('"', "\"\""));
        self.conn
            .prepare(&sql)
            .ok()
            .map(|mut stmt| {
                stmt.query_map([], |row| {
                    let name: String = row.get(1)?;
                    let col_type: String = row.get(2)?;
                    Ok((name, col_type))
                })
                .ok()
                .map(|rows| rows.filter_map(|r| r.ok()).collect())
                .unwrap_or_default()
            })
            .unwrap_or_default()
    }

    /// Get the row count for a table
    pub fn get_row_count(&self, table: &str) -> Option<usize> {
        let sql = format!("SELECT COUNT(*) FROM \"{}\"", table.replace('"', "\"\""));
        self.conn
            .query_row(&sql, [], |row| row.get::<_, i64>(0))
            .ok()
            .map(|c| c as usize)
    }

    /// Get the CREATE statement for a database object
    pub fn get_schema(&self, name: &str) -> Option<String> {
        let sql = "SELECT sql FROM sqlite_master WHERE name = ?1";
        self.conn
            .query_row(sql, [name], |row| row.get(0))
            .ok()
    }

    /// Get all schema CREATE statements
    pub fn get_all_schemas(&self) -> Vec<String> {
        let sql = "SELECT sql FROM sqlite_master WHERE sql IS NOT NULL ORDER BY type, name";
        self.conn
            .prepare(sql)
            .ok()
            .map(|mut stmt| {
                stmt.query_map([], |row| row.get(0))
                    .ok()
                    .map(|rows| rows.filter_map(|r| r.ok()).collect())
                    .unwrap_or_default()
            })
            .unwrap_or_default()
    }

    /// Get the full dump of the database (schema + data)
    pub fn dump(&self) -> Result<String, String> {
        let mut output = String::new();
        output.push_str("BEGIN TRANSACTION;\n");

        // Schemas
        for schema in self.get_all_schemas() {
            output.push_str(&schema);
            output.push_str(";\n");
        }

        // Data for each table
        for table in self.get_tables() {
            let sql = format!("SELECT * FROM \"{}\"", table.replace('"', "\"\""));
            if let Ok(result) = self.execute_query(&sql) {
                for row in &result.rows {
                    let values: Vec<String> = row
                        .iter()
                        .map(|v| {
                            if v.is_empty() {
                                "NULL".to_string()
                            } else if v.parse::<f64>().is_ok() {
                                v.clone()
                            } else {
                                format!("'{}'", v.replace('\'', "''"))
                            }
                        })
                        .collect();
                    output.push_str(&format!(
                        "INSERT INTO \"{}\" VALUES({});\n",
                        table.replace('"', "\"\""),
                        values.join(",")
                    ));
                }
            }
        }

        output.push_str("COMMIT;\n");
        Ok(output)
    }

    /// Execute a SQL script file
    pub fn execute_script(&self, sql: &str) -> Result<(), String> {
        self.conn.execute_batch(sql).map_err(|e| e.to_string())
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02X}", b)).collect()
}
