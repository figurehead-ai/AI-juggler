/**
 * D-62 (AJ-E3203) — on a desktop App's page, the one ask: "Find this
 * app's other profiles". It says exactly what is read (the names of the
 * folders in one folder, nothing inside them) and where, once; the answer
 * is kept, and Settings › Desktop apps turns it off again. A yes says
 * what it found.
 */
import { useState, type JSX } from "react";
import { Button, Card } from "../../components/primitives";
import { errorText } from "../../lib/bridge";
import type { AppModel } from "../../lib/apps";
import { askFolder, findNow, foundWords, useFindProfiles } from "../../lib/find-profiles";

export function FindProfiles({ app }: { app: AppModel }): JSX.Element | null {
  const { info, answer } = useFindProfiles();
  const [said, setSaid] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const folder = app.kind === "desktop" ? askFolder(info, app.appId) : null;
  if (said !== null) {
    return (
      <Card className="flex flex-wrap items-center justify-between gap-3 p-4" data-testid="app-find-profiles-done">
        <span className="min-w-0 flex-1 text-body text-ink" role="status">
          {said}
        </span>
        <Button size="sm" variant="ghost" onClick={() => setSaid(null)}>
          OK
        </Button>
      </Card>
    );
  }
  if (folder === null || app.appId === null) return null;
  const appId = app.appId;
  const yes = () => {
    setBusy(true);
    findNow(answer, appId).then(
      (names) => setSaid(foundWords(names)),
      (cause) => setSaid(`Couldn't look: ${errorText(cause)}`),
    );
  };
  const no = () => {
    setBusy(true);
    answer(false).catch((cause) => setSaid(`Couldn't keep your answer: ${errorText(cause)}`));
  };
  return (
    <Card className="flex flex-wrap items-center justify-between gap-x-6 gap-y-3 p-4" data-testid="app-find-profiles">
      <span className="flex min-w-0 max-w-[68ch] flex-1 flex-col gap-1">
        <span className="text-body font-medium text-ink">Find this app's other profiles</span>
        <span className="text-meta text-ink-2">
          {app.app} keeps each profile in a folder of its own, in <span className="break-all font-mono text-ink">{folder}</span>. AI Juggler can read the names of the folders there, and nothing inside them, so the profiles that aren't running show too.
        </span>
        <span className="text-meta text-muted">You're asked once. Settings › Desktop apps can turn it off later.</span>
      </span>
      <span className="flex shrink-0 items-center gap-2">
        <Button size="sm" variant="ghost" onClick={no} disabled={busy} data-testid="app-find-profiles-no">
          No thanks
        </Button>
        <Button size="sm" variant="secondary" onClick={yes} disabled={busy} data-testid="app-find-profiles-yes">
          Find them
        </Button>
      </span>
    </Card>
  );
}
