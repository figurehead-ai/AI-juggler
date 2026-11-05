/**
 * AJ-E2911 (D-75): Report a bug. The user's words first, then each part
 * that goes with them, scrubbed on this computer, to keep or leave out and
 * read in full; then the whole report exactly as the bug form will get it.
 * Nothing leaves until the user opens the form (GitHub, in the browser,
 * where it can still be edited before it's submitted) or copies it.
 */
import { useEffect, useState, type JSX } from "react";
import { Field, Input, Modal } from "./form";
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
  type BugPartInfo,
  type BugReportInfo,
} from "../lib/ipc/report";

/** form.tsx's `Input` surface, for the one multi-line field (design
 * review 33: a filled `bg-canvas` field read as disabled in light). */
const FIELD_SURFACE =
  "w-full rounded-xl border border-hairline bg-surface-2 px-3 text-sm text-ink placeholder:text-muted transition-colors focus:border-accent/50";

/** "What AI Juggler showed" inside a sentence: "what AI Juggler showed"
 * (a leading capitalised name, "AI Juggler", stays as it is). */
export function inSentence(title: string): string {
  return /^[A-Z][a-z]/.test(title) ? title[0]!.toLowerCase() + title.slice(1) : title;
}

/** The fold's line: what goes with the report, as kept now. */
export function goesWithWords(parts: readonly BugPartInfo[], keep: readonly string[]): string {
  const kept = parts.filter((part) => keep.includes(part.key)).map((part) => inSentence(part.title));
  return kept.length === 0 ? "Nothing else goes with it" : `Goes with it: ${kept.join(", ")}`;
}

/** Design review 28/38/56: the parts that go with the report, folded to
 * one line that names them; open, plain rows with hairlines, each row
 * its own fold for the part's full text (no "Show" on every row). */
function ReportParts({ parts, keep, onToggle }: { parts: readonly BugPartInfo[]; keep: readonly string[]; onToggle: (key: string, on: boolean) => void }): JSX.Element {
  return (
    <Disclosure
      testId="report-parts"
      summary={
        <span>
          <span className="text-ink-2">{goesWithWords(parts, keep)}</span>
          <span className="text-muted"> · Change</span>
        </span>
      }
    >
      <p className="mt-2 text-label leading-relaxed text-muted">
        Scrubbed on this computer: no messages, transcripts, credentials, names or home folder. Untick anything you'd rather
        leave out; open a row to read it in full.
      </p>
      <ul className="mt-1 flex flex-col divide-y divide-hairline">
        {parts.map((part) => (
          <li key={part.key} className="flex items-start gap-2 py-2" data-testid="report-part">
            <Checkbox
              className="mt-1"
              checked={keep.includes(part.key)}
              onChange={(event) => onToggle(part.key, event.target.checked)}
              aria-label={part.title}
              data-testid={`report-keep-${part.key}`}
            />
            <Disclosure
              className="min-w-0 flex-1"
              summary={
                <span className="text-xs">
                  <span className="font-medium text-ink">{part.title}</span>
                  <span className="text-muted"> · {part.about}</span>
                </span>
              }
            >
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
    </Disclosure>
  );
}

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
        <Field label="Title">
          <Input
            className="w-full"
            value={title}
            placeholder={report?.title || "A short summary"}
            onChange={(event) => setTitle(event.target.value)}
            data-testid="report-title"
          />
        </Field>
        <Field label="What happened?">
          <textarea
            data-autofocus
            className={`min-h-24 py-2 ${FIELD_SURFACE}`}
            value={words}
            placeholder="What you did, what you expected, and what happened instead."
            onChange={(event) => setWords(event.target.value)}
            data-testid="report-words"
          />
        </Field>
        {!report && !error && <p className="text-xs text-muted">Gathering…</p>}
        {report && <ReportParts parts={report.parts} keep={keep} onToggle={toggle} />}
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
