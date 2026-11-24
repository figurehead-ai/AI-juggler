/**
 * AJ-E2322 vitest — the move's last step says "Opened in <app>" only when
 * the conversation was seen open there; otherwise it says plainly that
 * AI Juggler couldn't see it open, and offers Open in that App, which
 * opens the moved conversation there.
 */
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { MoveDoneInfo } from "../../lib/ipc/switch-move";

const opened = vi.fn(async (_id: string, _conversation: string | null) => ({ outcome: "Starting Claude app · B… the session opens once it's up", reached: "started" as const }));
vi.mock("../../lib/ipc/places", () => ({ placesOpen: (id: string, conversation: string | null) => opened(id, conversation) }));

const { MoveDone } = await import("./MoveDone");

const done = (over: Partial<MoveDoneInfo>): MoveDoneInfo => ({
  move_id: "1700000000000-a1",
  payload: { state: "verified", detail: "every byte of 1 conversation is in B, checked before and after it opened" },
  catalog: { state: "verified", detail: "Claude app · B lists the conversation and its family as they were nested" },
  opened: { state: "verified", detail: "opened in Claude app · B" },
  model: { state: "pending", detail: "send a message to check it answers on B; AI Juggler never sends one for you" },
  notes: [],
  open_place_id: null,
  conversation: "conv-1",
  ...over,
});

afterEach(() => {
  cleanup();
  opened.mockClear();
});

describe("Move's last step (AJ-E2322)", () => {
  it("says Opened in the app, with no Open, only when it was seen open", () => {
    render(<MoveDone done={done({})} target="Claude app · B" />);
    const row = screen.getByTestId("switch-moved-opened");
    expect(row.getAttribute("data-state")).toBe("verified");
    expect(row.textContent).toContain("Opened in Claude app · B");
    expect(screen.queryByTestId("switch-moved-open")).toBeNull();
  });

  it("says plainly it couldn't see it open, and Open opens the moved conversation there", async () => {
    render(
      <MoveDone
        done={done({
          opened: { state: "pending", detail: "sent to Claude app · B, but AI Juggler couldn't see it open there. Open it from Claude app · B" },
          open_place_id: "claude-desktop:b",
        })}
        target="Claude app · B"
      />,
    );
    const row = screen.getByTestId("switch-moved-opened");
    expect(row.getAttribute("data-state")).toBe("pending");
    expect(row.textContent).toContain("Sent to Claude app · B, but AI Juggler couldn't see it open there");
    expect(row.textContent).not.toMatch(/Opened in/);
    const open = screen.getByTestId("switch-moved-open");
    expect(open.textContent).toBe("Open in Claude app · B");
    fireEvent.click(open);
    await waitFor(() => expect(screen.getByTestId("switch-moved-open-said").textContent).toContain("Starting Claude app · B"));
    expect(opened).toHaveBeenCalledWith("claude-desktop:b", "conv-1");
  });

  it("offers Open where the link couldn't be sent at all", () => {
    render(
      <MoveDone
        done={done({ opened: { state: "failed", detail: "moved, but not opened: Claude app · B didn't finish starting. Open it from there" }, open_place_id: "claude-desktop:b" })}
        target="Claude app · B"
      />,
    );
    expect(screen.getByTestId("switch-moved-opened").getAttribute("data-state")).toBe("failed");
    expect(screen.getByTestId("switch-moved-open")).toBeTruthy();
  });
});
