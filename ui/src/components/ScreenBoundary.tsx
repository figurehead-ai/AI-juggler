/**
 * A screen that throws shows "This screen hit a problem", not a blank
 * window (the 0.0.12 report: clicking Usage crashed the whole React tree,
 * rail and header included). One boundary per screen keeps the rail and
 * the header working; the root one in main.tsx is the last resort.
 *
 * The error goes through the crash-report path (`crash_ui_error`): the
 * screen's name, the error's message and its stack — never what was on
 * the screen — scrubbed, and kept only if the user's crash-report setting
 * allows.
 */
import { Component, useState, type ErrorInfo, type JSX, type ReactNode } from "react";
import { crashUiError } from "../lib/bridge";
import { useScrollMemory } from "../lib/scroll-memory";
import { Button, Card } from "./primitives";

type Props = {
  /** The screen's id for the report (`usage`, `home`, …, `app`). */
  screen: string;
  /** The screen's name in words ("Usage"). */
  label: string;
  children: ReactNode;
  /** The whole app (main.tsx): reopening the window, not a retry. */
  root?: boolean;
};

type State = { error: Error | null };

/** The words "Copy details" puts on the clipboard: the error and where
 * it happened in the code, nothing the user typed or saw. */
export function errorDetails(screen: string, error: Error): string {
  return [`Screen: ${screen}`, `${error.name}: ${error.message}`, error.stack ?? ""].filter(Boolean).join("\n");
}

export class ScreenBoundary extends Component<Props, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo): void {
    // Reporting must never throw from here; a failed report is said in
    // the console (the panel itself still shows).
    void crashUiError(this.props.screen, `${error.name}: ${error.message}`, `${error.stack ?? ""}\n${info.componentStack ?? ""}`).catch(
      (cause: unknown) => console.warn("AI Juggler couldn't keep the crash report", cause),
    );
  }

  render(): ReactNode {
    const { error } = this.state;
    if (!error) return this.props.children;
    return <ProblemPanel {...this.props} error={error} retry={() => this.setState({ error: null })} />;
  }
}

function ProblemPanel({
  screen,
  label,
  root,
  error,
  retry,
}: Props & { error: Error; retry: () => void }): JSX.Element {
  // Copying says whether it worked.
  const [copied, setCopied] = useState<"idle" | "copied" | "failed">("idle");
  const copy = () => {
    const clipboard = navigator.clipboard;
    if (!clipboard) {
      setCopied("failed");
      return;
    }
    clipboard.writeText(errorDetails(screen, error)).then(
      () => setCopied("copied"),
      () => setCopied("failed"),
    );
  };
  return (
    <div className={root ? "flex min-h-screen items-center justify-center bg-canvas p-6" : "flex h-full items-start justify-center p-6"}>
      <Card className="flex max-w-lg flex-col gap-3 p-6" data-testid="screen-problem" data-screen={screen} role="alert">
        <h2 className="text-base font-semibold text-ink">{root ? "AI Juggler hit a problem" : "This screen hit a problem"}</h2>
        <p className="text-sm leading-relaxed text-ink-2">
          {root
            ? "Something went wrong while showing the window. Reopening it usually helps."
            : `${label} couldn't be shown. The rest of AI Juggler still works: Try again, or copy the details to report it.`}
        </p>
        <div className="flex flex-wrap gap-2">
          <Button variant="primary" onClick={root ? () => window.location.reload() : retry}>
            {root ? "Reopen the window" : "Try again"}
          </Button>
          <Button variant="secondary" onClick={copy}>
            {copied === "copied" ? "Copied" : "Copy details"}
          </Button>
        </div>
        {copied === "failed" && (
          <div className="flex flex-col gap-1.5" role="status">
            <p className="text-xs text-ink-2">Couldn't copy the details. Select them here instead:</p>
            <pre className="max-h-40 select-text overflow-auto whitespace-pre-wrap rounded-md bg-surface-2 p-2 font-mono text-label text-ink" data-testid="screen-problem-details">
              {errorDetails(screen, error)}
            </pre>
          </div>
        )}
      </Card>
    </div>
  );
}

/** A screen that scrolls in the main area, inside its own boundary. It
 * opens where it was left (0.3.4), unless what it shows changed
 * (`subject`: a search, a section), which starts at the top. */
export function ScrollingScreen({ screen, label, subject, children }: { screen: string; label: string; subject?: string; children: ReactNode }): JSX.Element {
  const scroll = useScrollMemory(`screen:${screen}`, { subject });
  return (
    <div ref={scroll} className="relative h-full overflow-y-auto">
      <ScreenBoundary screen={screen} label={label}>
        {children}
      </ScreenBoundary>
    </div>
  );
}
