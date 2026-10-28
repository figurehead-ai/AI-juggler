/**
 * AJ-E1511 — "Start from a saved handover…" in New session. The pack is read
 * and checked first, then shown as the switch screen shows a switch: where
 * it came from, its fidelity, what it carries and what it doesn't, and
 * what redaction withheld when it was made. The user picks the account and
 * the folder in New session as for any session; Launch then starts it
 * primed with the pack.
 */
import { useState, type JSX } from "react";
import { Button } from "./primitives";
import { errorText } from "../lib/bridge";
import { packsOpenPreview, packsOpenStart, pickPackFolder, type PackOpenInfo, type PackStartedInfo } from "../lib/ipc/packs";

export interface PackChoice {
  opened: { path: string; info: PackOpenInfo } | null;
  error: string | null;
  choose: () => Promise<void>;
  clear: () => void;
  /** Starts the session; refuses without a folder to work in. */
  start: (profileId: string, folder: string | null) => Promise<PackStartedInfo>;
}

/** The pack New session starts from (none until one is chosen). Its own
 * folder, when it's on this Mac, becomes where to work. */
export function usePack(onFolder: (folder: string) => void): PackChoice {
  const [opened, setOpened] = useState<PackChoice["opened"]>(null);
  const [error, setError] = useState<string | null>(null);
  return {
    opened,
    error,
    choose: async () => {
      setError(null);
      const path = await pickPackFolder();
      if (!path) return;
      try {
        const info = await packsOpenPreview(path);
        setOpened({ path, info });
        if (info.project_folder) onFolder(info.project_folder);
      } catch (cause) {
        setOpened(null);
        setError(errorText(cause));
      }
    },
    clear: () => {
      setOpened(null);
      setError(null);
    },
    start: async (profileId, folder) => {
      if (!opened) throw new Error("No handover chosen");
      if (!folder) throw "pick where the session works first";
      return packsOpenStart(opened.path, opened.info.digest, profileId, folder);
    },
  };
}

function Row({ label, children }: { label: string; children: React.ReactNode }): JSX.Element {
  return (
    <div className="grid grid-cols-[7.5rem_1fr] gap-2 text-label">
      <span className="text-muted">{label}</span>
      <span className="min-w-0 text-ink">{children}</span>
    </div>
  );
}

/** The pack line in New session: a link to choose one, or what it holds. */
export function PackPicker({ pack }: { pack: PackChoice }): JSX.Element {
  if (!pack.opened) {
    return (
      <div className="mb-4 flex flex-wrap items-center gap-2 text-label">
        <button type="button" onClick={() => void pack.choose()} className="text-accent-hi underline-offset-2 hover:underline" data-testid="pack-choose">
          Start from a saved handover…
        </button>
        {pack.error && (
          <span role="alert" className="text-danger" data-testid="pack-error">
            {pack.error}
          </span>
        )}
      </div>
    );
  }
  const { info } = pack.opened;
  const made = new Date(info.created_at_ms).toLocaleString(undefined, { dateStyle: "medium", timeStyle: "short" });
  return (
    <section className="mb-4 flex flex-col gap-2 rounded-xl border border-accent/30 bg-accent-soft/40 p-3" aria-labelledby="pack-title" data-testid="pack-summary">
      <div className="flex items-center gap-2">
        <h4 id="pack-title" className="text-xs font-semibold">
          Starting from a saved handover
        </h4>
        <Button size="sm" variant="ghost" className="ml-auto" onClick={pack.clear} data-testid="pack-clear">
          Don't use it
        </Button>
      </div>
      <Row label="From">
        {info.from_tool} on {info.from_account} · {made}
      </Row>
      <Row label="Where">
        <span className="font-mono">{info.project_root}</span> · {info.branch}
        {!info.project_here && <span className="text-muted"> (not on this Mac: pick where to work below)</span>}
      </Row>
      <Row label="Continues as">{info.fidelity}</Row>
      <Row label="Carries">{info.carries.join(", ")}</Row>
      {info.not_carried.length > 0 && <Row label="Doesn't carry">{info.not_carried.join("; ")}</Row>}
      <Row label="Withheld">
        <span data-testid="pack-withheld">
          {info.withheld.length === 0
            ? "Nothing: no secrets were found when it was made."
            : `${info.withheld.map((item) => `${item.count} ${item.what}`).join(", ")}, when it was made. They stay out.`}
        </span>
      </Row>
      <details className="text-label">
        <summary className="cursor-pointer text-muted">The handover's brief</summary>
        <pre className="mt-1 max-h-48 overflow-auto whitespace-pre-wrap rounded bg-canvas p-2 font-mono text-label text-ink" aria-label="The handover's brief (read-only)" tabIndex={0}>
          {info.brief}
        </pre>
      </details>
    </section>
  );
}
