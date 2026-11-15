/**
 * AJ-E3011 round 3: Settings › Agent access and the pool's own inspector
 * show one pool's access with the same words and the same controls — what
 * its sessions may do on their own, its Custom actions, what you told it
 * to always allow, and how messages go into sessions. Rendered both ways;
 * if either drifts, this fails.
 */
import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { PoolAccessPanel } from "../../../components/PoolAccess";
import { scopeOf, type PoolInfo } from "../../../lib/ipc/pools";
import { InspectPool } from "../InspectPool";
import { PoolSetupSections } from "../PoolSetupSections";

afterEach(cleanup);

const NOW = 1_800_000_000_000;
const pool = (level: string): PoolInfo => ({
  id: "pool-web",
  name: "web team",
  kind: "user",
  project_id: null,
  access: { level, actions: ["read_roster", "read_tasks"], hop_limit: 4, delivery_stopped: false },
  rules: [{ id: 1, pool_id: "pool-web", action: "read_cards", created_at: NOW - 3_600_000, made_by: "you" }],
  spans_macs: false,
  scopes: 1,
  members: 0,
  ...scopeOf("user", null),
});

/** An area's words and controls, testids aside. */
function snapshot(area: Element | null): { controls: string[]; text: string } {
  if (!area) return { controls: ["(missing)"], text: "" };
  const controls = [...area.querySelectorAll("input, button")].map((control) => {
    const by = control.getAttribute("aria-labelledby");
    const name = by ? (document.getElementById(by)?.textContent ?? "") : (control.getAttribute("aria-label") ?? control.textContent ?? "");
    const state = control instanceof HTMLInputElement ? `:${control.type}:${control.checked}` : "";
    return `${name.trim()}${state}`;
  });
  return { controls, text: (area.textContent ?? "").replace(/\s+/g, " ").trim() };
}

const AREAS = ["pool-custom", "pool-rules", "pool-delivery"];

function settings(level: string) {
  const view = render(<PoolAccessPanel pool={pool(level)} now={NOW} />);
  const out = Object.fromEntries(AREAS.map((id) => [id, snapshot(view.container.querySelector(`[data-testid="${id}"]`))]));
  out.level = snapshot(view.container.querySelector('[data-testid="pool-access-level-choice"]'));
  view.unmount();
  return out;
}

function inspector(level: string) {
  const made = pool(level);
  const view = render(
    <>
      <InspectPool pool={made} project={undefined} now={NOW} onDeleted={() => undefined} />
      <PoolSetupSections pool={made} rules={{ rules: null, watched: [], reload: () => undefined }} members={[]} ruleLabel={() => ""} where="web" selection={{ kind: "pool" }} onAddMembers={() => undefined} />
    </>,
  );
  const out = Object.fromEntries(AREAS.map((id) => [id, snapshot(view.container.querySelector(`[data-testid="${id}"]`))]));
  out.level = snapshot(view.container.querySelector('[data-testid="pool-own-choice"]'));
  view.unmount();
  return out;
}

describe("AJ-E3011 round 3: Settings and the pool's inspector say one pool's access the same way", () => {
  it.each(["work_together", "custom"])("a pool at %s", (level) => {
    const there = settings(level);
    const here = inspector(level);
    expect(there).toEqual(here);
    // The words a newcomer reads, in both.
    expect(here["pool-rules"]!.text).toContain("Allowed without asking");
    expect(here["pool-delivery"]!.text).toContain("Stop and ask you after");
    expect(here.level!.controls.some((name) => name.startsWith("Work together:radio"))).toBe(true);
    if (level === "custom") expect(here["pool-custom"]!.controls.length).toBeGreaterThan(0);
  });
});
