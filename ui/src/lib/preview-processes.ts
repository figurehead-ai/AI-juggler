/**
 * The preview's process trees (no backend): what the Processes tab and a
 * session's header read for the preview's sessions, kept in agreement
 * with them (AJ-E1533: one source per fact).
 */
import type { ProcessesInfo } from "../types/generated/ProcessesInfo";

// AJ-E410 mock trees: a 3-level live tree (the §7.3 wireframe shape)
// plus a 2-orphan section — one fresh, one already `gone`.
export const MOCK_PROCESSES: Record<string, ProcessesInfo> = {
  // AJ-E1533: the preview's live session — the same numbers its header
  // shows (CPU 3.5% across the tree, two helpers).
  "sess-live-1": {
    rows: [
      { pid: 61001, parent_pid: 60900, name: "claude", cpu_pct: 2.5, rss_bytes: 410_000_000, age_ms: 20 * 60_000, child_count: 2, depth: 0 },
      { pid: 61020, parent_pid: 61001, name: "node (mcp)", cpu_pct: 0.5, rss_bytes: 90_000_000, age_ms: 19 * 60_000, child_count: 0, depth: 1 },
      { pid: 61044, parent_pid: 61001, name: "npm test", cpu_pct: 0.5, rss_bytes: 140_000_000, age_ms: 2 * 60_000, child_count: 0, depth: 1 },
    ],
    spark: Array.from({ length: 24 }, (_, i) => ({
      at_ms: Date.now() - (24 - i) * 2_000,
      cpu_pct: 2.5 + ((i * 5) % 3) / 2,
      rss_bytes: 640_000_000,
    })),
    orphans: [],
    unavailable_reason: null,
  },
  "sess-8": {
    rows: [
      { pid: 41201, parent_pid: 40100, name: "claude", cpu_pct: 11.0, rss_bytes: 890_000_000, age_ms: 42 * 60_000, child_count: 2, depth: 0 },
      { pid: 41233, parent_pid: 41201, name: "node (mcp)", cpu_pct: 1.0, rss_bytes: 96_000_000, age_ms: 40 * 60_000, child_count: 1, depth: 1 },
      { pid: 41302, parent_pid: 41233, name: "rg (search)", cpu_pct: 4.0, rss_bytes: 120_000_000, age_ms: 12_000, child_count: 0, depth: 2 },
    ],
    spark: Array.from({ length: 24 }, (_, i) => ({
      at_ms: Date.now() - (24 - i) * 2_000,
      cpu_pct: 6 + ((i * 7) % 9),
      rss_bytes: 980_000_000 + ((i * 13) % 5) * 20_000_000,
    })),
    orphans: [
      { session_id: "sess-8", pid: 39871, name: "node dev-server", rss_bytes: 210_000_000, parent_label: "claude", age_since_parent_exit_ms: 18 * 60_000, stop_state: null },
      { session_id: "sess-8", pid: 39890, name: "vite", rss_bytes: 88_000_000, parent_label: "claude", age_since_parent_exit_ms: 18 * 60_000, stop_state: "gone" },
    ],
    unavailable_reason: null,
  },
  "sess-2": {
    rows: [
      { pid: 51001, parent_pid: 50900, name: "claude", cpu_pct: 3.0, rss_bytes: 240_000_000, age_ms: 9 * 60_000, child_count: 0, depth: 0 },
    ],
    spark: Array.from({ length: 24 }, (_, i) => ({
      at_ms: Date.now() - (24 - i) * 2_000,
      cpu_pct: 2 + ((i * 3) % 4),
      rss_bytes: 240_000_000,
    })),
    orphans: [],
    unavailable_reason: null,
  },
};
