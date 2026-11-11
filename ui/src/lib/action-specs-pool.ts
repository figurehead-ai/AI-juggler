/**
 * AJ-E2807 — a pool's menu (its card on the Pools screen): open it,
 * rename it, delete it. A project's own pool is its project's: it can't
 * be renamed or deleted, and its menu says why.
 */
import type { ActionSpec } from "./action-types";
import { has } from "./action-specs";
import type { PoolInfo } from "../types/generated/PoolInfo";

const PROJECTS_OWN = (pool: PoolInfo) => (pool.kind === "user" ? null : "A project's own pool goes with its project");

export const POOL: ActionSpec<"pool">[] = [
  { id: "open", label: "Open its canvas", icon: "page", group: "go", rank: 10, applies: (_, h) => has(h.openPool), run: (pool, h) => h.openPool!(pool.id) },
  { id: "rename", label: "Rename…", icon: "rename", group: "work", rank: 10, applies: (_, h) => has(h.renamePool), blocked: PROJECTS_OWN, run: (pool, h) => h.renamePool!(pool) },
  { id: "delete", label: "Delete…", icon: "delete", group: "stop", rank: 10, applies: (_, h) => has(h.deletePool), blocked: PROJECTS_OWN, run: (pool, h) => h.deletePool!(pool) },
];
