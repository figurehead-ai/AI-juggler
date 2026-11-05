/**
 * AJ-E3511, AJ-E3512 (D-78): Settings › The Floor and the Floor list —
 * the Floor's settings (each a `floor.*` key; an open Floor reads a change
 * at once), Re-create the office, Reset the view, an App left off the
 * Floor or put back, each App's floor, and Show on the Floor. Without a
 * backend the preview keeps them in memory.
 */
import { invoke } from "./transport";
import { hasLiveBackend } from "../bridge";
import type { FloorSettingsInfo } from "../../types/generated/FloorSettingsInfo";
import type { FloorPlaceInfo } from "../../types/generated/FloorPlaceInfo";

export type { FloorPlaceInfo, FloorSettingsInfo };
type Places = FloorPlaceInfo[];

const preview: { settings: FloorSettingsInfo } = {
  settings: { open_at_start: false, on_top: false, apps_per_floor: 10, motion: "system", decay: true, hidden: [] },
};
const seam = () => (window as unknown as { __AIJ_FLOOR_SETTINGS__?: FloorSettingsInfo; __AIJ_FLOOR_PLACES__?: Places });

export async function floorSettings(): Promise<FloorSettingsInfo> {
  if (!hasLiveBackend()) return { ...(seam().__AIJ_FLOOR_SETTINGS__ ?? preview.settings) };
  return invoke<FloorSettingsInfo>("floor_settings");
}

const WORDS: Record<string, keyof FloorSettingsInfo> = {
  "floor.open_at_start": "open_at_start",
  "floor.on_top": "on_top",
  "floor.apps_per_floor": "apps_per_floor",
  "floor.motion": "motion",
  "floor.decay": "decay",
};

/** Sets `key` (a `floor.*` key) to `value` ("on"/"off", 4–16, or a motion word). */
export async function floorSettingSet(key: string, value: string): Promise<FloorSettingsInfo> {
  if (!hasLiveBackend()) {
    const field = WORDS[key];
    if (!field) throw new Error(`The Floor has no setting ${key}.`);
    const current = { ...(seam().__AIJ_FLOOR_SETTINGS__ ?? preview.settings) } as Record<string, unknown>;
    current[field] = field === "apps_per_floor" ? Number(value) : field === "motion" ? value : value === "on";
    preview.settings = current as FloorSettingsInfo;
    seam().__AIJ_FLOOR_SETTINGS__ = preview.settings;
    return { ...preview.settings };
  }
  return invoke<FloorSettingsInfo>("floor_setting_set", { key, value });
}

/** Re-create the office: a new look for every area, the same order. */
export async function floorRecreate(): Promise<void> {
  if (!hasLiveBackend()) return;
  return invoke<void>("floor_recreate");
}

/** Leaves an App off the Floor, or puts it back. */
export async function floorAppHiddenSet(appId: string, hidden: boolean): Promise<FloorSettingsInfo> {
  if (!hasLiveBackend()) {
    const current = seam().__AIJ_FLOOR_SETTINGS__ ?? preview.settings;
    preview.settings = { ...current, hidden: hidden ? [...current.hidden, appId] : current.hidden.filter((id) => id !== appId) };
    seam().__AIJ_FLOOR_SETTINGS__ = preview.settings;
    return { ...preview.settings };
  }
  return invoke<FloorSettingsInfo>("floor_app_hidden_set", { appId, hidden });
}

/** Each App's floor, in the Floor's order. */
export async function floorPlaces(): Promise<FloorPlaceInfo[]> {
  if (!hasLiveBackend()) return seam().__AIJ_FLOOR_PLACES__ ?? [];
  return invoke<Places>("floor_places");
}

/** Reset the view: an open Floor back on its first floor. */
export async function floorResetView(): Promise<void> {
  if (!hasLiveBackend()) return;
  return invoke<void>("floor_reset_view");
}

/** Show on the Floor: opens it on the floor holding `appId`'s area. */
export async function floorShowApp(appId: string): Promise<void> {
  if (!hasLiveBackend()) throw new Error("The Floor opens in the app, not in the preview.");
  return invoke<void>("floor_show_app", { appId });
}
