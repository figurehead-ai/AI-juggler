/**
 * AJ-E2304/E2308 — this session's own runtime, picked at launch (D-40):
 * as its launch settings say (the default), "Start in terminal", or, with
 * Settings › Sessions › Run without a terminal (Beta) on, "Run without a
 * terminal: answer from AI Juggler (Beta)". The backend decides what each
 * tool can do; a tool without a protocol always runs in its terminal.
 *
 * AJ-E2312 item 5 (LC1: the person picked "Run without a terminal" again
 * at every start): it's offered only where the App can run without a
 * terminal now (its tool has that runtime and nothing keeps it from
 * running, with the Beta switch on: D-127), and it's remembered for the
 * App it was started on, so New session opens on it there again; a start
 * there with another choice forgets it. It's still each session's own
 * request (a Beta is never a launch setting's standing choice), said in
 * More options' label ("runs without a terminal"). Kept with this data
 * folder's other kept choices (`lib/local-store`).
 *
 * Lab 2: `confirm` asks a tool's folder-trust question before the launch,
 * when it hasn't been answered for that folder (FolderTrustAsk).
 */
import { useState, type JSX } from "react";
import { runtimeChoices, type RuntimeChoicesInfo } from "../lib/ipc/runtime";
import { keep, readKept } from "../lib/local-store";
import { sharedRead, useSharedRead } from "../lib/shared-read";
import type { LaunchRecipeInfo } from "../types/generated/LaunchRecipeInfo";
import { useFolderTrust } from "./FolderTrustAsk";
import { Picker } from "./controls";

export type RuntimeChoice = "terminal" | "driven" | undefined;

/** Which launch settings can run without a terminal, kept: a sheet
 * opened again has it in its first frame, and reads it again behind. */
const choicesRead = sharedRead<RuntimeChoicesInfo>(() => runtimeChoices());

/** The Apps (their folder ids) last started without a terminal. */
const KEY = "aij.new-session.runtime";

function keptApps(): Record<string, "driven"> {
  try {
    const raw = readKept(KEY);
    const parsed: unknown = raw ? JSON.parse(raw) : {};
    if (typeof parsed !== "object" || parsed === null) return {};
    return Object.fromEntries(Object.entries(parsed).filter((entry): entry is [string, "driven"] => entry[1] === "driven"));
  } catch {
    return {};
  }
}

/** How the App was last started, when that was without a terminal. */
function keptRuntime(appId: string): RuntimeChoice {
  return keptApps()[appId];
}

/** Keeps the runtime a start on the App asked for: "Run without a
 * terminal" is remembered; any other forgets it. */
function keepRuntime(appId: string, runtime: RuntimeChoice): void {
  const apps = keptApps();
  if (runtime === "driven") apps[appId] = "driven";
  else delete apps[appId];
  keep(KEY, JSON.stringify(apps));
}

/** Whether the launch settings `profileId` can run without a terminal
 * now: its tool has that runtime, nothing keeps it from running, and the
 * Beta switch is on. */
function canRunWithout(choices: RuntimeChoicesInfo | null, profileId: string | null): boolean {
  if (!choices?.beta_on || profileId === null) return false;
  return choices.profiles.some((row) => row.profile_id === profileId && row.unavailable === null);
}

/** The launch settings a start is for (`id`) and their App (`account_id`,
 * its folder), where a choice is remembered. */
export type RuntimeFor = Pick<LaunchRecipeInfo, "id" | "account_id">;

export function useRuntimePick(app: RuntimeFor | null = null): {
  runtime: RuntimeChoice;
  element: JSX.Element;
  /** The folder-trust question, shown wherever the caller mounts it:
   * never inside a folded section (the 7b walk: inside New session's
   * folded More options it was never seen, and Start hung on
   * "Starting…"). */
  trustElement: JSX.Element | null;
  confirm: (profileId: string, cwd: string | null) => Promise<boolean>;
  /** After a start on the App: remembers how it was asked to run, where
   * "Run without a terminal" was offered. */
  remember: () => void;
} {
  const choices = useSharedRead(choicesRead) ?? null;
  // What the person picked in this sheet, per App: changing the App shows
  // that App's own.
  const [picked, setPicked] = useState<Record<string, RuntimeChoice>>({});
  const trust = useFolderTrust();
  const key = app?.account_id ?? "";
  const offered = canRunWithout(choices, app?.id ?? null);
  const wanted = key in picked ? picked[key] : app ? keptRuntime(app.account_id) : undefined;
  // Never a choice the App can't use: the launch settings say then.
  const runtime = wanted === "driven" && !offered ? undefined : wanted;
  const element = (
    <div className="flex items-center gap-2 text-xs text-muted" data-testid="runtime-pick">
      <span aria-hidden="true">How it runs</span>
      <Picker choice
        ariaLabel="How it runs"
        size="sm"
        value={runtime ?? "settings"}
        onChange={(next) => setPicked((was) => ({ ...was, [key]: next === "settings" ? undefined : next }))}
        options={[
          { value: "settings", label: "As its launch settings say" },
          { value: "terminal", label: "Start in terminal" },
          ...(offered ? [{ value: "driven" as const, label: "Run without a terminal", hint: "Answer from AI Juggler (Beta)" }] : []),
        ]}
      />
    </div>
  );
  const remember = () => {
    if (app && offered) keepRuntime(app.account_id, runtime);
  };
  return { runtime, element, trustElement: trust.element, confirm: trust.confirm, remember };
}
