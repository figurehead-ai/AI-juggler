/**
 * D1 (the 0.4.6 persona walk): This Mac said "Nothing running — your Mac
 * has room" and the top bar "Plenty of room" beside "72 % of all 30
 * cores" and "11 GB of 13 GB moved to disk", and nothing said what room
 * measures. It's said where the level shows (a line under This Mac's
 * level while calm, the reasons say it otherwise; and the top bar's
 * help), in the backend's words, built from the thresholds it uses
 * (`summary.measures`, `RoomThresholds::measures`): the window holds no
 * figures of its own.
 */
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { MachineStatusLine } from "./components/MachineStatusLine";
import { MachineState } from "./components/this-mac-parts";
import { ThisMacContext, type ThisMacInfo } from "./lib/machine";
import { mockMachineHealth } from "./preview/machine";
import { mockThisMac } from "./preview/this-mac";

const NOW = 1_728_800_000_000;
// Words no default would give: the screen shows what the backend sent.
const SAID = "Room is free memory and the processor. It turns tight only when the processor is over 75% for 2 minutes.";
const state = (level: "plenty" | "tight"): ThisMacInfo => {
  const info = mockThisMac([], mockMachineHealth(level, NOW), [], [0, 0], []);
  return { ...info, machine: { ...info.machine, summary: { ...info.machine.summary, measures: SAID } } };
};

afterEach(cleanup);

describe("what room measures, where its level shows, in the backend's words (D1)", () => {
  it("This Mac, calm: the line under its level", () => {
    render(<MachineState info={state("plenty")} />);
    expect(screen.getByTestId("this-mac-room-means").textContent).toBe(SAID);
  });

  it("This Mac, tight: its reasons say it, no line", () => {
    render(<MachineState info={state("tight")} />);
    expect(screen.queryByTestId("this-mac-room-means")).toBeNull();
  });

  it("the top bar's help says it after the level", () => {
    render(
      <ThisMacContext.Provider value={state("plenty")}>
        <MachineStatusLine minimal={false} short onOpen={vi.fn()} />
      </ThisMacContext.Provider>,
    );
    expect(screen.getByTestId("machine-status").getAttribute("title")).toBe(`This Mac · Plenty of room. ${SAID}`);
  });
});
