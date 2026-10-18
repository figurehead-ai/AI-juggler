/**
 * AJ-E306 vitest — flow (c): the Add Provider sheet. Cards carry the
 * install state, verified-range badge and kind; not-installed cards
 * are never selectable; desktop cards state the single-instance
 * limit BEFORE selection; choosing a card runs the LOCKED wizard
 * (flows (d)'s no-picker machinery).
 */
import { fireEvent, render, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";
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
  },
];

beforeEach(() => {
  w().__AIJ_PROVIDER_INSTALLS__ = INSTALLS;
});

describe("AJ-E306 flow (c) — AddProviderSheet", () => {
  it("renders one card per adapter — installed first, kind badges, verified ranges, desktop limit stated BEFORE selection", async () => {
    const { container } = render(
      <AddProviderSheet onClose={() => {}} onAccountCreated={() => {}} openLoginTerminal={() => {}} />,
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
    expect(cards[1]!.textContent).toContain("one Instance at a time");
  });

  it("not-installed cards are never selectable — only [Install →] (Req.1)", async () => {
    const { container } = render(
      <AddProviderSheet onClose={() => {}} onAccountCreated={() => {}} openLoginTerminal={() => {}} />,
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

  it("choosing a card opens the wizard LOCKED — no Step-0 gate, straight to the provider-scoped scan (Req.2)", async () => {
    const { container } = render(
      <AddProviderSheet onClose={() => {}} onAccountCreated={() => {}} openLoginTerminal={() => {}} />,
    );
    await waitFor(() =>
      expect(container.querySelectorAll("[data-testid='provider-card']").length).toBe(3),
    );
    fireEvent.click(container.querySelector("[data-testid='provider-choose']")!);
    // The locked wizard mounts at DETECT (no install step).
    await waitFor(() =>
      expect(
        container.querySelector("[data-testid='onboarding-wizard']"),
      ).not.toBeNull(),
    );
    expect(container.querySelector("[data-testid='wizard-step-install']")).toBeNull();
  });
});
