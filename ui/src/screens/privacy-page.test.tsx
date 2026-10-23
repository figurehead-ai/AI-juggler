/**
 * AJ-E1518 vitest — the Privacy page and the tools' terms line: every
 * line from the one claims list, each with its tests behind the line's
 * disclosure and linked to their source; what each tool's terms say about
 * several accounts, linked, on the page and while accounts are added.
 */
import { cleanup, fireEvent, render, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { PrivacyScreen } from "./PrivacyScreen";
import { FirstRun } from "./FirstRun";
import { TermsLine } from "./FirstRunSteps";
import { PrivacyCard } from "../components/PrivacyCard";
import claims from "../lib/trust-claims.json";
import { PREVIEW_TOOLS } from "../preview/tools";
import { setToolsForTests } from "../lib/tools";

const w = () => window as unknown as Record<string, unknown>;

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
    const links = within(tests).getAllByRole("link");
    expect(links.map((link) => link.textContent)).toEqual(claim.tests.map((reference) => reference.split("::")[1]));
    expect(links[0]!.getAttribute("href")).toBe(`https://github.com/figurehead-ai/AI-juggler/blob/main/${claim.tests[0]!.split("::")[0]}`);
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
      const link = within(rows[index]!).getByRole("link");
      expect(link.getAttribute("href")).toBe(tool.terms!.url);
      expect(link.textContent).toContain(tool.terms!.name);
    });
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

  it("Settings' Privacy card opens the page", async () => {
    const opened = vi.fn();
    window.addEventListener("aij:open-screen", opened);
    const view = render(<PrivacyCard />);
    fireEvent.click(await view.findByTestId("privacy-open-page"));
    window.removeEventListener("aij:open-screen", opened);
    expect(opened).toHaveBeenCalledTimes(1);
    expect((opened.mock.calls[0]![0] as CustomEvent).detail).toEqual({ id: "privacy" });
  });
});

describe("AJ-E1518 the terms line while accounts are added", () => {
  it("says what the tool's terms mean for several accounts, and links them", () => {
    const view = render(<TermsLine tools={["claude-code"]} />);
    const line = view.getByTestId("terms-line");
    const claude = PREVIEW_TOOLS.find((tool) => tool.id === "claude-code")!;
    expect(line.textContent).toContain(claude.terms!.accounts);
    expect(within(line).getByRole("link").getAttribute("href")).toBe(claude.terms!.url);
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
