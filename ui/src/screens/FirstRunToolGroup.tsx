/**
 * AJ-E1545 step 1, "Your apps": one group per tool, its header the tool's
 * own icon (read at run time from the installed app; a CLI-only tool its
 * vendor's app icon, else a terminal glyph: `ToolLogo`), its name, how
 * many were found, and a select-all for the group.
 */
import { useEffect, useId, useState, type JSX, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { onPlacesChanged, placesPairings } from "../lib/ipc/places";
import { desktopAppWord } from "../lib/app-name";
import { toolSpec } from "../lib/tools";
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

/** Each desktop profile's folder, by path: the folder's row becomes the
 * profile's ("Claude app · work", Desktop). Read again when a place
 * changes (a folder chosen for one moves its row); never paired by name
 * (the backend's evidence, or the user's own pick, only). */
export function usePairings(on: boolean): Map<string, Paired> {
  const [pairs, setPairs] = useState<Map<string, Paired>>(new Map());
  useEffect(() => {
    if (!on) return undefined;
    let live = true;
    const read = () =>
      void placesPairings().then(
        (rows) => {
          if (!live) return;
          setPairs(
            new Map(
              rows.flatMap((row) => {
                const tool = row.folder.tool;
                const app = tool ? toolSpec(tool).desktop_app : null;
                if (!row.folder.path || !tool || !app) return [];
                return [[row.folder.path, { title: `${desktopAppWord(app, tool)} · ${row.label}`, certain: row.certain, id: row.id, added: row.added }] as const];
              }),
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
  return pairs;
}

/** The found step's actions: in the setup wizard's sticky foot when it
 * gives one (AJ-E1545, DLG-1), else a bar that stays in view at the
 * bottom of the page (0.4 design review: Bring in never below the fold). */
export function FoundActions({ slot, children }: { slot?: HTMLElement | null; children: ReactNode }): JSX.Element {
  if (slot) {
    return createPortal(
      <div className="flex flex-1 flex-wrap items-center justify-between gap-4" data-testid="first-run-actions">
        {children}
      </div>,
      slot,
    );
  }
  return (
    <div className="sticky bottom-0 z-10 -mx-2 flex flex-wrap items-center justify-between gap-4 border-t border-hairline bg-canvas/95 px-2 py-3 backdrop-blur" data-testid="first-run-actions">
      {children}
    </div>
  );
}
