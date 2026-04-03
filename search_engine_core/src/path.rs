use redb::{Error, ReadOnlyTable};
use crate::model::Node;

pub fn build_path(
    table: &ReadOnlyTable<u64, Node>,
    id: u64,
) -> Result<String, Error> {
    let mut path_parts = Vec::new();
    let mut file_id = id;
    let mut drive = 'C';

    loop {
        let entry = table.get(&file_id)?;

        let node = match entry {
            Some(v) => v.value(),
            None => break,
        };

        path_parts.push(node.name.clone());
        drive = node.drive_letter;

        if node.parent_id == 0 || node.parent_id == file_id {
            break;
        }

        file_id = node.parent_id;
    }

    path_parts.reverse();

    Ok(format!("{}:\\{}", drive, path_parts.join("\\")))
}