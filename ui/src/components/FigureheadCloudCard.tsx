/**
 * Settings › Advanced › Figurehead & Cloud (AJ-E1203; named "Account &
 * Cloud" until AJ-E2110, D-35: an account is a subscription to an AI
 * tool, so the optional Figurehead sign-in says its own name) — the
 * single home for the optional Figurehead account (D-18, ADR-0014):
 * signed out, linking, the consent screen, and connected.
 *
 * Copy rule: the desktop path is complete — "You can do everything
 * without an account; Cloud adds…". Every cloud feature says what it
 * does and exactly what it sends. No prompts anywhere else, ever.
 */
import { useCallback, useEffect, useState, type JSX } from "react";
import { Button, Card, Chip } from "./primitives";
import { CardTitle } from "./CardTitle";
import { Connected, Consent } from "./FigureheadCloudParts";
import {
  errorText,
  figureheadClaim,
  figureheadDismissNotice,
  figureheadLinkCancel,
  figureheadLinkPoll,
  figureheadLinkStart,
  figureheadOpenPage,
  figureheadSaveChoices,
  figureheadStatus,
  type FigureheadStatusInfo,
} from "../lib/bridge";

/** The account's state and the calls that change it: the one connect
 * flow, for Settings and for first run's optional step (D-46). */
export function useFigurehead() {
  const [status, setStatus] = useState<FigureheadStatusInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const load = useCallback(async () => {
    try {
      setStatus(await figureheadStatus());
    } catch (cause) {
      setError(errorText(cause));
    }
  }, []);
  useEffect(() => {
    void load();
  }, [load]);

  // While a browser link waits, poll (the app paces it to the server).
  const waiting = status?.state === "linking" && status.link?.phase === "waiting";
  useEffect(() => {
    if (!waiting) return;
    const timer = window.setInterval(async () => {
      try {
        setStatus(await figureheadLinkPoll());
      } catch (cause) {
        setError(errorText(cause));
      }
    }, 1000);
    return () => window.clearInterval(timer);
  }, [waiting]);

  const run = async (action: () => Promise<FigureheadStatusInfo>) => {
    setBusy(true);
    setError(null);
    try {
      setStatus(await action());
    } catch (cause) {
      setError(errorText(cause));
    } finally {
      setBusy(false);
    }
  };
  return { status, error, busy, run };
}

export type Figurehead = ReturnType<typeof useFigurehead>;

/** The flow itself: signed out, linking, the consent screen, connected. */
export function FigureheadFlow({ fh }: { fh: Figurehead }): JSX.Element {
  const { status, error, busy, run } = fh;
  return (
    <>
      {status?.notice && (
        <div
          className="flex flex-wrap items-center gap-2 rounded-lg border border-warning/40 px-3 py-2 text-xs"
          role="status"
          data-testid="cloud-notice"
        >
          <span className="flex-1">{status.notice} Everything on this Mac is unchanged.</span>
          <Button size="sm" variant="secondary" onClick={() => void run(figureheadDismissNotice)}>
            Dismiss
          </Button>
        </div>
      )}
      {error && (
        <p className="rounded-lg border border-danger/30 bg-danger-soft px-3 py-2 text-xs text-danger" role="alert">
          {error}
        </p>
      )}
      {status === null ? (
        <p className="text-xs text-ink-2">Loading…</p>
      ) : status.state === "signed_out" ? (
        <SignedOut
          status={status}
          busy={busy}
          onConnect={(name) => void run(() => figureheadLinkStart(name))}
          onClaim={(code, name) => void run(() => figureheadClaim(code, name))}
        />
      ) : status.state === "linking" ? (
        <Linking
          status={status}
          onRetry={() => void run(async () => {
            await figureheadLinkCancel();
            return figureheadLinkStart(status.device_name);
          })}
          onCancel={() => void run(figureheadLinkCancel)}
        />
      ) : status.consent_version < status.consent_current ? (
        <Consent status={status} busy={busy} onSave={(features) => void run(() => figureheadSaveChoices(features))} />
      ) : (
        <Connected status={status} busy={busy} run={run} />
      )}
    </>
  );
}

/** Starts the browser link with this Mac's name, as the signed-out
 * form's Connect account does. */
export function connectFigurehead(fh: Figurehead): void {
  const name = fh.status?.device_name ?? "";
  if (name.trim()) void fh.run(() => figureheadLinkStart(name.trim()));
}

export function FigureheadCloudCard(): JSX.Element {
  const fh = useFigurehead();
  return (
    <Card className="flex flex-col gap-4 p-5" data-testid="account-cloud" data-state={fh.status?.state ?? "loading"}>
      <div className="flex items-center gap-2">
        <CardTitle>Figurehead &amp; Cloud</CardTitle>
        <Chip tone="neutral">Optional</Chip>
      </div>
      <FigureheadFlow fh={fh} />
    </Card>
  );
}

