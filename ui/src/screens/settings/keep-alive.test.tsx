/**
 * 0.3.3 vitest — the apps and profiles AI Juggler remembers: Settings ›
 * Desktop apps lists each with Start (Open while it runs) and Keep alive;
 * Home has a tile for one kept alive that isn't running; an ended
 * session offers Open in the app it was last open in; and the words for
 * each state.
 */
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { DesktopAppsPane } from "./DesktopAppsPane";
import { ConversationsProvider } from "../../lib/conversations";
import { HomeApps } from "../../components/HomeApps";
import { placeFor, placeWords } from "../../lib/places";
import { placesFind } from "../../lib/ipc/places";
import type { KnownPlaceInfo } from "../../lib/ipc/places";

const NOW = Date.UTC(2026, 9, 31, 12);

function place(over: Partial<KnownPlaceInfo>): KnownPlaceInfo {
  return {
    id: "claude-app|/Users/you/Profiles/b-lab",
    app_id: "claude-app",
    app: "Claude",
    instance: "b-lab",
    profile: "b-lab",
    label: "Claude · b-lab",
    app_pid: null,
    keep_alive: false,
    paused: null,
    account_id: null,
    seen_ms: NOW - 2 * 3_600_000,
    cannot_start: null,
    ...over,
  };
}

type Seams = { __AIJ_PLACES__?: { places: KnownPlaceInfo[]; sessions?: Record<string, string> } };
const seams = () => window as unknown as Seams;

afterEach(() => {
  cleanup();
  delete seams().__AIJ_PLACES__;
});

describe("the words for a place", () => {
  it("says running, down, kept alive or paused, and why", () => {
    expect(placeWords(place({ app_pid: 10 }), NOW)).toBe("Running");
    expect(placeWords(place({ app_pid: 10, keep_alive: true }), NOW)).toBe("Running · kept alive");
    expect(placeWords(place({}), NOW)).toBe("Not running · seen 2 h ago");
    expect(placeWords(place({ keep_alive: true }), NOW)).toBe("Not running · keep alive starts it");
    expect(placeWords(place({ keep_alive: true, paused: "it stopped 3 times in 10 minutes" }), NOW)).toBe("Keep alive paused: it stopped 3 times in 10 minutes");
  });

  it("finds a running app's place by its process, never by a name", () => {
    const rows = [place({ app_pid: 10 }), place({ id: "b", app_pid: null })];
    expect(placeFor(rows, 10)?.label).toBe("Claude · b-lab");
    expect(placeFor(rows, null)).toBeNull();
    expect(placeFor(rows, 11)).toBeNull();
  });
});

describe("Settings › Desktop apps: your apps and profiles", () => {
  it("lists each with Start or Open, and Keep alive turns on and off", async () => {
    seams().__AIJ_PLACES__ = { places: [place({}), place({ id: "codex-app|", app_id: "codex-app", app: "ChatGPT", instance: null, profile: null, label: "ChatGPT", app_pid: 20 })] };
    render(
      <ConversationsProvider>
        <DesktopAppsPane />
      </ConversationsProvider>,
    );
    const lab = await screen.findByTestId("desktop-place-claude-app-b-lab");
    expect(within(lab).getByTestId("desktop-place-open").textContent).toContain("Start");
    expect(within(lab).getByTestId("desktop-place-forget")).toBeTruthy();
    const chat = screen.getByTestId("desktop-place-codex-app");
    expect(within(chat).getByTestId("desktop-place-open").textContent).toContain("Open");
    expect(within(chat).queryByTestId("desktop-place-forget")).toBeNull();
    // The switch is named by its words and the place.
    const keep = within(lab).getByRole("switch", { name: "Keep alive Claude · b-lab" });
    fireEvent.click(keep);
    await waitFor(() => expect(seams().__AIJ_PLACES__!.places[0]!.keep_alive).toBe(true));
    await waitFor(() => expect(within(screen.getByTestId("desktop-place-claude-app-b-lab")).getByRole("switch").getAttribute("aria-checked")).toBe("true"));
    // Kept alive: no Forget (it would be remembered again at once).
    expect(within(screen.getByTestId("desktop-place-claude-app-b-lab")).queryByTestId("desktop-place-forget")).toBeNull();
  });

  it("a place that can't be started says why and offers neither Start nor Keep alive", async () => {
    seams().__AIJ_PLACES__ = { places: [place({ cannot_start: "Claude isn't where it was" })] };
    render(
      <ConversationsProvider>
        <DesktopAppsPane />
      </ConversationsProvider>,
    );
    const row = await screen.findByTestId("desktop-place-claude-app-b-lab");
    expect(row.textContent).toContain("Can't be started from here: Claude isn't where it was.");
    expect((within(row).getByTestId("desktop-place-open") as HTMLButtonElement).disabled).toBe(true);
    expect(within(row).getByRole("switch").hasAttribute("disabled")).toBe(true);
  });

  it("says where apps come from before any was seen", async () => {
    seams().__AIJ_PLACES__ = { places: [] };
    render(
      <ConversationsProvider>
        <DesktopAppsPane />
      </ConversationsProvider>,
    );
    expect((await screen.findByTestId("desktop-places-empty")).textContent).toContain("once AI Juggler has seen them running");
  });
});

describe("Home: an app kept alive that isn't running", () => {
  it("has a tile with why and Start, which starts it", async () => {
    seams().__AIJ_PLACES__ = { places: [place({ keep_alive: true }), place({ id: "other", label: "Claude · b-off", instance: "b-off", profile: "b-off" })] };
    render(<HomeApps items={[]} runwayOf={() => null} />);
    const tile = await screen.findByTestId("home-app-closed");
    expect(screen.getAllByTestId("home-app-closed")).toHaveLength(1);
    expect(tile.textContent).toContain("Not running · keep alive starts it");
    fireEvent.click(within(tile).getByRole("button", { name: "Start Claude · b-lab" }));
    await waitFor(() => expect(seams().__AIJ_PLACES__!.places[0]!.app_pid).not.toBeNull());
    // Running now, with nothing open in it yet: its tile stays, with Open.
    await waitFor(() => expect(screen.queryByTestId("home-app-closed")).toBeNull());
    const kept = screen.getByTestId("home-app-kept");
    expect(kept.textContent).toContain("Running · kept alive");
    expect(within(kept).getByRole("button", { name: "Open Claude · b-lab" })).toBeTruthy();
  });
});

describe("an ended session's app", () => {
  it("is the place it was last open in, by its tool and id", async () => {
    seams().__AIJ_PLACES__ = { places: [place({})], sessions: { "claude-code/s-1": "claude-app|/Users/you/Profiles/b-lab" } };
    expect((await placesFind("claude-code", "s-1"))?.label).toBe("Claude · b-lab");
    expect(await placesFind("claude-code", "s-2")).toBeNull();
  });
});
