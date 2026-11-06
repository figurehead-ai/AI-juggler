/**
 * Where the window is (AJ-E2018: moved out of App): the selected item or
 * the global screen, the links that open them, and the header's counts.
 */
import { useCallback, useEffect, useRef, useState } from "react";
import { fleetPeers } from "../lib/ipc/fleet";
import { statsEvent } from "../lib/ipc/usage-stats";
import { sessionTarget } from "../lib/ipc/conversations";
import { focusPageIfLost, useSettingsLinks, type GlobalId } from "../lib/nav";
import { useCleanupCount } from "../lib/cleanup-count";
import { useResource } from "../lib/use-resource";
import type { Selection } from "./selection";

/** AJ-E1103: the schema's screen names (History isn't counted; Home and
 * the retired Overview aren't either — the schema keeps the name). */
const STAT_SCREENS: Partial<Record<string, string>> = {
  accounts: "accounts",
  usage: "usage",
  fleet: "fleet",
  cleanup: "cleanup",
  settings: "settings",
};

export function useNavigation() {
  const [selection, setSelection] = useState<Selection>({ kind: "none" });
  const [global, setGlobal] = useState<GlobalId | null>(null);
  // F11: a page opened from something that went with the old page (not
  // the first one: the window opens with nothing focused, the skip link first).
  const shownPage = useRef(JSON.stringify([selection, global]));
  useEffect(() => {
    const page = JSON.stringify([selection, global]);
    if (page === shownPage.current) return;
    shownPage.current = page;
    focusPageIfLost();
  }, [selection, global]);
  // D-25: Fleet surfaces only when it's relevant. AJ-E2404: the pill
  // counts what is left to clean up, the Processes view's own count (the
  // same as This Mac's), and hears of every change.
  const cleanupCount = useCleanupCount();
  const pairedCount = useResource("fleet-peers", fleetPeers, { pollMs: 30_000 }).data?.length ?? 0;
  // AJ-E1103: which screens get opened (and the Cleanup Centre).
  useEffect(() => {
    const screen = global ? STAT_SCREENS[global] : undefined;
    if (screen) statsEvent("ui.screen_opened", { screen });
    if (global === "cleanup") statsEvent("cleanup_centre.opened");
  }, [global]);
  // AJ-E1203: a Cloud chip / the onboarding card land on Account & Cloud.
  const [settingsFocusCloud, setSettingsFocusCloud] = useState(false);
  // D-25: a link elsewhere (Accounts' pack-secrets line) opens one section.
  const [settingsSection, setSettingsSection] = useState<string | undefined>(undefined);
  // AJ-E1524: Home (the welcome while no account is in).
  const openHome = () => {
    setGlobal(null);
    setSelection({ kind: "none" });
  };
  const openCloudSettings = () => {
    setSettingsFocusCloud(true);
    setGlobal("settings");
  };
  /** Settings opens at its top again (no section, not Cloud). */
  const clearSettingsFocus = () => {
    setSettingsFocusCloud(false);
    setSettingsSection(undefined);
  };
  /** Settings at its top (⌘,, the palette). */
  const openSettings = () => {
    clearSettingsFocus();
    setGlobal("settings");
  };
  const openToolInSettings = useSettingsLinks(global, setGlobal, setSettingsSection, () => setSettingsFocusCloud(false));
  // AJ-E1010: the update notice's Review lands on Settings.
  useEffect(() => {
    const open = (event: Event) => {
      setSettingsFocusCloud(false);
      setSettingsSection((event as CustomEvent<{ section?: string } | undefined>).detail?.section);
      setGlobal("settings");
    };
    window.addEventListener("aij:open-settings", open);
    return () => window.removeEventListener("aij:open-settings", open);
  }, []);
  // F7: ⌘K's "Search History for …" opens History searching for it.
  const [historyQuery, setHistoryQuery] = useState("");
  useEffect(() => void (global !== "history" && setHistoryQuery("")), [global]);

  const selectSession = useCallback((id: string) => {
    setGlobal(null);
    setSelection(sessionTarget(id));
  }, []);
  const selectProject = useCallback((id: string) => {
    setGlobal(null);
    setSelection({ kind: "project", id });
  }, []);
  const selectLogin = useCallback((id: string) => {
    setGlobal(null);
    setSelection({ kind: "login", id });
  }, []);
  const inspectRemote = (machineId: string, instanceId: string | null) => {
    setGlobal(null);
    setSelection({ kind: "remote", machineId, instanceId });
  };
  /** An item's own page (the work area), leaving any global screen. */
  const openPage = useCallback((next: Selection) => {
    setGlobal(null);
    setSelection(next);
  }, []);

  return {
    selection, setSelection, global, setGlobal, cleanupCount, pairedCount,
    settingsFocusCloud, settingsSection, openHome, openCloudSettings, clearSettingsFocus, openSettings, openToolInSettings,
    historyQuery, setHistoryQuery, selectSession, selectProject, selectLogin, inspectRemote, openPage,
  };
}
