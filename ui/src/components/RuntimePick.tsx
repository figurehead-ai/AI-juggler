/**
 * AJ-E2304/E2308 — this session's own runtime, picked at launch (D-40):
 * as its launch settings say (the default), "Start in terminal", or, with
 * Settings › Sessions › Run without a terminal (Beta) on, "Run without a
 * terminal: answer from AI Juggler (Beta)". The backend decides what each
 * tool can do; a tool without a protocol always runs in its terminal.
 */
import { useEffect, useState, type JSX } from "react";
import { runtimeChoices } from "../lib/ipc/runtime";

export type RuntimeChoice = "terminal" | "driven" | undefined;

export function useRuntimePick(): { runtime: RuntimeChoice; element: JSX.Element } {
  const [runtime, setRuntime] = useState<RuntimeChoice>(undefined);
  const [beta, setBeta] = useState(false);
  useEffect(() => {
    void runtimeChoices().then((choices) => setBeta(choices.beta_on), () => setBeta(false));
  }, []);
  const element = (
    <label className="flex items-center gap-2 text-xs text-muted" data-testid="runtime-pick">
      <span>How it runs</span>
      <select
        className="h-7 rounded-lg border border-hairline bg-surface-2 px-2 text-xs text-ink"
        value={runtime ?? ""}
        onChange={(event) => setRuntime(event.target.value === "" ? undefined : (event.target.value as "terminal" | "driven"))}
      >
        <option value="">As its launch settings say</option>
        <option value="terminal">Start in terminal</option>
        {beta && <option value="driven">Run without a terminal: answer from AI Juggler (Beta)</option>}
      </select>
    </label>
  );
  return { runtime, element };
}
