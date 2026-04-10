CREATE TABLE IF NOT EXISTS files (
    id INTEGER PRIMARY KEY,
    path TEXT NOT NULL,
    name TEXT NOT NULL,
    is_directory INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_name ON files(name);
CREATE INDEX IF NOT EXISTS idx_path ON files(path);