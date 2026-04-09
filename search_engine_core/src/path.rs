use crate::model::Node;
use rusqlite::{Connection, Result};
use std::collections::HashMap;

pub fn build_full_path(
    conn: &Connection,
    id: u64,
    cache: &mut HashMap<u64, String>,
) -> Result<String> {
    // ✅ cache hit
    if let Some(path) = cache.get(&id) {
        return Ok(path.clone());
    }

    // 🔥 fetch node from SQLite
    let mut stmt = conn
        .prepare("SELECT parent_id, name, drive_letter, is_directory FROM files WHERE id = ?1")?;

    let mut rows = stmt.query([id as i64])?;

    let node = if let Some(row) = rows.next()? {
        Node {
            parent_id: row.get::<_, i64>(0)? as u64,
            name: row.get(1)?,
            drive_letter: {
                let s: String = row.get(2)?;
                s.chars().next().unwrap_or('C')
            },
            is_directory: row.get::<_, i64>(3)? != 0,
        }
    } else {
        return Ok(String::new());
    };

    // 🔥 build path recursively
    let path = if node.parent_id == 0 || node.parent_id == id {
        format!("{}:\\{}", node.drive_letter, node.name)
    } else {
        let parent_path = build_full_path(conn, node.parent_id, cache)?;

        if parent_path.ends_with('\\') {
            format!("{}{}", parent_path, node.name)
        } else {
            format!("{}\\{}", parent_path, node.name)
        }
    };

    cache.insert(id, path.clone());

    Ok(path)
}
