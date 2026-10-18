/**
 * AJ-E309 vitest — the first-run policy hook, renewal dates, the
 * Expert-setup hatch, and the funnel events:
 *  - the Policy question rides Confirm with NOTHING pre-selected (the
 *    copy lint renders the markup and finds no `checked`), Accept waits
 *    for an answer, ⌘↩ never answers it;
 *  - "Decide later" writes NOTHING and Summary says so; each stored
 *    option is written ONCE on the accept click and restated;
 *  - an existing choice is never re-asked; locked flows never ask;
 *  - renewal dates are optional per Confirm row, empty = "expiry
 *    unknown", set = written via account_set_renewal;
 *  - [Expert setup…] shows the read-only folder, Reveal, the shell
 *    line (once an account exists), env overrides, Import from folder;
 *  - every funnel event carries only allow-listed, free-text-free
 *    fields (no paths, labels, or credentials).
 */
import { act, fireEvent, render, waitFor } from "@testing-library/react";
import { renderToStaticMarkup } from "react-dom/server";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { OnboardingWizard } from "../screens/OnboardingWizard";
import { PackPolicyChooser, packPolicyRestatement } from "./PackPolicyChooser";
import { envRowProblem } from "./EnvOverlayEditor";
import * as bridgeModule from "../lib/bridge";
import type { ProviderInstallData } from "../lib/bridge";

const w = () => window as unknown as Record<string, unknown>;

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

const RECOMMENDATION = {
  items: [
    { provider: "claude-code", label: "Work", action: "import", rationale: "Signed in — import keeps history", default_on: true, path: "/Users/you/.claude-work" },
    { provider: "claude-code", label: "Side", action: "import", rationale: "Active — you'll sign in after import", default_on: true, path: "/Users/you/.claude-side" },
  ],
  summary: "Your juggling set: Claude ×2 (Work — signed in; Side — needs sign-in).",
  start_here: "-users-dev-api",
};

const SIGNED_IN_IMPORT = (label: string) => ({
  account: { id: `acct-${label.toLowerCase()}`, provider: "claude-code", label, plan: null, login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false },
  files_copied: 3,
  bytes_copied: 3,
  skipped: [],
  login: "logged_in",
  login_note: null,
  degraded: false,
});

beforeEach(() => {
  for (const key of [
    "__AIJ_PROVIDER_INSTALLS__",
    "__AIJ_RECOMMENDATION__",
    "__AIJ_REDACTION_POLICY__",
    "__AIJ_REDACTION_SET_GLOBAL__",
    "__AIJ_FUNNEL_EVENTS__",
    "__AIJ_RENEWAL_SET__",
    "__AIJ_SCAN_REPORT__",
    "__AIJ_EXPERT__",
    "__AIJ_RECIPE_ENV__",
    "__AIJ_RECIPE_ENV_REJECT__",
  ]) {
    delete w()[key];
  }
  w().__AIJ_PROVIDER_INSTALLS__ = INSTALLED;
  w().__AIJ_RECOMMENDATION__ = RECOMMENDATION;
});

const mountWizard = (locked = false) =>
  render(
    <OnboardingWizard
      provider="claude-code"
      locked={locked}
      onClose={() => {}}
      onAccountCreated={() => {}}
      openLoginTerminal={() => {}}
    />,
  );

/** Gate → scan → open the proposal (the set path's Confirm). */
const openProposal = async () => {
  const mounted = mountWizard();
  await waitFor(() =>
    expect(mounted.container.querySelector("[data-testid='install-continue']")).not.toBeNull(),
  );
  await act(async () => {
    fireEvent.click(mounted.container.querySelector("[data-testid='install-continue']")!);
  });
  await waitFor(() =>
    expect(mounted.container.querySelector("[data-testid='use-recommended']")).not.toBeNull(),
  );
  fireEvent.click(mounted.container.querySelector("[data-testid='use-recommended']")!);
  await waitFor(() =>
    expect(mounted.container.querySelector("[data-testid='accept-setup']")).not.toBeNull(),
  );
  return mounted;
};

const radio = (container: HTMLElement, value: string) =>
  container.querySelector<HTMLInputElement>(
    `[data-testid='policy-panel'] input[type='radio'][value='${value}']`,
  )!;

