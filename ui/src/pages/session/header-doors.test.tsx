/**
 * The 0.3 re-render's review: a session header's tool door says its
 * tool's name beside its mark (a bare mark read as an unlabelled icon,
 * and the header names only the app it runs in).
 */
import { cleanup, fireEvent, render } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { ActionHostProvider } from "../../components/ItemMenu";
import * as tools from "../../lib/tools";
import { SessionContinuesDoor, SessionToolChip } from "./HeaderDoors";
import type { ContinuesInfo } from "../../types/generated/ContinuesInfo";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

const withClaudeCode = () =>
  vi.spyOn(tools, "useTools").mockReturnValue([{ id: "claude-code", display_name: "Claude Code" }] as unknown as ReturnType<typeof tools.useTools>);

it("names the tool beside its mark, and opens its details", () => {
  withClaudeCode();
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

// SK-11: after a switch the new session's page showed nothing of the
// conversation it carried; its header says what it continues, a door to it.
it("says what a switched session continues, and opens it", () => {
  withClaudeCode();
  const continues: ContinuesInfo = { session_id: "sess-orig", account_id: "acc-work", account_label: "Work", provider: "claude-code", conversation_id: "conv-1" };
  const openSessionById = vi.fn();
  const openConversationById = vi.fn();
  const view = render(
    <ActionHostProvider host={{ openSessionById, openConversationById }}>
      <SessionContinuesDoor session={{ continues, project_id: "p-web" }} />
      <SessionContinuesDoor session={{ continues: { ...continues, session_id: null }, project_id: "p-web" }} />
      <SessionContinuesDoor session={{ continues: undefined, project_id: null }} />
    </ActionHostProvider>,
  );
  const [own, outside] = view.getAllByTestId("door-continues");
  expect(view.getAllByTestId("door-continues")).toHaveLength(2);
  expect(own!.textContent).toBe("continues from Claude Code CLI · Work");
  fireEvent.click(own!);
  expect(openSessionById).toHaveBeenCalledWith("sess-orig");
  // One another app ran: its conversation is the door.
  fireEvent.click(outside!);
  expect(openConversationById).toHaveBeenCalledWith("acc-work", "conv-1", "p-web");
});
