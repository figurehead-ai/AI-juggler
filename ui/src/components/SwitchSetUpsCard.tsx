/**
 * Review of #132 (M3): what switches set up on your accounts, kept after
 * the switch screen closes, each with its Undo. Without it, a set-up made
 * before a switch that then failed, or one undone too late, had nowhere
 * to be undone from. Shown in Settings beside the switch's secrets
 * choice; nothing shows when there's nothing to undo. A set-up whose
 * Undo was refused (it changed since) can be forgotten instead.
 * AJ-E1507 and AJ-E1508 share the record and the undo (a rules apply, a
 * settings copy), so the card names AI Juggler, not switches.
 */
import { useCallback, useEffect, useState, type JSX } from "react";
import { Card } from "./primitives";
import { errorText } from "../lib/bridge";
import {
  switchSetupForget,
  switchSetupList,
  switchSetupUndo,
  type SetUpInfo,
} from "../lib/ipc/switchSetup";

export function SwitchSetUpsCard(): JSX.Element | null {
  const [setUps, setSetUps] = useState<SetUpInfo[]>([]);
  const [status, setStatus] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  // The set-up whose Undo was just refused: it gets a Forget.
  const [refused, setRefused] = useState<string | null>(null);

  const reload = useCallback(async () => {
    setSetUps(await switchSetupList());
  }, []);

  useEffect(() => {
    void reload().catch((cause) => setError(errorText(cause)));
  }, [reload]);

  const run = async (setUp: SetUpInfo, action: (record: string) => Promise<string>) => {
    setError(null);
    setRefused(null);
    try {
      setStatus(await action(setUp.record));
      await reload();
    } catch (cause) {
      setStatus(null);
      setError(errorText(cause));
      if (action === switchSetupUndo) setRefused(setUp.record);
    }
  };

  if (setUps.length === 0 && !status && !error) return null;
  return (
    <Card className="flex flex-col gap-3 p-5" data-testid="switch-set-ups">
      <div>
        <h3 className="text-sm font-semibold">Set up by AI Juggler</h3>
        <p className="mt-0.5 text-xs leading-relaxed text-muted">
          What you had AI Juggler add or change: set-ups for a switch, settings copied into an
          account, and rules files brought in line. Undo puts each back, if it's still as it was
          made.
        </p>
      </div>
      {setUps.length > 0 && (
        <ul className="flex flex-col gap-1.5">
          {setUps.map((setUp) => (
            <li key={setUp.record} className="flex items-center justify-between gap-2 text-xs">
              <span>
                {setUp.what} <span className="text-muted">· {setUp.place}</span>
              </span>
              <span className="flex shrink-0 gap-1">
                {refused === setUp.record && (
                  <button
                    type="button"
                    onClick={() => void run(setUp, switchSetupForget)}
                    aria-label={`Forget: ${setUp.what} on ${setUp.place}`}
                    title="Stop listing it here and leave it as it is"
                    className="rounded border border-hairline px-1.5 py-0.5 text-[10px] hover:bg-surface-2"
                  >
                    Forget
                  </button>
                )}
                <button
                  type="button"
                  onClick={() => void run(setUp, switchSetupUndo)}
                  aria-label={`Undo: ${setUp.what} on ${setUp.place}`}
                  className="rounded border border-hairline px-1.5 py-0.5 text-[10px] hover:bg-surface-2"
                >
                  Undo
                </button>
              </span>
            </li>
          ))}
        </ul>
      )}
      {status && (
        <p className="text-[11px] text-muted" role="status">
          {status}
        </p>
      )}
      {error && (
        <p className="text-[11px] text-danger" role="alert">
          {error}
          {refused && " Forget stops listing it here and leaves it as it is."}
        </p>
      )}
    </Card>
  );
}
