use rusqlite::{Connection, Result, params};
use std::sync::{Arc, Mutex};

use crate::model::SearchResult;

#[derive(Clone, Debug)]
pub struct SearchEngine {
    conn: Arc<Mutex<Connection>>,
}

impl SearchEngine {
    pub fn open(path: &str) -> Result<Self> {
        println!("[ENGINE] Database opened at '{}'", path);

        let conn = Connection::open(path)?;

        // 🔥 WAL mode (returns a row → must use query_row)
        conn.query_row("PRAGMA journal_mode=WAL;", [], |_| Ok(()))?;

        // 🔥 no result → execute is fine
        conn.execute("PRAGMA synchronous=NORMAL;", [])?;

        // Schema
        conn.execute_batch(
            r#"
        CREATE TABLE IF NOT EXISTS files (
            id INTEGER PRIMARY KEY,
            path TEXT NOT NULL UNIQUE,
            name TEXT NOT NULL,
            is_directory INTEGER NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_name ON files(name);
        CREATE INDEX IF NOT EXISTS idx_path ON files(path);
        "#,
        )?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    // 🔍 SEARCH
    pub fn search(&self, query: &str) -> Result<Vec<SearchResult>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT path, name, is_directory 
             FROM files 
             WHERE name LIKE ?1 
             LIMIT 10",
        )?;

        let query = format!("{}%", query.to_lowercase());

        let rows = stmt.query_map([query], |row| {
            Ok(SearchResult {
                path: row.get(0)?,
                file_name: row.get(1)?,
                is_directory: row.get::<_, i32>(2)? != 0,
            })
        })?;

        let mut results = Vec::new();
        for r in rows {
            results.push(r?);
        }

        Ok(results)
    }

    // 🔥 INSERT / UPDATE
    pub fn index_path(&self, path: &str) -> Result<()> {
        use std::path::Path;

        let p = Path::new(path);

        let name = match p.file_name() {
            Some(n) => n.to_string_lossy().to_string(),
            None => return Ok(()),
        };

        let is_directory = p.is_dir();

        let conn = self.conn.lock().unwrap();

        conn.execute(
            r#"
            INSERT INTO files (path, name, is_directory)
            VALUES (?1, ?2, ?3)
            ON CONFLICT(path) DO UPDATE SET
                name=excluded.name,
                is_directory=excluded.is_directory
            "#,
            params![path, name.to_lowercase(), is_directory as i32],
        )?;

        Ok(())
    }

    // ❌ REMOVE
    pub fn remove_path(&self, path: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute("DELETE FROM files WHERE path = ?1", params![path])?;

        Ok(())
    }

    // 🔥 FULL SCAN (initial build)
    pub fn build_index(&self) -> Result<()> {
        use walkdir::WalkDir;

        println!("[INDEX] Full scan started...");

        let conn = self.conn.lock().unwrap();
        let tx = conn.unchecked_transaction()?; // faster bulk insert

        for entry in WalkDir::new("C:\\Users").into_iter().filter_map(|e| e.ok()) {
            let path = entry.path().to_string_lossy().to_string();

            let name = entry
                .file_name()
                .to_string_lossy()
                .to_string()
                .to_lowercase();

            let is_directory = entry.file_type().is_dir();

            let _ = tx.execute(
                "INSERT OR IGNORE INTO files (path, name, is_directory) VALUES (?1, ?2, ?3)",
                params![path, name, is_directory as i32],
            );
        }

        tx.commit()?;

        println!("[INDEX] Done");

        Ok(())
    }

    // 🔥 OPEN FILE
    pub fn open_path(&self, path: &str) -> std::io::Result<()> {
        std::process::Command::new("explorer").arg(path).spawn()?;
        Ok(())
    }
}
