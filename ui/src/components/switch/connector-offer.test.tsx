/**
 * AJ-E3013 (D-116): the switch's set-up list (and a new account's
 * settings) treat AI Juggler's connector as AI Juggler's own: never ticked
 * or copied as one of the user's skills, not counted among what the
 * target lacks, and its row offers the connector's own consent card for
 * the target's desktop app.
 */
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { setupSummary, SwitchSetupList } from "../SwitchSetupList";
import { forgetSharedReads } from "../../lib/shared-read";
import { CONNECTOR_OFFER, isConnectorOffer, withConnector, type OfferInfo, type SetupCheckInfo } from "../../lib/ipc/switchSetup";
import { resetPreviewConnector } from "../../lib/ipc/connector";

const offer = (over: Partial<OfferInfo>): OfferInfo => ({ id: "o1", kind: "skill", name: "release-notes", scope: "user", verdict: "copy", note: null, about: null, suggested: true, used: true, origin: "user", provided_by: null, get_it: null, can_copy: true, ...over });
const check = (offers: OfferInfo[]): SetupCheckInfo => ({ from: "Work Max", to: "Personal", offers, already_there: 2 });

beforeEach(() => {
  forgetSharedReads();
  resetPreviewConnector();
  Object.assign(window, {
    __AIJ_PLACES__: { places: [{ id: "app:claude-app:/Users/you/Profiles/Personal", app_id: "claude-app", app: "Claude", instance: "Personal", profile: "Personal", label: "Claude · Personal", app_pid: null, lost: false, keep_alive: false, paused: null, account_id: "claude-personal", seen_ms: 0, cannot_start: null, recent: [], found: false }] },
  });
});
afterEach(cleanup);

describe("AJ-E3013: AI Juggler's connector in the switch's set-up list", () => {
  it("is marked as AI Juggler's own: never suggested, never copied", () => {
    const marked = withConnector(check([offer({ id: "c", name: "ai-juggler" }), offer({})]));
    expect(marked.offers[0]).toMatchObject({ verdict: CONNECTOR_OFFER, suggested: false, can_copy: false });
    expect(marked.offers[1]).toMatchObject({ verdict: "copy", suggested: true });
    expect(isConnectorOffer(offer({ kind: "mcp-server", name: "ai-juggler" }))).toBe(true);
    expect(isConnectorOffer(offer({ kind: "rules", name: "ai-juggler" }))).toBe(false);
  });

  it("isn't counted among what the target lacks", () => {
    expect(setupSummary(withConnector(check([offer({ id: "c", name: "ai-juggler" })])), new Set())).toBe("Personal has everything else Work Max has set up.");
  });

  it("its row says it's AI Juggler's own, has no tick, and offers the target app's consent card", async () => {
    render(<SwitchSetupList check={withConnector(check([offer({ id: "c", name: "ai-juggler" })]))} picked={new Set()} onToggle={() => undefined} toFolder="claude-personal" />);
    const row = screen.getByTestId("switch-setup-connector");
    expect(row.textContent).toContain("AI Juggler's own");
    expect(row.textContent).toContain("Not one of your skills, so it isn't copied.");
    expect(row.querySelector('input[type="checkbox"]')).toBeNull();
    const card = await screen.findByTestId("switch-connector");
    expect(card.querySelector("h4")?.textContent).toBe("Add AI Juggler's connector to Claude app · Personal so its sessions can talk in pools");
  });

  it("without a desktop app for the target, it says where the connector comes from", () => {
    render(<SwitchSetupList check={withConnector(check([offer({ id: "c", name: "ai-juggler" })]))} picked={new Set()} onToggle={() => undefined} />);
    expect(screen.getByTestId("switch-setup-connector").textContent).toContain("Personal's Claude app gets it from AI Juggler, with your OK, when it joins a pool.");
    expect(screen.queryByTestId("switch-connector")).toBeNull();
  });
});
