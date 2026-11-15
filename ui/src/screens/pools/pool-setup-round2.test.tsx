/**
 * AJ-E3011 round 2 (docs/design/pool-setup-0.4.6.md, "Round 2"): each
 * friction point the two after-walks logged, with its fix.
 */
import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { resetPreviewAgentBridge } from "../../lib/ipc/agent-bridge";
import { draftOf, type Candidate, type Draft } from "../../lib/new-pool";
import { POOL_TEMPLATES, templateById } from "../../lib/pool-templates";
import { INTRO } from "../../lib/pool-words";
import { cliFolderOf, leadFolder, startNextLead } from "../../lib/start-lead";
import { draftPlaces, teamTools, teamToolsLine } from "../../lib/team-tools";
import type { MemberKind } from "../../lib/ipc/pool-model";
import { pageTabs } from "../../pages/registry";
import { RuleNodes } from "../../pages/pool/RuleNodes";
import { MemberOwnLevel } from "../../pages/pool/setup/OwnLevelChoice";
import { RoleCards } from "../../pages/pool/setup/RoleCards";
import { NewPoolEveryone } from "./NewPoolEveryone";
import { NewPoolReview } from "./NewPoolReview";
import { NewPoolWho, whoInWords } from "./NewPoolWho";

beforeEach(() => resetPreviewAgentBridge());
afterEach(cleanup);

const app = (ref: string, label: string, role: "member" | "reviewer" = "member") => ({ ref, kind: "app" as const, label, role, level: null, reach: "later" as const });

