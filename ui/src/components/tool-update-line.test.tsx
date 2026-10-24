/**
 * AJ-E1611 vitest — a tool's page says when a newer version is out,
 * whether AI Juggler was tested with it, and the command to update it
 * yourself; nothing when it's current; the daily check can be turned off.
 */
import { act, cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { ToolUpdateLine } from "./ToolUpdateLine";
import type { ToolUpdateInfo } from "../lib/ipc/toolUpdates";

const w = () => window as unknown as Record<string, unknown>;

const row = (over: Partial<ToolUpdateInfo> = {}): ToolUpdateInfo => ({
  tool: "codex",
  installed: "0.163.0",
  latest: "0.170.0",
  newer: true,
  verified: false,
  checked_ms: 1_800_000_000_000,
  update_command: "npm install -g @openai/codex@latest",
  checks_on: true,
  has_feed: true,
  ...over,
});

afterEach(() => {
  cleanup();
  delete w().__AIJ_TOOL_UPDATES__;
});

describe("AJ-E1611 — the tool update line", () => {
  it("says a newer version is out, not tested yet, and how to update it yourself", async () => {
    w().__AIJ_TOOL_UPDATES__ = { codex: row() };
    const view = render(<ToolUpdateLine tool="codex" name="Codex" />);
    const line = await view.findByTestId("provider-update-available");
    expect(line.textContent).toBe(
      "Codex 0.170.0 is out; not tested with AI Juggler yet. To update, run npm install -g @openai/codex@latest yourself.",
    );
    // A command to copy, never a button that runs it.
    expect(view.queryAllByRole("button")).toHaveLength(0);
  });

  it("says nothing more of a version it was tested with", async () => {
    w().__AIJ_TOOL_UPDATES__ = { codex: row({ verified: true }) };
    const view = render(<ToolUpdateLine tool="codex" name="Codex" />);
    expect((await view.findByTestId("provider-update-available")).textContent).toContain("Codex 0.170.0 is out.");
  });

  it("is quiet when the tool is current, and can turn the daily check off", async () => {
    const current = row({ newer: false, latest: "0.163.0" });
    w().__AIJ_TOOL_UPDATES__ = { codex: current };
    const view = render(<ToolUpdateLine tool="codex" name="Codex" />);
    const check = (await view.findByTestId("provider-update-check")) as HTMLInputElement;
    expect(view.queryByTestId("provider-update-available")).toBeNull();
    expect(check.checked).toBe(true);
    await act(async () => {
      fireEvent.click(check);
    });
    await waitFor(() => expect((view.getByTestId("provider-update-check") as HTMLInputElement).checked).toBe(false));
    expect(current.checks_on).toBe(false);
  });

  it("shows nothing for a tool with no public list of versions", async () => {
    w().__AIJ_TOOL_UPDATES__ = { acme: row({ tool: "acme", has_feed: false }) };
    const view = render(<ToolUpdateLine tool="acme" name="Acme" />);
    await new Promise((resolve) => setTimeout(resolve, 20));
    expect(view.queryByTestId("provider-update")).toBeNull();
  });
});
