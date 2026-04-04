use redb::TableDefinition;
use crate::model::Node;

// unchanged
pub const TABLE_MAP_FILE_ID: TableDefinition<u64, Node> =
    TableDefinition::new("hash_map_file_id");

// ✅ store is_directory
pub const TABLE_MAP_FILE_NAME: TableDefinition<(String, u64), bool> =
    TableDefinition::new("hash_map_file_name");