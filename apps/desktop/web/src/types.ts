// Shared types mirroring the Tauri command payloads. Kept in one place
// so the viewer (#19) and the chat panel (#20) agree on shapes.

export type DocumentInfo = {
  id: number;
  sha256: string;
  title: string | null;
  page_count: number;
  ingested_at: number;
};

// Mirrors `evidence_core::ingest::pdf::Bbox` — PDF points, origin
// bottom-left.
export type Bbox = {
  x0: number;
  y0: number;
  x1: number;
  y1: number;
};

// Mirrors `evidence_core::storage::SpanLocation`.
export type SpanLocation = {
  doc_id: number;
  page_num: number;
  bbox: Bbox;
};

// Mirrors `evidence_core::query::Citation`.
export type Citation = {
  span_id: number;
  doc_id: number;
  page_num: number;
  start_offset: number;
  end_offset: number;
};

// Mirrors `evidence_core::query::{Sentence, Answer}`.
export type Sentence = {
  text: string;
  citations: Citation[];
};

export type Answer = {
  sentences: Sentence[];
};
