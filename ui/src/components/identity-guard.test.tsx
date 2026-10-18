/**
 * AJ-E1001 vitest — the wrong-account guard: a mismatch pauses EVERY
 * launch (the bridge raises one dialog), shows expected vs detected in
 * words, offers switch / check again / cancel and never "continue
 * anyway"; the rail marker and the identity words are text.
 */
import { act, cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { IdentityMismatchDialog } from "./IdentityMismatchDialog";
import { launchSession, type IdentityCheckInfo } from "../lib/bridge";
import { identityWords } from "../lib/identity";

const w = () => window as unknown as Record<string, unknown>;

const mismatch: IdentityCheckInfo = {
  state: "mismatch",
  provider: "claude-code",
  account_id: "acc-work",
  account_label: "Work Max",
  model: null,
  expected: "o…@work.example · Acme",
  detected: "o…@home.example · Acme",
  reason: null,
  alternatives: [{ recipe_id: "rec-home", recipe_name: "Main", account_label: "Personal" }],
};

afterEach(cleanup);
beforeEach(() => {
  delete w().__AIJ_IDENTITY_CHECK__;
  delete w().__AIJ_LAUNCH_CALLS__;
});

describe("AJ-E1001 identity guard", () => {
  it("a verified Account launches straight away", async () => {
    await launchSession("rec-work", null);
    expect(w().__AIJ_LAUNCH_CALLS__).toEqual(["rec-work"]);
  });

  it("a mismatch pauses with expected vs detected, and switching launches the right Account", async () => {
    w().__AIJ_IDENTITY_CHECK__ = { "rec-work": mismatch };
    const { container } = render(<IdentityMismatchDialog />);
    let launched: Promise<unknown> = Promise.resolve();
    await act(async () => {
      launched = launchSession("rec-work", null);
    });
    const q = (id: string) => container.querySelector<HTMLElement>(`[data-testid='${id}']`);
    await waitFor(() => expect(q("identity-mismatch")).not.toBeNull());
    expect(q("identity-expected")!.textContent).toBe("o…@work.example · Acme");
    expect(q("identity-detected")!.textContent).toBe("o…@home.example · Acme");
    expect(container.textContent).not.toMatch(/continue anyway/i);
    expect(document.activeElement).toBe(q("identity-switch"));
    expect(w().__AIJ_LAUNCH_CALLS__).toBeUndefined();
    await act(async () => {
      fireEvent.click(q("identity-switch")!);
      await launched;
    });
    expect(w().__AIJ_LAUNCH_CALLS__).toEqual(["rec-home"]);
  });

  it("cancel launches nothing and says why; check again re-probes", async () => {
    w().__AIJ_IDENTITY_CHECK__ = { "rec-work": mismatch };
    const { container } = render(<IdentityMismatchDialog />);
    const q = (id: string) => container.querySelector<HTMLElement>(`[data-testid='${id}']`);
    let cancelled: Promise<unknown> = Promise.resolve();
    await act(async () => {
      cancelled = launchSession("rec-work", null).catch((error: Error) => error.message);
    });
    await waitFor(() => expect(q("identity-cancel")).not.toBeNull());
    await act(async () => {
      fireEvent.click(q("identity-cancel")!);
    });
    expect(await cancelled).toMatch(/Not launched — «Work Max» is signed in as o…@home.example/);
    expect(w().__AIJ_LAUNCH_CALLS__).toBeUndefined();

    let rechecked: Promise<unknown> = Promise.resolve();
    await act(async () => {
      rechecked = launchSession("rec-work", null);
    });
    await waitFor(() => expect(q("identity-recheck")).not.toBeNull());
    // The user fixed the sign-in: the next check passes.
    w().__AIJ_IDENTITY_CHECK__ = {};
    await act(async () => {
      fireEvent.click(q("identity-recheck")!);
      await rechecked;
    });
    expect(w().__AIJ_LAUNCH_CALLS__).toEqual(["rec-work"]);
  });

  it("without a dialog mounted, a mismatch never continues silently", async () => {
    w().__AIJ_IDENTITY_CHECK__ = { "rec-work": mismatch };
    await expect(launchSession("rec-work", null)).rejects.toThrow(/Not launched/);
    expect(w().__AIJ_LAUNCH_CALLS__).toBeUndefined();
  });

  it("identity states read as words", () => {
    expect(identityWords({ identity_state: "verified", identity_label: "o…@x.co" })!.text).toBe(
      "Signed in as o…@x.co — verified",
    );
    expect(identityWords({ identity_state: "unverified" })!.text).toMatch(/Identity unverified/);
    expect(identityWords({ identity_state: "signed_out" })!.text).toMatch(/Not signed in yet/);
    expect(identityWords({})).toBeNull();
  });
});
