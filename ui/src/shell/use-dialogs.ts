/**
 * What the shell opens over the work area (AJ-E2018: moved out of App):
 * the sheets, dialogs and flows, the palette, and the inspector's fold;
 * and what paired machines report.
 */
import { keepIfSame } from "../lib/same";
import { everyWhileShown } from "../lib/shown-window";
import { useCallback, useEffect, useState } from "react";
import { fleetRemote } from "../lib/ipc/fleet-remote";
import { providersDiscover } from "../lib/ipc/providers";
import { servicesList } from "../lib/ipc/services";
import type { PortWarningInfo } from "../lib/ipc/kill";
import type { RecoverySummaryData } from "../lib/ipc/cleanup";
import { useFirstRun } from "../lib/first-run";
import { useNewSessionSheet } from "../lib/new-session-sheet";
import { toolFound } from "../lib/providers";
import { readInspectorFolded, storeInspectorFolded } from "../lib/shortcuts";
import { primaryToolId } from "../lib/tools";
import { ADD_APP } from "../lib/found-tools";
import type { RemoteCommandOutcomeInfo } from "../types/generated/RemoteCommandOutcomeInfo";
import type { RemoteMachineInfo } from "../types/generated/RemoteMachineInfo";

/** A switch to run (the freeze dialog → stages → source-safe banner). */
export type SwitchFlowRequest = { sessionId: string; targetProfileId: string; targetLabel: string };

