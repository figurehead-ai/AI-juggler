/**
 * D-136 (AJ-E3804..E3808, E3815, E3819's UI part): Bot's states from the
 * approved mock-up, and the rules each one rests on: the lead follows each
 * row's own state; the scope chip rides on the question and × widens it;
 * with no model the look-ups and the box stay; clearing asks first and
 * names what stays; a watch is written only by the confirm card's click;
 * forget shows FORGET_AND_BACKUPS as the backend gives it.
 */
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { AiJobInfo } from "../../lib/ipc/ai";
import { BOT_NOW, R, STATES } from "../../preview/bot";
import { CLEAR_AND_BACKUPS, FORGET_AND_BACKUPS, lead, leadText } from "../../lib/bot-words";
import { ask, botState, openBot, resetBot } from "../../lib/bot-store";

let job: AiJobInfo | null = null;
vi.mock("../../lib/ai-availability", () => ({ useAiJob: () => job, useAiJobs: () => (job ? [job] : []), isShown: () => true }));
vi.mock("../../lib/ai-models", () => ({ useModelsCatalog: () => null, useModelsInstalled: () => null, refreshModels: () => undefined }));

const { BotPanelBody } = await import("./BotDock");
const { BotPage } = await import("./BotPage");
const { BotForgetHost } = await import("./BotSheets");
const { BotDock } = await import("./BotDock");

function seed(name: keyof typeof STATES): void {
  const state = STATES[name]!;
  window.__AIJ_BOT__ = structuredClone(state.bot);
  window.__AIJ_WATCH_VIEW__ = state.watches ? structuredClone(state.watches) : {};
  job = state.jobs.find((info) => info.job === "chat") ?? null;
}

function panel(): void {
  act(() => openBot());
  render(<BotPanelBody onClear={() => undefined} onMemories={() => undefined} />);
}

beforeEach(() => {
  resetBot();
  vi.spyOn(Date, "now").mockReturnValue(BOT_NOW);
});
afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  delete window.__AIJ_BOT__;
  delete window.__AIJ_WATCH_VIEW__;
});

describe("the lead follows each row's own state", () => {
  it("one clause per state, counts first, and before the last", () => {
    expect(leadText(lead([R.web, R.login, R.task], "en"))).toBe("Here’s what I found: 1 session asks you, 1 looks stuck, and 1 task is blocked.");
    expect(leadText(lead([R.loginDone, R.apiDone, R.taskDone], "en"))).toBe("Here’s what I found: 2 sessions finished and 1 task is done.");
    expect(leadText(lead([R.apiRun, R.webRun], "en"))).toBe("Here’s what I found: 2 sessions are working.");
    expect(leadText(lead([R.apiDone, R.docs], "de"))).toBe("Das habe ich gefunden: 2 Sitzungen sind fertig.");
    expect(leadText(lead([R.work, R.personal], "en"))).toBe("Here’s what I found: 2 accounts.");
    expect(leadText(lead([], "en"))).toBe("I found nothing that matches right now.");
  });

  it("never one word for several states, and a past answer counts its state then", () => {
    const words = leadText(lead([R.web, R.login], "en"));
    expect(words).not.toMatch(/waiting on you|2 sessions/);
    expect(leadText(lead([{ ...R.loginDone, then_state: "stuck" }], "en"))).toBe("Here’s what I found: 1 session looks stuck.");
  });

  it("names the scope it was asked about", () => {
    expect(leadText(lead([R.apiRun], "en", { kind: "session", id: "s-api", name: "api", sessions: [] }))).toBe("Here’s what I found about “api”: 1 session is working.");
  });
});

