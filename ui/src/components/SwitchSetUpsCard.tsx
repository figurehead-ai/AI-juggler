/**
 * Review of #132 (M3): what switches set up on your accounts, kept after
 * the switch screen closes, each with its Undo. Without it, a set-up made
 * before a switch that then failed, or one undone too late, had nowhere
 * to be undone from. Shown in Settings beside the switch's secrets
 * choice; nothing shows when there's nothing to undo or turn on. A
 * set-up whose Undo was refused (it changed since) can be forgotten
 * instead. AJ-E1507 and AJ-E1508 share the record and the undo (a rules
 * apply, a settings copy), so the card names AI Juggler, not switches.
 *
 * Below them, what a switch's other account still has to turn on itself
 * (a skill that comes with the claude.ai account): the same steps as the
 * switch showed, with Check again, until that account has it.
 */
import { useCallback, useEffect, useState, type JSX } from "react";
import { Card } from "./primitives";
import { errorText } from "../lib/bridge";
import {
  switchSetupForget,
  switchSetupList,
  switchSetupTurnOnDismiss,
  switchSetupTurnOns,
  switchSetupUndo,
  type SetUpInfo,
  type TurnOnInfo,
} from "../lib/ipc/switchSetup";
import { TurnOnSteps } from "./SwitchSetupTurnOn";

const BUTTON = "rounded border border-hairline px-2 py-0.5 text-meta text-ink hover:bg-surface-2";

const KIND_WORDS: Record<string, string> = { skill: "Skill", plugin: "Plugin", "mcp-server": "MCP server" };

export function SwitchSetUpsCard(): JSX.Element | null {
  const [setUps, setSetUps] = useState<SetUpInfo[]>([]);
  const [turnOns, setTurnOns] = useState<TurnOnInfo[]>([]);
  const [status, setStatus] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  // The set-up whose Undo was just refused: it gets a Forget.
  const [refused, setRefused] = useState<string | null>(null);

  const reload = useCallback(async () => {
    const [made, toTurnOn] = await Promise.all([switchSetupList(), switchSetupTurnOns()]);
    setSetUps(made);
    setTurnOns(toTurnOn);
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

  /** Check again for one: the backend reads the account again. */
  const checkAgain = async (turnOn: TurnOnInfo): Promise<string> => {
    const now = await switchSetupTurnOns();
    setTurnOns(now);
    const still = now.some((other) => other.id === turnOn.id);
    if (still) return `${turnOn.account} doesn't have ${turnOn.name} yet. It arrives with ${turnOn.account}'s next session.`;
    setStatus(`${turnOn.account} has ${turnOn.name} now.`);
    return "";
  };

  const dismiss = async (turnOn: TurnOnInfo) => {
    setError(null);
    try {
      setStatus(await switchSetupTurnOnDismiss(turnOn.id));
      await reload();
    } catch (cause) {
      setError(errorText(cause));
    }
  };

  if (setUps.length === 0 && turnOns.length === 0 && !status && !error) return null;
  return (
    <Card className="flex flex-col gap-3 p-5" data-testid="switch-set-ups">
      <div>
        <h3 className="text-section text-ink">Set up by AI Juggler</h3>
        <p className="mt-0.5 text-meta text-muted">
          What you had AI Juggler add or change: set-ups for a switch, settings copied into an
          account, and rules files brought in line. Undo puts each back, if it's still as it was
          made.
        </p>
      </div>
      {setUps.length > 0 && (
        <ul className="flex flex-col gap-1.5">
          {setUps.map((setUp) => (
            <li key={setUp.record} className="flex items-center justify-between gap-2 text-body text-ink">
              <span>
                {setUp.what} <span className="text-meta text-muted">· {setUp.place}</span>
              </span>
              <span className="flex shrink-0 gap-1">
                {refused === setUp.record && (
                  <button
                    type="button"
                    onClick={() => void run(setUp, switchSetupForget)}
                    aria-label={`Forget: ${setUp.what} on ${setUp.place}`}
                    title="Stop listing it here and leave it as it is"
                    className={BUTTON}
                  >
                    Forget
                  </button>
                )}
                <button
                  type="button"
                  onClick={() => void run(setUp, switchSetupUndo)}
                  aria-label={`Undo: ${setUp.what} on ${setUp.place}`}
                  className={BUTTON}
                >
                  Undo
                </button>
              </span>
            </li>
          ))}
        </ul>
      )}
      {turnOns.length > 0 && (
        <section className="flex flex-col gap-2" aria-labelledby="switch-turn-ons-title" data-testid="switch-turn-ons">
          <h4 id="switch-turn-ons-title" className="text-body font-medium text-ink">
            Still to turn on
          </h4>
          <p className="text-meta text-muted">
            These come with an account, so AI Juggler can't copy them: the account turns each on
            itself. Each leaves this list once the account has it.
          </p>
          <ul className="flex flex-col gap-3">
            {turnOns.map((turnOn) => (
              <li key={turnOn.id} className="flex flex-col gap-1" data-testid="switch-turn-on-row">
                <div className="flex items-center justify-between gap-2">
                  <span className="text-body text-ink">
                    <span className="font-medium">{turnOn.name}</span>{" "}
                    <span className="text-meta text-muted">
                      · {KIND_WORDS[turnOn.kind] ?? turnOn.kind} · for {turnOn.account}
                    </span>
                  </span>
                  <button
                    type="button"
                    className={BUTTON}
                    aria-label={`Stop reminding me about ${turnOn.name} on ${turnOn.account}`}
                    onClick={() => void dismiss(turnOn)}
                  >
                    Stop reminding me
                  </button>
                </div>
                <TurnOnSteps getIt={turnOn.get_it} name={turnOn.name} onCheckAgain={() => checkAgain(turnOn)} />
              </li>
            ))}
          </ul>
        </section>
      )}
      {status && (
        <p className="text-meta text-ink" role="status">
          {status}
        </p>
      )}
      {error && (
        <p className="text-meta text-danger" role="alert">
          {error}
          {refused && " Forget stops listing it here and leaves it as it is."}
        </p>
      )}
    </Card>
  );
}
