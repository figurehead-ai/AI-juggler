/**
 * AJ-E1525 (ADR-0019, ENGINEERING-GUIDE Q3): a new tool needs no UI
 * change. First run and Add account render from a catalog holding a tool
 * no screen has ever heard of: its name, its groups, its sign-in words and
 * its install page all come from its spec.
 */
import { cleanup, render, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { FirstRun } from "./screens/FirstRun";
import { AddProviderSheet } from "./screens/AddProviderSheet";
import { genericTool, primaryToolId, setToolsForTests, toolSpec, firstRunToolIds } from "./lib/tools";
import { ownSignIn, providerDisplayName, sameAsWords } from "./lib/providers";
import { PREVIEW_TOOLS } from "./preview/tools";
import type { ToolSpecInfo } from "./types/generated/ToolSpecInfo";

const w = () => window as unknown as Record<string, unknown>;
const NOW = Math.floor(Date.now() / 1000);

/** A tool the UI has no word of: only its spec describes it. */
const ACME: ToolSpecInfo = {
  ...genericTool("acme"),
  display_name: "Acme Coder",
  vendor: "Acme",
  install_url: "https://acme.example/install",
  sign_in_own: "Acme Coder's own sign-in (Acme)",
  sign_in_short: "Acme Coder's (Acme)",
  same_as: "as when you run Acme yourself",
  scan_hint: "Your Acme homes. Nothing is changed.",
  setup_groups: [{ id: "acme-homes", title: "Your Acme homes", path_contains: null, origin: null, shows_install: true }],
  default_label: "Acme",
  flow: { first_run: true, primary: true },
  supports: { in_place: true, limits: false, app_meter: false, thread_link: false },
};

const acmeSetup = {
  harness: "acme",
  path: "/Users/you/.acme",
  origin: "vendor-default",
  login: "logged_in",
  bytes: 1000,
  last_used_secs: NOW - 3600,
  managed: false,
  notes: [],
  signals: ["vendor-default"],
  inference: {
    purpose: "Main",
    purpose_confidence: "likely",
    login: "signed-in",
    login_confidence: "found",
    activity: "active",
    activity_confidence: "found",
    activity_words: "Active this week",
    keychain_bound: false,
  },
};

beforeEach(() => {
  for (const key of Object.keys(w()).filter((key) => key.startsWith("__AIJ_"))) delete w()[key];
  setToolsForTests([ACME, ...PREVIEW_TOOLS.map((tool) => ({ ...tool, flow: { ...tool.flow, primary: false } }))]);
  w().__AIJ_SCAN_REPORT__ = { installs: [acmeSetup], probes: [], seeds: [], desktop_apps: [], cancelled: false };
  w().__AIJ_PROVIDER_INSTALLS__ = [
    { provider: "acme", display_name: "Acme Coder", install_url: "https://acme.example/install", status: "installed", version: "3.1.0", binary: "/usr/local/bin/acme", notes: [], verified_against: ["3.1"] },
  ];
  w().__AIJ_PROVIDER_CAPS__ = [{ provider: "acme", can_use_in_place: true, reports_limits: false }];
});
afterEach(() => {
  cleanup();
  setToolsForTests(PREVIEW_TOOLS);
});

describe("AJ-E1525 a tool only its spec describes", () => {
  it("the words come from the spec, and an unknown id stays itself", () => {
    expect(providerDisplayName("acme")).toBe("Acme Coder");
    expect(ownSignIn("acme")).toBe("Acme Coder's own sign-in (Acme)");
    expect(sameAsWords(["acme"])).toBe("as when you run Acme yourself");
    expect(primaryToolId()).toBe("acme");
    expect(firstRunToolIds()[0]).toBe("acme");
    expect(providerDisplayName("never-heard-of")).toBe("never-heard-of");
    expect(toolSpec("never-heard-of").mark).toBeNull();
  });

  it("first run brings its setups in under its own group, named by its own words", async () => {
    const view = render(<FirstRun onDone={() => null} openLoginTerminal={() => null} />);
    const group = await view.findByTestId("first-run-group-acme-homes");
    expect(within(group).getAllByText(/Your Acme homes/).length).toBeGreaterThan(0);
    expect(group.textContent).toContain("Acme");
    expect(group.textContent).toContain("Acme Coder 3.1.0 installed");
    await waitFor(() => expect(view.getByTestId("first-run-found").textContent).toContain("Acme Coder setup"));
  });

  it("Add account offers it by its name", async () => {
    const view = render(<AddProviderSheet onClose={() => {}} onChoose={() => {}} />);
    await waitFor(() => expect(view.getAllByText(/Acme Coder/).length).toBeGreaterThan(0));
  });
});
