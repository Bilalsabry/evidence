-- 003_chunks_fts_triggers.sql
-- Keep `chunks_fts` in sync with `chunks` automatically.
-- FTS5 content-linked tables require the application to mirror writes; the
-- canonical pattern is three triggers (insert/delete/update). This makes
-- every `chunks` write pay a small FTS-index write, which is the right
-- default for our workload (offline ingest, then many BM25 reads). A bulk
-- "rebuild-on-ingest" path is a possible follow-up if ingest throughput
-- becomes a bottleneck.

CREATE TRIGGER chunks_ai AFTER INSERT ON chunks BEGIN
    INSERT INTO chunks_fts(rowid, text) VALUES (new.id, new.text);
END;

CREATE TRIGGER chunks_ad AFTER DELETE ON chunks BEGIN
    INSERT INTO chunks_fts(chunks_fts, rowid, text) VALUES('delete', old.id, old.text);
END;

CREATE TRIGGER chunks_au AFTER UPDATE ON chunks BEGIN
    INSERT INTO chunks_fts(chunks_fts, rowid, text) VALUES('delete', old.id, old.text);
    INSERT INTO chunks_fts(rowid, text) VALUES (new.id, new.text);
END;
