/**
 * AJ-E3007, AJ-E3011, D-126: New pool's Who's in — paged, the shell-only
 * App said, and (D-126) the Apps as logo cards; the rc3 critique: no kind
 * switch, the mock-up's sections in order, and two Apps of one tool told
 * apart by what differs.
 */
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { draftOf, type Candidate, type Draft, type DraftMember } from "../../lib/new-pool";
import type { MemberKind } from "../../lib/ipc/pool-model";
import { templateById } from "../../lib/pool-templates";
import { CANDIDATES_PAGE, NewPoolWho } from "./NewPoolWho";
import { APP_CARDS, coveredSessions, openInWords } from "./NewPoolWhoApps";
import { previewNodes } from "./preview-model";
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
    // Earlier sessions fold under the ones open now.
    fireEvent.click(screen.getByRole("button", { name: /^Earlier sessions, or in another project/ }));
    const list = screen.getByTestId("new-pool-sessions-earlier");
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
    const row = screen.getByTestId("new-pool-candidate");
    // A lone App of its tool goes by its app word (the rc3 critique: its short name).
    expect(within(row).getByRole("checkbox", { name: "Kimi Code" })).toBeTruthy();
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
    expect(within(cards[0]!).getByRole("checkbox", { name: "App 7" })).toBeTruthy();
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
  // it is and what's open in the pool's projects. The rc3 critique: two
  // cards read "Claude Code CLI"; each leads with its profile now, as the
  // preview names it.
  it("a CLI App's card leads with its profile, then Claude Code CLI · Terminal and what's open here, from New pool's own Apps", async () => {
    render(<NewPool sessions={MOCK_SESSIONS} projects={MOCK_PROJECTS} accounts={MOCK_ACCOUNTS} project="proj-ai-juggler" onClose={() => undefined} />);
    for (let n = 0; n < 2; n += 1) fireEvent.click(screen.getByTestId("new-pool-next"));
    const apps = screen.getByTestId("new-pool-apps");
    await waitFor(() => expect(within(apps).getAllByTestId("new-pool-candidate").some((one) => one.dataset.ref === "app:cli:claude-work")).toBe(true));
    const card = within(apps).getAllByTestId("new-pool-candidate").find((one) => one.dataset.ref === "app:cli:claude-work")!;
    expect(within(card).getByRole("checkbox", { name: "Work Max" })).toBeTruthy();
    expect(card.textContent).toContain("Work MaxClaude Code CLI · Terminal");
    expect(card.textContent).not.toContain("Profile Work Max");
    expect(card.textContent).toMatch(/open in ai-juggler$/);
    const other = within(apps).getAllByTestId("new-pool-candidate").find((one) => one.dataset.ref === "app:cli:claude-personal")!;
    expect(within(other).getByRole("checkbox", { name: "Personal" })).toBeTruthy();
  });

  it("says what's open of an App in the pool's projects", () => {
    expect(openInWords(1, "web")).toBe("1 open in web");
    expect(openInWords(0, "every project")).toBe("Nothing open in any project");
  });
});