const acceptAndReachSummary = async (container: HTMLElement) => {
  const spy = vi
    .spyOn(bridgeModule, "accountImport")
    .mockImplementation(async (_provider, _path, label) => SIGNED_IN_IMPORT(label) as never);
  await act(async () => {
    fireEvent.click(container.querySelector("[data-testid='accept-setup']")!);
  });
  await waitFor(() =>
    expect(container.querySelector("[data-testid='set-summary']")).not.toBeNull(),
  );
  spy.mockRestore();
};

describe("AJ-E309 Req.1 — the policy question, no pre-selected default", () => {
  it("copy lint: the chooser markup carries NO checked radio until the user picks", () => {
    const html = renderToStaticMarkup(<PackPolicyChooser value={null} onChange={() => {}} />);
    expect(html).not.toMatch(/checked/);
    expect(html).toContain("<fieldset");
    expect(html).toContain("<legend");
    expect((html.match(/type="radio"/g) ?? []).length).toBe(4);
    // Every option states its consequence (one sentence each).
    for (const words of ["Strip", "Keep", "Ask each time", "Decide later"]) {
      expect(html).toContain(words);
    }
  });

  it("rides the Confirm: Accept is disabled with a stated reason, and ⌘↩ cannot answer it", async () => {
    const { container } = await openProposal();
    const accept = container.querySelector<HTMLButtonElement>("[data-testid='accept-setup']")!;
    expect(accept.disabled).toBe(true);
    expect(accept.getAttribute("aria-describedby")).toBeTruthy();
    expect(container.querySelector("[data-testid='accept-blocked']")!.textContent).toContain(
      "Decide later",
    );
    const importSpy = vi.spyOn(bridgeModule, "accountImport");
    await act(async () => {
      fireEvent.keyDown(window, { key: "Enter", metaKey: true });
    });
    expect(importSpy).not.toHaveBeenCalled();
    importSpy.mockRestore();
  });

  it("Decide later writes NOTHING, stages no pack, and Summary says so", async () => {
    const { container } = await openProposal();
    fireEvent.click(radio(container, "decide_later"));
    await acceptAndReachSummary(container);
    expect(w().__AIJ_REDACTION_SET_GLOBAL__).toBeUndefined();
    expect(container.querySelector("[data-testid='policy-restatement']")!.textContent).toBe(
      "Packs: not chosen yet — your first switch asks before any pack is built",
    );
    const events = w().__AIJ_FUNNEL_EVENTS__ as { kind: string; fields: Record<string, string> }[];
    expect(events.filter((event) => event.kind === "policy_chosen")).toEqual([
      { kind: "policy_chosen", fields: { choice: "decide_later" } },
    ]);
  });

  it.each([
    ["strip", "Packs: strip secrets (all Providers)"],
    ["keep", "Packs: keep secrets — packs stay on this Mac (all Providers)"],
    ["ask", "Packs: ask each time (all Providers)"],
  ])("choosing %s writes the global once on the accept click and restates it", async (value, restated) => {
    const { container } = await openProposal();
    fireEvent.click(radio(container, value));
    // Picking a radio writes nothing yet — only the accept click does.
    expect(w().__AIJ_REDACTION_SET_GLOBAL__).toBeUndefined();
    await acceptAndReachSummary(container);
    expect(w().__AIJ_REDACTION_SET_GLOBAL__).toEqual([value]);
    expect(container.querySelector("[data-testid='policy-restatement']")!.textContent).toBe(restated);
  });

  it("an existing choice is never re-asked — Summary restates it", async () => {
    w().__AIJ_REDACTION_POLICY__ = { global: "keep", override: null, effective: "keep", source: "global", line: "" };
    const { container } = await openProposal();
    expect(container.querySelector("[data-testid='policy-panel']")).toBeNull();
    expect(container.querySelector<HTMLButtonElement>("[data-testid='accept-setup']")!.disabled).toBe(false);
    await acceptAndReachSummary(container);
    expect(w().__AIJ_REDACTION_SET_GLOBAL__).toBeUndefined();
    expect(container.querySelector("[data-testid='policy-restatement']")!.textContent).toBe(
      "Packs: keep secrets — packs stay on this Mac (all Providers)",
    );
  });

  it("locked flows (add Provider / add Account) never ask, keeping their click budgets", async () => {
    const { container } = mountWizard(true);
    await waitFor(() =>
      expect(container.querySelector("[data-testid='use-recommended']")).not.toBeNull(),
    );
    fireEvent.click(container.querySelector("[data-testid='use-recommended']")!);
    await waitFor(() =>
      expect(container.querySelector("[data-testid='accept-setup']")).not.toBeNull(),
    );
    expect(container.querySelector("[data-testid='policy-panel']")).toBeNull();
    expect(container.querySelector<HTMLButtonElement>("[data-testid='accept-setup']")!.disabled).toBe(false);
  });

  it("the restatement helper names an override's scope", () => {
    expect(packPolicyRestatement("strip", "override", "Codex")).toBe(
      "Packs: strip secrets (Codex override)",
    );
  });
});

