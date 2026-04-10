use rusqlite::{Connection, Result};
use std::collections::HashMap;

pub fn build_full_path(
    conn: &Connection,
    id: u64,
    cache: &mut HashMap<u64, String>,
) -> Result<String> {
    if let Some(p) = cache.get(&id) {
        return Ok(p.clone());
    }

    let mut stmt = conn.prepare(
        "SELECT parent_id, name, drive_letter FROM files WHERE id = ?1"
    )?;

    let mut rows = stmt.query([id as i64])?;

    if let Some(row) = rows.next()? {
        let parent_id = row.get::<_, i64>(0)? as u64;
        let name: String = row.get(1)?;
        let drive: String = row.get(2)?;

        let path = if parent_id == 0 || parent_id == id {
            format!("{}:\\{}", drive, name)
        } else {
            let parent = build_full_path(conn, parent_id, cache)?;
            format!("{}\\{}", parent, name)
        };

        cache.insert(id, path.clone());
        Ok(path)
    } else {
        Ok(String::new())
    }
}