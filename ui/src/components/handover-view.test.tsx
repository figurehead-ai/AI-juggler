/**
 * 0.3 — what the other tool was told: the one line, Open the handover
 * (the rows a saved one is read with), Save handover… on a session's page
 * with Reveal in Finder, and the Withheld words that never call a key
 * sent as the user chose "hidden".
 */
import { cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { SaveHandover, SwitchHandover, withheldWords } from "./HandoverView";
import type { HandoverSheetInfo } from "../lib/ipc/handover";
import sectionSource from "./HandoverSection.tsx?raw";
import continueSource from "../pages/session/ContinueOn.tsx?raw";

const w = () => window as unknown as Record<string, unknown>;
const SEAMS = ["__AIJ_HANDOVER_SUMMARY__", "__AIJ_HANDOVER__", "__AIJ_HANDOVER_SAVED__", "__AIJ_HANDOVER_SAVE_ERROR__", "__AIJ_HANDOVER_REVEALED__", "__AIJ_PACK__"];

beforeEach(() => SEAMS.forEach((seam) => delete w()[seam]));
afterEach(() => {
  cleanup();
  SEAMS.forEach((seam) => delete w()[seam]);
});

const LINE = "Hidden from Codex: 2 API keys · Left out: 12 earlier turns, 1 note";

const SHEET: HandoverSheetInfo = {
  from_tool: "Claude Code",
  from_account: "Work Max",
  to_tool: "Codex",
  to_account: "Personal",
  created_at_ms: 1_792_700_000_000,
  project_root: "~/Dev/shop",
  fidelity: "Full story, fresh start",
  carries: ["the conversation (24 turns)"],
  not_carried: ["12 earlier turns: they didn't fit, so the handover names what was asked in them"],
  withheld: [{ what: "API keys", count: 2 }],
  kept: [],
  brief: "The brief, quoted.",
  line: LINE,
};

describe("the Withheld words", () => {
  it("say what was hidden, what was sent as chosen, and what isn't known", () => {
    expect(withheldWords([{ what: "API keys", count: 2 }], [])).toBe("2 API keys, when it was made. They stay out.");
    expect(withheldWords([], [])).toBe("Nothing: no secrets were found when it was made.");
    expect(withheldWords([], [{ what: "API key", count: 1 }])).toBe("Nothing hidden. Sent as they were, as you chose: 1 API key.");
    expect(withheldWords(null, [])).toBe("Not recorded for this handover.");
  });
});

describe("a switch's handover", () => {
  it("shows nothing for a switch that wrote none (the same tool copies the session)", async () => {
    w().__AIJ_HANDOVER_SUMMARY__ = { "edge-same": null };
    const view = render(<SwitchHandover edgeId="edge-same" />);
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(view.queryByTestId("switch-handover")).toBeNull();
  });

  it("says what was told in one line and opens the handover read-only, on the click only", async () => {
    w().__AIJ_HANDOVER_SUMMARY__ = { "edge-1": { to_tool: "Codex", line: LINE } };
    w().__AIJ_HANDOVER__ = SHEET;
    const view = render(<SwitchHandover edgeId="edge-1" />);
    await waitFor(() => expect(view.getByTestId("handover-line").textContent).toBe(LINE));
    expect(view.queryByTestId("handover-sheet")).toBeNull();
    const open = view.getByRole("button", { name: "Open the handover" });
    expect(open.getAttribute("aria-expanded")).toBe("false");
    fireEvent.click(open);
    await waitFor(() => expect(view.getByTestId("pack-withheld").textContent).toBe("2 API keys, when it was made. They stay out."));
    const sheet = view.getByTestId("handover-sheet");
    expect(sheet.getAttribute("aria-label")).toBe("What Codex was told");
    expect(sheet.textContent).toContain("Codex on Personal");
    expect(sheet.textContent).toContain("12 earlier turns");
    expect(view.getByRole("button", { name: "Hide the handover" }).getAttribute("aria-expanded")).toBe("true");
  });

  it("with nothing to say, offers the handover without a line", async () => {
    const view = render(<SwitchHandover edgeId="edge-2" summary={{ to_tool: "Codex", line: null }} />);
    expect(view.queryAllByTestId("handover-line")).toHaveLength(0);
    expect(view.getAllByRole("button", { name: "Open the handover" }).length).toBeGreaterThan(0);
  });
});

describe("Save handover… on a session's page", () => {
  it("lives beside Continue on another account, on both session pages", () => {
    expect(sectionSource).toContain("<SaveHandover sessionId={session.id} />");
    expect(continueSource).toContain("<SaveHandover sessionId={source} />");
  });

  it("saves on the click, says where and what was hidden, and reveals it in Finder", async () => {
    w().__AIJ_HANDOVER_SAVED__ = { path: "/data/exports/pack-1", shown: "~/data/exports/pack-1", line: "Hidden: 1 API key" };
    const view = render(<SaveHandover sessionId="sess-1" />);
    expect(view.queryByTestId("saved-handover")).toBeNull();
    fireEvent.click(view.getByRole("button", { name: "Save handover…" }));
    await waitFor(() => expect(view.getByTestId("saved-handover").textContent).toContain("Handover saved to ~/data/exports/pack-1"));
    expect(view.getByTestId("handover-line").textContent).toBe("Hidden: 1 API key");
    fireEvent.click(view.getByRole("button", { name: "Reveal in Finder" }));
    await waitFor(() => expect(w().__AIJ_HANDOVER_REVEALED__).toEqual(["/data/exports/pack-1"]));
    // Open the handover: the saved pack, in a sheet of its own.
    fireEvent.click(view.getByRole("button", { name: "Open the handover" }));
    await waitFor(() => expect(view.getByRole("dialog", { name: "The saved handover" })).toBeTruthy());
    await waitFor(() => expect(view.getByTestId("pack-withheld")).toBeTruthy());
  });

  it("says why when it can't save", async () => {
    w().__AIJ_HANDOVER_SAVE_ERROR__ = "Choose what happens to keys and passwords first";
    const view = render(<SaveHandover sessionId="sess-1" />);
    fireEvent.click(view.getByRole("button", { name: "Save handover…" }));
    await waitFor(() => expect(view.getByRole("alert").textContent).toContain("Choose what happens to keys"));
    expect(view.queryByTestId("saved-handover")).toBeNull();
  });
});
