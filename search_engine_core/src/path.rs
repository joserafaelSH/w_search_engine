use std::collections::HashMap;
use redb::ReadableTable;
use redb::{Error, ReadOnlyTable};
use crate::model::Node;

pub fn build_full_path(
    id: u64,
    cache: &mut HashMap<u64, String>,
    table: &redb::ReadOnlyTable<u64, Node>,
) -> Result<String, Error> {
    // ✅ already computed
    if let Some(path) = cache.get(&id) {
        return Ok(path.clone());
    }

    let node_guard = table.get(&id)?;
    let node = match node_guard {
        Some(n) => n.value(),
        None => return Ok(String::new()), // or error
    };

    let path = if node.parent_id == 0 {
        // root
        format!("{}:\\{}", node.drive_letter, node.name)
    } else {
        let parent_path = build_full_path(node.parent_id, cache, table)?;

        if parent_path.ends_with('\\') {
            format!("{}{}", parent_path, node.name)
        } else {
            format!("{}\\{}", parent_path, node.name)
        }
    };

    cache.insert(id, path.clone());
    Ok(path)
}