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
import { resetToasts, toastEntries } from "../../components/Toast";

const NOW = Date.UTC(2026, 9, 31, 12);

function place(over: Partial<KnownPlaceInfo>): KnownPlaceInfo {
  return {
    recent: [],
    id: "app:claude-app:/Users/you/Profiles/b-lab",
    app_id: "claude-app",
    app: "Claude",
    instance: "b-lab",
    profile: "b-lab",
    label: "Claude · b-lab",
    app_pid: null,
    lost: false,
    keep_alive: false,
    paused: null,
    account_id: null,
    seen_ms: NOW - 2 * 3_600_000,
    cannot_start: null,
    found: false,
    ...over,
  };
}

type Seams = { __AIJ_PLACES__?: { places: KnownPlaceInfo[]; sessions?: Record<string, string>; offers?: { id: string; label: string }[] } };
const seams = () => window as unknown as Seams;

afterEach(() => {
  cleanup();
  resetToasts();
  delete seams().__AIJ_PLACES__;
});

describe("the words for a place", () => {
  it("says running, down, kept alive or paused, and why", () => {
    expect(placeWords(place({ app_pid: 10 }), NOW)).toBe("Running");
    expect(placeWords(place({ app_pid: 10, keep_alive: true }), NOW)).toBe("Running · kept alive");
    expect(placeWords(place({}), NOW)).toBe("Not running · seen 2 h ago");
    expect(placeWords(place({ keep_alive: true }), NOW)).toBe("Not running · keep alive starts it");
    expect(placeWords(place({ keep_alive: true, paused: "it stopped 3 times in 10 minutes" }), NOW)).toBe("Keep alive paused: it stopped 3 times in 10 minutes");
    // 0.3.8: running on with no window after macOS restarted its display.
    expect(placeWords(place({ app_pid: 10, lost: true, keep_alive: true }), NOW)).toBe("Lost its window when the system restarted the display");
  });

  it("finds a running app's place by its process, never by a name", () => {
    const rows = [place({ app_pid: 10 }), place({ id: "b", app_pid: null })];
    expect(placeFor(rows, 10)?.label).toBe("Claude · b-lab");
    expect(placeFor(rows, null)).toBeNull();
    expect(placeFor(rows, 11)).toBeNull();
  });
});

describe("Settings › Desktop apps: your apps", () => {
  it("lists each with Start or Open, and Keep alive turns on and off", async () => {
    seams().__AIJ_PLACES__ = { places: [place({}), place({ id: "app:codex-app:", app_id: "codex-app", app: "ChatGPT", instance: null, profile: null, label: "ChatGPT", app_pid: 20 })] };
    render(
      <ConversationsProvider>
        <DesktopAppsPane />
      </ConversationsProvider>,
    );
    const lab = await screen.findByTestId("desktop-place-claude-app-b-lab");
    expect(within(lab).getByTestId("desktop-place-open").textContent).toContain("Start");
    // Forget is in the row's ⋯ (design review #65).
    fireEvent.click(within(lab).getByTestId("desktop-place-more"));
    expect(screen.getByTestId("desktop-place-forget").textContent).toContain("Forget");
    fireEvent.keyDown(document.activeElement ?? document.body, { key: "Escape" });
    const chat = screen.getByTestId("desktop-place-codex-app");
    expect(within(chat).getByTestId("desktop-place-open").textContent).toContain("Open");
    expect(within(chat).queryByTestId("desktop-place-more")).toBeNull();
    // The switch is named by its words and the App (D-62).
    const keep = within(lab).getByRole("switch", { name: "Keep alive Claude app · b-lab" });
    fireEvent.click(keep);
    await waitFor(() => expect(seams().__AIJ_PLACES__!.places[0]!.keep_alive).toBe(true));
    await waitFor(() => expect(within(screen.getByTestId("desktop-place-claude-app-b-lab")).getByRole("switch").getAttribute("aria-checked")).toBe("true"));
    // Kept alive: no Forget (it would be remembered again at once).
    expect(within(screen.getByTestId("desktop-place-claude-app-b-lab")).queryByTestId("desktop-place-more")).toBeNull();
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
    expect((await screen.findByTestId("desktop-places-empty")).textContent).toContain("Apps you add show here");
  });
});

