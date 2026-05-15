import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

/**
 * Smoke screen for #18. Calls the Rust `app_version` command and renders
 * the result. Real UI (library + viewer + chat) lands in #19, #20, #22.
 */
export function App() {
  const [version, setVersion] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const ping = async () => {
    setLoading(true);
    setError(null);
    try {
      const v = await invoke<string>("app_version");
      setVersion(v);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  return (
    <main className="app-shell">
      <header className="app-header">
        <h1>evidence</h1>
        <p className="tagline">Auditable AI for high-stakes work.</p>
      </header>

      <section className="smoke">
        <button onClick={ping} disabled={loading} className="smoke-button">
          {loading ? "Calling…" : "Ping core"}
        </button>
        {version !== null && (
          <p className="ok" role="status">
            evidence-core v{version}
          </p>
        )}
        {error !== null && (
          <p className="err" role="alert">
            {error}
          </p>
        )}
      </section>

      <footer className="hint">
        Smoke screen. The library, PDF viewer, and chat panel ship in v0.3
        issues #19, #20, and #22.
      </footer>
    </main>
  );
}
