/**
 * AJ-E1204 vitest — the recovery-code sheet: the code is selectable text
 * with Copy and Save; the exact warning is shown; "Start backups" stays
 * disabled until "I've saved my recovery code" is ticked; Later saves
 * nothing.
 */
import { act, cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { RecoveryCodeSheet } from "./RecoveryCodeSheet";

const w = () => window as unknown as Record<string, unknown>;
const calls = () => (w().__AIJ_VAULT_CALLS__ as string[] | undefined) ?? [];
const CODE = "MZXW-6YTB-OI3D-EZLM-NFXG-K4TU-OJSW-Y5DF-MFZG-K3TF-ON2G-K3DF-NZTW-Q2AB";

afterEach(cleanup);
beforeEach(() => {
  for (const key of Object.keys(w()).filter((key) => key.startsWith("__AIJ_VAULT"))) delete w()[key];
  w().__AIJ_VAULT_CODE__ = CODE;
});

describe("AJ-E1204 recovery code", () => {
  it("shows the code with Copy and Save and the exact warning; confirm is required", async () => {
    w().__AIJ_VAULT_SAVE_DEST__ = "/Users/dev/Documents/AI Juggler recovery code.txt";
    let closed: boolean | null = null;
    const { container } = render(<RecoveryCodeSheet onClose={(saved) => (closed = saved)} />);
    await waitFor(() => expect(container.querySelector("[data-testid='recovery-code']")!.textContent).toBe(CODE));
    expect(container.querySelector("[data-testid='recovery-code']")!.className).toContain("select-all");
    expect(container.querySelector("[data-testid='recovery-warning']")!.textContent).toContain(
      "Without this code, a new Mac can't open your backups. We can't recover it for you.",
    );
    const done = () => container.querySelector<HTMLButtonElement>("[data-testid='recovery-done']")!;
    expect(done().disabled).toBe(true);
    await act(async () => {
      fireEvent.click(container.querySelector("[data-testid='recovery-save']")!);
    });
    expect(calls()).toContain("save:/Users/dev/Documents/AI Juggler recovery code.txt");
    fireEvent.click(container.querySelector("[data-testid='recovery-confirm']")!);
    expect(done().disabled).toBe(false);
    await act(async () => {
      fireEvent.click(done());
    });
    expect(calls()).toEqual(["setup", "save:/Users/dev/Documents/AI Juggler recovery code.txt", "confirm"]);
    expect(closed).toBe(true);
  });

  it("Later confirms nothing", async () => {
    let closed: boolean | null = null;
    const { container, getByText } = render(<RecoveryCodeSheet onClose={(saved) => (closed = saved)} />);
    await waitFor(() => expect(container.querySelector("[data-testid='recovery-code']")!.textContent).toBe(CODE));
    fireEvent.click(getByText(/Later/));
    expect(closed).toBe(false);
    expect(calls()).toEqual(["setup"]);
  });
});
