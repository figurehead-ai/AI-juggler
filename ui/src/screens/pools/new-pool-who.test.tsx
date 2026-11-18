/**
 * AJ-E3007, AJ-E3011, D-126: New pool's Who's in — paged, the shell-only
 * App said, and (D-126) the Apps as logo cards.
 */
import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { draftOf, type Candidate, type Draft } from "../../lib/new-pool";
import type { MemberKind } from "../../lib/ipc/pool-model";
import { templateById } from "../../lib/pool-templates";
import { CANDIDATES_PAGE, NewPoolWho } from "./NewPoolWho";
import { APP_CARDS, coveredSessions, openInWords } from "./NewPoolWhoApps";
import { MOCK_ACCOUNTS, MOCK_PROJECTS, MOCK_SESSIONS } from "../../preview/fixtures";
import { NewPool } from "./NewPool";
import type { AppModel } from "../../lib/apps";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";

afterEach(cleanup);

/** rc27 live pass 2: step 2 drew 143 session rows at once and was slow. A
 * group shows a page, and Show more adds the next. */
describe("New pool's Who list is paged", () => {
  it("draws a page of a long group, and Show more adds another", () => {
    const rows: Candidate[] = Array.from({ length: 143 }, (_, i) => ({
      ref: `sess-${i}`,
      kind: "session",
      label: `Session ${i}`,
      detail: "lab · ended",
      group: "recent",
      reach: "full",
      blocked: null,
      lastMs: 143 - i,
    }));
    const candidates = { session: rows, app: [], account: [], tool: [], mac: [], everyone: [] } as Record<MemberKind, Candidate[]>;
    render(<NewPoolWho draft={draftOf(templateById("feature-team"), null)} onDraft={() => undefined} candidates={candidates} onFix={() => undefined} />);
    const list = screen.getByTestId("new-pool-candidates");
    expect(within(list).getAllByRole("listitem")).toHaveLength(CANDIDATES_PAGE);
    fireEvent.click(screen.getByTestId("new-pool-more-recent"));
    expect(within(list).getAllByRole("listitem")).toHaveLength(2 * CANDIDATES_PAGE);
    expect(screen.getByTestId("new-pool-more-recent").textContent).toContain(`${143 - 2 * CANDIDATES_PAGE} more`);
    // A search pages its matches the same way.
    fireEvent.change(screen.getByRole("searchbox"), { target: { value: "Session 1" } });
    expect(within(list).getAllByRole("listitem").length).toBeLessThanOrEqual(CANDIDATES_PAGE);
  });
});

// The P8 walk (rc32, step 4): adding a Kimi Code App left the counters at
// "0 full, 0 shell only", and its row said only "new ones join".
describe("P8 an App member that works through its shell, in the wizard", () => {
  it("its card says Shell only, and once ticked it's in the team as an App", () => {
    const why = "Its tool has no agent tools yet: it works through its shell, and can't read the pool, post or take a task.";
    const kimi: Candidate = { ref: "app:cli:kimi-1", kind: "app", label: "Kimi Code · Lab", detail: "none open now", group: "all", reach: "later", blocked: null, shell: why, lastMs: 0 };
    const candidates = { session: [], app: [kimi], account: [], tool: [], mac: [], everyone: [] } as Record<MemberKind, Candidate[]>;
    let draft: Draft = { ...draftOf(templateById("blank"), null), members: [] };
    const { rerender } = render(<NewPoolWho draft={draft} onDraft={(next) => (draft = next)} candidates={candidates} onFix={() => undefined} />);
    fireEvent.click(screen.getByRole("radio", { name: /Apps/ }));
    const row = screen.getByTestId("new-pool-candidate");
    expect(row.textContent).toContain("Kimi Code · Lab");
    expect(within(row).getByTestId("shell-only").getAttribute("aria-description")).toBe(why);
    fireEvent.click(within(row).getByRole("checkbox"));
    expect(draft.members.map((member) => [member.ref, member.kind])).toEqual([["app:cli:kimi-1", "app"]]);
    rerender(<NewPoolWho draft={draft} onDraft={(next) => (draft = next)} candidates={candidates} onFix={() => undefined} />);
    expect(within(screen.getByTestId("new-pool-candidate")).getByRole("checkbox")).toHaveProperty("checked", true);
  });
});

