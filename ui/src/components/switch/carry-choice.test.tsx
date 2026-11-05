/**
 * D-76 (AJ-E211) vitest — the switch's advanced choice to leave out the
 * start of a very long conversation: hidden where it can't apply or isn't
 * worth it, closed at first, every choice saying what it carries and what
 * the new session won't know, and only a supported choice sent.
 */
import { act, cleanup, fireEvent, render, renderHook, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { CarryChoice, useCarryChoice } from "./CarryChoice";
import type { SwitchCutArgs, SwitchCutInfo } from "../../lib/ipc/switchCut";

type Seam = { __AIJ_SWITCH_CUT__?: (cut: SwitchCutArgs | null) => SwitchCutInfo };
const w = () => window as unknown as Seam;

const conversation = (cut: SwitchCutArgs | null): SwitchCutInfo => {
  const left = cut?.keep_last ? 1000 - cut.keep_last : 0;
  return {
    supported: true,
    turns: 1000,
    first_ms: Date.UTC(2026, 6, 1),
    last_ms: Date.UTC(2026, 10, 1),
    bytes: 7_800_000_000,
    turns_left_out: left,
    bytes_left_out: left * 7_000_000,
    from_ms: left ? Date.UTC(2026, 9, 20) : null,
    at_compaction: false,
    words: left ? `Carries the last ${cut?.keep_last} turns and leaves out ${left} turns.` : "Carries the whole conversation: 1,000 turns (7.8 GB).",
    loses: left ? "The new session won't know what was said or decided before then." : null,
  };
};

afterEach(() => {
  cleanup();
  delete w().__AIJ_SWITCH_CUT__;
});

describe("D-76 — leave out the start of a long conversation", () => {
  it("isn't offered where the switch can't leave part out", async () => {
    const { result } = renderHook(() => useCarryChoice("s-1", "p-2"));
    await act(async () => new Promise((resolve) => setTimeout(resolve, 200)));
    const { container } = render(<CarryChoice state={result.current} />);
    expect(container.textContent).toBe("");
    expect(result.current.args).toBeNull();
  });

  it("carries everything until a choice is made, then says what goes and what's lost", async () => {
    w().__AIJ_SWITCH_CUT__ = conversation;
    const hook = renderHook(() => useCarryChoice("s-1", "p-2"));
    await waitFor(() => expect(hook.result.current.info?.supported).toBe(true));
    expect(hook.result.current.args).toBeNull();
    const view = render(<CarryChoice state={hook.result.current} />);
    expect(view.getByTestId("carry-words").textContent).toContain("whole conversation");
    act(() => hook.result.current.setKind("last"));
    await waitFor(() => expect(hook.result.current.info?.turns_left_out).toBe(800));
    view.rerender(<CarryChoice state={hook.result.current} />);
    expect(hook.result.current.args).toEqual({ before_ms: null, keep_last: 200 });
    expect(view.getByTestId("carry-words").textContent).toContain("leaves out 800 turns");
    expect(view.getByTestId("carry-loses").textContent).toContain("won't know");
    fireEvent.change(view.getByTestId("carry-last-n"), { target: { value: "50" } });
    await waitFor(() => expect(hook.result.current.args).toEqual({ before_ms: null, keep_last: 50 }));
  });
});
