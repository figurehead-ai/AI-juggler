/**
 * AJ-E3011 §2–§7: every setup control has a visible label first and reads
 * back as a sentence; the roles are cards with one line each; what sessions
 * may do on their own says its consequence and an example ("Level" gone);
 * the talk table is calm for Allowed, amber for Ask first, red for Blocked;
 * the lead picker offers its three kinds; the agent tools' consent says
 * what it gives, for every pool, and what won't work until it's on.
 */
import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { pick } from "../../../test/pick";
import { agentBridgeGet, resetPreviewAgentBridge } from "../../../lib/ipc/agent-bridge";
import { joinSentence, leadSentence, levelSentence, memberSentence, OWN_LEVELS, roleCard, wakeWords, whereWords } from "../../../lib/pool-words";
import { draftOf } from "../../../lib/new-pool";
import { effectiveRule, nextRule, ruleOf, templateById } from "../../../lib/pool-templates";
import { talkSentence } from "../../../screens/pools/NewPoolReview";
import { nameWraps } from "../NodeParts";
import { LEVELS, levelLabel } from "../../../lib/pools";
import { JoinAs } from "./JoinAs";
import { LeadPicker } from "./LeadPicker";
import { MemberOwnLevel, OwnLevelChoice } from "./OwnLevelChoice";
import { PoolToolsCard } from "./PoolToolsCard";
import { RoleCards, RolePicker, holdersOf } from "./RoleCards";
import { TalkTable } from "./TalkTable";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});
beforeEach(() => resetPreviewAgentBridge());

describe("the sentences each choice reads back as", () => {
  it("who joins, as what, and what they may do", () => {
    expect(joinSentence("web", "member", "work_together")).toBe("New sessions in web join as Member and may Work together.");
    expect(joinSentence("web and docs", "reviewer", "read")).toBe("New sessions in web and docs join as Reviewer and may Look only.");
    expect(joinSentence("web", null, "full")).toBe("Only who's in joins: other sessions in web stay out of this pool.");
    expect(whereWords(false, ["web", "docs", "api"])).toBe("web, docs and api");
    expect(whereWords(true, [])).toBe("every project");
  });

  it("each member, its role and its own choice or the pool's", () => {
    expect(memberSentence("Codex · Personal", "reviewer", "read", "work_together")).toBe("Codex · Personal is a Reviewer and may Look only.");
    expect(memberSentence("Claude Code · Work", "member", null, "full")).toBe("Claude Code · Work is a Member and may Work together and wake each other, as the pool.");
    expect(memberSentence("Planner", "lead", null, "work_together")).toBe("Planner leads and may Work together, as the pool.");
    expect(memberSentence("Watcher", "observer", null, "read")).toBe("Watcher is an Observer and may Look only, as the pool.");
  });

  it("who leads, and what a pool without one lacks", () => {
    expect(leadSentence("Planner", null)).toBe("Planner leads: it plans the work and hands out tasks.");
    expect(leadSentence(null, "Claude Code · Work")).toBe("Claude Code · Work leads from its next session in these projects: it plans the work and hands out tasks.");
    // Round 2 (#3): where, and that none is open yet.
    expect(leadSentence(null, "Claude Code · Work", "web", 0)).toBe("Claude Code · Work leads from its next session in web (none is open now): it plans the work and hands out tasks.");
    expect(leadSentence(null, "Claude Code · Work", "web", 2)).toBe("Claude Code · Work leads from its next session in web: it plans the work and hands out tasks.");
    expect(leadSentence(null, null)).toMatch(/^No lead for now: nobody splits the work into tasks/);
  });

  it("what they may do on their own: the word Level is gone, waking says it costs a turn (D-28, D-119)", () => {
    expect(OWN_LEVELS.map((one) => one.label)).toEqual(["Look only", "Work together", "Work together and wake each other"]);
    expect(LEVELS.map((one) => one.label)).toEqual(["Look only", "Work together", "Work together and wake each other", "Custom"]);
    expect(levelLabel("full")).toBe("Work together and wake each other");
    expect(levelSentence("read")).toContain("waits for your OK in Needs you");
    expect(OWN_LEVELS[2]!.line).toContain("Each wake starts a turn on your subscription.");
    expect(wakeWords("full")).toBe("wakes on new messages");
    expect(wakeWords("work_together")).toBe("reads messages on its next turn");
    expect(wakeWords("full", true)).toBe("can't be woken");
    expect(wakeWords("read", false, true)).toBe("read messages on their next turn");
  });
});

