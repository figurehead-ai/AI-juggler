/**
 * AJ-E1524 — the welcome Home: shown with zero accounts and hidden with
 * one; the found-tools line matches what the probe found; each action
 * opens the right flow; one h1, the mark decorative, everything reachable
 * by keyboard; Usage and Fleet point back to it while no account is in.
 */
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { foundWords, listWords, Welcome, WelcomePointer } from "./Welcome";
import { HomeScreen } from "../screens/Home";
import type { AccountInfo, ProviderInstallData } from "../lib/bridge";

const w = () => window as unknown as Record<string, unknown>;
const row = (provider: string, display_name: string, status: string): ProviderInstallData => ({
  provider,
  display_name,
  install_url: `https://${provider}.example/install`,
  status,
  version: status === "installed" ? "1.0.0" : null,
  binary: null,
  notes: [],
});

beforeEach(() => {
  delete w().__AIJ_PROVIDER_INSTALLS__;
});
afterEach(cleanup);

describe("AJ-E1524 the welcome's words", () => {
  it("lists what's installed, or says nothing is yet", () => {
    expect(listWords(["A"])).toBe("A");
    expect(listWords(["A", "B"])).toBe("A and B");
    expect(listWords(["A", "B", "C"])).toBe("A, B and C");
    expect(foundWords([row("one", "Tool One", "installed"), row("two", "Tool Two", "unverified")])).toBe(
      "We found Tool One and Tool Two on this Mac.",
    );
    expect(foundWords([row("one", "Tool One", "installed"), row("two", "Tool Two", "not_installed")])).toBe(
      "We found Tool One on this Mac.",
    );
    expect(foundWords([row("one", "Tool One", "not_installed")])).toBe("No AI coding tools found yet.");
  });
});

describe("AJ-E1524 the welcome", () => {
  it("says what the probe found, with install links when nothing is installed", async () => {
    w().__AIJ_PROVIDER_INSTALLS__ = [row("one", "Tool One", "not_installed"), row("two", "Tool Two", "not_installed")];
    render(<Welcome onFindAccounts={() => {}} onFreshAccount={() => {}} />);
    await waitFor(() => expect(screen.getByTestId("welcome-found").textContent).toContain("No AI coding tools found yet."));
    expect(screen.getByRole("link", { name: "Install Tool One →" }).getAttribute("href")).toBe("https://one.example/install");
    expect(screen.getByRole("link", { name: "Install Tool Two →" })).toBeTruthy();
  });

  it("names the tools found, without install links", async () => {
    w().__AIJ_PROVIDER_INSTALLS__ = [row("one", "Tool One", "installed"), row("two", "Tool Two", "not_installed")];
    render(<Welcome onFindAccounts={() => {}} onFreshAccount={() => {}} />);
    await waitFor(() => expect(screen.getByTestId("welcome-found").textContent).toContain("We found Tool One on this Mac."));
    expect(screen.queryByRole("link")).toBeNull();
  });

  it("each action opens its flow; one h1; the mark is decorative; How it works is a disclosure", () => {
    const onFindAccounts = vi.fn();
    const onFreshAccount = vi.fn();
    const { container } = render(<Welcome onFindAccounts={onFindAccounts} onFreshAccount={onFreshAccount} />);
    fireEvent.click(screen.getByRole("button", { name: "Find my accounts" }));
    fireEvent.click(screen.getByRole("button", { name: "Add a fresh account" }));
    expect(onFindAccounts).toHaveBeenCalledOnce();
    expect(onFreshAccount).toHaveBeenCalledOnce();
    expect(container.querySelectorAll("h1")).toHaveLength(1);
    expect(container.querySelector(".welcome-rise")?.getAttribute("aria-hidden")).toBe("true");
    const how = screen.getByTestId("welcome-how") as HTMLDetailsElement;
    expect(how.tagName).toBe("DETAILS");
    expect(how.open).toBe(false);
    expect(how.querySelectorAll("ol > li")).toHaveLength(3);
    expect(container.querySelectorAll("ul > li")).toHaveLength(4);
    expect(screen.getByTestId("welcome-trust").textContent).toContain("never stores your passwords or keys");
    // The reading order: the headline, then the found line, then the actions.
    const order = [...container.querySelectorAll("h1, [data-testid=welcome-found], [data-testid=welcome-find]")].map(
      (node) => node.tagName === "H1" ? "h1" : (node as HTMLElement).dataset.testid,
    );
    expect(order).toEqual(["h1", "welcome-found", "welcome-find"]);
  });

  it("Home shows it with zero accounts and not with one", () => {
    const props = { sessions: [], projects: [], now: Date.now(), onOpenSession: () => {}, onNewSession: () => {}, onAddAccounts: () => {} };
    const { rerender } = render(<HomeScreen {...props} accounts={[]} />);
    expect(screen.getByTestId("home-empty-accounts")).toBeTruthy();
    const one = { id: "acc-1", provider: "claude-code", label: "Work", login_state: "logged_in" } as unknown as AccountInfo;
    rerender(<HomeScreen {...props} accounts={[one]} />);
    expect(screen.queryByTestId("home-empty-accounts")).toBeNull();
  });

  it("Home's Add a fresh account opens the fresh-account flow", () => {
    const onAddAccounts = vi.fn();
    const onFreshAccount = vi.fn();
    render(
      <HomeScreen sessions={[]} projects={[]} accounts={[]} now={Date.now()} onOpenSession={() => {}} onNewSession={() => {}} onAddAccounts={onAddAccounts} onFreshAccount={onFreshAccount} />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Add a fresh account" }));
    expect(onFreshAccount).toHaveBeenCalledOnce();
    expect(onAddAccounts).not.toHaveBeenCalled();
  });

  it("Usage's and Fleet's pointer goes back to Home", () => {
    const onOpenHome = vi.fn();
    render(<WelcomePointer onOpenHome={onOpenHome} />);
    fireEvent.click(screen.getByRole("button", { name: /Home shows what AI Juggler does/ }));
    expect(onOpenHome).toHaveBeenCalledOnce();
  });
});
