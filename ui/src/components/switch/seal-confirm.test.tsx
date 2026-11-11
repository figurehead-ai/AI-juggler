/**
 * D-62 (AJ-E3009): a switch into or out of a sealed pool waits for the
 * person's tick; any other switch doesn't.
 */
import type { JSX } from "react";
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import * as model from "../../lib/ipc/pool-model";
import { SwitchPoolLines, useSealOwed } from "./ConfirmBody";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

function Gate(): JSX.Element {
  const owed = useSealOwed("s-1", "rec-home");
  return <span data-testid="gate">{owed ? "waits" : "free"}</span>;
}

describe("D-62 a sealed pool's switch", () => {
  it("waits for the tick, and only then lets the switch go", async () => {
    vi.spyOn(model, "poolsSwitchLines").mockResolvedValue([
      { pool_id: "p-team", kind: "leaves", words: "Leaves «Team» for «Client», which is sealed.", confirm: true },
    ]);
    render(
      <>
        <SwitchPoolLines sessionId="s-1" targetProfileId="rec-home" />
        <Gate />
      </>,
    );
    await waitFor(() => expect(screen.getByTestId("gate").textContent).toBe("waits"));
    await act(async () => fireEvent.click(screen.getByLabelText(/Switch anyway/)));
    expect(screen.getByTestId("gate").textContent).toBe("free");
  });

  it("an ordinary switch never waits", async () => {
    vi.spyOn(model, "poolsSwitchLines").mockResolvedValue([{ pool_id: "p-team", kind: "stays", words: "Stays in «Team».", confirm: false }]);
    render(
      <>
        <SwitchPoolLines sessionId="s-1" targetProfileId="rec-home" />
        <Gate />
      </>,
    );
    await waitFor(() => expect(screen.getByTestId("switch-pool-lines")).toBeTruthy());
    expect(screen.getByTestId("gate").textContent).toBe("free");
    expect(screen.queryByTestId("switch-seal-confirm")).toBeNull();
  });

  it("0.4.1: asks again for where it continues, and Desktop's copy stays watched", async () => {
    const lines = vi.spyOn(model, "poolsSwitchLines").mockImplementation(async (_s, _p, destination) =>
      destination === "desktop"
        ? [{ pool_id: "p-team", kind: "watched", words: "Stays in «Team», watched: it runs in Claude.", confirm: false }]
        : [{ pool_id: "p-team", kind: "stays", words: "Stays in «Team».", confirm: false }],
    );
    const view = render(<SwitchPoolLines sessionId="s-1" targetProfileId="rec-home" destination="cli" />);
    await waitFor(() => expect(screen.getByTestId("switch-pool-lines").textContent).toContain("Stays in «Team»."));
    view.rerender(<SwitchPoolLines sessionId="s-1" targetProfileId="rec-home" destination="desktop" />);
    await waitFor(() => expect(screen.getByTestId("switch-pool-lines").textContent).toContain("watched"));
    expect(lines).toHaveBeenLastCalledWith("s-1", "rec-home", "desktop");
  });
});
