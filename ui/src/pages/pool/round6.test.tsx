/**
 * AJ-E3011 round 6 (Sam's round-5 done-gate and Casey's runs, the
 * coordinator's list): words for why the connector can't go in, never
 * "done" or "ready" while something essential can't work, chosen members
 * never dropped silently, the project's New pool door, Who leads saying
 * why an App can't lead, the templates folded from a project, the first
 * prompt to copy, and the setup's skeleton.
 */
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { resetToasts } from "../../components/Toast";
import { ToastHost } from "../../components/ToastStack";
import { forgetSharedReads } from "../../lib/shared-read";
import { reachOf, statusLine, unavailableWords } from "../../lib/connector";
import { resetPreviewConnector, type ConnectorStatusInfo } from "../../lib/ipc/connector";
import { resetPreviewPools } from "../../lib/ipc/pools";
import { draftOf, makePool, type Draft } from "../../lib/new-pool";
import { doneNewPoolRequest, requestNewPool, takeNewPoolRequest } from "../../lib/new-pool-request";
import { setupBlockers } from "../../lib/pool-setup";
import { firstPrompt } from "../../lib/pool-sentences";
import { templateById } from "../../lib/pool-templates";
import { sayNotAdded } from "../../screens/pools/NewPoolFrame";
import { NewPoolLead } from "../../screens/pools/NewPoolLead";
import { NewPoolTemplate } from "../../screens/pools/NewPoolTemplate";
import { NewPoolReview } from "../../screens/pools/NewPoolReview";
import { SetupSkeleton } from "./EmptyPoolSetup";
import { PoolChecklist } from "./setup/PoolChecklist";
import { FirstPrompt } from "./setup/FirstPrompt";

const status = (over: Partial<ConnectorStatusInfo>): ConnectorStatusInfo => ({ app_id: "app:claude-app:/x", state: "unavailable", changed: [], why_not: "AI Juggler's juggler-agent isn't beside it, or isn't safe to run", folder: null, picked_up: null, attests: false, can_wake: false, ...over });
const bridge = (over: Record<string, unknown>) => ({ asked: true, on: true, available: true, off_projects: [], off_profiles: [], ...over });

beforeEach(() => {
  forgetSharedReads();
  resetPreviewConnector();
  resetToasts();
  Object.assign(window, { __AIJ_CONNECTOR__: undefined, __AIJ_POOL_JOIN_DROPS__: undefined, __AIJ_POOLS__: undefined });
  resetPreviewPools();
});
afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

describe("round 6 #1: why the connector can't go in, in the user's words", () => {
  it("never the backend's text: the agent program's reason (missing, a folder others can write), a code when there is one, else one plain line", () => {
    expect(unavailableWords(status({}), bridge({ available: false, unavailable: "missing" }))).toBe("This build of AI Juggler doesn't include its helper.");
    expect(unavailableWords(status({}), bridge({ available: false, unavailable: "unsafe_location", unsafe_folder: "~/Downloads" }))).toBe("AI Juggler's helper can't run from a folder others can write (~/Downloads). Move AI Juggler to Applications, then open it again.");
    expect(unavailableWords({ ...status({}), ...{ unavailable: "not_offered" } }, bridge({}))).toBe("AI Juggler can't add its connector to this app yet: it can't tell this app's sessions apart.");
    const plain = unavailableWords(status({}), bridge({}));
    expect(plain).toBe("AI Juggler can't add its connector to this app yet.");
    expect(statusLine(status({}), bridge({}))).not.toContain("juggler-agent");
  });
});

