CREATE TABLE status_transition (
    record_id TEXT NOT NULL,
    revision INTEGER NOT NULL,
    from_status TEXT NOT NULL,
    to_status TEXT NOT NULL,
    transition_kind TEXT NOT NULL CHECK(transition_kind IN ('forward', 'undo')),
    PRIMARY KEY(record_id, revision),
    FOREIGN KEY(record_id, revision) REFERENCES record_revision(record_id, revision)
);

