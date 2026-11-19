/**
 * D-126 — the one Alert at three levels: info is neutral (never amber),
 * needs-you is amber and only for what the user must do, broken is red.
 * Each says itself at its level's urgency (status, or alert for broken),
 * its icon a different shape. Banner's tones are drawn through them, so
 * an update or a reminder is no longer tinted like a warning.
 */
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { Alert, ALERT_LEVELS } from "./Alert";
import { Banner, BANNER_LEVEL } from "./Banner";

afterEach(cleanup);

describe("Alert", () => {
  it("says a fact politely, in neutral, with no amber", () => {
    render(<Alert level="info" title="Info.">Each app has its own folder.</Alert>);
    const box = screen.getByRole("status");
    expect(box.textContent).toBe("Info. Each app has its own folder.");
    expect(box.className).toContain("bg-surface-2");
    expect(box.className).not.toMatch(/warning/);
  });

  it("puts amber only on what needs the user, with its action", () => {
    const signIn = vi.fn();
    render(
      <Alert level="needs-you" title="Work Max needs its sign-in." actions={<button type="button" onClick={signIn}>Sign in</button>}>
        Nothing is lost meanwhile.
      </Alert>,
    );
    const box = screen.getByRole("status");
    expect(box.className).toContain("var(--color-warning)");
    fireEvent.click(screen.getByRole("button", { name: "Sign in" }));
    expect(signIn).toHaveBeenCalled();
  });

  it("says what broke at once, in red", () => {
    render(<Alert level="broken" title="The pool's bridge stopped." />);
    const box = screen.getByRole("alert");
    expect(box.className).toContain("var(--color-danger)");
    expect(box.getAttribute("data-level")).toBe("broken");
  });

  it("has no live region of its own where its region already speaks, and a named close", () => {
    const close = vi.fn();
    const view = render(<Alert level="info" live={null} title="Saved" onDismiss={close} dismissLabel="Close the note" testId="a" />);
    expect(view.getByTestId("a").getAttribute("role")).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Close the note" }));
    expect(close).toHaveBeenCalled();
  });

  it("tells the levels apart by their icons' shapes, not colour alone", () => {
    const icons = Object.values(ALERT_LEVELS).map((level) => level.Icon);
    expect(new Set(icons).size).toBe(3);
  });
});

describe("Banner, through the Alert's levels", () => {
  it("maps each tone onto a level: accent and neutral are info, warning needs you, danger is broken", () => {
    expect(BANNER_LEVEL).toEqual({ accent: "info", neutral: "info", warning: "needs-you", danger: "broken" });
  });

  it("draws an update (accent) as info: no tint, no amber, its tone kept for callers", () => {
    render(<Banner tone="accent" title="AI Juggler 0.4.6 is available" detail="Nothing is installed until you choose." testId="b" actions={[{ label: "Review", primary: true, onClick: () => {} }]} />);
    const box = screen.getByTestId("b");
    expect(box.getAttribute("data-level")).toBe("info");
    expect(box.getAttribute("data-tone")).toBe("accent");
    expect(box.className).not.toContain("from-accent-soft");
    expect(box.getAttribute("role")).toBeNull();
    expect(screen.getByRole("button", { name: "Review" })).toBeTruthy();
  });

  it("keeps its meter, its close and the role it is given", () => {
    const close = vi.fn();
    render(<Banner tone="danger" role="alert" title="Showing the last state AI Juggler knew" meter={{ used: 80, label: "80% used" }} onDismiss={close} />);
    expect(screen.getByRole("alert").getAttribute("data-level")).toBe("broken");
    expect(screen.getByRole("meter", { name: "80% used" }).getAttribute("aria-valuenow")).toBe("80");
    fireEvent.click(screen.getByRole("button", { name: "Close: Showing the last state AI Juggler knew" }));
    expect(close).toHaveBeenCalled();
  });
});
