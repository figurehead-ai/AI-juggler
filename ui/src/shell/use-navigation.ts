/**
 * Where the window is (AJ-E2018: moved out of App): the selected item or
 * the global screen, the links that open them, and the header's counts.
 */
import { useCallback, useEffect, useRef, useState } from "react";
import { fleetPeers } from "../lib/ipc/fleet";
import { statsEvent } from "../lib/ipc/usage-stats";
import { sessionTarget } from "../lib/ipc/conversations";
import { focusPageIfLost, globalId, useSettingsLinks, type GlobalId } from "../lib/nav";
import { useCleanupCount } from "../lib/cleanup-count";
import { useResource } from "../lib/use-resource";
import { emptyHistory, goBack, goForward, visit, type NavHistory } from "../lib/nav-history";
import type { Selection } from "./selection";
import { landingOf, type Landing } from "../lib/landing";

/** A page the window was on: what was selected, or a global screen. */
type Page = { selection: Selection; global: GlobalId | null };
const samePage = (a: Page, b: Page) => a.global === b.global && JSON.stringify(a.selection) === JSON.stringify(b.selection);

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
  // Walk A3: Back and Forward (⌘[ ⌘], the top bar's ‹ ›). Each page
  // change joins the history, except the ones Back and Forward make.
  const [history, setHistory] = useState<NavHistory<Page>>(emptyHistory);
  const here = useRef<Page>({ selection, global });
  const moving = useRef(false);
  useEffect(() => {
    const page = { selection, global };
    if (samePage(page, here.current)) return;
    const left = here.current;
    here.current = page;
    if (moving.current) {
      moving.current = false;
      return;
    }
    setHistory((now) => visit(now, left));
  }, [selection, global]);
  const latest = useRef(history);
  useEffect(() => {
    latest.current = history;
  }, [history]);
  const move = useCallback((step: typeof goBack) => {
    const next = step(latest.current, here.current);
    if (!next) return;
    // A page that's this one (it can't be, but then nothing moves and
    // the next real change must still join the history).
    if (!samePage(next.to, here.current)) moving.current = true;
    setHistory(next.history);
    setSelection(next.to.selection);
    setGlobal(next.to.global);
  }, []);
  const back = useCallback(() => move(goBack), [move]);
  const forward = useCallback(() => move(goForward), [move]);
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
  // 0.4.2: the view a link named on the screen it opened (Accounts'
  // "out", Processes' Needs attention), by its id (lib/landing): kept
  // while that screen is open, so it lands the same however it opened.
  const [landing, setLanding] = useState<Landing | null>(null);
  // Dropped as the window leaves its screen (React's adjust-while-rendering,
  // not an effect: the screen never draws once with a stale view).
  const [landingOn, setLandingOn] = useState(global);
  if (landingOn !== global) {
    setLandingOn(global);
    if (landing && landing.id !== global) setLanding(null);
  }
  /** Screen `id` on its view `filter`, from a click here (the top bar's). */
  const openOn = useCallback((id: GlobalId, filter: string) => {
    setLanding({ id, filter });
    setGlobal(id);
  }, []);
  // The rail's "Search all history for …" (persona D3, rc33): History,
  // searching for its words (the screen itself opens on the same event).
  useEffect(() => {
    const open = (event: Event) => {
      const detail: unknown = event instanceof CustomEvent ? event.detail : null;
      if (typeof detail !== "object" || detail === null || !("id" in detail)) return;
      // A screen opened without a view named opens as it was left.
      if (globalId(detail.id)) setLanding(landingOf(detail));
      if (detail.id !== "history") return;
      if ("query" in detail && typeof detail.query === "string") setHistoryQuery(detail.query);
    };
    window.addEventListener("aij:open-screen", open);
    return () => window.removeEventListener("aij:open-screen", open);
  }, []);

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
    historyQuery, setHistoryQuery, landing, setLanding, openOn, selectSession, selectProject, selectLogin, inspectRemote, openPage,
    back, forward, canBack: history.back.length > 0, canForward: history.forward.length > 0,
  };
}
