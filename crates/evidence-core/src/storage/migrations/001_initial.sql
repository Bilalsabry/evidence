-- 001_initial.sql
-- Base schema for evidence: documents, pages, spans, chunks. Indexes here
-- support the foreign-key lookups; full-text and vector indexes come in 002.

CREATE TABLE documents (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    sha256      TEXT    NOT NULL UNIQUE,
    title       TEXT,
    page_count  INTEGER NOT NULL CHECK (page_count >= 0),
    ingested_at INTEGER NOT NULL -- Unix epoch milliseconds
) STRICT;

CREATE TABLE pages (
    id        INTEGER PRIMARY KEY AUTOINCREMENT,
    doc_id    INTEGER NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    page_num  INTEGER NOT NULL CHECK (page_num >= 1),
    raw_text  TEXT    NOT NULL,
    UNIQUE (doc_id, page_num)
) STRICT;

CREATE INDEX idx_pages_doc ON pages(doc_id);

CREATE TABLE spans (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    page_id      INTEGER NOT NULL REFERENCES pages(id) ON DELETE CASCADE,
    start_offset INTEGER NOT NULL CHECK (start_offset >= 0),
    end_offset   INTEGER NOT NULL CHECK (end_offset > start_offset),
    text         TEXT    NOT NULL,
    bbox_json    TEXT    NOT NULL -- JSON: {"x0":..,"y0":..,"x1":..,"y1":..}
) STRICT;

CREATE INDEX idx_spans_page ON spans(page_id);

CREATE TABLE chunks (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    span_id_start INTEGER NOT NULL REFERENCES spans(id),
    span_id_end   INTEGER NOT NULL REFERENCES spans(id),
    text          TEXT    NOT NULL,
    CHECK (span_id_end >= span_id_start)
) STRICT;

CREATE INDEX idx_chunks_span_range ON chunks(span_id_start, span_id_end);
