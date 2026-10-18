/**
 * AJ-E301 vitest — the parameterised wizard's Step-0 install gate:
 * one row per Provider from the probe, the not-installed gate (the
 * Continue button never advances, install help shows the vendor link
 * and "Continue without it"), the installed path (Continue advances
 * to the scan), the coded-error render (user_message + retry, never
 * a raw OS string), and the dialog's role/aria contract.
 */
import { act, fireEvent, render, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { OnboardingWizard } from "../screens/OnboardingWizard";
import * as bridgeModule from "../lib/bridge";
import type { ProviderInstallData, ScanReportData } from "../lib/bridge";

const w = () => window as unknown as Record<string, unknown>;

/** A user who already chose a pack policy (an earlier run / Accounts)
 * — the wizard asks nothing. Tests of the first-run question delete
 * this seam to start from the honest "unset". */
const POLICY_CHOSEN = {
  global: "strip",
  override: null,
  effective: "strip",
  source: "global",
  line: "Redaction for claude-code: Strip — replace keys with ••• (your global choice)",
};

beforeEach(() => {
  delete w().__AIJ_PROVIDER_INSTALLS__;
  delete w().__AIJ_ENVIRONMENT_SCAN__;
  delete w().__AIJ_SCAN_REPORT__;
  delete w().__AIJ_FUNNEL_EVENTS__;
  delete w().__AIJ_REDACTION_SET_GLOBAL__;
  delete w().__AIJ_RENEWAL_SET__;
  w().__AIJ_REDACTION_POLICY__ = { ...POLICY_CHOSEN };
});

const DENIED_REPORT: ScanReportData = {
  installs: [],
  probes: [
    { id: "s1-config-dirs", label: "Config directories", health: "ok", reason: null, detail: "0 candidate setup(s)" },
    { id: "s2-env-overrides", label: "Environment overrides", health: "ok", reason: null, detail: "none set" },
    { id: "s3-manager-t3", label: "t3 profiles", health: "ok", reason: null, detail: "not present" },
    { id: "s4-manager-cc-switch", label: "cc-switch database", health: "denied", reason: "cc-switch database is locked or unreadable", detail: "existence only" },
    { id: "s5-shell-aliases", label: "Shell aliases", health: "partial", reason: "1 assignment points at a directory that no longer exists", detail: "1 assignment(s) (1 dangling)" },
    { id: "s6-installed-clis", label: "Installed CLIs", health: "ok", reason: null, detail: "claude-code: installed" },
    { id: "s7-login-metadata", label: "Login metadata", health: "ok", reason: null, detail: "0 of 0 signed in" },
    { id: "s8-usage-recency", label: "Usage recency", health: "ok", reason: null, detail: "0 setup(s)" },
    { id: "s9-projects", label: "Projects", health: "ok", reason: null, detail: "0 project name(s)" },
    { id: "s10-desktop", label: "Desktop apps", health: "ok", reason: null, detail: "none present" },
  ],
  seeds: [],
  desktop_apps: [],
  cancelled: false,
};

const INSTALLED: ProviderInstallData[] = [
  {
    provider: "claude-code",
    display_name: "Claude Code",
    install_url: "https://claude.com/download",
    status: "installed",
    version: "2.1.296",
    binary: "/usr/local/bin/claude",
    notes: [],
  },
];
const MISSING: ProviderInstallData[] = [
  {
    provider: "claude-code",
    display_name: "Claude Code",
    install_url: "https://claude.com/download",
    status: "not_installed",
    version: null,
    binary: null,
    notes: [],
  },
  {
    provider: "codex",
    display_name: "Codex",
    install_url: "https://developers.openai.com/codex",
    status: "installed",
    version: "0.9.4",
    binary: "/opt/homebrew/bin/codex",
    notes: [],
  },
];

const mount = (extra?: { onClose?: () => void }) =>
  render(
    <OnboardingWizard
      provider="claude-code"
      onClose={extra?.onClose ?? (() => {})}
      onAccountCreated={() => {}}
      openLoginTerminal={() => {}}
    />,
  );

describe("AJ-E301 Step-0 install gate", () => {
  it("renders one row per Provider with its honest status", async () => {
    w().__AIJ_PROVIDER_INSTALLS__ = MISSING;
    const { container } = mount();
    await waitFor(() =>
      expect(container.querySelectorAll('[data-testid="provider-row"]').length).toBe(2),
    );
    const claudeRow = container.querySelector(
      "[data-testid='provider-row'][data-status='not_installed']",
    )!;
    expect(claudeRow.textContent).toContain("Not installed");
    const codexRow = container.querySelector(
      "[data-testid='provider-row'][data-status='installed']",
    )!;
    expect(codexRow.textContent).toContain("Installed v0.9.4");
  });

  it("a missing binary gates: Continue stays disabled, install help offers the vendor page + Continue without it", async () => {
    w().__AIJ_PROVIDER_INSTALLS__ = MISSING;
    const { container } = mount();
    await waitFor(() =>
      expect(container.querySelector("[data-testid='install-continue']")).not.toBeNull(),
    );
    const continueButton = container.querySelector(
      "[data-testid='install-continue']",
    ) as HTMLButtonElement;
    expect(continueButton.disabled).toBe(true);
    // The row click opens the install help — never advances.
    fireEvent.click(container.querySelector("[data-testid='provider-row'] button")!);
    const help = container.querySelector("[data-testid='install-help']")!;
    expect(help.textContent).toContain("Claude Code isn't installed on this Mac");
    const vendorLink = help.querySelector("a[href='https://claude.com/download']")!;
    expect(vendorLink.textContent).toContain("Install Claude Code →");
    expect(help.textContent).toContain("Continue without it");
    // Identity never became reachable.
    expect(container.querySelector("input[placeholder*='Work']")).toBeNull();
  });

  it("an installed Provider advances to the scan step", async () => {
    w().__AIJ_PROVIDER_INSTALLS__ = INSTALLED;
    const { container } = mount();
    await waitFor(() =>
      expect(container.querySelector("[data-testid='install-continue']")).not.toBeNull(),
    );
    await act(async () => {
      fireEvent.click(container.querySelector("[data-testid='install-continue']")!);
    });
    await waitFor(() =>
      expect(container.textContent).toContain("Let's look at what Claude Code already has"),
    );
  });

  it("the dialog is a role=dialog with an aria-label; Esc closes when idle", async () => {
    w().__AIJ_PROVIDER_INSTALLS__ = INSTALLED;
    let closed = false;
    const { container } = mount({ onClose: () => (closed = true) });
    const dialog = container.querySelector("[data-testid='onboarding-wizard']")!;
    expect(dialog.getAttribute("role")).toBe("dialog");
    expect(dialog.getAttribute("aria-modal")).toBe("true");
    await waitFor(() =>
      expect(container.querySelector("[data-testid='install-continue']")).not.toBeNull(),
    );
    // The label upgrades to the display name once the probe lands.
    expect(dialog.getAttribute("aria-label")).toContain("Claude Code");
    fireEvent.keyDown(window, { key: "Escape" });
    expect(closed).toBe(true);
  });

  it("coded errors render user_message with a retry — never a raw OS string", async () => {
    w().__AIJ_PROVIDER_INSTALLS__ = INSTALLED;
    // The bridge's createAccount rejects with a CODED error (the
    // backend wire shape) — the wizard must render user_message.
    const spy = vi.spyOn(bridgeModule, "createAccount").mockRejectedValue({
      code: "io",
      retryable: true,
      user_message: "Couldn't create the account's private folder. Nothing was half-made.",
    });
    const { container } = mount();
    await waitFor(() =>
      expect(container.querySelector("[data-testid='install-continue']")).not.toBeNull(),
    );
    await act(async () => {
      fireEvent.click(container.querySelector("[data-testid='install-continue']")!);
    });
    await waitFor(() =>
      expect(container.textContent).toContain("Let's look at what Claude Code already has"),
    );
    // Nothing to import (scan empty) → wait for the 10-tick progress
    // to finish, then continue with a fresh account.
    const fresh = await waitFor(() => {
      const button = Array.from(container.querySelectorAll("button")).find((candidate) =>
        candidate.textContent?.includes("fresh account"),
      );
      expect(button).toBeTruthy();
      return button!;
    });
    await act(async () => {
      fireEvent.click(fresh);
    });
    const input = await waitFor(() => {
      const field = container.querySelector(
        "input[placeholder*='Work']",
      ) as HTMLInputElement;
      expect(field).not.toBeNull();
      return field;
    });
    fireEvent.change(input, { target: { value: "Work" } });
    await act(async () => {
      fireEvent.click(
        Array.from(container.querySelectorAll("button")).find((button) =>
          button.textContent?.includes("Create account"),
        )!,
      );
    });
    await waitFor(() =>
      expect(container.querySelector("[data-testid='wizard-error']")).not.toBeNull(),
    );
    const errorBox = container.querySelector("[data-testid='wizard-error']")!;
    expect(errorBox.textContent).toContain("Couldn't create the account's private folder");
    expect(errorBox.textContent).toContain("Try again");
    spy.mockRestore();
  });
});

describe("AJ-E302 the 10-signal scan in the wizard", () => {
  it("streams progress as 'n of 10' with aria-live, then the Details rows", async () => {
    w().__AIJ_PROVIDER_INSTALLS__ = INSTALLED;
    const { container } = mount();
    await waitFor(() =>
      expect(container.querySelector("[data-testid='install-continue']")).not.toBeNull(),
    );
    await act(async () => {
      fireEvent.click(container.querySelector("[data-testid='install-continue']")!);
    });
    const progressBox = await waitFor(() => {
      const box = container.querySelector("[data-testid='scan-progress']")!;
      expect(box).not.toBeNull();
      return box;
    });
    expect(progressBox.getAttribute("aria-live")).toBe("polite");
    expect(progressBox.textContent).toMatch(/of 10 checks/);
    await waitFor(() =>
      expect(container.querySelector("[data-testid='scan-details-toggle']")).not.toBeNull(),
    );
    fireEvent.click(container.querySelector("[data-testid='scan-details-toggle']")!);
    await waitFor(() =>
      expect(container.querySelectorAll("[data-testid='scan-probe-row']").length).toBe(10),
    );
    const details = container.querySelector("[data-testid='scan-details']")!;
    expect(details.textContent).toContain("checked");
    expect(details.textContent).toContain("Config directories");
  });

  it("a denied probe with an empty result NEVER reads 'No existing setups found'", async () => {
    w().__AIJ_PROVIDER_INSTALLS__ = INSTALLED;
    w().__AIJ_SCAN_REPORT__ = DENIED_REPORT;
    const { container } = mount();
    await waitFor(() =>
      expect(container.querySelector("[data-testid='install-continue']")).not.toBeNull(),
    );
    await act(async () => {
      fireEvent.click(container.querySelector("[data-testid='install-continue']")!);
    });
    await waitFor(() =>
      expect(container.querySelector("[data-testid='scan-denied']")).not.toBeNull(),
    );
    expect(container.querySelector("[data-testid='scan-denied']")!.textContent).toContain(
      "We couldn't check 1 place",
    );
    expect(container.textContent).not.toContain("No existing Claude Code setups found");
    fireEvent.click(container.querySelector("[data-testid='scan-details-toggle']")!);
    await waitFor(() =>
      expect(container.querySelector("[data-testid='scan-details']")!.textContent).toContain(
        "cc-switch database",
      ),
    );
  });
});

describe("AJ-E303 candidate cards — inference, inline edit, ignore", () => {
  const CARDS_REPORT: ScanReportData = {
    installs: [
      {
        harness: "claude-code",
        path: "/Users/you/.claude-work",
        origin: "home-pattern",
        login: "logged_in",
        bytes: 9_000_000,
        last_used_secs: Math.floor(Date.now() / 1000) - 86_400,
        managed: false,
        notes: ["note one", "note two — never truncated"],
        inference: {
          purpose: "Work",
          purpose_confidence: "likely",
          login: "signed-in",
          login_confidence: "found",
          activity: "active",
          activity_confidence: "found",
          activity_words: "Active this week",
          keychain_bound: true,
        },
        signals: ["home-pattern", "CLAUDE_CONFIG_DIR in ~/.zshrc:3"],
      },
      {
        harness: "claude-code",
        path: "/Users/you/.claude-managed",
        origin: "home-pattern",
        login: "logged_in",
        bytes: 1,
        last_used_secs: Math.floor(Date.now() / 1000) - 91 * 86_400,
        managed: true,
        notes: [],
        inference: {
          purpose: "Managed",
          purpose_confidence: "guessed",
          login: "signed-in",
          login_confidence: "found",
          activity: "archive",
          activity_confidence: "guessed",
          activity_words: "Archive candidate",
          keychain_bound: false,
        },
        signals: ["home-pattern"],
      },
    ],
    probes: [],
    seeds: [],
    desktop_apps: [],
    cancelled: false,
  };

  const openCards = async () => {
    w().__AIJ_PROVIDER_INSTALLS__ = INSTALLED;
    w().__AIJ_SCAN_REPORT__ = CARDS_REPORT;
    const mounted = mount();
    await waitFor(() =>
      expect(mounted.container.querySelector("[data-testid='install-continue']")).not.toBeNull(),
    );
    await act(async () => {
      fireEvent.click(mounted.container.querySelector("[data-testid='install-continue']")!);
    });
    await waitFor(() =>
      expect(
        mounted.container.querySelectorAll("[data-testid='candidate-card']").length,
      ).toBe(2),
    );
    return mounted;
  };

  it("renders the inference line with confidence WORDS, merged signals, full notes, and the keychain caveat", async () => {
    const { container } = await openCards();
    const card = container.querySelector("[data-testid='candidate-card']")!;
    expect(card.textContent).toContain("Likely: Work");
    expect(card.textContent).toContain("Active this week");
    expect(card.textContent).toContain("2 signals merged");
    expect(card.textContent).toContain("Signed in");
    expect(card.textContent).toContain("note two — never truncated");
    expect(card.querySelector("[data-testid='keychain-caveat']")!.textContent).toContain(
      "you'll sign in once more after import",
    );
    expect(card.textContent).not.toMatch(/\d+(\.\d+)?%/);
  });

  it("already-managed renders disabled with 'Already in AI Juggler' inline", async () => {
    const { container } = await openCards();
    const managedCard = container.querySelector(
      "[data-testid='candidate-card'][data-managed='true']",
    )!;
    const importButton = managedCard.querySelector(
      "button[data-testid='import-as']",
    ) as HTMLButtonElement;
    expect(importButton.disabled).toBe(true);
    expect(importButton.textContent).toContain("Already in AI Juggler");
  });

  it("inline edit changes the import label WITHOUT re-running the scan, and Max 5× is offered", async () => {
    const scanSpy = vi.spyOn(bridgeModule, "environmentScanFull");
    const { container } = await openCards();
    const callsAfterOpen = scanSpy.mock.calls.length;
    const card = container.querySelector("[data-testid='candidate-card']")!;
    fireEvent.click(
      Array.from(card.querySelectorAll("button")).find((button) =>
        button.textContent === "Edit",
      )!,
    );
    const input = await waitFor(() => {
      const field = container.querySelector(
        "[data-testid='inline-edit-purpose']",
      ) as HTMLInputElement;
      expect(field).not.toBeNull();
      return field;
    });
    fireEvent.change(input, { target: { value: "Consulting" } });
    // Max 5× is in the inline plan list (Req.4).
    const editBox = container.querySelector("[data-testid='inline-edit']")!;
    expect(editBox.textContent).toContain("Max 5×");
    expect(editBox.textContent).toContain("Not sure yet");
    // The import button follows the edit…
    await waitFor(() =>
      expect(card.querySelector("[data-testid='import-as']")!.textContent).toContain(
        "Import as “Consulting”",
      ),
    );
    // …and NO scan re-ran (Req.5).
    expect(scanSpy.mock.calls.length).toBe(callsAfterOpen);
    scanSpy.mockRestore();
  });

  it("Ignore is reversible via the Ignored (n) drawer", async () => {
    const { container } = await openCards();
    const card = container.querySelector("[data-testid='candidate-card']")!;
    fireEvent.click(
      Array.from(card.querySelectorAll("button")).find((button) => button.textContent === "Ignore")!,
    );
    await waitFor(() =>
      expect(container.querySelectorAll("[data-testid='candidate-card']").length).toBe(1),
    );
    fireEvent.click(container.querySelector("[data-testid='ignored-toggle']")!);
    const drawer = container.querySelector("[data-testid='ignored-list']")!;
    expect(drawer.textContent).toContain(".claude-work");
    fireEvent.click(
      Array.from(drawer.querySelectorAll("button")).find((button) =>
        button.textContent === "Unignore",
      )!,
    );
    await waitFor(() =>
      expect(container.querySelectorAll("[data-testid='candidate-card']").length).toBe(2),
    );
  });
});

describe("AJ-E304 the set-level proposal", () => {
  const openAtDetect = async () => {
    w().__AIJ_PROVIDER_INSTALLS__ = INSTALLED;
    w().__AIJ_RECOMMENDATION__ = {
      items: [
        { provider: "claude-code", label: "Work", action: "import", rationale: "Signed in — import keeps history", default_on: true, path: "/Users/you/.claude-work" },
        { provider: "claude-code", label: "Personal", action: "import", rationale: "Signed in — import keeps history", default_on: true, path: "/Users/you/.claude-personal" },
        { provider: "claude-code", label: "Side", action: "import", rationale: "Active — you'll sign in after import", default_on: true, path: "/Users/you/.claude-side" },
        { provider: "claude-code", label: "Old", action: "import", rationale: "Dormant — include when you want it", default_on: false, path: "/Users/you/.claude-old" },
      ],
      summary: "Your juggling set: Claude ×3 (Work — signed in; Personal — signed in; Side — needs sign-in).",
      start_here: "-users-dev-api",
    };
    const mounted = mount();
    await waitFor(() =>
      expect(mounted.container.querySelector("[data-testid='install-continue']")).not.toBeNull(),
    );
    await act(async () => {
      fireEvent.click(mounted.container.querySelector("[data-testid='install-continue']")!);
    });
    await waitFor(() =>
      expect(mounted.container.querySelector("[data-testid='proposal-teaser']")).not.toBeNull(),
    );
    return mounted;
  };

  it("shows the one-sentence proposal with per-item rationales after Customise", async () => {
    const { container } = await openAtDetect();
    expect(container.querySelector("[data-testid='proposal-summary']")!.textContent).toContain(
      "Your juggling set: Claude ×3",
    );
    fireEvent.click(container.querySelector("[data-testid='proposal-customise']")!);
    await waitFor(() =>
      expect(container.querySelectorAll("[data-testid='proposal-item']").length).toBe(4),
    );
    const quiet = container.querySelector(
      "[data-testid='proposal-item'][data-quiet='true']",
    )!;
    expect(quiet.textContent).toContain("Dormant — include when you want it");
    // The dormant toggle starts OFF.
    const toggles = Array.from(
      container.querySelectorAll("[data-testid='proposal-toggle']"),
    ) as HTMLInputElement[];
    expect(toggles[3]!.checked).toBe(false);
    expect(toggles[0]!.checked).toBe(true);
  });

  it("accept executes sequentially; a failed item degrades ITSELF only (3 done, 1 failed, 0 aborts)", async () => {
    const { container } = await openAtDetect();
    const importSpy = vi
      .spyOn(bridgeModule, "accountImport")
      .mockImplementation(async (_harness, path: string) => {
        if (path.includes("personal")) {
          throw { code: "io", retryable: true, user_message: "Disk full — this one failed." };
        }
        return { account: { id: "a", provider: "claude-code", label: "x", plan: null, login_state: "logged_out", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false }, files_copied: 3, bytes_copied: 3, skipped: [], login: "logged_out", login_note: null } as never;
      });
    fireEvent.click(container.querySelector("[data-testid='proposal-customise']")!);
    await waitFor(() =>
      expect(container.querySelector("[data-testid='accept-setup']")).not.toBeNull(),
    );
    await act(async () => {
      fireEvent.click(container.querySelector("[data-testid='accept-setup']")!);
    });
    // AJ-E305: accounts that still need the vendor login queue for
    // the sign-ins step — the two successful imports (login
    // logged_out) queue; the failed one contributes nothing.
    await waitFor(() =>
      expect(container.querySelector("[data-testid='signin-queue']")).not.toBeNull(),
    );
    expect(container.querySelector("[data-testid='signin-queue']")!.textContent).toContain(
      "1 of 2",
    );
    // Skip never blocks the Summary (AJ-E305 Req.3).
    await act(async () => {
      fireEvent.click(container.querySelector("[data-testid='signin-skip']")!);
    });
    await act(async () => {
      fireEvent.click(container.querySelector("[data-testid='signin-skip']")!);
    });
    await waitFor(() =>
      expect(container.querySelector("[data-testid='set-summary']")).not.toBeNull(),
    );
    const items = Array.from(
      container.querySelectorAll("[data-testid='set-summary-item']"),
    );
    // Only the default-ON set runs (the dormant toggle stays off).
    expect(items.length).toBe(3);
    expect(items.filter((item) => item.getAttribute("data-state") === "done").length).toBe(2);
    const failed = items.find((item) => item.getAttribute("data-state") === "failed")!;
    expect(failed.textContent).toContain("Personal");
    expect(failed.textContent).toContain("Disk full — this one failed.");
    // Start here rides the summary.
    expect(container.querySelector("[data-testid='start-here']")!.textContent).toContain(
      "-users-dev-api",
    );
    importSpy.mockRestore();
  });

  it("nothing executes before a click: no import calls from the proposal alone", async () => {
    const importSpy = vi.spyOn(bridgeModule, "accountImport");
    await openAtDetect();
    expect(importSpy).not.toHaveBeenCalled();
    importSpy.mockRestore();
  });
});

describe("AJ-E305 flow (a) — first install in ≤5 primary clicks, 0 typed fields", () => {
  const mountWith = (extra: {
    onClose?: () => void;
    onLaunchFirstSession?: (startHere: string | null) => void;
    onAddAnother?: () => void;
  }) =>
    render(
      <OnboardingWizard
        provider="claude-code"
        onClose={extra.onClose ?? (() => {})}
        onAccountCreated={() => {}}
        openLoginTerminal={() => {}}
        onLaunchFirstSession={extra.onLaunchFirstSession}
        onAddAnother={extra.onAddAnother}
      />,
    );

  const openProposal = async (extra: {
    onClose?: () => void;
    onLaunchFirstSession?: (startHere: string | null) => void;
    onAddAnother?: () => void;
  } = {}) => {
    w().__AIJ_PROVIDER_INSTALLS__ = INSTALLED;
    w().__AIJ_RECOMMENDATION__ = {
      items: [
        { provider: "claude-code", label: "Work", action: "import", rationale: "Signed in — import keeps history", default_on: true, path: "/Users/you/.claude-work" },
        { provider: "claude-code", label: "Side", action: "import", rationale: "Active — you'll sign in after import", default_on: true, path: "/Users/you/.claude-side" },
      ],
      summary: "Your juggling set: Claude ×2 (Work — signed in; Side — needs sign-in).",
      start_here: "-users-dev-api",
    };
    const mounted = mountWith(extra);
    await waitFor(() =>
      expect(mounted.container.querySelector("[data-testid='install-continue']")).not.toBeNull(),
    );
    await act(async () => {
      fireEvent.click(mounted.container.querySelector("[data-testid='install-continue']")!);
    });
    await waitFor(() =>
      expect(mounted.container.querySelector("[data-testid='proposal-teaser']")).not.toBeNull(),
    );
    return mounted;
  };

  it("the primary path is 4 primary-button clicks + 1 policy answer (≤ 5), 0 typed fields, and ends on the three Summary exits", async () => {
    // A TRUE first run: no pack policy chosen yet (AJ-E309).
    delete w().__AIJ_REDACTION_POLICY__;
    const launch = vi.fn();
    const close = vi.fn();
    const addAnother = vi.fn();
    const { container } = await openProposal({
      onClose: close,
      onLaunchFirstSession: launch,
      onAddAnother: addAnother,
    });
    // Click 1 (primary): Use recommended setup →.
    fireEvent.click(container.querySelector("[data-testid='use-recommended']")!);
    await waitFor(() =>
      expect(container.querySelector("[data-testid='accept-setup']")).not.toBeNull(),
    );
    // AJ-E309: the policy question rides the Confirm (no extra step);
    // nothing is pre-selected and Accept waits for an answer.
    await waitFor(() =>
      expect(container.querySelector("[data-testid='policy-panel']")).not.toBeNull(),
    );
    const radios = Array.from(
      container.querySelectorAll<HTMLInputElement>("[data-testid='policy-panel'] input[type='radio']"),
    );
    expect(radios.length).toBe(4);
    expect(radios.some((radio) => radio.checked)).toBe(false);
    expect(container.querySelector<HTMLButtonElement>("[data-testid='accept-setup']")!.disabled).toBe(true);
    // Click 2 (the policy answer — a deliberate choice, no ⌘↩).
    fireEvent.click(radios.find((radio) => radio.value === "strip")!);
    expect(container.querySelector<HTMLButtonElement>("[data-testid='accept-setup']")!.disabled).toBe(false);
    // No typing anywhere on the path: the proposal carries labels;
    // nothing requires an input.
    // Click 3 (primary): Accept setup →.
    await act(async () => {
      fireEvent.click(container.querySelector("[data-testid='accept-setup']")!);
    });
    // The two logged_out imports queue for sign-ins (AJ-E305 Req.3).
    await waitFor(() =>
      expect(container.querySelector("[data-testid='signin-queue']")).not.toBeNull(),
    );
    expect(container.querySelector("[data-testid='signin-queue']")!.textContent).toContain(
      "1 of 2",
    );
    // [Skip for now] per account — secondary, never blocks Summary.
    await act(async () => {
      fireEvent.click(container.querySelector("[data-testid='signin-skip']")!);
    });
    await act(async () => {
      fireEvent.click(container.querySelector("[data-testid='signin-skip']")!);
    });
    await waitFor(() =>
      expect(container.querySelector("[data-testid='summary-exits']")).not.toBeNull(),
    );
    // The Summary restates the choice (Nielsen H3), written ONCE on
    // the accept click.
    expect(container.querySelector("[data-testid='policy-restatement']")!.textContent).toBe(
      "Packs: strip secrets (all Providers)",
    );
    expect(w().__AIJ_REDACTION_SET_GLOBAL__).toEqual(["strip"]);
    // Click 4 (primary): Launch first session → carries Start here.
    fireEvent.click(container.querySelector("[data-testid='launch-first-session']")!);
    expect(launch).toHaveBeenCalledWith("-users-dev-api");
    expect(close).toHaveBeenCalled();
    expect(addAnother).not.toHaveBeenCalled();
    // The remaining exits exist and route.
    expect(container.querySelector("[data-testid='add-another-account']")).not.toBeNull();
  });

  it("the zero-found state collapses to a provider picker — installed first, others gated — with NO dashed dead-end", async () => {
    w().__AIJ_PROVIDER_INSTALLS__ = [
      ...INSTALLED,
      {
        provider: "codex",
        display_name: "Codex",
        install_url: "https://developers.openai.com/codex",
        status: "not_installed",
        version: null,
        binary: null,
        notes: [],
      },
    ];
    w().__AIJ_SCAN_REPORT__ = { ...DENIED_REPORT, probes: DENIED_REPORT.probes.map((p) => ({ ...p, health: "ok" as const, reason: null })) };
    w().__AIJ_RECOMMENDATION__ = { items: [], summary: "", start_here: null };
    const { container } = mountWith({});
    await waitFor(() =>
      expect(container.querySelector("[data-testid='install-continue']")).not.toBeNull(),
    );
    await act(async () => {
      fireEvent.click(container.querySelector("[data-testid='install-continue']")!);
    });
    const zero = await waitFor(() => {
      const found = container.querySelector("[data-testid='zero-found']");
      expect(found).not.toBeNull();
      return found!;
    });
    expect(zero.textContent).toContain("Create your first account");
    const rows = Array.from(
      container.querySelectorAll("[data-testid='zero-found-provider']"),
    ) as HTMLButtonElement[];
    expect(rows.length).toBe(2);
    // Installed sorts first and is enabled; the missing one is gated
    // with its install link (Req.2).
    expect(rows[0]!.getAttribute("data-installed")).toBe("true");
    expect(rows[0]!.disabled).toBe(false);
    expect(rows[1]!.getAttribute("data-installed")).toBe("false");
    expect(rows[1]!.disabled).toBe(true);
    expect(zero.querySelector("a")!.getAttribute("href")).toBe(
      "https://developers.openai.com/codex",
    );
    // Choosing an installed provider lands on Confirm identity with
    // the label PRE-FILLED — zero typed fields.
    fireEvent.click(rows[0]!);
    const input = await waitFor(() => {
      const field = container.querySelector("input[placeholder^='e.g. Work']") as HTMLInputElement;
      expect(field).not.toBeNull();
      return field!;
    });
    expect(input.value).toBe("Main");
  });

  it("wizard state survives an accidental close on Confirm — label/plan restore on reopen", async () => {
    w().__AIJ_PROVIDER_INSTALLS__ = INSTALLED;
    w().__AIJ_SCAN_REPORT__ = { ...DENIED_REPORT, probes: DENIED_REPORT.probes.map((p) => ({ ...p, health: "ok" as const, reason: null })) };
    w().__AIJ_RECOMMENDATION__ = { items: [], summary: "", start_here: null };
    const first = mountWith({});
    await waitFor(() =>
      expect(first.container.querySelector("[data-testid='install-continue']")).not.toBeNull(),
    );
    await act(async () => {
      fireEvent.click(first.container.querySelector("[data-testid='install-continue']")!);
    });
    await waitFor(() =>
      expect(first.container.querySelector("[data-testid='zero-found']")).not.toBeNull(),
    );
    fireEvent.click(
      first.container.querySelectorAll("[data-testid='zero-found-provider']")[0]!,
    );
    const input = await waitFor(() => {
      const field = first.container.querySelector(
        "input[placeholder^='e.g. Work']",
      ) as HTMLInputElement;
      expect(field).not.toBeNull();
      return field!;
    });
    fireEvent.change(input, { target: { value: "Side project" } });
    const planChip = Array.from(
      first.container.querySelectorAll("button"),
    ).find((button) => button.textContent?.includes("Max 5×"))!;
    fireEvent.click(planChip);
    // Accidental close (nothing in flight — no confirm dialog).
    fireEvent.click(first.container.querySelector("[aria-label='Close setup']")!);
    expect(first.container.querySelector("[data-testid='wizard-close-confirm']")).toBeNull();
    // Reopen: label/plan restored, no credential-adjacent field stored.
    const second = mountWith({});
    await waitFor(() =>
      expect(second.container.querySelector("[data-testid='install-continue']")).not.toBeNull(),
    );
    await act(async () => {
      fireEvent.click(second.container.querySelector("[data-testid='install-continue']")!);
    });
    await waitFor(() =>
      expect(second.container.querySelector("[data-testid='zero-found']")).not.toBeNull(),
    );
    fireEvent.click(
      second.container.querySelectorAll("[data-testid='zero-found-provider']")[0]!,
    );
    const restored = await waitFor(() => {
      const field = second.container.querySelector(
        "input[placeholder^='e.g. Work']",
      ) as HTMLInputElement;
      expect(field).not.toBeNull();
      return field!;
    });
    expect(restored.value).toBe("Side project");
    const stored = window.sessionStorage.getItem("aij.wizard.state.claude-code");
    expect(stored).toContain("Side project");
    expect(stored).not.toContain("path");
    expect(stored).not.toContain("token");
    first.unmount();
    second.unmount();
    window.sessionStorage.clear();
  });
});

describe("AJ-E307 — Confirm-source, degraded import, duplicate block", () => {
  const w = () => window as unknown as Record<string, unknown>;

  const mountWizard = () =>
    render(
      <OnboardingWizard
        provider="claude-code"
        onClose={() => {}}
        onAccountCreated={() => {}}
        openLoginTerminal={() => {}}
      />,
    );

  const reachIdentityViaCandidate = async (mounted: ReturnType<typeof render>) => {
    w().__AIJ_PROVIDER_INSTALLS__ = INSTALLED;
    await waitFor(() =>
      expect(mounted.container.querySelector("[data-testid='install-continue']")).not.toBeNull(),
    );
    await act(async () => {
      fireEvent.click(mounted.container.querySelector("[data-testid='install-continue']")!);
    });
    await waitFor(() =>
      expect(mounted.container.querySelector("[data-testid='candidate-card']")).not.toBeNull(),
    );
    fireEvent.click(
      mounted.container.querySelector("[data-testid='import-as']")!,
    );
    await waitFor(() =>
      expect(mounted.container.querySelector("[data-testid='confirm-source']")).not.toBeNull(),
    );
  };

  it("Confirm-source shows path, origin badge, size, and the per-folder breakdown list", async () => {
    const mounted = mountWizard();
    await reachIdentityViaCandidate(mounted);
    const panel = mounted.container.querySelector("[data-testid='confirm-source']")!;
    // The origin badge renders (whatever the scan inferred).
    expect(panel.textContent).toContain("vendor-default");
    expect(panel.textContent).toContain("43.9 MB");
    const breakdown = panel.querySelector("[data-testid='source-breakdown']")!;
    expect(breakdown.querySelectorAll("li").length).toBeGreaterThan(0);
    expect(breakdown.textContent).toContain("projects/");
  });

  it("an oversize source offers ONLY the degraded pair — no raw ceiling string", async () => {
    w().__AIJ_SOURCE_INSPECT__ = {
      total_bytes: 3_300_000_000,
      file_count: 9000,
      folders: [
        { name: "projects/", bytes: 3_299_000_000 },
        { name: "settings.json", bytes: 60 },
      ],
      truncated: false,
      ceiling_bytes: 2_147_483_648,
      is_config: true,
      already_managed: false,
      login: "logged_in",
      keychain_bound: false,
    };
    const mounted = mountWizard();
    await reachIdentityViaCandidate(mounted);
    // The plain Import button is REPLACED by the offer pair.
    expect(mounted.container.querySelector("[data-testid='import-settings-only']")).not.toBeNull();
    expect(mounted.container.querySelector("[data-testid='add-fresh-instead']")).not.toBeNull();
    const body = mounted.container.textContent ?? "";
    expect(body).toContain("history in this folder is large");
    expect(body).not.toContain("2 GiB");
    expect(body).not.toContain("ceiling");
  });

  it("a non-config folder gets the plain reason + Pick a different folder", async () => {
    w().__AIJ_SOURCE_INSPECT__ = {
      total_bytes: 10,
      file_count: 1,
      folders: [{ name: "random.txt", bytes: 10 }],
      truncated: false,
      ceiling_bytes: 2_147_483_648,
      is_config: false,
      already_managed: false,
      login: "logged_out",
      keychain_bound: false,
    };
    const mounted = mountWizard();
    await reachIdentityViaCandidate(mounted);
    const reject = mounted.container.querySelector("[data-testid='source-not-config']")!;
    expect(reject.getAttribute("role")).toBe("alert");
    expect(reject.textContent).toContain("doesn't look like a Claude Code setup");
  });

  it("the duplicate block renders the existing label with [Open it]", async () => {
    const importSpy = vi
      .spyOn(bridgeModule, "accountImport")
      .mockRejectedValue({
        code: "already_imported",
        retryable: false,
        user_message: "Already imported as «Work» — open it from Accounts instead of importing again",
      });
    const mounted = mountWizard();
    await reachIdentityViaCandidate(mounted);
    const input = mounted.container.querySelector(
      "input[placeholder^='e.g. Work']",
    ) as HTMLInputElement;
    await act(async () => {
      fireEvent.change(input, { target: { value: "Second try" } });
    });
    await act(async () => {
      fireEvent.click(
        Array.from(mounted.container.querySelectorAll("button")).find(
          (button) => button.textContent === "Import account",
        )!,
      );
    });
    const blocked = await waitFor(() => {
      const found = mounted.container.querySelector("[data-testid='duplicate-blocked']");
      expect(found).not.toBeNull();
      return found!;
    });
    expect(blocked.getAttribute("role")).toBe("alert");
    expect(blocked.textContent).toContain("Already imported as «Work»");
    expect(blocked.textContent).toContain("Open it");
    importSpy.mockRestore();
  });
});
