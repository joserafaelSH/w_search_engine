use redb::{Database, Error, ReadableDatabase};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use crate::path::build_full_path;
use crate::db::{TABLE_MAP_FILE_ID, TABLE_MAP_FILE_NAME};
use crate::model::SearchResult;


pub fn search_internal(
    db: &Arc<Database>,
    query: &str,
) -> Result<Vec<SearchResult>, Error> {
    let mut results = Vec::with_capacity(100);

    let read_txn = db.begin_read()?;
    let table_name = read_txn.open_table(TABLE_MAP_FILE_NAME)?;
    let table_id = read_txn.open_table(TABLE_MAP_FILE_ID)?;

    let query = query.to_ascii_lowercase();

    let start = (query.clone(), 0);
    let end = (query.clone() + "\u{FFFF}", u64::MAX);

    let range = table_name.range(start..=end)?;

    // 🔥 CACHE FOR THIS SEARCH
    let mut path_cache: HashMap<u64, String> = HashMap::new();

    for entry in range.take(100) {
        let (key_guard, value_guard) = entry?;
        let (_, file_id) = key_guard.value();
        let is_directory = value_guard.value();

        if let Some(node) = table_id.get(&file_id)? {
            let node = node.value();

            // 🔥 BUILD FULL PATH HERE
            let full_path = build_full_path(file_id, &mut path_cache, &table_id)?;

            results.push(SearchResult {
                path: full_path,
                file_name: node.name.clone(),
                is_directory,
            });
        }
    }

    Ok(results)
}