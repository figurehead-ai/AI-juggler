/**
 * 0.4.4 (main's finding): "Measure its limits" measured nothing; it only
 * opened a page. One control for every place that offers it (Home's Next
 * steps and Runway, the Accounts card, the Account page), measuring where
 * it is through D-39's ask (the tool's own usage check, `quota_probe_run`:
 * an async command, so the window never waits on it):
 * - on for the account: a click asks ("Measuring…"), then reads its limits
 *   again, so the reading shows where the button was; a failure says why;
 * - turned off for it: says so, with the way to turn it on (its App's
 *   settings, where it was turned off);
 * - a tool with no such check: no button, and how its limits will come.
 */
import { useEffect, useRef, useState, type JSX } from "react";
import { Button } from "./primitives";
import { rememberTab } from "../pages/registry";
import { openApp } from "../lib/apps";
import { cliAppId } from "../lib/app-name";
import { errorText } from "../lib/bridge";
import { probeRun, probeStatus, type ProbeStatusInfo } from "../lib/ipc/probe";
import { quotaDisplays } from "../lib/kept-reads";
import { providerDisplayName } from "../lib/providers";

/** How an ask ended, in words (`ProbeStatusInfo.last_outcome`). */
export function probeOutcome(outcome: string | null, tool: string): string {
  switch (outcome) {
    case "ran":
      return "it answered";
    case "failed":
      return `${tool} couldn't check (signed out, or offline)`;
    case "timed_out":
      return "it took too long and was stopped";
    case "not_started":
      return `${tool} couldn't be started`;
    default:
      return "it didn't answer";
  }
}

/** After the tool answered, its limits are read again after these waits
 * (ms): the answer is in its own state file, which AI Juggler reads at its
 * next pass (about a second). */
const READ_AGAIN_MS: readonly number[] = [1_000, 2_000, 4_000];

/** Where to ask: the first of the account's folders it's on for, else
 * the first it's offered on (turned off there), else none. */
type Door = { folder: string; status: ProbeStatusInfo };

async function doorOf(folders: readonly string[]): Promise<Door | null> {
  const all = await Promise.all(folders.map((folder) => probeStatus(folder).then((status) => ({ folder, status }), () => null)));
  const offered = all.filter((door): door is Door => door !== null && door.status.offered);
  return offered.find((door) => door.status.state === "on") ?? offered[0] ?? null;
}

/** Opens the folder's App on its settings, where asking is turned on. */
function openTurnOn(folder: string): void {
  rememberTab("app", "settings");
  openApp(cliAppId(folder));
}

const wait = (ms: number) => new Promise<void>((done) => setTimeout(done, ms));
const LINK = "rounded font-medium text-accent-hi outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent/60 disabled:cursor-default disabled:text-muted disabled:no-underline";

export function MeasureLimits({
  folders,
  tool,
  name,
  look = "link",
  onMeasured,
  readAgainMs = READ_AGAIN_MS,
}: {
  /** The account's folders (one on an App's or folder's page). */
  folders: readonly string[];
  /** Its tool's id, for the words. */
  tool: string;
  /** The account's name, for screen readers. */
  name: string;
  /** A button (Home's Next steps) or a link (in a line of words). */
  look?: "button" | "link";
  /** After each read again: a page reading its own limits reads them too. */
  onMeasured?: () => void;
  readAgainMs?: readonly number[];
}): JSX.Element | null {
  const [door, setDoor] = useState<Door | null | undefined>(undefined);
  const [measuring, setMeasuring] = useState(false);
  const [said, setSaid] = useState<{ words: string; problem: boolean } | null>(null);
  const alive = useRef(true);
  const key = folders.join("\u0000");
  useEffect(() => {
    alive.current = true;
    void doorOf(folders).then((found) => alive.current && setDoor(found));
    return () => {
      alive.current = false;
    };
    // The folders by value: a new array of the same ones isn't a change.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key]);
  const toolName = providerDisplayName(tool);
  if (door === undefined) return null;
  if (door === null) {
    return (
      <span className="text-label text-muted" data-testid="measure-limits-how">
        They show once a session on it reports them.
      </span>
    );
  }
  if (door.status.state !== "on") {
    return (
      <span className="text-label text-muted" data-testid="measure-limits-off">
        Asking {toolName} for its limits is off here ·{" "}
        <button type="button" onClick={() => openTurnOn(door.folder)} className={LINK} data-testid="measure-limits-turn-on">
          Turn on ›<span className="sr-only"> — {name}</span>
        </button>
      </span>
    );
  }
  const measure = async () => {
    setMeasuring(true);
    setSaid(null);
    try {
      const after = await probeRun(door.folder);
      if (!alive.current) return;
      setDoor({ ...door, status: after });
      if (after.last_outcome !== "ran") {
        setSaid({ words: `Couldn't measure: ${probeOutcome(after.last_outcome, toolName)}.`, problem: true });
        return;
      }
      for (const ms of readAgainMs) {
        await wait(ms);
        if (!alive.current) return;
        await quotaDisplays.refresh();
        onMeasured?.();
      }
      // Still here: the reading hasn't come in yet.
      if (alive.current) setSaid({ words: `${toolName} answered; its limits show here once they're in.`, problem: false });
    } catch (cause) {
      if (alive.current) setSaid({ words: `Couldn't measure: ${errorText(cause)}`, problem: true });
    } finally {
      if (alive.current) setMeasuring(false);
    }
  };
  const label = measuring ? "Measuring…" : "Measure its limits";
  return (
    <span className="inline-flex flex-wrap items-baseline gap-x-2 gap-y-1" data-testid="measure-limits">
      {look === "button" ? (
        <Button size="sm" variant="secondary" disabled={measuring} aria-busy={measuring} aria-label={`${label} — ${name}`} onClick={() => void measure()} data-testid="measure-limits-run">
          {label}
        </Button>
      ) : (
        <button type="button" disabled={measuring} aria-busy={measuring} onClick={() => void measure()} className={`text-label ${LINK}`} data-testid="measure-limits-run">
          {label}
          {!measuring && " ›"}
          <span className="sr-only"> — {name}</span>
        </button>
      )}
      {said && (
        <output className={`text-label ${said.problem ? "text-warning" : "text-muted"}`} data-testid="measure-limits-said">
          {said.words}
        </output>
      )}
    </span>
  );
}
