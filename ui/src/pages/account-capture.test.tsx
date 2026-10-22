/**
 * AJ-E1530 Req.1 (ADR-0018 amendment, opt-in): an account used where it
 * is offers to measure its sessions started elsewhere; the exact change
 * shows first; it turns off from the same place; a changed status line is
 * the user's.
 */
import { act, cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { AccountCapture } from "./AccountCapture";

const w = () => window as unknown as Record<string, unknown>;
afterEach(() => {
  cleanup();
  delete w().__AIJ_CAPTURE__;
});

describe("AJ-E1530 measuring limits for sessions started elsewhere", () => {
  it("shows the exact change first, turns on, and off again from the same place", async () => {
    const view = render(<AccountCapture accountId="a1" />);
    await waitFor(() => expect(view.getByTestId("account-capture-words").textContent).toContain("aren't measured"));
    await act(async () => {
      fireEvent.click(view.getByTestId("account-capture-open"));
    });
    const diff = (await view.findByTestId("account-capture-diff")).textContent!;
    expect(diff).toContain("- \"statusLine\"");
    expect(diff).toContain("+ \"statusLine\"");
    expect(diff).toContain("Nothing else in the file changes.");
    expect(view.getByTestId("account-capture-preview").textContent).toContain("~/.claude/settings.json");
    await act(async () => {
      fireEvent.click(view.getByTestId("account-capture-on"));
    });
    await waitFor(() => expect(view.getByTestId("account-capture-words").textContent).toContain("On:"));
    expect(view.queryByTestId("account-capture-preview")).toBeNull();
    await act(async () => {
      fireEvent.click(view.getByTestId("account-capture-off"));
    });
    await waitFor(() => expect(view.getByTestId("account-capture-open")).toBeTruthy());
  });

  it("says when the user's status line changed since, and isn't offered where it doesn't apply", async () => {
    w().__AIJ_CAPTURE__ = {
      a1: { status: { offered: true, state: "changed", file: "~/.claude/settings.json", reason: null } },
      copy: { status: { offered: false, state: "off", file: null, reason: "this account is AI Juggler's own copy" } },
    };
    const view = render(<AccountCapture accountId="a1" />);
    await waitFor(() => expect(view.getByTestId("account-capture-words").textContent).toContain("changed since"));
    expect(view.getByTestId("account-capture-off").textContent).toBe("Forget it");
    cleanup();
    const copy = render(<AccountCapture accountId="copy" />);
    await act(async () => {});
    expect(copy.queryByTestId("account-capture")).toBeNull();
  });

  it("a refusal is said in words, and nothing turns on", async () => {
    w().__AIJ_CAPTURE__ = {
      a1: {
        status: { offered: true, state: "off", file: "~/.claude/settings.json", reason: null },
        refuse: "~/.claude/settings.json isn't valid JSON (a value it can't read), so AI Juggler won't change it",
      },
    };
    const view = render(<AccountCapture accountId="a1" />);
    await act(async () => {});
    await act(async () => {
      fireEvent.click(await view.findByTestId("account-capture-open"));
    });
    await waitFor(() => expect(view.getByTestId("account-capture-problem").textContent).toContain("isn't valid JSON"));
    expect(view.queryByTestId("account-capture-preview")).toBeNull();
  });
});
