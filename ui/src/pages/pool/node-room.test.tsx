/**
 * P1S6 step 3 (the rc29 walk): the writer's node showed no mark while its
 * Account was out of room. A member whose Account is out shows the one
 * Out mark (`RoomChip`); one with room, or only low, shows nothing.
 */
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

vi.mock("../../components/RoomChip", async (actual) => {
  const real = await actual<typeof import("../../components/RoomChip")>();
  return {
    ...real,
    useRoomMark: (profileId: string | null | undefined) =>
      profileId === "work"
        ? { kind: "out", label: "Out", short: "Out", title: "Work is out of room until 9:06 PM", about: false }
        : profileId === "personal-low"
          ? { kind: "low", label: "12% left", short: "12%", title: "Personal has 12% left", about: false }
          : null,
  };
});

const { NodeRoom } = await import("./NodeParts");

afterEach(cleanup);

describe("a member's room on the canvas", () => {
  it("shows the Out mark when its Account is out", () => {
    render(<NodeRoom profileId="work" />);
    const mark = screen.getByTestId("node-room");
    expect(mark.dataset.kind).toBe("out");
    expect(mark.textContent).toContain("Work is out of room until 9:06 PM");
  });
  it("shows nothing with room, or only low", () => {
    render(<NodeRoom profileId="personal-low" />);
    render(<NodeRoom profileId="personal" />);
    expect(screen.queryByTestId("node-room")).toBeNull();
  });
});