describe("every state of the mock-up", () => {
  it("empty: what Bot does today, three questions, the Beta row where a model passes", async () => {
    seed("empty");
    panel();
    expect((await screen.findByTestId("bot-empty"))?.textContent ?? '').toContain("Doesn’t talk to your agents or change anything else");
    expect(screen.getAllByTestId("bot-example")).toHaveLength(3);
    expect(screen.getByTestId("bot-beta-row")).toBeTruthy();
    expect(screen.getByTestId("bot-input")).toBeTruthy();
  });

  it("lookup: the lead, the rows as source links, Looked at", async () => {
    seed("lookup");
    panel();
    expect((await screen.findByTestId("bot-lead"))?.textContent ?? '').toContain("1 session asks you, 1 looks stuck, and 1 task is blocked.");
    const rows = screen.getAllByTestId("bot-row");
    expect(rows.map((r) => r.dataset.state)).toEqual(["asks_you", "stuck", "blocked"]);
    expect(within(rows[0]!).getByTestId("bot-row-open").getAttribute("title")).toMatch(/^web Session · Agent app one · shop, Asks you/);
    expect((screen.getByTestId("bot-looked"))?.textContent ?? '').toContain("every session, task and pool · just now");
    expect((screen.getByTestId("bot-watch-count"))?.textContent ?? '').toContain("5");
  });

  it("dontknow: says why, the nearest record labelled, or the nearest screen", async () => {
    seed("dontknow");
    panel();
    const leads = await screen.findAllByTestId("bot-lead");
    expect((leads[0])?.textContent ?? '').toContain("I don’t know. AI Juggler doesn’t record how long tests take.");
    expect((leads[1])?.textContent ?? '').toContain("I don’t know: I couldn’t find anything about that in AI Juggler.");
    expect(screen.getByText(/The nearest record is about the/)).toBeTruthy();
    expect(screen.getByTestId("bot-shortcut-finished_today")).toBeTruthy();
  });

  it("beta: written by AI, checked, How this was made, This is wrong kept on this Mac", async () => {
    seed("beta");
    panel();
    const card = await screen.findByTestId("bot-written");
    expect((card)?.textContent ?? '').toContain("Written by AI · Beta");
    expect((card)?.textContent ?? '').toContain("Checked against the 2 records below");
    fireEvent.click(screen.getByTestId("bot-how-made"));
    expect((screen.getByTestId("bot-made"))?.textContent ?? '').toContain("Written by Apple’s built-in AI, on this Mac.");
    fireEvent.click(screen.getByTestId("bot-wrong"));
    await waitFor(() => expect(window.__AIJ_BOT__?.calls?.some(([c]) => c === "bot_wrong")).toBe(true));
    fireEvent.click(screen.getByTestId("bot-records-only"));
    expect(screen.queryByTestId("bot-written")).toBeNull();
    expect(screen.getByTestId("bot-lead")).toBeTruthy();
  });

  it("nomodel: look-ups still work, the shortcuts, and the question box stays", async () => {
    seed("nomodel");
    panel();
    expect((await screen.findByTestId("bot-no-model"))?.textContent ?? '').toContain("It doesn’t talk to your agents.");
    for (const s of ["needs_me", "finished_today", "runway", "running"]) expect(screen.getByTestId(`bot-shortcut-${s}`)).toBeTruthy();
    expect(screen.getByTestId("bot-input")).toBeTruthy();
    expect(screen.queryByTestId("bot-beta-row")).toBeNull();
  });

  it("remember: the echo with Edit and Undo; the receipt names the memory with Forget", async () => {
    seed("remember");
    panel();
    expect((await screen.findByTestId("bot-memory-echo"))?.textContent ?? '').toContain("I’ll remember: you call “web” the api agent.");
    fireEvent.click(screen.getAllByTestId("bot-looked")[0]!.querySelector("button")!);
    expect((screen.getByTestId("bot-receipt"))?.textContent ?? '').toContain("Your name for “web”: the api agent");
    expect(screen.getByText(/you call it the api agent/)).toBeTruthy();
  });

  it("newtopic and long: dividers for the topic and the day, Earlier pages", async () => {
    seed("long");
    panel();
    expect((await screen.findByTestId("bot-earlier"))?.textContent ?? '').toContain("Earlier");
    expect(screen.getAllByTestId("bot-topic").length).toBeGreaterThan(0);
    expect(screen.getAllByTestId("bot-day").length).toBeGreaterThan(0);
    expect(botState().turns).toHaveLength(20);
    await act(async () => {
      fireEvent.click(screen.getByTestId("bot-earlier"));
    });
    await waitFor(() => expect(botState().turns).toHaveLength(24));
  });

  it("search: finds in the conversation and marks the matches", async () => {
    seed("search");
    panel();
    await screen.findAllByTestId("bot-turn");
    fireEvent.click(screen.getByTestId("bot-search"));
    fireEvent.change(screen.getByTestId("bot-find-input"), { target: { value: "login" } });
    await waitFor(() => expect(screen.getAllByTestId("bot-mark").length).toBeGreaterThan(0));
  });

  it("full and memories: the page holds the same conversation and the remembers tab", async () => {
    seed("full");
    render(<BotPage onBack={() => undefined} />);
    expect(await screen.findAllByTestId("bot-turn")).toHaveLength(2);
    expect(screen.queryByText(/Longer tasks/)).toBeNull();
    fireEvent.click(screen.getByRole("radio", { name: /What Bot remembers/ }));
    expect((await screen.findByTestId("bot-memories"))?.textContent ?? '').toContain("You call “web” the api agent");
  });
});

describe("the scope chip", () => {
  it("rides on the question, and × asks about everything", async () => {
    seed("lookup");
    act(() => openBot({ kind: "session", id: "s-api", name: "api", sessions: [] }));
    render(<BotPanelBody onClear={() => undefined} onMemories={() => undefined} />);
    expect((screen.getByTestId("bot-scope-chip"))?.textContent ?? '').toContain("About: api");
    expect(screen.getByTestId("bot-input").getAttribute("placeholder")).toBe("Ask about api…");
    await act(async () => {
      await ask("What did it change today?");
    });
    const asked: unknown = window.__AIJ_BOT__?.calls?.filter(([c]) => c === "bot_ask").at(-1)?.[1];
    expect(Reflect.get(Object(Reflect.get(Object(asked), "scope")), "id")).toBe("s-api");
    fireEvent.click(screen.getByTestId("bot-scope-x"));
    expect(screen.queryByTestId("bot-scope-chip")).toBeNull();
    await act(async () => {
      await ask("Anything else stuck?");
    });
    const widened: unknown = window.__AIJ_BOT__?.calls?.filter(([c]) => c === "bot_ask").at(-1)?.[1];
    expect(Reflect.get(Object(widened), "scope")).toBeNull();
  });
});

