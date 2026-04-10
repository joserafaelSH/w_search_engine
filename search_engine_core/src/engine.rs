use crate::indexer::build_index;
use crate::search::search_internal;
use rusqlite::params;
use rusqlite::{Connection, Result};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct SearchEngine {
    conn: Arc<Mutex<Connection>>,
}

impl SearchEngine {
    pub fn open(path: &str) -> Result<Self> {
        println!("[ENGINE] Opening DB: {}", path);

        let conn = Connection::open(path)?;

        // WAL for concurrency
        conn.query_row("PRAGMA journal_mode=WAL;", [], |_| Ok(()))?;
        conn.execute("PRAGMA synchronous=NORMAL;", [])?;

        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS files (
                id INTEGER PRIMARY KEY,
                parent_id INTEGER,
                name TEXT,
                drive_letter TEXT,
                is_directory INTEGER
            );

            CREATE INDEX IF NOT EXISTS idx_name ON files(name);
            CREATE INDEX IF NOT EXISTS idx_parent ON files(parent_id);
            "#,
        )?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn build_index(&self) -> Result<()> {
        let mut conn = self.conn.lock().unwrap();
        build_index(&mut conn)
    }

    pub fn search(&self, query: &str) -> Result<Vec<crate::model::SearchResult>> {
        let conn = self.conn.lock().unwrap();
        search_internal(&conn, query)
    }

    pub fn index_path(&self, path: &str) -> Result<()> {
        let name = match std::path::Path::new(path).file_name() {
            Some(n) => n.to_string_lossy().to_string(),
            None => return Ok(()),
        };

        if name.is_empty() || name == "." || name == ".." {
            return Ok(());
        }

        let is_directory = std::path::Path::new(path).is_dir();

        let conn = self.conn.lock().unwrap();

        conn.execute(
            "INSERT OR REPLACE INTO files (path, name, is_directory)
         VALUES (?1, ?2, ?3)",
            params![path, name.to_ascii_lowercase(), is_directory as i32],
        )?;

        println!("[INDEX] {}", path);

        Ok(())
    }

    pub fn remove_path(&self, path: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute("DELETE FROM files WHERE path = ?1", rusqlite::params![path])?;

        println!("[REMOVE] {}", path);

        Ok(())
    }
}
