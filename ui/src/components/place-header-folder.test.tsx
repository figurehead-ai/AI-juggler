/**
 * AJ-E2327 — an App whose sessions can't be shown (no tool folder linked)
 * says why, in its row's words and its icon's tooltip, never just a grey
 * icon and "nothing open"; its menu offers Choose its folder….
 */
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { PlaceHeader } from "./PlaceHeader";
import { AppFolderNote } from "./AppFolderNote";
import type { RailPlace } from "../lib/rail-items";

afterEach(cleanup);

const why = "It isn't running and its app lists no sessions yet, so AI Juggler can't tell which Claude Code folder it uses.";
const place: RailPlace = { key: "app|claude-app|team-a", app: "Claude app", appId: "claude-app", kind: "desktop", profile: "team-a", appPid: 100, host: null };

describe("AJ-E2327 an App with no folder", () => {
  it("says its sessions aren't shown and why, and offers Choose its folder…", () => {
    const choose = vi.fn();
    render(<PlaceHeader place={place} label="Claude app" count={0} needs={0} folded={null} onToggle={vi.fn()} folder={{ tool: "Claude Code", name: null, path: null, by: null, why }} onChooseFolder={choose} />);
    expect(screen.queryByTestId("place-nothing-open")).toBeNull();
    const words = screen.getByTestId("place-not-shown");
    expect(words.textContent).toBe("sessions not shown");
    expect(words.getAttribute("title")).toBe(why);
    expect(screen.getByTestId("place-header").textContent).toContain(`sessions not shown: ${why}`);
    expect(screen.getByTestId("app-icon").getAttribute("title")).toBe(`Claude app. Its sessions can't be shown: ${why}`);
    fireEvent.contextMenu(screen.getByTestId("place-header"), { clientX: 20, clientY: 30 });
    fireEvent.click(screen.getByTestId("place-header-menu-folder"));
    expect(choose).toHaveBeenCalledTimes(1);
  });

  it("linked, with nothing open: \"nothing open\", and its grey icon says what muted means", () => {
    render(<PlaceHeader place={place} label="Claude app" count={0} needs={0} folded={null} onToggle={vi.fn()} folder={{ tool: "Claude Code", name: "client-a", path: null, by: "running", why: null }} />);
    expect(screen.getByTestId("place-nothing-open")).toBeTruthy();
    expect(screen.getByTestId("app-icon").getAttribute("title")).toContain("Muted while nothing here needs you");
  });

  it("the note under it says why and chooses", () => {
    const choose = vi.fn();
    render(<AppFolderNote why={why} onChoose={choose} />);
    expect(screen.getByTestId("app-folder-note").textContent).toContain(why);
    fireEvent.click(screen.getByRole("button", { name: "Choose its folder…" }));
    expect(choose).toHaveBeenCalledTimes(1);
  });
});
