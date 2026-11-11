/**
 * The app's working set (AJ-E2018: moved out of App): projects, sessions,
 * accounts and launch settings, read together and again on every change
 * the backend tells of, with each session's freshness.
 */
import { useCallback, useEffect, useMemo, useState } from "react";
import { accountsList } from "../lib/ipc/accounts";
import { publishAgentTools } from "../lib/agent-tools";
import { fetchAppInfo, onAppReady } from "../lib/ipc/app-lifecycle";
import { setReservedKeys } from "../lib/shortcut-bindings";
import { onConversationsChanged } from "../lib/ipc/conversations";
import { instancesList, onSessionsUpdated } from "../lib/ipc/instances";
import { launchRecipesList } from "../lib/ipc/launch";
import { projectsBackfill, projectsList } from "../lib/ipc/project-folders";
import { onTerminalExit } from "../lib/ipc/terminal-io";
import { statsEvent } from "../lib/ipc/usage-stats";
import { switchBackSuggestions, type SwitchBackSuggestionData } from "../lib/ipc/switchBack";
import { backendAnswered, backendFailed } from "../lib/backend-health";
import { singleFlight, useLiveDeltas, type Freshness } from "../lib/live-deltas";
import { withMarks } from "../lib/palette";
import { withFolderMarks } from "../lib/marks";
import { useWithSelected } from "../lib/session-lookup";
import { staleSubscriptions } from "../lib/subscriptions";
import type { AccountInfo } from "../types/generated/AccountInfo";
import type { AppInfo } from "../types/generated/AppInfo";
import type { InstanceInfo } from "../types/generated/InstanceInfo";
import type { LaunchRecipeInfo } from "../types/generated/LaunchRecipeInfo";
import type { ProjectInfo } from "../types/generated/ProjectInfo";
import type { Selection } from "./selection";

/** AJ-E1103: the board became interactive once, after launch. */
let boardInteractiveCounted = false;

/** A sign-in's terminal, in the Sessions list until closed. */
export type LoginTerminal = { id: string; title: string; ended: boolean };

