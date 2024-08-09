// MIT License
// Copyright (c) 2024 Graham King

// We don't enforce a unique URL because draft articles may not have decided on the slug yet
pub const CREATE_ARTICLE_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS article (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,
    url TEXT NOT NULL,
    date DATETIME NULL,
    filename TEXT NOT NULL,
    is_draft BOOL NOT NULL,
    UNIQUE (filename)
)
"#;

pub const CREATE_CHUNK_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS article_chunk (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    article_id INTEGER NOT NULL,
    chunk_id INTEGER NOT NULL,
    text TEXT NOT NULL,
    embed BLOB NULL,
    FOREIGN KEY (article_id) REFERENCES article (id),
    UNIQUE (article_id, chunk_id)
)
"#;

pub const CREATE_SIMILARITY_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS article_similiarity (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    article_a INTEGER NOT NULL,
    article_b INTEGER NOT NULL,
    similarity REAL NOT NULL,
    FOREIGN KEY (article_a) REFERENCES article (id),
    FOREIGN KEY (article_b) REFERENCES article (id),
    UNIQUE(article_a, article_b)
)
"#;
