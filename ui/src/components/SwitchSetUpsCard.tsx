/**
 * Review of #132 (M3): what switches set up on your accounts, kept after
 * the switch screen closes, each with its Undo. Without it, a set-up made
 * before a switch that then failed, or one undone too late, had nowhere
 * to be undone from. Shown in Settings beside the switch's secrets
 * choice; nothing shows when there's nothing to undo.
 */
import { useCallback, useEffect, useState, type JSX } from "react";
import { Card } from "./primitives";
import { errorText } from "../lib/bridge";
import { switchSetupList, switchSetupUndo, type SetUpInfo } from "../lib/ipc/switchSetup";

export function SwitchSetUpsCard(): JSX.Element | null {
  const [setUps, setSetUps] = useState<SetUpInfo[]>([]);
  const [status, setStatus] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const reload = useCallback(async () => {
    setSetUps(await switchSetupList());
  }, []);

  useEffect(() => {
    void reload().catch((cause) => setError(errorText(cause)));
  }, [reload]);

  const undo = async (setUp: SetUpInfo) => {
    setError(null);
    try {
      setStatus(await switchSetupUndo(setUp.record));
      await reload();
    } catch (cause) {
      setError(errorText(cause));
    }
  };

  if (setUps.length === 0 && !status && !error) return null;
  return (
    <Card className="flex flex-col gap-3 p-5" data-testid="switch-set-ups">
      <div>
        <h3 className="text-sm font-semibold">Set up by switches</h3>
        <p className="mt-0.5 text-xs leading-relaxed text-muted">
          What a switch added to an account because it was missing there. Undo removes it, if
          it's still as it was made.
        </p>
      </div>
      {setUps.length > 0 && (
        <ul className="flex flex-col gap-1.5">
          {setUps.map((setUp) => (
            <li key={setUp.record} className="flex items-center justify-between gap-2 text-xs">
              <span>
                {setUp.what} <span className="text-muted">· {setUp.account}</span>
              </span>
              <button
                type="button"
                onClick={() => void undo(setUp)}
                aria-label={`Undo: ${setUp.what} on ${setUp.account}`}
                className="rounded border border-hairline px-1.5 py-0.5 text-[10px] hover:bg-surface-2"
              >
                Undo
              </button>
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
        </p>
      )}
    </Card>
  );
}
