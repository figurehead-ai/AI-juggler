import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { FlowHeader } from "./switch/SwitchHeader";
import { DEFAULT_PREFLIGHT } from "../preview/switching";
import { byAccount, targetsOf } from "../pages/session/ContinueOn";
import type { SwitchTargetData } from "../lib/ipc/switch-targets";
import type { SubscriptionInfo } from "../lib/subscriptions";

/**
 * F94 (the AJ-E2317 class, the PL1 final re-walk's F-PL1-27): opened from
 * a desktop App's row, the switch dialog named the source "Claude Code CLI
 * · personal" (the folder's CLI App) and never offered the desktop App on
 * that folder. The source is the App the conversation is in, by its id, as
 * its row is; the targets are each App by its own id, the source's never.
 */

afterEach(cleanup);

const SOURCE = "app:claude-app:/Users/someone/Library/Application Support/Claude-personal";
const TWIN = "app:claude-app:/Users/someone/Library/Application Support/Claude-personal-2";

const NO_MOVE = { offer: null, header: () => null };

describe("the switch dialog's source is the App the conversation is in", () => {
  it("a closed conversation of a desktop App's Code tab is that App's, by its id", () => {
    const preflight = {
      ...DEFAULT_PREFLIGHT,
      source_tool: "claude-code",
      source_account: "personal",
      source_title: "Refactor the settings screen",
      source_outside: true,
      // Closed: no app has it open now.
      source_app: null,
      source_app_id: SOURCE,
      source_app_name: "Claude app · personal",
    };
    render(<FlowHeader move={NO_MOVE} marks={null} target="client" preflight={preflight} destination="cli" />);
    const header = screen.getByTestId("switch-copy-header");
    expect(header.textContent).toContain("Refactor the settings screen · now on Claude app · personal");
    expect(screen.getByTestId("switch-strip").textContent).toContain("Claude app · personal");
    expect(header.textContent).not.toContain("Claude Code CLI · personal");
  });

  it("a terminal conversation is its folder's CLI App's", () => {
    const preflight = { ...DEFAULT_PREFLIGHT, source_tool: "claude-code", source_account: "personal", source_outside: true, source_app: null, source_app_id: "cli:personal", source_app_name: "Claude Code CLI · personal" };
    render(<FlowHeader move={NO_MOVE} marks={null} target="client" preflight={preflight} destination="cli" />);
    expect(screen.getByTestId("switch-strip").textContent).toContain("Claude Code CLI · personal");
  });
});

function row(account: string, label: string, place: string | null, app: string): SwitchTargetData {
  return {
    recipe_id: `r-${account}`,
    provider: "claude-code",
    account_id: account,
    account_label: label,
    badge: "Same conversation, another app",
    validated: true,
    cross_provider: false,
    drops: [],
    score: null,
    filtered_reason: null,
    desktop: place !== null,
    place_id: place,
    app_id: app,
    top_factors: [],
    factors: [],
    confidence: "Estimate",
    tte_min: null,
    suggested: false,
  };
}

const account = (folder: string, name: string): SubscriptionInfo => ({
  key: `sub-${folder}`,
  account_id: `acct:${folder}`,
  display_name: name,
  tool: "claude-code",
  state: "signed_in",
  email: null,
  organisation: null,
  reason: null,
  profiles: [{ id: folder, label: folder, folder: `~/.config/${folder}`, plan: null, renewal_date: null, fee_usd: null }],
  plan: null,
  renewal_date: null,
  fee_usd: null,
  differs: [],
  choices: [],
});

describe("the targets are each App by its own id", () => {
  const rows = [
    row("personal", "personal", SOURCE, SOURCE),
    row("personal", "personal", TWIN, TWIN),
    row("personal", "personal", null, "cli:personal"),
    row("client", "client", null, "cli:client"),
  ];

  it("leaves out the App it is in and its own folder in a terminal, and offers its folder's other desktop App", () => {
    const targets = targetsOf(rows, "personal", undefined, SOURCE);
    expect(targets.map((target) => target.placeId ?? target.profileId)).toEqual([TWIN, "client"]);
  });

  it("lists that desktop App to pick, never as a note that it shares the account", () => {
    const accounts = [account("personal", "Personal"), account("client", "Client")];
    const { others, sameAccount } = byAccount(targetsOf(rows, "personal", undefined, SOURCE), accounts, "personal");
    expect(sameAccount).toEqual([]);
    expect(others.flatMap((group) => group.targets.map((target) => target.placeId ?? target.profileId))).toEqual([TWIN, "client"]);
  });
});