describe("round 6 #2: never done or ready while something essential can't work", () => {
  const rows = [
    { step: "where" as const, done: true, byDefault: true },
    { step: "who" as const, done: true, byDefault: false },
  ];
  it("the team tools that can't run, and a desktop member without a connector, are said; the checklist says Can't work yet", () => {
    const blockers = setupBlockers({ kind: "missing" }, [{ name: "Claude app · Work Max", reach: reachOf({ member: "app:x", app_id: "x", connector: "unavailable", connected: false, wake: "next_turn" }) }]);
    expect(blockers).toEqual(["The team tools can't run, so no one here can message: not in this build of AI Juggler.", "Claude app · Work Max can't talk here yet: its app can't take AI Juggler's connector yet."]);
    render(<PoolChecklist rows={rows} onOpen={() => undefined} ready next="Planner leads" blockers={blockers} />);
    expect(screen.queryByTestId("pool-ready")).toBeNull();
    expect(screen.getByTestId("pool-checklist-count").textContent).toBe("Can't work yet");
    expect(screen.getByTestId("pool-checklist").textContent).not.toMatch(/\d of \d done/);
    expect(screen.getAllByTestId("pool-checklist-blocked")[0]!.textContent).toContain("Claude app · Work Max can't talk here yet");
    cleanup();
    render(<PoolChecklist rows={rows} onOpen={() => undefined} compact blockers={blockers} />);
    expect(screen.getByTestId("pool-checklist-line").textContent).toContain("Can't work yet: The team tools can't run");
  });
});

describe("round 6 #2 on Review: what can't work is said before it's made", () => {
  it("a desktop App whose connector can't go in, in words", async () => {
    Object.assign(window, {
      __AIJ_CONNECTOR__: { "app:claude-app:/Users/you/Profiles/Work": { state: "unavailable" } },
      __AIJ_PLACES__: { places: [{ id: "app:claude-app:/Users/you/Profiles/Work", app_id: "claude-app", app: "Claude", instance: "Work", profile: "Work Max", label: "Claude · Work Max", account_id: "claude-work", app_pid: null, lost: false, keep_alive: false, paused: null, seen_ms: 0, cannot_start: null, recent: [], found: false }] },
    });
    const draft: Draft = { ...draftOf(templateById("blank"), "web"), members: [{ ref: "app:app:claude-app:/Users/you/Profiles/Work", kind: "app", label: "Claude app · Work Max", role: "member", level: null, reach: "later" }] };
    render(<NewPoolReview draft={draft} name="web team" where="web" />);
    await waitFor(() => expect(screen.getByTestId("new-pool-review-blocked").textContent).toContain("Claude app · Work Max can't talk here yet: AI Juggler can't add its connector to this app yet."));
  });
});

describe("round 6 #3: a chosen member is never dropped silently", () => {
  it("one the pool doesn't keep is said, with why; the rest are made", async () => {
    Object.assign(window, { __AIJ_POOL_JOIN_DROPS__: ["app:app:claude-app:/Users/you/Profiles/Side"], __AIJ_CONNECTOR__: { "app:claude-app:/Users/you/Profiles/Side": { state: "unavailable", why_not: "AI Juggler's juggler-agent isn't beside it, or isn't safe to run" } } });
    const app = (ref: string, label: string) => ({ ref, kind: "app" as const, label, role: "member" as const, level: null, reach: "later" as const });
    const draft: Draft = { ...draftOf(templateById("blank"), "web"), name: "web team", members: [app("app:app:claude-app:/Users/you/Profiles/Work", "Claude app · Work Max"), app("app:app:claude-app:/Users/you/Profiles/Side", "Claude app · Side")] };
    const { notAdded } = await makePool(draft);
    expect(notAdded.map((one) => one.label)).toEqual(["Claude app · Side"]);
    render(<ToastHost />);
    await act(async () => sayNotAdded(notAdded, bridge({ available: false, unavailable: "missing" })));
    const toast = await screen.findByTestId("new-pool-not-added-toast");
    expect(toast.textContent).toContain("Claude app · Side isn't in the pool");
    expect(toast.textContent).toContain("This build of AI Juggler doesn't include its helper.");
    expect(toast.textContent).not.toContain("juggler-agent");
  });
});

