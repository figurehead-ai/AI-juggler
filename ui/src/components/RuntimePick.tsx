/**
 * AJ-E2304/E2308 — this session's own runtime, picked at launch (D-40):
 * as its launch settings say (the default), "Start in terminal", or, with
 * Settings › Sessions › Run without a terminal (Beta) on, "Run without a
 * terminal: answer from AI Juggler (Beta)". The backend decides what each
 * tool can do; a tool without a protocol always runs in its terminal.
 *
 * Lab 2: `confirm` asks a tool's folder-trust question before the launch,
 * when it hasn't been answered for that folder (FolderTrustAsk).
 */
import { useEffect, useState, type JSX } from "react";
import { runtimeChoices } from "../lib/ipc/runtime";
import { useFolderTrust } from "./FolderTrustAsk";
import { Picker } from "./controls";

export type RuntimeChoice = "terminal" | "driven" | undefined;

export function useRuntimePick(): {
  runtime: RuntimeChoice;
  element: JSX.Element;
  confirm: (profileId: string, cwd: string | null) => Promise<boolean>;
} {
  const [runtime, setRuntime] = useState<RuntimeChoice>(undefined);
  const trust = useFolderTrust();
  const [beta, setBeta] = useState(false);
  useEffect(() => {
    void runtimeChoices().then((choices) => setBeta(choices.beta_on), () => setBeta(false));
  }, []);
  const element = (
    <div className="flex items-center gap-2 text-xs text-muted" data-testid="runtime-pick">
      <span aria-hidden="true">How it runs</span>
      <Picker
        ariaLabel="How it runs"
        size="sm"
        value={runtime ?? "settings"}
        onChange={(next) => setRuntime(next === "settings" ? undefined : next)}
        options={[
          { value: "settings", label: "As its launch settings say" },
          { value: "terminal", label: "Start in terminal" },
          ...(beta ? [{ value: "driven" as const, label: "Run without a terminal", hint: "Answer from AI Juggler (Beta)" }] : []),
        ]}
      />
      {trust.element}
    </div>
  );
  return { runtime, element, confirm: trust.confirm };
}
