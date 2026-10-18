import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { MinimalInstanceColumn, SessionRow } from "./WorkflowRail";
import { rememberResourceSample } from "./indicator";
import type { InstanceInfo } from "../lib/bridge";
import {
  DENSITY_ORDER,
  isDensityShortcut,
  nextDensity,
  readDensity,
  staleCaption,
  staleMinutes,
  storeDensity,
  type Density,
} from "../lib/density";

/**
 * AJ-E403 evidence: density modes (field presence per mode), the
 * aij.density persistence + migration from the legacy collapse boolean,
 * the keyboard cycle rules, and the 2×-interval stale threshold
 * (119 s fresh / 121 s stale at a 60 s interval).
 */

afterEach(cleanup);

const NOW = 1_728_800_000_000;

function mk(overrides: Partial<InstanceInfo> = {}): InstanceInfo {
  return {
    id: "sess-density",
    launch_recipe_id: "p-1",
    adopted: false,
    provider: "claude-code",
    account_id: "claude-work",
    account_label: "Work Max",
    model: "opus",
    launch_model: "opus",
    running_model: null,
    model_source: "opus" === null ? "unknown" : "launch",
    status: "running",
    quota: { used_percentage: 40, resets_at_ms: NOW + 3_600_000, confidence: "computed" },
    tokens: { input: 10_000, output: 5_000, cache_read: 0, cache_write: 0, reasoning: 0 },
    cpu_pct: 12,
    rss_bytes: 1_100_000_000,
    child_count: 3,
    subtask_count: null,
    git_branch: "main",
    git_dirty_count: null,
    needs_attention: null,
    attention: null,
    started_at_ms: NOW - 42 * 60_000,
    last_activity_ms: NOW - 10_000,
    machine_id: "this-mac",
    cwd: "/Users/dev/ai-juggler",
    kind: "terminal",
    isolation_label: null,
    project_id: "proj-1",
    conversation_id: null,
    ...overrides,
  };
}

const fresh = { resources: NOW - 1_000, quota: NOW - 1_000 };

describe("AJ-E403 persistence + migration", () => {
  it("defaults to comfortable when nothing is stored", () => {
    const storage = new Map<string, string>();
    expect(readDensity(storageLike(storage))).toBe("comfortable");
  });

  it("migrates the legacy aij.rail.collapsed boolean into minimal and removes the old key", () => {
    const storage = new Map<string, string>([["aij.rail.collapsed", "1"]]);
    expect(readDensity(storageLike(storage))).toBe("minimal");
    expect(storage.has("aij.rail.collapsed")).toBe(false);
    expect(storage.get("aij.density")).toBe("minimal");
  });

  it("a legacy non-collapsed rail migrates without inventing minimal", () => {
    const storage = new Map<string, string>([["aij.rail.collapsed", "0"]]);
    expect(readDensity(storageLike(storage))).toBe("comfortable");
    expect(storage.has("aij.rail.collapsed")).toBe(false);
  });

  it("stores and reads back every mode", () => {
    for (const mode of DENSITY_ORDER) {
      const storage = new Map<string, string>();
      storeDensity(mode, storageLike(storage));
      expect(readDensity(storageLike(storage))).toBe(mode);
    }
  });

  it("unknown stored values fall back to comfortable, never crash", () => {
    const storage = new Map<string, string>([["aij.density", "cozy"]]);
    expect(readDensity(storageLike(storage))).toBe("comfortable");
  });
});

describe("AJ-E403 the stale-data rule (2× interval, integer minutes)", () => {
  it("119 s old at a 60 s interval is still fresh", () => {
    expect(staleMinutes(NOW - 119_000, 60_000, NOW)).toBeNull();
    expect(staleCaption(NOW - 119_000, 60_000, NOW)).toBeNull();
  });

  it("121 s old at a 60 s interval is stale with an integer-minute caption", () => {
    expect(staleMinutes(NOW - 121_000, 60_000, NOW)).toBe(2);
    expect(staleCaption(NOW - 121_000, 60_000, NOW)).toBe("stale - 2m");
  });

  it("exactly 2× the interval is the last fresh moment", () => {
    expect(staleMinutes(NOW - 120_000, 60_000, NOW)).toBeNull();
    expect(staleMinutes(NOW - 120_001, 60_000, NOW)).toBe(2);
  });

  it("renders the stale - Nm chip on an expired row, never on a fresh one", () => {
    const session = mk();
    const staleAt = { resources: NOW - 121_000, quota: NOW - 121_000 };
    const { container: staleContainer } = render(
      <SessionRow session={session} selected={false} onSelect={() => {}} freshness={{ [session.id]: staleAt }} now={NOW} />,
    );
    expect(staleContainer.textContent).toContain("stale - 2m");

    const { container: freshContainer } = render(
      <SessionRow session={session} selected={false} onSelect={() => {}} freshness={{ [session.id]: fresh }} now={NOW} />,
    );
    expect(freshContainer.textContent).not.toContain("stale -");
  });
});

