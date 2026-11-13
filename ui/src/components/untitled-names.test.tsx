/**
 * D-96 (persona D2, Devon): two sessions in one folder with no title yet
 * both read "Untitled session · billing". Every surface names one by the
 * one short-name rule, so each says its facts — folder · when it started
 * — and two of one minute are numbered: a rail row, a pool's node (the
 * session page's header: `inspector-shell.test`). D-103.8: never its
 * Account, which a list's row says on its second line.
 */
import { act, cleanup, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { itemsOf } from "../lib/rail-items";
import { clock } from "../lib/time-words";
import { SessionRow } from "./row/SessionRow";
import { first, openPage, resetCanvasTests, second, seedTeam } from "../pages/pool/canvas.test-kit";
import type { InstanceInfo } from "../types/generated/InstanceInfo";

const AT = new Date(2026, 10, 10, 1, 14, 0).getTime();
const NOW = AT + 3_600_000;

/** An untitled session in `billing` on the App "Work". */
function untitled(base: InstanceInfo, over: Partial<InstanceInfo> = {}): InstanceInfo {
  return { ...base, label: undefined, title: undefined, git_branch: "main", cwd: "/Users/you/Dev/billing", account_label: "Work", started_at_ms: AT, ...over };
}

beforeEach(() => {
  vi.useFakeTimers({ toFake: ["Date"], now: NOW });
  resetCanvasTests();
});
afterEach(() => {
  cleanup();
  vi.useRealTimers();
  localStorage.clear();
});

describe("a rail row", () => {
  it("names an untitled session by its facts, and two of one folder apart", () => {
    const items = itemsOf({ sessions: [untitled(first), untitled(second, { started_at_ms: AT + 5 * 60_000 })], conversations: [], projects: [], seen: {} });
    expect(items.map((item) => item.title)).toEqual([`billing · started ${clock(AT)}`, `billing · started ${clock(AT + 5 * 60_000)}`]);
    render(<SessionRow item={items[0]!} density="cozy" now={NOW} onSelect={vi.fn()} />);
    expect(document.body.textContent).toContain(`billing · started ${clock(AT)}`);
    expect(document.body.textContent).not.toContain("Untitled session");
  });

  it("D-103.8: a list's row says its Account on its second line, never in its name", () => {
    const [item] = itemsOf({ sessions: [untitled(first)], conversations: [], projects: [], seen: {} });
    render(<SessionRow item={item!} density="list" now={NOW} onSelect={vi.fn()} />);
    expect(document.querySelector("[data-testid=page-session-title]")?.textContent).toBe(`billing · started ${clock(AT)}`);
    expect(document.querySelector("[data-testid=page-session-title]")?.parentElement?.nextElementSibling?.textContent).toContain("Work");
  });

  it("numbers two of one folder and minute; a titled one keeps its title", () => {
    const items = itemsOf({ sessions: [untitled(second, { started_at_ms: AT + 20_000 }), untitled(first), untitled({ ...first, id: "titled" }, { title: "Ship it" })], conversations: [], projects: [], seen: {} });
    const facts = `billing · started ${clock(AT)}`;
    expect(items.map((item) => item.title)).toEqual([`${facts} (2)`, `${facts} (1)`, "Ship it"]);
  });
});

describe("a pool's node", () => {
  it("says the facts once, and numbers two of one minute", async () => {
    seedTeam();
    const view = await openPage("pool-team", [untitled(first), untitled(second, { started_at_ms: AT + 20_000 })]);
    await act(async () => {});
    const facts = `billing · started ${clock(AT)}`;
    await waitFor(() => expect(view.container.querySelector(`[data-node-name="${first.id}"]`)?.textContent).toBe(`${facts} (1)`));
    expect(view.container.querySelector(`[data-node-name="${second.id}"]`)?.textContent).toBe(`${facts} (2)`);
    // Never "started" twice (the canvas adds a time only to a name without one).
    expect(view.container.textContent).not.toMatch(/started [^·(]* · started/);
  });
});
