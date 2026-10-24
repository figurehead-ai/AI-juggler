/**
 * AJ-E2108 vitest — a session's last messages: read only once its page is
 * shown, once per session; labelled You / Claude / Claude asks; off in
 * Settings says so; a paired Mac's session is never read here; the answer
 * names where to answer it.
 */
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import * as ipc from "../../lib/ipc/conversations";
import { LastMessages } from "./LastMessages";
import type { SessionModel } from "./model";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

const model = (overrides: Partial<SessionModel> = {}): SessionModel => ({
  tool: "claude-code",
  profileId: "work",
  profileLabel: "Work",
  conversationId: "c1",
  sessionId: null,
  live: true,
  state: { words: "Asks you", tone: "warning", asks: true, since: 0 },
  places: [{ app: "Claude", app_id: "claude-app", profile: "Work", instance: "Work", pid: 42, kind: "desktop", app_pid: 41, label: "Claude · Work" }],
  viewedMs: null,
  cwd: "/w/app",
  model: null,
  ...overrides,
});

const shown = {
  state: "shown",
  redacted: 1,
  turns: [
    { speaker: "you", text: "Use the key •••" },
    { speaker: "assistant", text: "Keep the old layout?" },
  ],
};

describe("AJ-E2108 last messages", () => {
  it("are read only once the page is shown, and once", async () => {
    const read = vi.spyOn(ipc, "conversationsLastMessages").mockResolvedValue(shown);
    const view = render(<LastMessages model={model()} visible={false} />);
    expect(read).not.toHaveBeenCalled();
    view.rerender(<LastMessages model={model()} visible />);
    await waitFor(() => expect(screen.getAllByTestId("last-turn")).toHaveLength(2));
    view.rerender(<LastMessages model={model()} visible={false} />);
    view.rerender(<LastMessages model={model()} visible />);
    expect(read).toHaveBeenCalledTimes(1);
    expect(read).toHaveBeenCalledWith("work", "c1");
  });

  it("label who spoke, the question in amber, and say nothing is kept", async () => {
    vi.spyOn(ipc, "conversationsLastMessages").mockResolvedValue(shown);
    render(<LastMessages model={model()} visible />);
    const turns = await screen.findAllByTestId("last-turn");
    expect(turns.map((turn) => turn.firstElementChild?.textContent)).toEqual(["You", "Claude asks"]);
    expect(turns[1]!.dataset.asks).toBe("true");
    expect(screen.getByTestId("last-messages-note").textContent).toContain("nothing kept · 1 key taken out");
    expect(screen.getByTestId("last-messages-answer").textContent).toContain("Answer it in the Claude app");
    expect(screen.getByTestId("last-messages-answer").textContent).toContain("pick this one in its sidebar");
  });

  it("off in Settings: says so, with the way to turn them on", async () => {
    vi.spyOn(ipc, "conversationsLastMessages").mockResolvedValue({ state: "off", turns: [], redacted: 0 });
    render(<LastMessages model={model()} visible />);
    expect((await screen.findByTestId("last-messages-off")).textContent).toContain("Last messages are off");
    expect(screen.queryByTestId("last-turn")).toBeNull();
  });

  it("a chat with no transcript here shows its title only", async () => {
    vi.spyOn(ipc, "conversationsLastMessages").mockResolvedValue({ state: "none", turns: [], redacted: 0 });
    render(<LastMessages model={model()} visible />);
    expect((await screen.findByTestId("last-messages-none")).textContent).toContain("its title is all");
  });

  it("a paired Mac's session is never read here", () => {
    const read = vi.spyOn(ipc, "conversationsLastMessages");
    render(<LastMessages model={model()} visible remoteMac="Mac mini" />);
    expect(screen.getByTestId("last-messages-remote").textContent).toBe("Open on Mac mini to read: its messages stay on that Mac.");
    expect(read).not.toHaveBeenCalled();
  });
});
