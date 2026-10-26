import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, waitFor } from "@testing-library/react";
import type { InstanceInfo } from "./bridge";

const asked: string[][] = [];
vi.mock("./ipc/sessions", () => ({
  sessionsGet: async (ids: string[]) => {
    asked.push(ids);
    return ids.map((id) => ({ id, status: "finished" }) as InstanceInfo);
  },
}));

const { useWithSelected } = await import("./session-lookup");

afterEach(() => {
  cleanup();
  asked.length = 0;
});

function Probe({ sessions, selected, out }: { sessions: InstanceInfo[]; selected: string | null; out: { ids?: string[] } }) {
  out.ids = useWithSelected(sessions, selected).map((s) => s.id);
  return null;
}

const working = [{ id: "open-1" }, { id: "recent-1" }] as InstanceInfo[];

describe("the working set and a session past it", () => {
  it("leaves the list alone when the selected session is in it", () => {
    const out: { ids?: string[] } = {};
    render(<Probe sessions={working} selected="open-1" out={out} />);
    expect(out.ids).toEqual(["open-1", "recent-1"]);
    expect(asked).toEqual([]);
  });

  it("reads an old session by its id, and it joins the list while selected", async () => {
    const out: { ids?: string[] } = {};
    const view = render(<Probe sessions={working} selected="old-9" out={out} />);
    await waitFor(() => expect(out.ids).toEqual(["open-1", "recent-1", "old-9"]));
    expect(asked).toEqual([["old-9"]]);
    view.rerender(<Probe sessions={working} selected={null} out={out} />);
    expect(out.ids).toEqual(["open-1", "recent-1"]);
  });
});
