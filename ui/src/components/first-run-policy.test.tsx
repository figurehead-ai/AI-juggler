/**
 * AJ-E309 / D-14 vitest — what first run must and mustn't do with your
 * choices, now that the old account wizard is gone (AJ-E1022):
 *  - first run (every step: what's here, a fresh account, a folder)
 *    never asks how packs handle secrets and never writes that choice —
 *    the first switch asks while none exists (D-14);
 *  - every funnel event carries only allow-listed, free-text-free fields
 *    (no paths, labels, or credentials);
 *  - the pack restatement words; env overrides refuse isolation and
 *    credential keys inline.
 */
import { act, cleanup, fireEvent, render, within, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { FirstRun } from "../screens/FirstRun";
import { packPolicyRestatement } from "./PackPolicyChooser";
import { envRowProblem } from "./EnvOverlayEditor";

const w = () => window as unknown as Record<string, unknown>;
const NOW = Math.floor(Date.now() / 1000);

const SCAN = {
  installs: [
    {
      harness: "claude-code",
      path: "/Users/you/.claude-work",
      origin: "home-pattern",
      login: "logged_out",
      bytes: 1000,
      last_used_secs: NOW - 60,
      managed: false,
      notes: [],
      signals: [],
      inference: { purpose: "Work", purpose_confidence: "likely", login: "signed-out", login_confidence: "found", activity: "active", activity_confidence: "found", activity_words: "Active", keychain_bound: false },
    },
  ],
  probes: [],
  seeds: [],
  desktop_apps: [],
  cancelled: false,
};

beforeEach(() => {
  for (const key of Object.keys(w()).filter((key) => key.startsWith("__AIJ_"))) delete w()[key];
  w().__AIJ_SCAN_REPORT__ = SCAN;
  w().__AIJ_OS_ACCESS__ = [];
});
afterEach(cleanup);

const PACK_QUESTION = /context packs handle secrets|Decide later/i;

describe("D-14 — first run never asks about pack secrets", () => {
  it("bringing in, a fresh account and a folder: no pack question, nothing written", async () => {
    w().__AIJ_PICK_DIRECTORY__ = "/Users/you/old-claude";
    const { findByTestId, getByTestId, container } = render(
      <FirstRun mode="add" onDone={() => null} openLoginTerminal={() => null} />,
    );
    await findByTestId("first-run-found");
    expect(container.textContent).not.toMatch(PACK_QUESTION);
    // Copies, so the sign-in step shows too (used where they are, they keep theirs).
    fireEvent.click(within(getByTestId("first-run-placement-copy")).getByRole("radio"));
    fireEvent.click(getByTestId("first-run-bring-in"));
    await findByTestId("first-run-signin");
    expect(container.textContent).not.toMatch(PACK_QUESTION);
    cleanup();

    const fresh = render(<FirstRun mode="add" onDone={() => null} openLoginTerminal={() => null} />);
    await fresh.findByTestId("first-run-found");
    fireEvent.click(fresh.getByRole("button", { name: "Add a fresh account instead" }));
    await fresh.findByTestId("first-run-fresh");
    expect(fresh.container.textContent).not.toMatch(PACK_QUESTION);
    await act(async () => {
      fireEvent.click(await fresh.findByTestId("first-run-fresh-create"));
    });
    await fresh.findByTestId("first-run-signin");
    cleanup();

    const folder = render(<FirstRun mode="add" onDone={() => null} openLoginTerminal={() => null} />);
    await folder.findByTestId("first-run-found");
    await act(async () => {
      fireEvent.click(folder.getByRole("button", { name: "Import from another folder…" }));
    });
    await folder.findByTestId("first-run-folder-bring-in");
    expect(folder.container.textContent).not.toMatch(PACK_QUESTION);
    expect(w().__AIJ_REDACTION_SET_GLOBAL__).toBeUndefined();
  });
});

describe("AJ-E309 Req.6 — funnel events carry no paths, labels, or credentials", () => {
  it("first run's bring-in, fresh account and folder import emit only allow-listed fields", async () => {
    w().__AIJ_PICK_DIRECTORY__ = "/Users/you/old-claude";
    const run = render(<FirstRun onDone={() => null} openLoginTerminal={() => null} />);
    await run.findByTestId("first-run-found");
    fireEvent.click(within(run.getByTestId("first-run-placement-copy")).getByRole("radio"));
    await act(async () => {
      fireEvent.click(run.getByTestId("first-run-bring-in"));
    });
    await run.findByTestId("first-run-signin");
    cleanup();
    const fresh = render(<FirstRun mode="add" onDone={() => null} openLoginTerminal={() => null} />);
    await fresh.findByTestId("first-run-found");
    fireEvent.click(fresh.getByRole("button", { name: "Add a fresh account instead" }));
    await act(async () => {
      fireEvent.click(await fresh.findByTestId("first-run-fresh-create"));
    });
    cleanup();
    const folder = render(<FirstRun mode="add" onDone={() => null} openLoginTerminal={() => null} />);
    await folder.findByTestId("first-run-found");
    await act(async () => {
      fireEvent.click(folder.getByRole("button", { name: "Import from another folder…" }));
    });
    await act(async () => {
      fireEvent.click(await folder.findByTestId("first-run-folder-bring-in"));
    });
    await waitFor(() => expect(folder.getByTestId("first-run-signin")).toBeTruthy());

    const events = w().__AIJ_FUNNEL_EVENTS__ as { kind: string; fields: Record<string, string> }[];
    const kinds = events.map((event) => event.kind);
    for (const expected of ["flow_step", "scan_started", "scan_completed", "import_started", "import_completed", "login_queued"]) {
      expect(kinds).toContain(expected);
    }
    const forbidden = ["path", "label", "token", "dir", "source", "account", "email", "key", "name", "secret"];
    for (const event of events) {
      for (const [key, value] of Object.entries(event.fields)) {
        expect(forbidden).not.toContain(key);
        expect(value).toMatch(/^[a-z0-9][a-z0-9_:-]*$/);
      }
    }
    const serialized = JSON.stringify(events);
    for (const leak of ["/Users", "Work", "old-claude", ".claude", "Claude Code"]) {
      expect(serialized).not.toContain(leak);
    }
  });
});

describe("AJ-E309 — the pack restatement words", () => {
  it.each([
    ["strip", "Packs: strip secrets (all tools)"],
    ["keep", "Packs: keep secrets — packs stay on this Mac (all tools)"],
    ["ask", "Packs: ask each time (all tools)"],
  ] as const)("%s reads %s", (value, words) => {
    expect(packPolicyRestatement(value, "global", "Claude Code")).toBe(words);
  });
  it("names an override's scope; nothing chosen says the first switch asks", () => {
    expect(packPolicyRestatement("strip", "override", "Codex")).toBe("Packs: strip secrets (Codex override)");
    expect(packPolicyRestatement(null, "unset", "Claude Code")).toBe(
      "Packs: not chosen yet — your first switch asks before any pack is built",
    );
  });
});

describe("AJ-E309 Req.5 — env overrides refuse isolation and credential keys inline", () => {
  const protectedKeys = ["CLAUDE_CONFIG_DIR", "AI_JUGGLER_CLAUDE_BIN"];
  it.each([
    ["CLAUDE_CONFIG_DIR", "x", "Managed by AI Juggler"],
    ["ANTHROPIC_API_KEY", "x", "Looks like a credential"],
    ["CLAUDE_CODE_OAUTH_TOKEN", "x", "Looks like a credential"],
    ["lower", "x", "Use capitals"],
    ["MULTI", "a\nb", "One line"],
  ])("%s is refused", (key, value, why) => {
    expect(envRowProblem(key, value, protectedKeys)).toContain(why);
  });
  it("ordinary tuning variables pass (whole-segment matching)", () => {
    expect(envRowProblem("CLAUDE_CODE_MAX_OUTPUT_TOKENS", "8000", protectedKeys)).toBeNull();
    expect(envRowProblem("ANTHROPIC_BASE_URL", "http://127.0.0.1:8080", protectedKeys)).toBeNull();
  });
});
