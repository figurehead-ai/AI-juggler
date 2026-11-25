/**
 * 0.5.1 F76 (PAT-COLOR-2, COL-5): amber is never a fill behind words. The
 * kit's warning chip is the neutral chip's tone with its words, and the
 * mark they carry, in amber; every chip, pill and tag that says a warning
 * draws it from the one place (`WARNING_CHIP`): "Probably stuck · 3 h" on
 * a session's page, a project's "1 stuck", "⚠ Renews in 6 days", a cost's
 * "▲ 28%", a low room chip.
 */
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { Chip, WARNING_CHIP } from "./primitives";
import { Pill } from "./Pill";
import { RenewalChip } from "./RenewalChip";
import { RoomChip } from "./RoomChip";
import { Delta } from "./charts/StatTile";
import { chipToneClasses } from "./SessionCards";
import { SessionChips } from "../pages/session/SessionHead";
import type { SessionModel } from "../pages/session/model";

afterEach(cleanup);

/** No amber behind the words; the words amber on the neutral tone. */
function calm(className: string): void {
  expect(className).not.toMatch(/warning-soft/);
  expect(className).not.toMatch(/border-warning/);
  expect(className).toContain("text-warning");
  expect(className).toContain("bg-surface-2");
}

describe("0.5.1 F76: the kit's one warning chip", () => {
  it("is the neutral tone with amber words", () => {
    calm(WARNING_CHIP);
  });

  it("Chip and Pill draw it", () => {
    render(<Chip tone="warning">1 stuck</Chip>);
    calm(screen.getByText("1 stuck").className);
    cleanup();
    render(<Pill tone="warning" testId="pill">1 stuck</Pill>);
    calm(screen.getByTestId("pill").className);
  });

  it("the renewal chip, a cost's rise and a low room chip draw it", () => {
    render(<RenewalChip renewal={{ tier: "warning", text: "Renews in 6 days" }} />);
    calm(screen.getByText(/Renews in 6 days/).className);
    cleanup();
    render(<Delta now={128} before={100} better="down" period="last week" />);
    calm(screen.getByTestId("stat-delta").firstElementChild!.className);
    cleanup();
    render(<RoomChip mark={{ kind: "low", label: "Low · 12% left", short: "Low", title: "Low · 12% left", about: false, resetsAtMs: null }} />);
    calm(screen.getByTestId("room-chip").className);
  });

  it("a session page's state chip draws it (\"Probably stuck · 3 h\")", () => {
    const model: SessionModel = {
      live: true,
      state: { words: "Probably stuck", tone: "warning", since: Date.now() - 3 * 3_600_000, working: false, asks: true },
      places: [], tool: "claude-code", sessionId: "s1", viewedMs: null, cwd: "/Users/someone/Dev/app", profileId: "p", profileLabel: "Work",
      conversationId: null, switchSource: "s1", model: null,
    };
    render(<SessionChips model={model} account={undefined} now={Date.now()} />);
    const chip = screen.getByTestId("session-state-chip");
    expect(chip.textContent).toContain("Probably stuck");
    calm(chip.className);
  });

  it("the rows' compact state chip draws it", () => {
    calm(chipToneClasses.warning);
  });
});
