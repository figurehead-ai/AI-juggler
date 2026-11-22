/**
 * ADR-0035 §9 (AJ-E2023) — `useRemote(section)`: a section of the remote
 * configuration, typed by its name (`useRemote("flags")?.flags.notices_on`).
 *
 * Lazy: nothing is read until a view asks. Cached: one answer per section,
 * shared by every view and painted at once when a view opens again. Kept
 * current by the backend's `remote-config://changed` (no timer): every
 * section read so far is read again then. `null` until the first answer.
 */
import { useEffect } from "react";
import { sharedRead, useSharedRead, whenForgotten, type SharedRead } from "./shared-read";
import { isSection, onRemoteChanged, remoteValues, type RemoteSection, type RemoteValuesInfo, type RemoteValuesOf } from "./ipc/remote";

/** Kept until the backend says it changed. */
const KEPT = Number.POSITIVE_INFINITY;

/** One shared read per section (a new section is a compile error here). */
const READS: { [S in RemoteSection]: SharedRead<RemoteValuesInfo> } = {
  flags: sharedRead(() => remoteValues("flags")),
};

const asked = new Set<RemoteSection>();

let stop: (() => void) | null = null;
function listen(): void {
  if (stop) return;
  stop = onRemoteChanged(() => {
    for (const section of asked) void READS[section].refresh();
  });
  whenForgotten(() => {
    stop?.();
    stop = null;
    asked.clear();
  });
}

/** `section`'s values in effect (`null` until read). */
export function useRemote<S extends RemoteSection>(section: S): RemoteValuesOf<S> | null {
  useEffect(() => {
    asked.add(section);
    listen();
  }, [section]);
  const values = useSharedRead(READS[section], { maxAgeMs: KEPT });
  return values !== undefined && isSection(values, section) ? values : null;
}