export function useAppData(selection: Selection, openPage: (next: Selection) => void) {
  // Freshness per Instance id: when each value class was last sampled.
  // Seeded at list load and refreshed by the resource/quota ticks; the
  // `__AIJ_SAMPLED_AT__` hook exists for the visual fixtures (stale-row
  // captures) exactly like `__AIJ_FIXTURE__`.
  const [freshness, setFreshness] = useState<Freshness>({});
  const baselineFreshness = useCallback((ids: string[]): Freshness => {
    const forced = (window as unknown as { __AIJ_SAMPLED_AT__?: Record<string, number> })
      .__AIJ_SAMPLED_AT__;
    const at = Date.now();
    const map: Freshness = {};
    for (const id of ids) {
      map[id] = {
        resources: forced?.[id] ?? at,
        quota: forced?.[id] ?? at,
      };
    }
    return map;
  }, []);
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [projects, setProjects] = useState<ProjectInfo[]>([]);
  // The working set; a session past it joins while selected (session-lookup).
  const [workingSessions, setSessions] = useState<InstanceInfo[]>([]);
  const sessions = useWithSelected(workingSessions, selection.kind === "session" ? selection.id : null);
  // AJ-E1007/E1502: archived Instances, and a forgotten project and its (ended)
  // sessions, leave the rail and New session until it's used again; History keeps them.
  const shownProjects = useMemo(() => projects.filter((project) => !project.forgotten), [projects]);
  const railSessions = useMemo(() => {
    const forgotten = new Set(projects.filter((project) => project.forgotten).map((project) => project.id));
    return sessions.filter((row) => row.lifecycle !== "archived" && !(row.project_id !== null && forgotten.has(row.project_id)));
  }, [sessions, projects]);
  const [accounts, setAccounts] = useState<AccountInfo[]>([]);
  const [profiles, setProfiles] = useState<LaunchRecipeInfo[]>([]);
  // AJ-E1503 Req.4: until the first load answers, lists show row-shaped
  // placeholders, never a false "nothing here".
  const [loaded, setLoaded] = useState(false);
  const [loginTerminals, setLoginTerminals] = useState<LoginTerminal[]>([]);
  // AJ-E207: the suggested returns (reset-trigger cards).
  const [switchBackCards, setSwitchBackCards] = useState<SwitchBackSuggestionData[]>([]);

  useEffect(() => {
    // AJ-E2905 R7: the system's reserved keys come with the app info.
    const take = (next: AppInfo | null): void => {
      if (next) setReservedKeys(next.reserved_keys);
      setInfo(next);
    };
    void fetchAppInfo().then(take);
    return onAppReady(take);
  }, []);

  const refresh = useMemo(() => singleFlight(async () => {
    try {
      const [projectRows, sessionRows, accountRows, profileRows, backSuggestions] =
        await Promise.all([
          projectsList(),
          instancesList(),
          accountsList(),
          launchRecipesList(),
          switchBackSuggestions().catch(() => []),
        ]);
      setProjects(withMarks(projectRows)); // AJ-E1532: each project its mark
      setSessions(sessionRows);
      setAccounts(withFolderMarks(accountRows)); // AJ-E3206: each App's and Account's colour, each folder its initials
      publishAgentTools(accountRows); // AJ-E3002 R3: which Apps' sessions get the agent tools
      setProfiles(profileRows);
      setSwitchBackCards(backSuggestions);
      if (!boardInteractiveCounted) {
        boardInteractiveCounted = true;
        statsEvent("perf.board_interactive_ms", {}, performance.now());
      }
      setFreshness((current) => {
        // Keep known sample times; new rows get the load baseline.
        const baseline = baselineFreshness(sessionRows.map((row) => row.id));
        const next: Freshness = { ...baseline };
        for (const [id, sampled] of Object.entries(current)) {
          if (next[id]) next[id] = sampled;
        }
        return next;
      });
      backendAnswered();
    } catch (cause) {
      // AJ-E2504 (§3, §8): never swallowed — the last data stays, the window
      // says the service stopped answering, and the cause goes to the console.
      console.warn("AI Juggler: the refresh failed", cause);
      backendFailed();
    } finally {
      setLoaded(true);
    }
  }), [baselineFreshness]);

  // An account added, renamed, signed in or removed: the app's lists now,
  // and the accounts' sign-ins on the next view of them.
  const accountsChanged = useCallback(() => {
    staleSubscriptions();
    void refresh();
  }, [refresh]);

  // Day one: the REAL project history from imported transcripts (an idempotent
  // scan); AJ-E1529: again when the files change (not when only what's open did).
  useEffect(() => {
    void projectsBackfill().then(() => void refresh());
    return onConversationsChanged((change) => change === "index" && void refresh());
  }, [refresh]);

  useEffect(() => {
    void refresh();
    return onSessionsUpdated(() => void refresh());
  }, [refresh]);

  // AJ-E402/E401: the 5 s resource and 30 s quota ticks, merged by id.
  useLiveDeltas(setSessions, setFreshness);

  // Login terminals end → mark ended (kept for scrollback until closed).
  useEffect(() => {
    return onTerminalExit((id) => {
      setLoginTerminals((current) =>
        current.map((t) => (t.id === id ? { ...t, ended: true } : t)),
      );
      void refresh();
    });
  }, [refresh]);

  // Login flows (Accounts screen) hand their terminal here; it joins the
  // Sessions rail list and gets selected.
  const openLoginTerminal = useCallback((id: string, title: string, options?: { select?: boolean }) => {
    // A sign-in may change what the accounts read says: the next view of
    // them reads it again.
    staleSubscriptions();
    setLoginTerminals((current) =>
      current.some((t) => t.id === id) ? current : [...current, { id, title, ended: false }],
    );
    if (options?.select === false) return; // Add account shows it in its own step
    openPage({ kind: "login", id }); // the panel must show the terminal, not stay on Accounts
  }, [openPage]);

  return {
    info, projects, sessions, shownProjects, railSessions, accounts, profiles, loaded, freshness,
    loginTerminals, switchBackCards, refresh, accountsChanged, openLoginTerminal,
  };
}
