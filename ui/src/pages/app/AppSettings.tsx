/**
 * D-62 (AJ-E3204) — an App's Settings: its name (yours, or the default),
 * then what it runs with. A desktop App: Start and Keep alive, the same
 * row as Settings › Desktop apps. A CLI App: its folder's settings
 * (tidy-up and the window preview), as its folder page had. Plan, renewal
 * and fee are its Account's (AJ-E3202 req. 4): shown here, set there; a
 * folder with no Account yet keeps its own editors.
 */
import { useState, type JSX } from "react";
import { Button } from "../../components/primitives";
import { RenameSheet } from "../../components/ManageWork";
import { PaneSection, SettingRow } from "../../screens/settings/parts";
import { PlaceRow } from "../../screens/settings/DesktopAppsPane";
import { AccountSettingsSection } from "../account/Settings";
import { AccountCapture } from "../AccountCapture";
import { appsRename } from "../../lib/ipc/apps";
import type { AppModel } from "../../lib/apps";
import { useSubscriptions } from "../../lib/subscriptions";
import { AppTerms, accountForApp } from "./AppTerms";

export function AppSettings({ app, now, resetsAtMs = null }: { app: AppModel; now: number; resetsAtMs?: number | null }): JSX.Element {
  const [renaming, setRenaming] = useState(false);
  const subs = useSubscriptions();
  const sub = subs ? accountForApp(subs, app.folder?.id ?? null, app.folder?.subscription_id ?? app.accountId) : null;
  return (
    <div className="flex w-full max-w-content flex-col gap-grid" data-testid="app-settings">
      <PaneSection id="app-name" title="Name">
        <SettingRow
          title={app.name}
          description={app.named ? `Your name for ${app.defaultName}.` : "The name AI Juggler gives it: its app, then its profile."}
          testId="app-name-row"
          control={
            <>
              {app.named && (
                <Button size="sm" variant="ghost" onClick={() => void appsRename(app.id, null)} data-testid="app-name-reset">
                  Use the default
                </Button>
              )}
              <Button size="sm" variant="secondary" onClick={() => setRenaming(true)} data-testid="app-rename">
                Rename
              </Button>
            </>
          }
        />
      </PaneSection>
      {app.place && (
        <PaneSection id="app-running" title="Start and keep alive">
          <PlaceRow place={app.place} name={app.name} />
        </PaneSection>
      )}
      {sub && subs && <AppTerms sub={sub} all={subs} now={now} />}
      {app.kind === "cli" && app.folder && <AccountSettingsSection account={app.folder} now={now} resetsAtMs={resetsAtMs} termsOnAccount={sub !== null} />}
      {/* AJ-E1530 Req.1: opt-in, shown only where it applies, last. */}
      {app.kind === "cli" && app.folder && <AccountCapture accountId={app.folder.id} />}
      {renaming && (
        <RenameSheet
          title={`Rename ${app.name}`}
          initial={app.named ? app.name : ""}
          placeholder={app.defaultName}
          hint="Shown on its page and wherever it's named. Empty goes back to the default."
          testId="app-rename-sheet"
          onSave={async (name) => {
            await appsRename(app.id, name.trim() || null);
            setRenaming(false);
          }}
          onClose={() => setRenaming(false)}
        />
      )}
    </div>
  );
}
