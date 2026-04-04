use redb::{Database, Error};
use std::sync::Arc;

use crate::db::{TABLE_MAP_FILE_ID, TABLE_MAP_FILE_NAME};
use crate::search::search_internal;
use crate::model::SearchResult;
use crate::indexer::build_index;

pub struct SearchEngine {
    db: Arc<Database>, // ✅ changed
}

impl SearchEngine {
    pub fn open(path: &str) -> Result<Self, Error> {
        let db = Database::create(path)?;

        let write_txn = db.begin_write()?;
        write_txn.open_table(TABLE_MAP_FILE_ID)?;
        write_txn.open_table(TABLE_MAP_FILE_NAME)?;
        write_txn.commit()?;
        
        Ok(Self { db: Arc::new(db) }) // ✅ wrap in Arc
    }

    pub fn search(&self, query: &str) -> Result<Vec<SearchResult>, Error> {
        search_internal(&self.db, query) // ✅ pass Arc reference
    }

    pub fn open_path(&self, path: &str) -> std::io::Result<()> {
        std::process::Command::new("explorer")
            .arg(path)
            .spawn()?;
        Ok(())
    }

    pub fn build_index(&self) -> Result<(), redb::Error> {
        build_index(self.db.clone()) // ✅ clone Arc
    }

    // 🔥 Optional but VERY useful
    pub fn db(&self) -> Arc<Database> {
        self.db.clone()
    }
}