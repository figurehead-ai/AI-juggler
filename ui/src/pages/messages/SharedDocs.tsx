/**
 * E18 (AJ-E1804) — the documents agents shared with a pool: who shared
 * each, when, and what redaction found in it, said as packs say it (the
 * same "withheld" words). Opening one shows its text as stored, read-only.
 * Twenty at a time.
 */
import { useEffect, useState, type JSX } from "react";
import { Button } from "../../components/primitives";
import { errorText } from "../../lib/bridge";
import { ago } from "../../lib/time-words";
import { messagesDoc, messagesDocs, onMessagesChanged, type DocContent, type DocInfo } from "../../lib/ipc/messages";
import { byteSize } from "../../lib/compact";

/** What was found in a doc and how it was treated. */
export function withheldWords(doc: DocInfo): string {
  if (doc.withheld.length === 0) return "No secrets found";
  const one = (what: string) => (what.endsWith("s") ? what.slice(0, -1) : what);
  const found = doc.withheld.map((item) => `${item.count} ${item.count === 1 ? one(item.what) : item.what}`).join(", ");
  return doc.policy === "keep" ? `${found} kept, as you chose` : `${found} withheld`;
}

export function SharedDocs({ poolId, now }: { poolId: string; now: number }): JSX.Element {
  const [docs, setDocs] = useState<DocInfo[] | null>(null);
  const [next, setNext] = useState<string | null>(null);
  const [open, setOpen] = useState<DocContent | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    let alive = true;
    const read = () =>
      void messagesDocs(poolId).then(
        (page) => alive && (setDocs(page.docs), setNext(page.next)),
        (cause) => alive && setError(errorText(cause)),
      );
    read();
    const off = onMessagesChanged(read);
    return () => {
      alive = false;
      off();
    };
  }, [poolId]);
  const show = (doc: DocInfo) =>
    void messagesDoc(poolId, doc.id).then(
      (content) => setOpen(content),
      (cause) => setError(errorText(cause)),
    );
  return (
    <section className="flex flex-col gap-3" aria-labelledby={`docs-${poolId}`} data-testid="shared-docs">
      <h3 id={`docs-${poolId}`} className="text-body font-semibold text-ink">
        Shared documents
      </h3>
      {docs === null ? (
        <span className="h-10 rounded-lg bg-surface-3 motion-safe:animate-pulse" role="status" aria-label="Loading the documents" />
      ) : docs.length === 0 ? (
        <p className="text-meta text-muted">None yet. When an agent shares a document with this pool, it shows here, with any secrets handled as you chose for packs.</p>
      ) : (
        <ul className="flex flex-col divide-y divide-hairline/60 rounded-xl border border-hairline">
          {docs.map((doc) => (
            <li key={doc.id} className="flex flex-wrap items-center gap-3 px-3 py-2.5" data-testid="shared-doc">
              <div className="flex min-w-0 flex-1 flex-col gap-0.5">
                <span className="truncate font-mono text-meta text-ink">{doc.name}</span>
                <span className="text-label text-muted">
                  {doc.shared_by} · {ago(doc.at_ms, now)} ago · {byteSize(doc.size_bytes)}
                </span>
              </div>
              <span className={"text-label " + (doc.withheld.length > 0 && doc.policy !== "keep" ? "text-ink" : "text-muted")} data-testid="shared-doc-withheld">
                {withheldWords(doc)}
              </span>
              <Button size="sm" variant="secondary" onClick={() => show(doc)} aria-expanded={open?.doc.id === doc.id}>
                Read
                <span className="sr-only"> {doc.name}</span>
              </Button>
            </li>
          ))}
        </ul>
      )}
      {next && (
        <Button size="sm" variant="ghost" className="self-start" onClick={() => void messagesDocs(poolId, next).then((page) => (setDocs((was) => [...(was ?? []), ...page.docs]), setNext(page.next)))}>
          Show more documents
        </Button>
      )}
      {open && (
        <div className="flex flex-col gap-2 rounded-xl border border-hairline bg-surface-2/60 p-3" data-testid="shared-doc-text">
          <div className="flex items-center justify-between gap-2">
            <span className="text-meta text-ink">
              <span className="font-mono">{open.doc.name}</span> · {open.doc.policy_words}
            </span>
            <Button size="sm" variant="ghost" onClick={() => setOpen(null)}>
              Close
            </Button>
          </div>
          <pre className="max-h-80 overflow-auto whitespace-pre-wrap break-words rounded-lg bg-canvas p-3 font-mono text-meta leading-relaxed text-ink" tabIndex={0} aria-label={`${open.doc.name} (read-only)`}>
            {open.text}
          </pre>
        </div>
      )}
      {error && (
        <p role="alert" className="text-meta text-danger">
          {error}
        </p>
      )}
    </section>
  );
}