describe("AJ-E309 Req.3 — optional renewal dates (D-12)", () => {
  it("each Confirm row offers one; set → written, empty → 'expiry unknown'", async () => {
    w().__AIJ_REDACTION_POLICY__ = { global: "strip", override: null, effective: "strip", source: "global", line: "" };
    const { container } = await openProposal();
    const fields = container.querySelectorAll("[data-testid='renewal-date']");
    expect(fields.length).toBe(2);
    expect(fields[0]!.getAttribute("data-state")).toBe("unknown");
    expect(fields[0]!.textContent).toContain("Expiry unknown");
    // Open the first row's field: a LABELLED native date input.
    fireEvent.click(container.querySelector("[data-testid='renewal-date-add']")!);
    const input = container.querySelector<HTMLInputElement>("[data-testid='renewal-date-input']")!;
    expect(input.type).toBe("date");
    expect(input.getAttribute("aria-label")).toBe("Renewal date for Work (optional)");
    fireEvent.change(input, { target: { value: "2026-11-30" } });
    await acceptAndReachSummary(container);
    expect(w().__AIJ_RENEWAL_SET__).toEqual([["acct-work", "2026-11-30"]]);
    const restated = Array.from(container.querySelectorAll("[data-testid='renewal-restatement']")).map(
      (row) => row.textContent,
    );
    expect(restated).toEqual([
      "Work: renews 2026-11-30 — we'll warn you before it",
      "Side: expiry unknown",
    ]);
  });

  it("Skip clears the date without penalty — nothing is written", async () => {
    w().__AIJ_REDACTION_POLICY__ = { global: "strip", override: null, effective: "strip", source: "global", line: "" };
    const { container } = await openProposal();
    fireEvent.click(container.querySelector("[data-testid='renewal-date-add']")!);
    fireEvent.change(container.querySelector("[data-testid='renewal-date-input']")!, {
      target: { value: "2026-12-01" },
    });
    fireEvent.click(container.querySelector("[data-testid='renewal-date-skip']")!);
    expect(container.querySelector("[data-testid='renewal-date-input']")).toBeNull();
    await acceptAndReachSummary(container);
    expect(w().__AIJ_RENEWAL_SET__).toBeUndefined();
  });
});

describe("AJ-E309 Req.5 — [Expert setup…]", () => {
  it("before an account exists: read-only folder, Reveal, Import from folder, honest pending notes", async () => {
    w().__AIJ_REDACTION_POLICY__ = { global: "strip", override: null, effective: "strip", source: "global", line: "" };
    const { container } = await openProposal();
    const hatch = await waitFor(() => {
      const found = container.querySelector<HTMLButtonElement>("[data-testid='expert-hatch']");
      expect(found).not.toBeNull();
      return found!;
    });
    expect(hatch.getAttribute("aria-expanded")).toBe("false");
    fireEvent.click(hatch);
    await waitFor(() =>
      expect(container.querySelector("[data-testid='expert-state-dir']")).not.toBeNull(),
    );
    expect(hatch.getAttribute("aria-expanded")).toBe("true");
    const dir = container.querySelector<HTMLInputElement>("[data-testid='expert-state-dir']")!;
    expect(dir.readOnly).toBe(true);
    expect(dir.value).toContain("AI Juggler/accounts/claude-code");
    expect(container.querySelector("[data-testid='expert-reveal']")).not.toBeNull();
    expect(container.querySelector("[data-testid='expert-snippet-pending']")).not.toBeNull();
    expect(container.querySelector("[data-testid='expert-env-pending']")).not.toBeNull();
    expect(container.querySelector("[data-testid='expert-import-folder']")).not.toBeNull();
    expect(container.querySelector("[data-testid='expert-export-metrics']")).not.toBeNull();
    const events = w().__AIJ_FUNNEL_EVENTS__ as { kind: string; fields: Record<string, string> }[];
    expect(events).toContainEqual({ kind: "expert_opened", fields: { surface: "wizard" } });
  });

  it("after the accept: the shell line and the env editor for the new accounts", async () => {
    w().__AIJ_REDACTION_POLICY__ = { global: "strip", override: null, effective: "strip", source: "global", line: "" };
    const recipes = vi.spyOn(bridgeModule, "launchRecipesList").mockResolvedValue([
      { id: "recipe-work", name: "Main", account_id: "acct-work", account_label: "Work", provider: "claude-code", model: null, default_cwd: null },
    ]);
    const { container } = await openProposal();
    await acceptAndReachSummary(container);
    const summaryHatch = Array.from(container.querySelectorAll<HTMLButtonElement>("[data-testid='expert-hatch']")).at(-1)!;
    fireEvent.click(summaryHatch);
    await waitFor(() =>
      expect(container.querySelector("[data-testid='expert-snippet']")).not.toBeNull(),
    );
    expect(container.querySelector("[data-testid='expert-snippet']")!.textContent).toContain(
      "CLAUDE_CONFIG_DIR=",
    );
    // Two accounts were made → a picker.
    expect(container.querySelectorAll("[data-testid='expert-account'] option").length).toBe(2);
    await waitFor(() =>
      expect(container.querySelector("[data-testid='env-overlay-editor']")).not.toBeNull(),
    );
    recipes.mockRestore();
  });
});

