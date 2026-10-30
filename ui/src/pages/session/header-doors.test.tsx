/**
 * The 0.3 re-render's review: a session header's tool door says its
 * tool's name beside its mark (a bare mark read as an unlabelled icon,
 * and the header names only the app it runs in).
 */
import { cleanup, fireEvent, render } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { ActionHostProvider } from "../../components/ItemMenu";
import * as tools from "../../lib/tools";
import { SessionToolChip } from "./HeaderDoors";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

it("names the tool beside its mark, and opens its details", () => {
  vi.spyOn(tools, "useTools").mockReturnValue([{ id: "claude-code", display_name: "Claude Code" }] as unknown as ReturnType<typeof tools.useTools>);
  const openTool = vi.fn();
  const view = render(
    <ActionHostProvider host={{ openTool }}>
      <SessionToolChip tool="claude-code" />
    </ActionHostProvider>,
  );
  const chip = view.getByTestId("context-tool");
  expect(chip.textContent).toBe("Claude Code");
  expect(chip.getAttribute("aria-label")).toBe("About Claude Code");
  fireEvent.click(chip);
  expect(openTool).toHaveBeenCalledWith(expect.objectContaining({ id: "claude-code" }));
});
