use rusqlite::{Connection, Result};
use std::collections::HashMap;

use crate::path::build_full_path;
use crate::model::SearchResult;

pub fn search_internal(
    conn: &Connection,
    query: &str,
) -> Result<Vec<SearchResult>> {
    let mut results = Vec::with_capacity(100);

    let query = query.to_ascii_lowercase();

    // 🔥 prefix search (LIKE 'query%')
    let like_query = format!("{}%", query);

    let mut stmt = conn.prepare(
        "SELECT id, name, is_directory FROM files 
         WHERE lower(name) LIKE ?1 
         LIMIT 100"
    )?;

    let mut rows = stmt.query([like_query])?;

    // 🔥 cache for path reconstruction
    let mut path_cache: HashMap<u64, String> = HashMap::new();

    while let Some(row) = rows.next()? {
        let file_id = row.get::<_, i64>(0)? as u64;
        let file_name: String = row.get(1)?;
        let is_directory = row.get::<_, i64>(2)? != 0;

        // 🔥 build full path (recursive with cache)
        let full_path = build_full_path(conn, file_id, &mut path_cache)?;

        results.push(SearchResult {
            path: full_path,
            file_name,
            is_directory,
        });
    }

    Ok(results)
}