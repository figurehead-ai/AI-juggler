/**
 * E14 vitest — the machine launch advisor (AJ-E1407): "Has this Mac got
 * room?" beside the account's own verdict, in words with its reasons and
 * what would make room — and it never blocks Start.
 */
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { MachineAdviceLine, MachineChip, basisWords } from "./MachineAdvice";
import { NewSessionModal } from "../screens/NewSession";
import * as bridgeModule from "../lib/bridge";
import type { MachineAdviceInfo } from "../lib/machine";

const w = () => window as unknown as Record<string, unknown>;

const advice = (overrides: Partial<MachineAdviceInfo>): MachineAdviceInfo => ({
  recipe_id: "p-1",
  verdict: "ok",
  words: "This Mac can take another session",
  reasons: [],
  projected_bytes: 2_100_000_000,
  based_on: 6,
  relief_session_id: null,
  relief_bytes: null,
  ...overrides,
});

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  for (const key of Object.keys(w()).filter((key) => key.startsWith("__AIJ_"))) delete w()[key];
});

describe("the advice line", () => {
  it("says what a launch costs, learned or not", () => {
    expect(basisWords(advice({}))).toBe("about 2.1 GB a session, from your last 6");
    expect(basisWords(advice({ based_on: 0, projected_bytes: 1e9 }))).toBe("about 1.0 GB a session — not measured yet");
    render(<MachineAdviceLine advice={advice({})} nameOf={() => null} />);
    const line = screen.getByTestId("machine-advice");
    expect(line.getAttribute("data-verdict")).toBe("ok");
    expect(line.textContent).toContain("This Mac can take another session");
    expect(line.textContent).not.toContain("You can still start it");
  });

  it("says a tight or no verdict on the default size is not measured yet", () => {
    render(
      <MachineAdviceLine
        advice={advice({ verdict: "tight", words: "Tight — this Mac can take it, just", reasons: ["memory is getting tight"], based_on: 0, projected_bytes: 1e9 })}
        nameOf={() => null}
      />,
    );
    expect(screen.getByTestId("machine-advice").textContent).toContain("Assumes about 1.0 GB a session — not measured yet.");
    cleanup();
    render(<MachineAdviceLine advice={advice({ verdict: "no", words: "Not now", reasons: ["memory is short"] })} nameOf={() => null} />);
    expect(screen.getByTestId("machine-advice").textContent).not.toContain("not measured yet");
    cleanup();
    render(<MachineChip advice={advice({ verdict: "tight", words: "Tight", reasons: ["memory is getting tight"], based_on: 0, projected_bytes: 1e9 })} />);
    expect(screen.getByTestId("machine-chip").getAttribute("title")).toContain("not measured yet");
  });

  it("explains a tight or no verdict, names what would make room, and never forbids", () => {
    render(
      <MachineAdviceLine
        advice={advice({
          verdict: "no",
          words: "Not now — this Mac is short of room",
          reasons: ["macOS reports critical memory pressure", "it would leave about 800 MB of memory free"],
          relief_session_id: "s-heavy",
          relief_bytes: 3_200_000_000,
        })}
        nameOf={(id) => (id === "s-heavy" ? "feat/read-model" : null)}
      />,
    );
    const text = screen.getByTestId("machine-advice").textContent!;
    expect(text).toContain("Not now — this Mac is short of room");
    expect(text).toContain("macOS reports critical memory pressure; it would leave about 800 MB of memory free.");
    expect(text).toContain("Closing feat/read-model frees about 3.2 GB.");
    expect(text).toContain("You can still start it.");
  });

  it("chips a launch-settings row only when it isn't OK", () => {
    const { container } = render(<MachineChip advice={advice({})} />);
    expect(container.innerHTML).toBe("");
    cleanup();
    render(<MachineChip advice={advice({ verdict: "tight", words: "Tight", reasons: ["x"] })} />);
    expect(screen.getByTestId("machine-chip").textContent).toBe("This Mac: tight");
  });
});

describe("in the New session sheet", () => {
  beforeEach(() => {
    w().__AIJ_RECIPES__ = [
      { id: "p-1", name: "Main", account_id: "claude-work", account_label: "Work Max", provider: "claude-code", model: "opus", default_cwd: null },
    ];
    w().__AIJ_QUOTA_DISPLAY__ = [];
    w().__AIJ_RANKING__ = [];
  });

  it("shows the Mac's verdict beside the account's and still launches", async () => {
    w().__AIJ_MACHINE__ = "pressure";
    const launch = vi.spyOn(bridgeModule, "launchSession").mockResolvedValue({ session_id: "s1", pid: 1 } as never);
    const launched = vi.fn();
    render(
      <NewSessionModal
        projects={[]}
        accounts={[{ id: "claude-work", provider: "claude-code", label: "Work Max", plan: null, login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false }] as never}
        initialDirectory={null}
        onClose={() => {}}
        onLaunched={launched}
      />,
    );
    const line = await screen.findByTestId("machine-advice");
    await waitFor(() => expect(line.getAttribute("data-verdict")).toBe("no"));
    expect(line.textContent).toContain("You can still start it.");
    const start = screen.getByTestId("launch-now");
    expect((start as HTMLButtonElement).disabled).toBe(false);
    fireEvent.click(start);
    await waitFor(() => expect(launch).toHaveBeenCalled());
  });
});
