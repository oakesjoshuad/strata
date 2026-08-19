CREATE TABLE engineering_record (
    id TEXT PRIMARY KEY,
    kind TEXT NOT NULL,
    number INTEGER NOT NULL,
    title TEXT NOT NULL,
    status TEXT NOT NULL,
    document TEXT NOT NULL CHECK(json_valid(document)),
    revision INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(kind, number)
);

CREATE TABLE record_relation (
    source_id TEXT NOT NULL REFERENCES engineering_record(id),
    relation TEXT NOT NULL,
    target_id TEXT NOT NULL REFERENCES engineering_record(id),
    PRIMARY KEY(source_id, relation, target_id)
);

CREATE TABLE record_revision (
    record_id TEXT NOT NULL REFERENCES engineering_record(id),
    revision INTEGER NOT NULL,
    document TEXT NOT NULL CHECK(json_valid(document)),
    changed_at TEXT NOT NULL,
    changed_by TEXT,
    change_summary TEXT,
    PRIMARY KEY(record_id, revision)
);

CREATE TABLE evidence (
    id TEXT PRIMARY KEY,
    record_id TEXT NOT NULL REFERENCES engineering_record(id),
    kind TEXT NOT NULL,
    title TEXT NOT NULL,
    uri TEXT,
    content TEXT,
    metadata TEXT CHECK(metadata IS NULL OR json_valid(metadata))
);

CREATE TABLE code_reference (
    record_id TEXT NOT NULL REFERENCES engineering_record(id),
    relation TEXT NOT NULL,
    path TEXT NOT NULL,
    symbol TEXT,
    line_start INTEGER,
    line_end INTEGER
);

CREATE VIRTUAL TABLE engineering_record_fts USING fts5(
    record_id UNINDEXED,
    title,
    body,
    tags,
    tokenize='porter unicode61'
);
