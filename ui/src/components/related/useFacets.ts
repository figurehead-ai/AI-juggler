/**
 * E31 (AJ-E3103) — what a related list's filters offer, read only once
 * its filters show (a list past 8 rows): the projects, Apps, Accounts and
 * Macs its rows are on, each with how many rows it shows (the backend's
 * `sessions_facets`, the same rows the list pages through; the design
 * preview counts its own rows). Each value is named the way the rest of
 * AI Juggler names it: a project by its name, an App by its name (D-62),
 * an Account by its name, a Mac by its Fleet label.
 */
import { useEffect, useMemo, useState } from "react";
import { fleetPeers, fleetSelf } from "../../lib/ipc/fleet";
import { hasLiveBackend } from "../../lib/bridge";
import type { AccountInfo } from "../../types/generated/AccountInfo";
import type { ProjectInfo } from "../../types/generated/ProjectInfo";
import { sessionsCounts, sessionsFacets, type SessionsQuery } from "../../lib/ipc/sessions";
import { useApps } from "../../lib/apps";
import { accountName, useSubscriptions } from "../../lib/subscriptions";
import { RECENT_MS, facetsOf, type Belongs, type Facet, type FacetKind, type StateFilter } from "./order";
import { THIS_MACHINE } from "../../lib/machine-words";

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

/** The values, `null` until read; `wanted` false reads nothing. */
export function useFacets(key: string, query: SessionsQuery, local: () => Belongs[], wanted: boolean, version: number): FacetsByKind | null {
  const live = hasLiveBackend();
  const [facets, setFacets] = useState<{ key: string; value: FacetsByKind } | null>(null);
  const full = `${key}#${version}`;
  useEffect(() => {
    if (!wanted) return;
    if (!live) {
      setFacets({ key: full, value: facetsOf(local()) });
      return;
    }
    let alive = true;
    let pending = read.get(full);
    if (!pending) {
      pending = sessionsFacets(query).then((info) => ({ project: info.projects, app: info.apps, account: info.subscriptions, machine: info.machines }));
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
    const accountNames = new Map((accounts ?? []).flatMap((row) => (row.account_id ? [[row.account_id, accountName(row)] as [string, string]] : [])));
    return (kind, id) => {
      switch (kind) {
        case "project":
          return id === "" ? "No project" : (projectNames.get(id) ?? "A moved project");
        case "app":
          return appNames.get(id) ?? id.replace(/^(cli|app):/, "");
        case "account":
          return id === "" ? "No account" : (accountNames.get(id) ?? "An Account");
        default:
          return macs.get(id) ?? (id === "this-mac" ? THIS_MACHINE : id);
      }
    };
  }, [projects, apps, accounts, macs]);
}
