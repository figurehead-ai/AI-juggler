/**
 * AJ-E2507 R2 and R3 — your menu's facts and hints: your name and
 * initial; with a Figurehead account its plan, Cloud backup's when-last
 * and My Macs' count; the hints ⌘, · ? · version · channel; and the line
 * named "Report a bug…" (as AJ-E2911 names it).
 */
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { UserMenu } from "./UserMenu";
import { agoWords } from "../lib/time-words";
import { keysOf } from "../lib/shortcuts";
import { clearSeams, setSeam } from "../test/fixtures";

const SEAMS = ["__AIJ_FIGUREHEAD__", "__AIJ_USER__", "__AIJ_CLOUD_BACKUP__", "__AIJ_FLEET_PEERS__", "__AIJ_APP_INFO__", "__AIJ_UPDATE_STATUS__"];

beforeEach(() => clearSeams(...SEAMS));
afterEach(() => {
  cleanup();
  clearSeams(...SEAMS);
});

describe("AJ-E2507 — your menu", () => {
  it("R2: your name and initial; with an account its plan, Cloud backup's when-last and My Macs' count", async () => {
    const now = Date.now();
    setSeam("__AIJ_USER__", { name: "Faruk", initial: "F", computer: "Mac", computers: "Macs" });
    setSeam("__AIJ_FIGUREHEAD__", { state: "linked", email: "faruk@example.com", plan: "Pro", device_name: "Studio", features: [], link: null, refreshed_at_ms: now, offline: false, notice: null, consent_version: 1, consent_current: 1 });
    setSeam("__AIJ_CLOUD_BACKUP__", { enabled: true, code_saved: true, schedule: "weekly", last: { at_ms: now - 2 * 3_600_000 }, uploading: false, limit_bytes: 1 });
    setSeam("__AIJ_FLEET_PEERS__", [{ machine_id: "m1" }, { machine_id: "m2" }]);
    render(<UserMenu />);
    const button = screen.getByTestId("user-menu-button");
    await waitFor(() => expect(button.getAttribute("aria-label")).toBe("Your menu: Faruk"));
    expect(button.textContent).toBe("F");
    fireEvent.click(button);
    const menu = await screen.findByRole("menu", { name: "Your menu" });
    const who = within(menu).getByTestId("user-menu-who");
    await waitFor(() => expect(who.textContent).toContain("Pro"));
    expect(who.textContent).toBe("FFarukf…@example.com · FigureheadPro");
    expect(within(menu).getByTestId("user-menu-backup").textContent).toBe(`Cloud backup${agoWords(now - 2 * 3_600_000, Date.now())}`);
    expect(within(menu).getByTestId("user-menu-computers").textContent).toBe("My Macs2");
  });

  it("R2: without an account, the system's name and initial and no plan", async () => {
    setSeam("__AIJ_USER__", { name: "Sam", initial: "S", computer: "Mac", computers: "Macs" });
    render(<UserMenu />);
    fireEvent.click(await screen.findByRole("button", { name: "Your menu: Sam" }));
    const who = within(await screen.findByRole("menu", { name: "Your menu" })).getByTestId("user-menu-who");
    expect(who.textContent).toBe("SSamOn this Mac · no Figurehead account");
  });

  it("R3: the hints ⌘, · ? · version · channel, and “Report a bug…”", async () => {
    setSeam("__AIJ_APP_INFO__", { version: "0.4.1", name: "AI Juggler", edition: "main" });
    render(<UserMenu />);
    fireEvent.click(await screen.findByTestId("user-menu-button"));
    const menu = await screen.findByRole("menu", { name: "Your menu" });
    const hint = (item: HTMLElement) => item.querySelector("[data-label]")!.nextElementSibling?.textContent ?? null;
    const line = (label: string) => within(menu).getAllByRole("menuitem").find((item) => item.querySelector("[data-label]")?.textContent === label)!;
    expect(keysOf("settings")[0]).toBe("⌘,");
    expect(hint(line("Settings"))).toBe("⌘,");
    expect(hint(line("Keyboard shortcuts"))).toBe("?");
    await waitFor(() => expect(hint(line("About and updates"))).toBe("0.4.1 · Beta"));
    expect(line("Report a bug…")).toBeTruthy();
    expect(within(menu).queryByText("Report a problem…")).toBeNull();
  });
});
