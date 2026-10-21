/**
 * AJ-E306 / AJ-E1023 vitest — flow (c): Add account's tool choice (shown
 * when more than one tool is installed). Cards carry the install state,
 * verified-range badge and kind; not-installed cards are never
 * selectable; desktop cards state the single-instance limit BEFORE
 * selection; choosing a card hands that tool to Add account.
 */
import { cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

afterEach(cleanup);
import { AddProviderSheet } from "../screens/AddProviderSheet";

const w = () => window as unknown as Record<string, unknown>;

const INSTALLS = [
  {
    provider: "claude-code",
    display_name: "Claude Code",
    install_url: "https://claude.com/download",
    status: "installed",
    version: "2.1.296",
    binary: "/usr/local/bin/claude",
    notes: [],
    verified_against: ["2.0.x", "2.1.x"],
    kind: "terminal" as const,
    single_instance_note: null,
    can_use_in_place: true,
  },
  {
    provider: "fake-desktop",
    display_name: "Fake Desktop",
    install_url: "https://example.com",
    status: "unverified",
    version: "0.4.0",
    binary: null,
    notes: [],
    verified_against: [],
    kind: "desktop" as const,
    single_instance_note: "Single-account app — Juggler tracks, cannot isolate",
  },
  {
    provider: "codex",
    display_name: "Codex",
    install_url: "https://developers.openai.com/codex",
    status: "not_installed",
    version: null,
    binary: null,
    notes: [],
    verified_against: [],
    kind: "terminal" as const,
    single_instance_note: null,
    can_use_in_place: true,
  },
];

beforeEach(() => {
  w().__AIJ_PROVIDER_INSTALLS__ = INSTALLS;
});

describe("AJ-E306 flow (c) — Add account's tool choice", () => {
  it("renders one card per adapter — installed first, kind badges, verified ranges, desktop limit stated BEFORE selection", async () => {
    const { container } = render(
      <AddProviderSheet onClose={() => {}} onChoose={() => {}} />,
    );
    const cards = await waitFor(() => {
      const found = container.querySelectorAll("[data-testid='provider-card']");
      expect(found.length).toBe(3);
      return Array.from(found);
    });
    // Installed sorts first (Req.1); each card carries its status.
    expect(cards[0]!.getAttribute("data-status")).toBe("installed");
    expect(cards[0]!.getAttribute("data-kind")).toBe("terminal");
    expect(cards[1]!.getAttribute("data-kind")).toBe("desktop");
    expect(cards[2]!.getAttribute("data-status")).toBe("not_installed");
    // The verified range badge is honest both ways.
    expect(cards[0]!.querySelector("[data-testid='verified-badge']")!.textContent).toContain(
      "Verified against 2.0.x, 2.1.x",
    );
    expect(cards[1]!.querySelector("[data-testid='verified-badge']")!.textContent).toContain(
      "Unverified",
    );
    // Desktop: kind badge + the single-instance limit inline (Req.3).
    expect(cards[1]!.textContent).toContain("Desktop");
    expect(cards[1]!.textContent).toContain("one session at a time");
  });

  it("not-installed cards are never selectable — only [Install →] (Req.1)", async () => {
    const { container } = render(
      <AddProviderSheet onClose={() => {}} onChoose={() => {}} />,
    );
    await waitFor(() =>
      expect(container.querySelectorAll("[data-testid='provider-card']").length).toBe(3),
    );
    const missing = container.querySelector(
      "[data-testid='provider-card'][data-status='not_installed']",
    )!;
    expect(missing.querySelector("[data-testid='provider-choose']")).toBeNull();
    const link = missing.querySelector("a")!;
    expect(link.getAttribute("href")).toBe("https://developers.openai.com/codex");
    expect(link.textContent).toContain("Install Codex");
  });

  it("choosing a card hands that tool to Add account (Req.2); Escape closes; it is a labelled dialog", async () => {
    const onChoose = vi.fn();
    const onClose = vi.fn();
    const { container, getByRole } = render(<AddProviderSheet onClose={onClose} onChoose={onChoose} />);
    await waitFor(() =>
      expect(container.querySelectorAll("[data-testid='provider-card']").length).toBe(3),
    );
    expect(getByRole("dialog", { name: "Add an account: which tool?" })).toBeTruthy();
    const choices = container.querySelectorAll<HTMLButtonElement>("[data-testid='provider-choose']");
    fireEvent.click(choices[1]!);
    // Installed sort first: the second card is the installed desktop tool.
    expect(onChoose).toHaveBeenCalledWith("fake-desktop");
    fireEvent.keyDown(window, { key: "Escape" });
    expect(onClose).toHaveBeenCalledOnce();
  });
});

describe("AJ-E1023 the tool choice is a real modal dialog (WCAG 2.4.3)", () => {
  it("a parent re-render with a new onClose doesn't pull focus off the card the user is on", async () => {
    const { container, rerender } = render(<AddProviderSheet onClose={() => {}} onChoose={() => {}} />);
    await waitFor(() => expect(container.querySelectorAll("[data-testid='provider-choose']").length).toBe(2));
    const second = container.querySelectorAll<HTMLButtonElement>("[data-testid='provider-choose']")[1]!;
    second.focus();
    // App passes a fresh arrow on every render (its 15 s clock, session events).
    rerender(<AddProviderSheet onClose={() => {}} onChoose={() => {}} />);
    rerender(<AddProviderSheet onClose={() => {}} onChoose={() => {}} />);
    expect(document.activeElement).toBe(second);
  });

  it("Tab and Shift+Tab stay inside it", async () => {
    const { container } = render(<AddProviderSheet onClose={() => {}} onChoose={() => {}} />);
    await waitFor(() => expect(container.querySelectorAll("[data-testid='provider-choose']").length).toBe(2));
    const focusables = Array.from(
      container.querySelectorAll<HTMLElement>("[role='dialog'] button:not(:disabled), [role='dialog'] a[href]"),
    );
    const first = focusables[0]!;
    const last = focusables[focusables.length - 1]!;
    last.focus();
    fireEvent.keyDown(window, { key: "Tab" });
    expect(document.activeElement).toBe(first);
    fireEvent.keyDown(window, { key: "Tab", shiftKey: true });
    expect(document.activeElement).toBe(last);
  });

  it("closing returns focus to what opened it", async () => {
    const opener = document.createElement("button");
    opener.textContent = "Add account";
    document.body.append(opener);
    opener.focus();
    const view = render(<AddProviderSheet onClose={() => {}} onChoose={() => {}} />);
    await waitFor(() => expect(view.container.querySelectorAll("[data-testid='provider-card']").length).toBe(3));
    expect(document.activeElement).not.toBe(opener);
    view.unmount();
    expect(document.activeElement).toBe(opener);
    opener.remove();
  });

  it("the latest onClose runs on Escape", async () => {
    const first = vi.fn();
    const latest = vi.fn();
    const { container, rerender } = render(<AddProviderSheet onClose={first} onChoose={() => {}} />);
    await waitFor(() => expect(container.querySelectorAll("[data-testid='provider-card']").length).toBe(3));
    rerender(<AddProviderSheet onClose={latest} onChoose={() => {}} />);
    fireEvent.keyDown(window, { key: "Escape" });
    expect(first).not.toHaveBeenCalled();
    expect(latest).toHaveBeenCalledOnce();
  });
});