// The rc3 critique (the maintainer: "cryptic"): Who's in had an "Apps N |
// Sessions N | More" switch and a line explaining it. Now every kind is on
// the page in the mock-up's order, and nothing the switch reached is lost.
describe("Who's in without a kind switch (the rc3 critique)", () => {
  const base = MOCK_SESSIONS.find((one) => one.status === "running") ?? MOCK_SESSIONS[0]!;
  const sessions: InstanceInfo[] = [{ ...base, id: "s1", status: "running", project_id: "web", account_id: "work" }];
  const app = (id: string, label: string, profile: string, folder: string, kind: "cli" | "desktop" = "cli", word = "Claude Code CLI"): AppModel => ({ id, kind, tool: "claude-code", appId: kind === "desktop" ? "claude-app" : null, app: word, profile, name: label, defaultName: label, named: false, railKey: id, folder: { ...MOCK_ACCOUNTS[0]!, id: folder }, accountId: null, place: null, slot: 0 });
  const appRow = (one: AppModel): Candidate => ({ ref: `app:${one.id}`, kind: "app", label: one.name, detail: "none open now", group: "all", reach: "later", blocked: null, lastMs: 0 });
  const row = (ref: string, kind: MemberKind, label: string, group: Candidate["group"], extra: Partial<Candidate> = {}): Candidate => ({ ref, kind, label, detail: `${label}'s line`, group, reach: kind === "session" ? "full" : "later", blocked: null, lastMs: 0, ...extra });
  const work = app("cli:work", "Claude Code CLI · Work Max", "Work Max", "work");
  const candidates = {
    app: [appRow(work)],
    session: [
      row("session:s1", "session", "Draft the settings page", "open"),
      row("session:s2", "session", "Add the export button", "open"),
      row("session:s3", "session", "Yesterday's spike", "recent"),
      row("session:s4", "session", "Api cleanup", "open", { blocked: "Works in api, not in this pool's projects.", fix: { label: "Add api", action: "scope", arg: "api" } }),
    ],
    account: [row("account:a1", "account", "Work Max account", "open")],
    tool: [row("tool:codex", "tool", "Codex", "all")],
    mac: [row("mac:m1", "mac", "This Mac", "open")],
    everyone: [],
  } as Record<MemberKind, Candidate[]>;
  const member = (one: Candidate): DraftMember => ({ ref: one.ref, kind: one.kind, label: one.label, role: "member", level: null, reach: one.reach });

  it("has no switch: Your apps, then the sessions open now in web with their checks, then More ways to add, in that order", () => {
    let draft: Draft = { ...draftOf(templateById("blank"), "web"), projects: ["web"], members: [member(candidates.app[0]!)] };
    render(<NewPoolWho draft={draft} onDraft={(next) => (draft = next)} candidates={candidates} onFix={() => undefined} where="web" sessions={sessions} apps={[work]} />);
    expect(screen.queryByRole("radiogroup", { name: "Kind of member" })).toBeNull();
    expect(screen.queryByTestId("new-pool-kind-line")).toBeNull();
    const yours = screen.getByRole("region", { name: /^Your apps/ });
    const open = screen.getByRole("region", { name: "Sessions open now in web" });
    expect(within(open).getByRole("heading").textContent).toBe("Sessions open now in web2");
    const more = screen.getByRole("button", { name: "More ways to add: a Mac, an account or a tool · for bigger setups" });
    expect(yours.compareDocumentPosition(open) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(open.compareDocumentPosition(more) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    // Each open session its own row with its check: in with its App
    // (the mock's quiet check), or ticked by itself.
    const rows = within(open).getAllByRole("checkbox");
    expect(rows.map((one) => one.textContent)).toEqual(["Draft the settings pageDraft the settings page's lineIn, with Claude Code CLI", "Add the export buttonAdd the export button's linefull tools"]);
    expect(rows.map((one) => one.getAttribute("aria-checked"))).toEqual(["false", "false"]);
    expect(within(open).getAllByTestId("new-pool-candidate")[0]!.dataset.brought).toBe("true");
    fireEvent.click(rows[1]!);
    expect(draft.members.map((one) => one.ref)).toEqual(["app:cli:work", "session:s2"]);
  });

  it("keeps what the switch reached: earlier sessions and other projects' under a fold, Accounts, Tools and Macs under More ways to add (closed)", () => {
    let draft: Draft = { ...draftOf(templateById("blank"), "web"), projects: ["web"], members: [] };
    const fixed: string[] = [];
    render(<NewPoolWho draft={draft} onDraft={(next) => (draft = next)} candidates={candidates} onFix={(one) => fixed.push(one.ref)} where="web" sessions={sessions} apps={[work]} />);
    const earlier = screen.getByRole("button", { name: "Earlier sessions, or in another project · 2" });
    expect(earlier.getAttribute("aria-expanded")).toBe("false");
    fireEvent.click(earlier);
    const folded = screen.getByTestId("new-pool-sessions-earlier");
    expect(within(folded).getAllByRole("checkbox").map((one) => one.textContent?.slice(0, 15))).toEqual(["Api cleanupApi ", "Yesterday's spi"]);
    fireEvent.click(within(folded).getByRole("button", { name: "Add api" }));
    expect(fixed).toEqual(["session:s4"]);
    const more = screen.getByRole("button", { name: /^More ways to add/ });
    expect(more.getAttribute("aria-expanded")).toBe("false");
    expect(screen.queryByRole("region", { name: /^Accounts/ })).toBeNull();
    fireEvent.click(more);
    for (const [name, label] of [["Accounts", "Work Max account"], ["Tools", "Codex"], ["Macs", "This Mac"]] as const) {
      expect(within(screen.getByRole("region", { name })).getByRole("checkbox", { name: new RegExp(`^${label}`) })).toBeTruthy();
    }
    fireEvent.click(within(screen.getByRole("region", { name: "Tools" })).getByRole("checkbox"));
    expect(draft.members.map((one) => [one.ref, one.kind])).toEqual([["tool:codex", "tool"]]);
  });

  it("opens More ways to add when the pool starts with an Account", () => {
    render(<NewPoolWho draft={{ ...draftOf(templateById("rules-by-account"), "web"), members: [] }} onDraft={() => undefined} candidates={candidates} onFix={() => undefined} where="web" />);
    expect(screen.getByRole("button", { name: /^More ways to add/ }).getAttribute("aria-expanded")).toBe("true");
    expect(screen.getByRole("region", { name: "Accounts" })).toBeTruthy();
  });

  it("names two Apps of one tool by their profiles, as the preview does, and a lone one by its app word", () => {
    const personal = app("cli:personal", "Claude Code CLI · Personal", "Personal", "personal");
    const desktop = app("app:claude-app:/p/Work", "Claude app · Work", "Work", "desktop-work", "desktop", "Claude app");
    const apps = [work, personal, desktop];
    const rows = apps.map(appRow);
    const draft: Draft = { ...draftOf(templateById("blank"), "web"), projects: ["web"], members: rows.map(member) };
    render(<NewPoolWho draft={draft} onDraft={() => undefined} candidates={{ ...candidates, app: rows, session: [] }} onFix={() => undefined} where="web" sessions={[]} apps={apps} />);
    const cards = within(screen.getByTestId("new-pool-apps")).getAllByRole("checkbox");
    const names = cards.map((one) => document.getElementById(one.getAttribute("aria-labelledby")!)!.textContent);
    expect(names).toEqual(["Work Max", "Personal", "Claude app"]);
    const nodes = previewNodes(draft, (_, label) => label, "web", { apps, sessions: [] }).members.map((node) => node.name);
    expect(nodes).toEqual(names);
    const text = within(screen.getByTestId("new-pool-apps")).getAllByTestId("new-pool-candidate").map((one) => one.textContent);
    expect(text[0]).toContain("Work MaxClaude Code CLI · Terminal");
    expect(text[0]).not.toContain("Profile Work Max");
    expect(text[1]).toContain("PersonalClaude Code CLI · Terminal");
    expect(text[1]).not.toContain("Profile Personal");
    expect(text[2]).toContain("Claude appDesktop appProfile Work");
  });
});
