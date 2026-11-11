/**
 * AJ-E704 vitest — the pairing wizard's code input and failure copy:
 * digits auto-advance, Backspace steps back, a pasted 6-digit code or
 * pairing link fills every box, Connect stays disabled until a machine
 * and six digits are chosen, and every failure reason has its own
 * words (never a generic "something went wrong").
 */
import { act, cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { PairingWizard } from "../screens/PairingWizard";

const w = () => window as unknown as Record<string, unknown>;

afterEach(cleanup);

beforeEach(() => {
  for (const key of ["__AIJ_PAIR_JOIN__", "__AIJ_PAIR_HOST__", "__AIJ_PAIR_JOIN_FAIL__", "__AIJ_FLEET_CALLS__"]) delete w()[key];
  w().__AIJ_LINK__ = { enabled: true, port: 47821, error: null };
  w().__AIJ_DISCOVERED__ = [{ instance_id: "a1b2c3d4", label: "Unnamed machine (a1b2c3d4)", version: "1.0" }];
});

const join = () =>
  render(
    <PairingWizard mode="join" selfLabel="Studio Mac" selfFingerprint="3F2A" onClose={() => {}} onPaired={() => {}} />,
  );

const digits = (root: HTMLElement) =>
  Array.from(root.querySelectorAll<HTMLInputElement>('[data-testid="pairing-digit"]'));

describe("PairingWizard — join code input", () => {
  it("auto-advances, steps back on Backspace, and gates Connect", async () => {
    const view = join();
    await waitFor(() => expect(view.getByTestId("pairing-machine")).toBeTruthy());
    const boxes = digits(view.container);
    const connect = view.getByTestId("pairing-connect") as HTMLButtonElement;
    boxes[0]!.focus();
    for (const [index, digit] of ["4", "8", "2"].entries()) {
      fireEvent.change(boxes[index]!, { target: { value: digit } });
    }
    expect(document.activeElement).toBe(boxes[3]);
    fireEvent.keyDown(boxes[3]!, { key: "Backspace" });
    expect(document.activeElement).toBe(boxes[2]);
    expect(connect.disabled).toBe(true);
    for (const [index, digit] of ["9", "1", "3"].entries()) {
      fireEvent.change(boxes[index + 3]!, { target: { value: digit } });
    }
    await waitFor(() => expect(connect.disabled).toBe(false));
  });

  it("a pasted code or pairing link fills every box", async () => {
    const view = join();
    await waitFor(() => expect(view.getByTestId("pairing-machine")).toBeTruthy());
    const boxes = digits(view.container);
    fireEvent.paste(boxes[0]!, { clipboardData: { getData: () => "731 058" } });
    expect(boxes.map((box) => box.value).join("")).toBe("731058");
    fireEvent.paste(boxes[0]!, {
      clipboardData: { getData: () => "aijuggler-pair:v1:482913:C2E91F7A0B368D54E1A07C2946BF3D18" },
    });
    expect(boxes.map((box) => box.value).join("")).toBe("482913");
    expect(view.getByTestId("pairing-link-pasted")).toBeTruthy();
    // Anything else is ignored, never half-applied.
    fireEvent.paste(boxes[0]!, { clipboardData: { getData: () => "hello" } });
    expect(boxes.map((box) => box.value).join("")).toBe("482913");
  });
});

describe("PairingWizard — failure copy", () => {
  const reasons = ["mismatch", "cancelled", "unreachable", "fingerprint", "not_open", "busy"];
  it.each(reasons)("%s has its own title and next action", async (reason) => {
    w().__AIJ_PAIR_JOIN_FAIL__ = reason;
    const view = join();
    await waitFor(() => expect(view.getByTestId("pairing-machine")).toBeTruthy());
    const boxes = digits(view.container);
    fireEvent.paste(boxes[0]!, { clipboardData: { getData: () => "482913" } });
    await act(async () => {
      fireEvent.click(view.getByTestId("pairing-connect"));
    });
    const failure = await view.findByTestId("pairing-failure");
    expect(failure.getAttribute("data-reason")).toBe(reason);
    expect(failure.textContent).not.toContain("Pairing didn't finish");
    expect(view.getByTestId("pairing-retry")).toBeTruthy();
  });
});

describe("PairingWizard — the keyboard (AJ-E710's sweep)", () => {
  it("takes focus when it opens, keeps Tab inside, and gives focus back when it closes", async () => {
    const opener = document.createElement("button");
    opener.textContent = "Join a machine…";
    document.body.appendChild(opener);
    opener.focus();
    const view = join();
    const dialog = view.getByTestId("pairing-wizard");
    // Join's machines arrive after it opens; the first one takes focus.
    await waitFor(() => expect(dialog.contains(document.activeElement)).toBe(true));
    // Tab from the last control comes back to the first, never the page behind.
    const focusable = Array.from(dialog.querySelectorAll<HTMLElement>("button:not(:disabled), input:not(:disabled)"));
    focusable.at(-1)!.focus();
    fireEvent.keyDown(focusable.at(-1)!, { key: "Tab" });
    expect(dialog.contains(document.activeElement)).toBe(true);
    view.unmount();
    expect(document.activeElement).toBe(opener);
    opener.remove();
  });
});

describe("PairingWizard — Pair by address from the keyboard (AJ-E710)", () => {
  it("opening it moves focus into the address field", async () => {
    const view = join();
    fireEvent.click(await view.findByTestId("pairing-toggle-address"));
    await waitFor(() => expect(document.activeElement).toBe(view.getByTestId("pairing-address")));
  });
});
