/**
 * The 0.4 design review on ⌘K's rows and preview (52, 72): what ↩ does
 * is said on the active row only, as "↩ Open"; a session's row shows its
 * App's mark, never AI Juggler's; the preview names the Account by its
 * real name and Continue as Home does.
 */
import { cleanup, render, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";
import type { ProjectInfo } from "../../types/generated/ProjectInfo";
import { loadSubscriptions, staleSubscriptions, type SubscriptionInfo } from "../../lib/subscriptions";
import * as subs from "../../lib/ipc/subscriptions";
import { OWN_APP_ID } from "../AppIcon";
import { appOfSession } from "../../lib/app-of";
import { RowView, rowApp } from "./RowView";
import { Preview, type Previewable } from "./Preview";
import type { PaletteRow } from "./rows";

afterEach(() => {
  cleanup();
  staleSubscriptions();
  vi.restoreAllMocks();
});

const session = { id: "s1", provider: "claude-code", account_id: "work", account_label: "Work Max", status: "running", kind: "terminal", git_branch: null, cwd: "/w/alpha", project_id: null, conversation_id: null, quota: null, label: "feat/read-model", last_activity_ms: Date.now() } as unknown as InstanceInfo;
const local: Extract<PaletteRow, { where: "local" }> = { where: "local", row: session, id: "s1", desktop: false, ranges: [] };

describe("⌘K's row (0.4 design review, 52 and 72)", () => {
  it("says what ↩ does as \"↩ Open\", shown only on the active (selected) row", () => {
    const project: PaletteRow = { where: "project", project: { id: "p1", name: "payments-api", root: "/w/payments-api" } as ProjectInfo, id: "project::p1", ranges: [] };
    const view = render(<RowView entry={project} />);
    const hint = view.getByTestId("goto-row-hint");
    expect(hint.textContent).toBe("↩ Open");
    expect(hint.className).toContain("hidden");
    expect(hint.className).toContain("in-aria-selected:inline");
  });

  it("marks one of AI Juggler's sessions with its App's icon, not AI Juggler's", () => {
    expect(rowApp(local).appId).not.toBe(OWN_APP_ID);
    expect(rowApp(local)).toEqual(appOfSession(session));
  });
});

describe("⌘K's preview (0.4 design review, 72)", () => {
  it("names the Account by its real name, and Continue as Home does", async () => {
    vi.spyOn(subs, "subscriptionsList").mockResolvedValue([
      {
        key: "k-work", account_id: "acct:anthropic:work", name: null, display_name: "Claude Max 20x", tool: "claude-code", state: "signed_in",
        email: null, organisation: null, reason: null, plan: "max-20x", renewal_date: null, fee_usd: null, differs: [], choices: [],
        profiles: [{ id: "work", label: "work", folder: "~/.claude-work", plan: null, renewal_date: null, fee_usd: null }],
      },
    ] as SubscriptionInfo[]);
    await loadSubscriptions(true);
    const view = render(<Preview entry={local as Previewable} id="p" doings={{ open: vi.fn(), goTo: vi.fn(), continueElsewhere: vi.fn() }} />);
    const account = () => [...view.container.querySelectorAll("dt")].find((term) => term.textContent === "Account")!.nextElementSibling!.textContent;
    await waitFor(() => expect(account()).toBe("Claude Max 20x"));
    expect(view.getByTestId("goto-preview-continue").textContent).toBe("Continue in another app…");
  });
});