describe("round 6 #4: a project's New pool request waits for its setup", () => {
  it("a screen that mounts again still opens it; it's done once the setup closes, and too old after 5 s", () => {
    vi.useFakeTimers();
    requestNewPool({ template: "blank", project: "web" });
    expect(takeNewPoolRequest()).toEqual({ template: "blank", project: "web" });
    expect(takeNewPoolRequest()).toEqual({ template: "blank", project: "web" });
    doneNewPoolRequest();
    expect(takeNewPoolRequest()).toBeNull();
    requestNewPool({ template: "blank", project: "web" });
    vi.advanceTimersByTime(6_000);
    expect(takeNewPoolRequest()).toBeNull();
  });
});

describe("round 6 #5: Who leads says why an App can't lead, and why a session elsewhere can't", () => {
  it("a desktop App whose connector can't go in is greyed with the reason; a session another app runs is explained", async () => {
    Object.assign(window, { __AIJ_CONNECTOR__: { "app:claude-app:/Users/you/Profiles/Work": { state: "unavailable" } } });
    const draft: Draft = {
      ...draftOf(templateById("blank"), "web"),
      members: [
        { ref: "app:app:claude-app:/Users/you/Profiles/Work", kind: "app", label: "Claude app · Work Max", role: "member", level: null, reach: "later" },
        { ref: "conversation:claude-side/c1", kind: "session", label: "Plan the trip", role: "member", level: null, reach: "watched" },
      ],
    };
    render(<NewPoolLead draft={draft} onDraft={() => undefined} sessions={[]} nameOf={(_, label) => label} where="web" />);
    const row = await screen.findByText("Claude app · Work Max's next session");
    await waitFor(() => expect(row.closest("label")?.querySelector("input")?.disabled).toBe(true));
    expect(row.closest("label")?.textContent).toContain("Can't lead: AI Juggler can't add its connector to this app yet.");
    expect(screen.getByTestId("new-pool-lead-elsewhere").textContent).toContain("Plan the trip runs in another app, so it can't lead from here");
    expect(screen.queryByTestId("new-pool-lead-add-app")).toBeNull();
  });
});

describe("round 6 #7, #10, #11", () => {
  it("#7: from a project, Start from scratch is open and the templates are under a closed Or start from a template", () => {
    render(<NewPoolTemplate chosen="blank" onChoose={() => undefined} first fold />);
    const fold = screen.getByTestId("new-pool-templates-fold");
    expect(fold.hasAttribute("open")).toBe(false);
    expect(fold.textContent).toContain("Or start from a template");
    expect(screen.getAllByTestId("new-pool-template").filter((card) => !fold.contains(card)).map((card) => card.getAttribute("data-template"))).toEqual(["blank"]);
  });

  it("#10: what to tell the lead first, from the roles, with Copy", async () => {
    const shape = { members: [{ ref: "app:cli:work", label: "Claude Code · Work", role: "member" as const, level: null, later: true }, { ref: "app:cli:codex", label: "Codex · Personal", role: "reviewer" as const, level: null, later: true }], leadNext: "app:cli:work" };
    expect(firstPrompt(shape, "web")).toEqual({ to: "Claude Code · Work", words: "Plan the work in web, hand out tasks, and ask Codex to review." });
    expect(firstPrompt({ members: [], leadNext: null }, "web")).toBeNull();
    const writeText = vi.fn(() => Promise.resolve());
    Object.assign(navigator, { clipboard: { writeText } });
    render(<FirstPrompt shape={shape} where="web" />);
    expect(screen.getByTestId("first-prompt").textContent).toContain('Then tell Claude Code · Work:');
    await act(async () => fireEvent.click(screen.getByTestId("first-prompt-copy")));
    expect(writeText).toHaveBeenCalledWith("Plan the work in web, hand out tasks, and ask Codex to review.");
  });

  it("#11: the setup's skeleton names the pool at once", () => {
    render(<SetupSkeleton title="Set up web's own pool" />);
    expect(screen.getByTestId("pool-page-loading").textContent).toContain("Set up web's own pool");
  });
});

