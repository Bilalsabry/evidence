import {
  forwardRef,
  useCallback,
  useEffect,
  useImperativeHandle,
  useRef,
  useState,
} from "react";
import { invoke } from "@tauri-apps/api/core";
import * as pdfjsLib from "pdfjs-dist";
// Vite resolves `?url` to the emitted worker asset. pdf.js runs parsing
// off the main thread; without this it falls back to a (deprecated)
// main-thread path and warns.
import PdfWorkerUrl from "pdfjs-dist/build/pdf.worker.min.mjs?url";
import type { PDFDocumentProxy, PageViewport } from "pdfjs-dist";
import type { Bbox, SpanLocation } from "./types";

pdfjsLib.GlobalWorkerOptions.workerSrc = PdfWorkerUrl;

// One render scale for v0.3. Crisp enough for drug labels; a zoom
// control is a follow-up. pdf.js renders at devicePixelRatio internally
// via the canvas transform below.
const RENDER_SCALE = 1.3;

/** Imperative handle the chat panel (#20) drives. */
export type PdfViewerHandle = {
  /** Load a document into the viewer (no-op if already current). */
  openDocument: (docId: number) => Promise<void>;
  /**
   * Scroll to and highlight a cited span. Switches documents first if
   * the span lives in a different one.
   */
  highlightSpan: (spanId: number) => Promise<void>;
};

type PageDim = { width: number; height: number };

type Highlight = {
  pageNum: number;
  left: number;
  top: number;
  width: number;
  height: number;
};

/**
 * Convert a span bbox (PDF points, origin bottom-left) to a CSS rect in
 * the rendered page's pixel space (origin top-left).
 *
 * pdf.js's `viewport.convertToViewportRectangle` does the PDF→viewport
 * transform for us, including the y-axis flip and any page rotation. It
 * returns `[xa, ya, xb, yb]` whose corners may be in either order
 * depending on rotation, so we normalize to left/top/width/height.
 */
function bboxToRect(viewport: PageViewport, bbox: Bbox): Omit<Highlight, "pageNum"> {
  const [xa, ya, xb, yb] = viewport.convertToViewportRectangle([
    bbox.x0,
    bbox.y0,
    bbox.x1,
    bbox.y1,
  ]);
  return {
    left: Math.min(xa, xb),
    top: Math.min(ya, yb),
    width: Math.abs(xb - xa),
    height: Math.abs(yb - ya),
  };
}

