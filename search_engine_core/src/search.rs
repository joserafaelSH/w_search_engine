use redb::{Database, Error, ReadableDatabase};
use std::collections::HashSet;

use crate::db::{TABLE_MAP_FILE_ID, TABLE_MAP_FILE_NAME};
use crate::model::SearchResult;
use crate::path::build_path;

pub fn search_internal(
    db: &Database,
    name: &str,
) -> Result<Vec<SearchResult>, Error> {
    let mut output = HashSet::new();

    let read_txn = db.begin_read()?;

    let table_name = read_txn.open_table(TABLE_MAP_FILE_NAME)?;
    let table_id = read_txn.open_table(TABLE_MAP_FILE_ID)?;

    let name = name.to_ascii_lowercase();

    let start = (name.clone(), 0);
    let end = (name.clone() + "\u{FFFF}", u64::MAX);

    let range = table_name.range(start..=end)?;

    for entry in range.take(100) {
        let (key_guard, _) = entry?;
        let (file_name, file_id) = key_guard.value();

        if let Ok(path) = build_path(&table_id, file_id) {
            if !path.is_empty() {
                let ignored = ["\\target\\", "\\.git\\", "\\windows\\prefetch\\"];

                if ignored.iter().any(|p| path.contains(p)) {
                    continue;
                }

                output.insert(SearchResult {
                    path,
                    file_name: file_name.clone(),
                    is_directory: false, // can improve later
                });
            }
        }
    }

    Ok(output.into_iter().collect())
}