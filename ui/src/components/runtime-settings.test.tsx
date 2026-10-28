/**
 * The 0.2 walk (F12): Settings › Sessions › How sessions run named each
 * launch setting by its id ("prof-…") and offered "Through Codex's
 * app-server". It names them as everywhere ("Work · Default") and says how
 * a session runs in plain words.
 */
import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import * as runtime from "../lib/ipc/runtime";
import { RuntimeSettings, RUN_WORDS } from "./RuntimeSettings";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

describe("How sessions run", () => {
  it("names each launch setting and says how it runs in plain words", async () => {
    vi.spyOn(runtime, "runtimeChoices").mockResolvedValue({
      beta_on: false,
      profiles: [
        { profile_id: "prof-1a2b3c", account_label: "Work", name: "Main", label: "Codex's app-server", default: "driven", chosen: null, beta: false, unavailable: null },
        { profile_id: "prof-9z8y7x", account_label: "Personal", name: "Opus pairing", label: "Codex's app-server", default: "driven", chosen: null, beta: false, unavailable: null },
      ],
    });
    const view = render(<RuntimeSettings />);
    const rows = await view.findAllByTestId(/^runtime-prof-/);
    expect(rows.map((node) => node.querySelector("span")?.textContent)).toEqual(["Work · Default", "Personal · Opus pairing"]);
    const text = view.getByTestId("settings-runtime").textContent!;
    expect(text).not.toMatch(/prof-|app-server/);
    expect(text).toContain(`${RUN_WORDS.driven} (default)`);
    expect(text).toContain(RUN_WORDS.terminal);
  });
});
