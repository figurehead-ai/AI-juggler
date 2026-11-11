/**
 * AJ-E3206 R3: Settings › Accounts › "Tools on this computer": each tool's
 * row opens that Tool's page (AJ-E3104), with the action's own words.
 */
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { AccountsPane } from "./AccountsPane";
import { ActionHostProvider } from "../../components/ItemMenu";

type Host = Parameters<typeof ActionHostProvider>[0]["host"];

afterEach(cleanup);

describe("Settings › Accounts › Tools on this computer", () => {
  it("each tool's row opens that Tool's page", async () => {
    const openTool = vi.fn();
    const host: Partial<Host> = { openTool };
    render(
      <ActionHostProvider host={host as Host}>
        <AccountsPane />
      </ActionHostProvider>,
    );
    const rows = await screen.findAllByTestId("settings-tool");
    expect(rows.length).toBeGreaterThan(0);
    const open = await screen.findAllByTestId("settings-tool-open");
    expect(open).toHaveLength(rows.length);
    expect(open[0]!.textContent).toBe("Open its page");
    // WCAG 2.5.3: the name starts with the words on the button.
    expect(open[0]!.getAttribute("aria-label")).toMatch(/^Open its page: .+$/);
    fireEvent.click(open[0]!);
    expect(openTool).toHaveBeenCalledTimes(1);
    expect(openTool.mock.calls[0]![0]).toHaveProperty("id");
  });
});
