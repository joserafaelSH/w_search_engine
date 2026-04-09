use std::fmt;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
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
impl fmt::Display for SearchResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {}, {})", self.path, self.file_name, self.is_directory)
    }
}