describe("AJ-E309 Req.5 — env overrides refuse isolation and credential keys inline", () => {
  const protectedKeys = ["CLAUDE_CONFIG_DIR", "AI_JUGGLER_CLAUDE_BIN"];
  it.each([
    ["CLAUDE_CONFIG_DIR", "x", "Managed by AI Juggler"],
    ["ANTHROPIC_API_KEY", "x", "Looks like a credential"],
    ["CLAUDE_CODE_OAUTH_TOKEN", "x", "Looks like a credential"],
    ["lower", "x", "Use capitals"],
    ["MULTI", "a\nb", "One line"],
  ])("%s is refused", (key, value, why) => {
    expect(envRowProblem(key, value, protectedKeys)).toContain(why);
  });
  it("ordinary tuning variables pass (whole-segment matching)", () => {
    expect(envRowProblem("CLAUDE_CODE_MAX_OUTPUT_TOKENS", "8000", protectedKeys)).toBeNull();
    expect(envRowProblem("ANTHROPIC_BASE_URL", "http://127.0.0.1:8080", protectedKeys)).toBeNull();
  });
});

describe("AJ-E309 Req.6 — funnel events carry no paths, labels, or credentials", () => {
  it("a full first-run emits only allow-listed, free-text-free fields", async () => {
    const { container } = await openProposal();
    // Customise (secondary) toggles the block; reopen it for the accept.
    const customise = container.querySelector("[data-testid='proposal-customise']");
    expect(customise).not.toBeNull();
    fireEvent.click(customise!);
    if (container.querySelector("[data-testid='accept-setup']") === null) {
      fireEvent.click(customise!);
    }
    await waitFor(() => expect(container.querySelector("[data-testid='policy-panel']")).not.toBeNull());
    // Let the scan finish so its completion event lands.
    await waitFor(() => expect(container.querySelector("[data-testid='scan-progress']")).toBeNull(), {
      timeout: 4000,
    });
    fireEvent.click(radio(container, "strip"));
    await acceptAndReachSummary(container);
    const events = w().__AIJ_FUNNEL_EVENTS__ as { kind: string; fields: Record<string, string> }[];
    const kinds = events.map((event) => event.kind);
    for (const expected of [
      "flow_step",
      "scan_started",
      "scan_completed",
      "proposal_shown",
      "proposal_customised",
      "policy_chosen",
      "proposal_accepted",
      "import_started",
      "import_completed",
    ]) {
      expect(kinds).toContain(expected);
    }
    const forbidden = ["path", "label", "token", "dir", "source", "account", "email", "key", "name", "secret"];
    for (const event of events) {
      for (const [key, value] of Object.entries(event.fields)) {
        expect(forbidden).not.toContain(key);
        expect(value).toMatch(/^[a-z0-9][a-z0-9_:-]*$/);
      }
    }
    const serialized = JSON.stringify(events);
    for (const leak of ["/Users", "Work", "Side", ".claude"]) {
      expect(serialized).not.toContain(leak);
    }
  });
});
