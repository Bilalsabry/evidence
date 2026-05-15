-- 002_chunks_indexes.sql
-- Retrieval indexes over `chunks`: FTS5 for BM25, vec0 for ANN search.
-- The FTS table is content-linked so it doesn't duplicate `chunks.text` on
-- disk; callers must explicitly mirror inserts/updates/deletes (or install
-- the triggers added in issue #3).
-- The vector table is fixed at 384 dimensions to match bge-small-en-v1.5
-- (issue #4); a future migration will lift this when the embedder is
-- configurable.

CREATE VIRTUAL TABLE chunks_fts USING fts5(
    text,
    content='chunks',
    content_rowid='id',
    tokenize='unicode61'
);

CREATE VIRTUAL TABLE chunks_vec USING vec0(
    embedding float[384]
);
