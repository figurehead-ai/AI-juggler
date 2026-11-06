/**
 * The 0.4 review on an ended session's page: each thing said once.
 *
 * - 46: "Claude Code" was the App chip, the facts line's tool door and
 *   Where it runs' App row: the chip alone names it. The model reads as
 *   people say it ("Sonnet"), never its id.
 * - 84: a Runway card that only said it didn't know is left out.
 */
import { cleanup, render } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { ActionHostProvider } from "../../components/ItemMenu";
import * as tools from "../../lib/tools";
import { SessionToolChip } from "./HeaderDoors";
import { Runway } from "./Runway";
import { WhereItRuns } from "./WhereItRuns";
import type { SessionModel } from "./model";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

const model = {
  live: false,
  state: { words: "Ended", tone: "neutral", since: null, working: false, asks: false },
  places: [],
  tool: "claude-code",
  sessionId: "s1",
  conversationId: null,
  viewedMs: null,
  cwd: "/Users/you/Dev/ai-juggler",
  profileId: "claude-work",
  profileLabel: "Work",
  model: "sonnet",
} as unknown as SessionModel;

it("leaves the tool's door out where the App chip names the tool, and keeps it where it doesn't", () => {
  vi.spyOn(tools, "useTools").mockReturnValue([{ id: "claude-code", display_name: "Claude Code" }] as unknown as ReturnType<typeof tools.useTools>);
  const host = { openTool: vi.fn() };
  const said = render(
    <ActionHostProvider host={host}>
      <SessionToolChip tool="claude-code" said="Claude Code · Work" />
    </ActionHostProvider>,
  );
  expect(said.queryByTestId("context-tool")).toBeNull();
  cleanup();
  const other = render(
    <ActionHostProvider host={host}>
      <SessionToolChip tool="claude-code" said="Claude app · Work" />
    </ActionHostProvider>,
  );
  expect(other.getByTestId("context-tool").textContent).toBe("Claude Code");
});

it("Where it runs has no App row (the chip's), and names the model as people say it", () => {
  const view = render(<WhereItRuns model={model} accountWords={null} outside={false} />);
  const card = view.getByTestId("where-it-runs");
  const labels = [...card.querySelectorAll("dt")].map((dt) => dt.textContent);
  expect(labels).not.toContain("App");
  expect(card.textContent).not.toContain("Claude Code");
  const at = labels.indexOf("Model");
  expect(card.querySelectorAll("dd")[at]!.textContent).toBe("Sonnet");
});

it("leaves the Runway card out while nothing is measured, unless it has more to say", () => {
  const none = render(<Runway display={undefined} title={null} />);
  expect(none.queryByTestId("session-runway")).toBeNull();
  cleanup();
  // The Usage tab's card says what the window counts: it stays.
  const more = render(
    <Runway display={undefined} title={null} testId="session-usage-runway">
      <p>The window counts every session on the account.</p>
    </Runway>,
  );
  expect(more.getByTestId("account-runway").dataset.state).toBe("not-measured");
});
