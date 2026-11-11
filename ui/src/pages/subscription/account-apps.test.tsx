/**
 * AJ-E3205 R2: an Account's Apps tab lists its busy Apps first, then its
 * quiet ones (no session yet) last, on a tinted surface of their own.
 */
import { cleanup, render, within } from "@testing-library/react";
import { afterEach, expect, it } from "vitest";
import type { AppModel } from "../../lib/apps";
import type { AccountApp } from "../../lib/account-items";
import { AccountAppRows } from "./AccountApps";

afterEach(cleanup);

const NOW = Date.UTC(2026, 10, 7, 12);

function app(id: string, name: string, sessions: number): AccountApp {
  const model: AppModel = {
    id, kind: "cli", tool: "claude-code", appId: null, app: "Claude Code", profile: name, name, defaultName: name,
    named: false, railKey: id, folder: null, accountId: "acct:work", place: null, slot: 0,
  };
  return { app: model, sessions, open: 0, working: 0, lastMs: sessions > 0 ? NOW - 60_000 : 0 };
}

it("lists busy Apps first and the quiet ones last, on a tinted surface", () => {
  const view = render(<AccountAppRows apps={[app("cli:quiet", "Spare", 0), app("cli:busy", "Work Max", 3)]} now={NOW} />);
  const all = view.getByTestId("account-apps");
  const quiet = view.getByTestId("account-apps-quiet");
  expect(quiet.className).toContain("bg-surface-2");
  expect(within(quiet).getByText("No sessions yet")).toBeTruthy();
  expect(quiet.textContent).toContain("Spare");
  expect(quiet.textContent).not.toContain("Work Max");
  // The busy one comes before the quiet group.
  expect((all.textContent ?? "").indexOf("Work Max")).toBeLessThan((all.textContent ?? "").indexOf("Spare"));
});

it("has no quiet group when every App has sessions", () => {
  expect(render(<AccountAppRows apps={[app("cli:busy", "Work Max", 3)]} now={NOW} />).queryByTestId("account-apps-quiet")).toBeNull();
});
