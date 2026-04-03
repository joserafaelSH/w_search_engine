use redb_derive::{Key, Value};
use redb::Value;

#[derive(Debug, Key, Value, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct Node {
    pub parent_id: u64,
    pub name: String,
    pub drive_letter: char,
    pub is_directory: bool,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct SearchResult {
    pub path: String,
    pub file_name: String,
    pub is_directory: bool,
}