/**
 * AJ-E1518 vitest — the Privacy page and the tools' terms line: every
 * line from the one claims list, each with its tests behind the line's
 * disclosure and linked to their source; what each tool's terms say about
 * several accounts, linked, on the page and while accounts are added.
 */
import { cleanup, fireEvent, render, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { PrivacyScreen } from "./PrivacyScreen";
import { FirstRun } from "./FirstRun";
import { TermsLine } from "./FirstRunSteps";
import claims from "../lib/trust-claims.json";
import { PREVIEW_TOOLS } from "../preview/tools";
import { setToolsForTests } from "../lib/tools";

const w = () => window as unknown as Record<string, unknown>;
const opened = () => (w().__AIJ_OPENED__ ?? []) as Array<{ kind: string; id: string }>;

afterEach(cleanup);
beforeEach(() => {
  for (const key of Object.keys(w()).filter((key) => key.startsWith("__AIJ_"))) delete w()[key];
});

describe("AJ-E1518 the Privacy page", () => {
  it("says every claim, grouped as the list groups them", () => {
    const view = render(<PrivacyScreen />);
    for (const group of claims.groups) {
      const section = view.getByTestId(`privacy-${group.id}`);
      expect(within(section).getByRole("heading").textContent).toBe(group.title);
      const rows = within(section).getAllByTestId("privacy-claim");
      expect(rows.map((row) => row.querySelector("summary")?.textContent)).toEqual(group.claims.map((claim) => claim.text));
    }
  });

  it("keeps each claim's tests behind it, each linked to its file", () => {
    const view = render(<PrivacyScreen />);
    const claim = claims.groups.flatMap((group) => group.claims).find((entry) => entry.id === "no-ai-provider")!;
    const row = view.getAllByTestId("privacy-claim").find((entry) => entry.textContent?.startsWith(claim.text))!;
    const details = row.querySelector("details")!;
    expect(details.open).toBe(false);
    fireEvent.click(row.querySelector("summary")!);
    const tests = within(row).getByTestId("privacy-tests");
    expect(tests.textContent).toContain(`Checked by these ${claim.tests.length} tests:`);
    const links = within(tests).getAllByRole("button");
    expect(links.map((link) => link.textContent)).toEqual(claim.tests.map((reference) => reference.split("::")[1]));
    // The page names the file; the app picks the address (trust_open).
    fireEvent.click(links[0]!);
    expect(opened()).toEqual([{ kind: "source", id: claim.tests[0]!.split("::")[0] }]);
  });

  it("says what the agent tools expose, per pool level, each line with its tests", () => {
    const view = render(<PrivacyScreen />);
    const section = view.getByTestId("privacy-changes");
    for (const id of ["agent-tools", "agent-tools-sees"]) {
      const claim = claims.groups.flatMap((group) => group.claims).find((entry) => entry.id === id)!;
      const row = within(section).getAllByTestId("privacy-claim").find((entry) => entry.textContent?.startsWith(claim.text));
      expect(row, id).toBeTruthy();
      expect(claim.tests.some((reference) => reference.startsWith("crates/juggler-agentbridge/tests/bridge.rs::"))).toBe(true);
    }
    const sees = claims.groups.flatMap((group) => group.claims).find((entry) => entry.id === "agent-tools-sees")!.text;
    for (const level of ["Read", "Work together", "Full"]) expect(sees).toContain(level);
    expect(sees).toContain("never an email or a plan");
  });

  it("every claim names at least one test", () => {
    for (const claim of claims.groups.flatMap((group) => group.claims)) {
      expect(claim.tests.length, claim.id).toBeGreaterThan(0);
    }
  });

  it("lists each tool's terms with its line and a link, from the catalog", () => {
    const view = render(<PrivacyScreen />);
    const rows = view.getAllByTestId("privacy-terms-row");
    const withTerms = PREVIEW_TOOLS.filter((tool) => tool.terms !== null);
    expect(rows).toHaveLength(withTerms.length);
    withTerms.forEach((tool, index) => {
      expect(rows[index]!.textContent).toContain(tool.terms!.accounts);
      const link = within(rows[index]!).getByRole("button");
      expect(link.textContent).toContain(tool.terms!.name);
      fireEvent.click(link);
      expect(opened().at(-1)).toEqual({ kind: "terms", id: tool.id });
    });
  });

  it("says how each tool's sessions run under its terms, only for a tool that has a runtime", () => {
    const view = render(<PrivacyScreen />);
    const rows = view.getAllByTestId("privacy-terms-row");
    const withTerms = PREVIEW_TOOLS.filter((tool) => tool.terms !== null);
    withTerms.forEach((tool, index) => {
      const note = within(rows[index]!).queryByTestId("privacy-runtime-note");
      if (tool.runtime?.terms_note) expect(note?.textContent).toBe(tool.runtime.terms_note);
      else expect(note).toBeNull();
    });
    expect(view.getAllByTestId("privacy-runtime-note")).toHaveLength(2);
  });

  it("the private report opens from the page's last line", () => {
    const view = render(<PrivacyScreen />);
    fireEvent.click(view.getByRole("button", { name: "report it privately" }));
    expect(opened()).toEqual([{ kind: "report", id: "" }]);
  });

  it("a tool without terms has no row", () => {
    setToolsForTests([{ ...PREVIEW_TOOLS[0]!, terms: null }]);
    try {
      const view = render(<PrivacyScreen />);
      expect(view.queryByTestId("privacy-terms")).toBeNull();
    } finally {
      setToolsForTests(PREVIEW_TOOLS);
    }
  });

  it("Settings › Privacy holds every line, in one place (0.1.5)", async () => {
    const { PrivacyPane } = await import("./settings/PrivacyPane");
    const view = render(<PrivacyPane />);
    expect(view.getByTestId("privacy-page")).toBeTruthy();
    expect(view.getAllByTestId("privacy-claim").length).toBe(claims.groups.flatMap((group) => group.claims).length);
    expect(view.queryByTestId("privacy-open-page")).toBeNull();
  });
});

describe("AJ-E1518 the terms line while accounts are added", () => {
  it("says what the tool's terms mean for several accounts, and links them", () => {
    const view = render(<TermsLine tools={["claude-code"]} />);
    const line = view.getByTestId("terms-line");
    const claude = PREVIEW_TOOLS.find((tool) => tool.id === "claude-code")!;
    expect(line.textContent).toContain(claude.terms!.accounts);
    fireEvent.click(within(line).getByRole("button"));
    expect(opened()).toEqual([{ kind: "terms", id: "claude-code" }]);
  });

  it("one line per tool being added, none for a tool without terms", () => {
    const view = render(<TermsLine tools={["claude-code", "codex", "no-such-tool"]} />);
    expect(within(view.getByTestId("terms-line")).getAllByRole("listitem")).toHaveLength(2);
    cleanup();
    const none = render(<TermsLine tools={["no-such-tool"]} />);
    expect(none.queryByTestId("terms-line")).toBeNull();
  });

  it("Add account shows it for the tool being added", async () => {
    const view = render(<FirstRun mode="add" provider="codex" onDone={() => null} openLoginTerminal={() => null} />);
    const line = await view.findByTestId("terms-line");
    const codex = PREVIEW_TOOLS.find((tool) => tool.id === "codex")!;
    expect(line.textContent).toContain(codex.terms!.accounts);
    expect(line.textContent).not.toContain("Anthropic");
  });
});

describe("AJ-E1518 nothing switches on its own (the window)", () => {
  // Every source file, as text (tests excluded below).
  const sources = import.meta.glob("../**/*.{ts,tsx}", { query: "?raw", import: "default", eager: true }) as Record<string, string>;
  // The switch screens: each starts a switch from its own confirm button.
  const SCREENS = [
    "../components/SwitchFlow.tsx",
    "../components/SwitchBackFlow.tsx",
    "../components/CrossMachineSwitchFlow.tsx",
    "../components/CrossMachineSwitchBack.tsx",
  ];
  const MOVES = ["switchExecute", "switchBackExecute", "fleetSwitchExecute", "fleetSwitchBackExecute", "switchUndo", "sessionHandover"];

  it("only the switch screens start a switch", () => {
    const callers: string[] = [];
    for (const [path, text] of Object.entries(sources)) {
      if (/\.test\.tsx?$/.test(path) || path.includes("/test/")) continue;
      // Comments don't count, whatever they say.
      const code = text.replace(/\/\*[\s\S]*?\*\//g, "").replace(/\/\/[^\n]*/g, "");
      for (const name of MOVES) {
        const calls = code.match(new RegExp(`(?<![\\w.])${name}\\s*\\(`, "g")) ?? [];
        const defined = new RegExp(`function\\s+${name}\\s*\\(`).test(code) ? 1 : 0;
        if (calls.length - defined > 0) callers.push(`${path} calls ${name}`);
      }
    }
    expect(callers.filter((caller) => !SCREENS.some((screen) => caller.startsWith(`${screen} `)))).toEqual([]);
    expect(callers.length, "the scan found the switch screens' calls").toBeGreaterThanOrEqual(SCREENS.length);
  });
});
