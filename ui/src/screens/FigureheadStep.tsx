/**
 * D-46 (D-18, AJ-E1203 Req.3): first run ends with the optional
 * Figurehead account. **Connect account** and **Continue without an
 * account** are equal buttons; nothing is pre-selected; skipping is one
 * click and nothing reminds the user later. Connecting is the same flow
 * as Settings › Figurehead & Cloud (`useFigurehead`, `FigureheadFlow`):
 * the browser link, the consent screen, connected — never a second one.
 */
import { useState, type JSX } from "react";
import { Button } from "../components/primitives";
import { FigureheadFlow, connectFigurehead, useFigurehead } from "../components/FigureheadCloudCard";

export function FigureheadStep({ onDone }: { onDone: (connected: boolean) => void }): JSX.Element {
  const fh = useFigurehead();
  const [connecting, setConnecting] = useState(false);
  const connected = fh.status?.state === "linked";
  return (
    <section className="flex flex-col gap-5" aria-labelledby="first-run-figurehead-title" data-testid="first-run-figurehead">
      <div className="flex flex-col gap-2">
        <h2 id="first-run-figurehead-title" className="text-title text-ink">
          Connect a Figurehead account (optional)
        </h2>
        <p className="max-w-2xl text-body text-ink">
          You can do everything in AI Juggler without an account. A free Figurehead account adds Cloud features you
          choose to turn on: encrypted backups you can restore on another computer, and finding your own Macs for Fleet
          without a pairing code. Your sign-ins to Claude, Codex and other tools never leave this computer.
        </p>
      </div>
      {!connecting ? (
        <div className="flex flex-wrap gap-3" data-testid="first-run-figurehead-choice">
          {/* Equal buttons, neither one the default (D-18). */}
          <Button
            variant="secondary"
            disabled={fh.status === null}
            onClick={() => {
              setConnecting(true);
              if (fh.status?.state === "signed_out") connectFigurehead(fh);
            }}
            data-testid="first-run-figurehead-connect"
          >
            Connect account
          </Button>
          <Button variant="secondary" onClick={() => onDone(false)} data-testid="first-run-figurehead-skip">
            Continue without an account
          </Button>
        </div>
      ) : (
        <div className="flex flex-col gap-4">
          <FigureheadFlow fh={fh} />
          <div className="flex flex-wrap gap-3">
            <Button variant={connected ? "primary" : "secondary"} onClick={() => onDone(connected)} data-testid="first-run-figurehead-continue">
              {connected ? "Continue" : "Continue without an account"}
            </Button>
          </div>
        </div>
      )}
      <p className="text-meta text-ink">You can connect one later, or never, in Settings › Figurehead &amp; Cloud.</p>
    </section>
  );
}
