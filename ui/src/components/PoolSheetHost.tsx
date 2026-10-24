import type { JSX } from "react";

import type { InstanceInfo, ProjectInfo } from "../lib/bridge";
import { lazyNamed } from "../lib/lazy";
import { usePoolSheet, usePools } from "../lib/pools";

// The sheet loads the first time it opens (lib/lazy).
const PoolSheet = lazyNamed(() => import("./PoolSheet"), "PoolSheet");

/** AJ-E1702: where the pool sheet opens, from any screen
 * (`openPoolSheet`): making a pool, moving a session, editing one. */
export function PoolSheetHost({ sessions, projects }: { sessions: InstanceInfo[]; projects: ProjectInfo[] }): JSX.Element | null {
  const target = usePoolSheet();
  const pools = usePools();
  if (!target || !pools) return null;
  return <PoolSheet target={target} pools={pools} sessions={sessions} projects={projects} />;
}