function SignedOut({
  status,
  busy,
  onConnect,
  onClaim,
}: {
  status: FigureheadStatusInfo;
  busy: boolean;
  onConnect: (name: string) => void;
  onClaim: (code: string, name: string) => void;
}): JSX.Element {
  const [name, setName] = useState(status.device_name);
  const [codeMode, setCodeMode] = useState(false);
  const [code, setCode] = useState("");
  const validName = name.trim().length > 0 && name.trim().length <= 64;
  return (
    <div className="flex flex-col gap-3" data-testid="cloud-signed-out">
      <p className="text-xs leading-relaxed text-ink-2">
        You can do everything in AI Juggler without an account. A free Figurehead account adds
        Cloud features you choose to turn on — encrypted backups you can restore on another Mac,
        and finding your own Macs for Fleet without a pairing code. Your sign-ins to Claude,
        Codex and other tools never leave this Mac.
      </p>
      <label className="flex flex-col gap-1 text-label font-medium">
        This Mac's name on Figurehead
        <input
          value={name}
          maxLength={64}
          onChange={(event) => setName(event.target.value)}
          className="max-w-xs rounded-lg border border-hairline bg-canvas px-2.5 py-1.5 text-sm font-normal text-ink outline-none focus:border-accent/50"
          data-testid="cloud-name"
        />
      </label>
      <div className="flex flex-wrap gap-2">
        <Button
          variant="primary"
          disabled={busy || !validName}
          onClick={() => onConnect(name.trim())}
          data-testid="cloud-connect"
        >
          Connect account
        </Button>
        <Button variant="secondary" onClick={() => setCodeMode((now) => !now)} data-testid="cloud-use-code">
          Use a pairing code
        </Button>
      </div>
      {codeMode && (
        <form
          className="flex flex-col gap-2 rounded-lg border border-hairline p-3"
          onSubmit={(event) => {
            event.preventDefault();
            if (code.trim() && validName) onClaim(code.trim(), name.trim());
          }}
        >
          <p className="text-label leading-relaxed text-muted">
            On figurehead.ai, open Devices and choose <span className="text-ink">Generate pairing code</span>,
            then type it here. Codes work once, for 10 minutes.{" "}
            <button
              type="button"
              className="text-accent-hi underline-offset-2 hover:underline"
              onClick={() => void figureheadOpenPage("devices")}
            >
              Open Devices
            </button>
          </p>
          <div className="flex flex-wrap items-center gap-2">
            <input
              value={code}
              onChange={(event) => setCode(event.target.value)}
              placeholder="QVM3-GABM"
              aria-label="Pairing code from figurehead.ai"
              className="w-40 rounded-lg border border-hairline bg-canvas px-2.5 py-1.5 font-mono text-sm uppercase text-ink outline-none focus:border-accent/50"
              data-testid="cloud-code"
            />
            <Button type="submit" size="sm" variant="primary" disabled={busy || !code.trim()} data-testid="cloud-claim">
              Link this Mac
            </Button>
          </div>
        </form>
      )}
    </div>
  );
}

function Linking({
  status,
  onRetry,
  onCancel,
}: {
  status: FigureheadStatusInfo;
  onRetry: () => void;
  onCancel: () => void;
}): JSX.Element {
  const link = status.link;
  if (!link) return <p className="text-xs text-ink-2">Connecting…</p>;
  if (link.phase !== "waiting") {
    const copy =
      link.phase === "denied"
        ? "The request was denied in the browser, so nothing was linked."
        : link.phase === "expired"
          ? "The request expired before it was approved, so nothing was linked."
          : (link.message ?? "Linking didn't finish.");
    return (
      <div className="flex flex-col gap-3" role="alert" data-testid="cloud-link-failed" data-phase={link.phase}>
        <p className="text-xs">{copy}</p>
        <div className="flex gap-2">
          <Button variant="primary" onClick={onRetry} data-testid="cloud-retry">
            Try again
          </Button>
          <Button variant="secondary" onClick={onCancel}>
            Cancel
          </Button>
        </div>
      </div>
    );
  }
  return (
    <div className="flex flex-col gap-3" data-testid="cloud-linking">
      <p className="text-xs leading-relaxed text-ink-2">
        We opened figurehead.ai in your browser. Check that it shows this code, then click{" "}
        <span className="text-ink">Approve this Mac</span>.
      </p>
      <p
        className="self-start rounded-lg border border-hairline bg-canvas px-4 py-2 font-mono text-2xl font-semibold tracking-[0.2em]"
        aria-label={`Code ${link.user_code.split("").join(" ")}`}
        data-testid="cloud-user-code"
      >
        {link.user_code}
      </p>
      <p className="text-label text-muted" role="status">
        Waiting for approval…
      </p>
      <div className="flex flex-wrap gap-2">
        <Button variant="secondary" onClick={() => void figureheadOpenPage("verify")} data-testid="cloud-open-verify">
          Open the page again
        </Button>
        <Button variant="secondary" onClick={onCancel} data-testid="cloud-cancel">
          Cancel
        </Button>
      </div>
    </div>
  );
}
