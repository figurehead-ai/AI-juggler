import { useFolderDrop } from "../lib/use-folder-drop";
import { useEscapeCloses } from "../lib/use-escape-closes";
import { useEffect, useMemo, useRef, useState, type JSX } from "react";
import { Button } from "../components/primitives";
import { IconProjects } from "../components/icons";
import { IconClose } from "../components/icons-controls";
import { IconFolderOpen } from "../components/icons-actions";
import { Alert } from "../components/Alert";
import { Section } from "../components/Section";
import { LogoTile } from "../components/ToolLogo";
import { funnelStep } from "../lib/funnel";
import { FirstRun } from "./FirstRun";
import { launchRecipesList } from "../lib/ipc/launch";
import { pickDirectory } from "../lib/ipc/project-folders";
import { agoWords } from "../lib/time-words";
import type { LaunchRecipeInfo } from "../types/generated/LaunchRecipeInfo";
import type { ProjectInfo } from "../types/generated/ProjectInfo";
import type { AccountInfo } from "../types/generated/AccountInfo";
import type { QuotaDisplayData } from "../lib/ipc/quota";
import { pickLaunch } from "../lib/launch-pick";
import { DialogFrame, DialogOverlay } from "../components/DialogFrame";
import { SELECTED_FILL } from "../components/controls/selected";

/**
 * AJ-E305 flow (b): a new project in ≤ 3 clicks and zero typed paths.
 *
 * 0.5.1 F68: on D-142's dialog frame (size M: an icon tile, the title and
 * one help line in the fixed header, the body the choice, Cancel in the
 * footer), and it chooses only what is its own: where. A recent folder,
 * the folder picker or a folder dropped on it opens New session on that
 * folder, which chooses the App as it always does (its stated choice with
 * its why, ⌘↩ or Start): one way to pick an App, never a second, older
 * one here (the critique: a bare App row, "quota unknown", the primary
 * mid-body). With no App yet, the add-an-app detour opens over this sheet
 * and, once an App is there, carries on to New session on the same
 * folder (Req.7).
 */

/** The preselect rule (Req.6, AJ-E1501): `lib/launch-pick`, the one
 * New session uses (a pick of yours, else the most room by the one order
 * by room). */
export function pickRecipe(
  recipes: LaunchRecipeInfo[],
  project: ProjectInfo | null,
  quotas: QuotaDisplayData[],
  accounts: AccountInfo[] = [],
): LaunchRecipeInfo | null {
  return pickLaunch(recipes, project, quotas, accounts)?.recipe ?? null;
}

/** A recent folder's line: the App last used there and when. */
function lastUsed(row: ProjectInfo, now: number): string {
  const app = row.accounts_used[row.accounts_used.length - 1];
  const when = row.last_activity_ms ? agoWords(row.last_activity_ms, now) : "";
  if (app) return when ? `Last used with ${app} · ${when}` : `Last used with ${app}`;
  return when ? `Worked in ${when}` : "Not worked in yet";
}

