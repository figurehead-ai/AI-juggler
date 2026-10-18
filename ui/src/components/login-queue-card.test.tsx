/**
 * AJ-E308 vitest — the ONE shared login-queue card: per-state copy
 * (all seven), the auto-detect poll, the aria-live status line, the
 * inline (never-alert) spawn failure, and skip.
 */
import { fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { LoginQueueCard, loginStateCopy } from "./LoginQueueCard";
import * as bridgeModule from "../lib/bridge";

beforeEach(() => {
  vi.spyOn(bridgeModule, "accountLoginCommand").mockResolvedValue("claude login");
  vi.spyOn(bridgeModule, "accountLoginState").mockResolvedValue("logged_out");
  vi.spyOn(bridgeModule, "terminalSpawnLogin").mockResolvedValue({ id: "t1" } as never);
});
afterEach(() => vi.restoreAllMocks());

describe("AJ-E308 loginStateCopy — the seven states", () => {
  it("each state has distinct, human copy; unknown never says logged out", () => {
    expect(loginStateCopy("signed_in").headline).toBe("Signed in");
    expect(loginStateCopy("partial").headline).toBe("Sign-in didn't finish");
    expect(loginStateCopy("partial").reopenLabel).toBe("Reopen sign-in");
    expect(loginStateCopy("expired").headline).toBe("Sign-in expired");
    expect(loginStateCopy("sso_pending").detail).toContain("browser/IdP window");
    expect(loginStateCopy("api_key").headline).toBe("API key account");
    expect(loginStateCopy("logged_out").headline).toBe("Waiting…");
    expect(loginStateCopy("unknown").detail).toContain("sign in when online");
  });
});

describe("AJ-E308 LoginQueueCard", () => {
  it("renders the aria-live status line with the vendor-frame copy and numbered steps", async () => {
    const { container } = render(
      <LoginQueueCard accountId="a1" label="Work" providerName="Claude Code" initialState="logged_out" />,
    );
    const status = await waitFor(() => {
      const found = container.querySelector("[data-testid='login-status']")!;
      expect(found).not.toBeNull();
      return found;
    });
    expect(status.getAttribute("aria-live")).toBe("polite");
    expect(status.textContent).toContain("○ Waiting…");
    const text = container.textContent ?? "";
    expect(text).toContain("This is Claude Code's own sign-in. AI Juggler never sees your credentials.");
    expect(text).toContain("1. Sign in → 2. follow the vendor prompts → 3. return");
  });

  it("the 2s poll auto-detects sign-in — the status flips without any click", async () => {
    vi.mocked(bridgeModule.accountLoginState).mockResolvedValue("signed_in");
    const { container } = render(
      <LoginQueueCard accountId="a1" label="Work" providerName="Claude Code" initialState="logged_out" />,
    );
    await waitFor(() =>
      expect(container.querySelector("[data-testid='login-status']")!.textContent).toContain(
        "● Signed in ✓ auto-detected",
      ),
    );
  });

  it("a spawn failure renders inline with role=alert — never window.alert, never unhandled", async () => {
    const alertSpy = vi.spyOn(window, "alert").mockImplementation(() => {});
    vi.spyOn(bridgeModule, "terminalSpawnLogin").mockRejectedValue({
      code: "io",
      retryable: true,
      user_message: "Terminal could not open.",
    });
    const { container } = render(
      <LoginQueueCard accountId="a1" label="Work" providerName="Claude Code" initialState="logged_out" />,
    );
    await waitFor(() =>
      expect(container.querySelector("button")!.textContent).toContain("Sign in"),
    );
    fireEvent.click(
      Array.from(container.querySelectorAll("button")).find((b) => b.textContent === "Sign in")!,
    );
    const inline = await waitFor(() => {
      const found = container.querySelector("[role='alert']");
      expect(found).not.toBeNull();
      return found!;
    });
    expect(inline.textContent).toContain("Terminal could not open.");
    expect(alertSpy).not.toHaveBeenCalled();
  });

  it("api_key accounts show the badge and NO sign-in buttons (rings never apply)", async () => {
    vi.mocked(bridgeModule.accountLoginState).mockResolvedValue("api_key");
    const { container } = render(
      <LoginQueueCard accountId="a1" label="Bot" providerName="Claude Code" initialState="api_key" />,
    );
    await waitFor(() =>
      expect(container.querySelector("[data-testid='login-queue-card']")!.getAttribute("data-state")).toBe(
        "api_key",
      ),
    );
    expect(container.textContent).toContain("API key account");
    expect(container.textContent).not.toContain("Copy command");
  });

  it("partial offers [Reopen sign-in]; skip advances the queue", async () => {
    vi.mocked(bridgeModule.accountLoginState).mockResolvedValue("partial");
    const onSkip = vi.fn();
    const { container } = render(
      <LoginQueueCard accountId="a1" label="Work" providerName="Claude Code" initialState="partial" onSkip={onSkip} />,
    );
    await waitFor(() =>
      expect(container.querySelector("[data-testid='login-status']")!.textContent).toContain(
        "Sign-in didn't finish",
      ),
    );
    expect(
      Array.from(container.querySelectorAll("button")).some((b) => b.textContent === "Reopen sign-in"),
    ).toBe(true);
    fireEvent.click(container.querySelector("[data-testid='signin-skip']")!);
    expect(onSkip).toHaveBeenCalled();
  });
});
