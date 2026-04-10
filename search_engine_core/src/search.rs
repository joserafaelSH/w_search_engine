use rusqlite::{Connection, Result};
use std::collections::HashMap;

use crate::model::SearchResult;
use crate::path::build_full_path;

pub fn search_internal(
    conn: &Connection,
    query: &str,
) -> Result<Vec<SearchResult>> {
    let mut results = Vec::new();

    let query = format!("{}%", query.to_ascii_lowercase());

    let mut stmt = conn.prepare(
        "SELECT id, name, is_directory FROM files
         WHERE lower(name) LIKE ?1
         LIMIT 50"
    )?;

    let mut rows = stmt.query([query])?;

    let mut cache = HashMap::new();

    while let Some(row) = rows.next()? {
        let id = row.get::<_, i64>(0)? as u64;
        let name: String = row.get(1)?;
        let is_dir = row.get::<_, i64>(2)? != 0;

        let path = build_full_path(conn, id, &mut cache)?;
        if !is_valid_name(&name) {
            continue;
        }
        results.push(SearchResult {
            path,
            file_name: name,
            is_directory: is_dir,
        });
    }

    Ok(results)
}


fn is_valid_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }

    if name.contains("dubug") {
        return false;
    }

    // ignore current/parent
    if name == "." || name == ".." {
        return false;
    }

    // 🔥 system NTFS files
    if name.starts_with('$') {
        return false;
    }

    // 🔥 temp / noisy
    if name.starts_with('~') || name.ends_with(".tmp") {
        return false;
    }

    // 🔥 skip weird control characters
    if name.chars().any(|c| c.is_control()) {
        return false;
    }

    // 🔥 skip garbage decoding (common in broken USN reads)
    if name.contains('\u{FFFD}') {
        return false;
    }

    true
}