describe("AJ-E3011 round 2", () => {
  it("#1: Review says the team tools as a line, decided before Make pool, and names a project they're off for", async () => {
    const draft: Draft = { ...draftOf(templateById("blank"), "p-web"), members: [app("app:cli:work", "Claude Code · Work")] };
    const review = render(<NewPoolReview draft={draft} name="web team" where="web" projects={[{ id: "p-web", name: "web" }]} />);
    // Round 3: Review's line and the card are the diagram's own.
    expect((await review.findByTestId("new-pool-review-tools-line")).textContent).toBe("Off: they can't message yet.");
    expect(review.getByTestId("new-pool-tools").textContent).toContain("switch it off for a project or an app in Settings › Agent access");
    fireEvent.click(review.getByTestId("new-pool-tools-on"));
    await review.findByText("On: they can message each other and share notes and tasks.");
    expect(review.queryByTestId("new-pool-tools")).toBeNull();
    // On, but off for web: Review names it.
    const off = teamTools({ asked: true, on: true, available: true, off_projects: ["p-web"], off_profiles: [] }, draftPlaces(draft, [{ id: "p-web", name: "web" }]).projects, []);
    expect(teamToolsLine(off)).toBe("Off for web: they can't message there.");
    // A CLI App's folder is the switch an App has.
    expect(draftPlaces(draft, []).apps).toEqual([{ id: "work", name: "Claude Code · Work" }]);
  });

  it("#2: an App whose next session leads is one card on the diagram, with Start one now (#3) that only opens New session", () => {
    const rule = { member: "app:cli:work", kind: "app", role: "member", level: null, joined_at: 1, sessions: 0, session_ids: [] };
    const starts: string[] = [];
    const opened: unknown[] = [];
    const listen = (event: Event) => opened.push(event instanceof CustomEvent ? event.detail : null);
    window.addEventListener("aij:new-session", listen);
    render(
      <RuleNodes
        rules={[rule]}
        watched={[]}
        labelOf={() => "Claude Code · Work"}
        selection={{ kind: "none" }}
        onSelect={() => undefined}
        lead
        leadNext={{ ref: "app:cli:work", name: "Claude Code · Work" }}
        where="web"
        onStartLead={(member) => {
          starts.push(member);
          startNextLead(member, "/work/web");
        }}
      />,
    );
    // No Lead placeholder beside the App's own card.
    expect(screen.queryByTestId("canvas-lead-card")).toBeNull();
    const card = screen.getByTestId("canvas-rule-node");
    expect(within(card).getByTestId("canvas-rule-leads-next").textContent).toBe("Leads next");
    expect(within(card).getByTestId("canvas-rule-next-line").textContent).toBe("Its next session leads · its others join as Member");
    expect(card.textContent).toContain("No session open yet · new ones in web join");
    expect(card.textContent).not.toContain("Member ");
    // Nothing starts until the click, and the click only opens the sheet.
    expect(opened).toEqual([]);
    fireEvent.click(screen.getByTestId("canvas-rule-start-lead"));
    window.removeEventListener("aij:new-session", listen);
    expect(starts).toEqual(["app:cli:work"]);
    expect(opened).toEqual([{ accountId: "work", dir: "/work/web" }]);
  });

  it("#3: Start one now opens on the App's own folder, in the pool's one project; a desktop App doesn't start here", () => {
    expect(cliFolderOf("app:cli:work")).toBe("work");
    expect(cliFolderOf("app:app:host:folder")).toBeNull();
    const projects = [{ id: "p-web", root: "/work/web" }];
    expect(leadFolder({ scope_all: false, scope: ["p-web"] }, projects)).toBe("/work/web");
    expect(leadFolder({ scope_all: true, scope: [] }, projects)).toBeNull();
    expect(leadFolder({ scope_all: false, scope: ["p-web", "p-docs"] }, projects)).toBeNull();
  });

  it("#2, #10: Everyone else names the App's role line and labels each picker", () => {
    const draft: Draft = { ...draftOf(templateById("blank"), "p-web"), members: [app("app:cli:work", "Claude Code · Work"), app("app:cli:codex", "Codex · Personal", "reviewer")], leadNext: "app:cli:work" };
    render(<NewPoolEveryone draft={draft} onDraft={() => undefined} where="web" nameOf={(_, label) => label} />);
    const rows = screen.getAllByTestId("new-pool-member-row");
    expect(within(rows[0]!).getByTestId("new-pool-member-leads-next").textContent).toBe("Leads next");
    expect(within(rows[0]!).getByTestId("new-pool-member-sentence").textContent).toBe("Claude Code · Work: its next session leads; its others join as Member and may Work together, as the pool.");
    // Round 3 (#1): its Lead reads as its next session's, in the same picker.
    expect(within(rows[0]!).getByTestId("new-pool-member-role").textContent).toBe("Role:Lead: its next session leads");
    expect(within(rows[1]!).getByTestId("new-pool-member-role").textContent).toContain("Role:");
    expect(within(rows[1]!).getByTestId("new-pool-member-level").textContent).toContain("On its own:");
    expect(within(rows[1]!).getByTestId("new-pool-member-level").textContent).toContain("Same as the pool (Work together)");
    // Its others, by name, in the roles' reference; not a second Member.
    expect(within(screen.getByTestId("new-pool-role-cards")).getByText(/Claude Code · Work's other sessions/)).toBeTruthy();
  });

  it("#10: a member's own choice says On its own: before its value", () => {
    render(<MemberOwnLevel name="Codex" value={null} poolLevel="work_together" onChange={() => undefined} testId="own" />);
    expect(screen.getByTestId("own").textContent).toBe("On its own:Same as the pool (Work together)");
  });

  it("#6, #8: Who's in says who in words, and keeps the rarer kinds under More", () => {
    expect(whoInWords([app("app:cli:a", "A"), app("app:cli:b", "B"), { ref: "all", kind: "everyone" }], "web")).toBe("2 apps, and every new session in web");
    expect(whoInWords([{ ref: "all", kind: "everyone" }], "web")).toBe("Every new session in web");
    expect(whoInWords([], "web")).toBe("No one yet");
    const none = { session: [], app: [], account: [], tool: [], mac: [], everyone: [] } as Record<MemberKind, Candidate[]>;
    const draft: Draft = { ...draftOf(templateById("blank"), "p-web"), members: [{ ref: "all", kind: "everyone", label: "Everyone", role: "member", level: null, reach: "later" }] };
    render(<NewPoolWho draft={draft} onDraft={() => undefined} candidates={none} onFix={() => undefined} where="web" nameOf={(ref, label) => (ref === "all" ? "Everyone in web" : label)} />);
    expect(screen.getByTestId("new-pool-kind-label").textContent).toBe("Add whole apps — every session they start here joins — or single sessions open now");
    expect(within(screen.getByTestId("new-pool-kind")).getAllByRole("radio").map((one) => one.textContent)).toEqual(["Apps 0", "Sessions 0", "More"]);
    expect(screen.getByTestId("new-pool-side-everyone").textContent).toBe("Everyone in webEvery new session in web joins by itself, now and later.");
    expect(screen.queryByTestId("new-pool-kind-picker")).toBeNull();
    fireEvent.click(within(screen.getByTestId("new-pool-kind")).getByRole("radio", { name: "More" }));
    expect(screen.getByTestId("new-pool-kind-picker").textContent).toContain("Kind:Accounts 0");
    expect(within(screen.getByTestId("new-pool-kind")).getByRole("radio", { name: "More" }).getAttribute("aria-checked")).toBe("true");
  });

  it("#9: the roles are a plain reference, not cards that look like buttons", () => {
    render(<RoleCards holders={{}} />);
    const list = screen.getByRole("list", { name: "What each role does" });
    for (const item of within(list).getAllByTestId("role-card")) expect(item.className).not.toMatch(/bg-|rounded|hover:/);
  });

  it("#11: the templates' chips are plain words", () => {
    const words = POOL_TEMPLATES.flatMap((template) => template.chips.map((chip) => chip.label));
    expect(words.join(" | ")).not.toMatch(/talk:|level:|↔|sealed|spans|member$|docs/);
    expect(words).toEqual(expect.arrayContaining(["everyone may message", "look only", "members ask before messaging each other", "kept apart"]));
  });

  it("#12, #13: a pool is defined in the intro's first line, and it's a diagram", () => {
    expect(INTRO.what).toBe("A pool is a team of your AI sessions working on the same projects.");
    expect(INTRO.after).toContain("diagram");
  });

  it("#16: the Subagents and Agent access tabs say what they are", () => {
    const [subagents] = pageTabs("conversation", { hasUsage: true, live: false, hasSubagents: true }).filter((tab) => tab.id === "subagents");
    expect(subagents?.hint).toMatch(/^Helpers this session's tool started/);
  });
});
