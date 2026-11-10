/**
 * rc22 (AJ-E3306, walk A3): a session AI Juggler started in a Claude app
 * profile is that profile's. Its header's App chip names "Claude app ·
 * Work Max" and opens that profile's page in one click, never the CLI App
 * on the profile's folder ("Claude Code · Work Max"); the chip shows in a
 * folded header too (it was hidden until the header was unfolded); and
 * the line on its page names the same App, as a link.
 */
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { ActionHostProvider } from "../../components/ItemMenu";
import { OPEN_APP } from "../../lib/apps";
import { forgetMarksForTests } from "../../lib/marks";
import { setToolsForTests } from "../../lib/tools";
import { PREVIEW_TOOLS } from "../../preview/tools";
import type { KnownPlaceInfo } from "../../types/generated/KnownPlaceInfo";
import { PageHeader } from "../PageShell";
import { SessionAppLink } from "./AccountMenu";
import type { SessionModel } from "./model";
import { SessionChips } from "./SessionHead";

setToolsForTests(PREVIEW_TOOLS);


const PROFILE: KnownPlaceInfo = {
  id: "app:claude-app:/Users/you/Library/Application Support/ClaudeProfiles/Work Max",
  app_id: "claude-app",
  app: "Claude",
  instance: "Work Max",
  profile: "Work Max",
  label: "Claude · Work Max",
  app_pid: null,
  lost: false,
  keep_alive: false,
  paused: null,
  account_id: "folder-work-max",
  seen_ms: 1,
  cannot_start: null,
  recent: [],
  found: false,
};

/** AI Juggler's session in the Claude app, on the profile's folder. */
const model: SessionModel = {
  tool: "claude-code",
  profileId: "folder-work-max",
  profileLabel: "Work Max",
  conversationId: null,
  sessionId: "a1",
  switchSource: "a1",
  live: false,
  state: { words: "Idle", tone: "neutral", asks: false, working: false, since: null },
  places: [],
  viewedMs: null,
  cwd: "/Users/you/repos/web",
  model: null,
  desktop: true,
};

let opened: string[] = [];
const listen = (event: Event) => {
  const detail: unknown = event instanceof CustomEvent ? event.detail : null;
  if (detail && typeof detail === "object" && "id" in detail) opened.push(String(detail.id));
};

beforeEach(() => {
  forgetMarksForTests();
  Object.assign(window, { __AIJ_PLACES__: { places: [PROFILE] } });
  opened = [];
  window.addEventListener(OPEN_APP, listen);
});
afterEach(() => {
  cleanup();
  window.removeEventListener(OPEN_APP, listen);
  Reflect.deleteProperty(window, "__AIJ_PLACES__");
  vi.restoreAllMocks();
});

it("shows the App chip in a folded header and opens the profile's page in one click", async () => {
  render(
    <ActionHostProvider host={{}}>
      <PageHeader kind="session" layout="session" mark={null} name="Fix the login" folded onToggleFold={() => {}} chips={<SessionChips model={model} account={undefined} now={0} />} />
    </ActionHostProvider>,
  );
  // Folded: the chip is there, its name a door to the profile's page.
  const chip = await screen.findByRole("button", { name: "Open Claude app · Work Max's page" });
  expect(screen.getByTestId("session-app-chip").textContent).toBe("Claude app · Work Max");
  fireEvent.click(chip);
  expect(opened).toEqual([PROFILE.id]);
  expect(screen.queryByText(/Claude Code/)).toBeNull();
});

it("names the same App on the page's line, as a link to its page", async () => {
  render(<SessionAppLink model={model} />);
  const link = await screen.findByRole("button", { name: "Claude app · Work Max" });
  fireEvent.click(link);
  expect(opened).toEqual([PROFILE.id]);
});