export function useDialogs() {
  const newSheet = useNewSessionSheet();
  // D-25: Accounts › Add account reuses first run's steps, for this tool
  // (AJ-E1023: chosen first only when more than one is installed).
  const [addAccounts, setAddAccounts] = useState<string | null>(null);
  // AJ-E1524: the welcome's "Add a fresh account" opens on that step.
  const [addFresh, setAddFresh] = useState(false);
  const [chooseTool, setChooseTool] = useState(false);
  const startAddAccount = useCallback((fresh = false) => {
    setAddFresh(fresh);
    void Promise.all([providersDiscover(), fresh ? [] : servicesList().catch(() => [])]).then(
      ([rows, services]) => {
        // Found counts, even when its version couldn't be read (0.0.11).
        const ready = rows.filter(toolFound);
        // AJ-E1601: the chooser's More offers a service a found tool runs through.
        const more = services.some((row) => row.tools.some((tool) => ready.some((found) => found.provider === tool.tool)));
        if (ready.length > 1 || more) setChooseTool(true);
        else setAddAccounts(ready[0]?.provider ?? primaryToolId() ?? "");
      },
      () => setAddAccounts(primaryToolId() ?? ""),
    );
  }, []);
  // D-97 (1): Home's Next steps opens one found tool's Add flow, by id.
  useEffect(() => {
    const open = (event: Event) => {
      const detail: unknown = event instanceof CustomEvent ? event.detail : null;
      const tool = typeof detail === "object" && detail !== null && "tool" in detail && typeof detail.tool === "string" ? detail.tool : "";
      if (!tool) return;
      setAddFresh(false);
      setAddAccounts(tool);
    };
    window.addEventListener(ADD_APP, open);
    return () => window.removeEventListener(ADD_APP, open);
  }, []);
  const [accountsNonce, setAccountsNonce] = useState(0);
  // D-25: the account-limit decision, for this session id.
  const [limitFor, setLimitFor] = useState<string | null>(null);
  // The tool chip leaves the header when its view opens: focus follows.
  const [inspectorFocus, setInspectorFocus] = useState(0);
  // AJ-E205: the executing switch flow. Null = closed.
  const [switchFlow, setSwitchFlow] = useState<SwitchFlowRequest | null>(null);
  // AJ-E207: the switch-back review modal. Null = closed; suggest-only.
  const [switchBack, setSwitchBack] = useState<string | null>(null);
  // AJ-E505: the pre-launch port collision (advisor informs, never
  // blocks — Launch always proceeds on "Launch anyway").
  const [portWarning, setPortWarning] = useState<{
    warning: PortWarningInfo;
    proceed: () => void;
  } | null>(null);
  // AJ-E414: the "Go to instance…" palette — summoned by the global
  // hotkey (backend event) or in-app ⌘K; the binding is rebindable.
  const [goToOpen, setGoToOpen] = useState(false);
  const [goToHotkey, setGoToHotkey] = useState("");
  // D-25: a fresh install opens on first run (what's here → sign in →
  // computer access); a fresh account and a folder import are its steps.
  const [firstRun, setFirstRun, firstRunAtAccess, setFirstRunAtAccess] = useFirstRun();
  // AJ-E305 flow (b): the New project sheet. Null = closed; a string
  // pre-selects that directory (the Summary's Start here).
  const [newProjectDir, setNewProjectDir] = useState<string | null>(null);
  // AJ-E510: the ONE-time crash-recovery summary — shown until
  // dismissed; a second relaunch (no new findings) shows nothing.
  const [recovery, setRecovery] = useState<RecoverySummaryData | null>(null);
  useEffect(() => {
    void import("../lib/ipc/cleanup").then((bridge) => bridge.recoverySummaryGet()).then(setRecovery, () => setRecovery(null));
  }, []);
  // AJ-E711: the last palette-started remote focus, shown in Actions.
  const [remoteFocusOutcome, setRemoteFocusOutcome] = useState<{
    machineId: string;
    instanceId: string;
    outcome: RemoteCommandOutcomeInfo;
  } | null>(null);
  // AJ-E1504: the `?` sheet, and the inspector folded to one line
  // (⌘⇧I), remembered for this window across launches.
  const [shortcutsOpen, setShortcutsOpen] = useState(false);
  const [inspectorFolded, setInspectorFolded] = useState(() => readInspectorFolded());
  const toggleInspector = useCallback(() => {
    setInspectorFolded((folded) => {
      storeInspectorFolded(!folded);
      return !folded;
    });
  }, []);
  /** New project, keeping a directory already chosen. */
  const newProject = () => setNewProjectDir((current) => current ?? "");

  return {
    newSheet, addAccounts, setAddAccounts, addFresh, chooseTool, setChooseTool, startAddAccount,
    accountsNonce, setAccountsNonce, limitFor, setLimitFor, inspectorFocus, setInspectorFocus,
    switchFlow, setSwitchFlow, switchBack, setSwitchBack, portWarning, setPortWarning,
    goToOpen, setGoToOpen, goToHotkey, setGoToHotkey, firstRun, setFirstRun, firstRunAtAccess, setFirstRunAtAccess,
    newProjectDir, setNewProjectDir, newProject, recovery, setRecovery, remoteFocusOutcome, setRemoteFocusOutcome,
    shortcutsOpen, setShortcutsOpen, inspectorFolded, toggleInspector,
  };
}

/** AJ-E705: what paired machines report, polled while one is selected. */
export function useRemoteMachines(remoteSelected: boolean) {
  const [remoteMachines, setRemoteMachines] = useState<RemoteMachineInfo[] | null>(null);
  const [remoteNow, setRemoteNow] = useState(() => Date.now());
  useEffect(() => {
    let alive = true;
    const load = async () => {
      try {
        const list = await fleetRemote();
        if (alive) {
          setRemoteMachines((was) => keepIfSame(was, list));
          // the "seen … ago" clock, only while there's a Mac to time
          if (list.length > 0) setRemoteNow(Date.now());
        }
      } catch {
        // A slow or offline peer never blocks the board.
      }
    };
    // A local read of what peers reported: fast while one is open; only
    // while the window is shown (0.4.1, idle CPU).
    const stop = everyWhileShown(() => void load(), remoteSelected ? 2000 : 5000, { now: true });
    return () => {
      alive = false;
      stop();
    };
  }, [remoteSelected]);
  return { remoteMachines, remoteNow, setRemoteNow };
}