describe("acting needs a confirm card", () => {
  it("Watch… builds the card from the row's id; nothing is written until Watch is clicked", async () => {
    seed("watch");
    panel();
    await screen.findAllByTestId("bot-row");
    fireEvent.mouseEnter(screen.getAllByTestId("bot-row")[1]!);
    fireEvent.click(screen.getAllByTestId("bot-row-watch")[1]!);
    await waitFor(() => expect(screen.getByTestId("bot-watch-echo").textContent).toContain("Tell you when “web” finishes. Once."));
    expect(window.__AIJ_WATCH_VIEW__?.calls ?? []).toHaveLength(0);
    fireEvent.click(screen.getByTestId("bot-watch-asks_you"));
    await waitFor(() => expect((screen.getByTestId("bot-watch-echo"))?.textContent ?? '').toContain("finishes, or asks you"));
    expect(window.__AIJ_WATCH_VIEW__?.calls ?? []).toHaveLength(0);
    fireEvent.click(screen.getByTestId("bot-watch-confirm"));
    await screen.findByTestId("bot-watch-done");
    const calls = window.__AIJ_WATCH_VIEW__?.calls ?? [];
    expect(calls).toHaveLength(1);
    const draft: unknown = calls[0]![1];
    expect(Reflect.get(Object(draft), "targets")).toEqual([{ kind: "session", id: "s-web" }]);
    expect(Reflect.get(Object(draft), "chatTurn")).toBe("t9");
    expect(Reflect.get(Object(draft), "conditions")).toEqual([{ code: "finishes" }, { code: "asks_you" }]);
  });

  it("Cancel writes nothing", async () => {
    seed("watch");
    panel();
    await screen.findAllByTestId("bot-row");
    fireEvent.click(screen.getAllByTestId("bot-row-watch")[0]!);
    await screen.findByTestId("bot-watch-card");
    fireEvent.click(screen.getByTestId("bot-watch-cancel"));
    expect(screen.queryByTestId("bot-watch-card")).toBeNull();
    expect(window.__AIJ_WATCH_VIEW__?.calls ?? []).toHaveLength(0);
  });
});

describe("clearing and forgetting ask first", () => {
  it("the clear sheet names what goes and what stays, with its own backup line", async () => {
    seed("clear");
    act(() => openBot());
    render(<BotDock />);
    await screen.findAllByTestId("bot-turn");
    fireEvent.click(screen.getByTestId("bot-more"));
    fireEvent.click(screen.getByTestId("bot-menu-clear"));
    const sheet = await screen.findByTestId("bot-clear");
    await waitFor(() => expect((sheet)?.textContent ?? '').toContain("312 questions"));
    expect((sheet)?.textContent ?? '').toContain("Your 4 notes and 5 watches stay.");
    expect((screen.getByTestId("bot-clear-backups"))?.textContent ?? '').toContain(CLEAR_AND_BACKUPS);
    expect(window.__AIJ_BOT__?.calls?.some(([c]) => c === "bot_clear")).toBeFalsy();
    fireEvent.click(screen.getByTestId("bot-clear-confirm"));
    await waitFor(() => expect(botState().turns).toHaveLength(0));
    expect(window.__AIJ_BOT__?.calls?.some(([c]) => c === "bot_clear")).toBe(true);
  });

  it("forget shows FORGET_AND_BACKUPS word for word, and only its button forgets", async () => {
    seed("forget");
    render(
      <>
        <BotPage onBack={() => undefined} />
        <BotForgetHost />
      </>,
    );
    fireEvent.click(screen.getByRole("radio", { name: /What Bot remembers/ }));
    const forget = await screen.findAllByTestId("bot-memory-forget");
    fireEvent.click(forget[0]!);
    await waitFor(() => expect((screen.getAllByTestId("bot-forget-backups")[0])?.textContent ?? '').toContain(FORGET_AND_BACKUPS));
    expect((screen.getAllByTestId("bot-forget")[0])?.textContent ?? '').toContain("Forget that you call “web” the api agent?");
    expect(window.__AIJ_BOT__?.calls?.some(([c]) => c === "bot_forget")).toBeFalsy();
    fireEvent.click(screen.getAllByTestId("bot-forget-confirm")[0]!);
    await waitFor(() => expect(window.__AIJ_BOT__?.calls?.find(([c]) => c === "bot_forget")?.[1]).toBe("alias:session:s-web"));
  });
});

describe("a planted order in a record's name", () => {
  it("is never shown: the row says its name was hidden", async () => {
    seed("lookup");
    window.__AIJ_BOT__!.turns![0]!.rows[0] = { ...R.web, title: "", name_hidden: true };
    panel();
    expect(await screen.findByText("A record with a hidden name")).toBeTruthy();
  });
});
