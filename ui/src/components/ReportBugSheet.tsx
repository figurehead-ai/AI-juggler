/**
 * AJ-E2911 (D-75): Report a bug. The user's words first, then each part
 * that goes with them, scrubbed on this computer, to keep or leave out and
 * read in full; then the whole report exactly as the bug form will get it.
 * Nothing leaves until the user opens the form (GitHub, in the browser,
 * where it can still be edited before it's submitted) or copies it.
 */
import { useEffect, useState, type JSX } from "react";
import { Modal } from "./form";
import { Button } from "./primitives";
import { Checkbox, Disclosure } from "./controls";
import { errorText } from "../lib/bridge";
import { closeReportBug, openReportBug, useReportBug } from "../lib/report-bug";
import {
  bugReportCompose,
  bugReportOpen,
  bugReportPreview,
  onReportBugAsked,
  type BugComposedInfo,
  type BugContextArgs,
  type BugReportInfo,
} from "../lib/ipc/report";

/** Mounted once; opens when something asks (`openReportBug`). */
export function ReportBugHost(): JSX.Element | null {
  const context = useReportBug();
  useEffect(() => onReportBugAsked(() => openReportBug()), []);
  if (!context) return null;
  return <ReportBugSheet context={context} onClose={closeReportBug} />;
}

export function ReportBugSheet({ context, onClose }: { context: BugContextArgs; onClose: () => void }): JSX.Element {
  const [report, setReport] = useState<BugReportInfo | null>(null);
  const [title, setTitle] = useState("");
  const [words, setWords] = useState("");
  const [keep, setKeep] = useState<string[]>([]);
  const [composed, setComposed] = useState<BugComposedInfo | null>(null);
  const [status, setStatus] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let live = true;
    bugReportPreview(context)
      .then((info) => {
        if (!live) return;
        setReport(info);
        setKeep(info.parts.map((p) => p.key));
      })
      .catch((cause: unknown) => live && setError(errorText(cause)));
    return () => {
      live = false;
    };
  }, [context]);

  useEffect(() => {
    if (!report) return;
    let live = true;
    const timer = window.setTimeout(() => {
      bugReportCompose(report, title, words, keep)
        .then((info) => live && setComposed(info))
        .catch((cause: unknown) => live && setError(errorText(cause)));
    }, 150);
    return () => {
      live = false;
      window.clearTimeout(timer);
    };
  }, [report, title, words, keep]);

  const toggle = (key: string, on: boolean) => setKeep((now) => (on ? [...now, key] : now.filter((k) => k !== key)));
  const open = async () => {
    if (!report) return;
    setError(null);
    try {
      await bugReportOpen(report, title, words, keep);
      setStatus("The bug form is open in your browser. Check it there, then submit it.");
    } catch (cause) {
      setError(errorText(cause));
    }
  };
  const copy = async () => {
    if (!composed) return;
    try {
      await navigator.clipboard.writeText(`${composed.title ? `${composed.title}\n\n` : ""}${composed.body}`);
      setStatus("Copied. Paste it into an issue or a message.");
    } catch {
      setStatus("Couldn't copy here. Select the report below and copy it.");
    }
  };

  return (
    <Modal
      title="Report a bug"
      subtitle="Nothing leaves this computer until you open the form or copy the report."
      onClose={onClose}
      wide
      testId="report-bug"
    >
      <div className="flex max-h-[70vh] flex-col gap-4 overflow-y-auto pr-1">
        <label className="flex flex-col gap-1 text-xs">
          <span className="font-medium text-ink">Title</span>
          <input
            className="rounded-lg border border-hairline bg-canvas px-2.5 py-1.5 text-sm text-ink"
            value={title}
            placeholder={report?.title || "A short summary"}
            onChange={(event) => setTitle(event.target.value)}
            data-testid="report-title"
          />
        </label>
        <label className="flex flex-col gap-1 text-xs">
          <span className="font-medium text-ink">What happened?</span>
          <textarea
            data-autofocus
            className="min-h-24 rounded-lg border border-hairline bg-canvas px-2.5 py-1.5 text-sm text-ink"
            value={words}
            placeholder="What you did, what you expected, and what happened instead."
            onChange={(event) => setWords(event.target.value)}
            data-testid="report-words"
          />
        </label>
        <section aria-labelledby="report-parts" className="flex flex-col gap-2">
          <h3 id="report-parts" className="text-xs font-semibold text-ink">
            What goes with it
          </h3>
          <p className="text-label leading-relaxed text-muted">
            Scrubbed on this computer: no messages, transcripts, credentials, names or home folder. Untick anything you'd
            rather leave out.
          </p>
          {!report && !error && <p className="text-xs text-muted">Gathering…</p>}
          <ul className="flex flex-col gap-1.5">
            {report?.parts.map((part) => (
              <li key={part.key} className="rounded-lg border border-hairline p-2" data-testid="report-part">
                <label className="flex items-center gap-2 text-xs">
                  <Checkbox
                    checked={keep.includes(part.key)}
                    onChange={(event) => toggle(part.key, event.target.checked)}
                    data-testid={`report-keep-${part.key}`}
                  />
                  <span className="font-medium text-ink">{part.title}</span>
                  <span className="text-muted">· {part.about}</span>
                </label>
                <Disclosure summary="Show" className="mt-1 pl-6">
                  <pre
                    className="mt-1 max-h-40 overflow-auto whitespace-pre-wrap break-all rounded bg-canvas p-2 font-mono text-label text-ink"
                    tabIndex={0}
                    aria-label={`${part.title} (read-only)`}
                  >
                    {part.text}
                  </pre>
                </Disclosure>
              </li>
            ))}
          </ul>
        </section>
        {composed && (
          <Disclosure summary="The whole report, as the form gets it" testId="report-whole">
            <dl className="mt-2 flex flex-col gap-2">
              {composed.fields.map((field) => (
                <div key={field.id}>
                  <dt className="text-label font-medium text-ink-2">{field.label}</dt>
                  <dd>
                    <pre className="mt-0.5 max-h-48 overflow-auto whitespace-pre-wrap break-all rounded bg-canvas p-2 font-mono text-label text-ink">
                      {field.value || "—"}
                    </pre>
                  </dd>
                </div>
              ))}
            </dl>
            {composed.cut && (
              <p className="mt-2 text-label text-muted" data-testid="report-cut">
                The logs are cut to fit a link, oldest lines first. Copy has all of them.
              </p>
            )}
          </Disclosure>
        )}
        <p className="text-label leading-relaxed text-muted">
          The form opens on GitHub, where issues are public. You can still change it there before you submit it. For a
          security problem, report it privately instead (SECURITY.md).
        </p>
        {status && (
          <p className="text-xs text-ink-2" role="status" data-testid="report-status">
            {status}
          </p>
        )}
        {error && (
          <p className="text-xs text-danger" role="alert">
            {error}
          </p>
        )}
      </div>
      <div className="mt-4 flex flex-wrap justify-end gap-2">
        <Button size="sm" variant="secondary" onClick={() => void copy()} disabled={!composed} data-testid="report-copy">
          Copy
        </Button>
        <Button size="sm" variant="primary" onClick={() => void open()} disabled={!composed} data-testid="report-open">
          Open the bug form
        </Button>
      </div>
    </Modal>
  );
}
