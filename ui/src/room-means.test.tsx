/**
 * D1 (the 0.4.6 persona walk): This Mac said "Nothing running — your Mac
 * has room" and the top bar "Plenty of room" beside "72 % of all 30
 * cores" and "11 GB of 13 GB moved to disk", and nothing said what room
 * measures. It's said where the level shows: a line under This Mac's
 * level while calm (the reasons say it otherwise), and the top bar's
 * help.
 */
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { MachineStatusLine } from "./components/MachineStatusLine";
import { MachineState } from "./components/this-mac-parts";
import { ThisMacContext, type ThisMacInfo } from "./lib/machine";
import { ROOM_MEASURES } from "./lib/machine-words";
import { mockMachineHealth } from "./preview/machine";
import { mockThisMac } from "./preview/this-mac";

const NOW = 1_728_800_000_000;
const state = (level: "plenty" | "tight"): ThisMacInfo => mockThisMac([], mockMachineHealth(level, NOW), [], [0, 0], []);

afterEach(cleanup);

describe("what room measures, where its level shows (D1)", () => {
  it("the words name what counts and when it turns tight", () => {
    for (const part of ["memory", "swap", "processor", "disk", "over 90% for a full minute", "a quarter of your memory", "under 20 GB free"]) expect(ROOM_MEASURES).toContain(part);
  });

  it("This Mac, calm: the line under its level", () => {
    render(<MachineState info={state("plenty")} />);
    expect(screen.getByTestId("this-mac-room-means").textContent).toBe(ROOM_MEASURES);
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
    expect(screen.getByTestId("machine-status").getAttribute("title")).toBe(`This Mac · Plenty of room. ${ROOM_MEASURES}`);
  });
});