describe("Home: an app that isn't running", () => {
  it("has a tile with why and Start, which starts it", async () => {
    seams().__AIJ_PLACES__ = { places: [place({ keep_alive: true }), place({ id: "other", label: "Claude · b-off", instance: "b-off", profile: "b-off" })] };
    render(<HomeApps items={[]} runwayOf={() => null} />);
    await screen.findAllByTestId("home-app-closed");
    // 0.3.4 (D-64): every remembered app that's closed has a tile, the
    // kept-alive one first.
    const [tile, other] = screen.getAllByTestId("home-app-closed");
    expect(tile!.textContent).toContain("Not running · keep alive starts it");
    expect(other!.textContent).toContain("b-off");
    expect(within(other!).getByRole("button", { name: "Start Claude app · b-off" })).toBeTruthy();
    fireEvent.click(within(tile!).getByRole("button", { name: "Start Claude app · b-lab" }));
    await waitFor(() => expect(seams().__AIJ_PLACES__!.places[0]!.app_pid).not.toBeNull());
    // Running now, with nothing open in it yet: its tile stays, with Open.
    await waitFor(() => expect(screen.getAllByTestId("home-app-closed")).toHaveLength(1));
    const kept = screen.getByTestId("home-app-kept");
    expect(kept.textContent).toContain("Running · kept alive");
    expect(within(kept).getByRole("button", { name: "Open Claude app · b-lab" })).toBeTruthy();
  });
});

describe("0.3.8: a profile that lost its window", () => {
  it("says so, Open asks to restart it, and only Restart restarts it", async () => {
    seams().__AIJ_PLACES__ = { places: [place({ app_pid: 10, lost: true })] };
    render(
      <ConversationsProvider>
        <DesktopAppsPane />
      </ConversationsProvider>,
    );
    const row = await screen.findByTestId("desktop-place-claude-app-b-lab");
    expect(row.textContent).toContain("Lost its window when the system restarted the display");
    fireEvent.click(within(row).getByTestId("desktop-place-open"));
    await waitFor(() => expect(toastEntries()[0]?.options.testId).toBe("go-to-lost"));
    const asked = toastEntries()[0]!.options;
    expect(asked.message).toBe("Claude · b-lab lost its window when the system restarted the display. Restart it?");
    expect(asked.tone).toBe("warning");
    expect(asked.detail).toBe("Restarting stops anything still working in it.");
    // Asking restarts nothing.
    expect(seams().__AIJ_PLACES__!.places[0]!.app_pid).toBe(10);
    expect(asked.action?.label).toBe("Restart");
    asked.action!.run();
    await waitFor(() => expect(toastEntries()[0]?.options.message).toBe("Restarting Claude · b-lab"));
    expect(seams().__AIJ_PLACES__!.places[0]!.lost).toBe(false);
    expect(seams().__AIJ_PLACES__!.places[0]!.app_pid).not.toBe(10);
    await waitFor(() => expect(screen.getByTestId("desktop-place-claude-app-b-lab").textContent).toContain("Running"));
  });
});

describe("an ended session's app", () => {
  it("is the place it was last open in, by its tool and id", async () => {
    seams().__AIJ_PLACES__ = { places: [place({})], sessions: { "claude-code/s-1": "app:claude-app:/Users/you/Profiles/b-lab" } };
    expect((await placesFind("claude-code", "s-1"))?.label).toBe("Claude · b-lab");
    expect(await placesFind("claude-code", "s-2")).toBeNull();
  });
});

describe("D-84 Settings › Desktop apps: found on this computer", () => {
  it("an app seen running but never added is only offered: Add or Not now", async () => {
    seams().__AIJ_PLACES__ = {
      places: [],
      offers: [
        { id: "app:claude-app:/Users/you/ClaudeProfiles/aij-test", label: "Claude app · aij-test" },
        { id: "app:codex-app:", label: "Codex app" },
      ],
    };
    render(
      <ConversationsProvider>
        <DesktopAppsPane />
      </ConversationsProvider>,
    );
    const offers = await screen.findAllByTestId("desktop-offer");
    expect(offers.map((row) => row.textContent)).toEqual(["Claude app · aij-testAddNot now", "Codex appAddNot now"]);
    // Not in Your apps until added.
    expect(screen.getByTestId("desktop-places-empty")).toBeTruthy();
    fireEvent.click(within(offers[1]!).getByTestId("desktop-offer-not-now"));
    await waitFor(() => expect(screen.getAllByTestId("desktop-offer")).toHaveLength(1));
    fireEvent.click(screen.getByTestId("desktop-offer-add"));
    await waitFor(() => expect(screen.queryByTestId("desktop-offer")).toBeNull());
  });
});
