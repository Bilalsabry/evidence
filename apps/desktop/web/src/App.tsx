import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
import { Panel, PanelGroup, PanelResizeHandle } from "react-resizable-panels";
import { PdfViewer, type PdfViewerHandle } from "./PdfViewer";
import { ChatPanel } from "./ChatPanel";
import type { DocumentInfo } from "./types";

type IngestProgressEvent = {
  ingest_id: number;
  stage: "hashing" | "parsing" | "persisting" | "embedding" | "done";
  current: number;
  total: number;
};

const STAGE_LABEL: Record<IngestProgressEvent["stage"], string> = {
  hashing: "Hashing",
  parsing: "Parsing PDF",
  persisting: "Writing to index",
  embedding: "Embedding chunks",
  done: "Done",
};

export function App() {
  const [version, setVersion] = useState<string | null>(null);
  const [documents, setDocuments] = useState<DocumentInfo[]>([]);
  const [progress, setProgress] = useState<IngestProgressEvent | null>(null);
  const [ingestError, setIngestError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [openDocId, setOpenDocId] = useState<number | null>(null);
  const viewerRef = useRef<PdfViewerHandle | null>(null);

  const openInViewer = useCallback((docId: number) => {
    setOpenDocId(docId);
    void viewerRef.current?.openDocument(docId);
  }, []);

  // A citation chip was clicked: jump the viewer to that span. The
  // viewer switches documents on its own if the span lives elsewhere.
  const onCiteClick = useCallback((spanId: number) => {
    void viewerRef.current?.highlightSpan(spanId);
  }, []);

  const refreshLibrary = useCallback(() => {
    invoke<DocumentInfo[]>("list_documents")
      .then(setDocuments)
      .catch((e) => setIngestError(String(e)));
  }, []);

  // Smoke ping + initial library load.
  useEffect(() => {
    invoke<string>("app_version")
      .then(setVersion)
      .catch((e) => setIngestError(String(e)));
    refreshLibrary();
  }, [refreshLibrary]);

  // Listen for ingest progress events for the lifetime of the component.
  useEffect(() => {
    const promise = listen<IngestProgressEvent>("ingest:progress", (e) => {
      setProgress(e.payload);
    });
    return () => {
      promise.then((unlisten) => unlisten());
    };
  }, []);

  const pickAndIngest = useCallback(async () => {
    setIngestError(null);
    const selected = await openFileDialog({
      multiple: false,
      filters: [{ name: "PDF", extensions: ["pdf"] }],
    });
    if (!selected || typeof selected !== "string") return;

    setBusy(true);
    setProgress(null);
    try {
      await invoke<{ ingest_id: number }>("ingest_with_progress", {
        path: selected,
      });
      refreshLibrary();
    } catch (e) {
      const msg = String(e);
      // Duplicate is a known case; render it inline rather than as a hard error.
      if (msg.includes("already ingested")) {
        setIngestError(
          "This file is already in your library — open it from the list.",
        );
      } else {
        setIngestError(msg);
      }
    } finally {
      setBusy(false);
      setProgress(null);
    }
  }, [refreshLibrary]);

  return (
    <main className="app-shell">
      <header className="app-header">
        <div>
          <h1>evidence</h1>
          <p className="tagline">
            {version
              ? `core v${version} — auditable AI for high-stakes work`
              : "loading…"}
          </p>
        </div>
        <div className="header-actions">
          <button onClick={pickAndIngest} disabled={busy} className="primary">
            {busy ? "Ingesting…" : "Ingest a PDF"}
          </button>
        </div>
      </header>

      {(progress || ingestError) && (
        <section className="actions">
          {progress && <ProgressBar progress={progress} />}
          {ingestError && (
            <p className="err" role="alert">
              {ingestError}
            </p>
          )}
        </section>
      )}

      <PanelGroup direction="horizontal" className="three-pane">
        <Panel defaultSize={22} minSize={14} className="pane library-pane">
          <h2>Library</h2>
          {documents.length === 0 ? (
            <p className="empty">
              Nothing here yet. Ingest a PDF to get started.
            </p>
          ) : (
            <ul className="doc-list">
              {documents.map((d) => (
                <li key={d.id}>
                  <button
                    className={
                      d.id === openDocId ? "doc-item open" : "doc-item"
                    }
                    onClick={() => openInViewer(d.id)}
                    aria-label={`Open ${
                      d.title ?? d.sha256.slice(0, 12)
                    } in viewer`}
                  >
                    <span className="doc-title">
                      {d.title ?? d.sha256.slice(0, 12)}
                    </span>
                    <span className="doc-meta">{d.page_count} pp</span>
                  </button>
                </li>
              ))}
            </ul>
          )}
        </Panel>

        <PanelResizeHandle className="resize-handle" />

        <Panel defaultSize={50} minSize={25} className="pane viewer-pane">
          <PdfViewer ref={viewerRef} />
        </Panel>

        <PanelResizeHandle className="resize-handle" />

        <Panel defaultSize={28} minSize={18} className="pane chat-pane">
          <ChatPanel onCiteClick={onCiteClick} />
        </Panel>
      </PanelGroup>
    </main>
  );
}

function ProgressBar({ progress }: { progress: IngestProgressEvent }) {
  const pct =
    progress.total === 0
      ? 0
      : Math.round((progress.current / progress.total) * 100);
  return (
    <div className="progress" role="progressbar" aria-valuenow={pct}>
      <span className="progress-label">
        {STAGE_LABEL[progress.stage]} — {progress.current}/{progress.total}
      </span>
      <div className="progress-track">
        <div className="progress-fill" style={{ width: `${pct}%` }} />
      </div>
    </div>
  );
}
