import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Answer, Citation } from "./types";

// v0.3 is one question → one answer (no conversation memory). Retrieval
// depth + support gate are fixed here; surfacing them as controls is a
// later UX pass.
const RETRIEVAL_K = 8;
const CHECK_SUPPORT = true;

const REFUSAL_COPY =
  "I can't answer that without a verifiable citation. Try rephrasing, or ingest a document that covers it.";

type Turn =
  | { kind: "question"; text: string }
  | { kind: "answer"; answer: Answer }
  | { kind: "refusal"; detail: string };

export function ChatPanel({
  onCiteClick,
}: {
  onCiteClick: (spanId: number) => void;
}) {
  const [input, setInput] = useState("");
  const [turns, setTurns] = useState<Turn[]>([]);
  const [busy, setBusy] = useState(false);
  const logRef = useRef<HTMLDivElement | null>(null);

  // Pin the transcript to the newest turn.
  useEffect(() => {
    logRef.current?.scrollTo({ top: logRef.current.scrollHeight });
  }, [turns, busy]);

  const submit = useCallback(async () => {
    const question = input.trim();
    if (!question || busy) return;
    setInput("");
    setTurns((t) => [...t, { kind: "question", text: question }]);
    setBusy(true);
    try {
      const answer = await invoke<Answer>("query", {
        question,
        k: RETRIEVAL_K,
        checkSupport: CHECK_SUPPORT,
      });
      setTurns((t) => [...t, { kind: "answer", answer }]);
    } catch (e) {
      // Every non-Ok path is a refusal from the validator's point of
      // view — the gate did its job. Show why, don't crash.
      setTurns((t) => [...t, { kind: "refusal", detail: String(e) }]);
    } finally {
      setBusy(false);
    }
  }, [input, busy]);

  const onKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
      if (e.key === "Enter" && !e.shiftKey) {
        e.preventDefault();
        void submit();
      }
    },
    [submit],
  );

  return (
    <div className="chat-panel">
      <div className="chat-log" ref={logRef}>
        {turns.length === 0 && (
          <p className="chat-empty">
            Ask a question. Answers cite the exact span they came from —
            click a chip to see it highlighted in the document.
          </p>
        )}
        {turns.map((turn, i) => (
          <ChatTurn key={i} turn={turn} onCiteClick={onCiteClick} />
        ))}
        {busy && <div className="chat-bubble pending">Thinking…</div>}
      </div>

      <div className="chat-input">
        <textarea
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={onKeyDown}
          placeholder="Ask a question…  (Enter to send, Shift+Enter for newline)"
          rows={3}
          aria-label="Question"
          disabled={busy}
        />
        <button
          className="primary"
          onClick={() => void submit()}
          disabled={busy || input.trim().length === 0}
        >
          Ask
        </button>
      </div>
    </div>
  );
}

function ChatTurn({
  turn,
  onCiteClick,
}: {
  turn: Turn;
  onCiteClick: (spanId: number) => void;
}) {
  if (turn.kind === "question") {
    return <div className="chat-bubble question">{turn.text}</div>;
  }
  if (turn.kind === "refusal") {
    return (
      <div className="chat-bubble refusal" role="alert">
        <p>{REFUSAL_COPY}</p>
        <p className="refusal-detail">{turn.detail}</p>
      </div>
    );
  }
  return (
    <div className="chat-bubble answer">
      {turn.answer.sentences.map((s, i) => (
        <p key={i} className="answer-sentence">
          {s.text}
          {s.citations.map((c) => (
            <CitationChip key={c.span_id} citation={c} onClick={onCiteClick} />
          ))}
        </p>
      ))}
    </div>
  );
}

function CitationChip({
  citation,
  onClick,
}: {
  citation: Citation;
  onClick: (spanId: number) => void;
}) {
  return (
    <button
      className="cite-chip"
      onClick={() => onClick(citation.span_id)}
      aria-label={`Citation: span ${citation.span_id} on page ${citation.page_num}. Activate to highlight it in the document.`}
      title={`Span ${citation.span_id}, page ${citation.page_num}`}
    >
      p{citation.page_num}
    </button>
  );
}
