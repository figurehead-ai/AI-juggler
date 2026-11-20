/**
 * D1 (the 0.4.6 persona walk): This Mac said "Nothing running — your Mac
 * has room" and the top bar "Plenty of room" beside "72 % of all 30
 * cores" and "11 GB of 13 GB moved to disk", and nothing said what room
 * measures. Where its level shows, calm stays short: one line says what
 * room counts, no figures; when it turns tight sits behind a closed "When
 * does it turn tight?"; the top bar's help says both. All in the
 * backend's words, built from the thresholds it uses (`summary.counts`,
 * `summary.measures`, `RoomThresholds`): the window holds no figures of
 * its own.
 */
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { MachineStatusLine } from "./components/MachineStatusLine";
import { MachineState } from "./components/this-mac-parts";
import { ThisMacContext, type ThisMacInfo } from "./lib/machine";
import { mockMachineHealth } from "./preview/machine";
import { mockThisMac } from "./preview/this-mac";

const NOW = 1_728_800_000_000;
// Words no default would give: the screen shows what the backend sent.
const COUNTS = "Room counts free memory and the processor.";
const WHEN = "It turns tight only when the processor is over 75% for 2 minutes.";
const state = (level: "plenty" | "tight"): ThisMacInfo => {
  const info = mockThisMac([], mockMachineHealth(level, NOW), [], [0, 0], []);
  return { ...info, machine: { ...info.machine, summary: { ...info.machine.summary, counts: COUNTS, measures: WHEN } } };
};

afterEach(cleanup);

describe("what room measures, where its level shows, in the backend's words (D1)", () => {
  it("This Mac, calm: one short line, no figures; when it turns tight is behind a closed fold the keyboard reaches", () => {
    render(<MachineState info={state("plenty")} />);
    expect(screen.getByTestId("this-mac-room-means").textContent).toBe(COUNTS);
    const fold = screen.getByTestId("this-mac-room-when-toggle");
    expect(fold.tagName).toBe("BUTTON");
    expect(fold.textContent).toContain("When does it turn tight?");
    expect(fold.getAttribute("aria-expanded")).toBe("false");
    const words = screen.getByText(WHEN);
    expect(words.closest("[hidden]")).not.toBeNull();
    fireEvent.click(fold);
    expect(fold.getAttribute("aria-expanded")).toBe("true");
    expect(words.closest("[hidden]")).toBeNull();
  });

  it("This Mac, tight: its reasons say it, no line and no fold", () => {
    render(<MachineState info={state("tight")} />);
    expect(screen.queryByTestId("this-mac-room-means")).toBeNull();
    expect(screen.queryByTestId("this-mac-room-when")).toBeNull();
  });

  it("the top bar's help says both after the level", () => {
    render(
      <ThisMacContext.Provider value={state("plenty")}>
        <MachineStatusLine minimal={false} short onOpen={vi.fn()} />
      </ThisMacContext.Provider>,
    );
    expect(screen.getByTestId("machine-status").getAttribute("title")).toBe(`This Mac · Plenty of room. ${COUNTS} ${WHEN}`);
  });
});
