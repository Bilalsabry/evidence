import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open as openFileDialog } from "@tauri-apps/plugin-dialog";

type DocumentInfo = {
  id: number;
  sha256: string;
  title: string | null;
  page_count: number;
  ingested_at: number;
};

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

  // Smoke ping + initial library load.
  useEffect(() => {
    invoke<string>("app_version")
      .then(setVersion)
      .catch((e) => setIngestError(String(e)));
    refreshLibrary();
  }, []);

  // Listen for ingest progress events for the lifetime of the component.
  useEffect(() => {
    const promise = listen<IngestProgressEvent>("ingest:progress", (e) => {
      setProgress(e.payload);
    });
    return () => {
      promise.then((unlisten) => unlisten());
    };
  }, []);

  const refreshLibrary = useCallback(() => {
    invoke<DocumentInfo[]>("list_documents")
      .then(setDocuments)
      .catch((e) => setIngestError(String(e)));
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
        <h1>evidence</h1>
        <p className="tagline">
          {version ? `core v${version} — auditable AI for high-stakes work` : "loading…"}
        </p>
      </header>

      <section className="actions">
        <button onClick={pickAndIngest} disabled={busy} className="primary">
          {busy ? "Ingesting…" : "Ingest a PDF"}
        </button>
        {progress && (
          <ProgressBar progress={progress} />
        )}
        {ingestError && (
          <p className="err" role="alert">
            {ingestError}
          </p>
        )}
      </section>

      <section className="library">
        <h2>Library</h2>
        {documents.length === 0 ? (
          <p className="empty">Nothing here yet. Ingest a PDF to get started.</p>
        ) : (
          <table>
            <thead>
              <tr>
                <th>Title</th>
                <th>Pages</th>
                <th>Ingested</th>
              </tr>
            </thead>
            <tbody>
              {documents.map((d) => (
                <tr key={d.id}>
                  <td>{d.title ?? d.sha256.slice(0, 12)}</td>
                  <td>{d.page_count}</td>
                  <td>{new Date(d.ingested_at).toLocaleString()}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>

      <footer className="hint">
        PDF viewer + chat panel land in #19 and #20.
      </footer>
    </main>
  );
}

function ProgressBar({ progress }: { progress: IngestProgressEvent }) {
  const pct =
    progress.total === 0 ? 0 : Math.round((progress.current / progress.total) * 100);
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