describe("the controls", () => {
  it("New sessions join as: a labelled picker whose choice is the everyone member's role", () => {
    const change = vi.fn();
    render(<JoinAs where="web" value="member" onChange={change} />);
    const picker = screen.getByTestId("join-as-picker");
    expect(screen.getByTestId("join-as").textContent).toContain("New sessions in web join as");
    expect(picker.getAttribute("aria-label")).toBe("New sessions in web join as: Member");
    pick(picker, "Reviewer");
    expect(change).toHaveBeenCalledWith("reviewer");
    pick(picker, "Don't join");
    expect(change).toHaveBeenCalledWith(null);
  });

  it("the roles are four cards, one line each, saying who holds them", () => {
    render(<RoleCards holders={holdersOf([{ name: "Planner", role: "lead" }, { name: "Codex", role: "reviewer" }, { name: "new sessions", role: "member" }])} />);
    const cards = screen.getAllByTestId("role-card");
    expect(cards.map((card) => card.dataset.role)).toEqual(["lead", "member", "reviewer", "observer"]);
    expect(cards[0]!.textContent).toContain("Plans the work and hands out tasks. One session.");
    expect(cards[2]!.textContent).toContain("Doesn't build.");
    expect(within(cards[0]!).getByTestId("role-card-holders").textContent).toBe("Now: Planner");
    expect(within(cards[3]!).getByTestId("role-card-holders").textContent).toBe("No one yet");
  });

  it("a member's role picker shows the same lines, and an App can't lead (D-92)", () => {
    const change = vi.fn();
    render(<RolePicker member="app" name="Claude Code · Work" value="member" onChange={change} testId="role" />);
    fireEvent.click(screen.getByTestId("role"));
    const lead = screen.getAllByRole("menuitemradio").find((line) => line.textContent?.startsWith("Lead"))!;
    expect(lead.getAttribute("aria-disabled") ?? String(lead instanceof HTMLButtonElement && lead.disabled)).toBe("true");
    expect(screen.getAllByRole("menuitemradio").map((line) => line.textContent)).toContain("ReviewerReads everything, checks work in review and gives its verdict. Doesn't build.");
  });

  it("what they may do on their own: a labelled choice with its consequence and an example, and what it means", () => {
    const change = vi.fn();
    const { rerender } = render(<OwnLevelChoice value="work_together" onChange={change} />);
    expect(screen.getByTestId("own-level").textContent).toContain("What these sessions may do on their own");
    expect(screen.getByRole("group", { name: "What these sessions may do on their own" })).toBeTruthy();
    expect(screen.getAllByRole("radio")).toHaveLength(3);
    // Round 2 (#4): what it covers, and what stays with each app.
    expect(screen.getByTestId("own-level-scope").textContent).toBe("This is how they work together through AI Juggler. What each may change in your files stays with its own app's settings.");
    expect(screen.getByTestId("own-level-line").textContent).toBe(OWN_LEVELS[1]!.line);
    expect(screen.getByTestId("own-level-example").textContent).toMatch(/^Example: /);
    // Round 2 (#5): waking's example shows at once, not behind a fold.
    expect(screen.getByTestId("own-level-example-full").textContent).toBe("Example: Codex finishes a review; its note wakes Claude's idle session to carry on, without you. Each wake uses a turn on your subscription.");
    fireEvent.click(screen.getByTestId("own-level-choice-full"));
    expect(change).toHaveBeenCalledWith("full");
    rerender(<OwnLevelChoice value="full" onChange={change} />);
    expect(screen.getByTestId("own-level-line").textContent).toContain("wakes an idle session");
    expect(screen.getByTestId("own-level-explain").textContent).toContain("What does this mean?");
  });

  it("a member's own choice offers Same as the pool first", () => {
    const change = vi.fn();
    render(<MemberOwnLevel name="Codex" value={null} poolLevel="work_together" onChange={change} testId="member-own" />);
    const picker = screen.getByTestId("member-own");
    expect(picker.textContent).toContain("Same as the pool (Work together)");
    fireEvent.click(picker);
    expect(screen.getAllByRole("menuitemradio")[0]!.textContent).toContain("Same as the pool");
    fireEvent.click(screen.getAllByRole("menuitemradio")[1]!);
    expect(change).toHaveBeenCalledWith("read");
  });

  it("the talk table: plain row labels, Allowed calm, Ask first amber with a dot, Blocked red with a glyph", () => {
    const cycle = vi.fn();
    render(<TalkTable ruleOf={(from, to) => (from === "member" && to === "member" ? "ask" : from === "member" && to === "reviewer" ? "blocked" : "allowed")} onCycle={cycle} />);
    const table = screen.getByTestId("talk-table");
    expect(table.textContent).not.toContain("From");
    expect(within(table).getAllByRole("rowheader").map((head) => head.textContent)).toEqual(["Lead can message…", "Member can message…", "Reviewer can message…", "Observer can message…"]);
    const cell = (pair: string) => screen.getAllByTestId("talk-table-cell").find((one) => one.dataset.pair === pair)!;
    const allowed = cell("lead>member").firstElementChild!;
    expect(allowed.className).toContain("text-muted");
    expect(allowed.className).not.toMatch(/bg-|text-success/);
    expect(cell("member>member").firstElementChild!.className).toContain("text-warning");
    expect(cell("member>member").querySelector(".level-mark")).toBeTruthy();
    expect(cell("member>reviewer").firstElementChild!.className).toContain("text-danger");
    expect(cell("member>reviewer").textContent).toContain("⊘");
    expect(cell("member>member").getAttribute("aria-label")).toBe("Member to Members: Ask first. Press to change.");
    fireEvent.click(cell("lead>member"));
    expect(cycle).toHaveBeenCalledWith("lead", "member");
    expect(screen.getByTestId("talk-table-above").textContent).toContain("Ask first waits for your OK in Needs you");
    expect(screen.getByTestId("talk-table-below").textContent).toContain("click the line between them on the diagram");
  });

  it("the lead picker: sessions open now, each App's next session, then no lead for now", () => {
    const change = vi.fn();
    render(
      <LeadPicker
        sessions={[{ ref: "session:s1", name: "web · fix the cart", detail: "Working" }]}
        apps={[{ ref: "app:cli:work", name: "Claude Code · Work" }, { ref: "app:cli:codex", name: "Codex · Personal" }]}
        value={{ kind: "none" }}
        onChange={change}
      />,
    );
    const options = screen.getAllByTestId("lead-picker-option");
    expect(options.map((one) => one.dataset.kind)).toEqual(["session", "next", "next", "none"]);
    expect(screen.getByRole("group", { name: "Who leads" })).toBeTruthy();
    expect(screen.getByLabelText("Claude Code · Work's next session")).toBeTruthy();
    fireEvent.click(options[1]!);
    expect(change).toHaveBeenCalledWith({ kind: "next", app: "app:cli:work" });
    fireEvent.click(options[0]!);
    expect(change).toHaveBeenCalledWith({ kind: "session", ref: "session:s1" });
    const none = options[3];
    expect(none instanceof HTMLInputElement && none.checked).toBe(true);
  });

  it("the team tools (round 2, #1, D-30): what they give, who, the cost, without them, asked once and off per project or app", async () => {
    render(<PoolToolsCard />);
    const card = await screen.findByTestId("pool-tools-card");
    expect(card.querySelector("h4")?.textContent).toBe("Let your assistants work as a team");
    for (const words of [
      "They can message each other and share notes and tasks.",
      "Example: Claude sees that Codex is reviewing and sends it a note.",
      "never a session outside a pool",
      "Nothing extra: a session uses its own turns only when it acts.",
      "Without it, the pool is set up, but nobody in it can message.",
      "Asked once. After that you can switch it off for a project or an app in Settings › Agent access.",
    ])
      expect(card.textContent).toContain(words);
    // Never "one switch for every pool" (run 2 left it off for that).
    expect(card.textContent).not.toMatch(/one switch|every pool/i);
    expect(screen.getByTestId("pool-tools-card-on").textContent).toBe("Turn on team tools");
    // The link opens Settings › Agent access at its agent tools.
    const opened: string[] = [];
    const listen = (event: Event) => opened.push(event instanceof CustomEvent ? JSON.stringify(event.detail) : "");
    window.addEventListener("aij:open-settings", listen);
    fireEvent.click(screen.getByTestId("team-tools-settings"));
    window.removeEventListener("aij:open-settings", listen);
    expect(opened).toEqual([JSON.stringify({ section: "agent-tools" })]);
    fireEvent.click(screen.getByTestId("pool-tools-card-not-now"));
    expect((await screen.findByTestId("pool-tools-card-state")).textContent).toBe("Off: they can't message yet.");
    fireEvent.click(screen.getByTestId("pool-tools-card-on"));
    await vi.waitFor(() => expect(screen.queryByTestId("pool-tools-card")).toBeNull());
  });

  it("the team tools off for one of the pool's projects: the card says where, and Turn on switches that project back on", async () => {
    Object.assign(window, { __AIJ_AGENT_BRIDGE__: { asked: true, on: true, off_projects: ["p-web"] } });
    resetPreviewAgentBridge();
    render(<PoolToolsCard projects={[{ id: "p-web", name: "web" }, { id: "p-docs", name: "docs" }]} />);
    expect((await screen.findByTestId("pool-tools-card-state")).textContent).toBe("Off for web: they can't message there.");
    expect(screen.queryByTestId("pool-tools-card-not-now")).toBeNull();
    fireEvent.click(screen.getByTestId("pool-tools-card-on"));
    await vi.waitFor(() => expect(screen.queryByTestId("pool-tools-card")).toBeNull());
    // web's own switch is back on; nothing else changed.
    expect(await agentBridgeGet()).toMatchObject({ asked: true, on: true, off_projects: [] });
    Object.assign(window, { __AIJ_AGENT_BRIDGE__: undefined });
  });
});

