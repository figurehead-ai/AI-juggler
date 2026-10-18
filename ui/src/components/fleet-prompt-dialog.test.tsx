/**
 * AJ-E709 vitest — "Ask each time": the prompt names the machine and the
 * action, focuses Deny, says silence is a Deny, and only an explicit
 * Allow is sent as an allow.
 */
import { act, cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { FleetPromptDialog } from "./FleetPromptDialog";

const w = () => window as unknown as Record<string, unknown>;

afterEach(cleanup);
beforeEach(() => {
  w().__AIJ_LINK__ = { enabled: true, port: 47821, error: null };
  w().__AIJ_FLEET_PROMPTS__ = [
    { id: 7, peer_label: "Laptop", action: "stop Work Max (claude-code)", expires_at_ms: Date.now() + 20_000 },
  ];
  delete w().__AIJ_FLEET_CALLS__;
});

describe("FleetPromptDialog", () => {
  it("asks plainly, focuses Deny, and sends an Allow only when clicked", async () => {
    const view = render(<FleetPromptDialog />);
    const dialog = await view.findByTestId("fleet-prompt");
    expect(dialog.textContent).toContain("Laptop asks to act on this Mac");
    expect(dialog.textContent).toContain("stop Work Max (claude-code)");
    expect(dialog.textContent).toMatch(/denied in \d+ s/);
    await waitFor(() => expect(document.activeElement).toBe(view.getByTestId("fleet-prompt-deny")));
    await act(async () => {
      fireEvent.click(view.getByTestId("fleet-prompt-allow"));
    });
    expect(w().__AIJ_FLEET_CALLS__).toEqual(["prompt:7:allow"]);
    await waitFor(() => expect(view.queryByTestId("fleet-prompt")).toBeNull());
  });

  it("shows nothing while Link is off", async () => {
    w().__AIJ_LINK__ = { enabled: false, port: null, error: null };
    const view = render(<FleetPromptDialog />);
    await new Promise((resolve) => setTimeout(resolve, 50));
    expect(view.queryByTestId("fleet-prompt")).toBeNull();
  });
});
