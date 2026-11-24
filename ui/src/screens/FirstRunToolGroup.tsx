/**
 * AJ-E1545 step 1, "Your apps": one group per tool, its header the tool's
 * own icon (read at run time from the installed app; a CLI-only tool its
 * vendor's app icon, else a terminal glyph: `ToolLogo`), its name, how
 * many were found, and a select-all for the group.
 */
import { useEffect, useId, useState, type JSX, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { onPlacesChanged, placesOfferAnswer, placesPairings } from "../lib/ipc/places";
import { errorText } from "../lib/bridge";
import type { DiscoveredInfo } from "../types/generated/DiscoveredInfo";
import type { RowProgress } from "./FirstRunFoundRow";
import { Checkbox } from "../components/controls";
import { ToolLogo } from "../components/ToolLogo";
import { providerDisplayName, toolFound } from "../lib/providers";
import type { ProviderInstallData } from "../lib/ipc/providers";
import { cliWords } from "./FirstRunMore";

export function ToolGroup({
  tool,
  title,
  testId,
  cliTestId,
  count,
  install,
  all,
  some,
  disabled,
  onAll,
  children,
}: {
  tool: string;
  /** Add app's groups by kind of setup (its spec's names); else the tool's. */
  title?: string;
  testId?: string;
  cliTestId?: string;
  count: number;
  install: ProviderInstallData | undefined;
  /** Every row ticked; some ticked (the box shows a dash). */
  all: boolean;
  some: boolean;
  disabled: boolean;
  onAll: (on: boolean) => void;
  children: ReactNode;
}): JSX.Element {
  const id = useId().replace(/:/g, "");
  const name = title ?? providerDisplayName(tool);
  return (
    <fieldset className="border-b border-hairline last:border-b-0" data-testid={testId ?? `first-run-group-${tool}`}>
      <legend className="sr-only">{name}</legend>
      <div className="flex items-center justify-between gap-3 bg-surface-2 px-5 py-2.5">
        <span className="flex min-w-0 items-center gap-2.5">
          <ToolLogo tool={tool} size={24} />
          <span className="text-body font-semibold text-ink">{name}</span>
          <span className="text-meta text-muted">· {count}</span>
          {install && toolFound(install) && <span className="hidden truncate text-meta text-muted md:inline" data-testid={cliTestId}>{cliWords(providerDisplayName(tool), install)}</span>}
        </span>
        <label htmlFor={`${id}-all`} className="flex shrink-0 cursor-pointer items-center gap-2 text-meta text-ink-2">
          <Checkbox id={`${id}-all`} checked={all} indeterminate={some && !all} disabled={disabled} onChange={(event) => onAll(event.target.checked)} data-testid={`first-run-all-${tool}`} />
          All <span className="sr-only">{name}</span>
        </label>
      </div>
      {children}
    </fieldset>
  );
}

/** A found folder's desktop profile, by AJ-E2327's evidence. */
export interface Paired {
  /** "Claude app · work". */
  title: string;
  /** Evidence says so for sure; else the row asks "Is this right?". */
  certain: boolean;
  /** The profile's place id (AJ-E2327's "Choose its folder…" takes it). */
  id: string;
  /** It's one of the user's Apps already (a folder chosen before is
   * kept until its Add links it). */
  added: boolean;
}

/** A desktop profile as the backend pairs it: its tool's id, the folder
 * evidence pairs it with (full path, as the scan lists it), and whether
 * it runs now. */
export interface DesktopProfile extends Paired {
  tool: string;
  path: string | null;
  /** The paired folder's own name, for the words. */
  folderName: string | null;
  running: boolean;
}

/** Every desktop profile known (Apps and offers), paired by AJ-E2327's
 * evidence; read again when a place changes (a folder chosen for one
 * moves its row). Never paired by name: the backend's evidence, or the
 * user's own pick, only. */
export function usePairings(on: boolean): DesktopProfile[] {
  const [rows, setRows] = useState<DesktopProfile[]>([]);
  useEffect(() => {
    if (!on) return undefined;
    let live = true;
    const read = () =>
      void placesPairings().then(
        (all) => {
          if (!live) return;
          setRows(
            all.flatMap((row) =>
              row.tool ? [{ id: row.id, title: row.label, certain: row.certain, added: row.added, tool: row.tool, path: row.path, folderName: row.folder.name, running: row.running }] : [],
            ),
          );
        },
        () => null,
      );
    read();
    const stop = onPlacesChanged(read);
    return () => {
      live = false;
      stop();
    };
  }, [on]);
  return rows;
}

/** One path as another spelling of it would be (macOS's `/tmp` is
 * `/private/tmp`; a trailing slash). */
const samePath = (a: string, b: string) => {
  const plain = (path: string) => path.replace(/\/+$/, "").replace(/^\/private(?=\/(?:tmp|var|etc)\/)/, "");
  return plain(a) === plain(b);
};

/** The progress key of a desktop profile brought in on its own row. */
export const desktopKey = (id: string) => `desktop:${id}`;

/** AJ-E1545 R2 (0.5.1 F35): one row per thing the user recognises. A
 * profile paired with a folder on the list is that folder's row
 * (`pairs`, by its path); any other is a row of its own in its tool's
 * group (`alone`), its folder already in AI Juggler or not found. Only
 * `tools`' profiles. */
export function pairRows(profiles: readonly DesktopProfile[], installs: readonly DiscoveredInfo[], tools: readonly string[]): { pairs: Map<string, Paired>; alone: DesktopProfile[] } {
  const pairs = new Map<string, Paired>();
  const alone: DesktopProfile[] = [];
  for (const profile of profiles) {
    if (!tools.includes(profile.tool)) continue;
    const path = profile.path;
    const found = path === null ? undefined : installs.find((one) => one.harness === profile.tool && samePath(one.path, path));
    // A folder in AI Juggler already can't be ticked: its profile, not
    // added yet, keeps a row of its own to tick.
    if (found && !pairs.has(found.path) && !(found.managed && !profile.added)) pairs.set(found.path, profile);
    else alone.push(profile);
  }
  return { pairs, alone };
}

/** A folder's paired profile, not added yet, added with it (its row's
 * tick and Bring in: D-84), as its Add does. */
export async function addPaired(pair: Paired | undefined): Promise<void> {
  if (pair && !pair.added) await placesOfferAnswer(pair.id, true);
}

/** D-84: the user's own tick and Bring in add each profile, as its Add
 * does (its folder linked by evidence and brought in where it is). Each
 * one's progress as it goes; stops before the next once `stop` is set. */
export async function bringInProfiles(ids: readonly string[], stop: { current: boolean }, mark: (key: string, progress: NonNullable<RowProgress>) => void): Promise<{ failed: number; stopped: boolean }> {
  let failed = 0;
  for (const id of ids) {
    if (stop.current) return { failed, stopped: true };
    mark(desktopKey(id), { state: "working" });
    try {
      await placesOfferAnswer(id, true);
      mark(desktopKey(id), { state: "done" });
    } catch (cause) {
      failed += 1;
      mark(desktopKey(id), { state: "failed", error: errorText(cause) });
    }
  }
  return { failed, stopped: false };
}

/** The found step's actions: in the setup wizard's sticky foot when it
 * gives one (AJ-E1545, DLG-1), its links in the middle and its primary
 * at the end, after the wizard's Skip this step (0.5.1 F50, BTN-3); else
 * a bar that stays in view at the bottom of the page (0.4 design review:
 * Bring in never below the fold), the primary last. */
export function FoundActions({
  slot,
  end,
  note,
  primary,
  children,
}: {
  slot?: HTMLElement | null;
  end?: HTMLElement | null;
  /** A line beside the primary (in the wizard, at the links' end). */
  note?: ReactNode;
  primary?: ReactNode;
  /** The links. */
  children: ReactNode;
}): JSX.Element {
  if (slot && end) {
    return (
      <>
        {createPortal(
          <div className="flex flex-1 flex-wrap items-center justify-between gap-4" data-testid="first-run-actions">
            {children}
            {note}
          </div>,
          slot,
        )}
        {primary && createPortal(primary, end)}
      </>
    );
  }
  return (
    <div className="sticky bottom-0 z-10 -mx-2 flex flex-wrap items-center justify-between gap-4 border-t border-hairline bg-canvas/95 px-2 py-3 backdrop-blur" data-testid="first-run-actions">
      {children}
      {(note || primary) && (
        <div className="flex items-center gap-3">
          {note}
          {primary}
        </div>
      )}
    </div>
  );
}
