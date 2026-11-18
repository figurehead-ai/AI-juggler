/**
 * D-126: How they work and Finish as the mock-ups draw them — Who leads,
 * the roles, the messages presets, On their own, the team tools and
 * Review — each as the kit's cards, with the same words and choices as
 * before (paths-parity keeps the diagram's alike).
 */
import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { resetPreviewAgentBridge } from "../../lib/ipc/agent-bridge";
import { draftOf, type Draft } from "../../lib/new-pool";
import { templateById } from "../../lib/pool-templates";
import { LeadCards } from "./LeadCards";
import { NewPoolLead } from "./NewPoolLead";

beforeEach(() => resetPreviewAgentBridge());
afterEach(cleanup);

const app = (ref: string, label: string, role: "member" | "reviewer" = "member") => ({ ref, kind: "app" as const, label, role, level: null, reach: "later" as const });
const NO_LOGOS = { apps: [], sessions: [] };

describe("Who leads (D-126 cards)", () => {
  it("one radio group of cards with logos: open sessions, each App's next session, no lead; the arrows choose", () => {
    const picked: string[] = [];
    render(
      <LeadCards
        sessions={[{ ref: "session:s1", name: "Planner", detail: "web · Claude Code · working" }]}
        apps={[{ ref: "app:cli:a", name: "Claude Code · Work" }, { ref: "app:cli:b", name: "Codex · Personal", blocked: "Can't lead: its app can't take the connector." }]}
        value={{ kind: "next", app: "app:cli:a" }}
        onChange={(choice) => picked.push(choice.kind === "next" ? choice.app : choice.kind)}
        where="web"
        logos={NO_LOGOS}
      />,
    );
    const group = screen.getByRole("radiogroup", { name: "Who leads" });
    expect(within(group).getAllByRole("radio").map((radio) => radio.getAttribute("aria-labelledby") && document.getElementById(radio.getAttribute("aria-labelledby")!)?.textContent)).toEqual([
      "Planner",
      "Claude Code · Work's next session",
      "Codex · Personal's next session",
      "No lead for now",
    ]);
    expect(within(group).getByRole("radio", { name: "Claude Code · Work's next session" })).toHaveProperty("checked", true);
    expect(within(group).getByRole("radio", { name: "Codex · Personal's next session" })).toHaveProperty("disabled", true);
    expect(screen.getAllByTestId("member-logo")).toHaveLength(3);
    expect(screen.getByTestId("lead-cards-sentence").textContent).toBe("Claude Code · Work takes charge as soon as one of its sessions starts in web: it plans the work and hands out tasks.");
    // The arrows skip the one that can't lead.
    fireEvent.keyDown(within(group).getByRole("radio", { name: "Claude Code · Work's next session" }), { key: "ArrowDown" });
    expect(picked).toEqual(["none"]);
  });

  it("the step draws what a lead does in three steps, then the cards", () => {
    let draft: Draft = { ...draftOf(templateById("blank"), "web"), members: [app("app:cli:a", "Claude Code · Work")] };
    render(<NewPoolLead draft={draft} onDraft={(next) => (draft = next)} sessions={[]} nameOf={(_, label) => label} where="web" />);
    expect(within(screen.getByTestId("new-pool-lead-how")).getAllByRole("listitem").map((one) => one.textContent)).toEqual(["1You tell the lead what you want.", "2It splits that into tasks for the others.", "3You approve anything that needs you."]);
    fireEvent.click(screen.getByRole("radio", { name: "Claude Code · Work's next session" }));
    expect(draft.leadNext).toBe("app:cli:a");
  });
});
