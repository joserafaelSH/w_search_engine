use redb::TableDefinition;
use crate::model::Node;

pub const TABLE_MAP_FILE_ID: TableDefinition<u64, Node> =
    TableDefinition::new("hash_map_file_id");

pub const TABLE_MAP_FILE_NAME: TableDefinition<(String, u64), ()> =
    TableDefinition::new("hash_map_file_name");