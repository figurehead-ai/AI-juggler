/**
 * AJ-E1511 — "Start from a saved handover…" in New session. The pack is read
 * and checked first, then shown as the switch screen shows a switch: where
 * it came from, its fidelity, what it carries and what it doesn't, and
 * what redaction withheld when it was made (the rows a switch's handover
 * is read with too, `HandoverView`). The user picks the account and
 * the folder in New session as for any session; Launch then starts it
 * primed with the pack.
 */
import { useState, type JSX } from "react";
import { Button } from "./primitives";
import { HandoverRows } from "./HandoverView";
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

/** The pack line in New session: a link to choose one, or what it holds. */
export function PackPicker({ pack }: { pack: PackChoice }): JSX.Element {
  if (!pack.opened) {
    return (
      <div className="mb-4 flex flex-wrap items-center gap-2 text-label">
        <button type="button" onClick={() => void pack.choose()} className="inline-flex min-h-6 items-center text-accent-hi underline-offset-2 hover:underline" data-testid="pack-choose">
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
      <HandoverRows
        data={info}
        whereNote={!info.project_here && <span className="text-muted"> (not on this computer: pick where to work below)</span>}
      />
    </section>
  );
}
