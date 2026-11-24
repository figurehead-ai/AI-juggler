/**
 * AJ-E2326 R4 and R6, the words. A conversation AI Juggler continued from
 * a Claude app profile, then resumed by that profile's Code child too:
 * both rows say "Open in two places", where else, with one click to stop
 * ours (never the other app's); and the session's page says where it came
 * from ("Continued here from Claude app · <profile>"), never only a CLI run.
 */
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { SessionRow } from "./SessionRow";
import { ActionHostProvider } from "../ItemMenu";
import { itemsOf } from "../../lib/rail-items";
import { claudeApp, NOW, open } from "../../lib/rail-matrix.test-kit";
import { instance } from "../../test/fixtures";
import { fromInstance } from "../../pages/session/model";
import { WhereItRuns } from "../../pages/session/WhereItRuns";
import type { PlaceInfo } from "../../lib/ipc/conversations";

afterEach(() => {
  cleanup();
  Reflect.deleteProperty(window, "__AIJ_PLACES__");
});

const PROFILE = claudeApp("b-work", 52_844);
const OURS: PlaceInfo = { app: "AI Juggler", shown: "AI Juggler", app_id: null, profile: null, instance: null, pid: 610, kind: null, app_pid: 42, label: "AI Juggler" };
const CHILD: PlaceInfo = { ...PROFILE.place, pid: 777, label: "Claude app · b-work" };
const RUN = instance({ id: "sess-echo", status: "running", conversation_id: "echo", account_id: PROFILE.folder, account_label: "b-work", started_at_ms: NOW - 120_000 });
const ECHO = open("echo", PROFILE, { session_id: "sess-echo", places: [OURS, CHILD], title: "Echo lab", started_ms: NOW - 5 * 3_600_000 });

describe("Open in two places", () => {
  it("both rows say it and where else; Stop ours stops AI Juggler's session only", () => {
    const items = itemsOf({ sessions: [RUN], conversations: [ECHO], projects: [], seen: {} });
    const stopSession = vi.fn();
    for (const [key, also] of [["s:sess-echo", "Claude app · b-work"], [`c:${PROFILE.folder}/echo`, "AI Juggler"]] as const) {
      const item = items.find((row) => row.key === key)!;
      render(
        <ActionHostProvider host={{ stopSession }}>
          <SessionRow item={item} density="cozy" now={NOW} onSelect={vi.fn()} />
        </ActionHostProvider>,
      );
      const note = screen.getByTestId("two-places");
      expect(note.textContent).toContain("Open in two places");
      expect(note.textContent).toContain(`also in ${also}`);
      const stop = screen.getByTestId("two-places-stop");
      expect(stop.textContent).toBe("Stop ours");
      expect(stop.getAttribute("aria-label")).toBe(`Stop AI Juggler's session on ${item.title}`);
      fireEvent.click(stop);
      cleanup();
    }
    expect(stopSession.mock.calls.map(([row]) => row.id)).toEqual(["sess-echo", "sess-echo"]);
  });

  it("a conversation open in one place has no note", () => {
    const items = itemsOf({ sessions: [RUN], conversations: [{ ...ECHO, places: [OURS] }], projects: [], seen: {} });
    render(<SessionRow item={items[0]!} density="cozy" now={NOW} onSelect={vi.fn()} />);
    expect(screen.queryByTestId("two-places")).toBeNull();
  });
});

describe("a session AI Juggler continued", () => {
  const seed = () => Reflect.set(window, "__AIJ_PLACES__", { places: [PROFILE.desktop], sessions: { "claude-code/echo": PROFILE.desktop!.id } });

  it("says where it came from: the App it was open in", async () => {
    seed();
    render(<WhereItRuns model={fromInstance(RUN, ECHO)} accountWords={null} outside={false} />);
    await waitFor(() => expect(screen.getByTestId("where-continued-from").textContent).toBe("Continued here from Claude app · b-work"));
  });

  it("a session that started the conversation here says nothing of the kind", async () => {
    seed();
    render(<WhereItRuns model={fromInstance(RUN, { ...ECHO, started_ms: NOW - 60_000 })} accountWords={null} outside={false} />);
    await new Promise((resolve) => setTimeout(resolve, 20));
    expect(screen.queryByTestId("where-continued-from")).toBeNull();
  });
});