// The coordinator's check (0.4.6): an Observer "watches and reads;
// changes nothing", the backend asks you for every message it sends
// (agent_access::decide's observer flag), and Start from scratch blocks
// its sends as every template does. The table, the role card and the
// review sentence agree.
describe("an Observer only reads, everywhere it's said", () => {
  it("Start from scratch blocks an Observer's sends, as the templates do", () => {
    const scratch = draftOf(templateById("blank"), "web");
    for (const to of ["lead", "member", "reviewer", "observer"] as const) expect(ruleOf(scratch.talk, "observer", to)).toBe("blocked");
    expect(ruleOf(draftOf(templateById("writer-reviewer"), "web").talk, "observer", "member")).toBe("blocked");
    expect(talkSentence(scratch)).toBe("Everyone may message everyone; an Observer only reads.");
    expect(roleCard("observer").line).toBe("Watches and reads. Changes nothing.");
  });

  it("the table shows what the backend does: an Observer's Allowed asks you, and it cycles only between Ask first and Blocked", () => {
    expect(effectiveRule("observer", "allowed")).toBe("ask");
    expect(effectiveRule("member", "allowed")).toBe("allowed");
    expect(nextRule("blocked", "observer")).toBe("ask");
    expect(nextRule("ask", "observer")).toBe("blocked");
    expect(nextRule("allowed", "observer")).toBe("blocked");
    render(<TalkTable ruleOf={() => "allowed"} onCycle={() => undefined} />);
    const cell = (pair: string) => screen.getAllByTestId("talk-table-cell").find((one) => one.dataset.pair === pair)!;
    expect(cell("observer>lead").dataset.rule).toBe("ask");
    expect(cell("lead>member").dataset.rule).toBe("allowed");
    // A pool that never set it (a project's own) reads true: the review too.
    expect(talkSentence({ talk: {} })).toContain("Observer to Lead: ask first");
  });

  it("an Observer's own row blocked is calm, not an exception's red", () => {
    render(<TalkTable ruleOf={(from) => (from === "observer" ? "blocked" : "allowed")} onCycle={() => undefined} />);
    const blocked = screen.getAllByTestId("talk-table-cell").find((one) => one.dataset.pair === "observer>member")!;
    expect(blocked.firstElementChild!.className).toContain("text-muted");
    expect(blocked.firstElementChild!.className).not.toContain("text-danger");
    expect(screen.getByTestId("talk-table-below").textContent).toContain("An Observer only reads");
  });
});

describe("a node's name breaks only between words (AJ-E3012)", () => {
  it("wraps words that fit its room, and asks for one line with an ellipsis when a word doesn't", () => {
    expect(nameWraps("Fix the cart totals", 80, 12)).toBe(true);
    expect(nameWraps("Planner", 30, 12)).toBe(false);
    expect(nameWraps("Checkout copy pass", 40, 12)).toBe(false);
  });
});
