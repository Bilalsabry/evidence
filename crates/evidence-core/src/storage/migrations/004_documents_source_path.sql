-- 004_documents_source_path.sql
-- Record where each document was ingested from, so the desktop PDF
-- viewer can re-read the original bytes (issue #19). Nullable: rows
-- ingested before this migration have no recorded path and the viewer
-- treats them as "source unavailable" rather than failing.

ALTER TABLE documents ADD COLUMN source_path TEXT;
