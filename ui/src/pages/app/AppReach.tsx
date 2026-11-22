/**
 * AJ-E1610 (D-66, D-105, D-106) — what a desktop App's app allows from AI
 * Juggler, from its adapter's data (`KnownPlaceInfo.reach`): "Open here"
 * by the app's own documented links (a new chat, a new session in a
 * project), sent to this profile's running app alone; and, for each part
 * with no sound way in, one plain line saying why, so the page never
 * promises what the app doesn't allow.
 *
 * Calm (D-117): a quiet section at the foot of the Overview; ghost
 * buttons (the page's one filled button stays the header's); the lines in
 * the grey hierarchy with a quiet glyph, no tint, no border.
 */
import type { JSX } from "react";
import { Section } from "../../components/charts/ChartCard";
import { Button } from "../../components/primitives";
import { IconInfo, IconPlus } from "../../components/icons-actions";
import { showGoTo } from "../../lib/go-to";
import { showToast } from "../../components/Toast";
import { errorText } from "../../lib/bridge";
import { placesOpenRoute } from "../../lib/ipc/places";
import type { AppModel } from "../../lib/apps";

/** A project to open a new session in: its name and folder. */
export type ReachFolder = { name: string; root: string };

export function AppReach({ app, folder }: { app: AppModel; folder: ReachFolder | null }): JSX.Element | null {
  const place = app.place;
  const reach = place?.reach ?? null;
  if (!place || !reach) return null;
  const running = place.app_pid !== null;
  const send = (action: string, root: string | null): void => {
    placesOpenRoute(place.id, action, root).then(
      (said) => showGoTo(said),
      (error: unknown) => showToast({ id: "go-to", message: errorText(error), tone: "error" }),
    );
  };
  const chat = reach.open_here.includes("new_chat");
  const inFolder = reach.open_here.includes("new_in_folder") && folder !== null;
  const doors = chat || inFolder;
  if (!doors && reach.track_only.length === 0) return null;
  return (
    <Section title="What works from here" icon={<IconInfo />} testId="app-reach">
      {doors && (
        <div className="flex flex-wrap items-center gap-2" data-testid="app-reach-open">
          {chat && (
            <Button size="sm" variant="ghost" disabled={!running} onClick={() => send("new_chat", null)} data-testid="app-reach-new-chat">
              <IconPlus className="h-3.5 w-3.5" aria-hidden="true" />
              New chat
            </Button>
          )}
          {inFolder && (
            <Button size="sm" variant="ghost" disabled={!running} onClick={() => send("new_in_folder", folder.root)} data-testid="app-reach-new-in-folder">
              <IconPlus className="h-3.5 w-3.5" aria-hidden="true" />
              New session in {folder.name}
            </Button>
          )}
          {!running && <span className="text-meta text-ink-2">Start it first.</span>}
        </div>
      )}
      {reach.track_only.length > 0 && (
        <ul className="flex flex-col gap-1.5" data-testid="app-reach-limits">
          {reach.track_only.map((track) => (
            <li key={track.part} className="flex items-start gap-2 text-meta text-ink-2" data-testid={`app-reach-why-${track.part}`}>
              <IconInfo className="mt-0.5 h-3.5 w-3.5 shrink-0 text-muted" aria-hidden="true" />
              <span>{track.why}</span>
            </li>
          ))}
        </ul>
      )}
    </Section>
  );
}