export function NewProjectSheet({
  projects,
  initialDirectory,
  onClose,
  onFolder,
  openLoginTerminal,
  onAccountsChanged,
}: {
  projects: ProjectInfo[];
  accounts: AccountInfo[];
  initialDirectory: string | null;
  onClose: () => void;
  /** F68: the folder chosen: New session opens on it. */
  onFolder: (directory: string) => void;
  openLoginTerminal: (id: string, title: string, options?: { select?: boolean; account?: string }) => void;
  onAccountsChanged: () => void;
}): JSX.Element {
  const [directory, setDirectory] = useState<string | null>(initialDirectory);
  const [recipes, setRecipes] = useState<LaunchRecipeInfo[] | null>(null);
  const [detour, setDetour] = useState(false);
  const [now] = useState(() => Date.now());
  // Esc closes it wherever focus is, even right after ⌘⇧N (as New session, #198).
  // While the add-app detour is open its own Escape leaves it and returns
  // here, the folder still chosen (Req.7): this sheet's listener, added
  // first, must not close the sheet under it.
  useEscapeCloses(() => {
    if (!detour) onClose();
  });
  const tilesRef = useRef<HTMLDivElement | null>(null);

  const reload = () => {
    void launchRecipesList().then(setRecipes);
  };
  useEffect(() => {
    funnelStep("b-new-project", "open");
    void launchRecipesList().then(setRecipes);
  }, []);

  // A folder chosen with an App to start it: New session takes it from here.
  const ready = directory !== null && recipes !== null && recipes.length > 0;
  useEffect(() => {
    if (ready && directory !== null) onFolder(directory);
  }, [ready, directory, onFolder]);

  const chooseDirectory = (dir: string) => {
    funnelStep("b-new-project", "pick-dir");
    setDirectory(dir);
  };
  const { dragging, zone } = useFolderDrop(chooseDirectory); // wf03: a dropped folder

  // Arrow keys move focus within the folders; Enter and Space choose.
  const onTilesKey = (event: React.KeyboardEvent) => {
    const tiles = Array.from(tilesRef.current?.querySelectorAll<HTMLButtonElement>("button[data-tile]") ?? []);
    const index = tiles.findIndex((tile) => tile === document.activeElement);
    if (index === -1) return;
    const forward = event.key === "ArrowRight" || event.key === "ArrowDown";
    const back = event.key === "ArrowLeft" || event.key === "ArrowUp";
    if (!forward && !back) return;
    event.preventDefault();
    const next = forward ? (index + 1) % tiles.length : (index - 1 + tiles.length) % tiles.length;
    tiles[next]?.focus();
  };

  const recents = useMemo(() => [...projects].sort((a, b) => (b.last_activity_ms ?? 0) - (a.last_activity_ms ?? 0)).slice(0, 6), [projects]);
  const name = directory?.split("/").pop() ?? null;
  const noApp = directory !== null && recipes !== null && recipes.length === 0;

  return (
    <DialogOverlay className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4 backdrop-blur-sm">
      <DialogFrame
        size="M"
        className="rounded-card border border-hairline bg-surface shadow-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="new-project-title"
        headerClassName="flex items-start gap-3 px-6 pt-5 pb-4"
        header={
          <>
            <LogoTile icon={IconProjects} size={40} on />
            <div className="flex min-w-0 flex-1 flex-col gap-0.5">
              <h2 id="new-project-title" className="type-section">
                New project
              </h2>
              <p className="type-help">Choose its folder. New session then picks the app, as it always does.</p>
            </div>
            <Button variant="ghost" size="sm" type="button" onClick={onClose} aria-label="Close" title="Close (Esc)" className="-mr-2 -mt-1 size-7 shrink-0 px-0 text-ink-2">
              <IconClose aria-hidden="true" width={24} height={24} strokeWidth={2} className="shrink-0" />
            </Button>
          </>
        }
        bodyClassName={"flex flex-col gap-5 px-6 pt-1 pb-6 " + (dragging ? "bg-accent-soft/40" : "")}
        bodyTestId="new-project-drop"
        bodyProps={zone}
        footerClassName="flex flex-wrap items-center justify-end gap-2 border-t border-hairline px-6 py-3"
        footer={
          <>
            <span className="mr-auto text-meta text-ink-2" data-testid="new-project-where">
              {name ? `Starting in ${name}` : "Or drop a folder anywhere on this window."}
            </span>
            <Button variant="ghost" type="button" onClick={onClose} aria-keyshortcuts="Escape">
              Cancel
            </Button>
          </>
        }
      >
        {noApp && !detour && (
          <Alert
            level="needs-you"
            title="No app set up yet"
            testId="no-account-detour"
            actions={
              <Button size="sm" variant="primary" type="button" onClick={() => setDetour(true)}>
                Add an app
              </Button>
            }
          >
            Add one, and {name} opens in New session with it, the folder kept.
          </Alert>
        )}
        <Section title="Where it lives" help={recents.length > 0 ? "A recent folder, or another one" : undefined} testId="project-where">
          {/* Arrows move across the folders and Choose folder… alike. */}
          <div ref={tilesRef} onKeyDown={onTilesKey} className="flex flex-col gap-2.5">
            <div className="grid grid-cols-1 gap-2.5 sm:grid-cols-2" data-testid="project-tiles">
              {recents.map((row) => (
                <button
                  key={row.id}
                  type="button"
                  data-tile
                  aria-current={directory === row.root ? "true" : undefined}
                  onClick={() => chooseDirectory(row.root)}
                  className={"flex min-w-0 items-center gap-3 rounded-card px-3 py-2.5 text-left transition-colors " + (directory === row.root ? SELECTED_FILL : "bg-surface-2 hover:bg-surface-3")}
                >
                  <LogoTile icon={IconProjects} size={32} />
                  <span className="flex min-w-0 flex-col">
                    {/* WCAG 2.5.3: the name is the tile's own words. */}
                    <span className="truncate text-body font-semibold text-ink">
                      <span className="sr-only">Work in </span>
                      {row.name}
                    </span>
                    <span className="truncate text-meta text-muted">{lastUsed(row, now)}</span>
                  </span>
                </button>
              ))}
            </div>
            <Button
              variant="secondary"
              type="button"
              data-tile
              className="self-start"
              onClick={async () => {
                const picked = await pickDirectory();
                if (picked) chooseDirectory(picked);
              }}
            >
              <IconFolderOpen aria-hidden="true" width={16} height={16} />
              Choose folder…
            </Button>
          </div>
        </Section>
      </DialogFrame>

      {/* The inline flow (d) detour: once an app is there, New session
          opens on the folder chosen (Req.7). */}
      {detour && (
        <FirstRun
          mode="add"
          onDone={() => setDetour(false)}
          onAccountsChanged={() => {
            onAccountsChanged();
            reload();
          }}
          openLoginTerminal={openLoginTerminal}
        />
      )}
    </DialogOverlay>
  );
}