export const PdfViewer = forwardRef<PdfViewerHandle, { className?: string }>(
  function PdfViewer({ className }, ref) {
    const scrollRef = useRef<HTMLDivElement | null>(null);
    const pageElsRef = useRef<Map<number, HTMLDivElement>>(new Map());
    const pdfRef = useRef<PDFDocumentProxy | null>(null);
    const renderedRef = useRef<Set<number>>(new Set());

    const [docId, setDocId] = useState<number | null>(null);
    const [pageDims, setPageDims] = useState<PageDim[]>([]);
    const [highlight, setHighlight] = useState<Highlight | null>(null);
    const [error, setError] = useState<string | null>(null);
    const [loading, setLoading] = useState(false);

    // Render one page's canvas into its placeholder, once.
    const renderPage = useCallback(async (pageNum: number) => {
      const pdf = pdfRef.current;
      const host = pageElsRef.current.get(pageNum);
      if (!pdf || !host || renderedRef.current.has(pageNum)) return;
      renderedRef.current.add(pageNum);

      const page = await pdf.getPage(pageNum);
      const viewport = page.getViewport({ scale: RENDER_SCALE });
      const ratio = window.devicePixelRatio || 1;

      const canvas = document.createElement("canvas");
      canvas.width = Math.floor(viewport.width * ratio);
      canvas.height = Math.floor(viewport.height * ratio);
      canvas.style.width = `${viewport.width}px`;
      canvas.style.height = `${viewport.height}px`;
      const ctx = canvas.getContext("2d");
      if (!ctx) return;
      host.querySelector("canvas")?.remove();
      host.appendChild(canvas);

      await page.render({
        canvasContext: ctx,
        viewport,
        transform: ratio !== 1 ? [ratio, 0, 0, ratio, 0, 0] : undefined,
      }).promise;
    }, []);

    const openDocument = useCallback(
      async (nextDocId: number) => {
        if (pdfRef.current && docId === nextDocId) return;
        setLoading(true);
        setError(null);
        setHighlight(null);
        renderedRef.current.clear();
        try {
          const bytes = await invoke<number[]>("document_bytes", {
            docId: nextDocId,
          });
          const pdf = await pdfjsLib.getDocument({
            data: new Uint8Array(bytes),
          }).promise;
          pdfRef.current?.destroy();
          pdfRef.current = pdf;

          const dims: PageDim[] = [];
          for (let n = 1; n <= pdf.numPages; n++) {
            const page = await pdf.getPage(n);
            const vp = page.getViewport({ scale: RENDER_SCALE });
            dims.push({ width: vp.width, height: vp.height });
          }
          setPageDims(dims);
          setDocId(nextDocId);
        } catch (e) {
          setError(String(e));
          setPageDims([]);
        } finally {
          setLoading(false);
        }
      },
      [docId],
    );

    const highlightSpan = useCallback(
      async (spanId: number) => {
        try {
          const loc = await invoke<SpanLocation>("span_location", { spanId });
          if (loc.doc_id !== docId || !pdfRef.current) {
            await openDocument(loc.doc_id);
          }
          const pdf = pdfRef.current;
          if (!pdf) return;

          await renderPage(loc.page_num);
          const host = pageElsRef.current.get(loc.page_num);
          host?.scrollIntoView({ behavior: "smooth", block: "start" });

          const page = await pdf.getPage(loc.page_num);
          const viewport = page.getViewport({ scale: RENDER_SCALE });
          setHighlight({ pageNum: loc.page_num, ...bboxToRect(viewport, loc.bbox) });
        } catch (e) {
          setError(String(e));
        }
      },
      [docId, openDocument, renderPage],
    );

    useImperativeHandle(ref, () => ({ openDocument, highlightSpan }), [
      openDocument,
      highlightSpan,
    ]);

    // Lazily render pages as they scroll into view (keeps a 200-page PDF
    // from rendering every canvas up front).
    useEffect(() => {
      const root = scrollRef.current;
      if (!root || pageDims.length === 0) return;
      const io = new IntersectionObserver(
        (entries) => {
          for (const entry of entries) {
            if (!entry.isIntersecting) continue;
            const n = Number(
              (entry.target as HTMLElement).dataset.pageNum ?? "0",
            );
            if (n > 0) void renderPage(n);
          }
        },
        { root, rootMargin: "200px 0px" },
      );
      for (const el of pageElsRef.current.values()) io.observe(el);
      return () => io.disconnect();
    }, [pageDims, renderPage]);

    useEffect(() => {
      return () => {
        pdfRef.current?.destroy();
        pdfRef.current = null;
      };
    }, []);

    return (
      <div className={`pdf-viewer ${className ?? ""}`} ref={scrollRef}>
        {loading && <p className="pdf-status">Loading document…</p>}
        {error && (
          <p className="pdf-status err" role="alert">
            {error}
          </p>
        )}
        {!loading && !error && pageDims.length === 0 && (
          <p className="pdf-status empty">
            Select a document to view it here.
          </p>
        )}
        {pageDims.map((dim, i) => {
          const pageNum = i + 1;
          return (
            <div
              key={pageNum}
              className="pdf-page"
              data-page-num={pageNum}
              ref={(el) => {
                if (el) pageElsRef.current.set(pageNum, el);
                else pageElsRef.current.delete(pageNum);
              }}
              style={{ width: dim.width, height: dim.height }}
            >
              {highlight?.pageNum === pageNum && (
                <div
                  className="pdf-highlight"
                  style={{
                    left: highlight.left,
                    top: highlight.top,
                    width: highlight.width,
                    height: highlight.height,
                  }}
                />
              )}
            </div>
          );
        })}
      </div>
    );
  },
);
