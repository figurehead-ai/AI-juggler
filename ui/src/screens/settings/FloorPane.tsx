/**
 * AJ-E3511 (D-78): Settings › The Floor, as the approved wireframe draws
 * it — few choices with defaults that work, depth under More. Every
 * setting is a `floor.*` key; an open Floor reads a change at once. Apps
 * per floor moves areas on in order; Motion follows the system's Reduce
 * motion unless set; Re-create asks first (`RecreateSheet`).
 */
import { useEffect, useState, type JSX } from "react";
import { Button } from "../../components/primitives";
import { Disclosure, Segmented } from "../../components/controls";
import { showToast } from "../../components/Toast";
import { errorText } from "../../lib/bridge";
import { floorAppHiddenSet, floorRecreate, floorResetView, floorSettings, floorSettingSet, type FloorSettingsInfo } from "../../lib/ipc/floor-controls";
import { floorSnapshot, type FloorApp } from "../../lib/ipc/floor";
import { Pane, PaneSection, SettingRow, SwitchRow } from "./parts";
import { RecreateSheet } from "./RecreateSheet";

type Motion = "system" | "full" | "calm" | "off";
const MOTIONS: { value: Motion; label: string }[] = [
  { value: "system", label: "Follow the system" },
  { value: "full", label: "Full" },
  { value: "calm", label: "Calm" },
  { value: "off", label: "Off" },
];

export function FloorPane({ anchor }: { anchor?: string }): JSX.Element {
  const [settings, setSettings] = useState<FloorSettingsInfo | null>(null);
  const [apps, setApps] = useState<FloorApp[]>([]);
  const [asking, setAsking] = useState(false);
  useEffect(() => {
    void floorSettings().then(setSettings, () => setSettings(null));
    void floorSnapshot().then((snapshot) => setApps(snapshot.apps), () => setApps([]));
  }, []);
  const set = (key: string, value: string) =>
    void floorSettingSet(key, value).then(setSettings, (cause: unknown) => showToast({ id: "floor-setting", tone: "warning", message: errorText(cause) }));
  const nameOf = (id: string) => apps.find((app) => app.id === id)?.name ?? id;
  const perFloor = settings?.apps_per_floor ?? 10;
  return (
    <Pane id="the-floor" title="The Floor" anchor={anchor}>
      <p className="-mt-3 max-w-[70ch] text-meta text-ink-2">Your company as an office: every App an area, every session a person at a desk. It opens in its own window.</p>
      <PaneSection id="floor-opening" title="Opening">
        <SwitchRow
          title="Open the Floor when AI Juggler starts"
          description="Otherwise nothing of the Floor runs until you open it."
          checked={settings?.open_at_start ?? false}
          disabled={!settings}
          onChange={(on) => set("floor.open_at_start", on ? "on" : "off")}
          testId="floor-open-at-start"
        />
        <SwitchRow
          title="Keep it on top"
          description="Above other windows, on any display."
          checked={settings?.on_top ?? false}
          disabled={!settings}
          onChange={(on) => set("floor.on_top", on ? "on" : "off")}
          testId="floor-on-top"
        />
      </PaneSection>
      <PaneSection id="floor-office" title="The office">
        <SettingRow
          title="Apps per floor"
          description="Fewer Apps a floor, more floors. Areas keep their order: one that comes later moves on, one that comes earlier never moves."
          testId="floor-per-floor"
          control={
            <span className="flex items-center gap-1" role="group" aria-label="Apps per floor">
              <Button size="sm" variant="secondary" disabled={!settings || perFloor <= 4} aria-label="Fewer Apps per floor" onClick={() => set("floor.apps_per_floor", String(perFloor - 1))}>
                −
              </Button>
              <output className="w-8 text-center text-body tabular-nums text-ink" aria-live="polite" data-testid="floor-per-floor-value">
                {perFloor}
              </output>
              <Button size="sm" variant="secondary" disabled={!settings || perFloor >= 16} aria-label="More Apps per floor" onClick={() => set("floor.apps_per_floor", String(perFloor + 1))}>
                +
              </Button>
            </span>
          }
        />
        <SettingRow
          title="Motion"
          description="Full: people type and walk (8 frames a second, about 2% of one core). Calm: half as often, no one walks. Off: nothing moves; it draws only when something changes."
          testId="floor-motion"
          control={<Segmented label="Motion" value={(settings?.motion ?? "system") as Motion} segments={MOTIONS} onChange={(value) => set("floor.motion", value)} testId="floor-motion-choice" />}
        />
        <SwitchRow
          title="Show unused Apps' decay"
          description="An App unused for a week gathers dust and boxes; after a month its lights go off. Off: it only dims."
          checked={settings?.decay ?? true}
          disabled={!settings}
          onChange={(on) => set("floor.decay", on ? "on" : "off")}
          testId="floor-decay"
        />
        <SettingRow
          title="Reset the view"
          description="The camera back to the first floor, as when the Floor opens."
          testId="floor-reset-view"
          control={
            <Button size="sm" variant="secondary" onClick={() => void floorResetView().then(() => showToast({ id: "floor-reset", message: "The Floor is back on its first floor." }))}>
              Reset
            </Button>
          }
        />
        <SettingRow
          title="Re-create the office"
          description="A new look and layout for every area, in the same order."
          testId="floor-recreate"
          control={
            <Button size="sm" variant="secondary" onClick={() => setAsking(true)} data-testid="floor-recreate-open">
              Re-create…
            </Button>
          }
        />
      </PaneSection>
      {settings && settings.hidden.length > 0 && (
        <Disclosure summary={`More: Apps hidden from the Floor (${settings.hidden.length})`} testId="floor-hidden">
          <ul className="mt-2 flex flex-col divide-y divide-hairline/60 rounded-xl border border-hairline bg-surface">
            {settings.hidden.map((id) => (
              <li key={id} className="flex items-center gap-3 px-4 py-2.5" data-testid="floor-hidden-app">
                <span className="min-w-0 flex-1 truncate text-body text-ink">{nameOf(id)}</span>
                <Button size="sm" variant="secondary" onClick={() => void floorAppHiddenSet(id, false).then(setSettings)}>
                  Show on the Floor
                </Button>
              </li>
            ))}
          </ul>
        </Disclosure>
      )}
      {asking && (
        <RecreateSheet
          example={apps[0]?.name ?? null}
          onCancel={() => setAsking(false)}
          onConfirm={() => {
            setAsking(false);
            void floorRecreate().then(
              () => showToast({ id: "floor-recreated", message: "The office is re-created: every area has a new look, in the same order." }),
              (cause: unknown) => showToast({ id: "floor-recreated", tone: "warning", message: `Nothing changed: ${errorText(cause)}` }),
            );
          }}
        />
      )}
    </Pane>
  );
}