describe("AJ-E403 density field presence per mode", () => {
  it("Comfortable shows all I-01..I-11", () => {
    // Seed a measured history (3 distinct samples) so the sparkline
    // draws — the store dedupes identical rapid samples by design.
    rememberResourceSample("sess-density", 8, 900_000_000, NOW - 10_000);
    rememberResourceSample("sess-density", 12, 1_000_000_000, NOW - 5_000);
    rememberResourceSample("sess-density", 11, 1_100_000_000, NOW);
    const { container } = render(
      <SessionRow session={mk()} selected={false} onSelect={() => {}} freshness={{ ["sess-density"]: fresh }} now={NOW} />,
    );
    const text = container.textContent ?? "";
    for (const field of ["Claude", "opus", "Running", "% left", "15.0k tok", "main", "12% · 1.0 GB"]) {
      expect(text).toContain(field);
    }
    expect(container.querySelector("svg polyline")).not.toBeNull(); // sparkline
  });

  it("Compact shows I-01, I-02, I-04, I-05 bar-only, I-09, I-10 + CPU/RAM text; no sparkline, no % text", () => {
    const { container } = render(
      <SessionRow session={mk()} selected={false} onSelect={() => {}} density="compact" freshness={{ ["sess-density"]: fresh }} now={NOW} />,
    );
    const text = container.textContent ?? "";
    for (const field of ["Claude", "Running", "main", "12% · 1.0 GB"]) {
      expect(text).toContain(field);
    }
    // I-05 bar-only: the bar renders (title carries semantics), the
    // `% left` number text does not; the sparkline is replaced by text.
    expect(text).not.toContain("% left");
    expect(container.querySelector("svg polyline")).toBeNull();
    expect(container.textContent).toContain("WM"); // I-02 disc initials ride in the tooltip? no — title attr; assert via aria
  });

  it("Compact never hides the I-04 state text or the I-10 badge", () => {
    const waiting = mk({ status: "waiting_user", needs_attention: "waiting_user" });
    const { container } = render(
      <SessionRow session={waiting} selected={false} onSelect={() => {}} density="compact" freshness={{ ["sess-density"]: fresh }} now={NOW} />,
    );
    expect(container.textContent).toContain("Needs you");
  });

  it("Minimal shows disc + state glyph + ! badge and keeps the full text in aria-label", () => {
    const rows = [mk(), mk({ id: "sess-wait", status: "waiting_user", needs_attention: "waiting_user" })];
    const { container } = render(
      <MinimalInstanceColumn sessions={rows} selectedId={null} onSelect={() => {}} />,
    );
    const buttons = container.querySelectorAll("button");
    expect(buttons.length).toBe(2);
    // Full I-01..I-11 text reachable in every minimal row (Req.3).
    expect(buttons[0]!.getAttribute("aria-label")).toContain("Work Max");
    expect(buttons[0]!.getAttribute("aria-label")).toContain("60% left");
    expect(buttons[0]!.getAttribute("aria-label")).toContain("opus");
    // The ! badge count: one instance needs you.
    expect(container.textContent).toContain("! 1");
  });
});

describe("AJ-E403 keyboard cycle", () => {
  it("Ctrl+D qualifies everywhere; Cmd+D only inside the app (web bookmark stays free)", () => {
    const d = (overrides: Partial<KeyboardEvent>) =>
      ({ key: "d", ctrlKey: false, metaKey: false, shiftKey: false, altKey: false, ...overrides }) as KeyboardEvent;
    expect(isDensityShortcut(d({ ctrlKey: true }), false)).toBe(true);
    expect(isDensityShortcut(d({ metaKey: true }), true)).toBe(true);
    expect(isDensityShortcut(d({ metaKey: true }), false)).toBe(false); // web preview
    expect(isDensityShortcut(d({ ctrlKey: true, shiftKey: true }), true)).toBe(false);
    expect(isDensityShortcut(d({ key: "b", ctrlKey: true }), true)).toBe(false);
  });

  it("the cycle visits comfortable → compact → minimal → comfortable via dispatched keydowns", () => {
    // Harness: the same wiring App uses (predicate + cycle + store).
    let current: Density = "comfortable";
    const order: Density[] = [];
    const cycle = () => {
      current = nextDensity(current);
      order.push(current);
    };
    const onKey = (event: KeyboardEvent) => {
      if (isDensityShortcut(event, true)) cycle();
    };
    window.addEventListener("keydown", onKey);
    for (let i = 0; i < 3; i += 1) {
      window.dispatchEvent(new KeyboardEvent("keydown", { key: "d", ctrlKey: true, bubbles: true }));
    }
    window.removeEventListener("keydown", onKey);
    expect(order).toEqual(["compact", "minimal", "comfortable"]);
  });
});

/** Minimal Storage facade over a Map (localStorage test double). */
function storageLike(map: Map<string, string>): Storage {
  return {
    get length() {
      return map.size;
    },
    clear: () => map.clear(),
    getItem: (key: string) => map.get(key) ?? null,
    key: (index: number) => [...map.keys()][index] ?? null,
    removeItem: (key: string) => void map.delete(key),
    setItem: (key: string, value: string) => void map.set(key, value),
  };
}