// D-126: the Apps as logo cards, everyone as a switch, what's open now.
describe("Who's in's Apps as cards (D-126)", () => {
  const app = (n: number, group: Candidate["group"] = "all"): Candidate => ({ ref: `app:cli:a${n}`, kind: "app", label: `App ${n} · Work`, detail: "none open now", group, reach: "later", blocked: null, lastMs: 0 });

  it("each App is a checkbox card with its logo; open ones first, a page of six, then Show all", () => {
    const rows = [app(1), app(2), app(3), app(4), app(5), app(6), app(7, "open"), app(8)];
    const candidates = { session: [], app: rows, account: [], tool: [], mac: [], everyone: [] } as Record<MemberKind, Candidate[]>;
    render(<NewPoolWho draft={{ ...draftOf(templateById("blank"), null), members: [] }} onDraft={() => undefined} candidates={candidates} onFix={() => undefined} />);
    const cards = screen.getAllByTestId("new-pool-candidate");
    expect(cards).toHaveLength(APP_CARDS);
    expect(cards[0]!.dataset.ref).toBe("app:cli:a7");
    expect(within(cards[0]!).getByRole("checkbox", { name: "App 7 · Work" })).toBeTruthy();
    expect(within(cards[0]!).getByTestId("member-logo")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Show all 8 apps" }));
    expect(screen.getAllByTestId("new-pool-candidate")).toHaveLength(8);
  });

  it("every new session in the projects is a switch: off takes everyone out, on puts it back", () => {
    let draft: Draft = draftOf(templateById("blank"), "p-web");
    const none = { session: [], app: [], account: [], tool: [], mac: [], everyone: [] } as Record<MemberKind, Candidate[]>;
    const view = render(<NewPoolWho draft={draft} onDraft={(next) => (draft = next)} candidates={none} onFix={() => undefined} where="web" />);
    const all = screen.getByRole("switch", { name: "Every new session in web" });
    expect(all.getAttribute("aria-checked")).toBe("true");
    fireEvent.click(all);
    expect(draft.members.some((member) => member.ref === "all")).toBe(false);
    view.rerender(<NewPoolWho draft={draft} onDraft={(next) => (draft = next)} candidates={none} onFix={() => undefined} where="web" />);
    fireEvent.click(screen.getByRole("switch", { name: "Every new session in web" }));
    expect(draft.members.find((member) => member.ref === "all")?.role).toBe("member");
  });

  it("lists the sessions open now that a chosen App or everyone brings in, and who brings each", () => {
    const base = MOCK_SESSIONS.find((one) => one.status === "running") ?? MOCK_SESSIONS[0]!;
    const sessions: InstanceInfo[] = [
      { ...base, id: "s1", status: "running", project_id: "web", account_id: "work" },
      { ...base, id: "s2", status: "running", project_id: "web", account_id: "other" },
      { ...base, id: "s3", status: "running", project_id: "api", account_id: "work" },
    ];
    const apps = [{ id: "cli:work", kind: "cli", tool: "claude-code", appId: null, app: "Claude Code", profile: "Work", name: "Claude Code · Work", defaultName: "Claude Code · Work", named: false, railKey: "k", folder: { ...MOCK_ACCOUNTS[0]!, id: "work" }, accountId: null, place: null, slot: 0 } satisfies AppModel];
    const member = { ref: "app:cli:work", kind: "app" as const, label: "Claude Code · Work", role: "member" as const, level: null, reach: "later" as const };
    const draft = { every: false, projects: ["web"], members: [member] };
    expect(coveredSessions(draft, sessions, apps, "everyone in web").map((one) => [one.session.id, one.by])).toEqual([["s1", "In, with Claude Code"]]);
    const all = { ...draft, members: [member, { ...member, ref: "all", kind: "everyone" as const }] };
    expect(coveredSessions(all, sessions, apps, "everyone in web").map((one) => one.by)).toEqual(["In, with Claude Code", "In, as everyone in web"]);
  });

  // The in-app check: a CLI App's card fell back to its row's words
  // ("Claude Code CLI · Work Max", "1 open now") where Who's in read the
  // Apps without the folders; New pool's own Apps (with them) say what
  // it is, its profile and what's open in the pool's projects.
  it("a CLI App's card says Terminal, its profile and what's open here, from New pool's own Apps", async () => {
    render(<NewPool sessions={MOCK_SESSIONS} projects={MOCK_PROJECTS} accounts={MOCK_ACCOUNTS} project="proj-ai-juggler" onClose={() => undefined} />);
    for (let n = 0; n < 2; n += 1) fireEvent.click(screen.getByTestId("new-pool-next"));
    const card = (await screen.findAllByTestId("new-pool-candidate")).find((one) => one.dataset.ref === "app:cli:claude-work")!;
    expect(within(card).getByRole("checkbox", { name: "Claude Code CLI" })).toBeTruthy();
    expect(card.textContent).toContain("TerminalProfile Work Max");
    expect(card.textContent).toMatch(/open in ai-juggler$/);
  });

  it("says what's open of an App in the pool's projects", () => {
    expect(openInWords(1, "web")).toBe("1 open in web");
    expect(openInWords(0, "every project")).toBe("Nothing open in any project");
  });
});
