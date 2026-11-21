/**
 * E31 (AJ-E3103) — what a list's filters offer (History's and every
 * related list's), read only once its filters show: the projects, Apps,
 * Accounts, Macs and tools its rows are on, each with how many rows it
 * shows with the other filters as chosen (the backend's `sessions_facets`,
 * the same rows the list pages through; the design preview counts its own
 * rows the same way). Each value is named the way the rest of AI Juggler
 * names it: a project by its name, an App by its name (D-62), an Account
 * by its name, a Mac by its Fleet label, a tool by its name.
 */
import { useEffect, useMemo, useState } from "react";
import { fleetPeers, fleetSelf } from "../../lib/ipc/fleet";
import { hasLiveBackend } from "../../lib/bridge";
import type { AccountInfo } from "../../types/generated/AccountInfo";
import type { ProjectInfo } from "../../types/generated/ProjectInfo";
import { sessionsCounts, sessionsFacets, type SessionsQuery } from "../../lib/ipc/sessions";
import { useApps } from "../../lib/apps";
import { accountNamesApart, useSubscriptions } from "../../lib/subscriptions";
import { FACET_KINDS, RECENT_MS, facetFilters, narrowedFacets, type Belongs, type Facet, type FacetKind, type Filters, type StateFilter } from "./order";
import { THIS_MACHINE } from "../../lib/machine-words";
import { providerDisplayName } from "../../lib/providers";

export type FacetsByKind = Record<FacetKind, Facet[]>;

/** Read once per list and state (and search), kept for the window's life. */
const read = new Map<string, Promise<FacetsByKind>>();
/** The State filter's counts, once per list and search. */
const stateRead = new Map<string, Promise<StateCounts>>();

/** Forgets what was read (a session started or ended, tests). */
export function forgetFacets(): void {
  read.clear();
  stateRead.clear();
}

/** How many rows each State value shows. */
export type StateCounts = Record<Exclude<StateFilter, "all">, number>;

/** Walk K2 (AJ-E3306): the State filter's "· N", as every other filter
 * has, over the list in its search (as the others are over the list in
 * its state and search): Open now and Ended from the list's counts,
 * Recent from its counts since `opened` less 7 days (the rows the Recent
 * filter asks for). `base` is the list's query with no state; `local`
 * counts the design preview's rows. `null` until read. */
export function useStateCounts(key: string, base: SessionsQuery, opened: number, local: () => StateCounts, wanted: boolean, version: number): StateCounts | null {
  const live = hasLiveBackend();
  const [counts, setCounts] = useState<{ key: string; value: StateCounts } | null>(null);
  const full = `${key}#${version}`;
  useEffect(() => {
    let alive = true;
    if (wanted && !live) setCounts({ key: full, value: local() });
    else if (wanted) {
      let pending = stateRead.get(full);
      if (!pending) {
        const whole = { ...base, which: null, since_ms: null };
        pending = Promise.all([sessionsCounts(whole), sessionsCounts({ ...whole, since_ms: opened - RECENT_MS })]).then(([all, recent]) => ({ open: all.open, recent: recent.ended, ended: all.ended }));
        stateRead.set(full, pending);
        pending.catch(() => stateRead.delete(full));
      }
      pending.then(
        (value) => alive && setCounts({ key: full, value }),
        () => null,
      );
    }
    return () => {
      alive = false;
    };
    // The key names the query.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [full, wanted, live]);
  return counts?.key === full ? counts.value : null;
}

/** The chosen values in a key ("*" for none; "" is "No project"). */
export function chosenKey(filters: Filters): string {
  return FACET_KINDS.map((kind) => filters[kind] ?? "*").join("|");
}

/** The values over `scope` (the list's own entity, state and search),
 * each kind's counted with the other kinds' choices in `filters` (the
 * 0.4.6 RC walk K2, F-K2-5: App = Codex CLI chosen, Tool still offered
 * Claude Code); `local(filters)`: the preview's rows those choices show.
 * `null` until read; `wanted` false reads nothing. */
export function useFacets(key: string, scope: SessionsQuery, filters: Filters, local: (filters: Filters) => Belongs[], wanted: boolean, version: number): FacetsByKind | null {
  const live = hasLiveBackend();
  const [facets, setFacets] = useState<{ key: string; value: FacetsByKind } | null>(null);
  const full = `${key}|${chosenKey(filters)}#${version}`;
  useEffect(() => {
    if (!wanted) return;
    if (!live) {
      setFacets({ key: full, value: narrowedFacets(local, filters) });
      return;
    }
    let alive = true;
    let pending = read.get(full);
    if (!pending) {
      pending = sessionsFacets(scope, facetFilters(filters)).then((info) => ({ project: info.projects, app: info.apps, account: info.subscriptions, machine: info.machines, tool: info.tools }));
      read.set(full, pending);
      pending.catch(() => read.delete(full));
    }
    pending.then(
      (value) => alive && setFacets({ key: full, value }),
      () => null,
    );
    return () => {
      alive = false;
    };
    // The key names the query.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [full, wanted, live]);
  return facets?.key === full ? facets.value : null;
}

/** The Macs' names from Fleet (read only when a list's rows are on more
 * than one). */
function useMacNames(wanted: boolean): Map<string, string> {
  const [names, setNames] = useState<Map<string, string>>(new Map());
  useEffect(() => {
    if (!wanted) return;
    let alive = true;
    void Promise.all([fleetSelf(), fleetPeers()]).then(
      ([self, peers]) => alive && setNames(new Map([[self.machine_id, THIS_MACHINE], ...peers.map((peer) => [peer.machine_id, peer.label] as [string, string])])),
      () => null,
    );
    return () => {
      alive = false;
    };
  }, [wanted]);
  return names;
}

/** Names each kind's values: `(kind, id) → words`. */
export function useFacetNames(folders: readonly AccountInfo[], projects: readonly ProjectInfo[], facets: FacetsByKind | null): (kind: FacetKind, id: string) => string {
  const apps = useApps(folders);
  const accounts = useSubscriptions(0, facets !== null && facets.account.length > 1);
  const macs = useMacNames(facets !== null && facets.machine.length > 1);
  return useMemo(() => {
    const projectNames = new Map(projects.map((project) => [project.id, project.name]));
    const appNames = new Map((apps ?? []).map((app) => [app.id, app.name]));
    const named = accountNamesApart(accounts ?? []);
    const accountNames = new Map((accounts ?? []).flatMap((row) => (row.account_id ? [[row.account_id, named(row)] as [string, string]] : [])));
    return (kind, id) => {
      switch (kind) {
        case "project":
          return id === "" ? "No project" : (projectNames.get(id) ?? "A moved project");
        case "app":
          return appNames.get(id) ?? id.replace(/^(cli|app):/, "");
        case "account":
          return id === "" ? "No account" : (accountNames.get(id) ?? "An Account");
        case "tool":
          return providerDisplayName(id);
        default:
          return macs.get(id) ?? (id === "this-mac" ? THIS_MACHINE : id);
      }
    };
  }, [projects, apps, accounts, macs]);
}
