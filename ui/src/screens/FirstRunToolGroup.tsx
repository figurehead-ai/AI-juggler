/**
 * AJ-E1545 step 1, "Your apps": one group per tool, its header the tool's
 * own icon (read at run time from the installed app; a CLI-only tool its
 * vendor's app icon, else a terminal glyph: `ToolLogo`), its name, how
 * many were found, and a select-all for the group.
 */
import { useEffect, useId, useState, type JSX, type ReactNode } from "react";
import { placesPairings } from "../lib/ipc/places";
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
}

/** Each desktop profile's folder, by path: the folder's row becomes the
 * profile's ("Claude app · work", Desktop). Read once; never paired by
 * name (the backend's evidence only). */
export function usePairings(on: boolean): Map<string, Paired> {
  const [pairs, setPairs] = useState<Map<string, Paired>>(new Map());
  useEffect(() => {
    if (!on) return;
    void placesPairings().then(
      (rows) =>
        setPairs(
          new Map(
            rows.flatMap((row) => {
              const tool = row.folder.tool;
              const app = tool ? toolSpec(tool).desktop_app : null;
              if (!row.folder.path || !tool || !app) return [];
              return [[row.folder.path, { title: `${desktopAppWord(app, tool)} · ${row.label}`, certain: row.certain }] as const];
            }),
          ),
        ),
      () => null,
    );
  }, [on]);
  return pairs;
}
