/**
 * Persona D3 on rc33 (Devon: "find the session where I was fixing the
 * login flow in the api repo"). The rail's search for "login" emptied the
 * rail to its Also line, with no word of why; History's empty search named
 * Settings › Privacy with no way to it. Now the rail says nothing matches
 * and offers History's search, which opens with the words in its field;
 * History's line links to the setting itself, which only shows it (with
 * its own explanation), never turns it on.
 */
import { act, cleanup, fireEvent, render, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { InstanceBoard } from "./WorkflowRail";
import { OutsideSearchSetting } from "./OutsideSearchSetting";
import { ConversationsProvider } from "../lib/conversations";
import { DEFAULT_VIEW, resetRailViewCache, setRailView } from "../lib/rail-view";
import { resetSeen } from "../lib/seen";
import { resetRailOrderCache } from "../lib/rail-order-store";
import { needsFor } from "../test/needs-kit";
import { History } from "../screens/History";
import { targetOf } from "../screens/settings/panes";
import { matchedRow, splitAnchor } from "../screens/settings/settings-index";
import { useNavigation } from "../shell/use-navigation";
import { clearSeams, detailOf, instance, project, setSeam } from "../test/fixtures";
import type { OutsideSearchInfo } from "../lib/ipc/outsideSearch";

const NOW = new Date(2026, 10, 11, 10).getTime();
const projects = [project({ id: "p-api", root: "/Users/you/Dev/api", name: "api" })];

function own(id: string) {
  return instance({ id, account_id: "claude-work", account_label: "Work", git_branch: "main", started_at_ms: NOW - 60_000, last_activity_ms: NOW - 30_000, cwd: "/Users/you/Dev/api", project_id: "p-api" });
}

/** The `aij:` events the page sent, by name. */
function heard(name: string): unknown[] {
  const got: unknown[] = [];
  const on = (event: Event) => got.push(detailOf(event));
  window.addEventListener(name, on);
  afterEach(() => window.removeEventListener(name, on));
  return got;
}

beforeEach(() => {
  resetRailViewCache();
  resetRailOrderCache();
  setRailView(DEFAULT_VIEW);
  resetSeen();
});
afterEach(() => {
  cleanup();
  localStorage.clear();
  clearSeams("__AIJ_FIXTURE__", "__AIJ_OUTSIDE_SEARCH__", "__AIJ_OUTSIDE_SEARCH_CALLS__");
  resetRailViewCache();
  resetRailOrderCache();
});

describe("the rail's search that finds nothing", () => {
  it("says so, and offers History's search for the words", async () => {
    const screens = heard("aij:open-screen");
    const sessions = [own("s1")];
    setSeam("__AIJ_FIXTURE__", { conversations: [] });
    needsFor({ sessions, conversations: [], projects });
    const view = render(
      <ConversationsProvider>
        <InstanceBoard sessions={sessions} loginTerminals={[]} selectedId={null} onSelect={() => {}} density="comfortable" freshness={{}} now={NOW} projects={projects} snooze={{}} onSnooze={() => {}} />
      </ConversationsProvider>,
    );
    await view.findByTestId("place-board");
    expect(view.queryByTestId("rail-no-match")).toBeNull();
    fireEvent.change(view.getByLabelText(/^Search sessions/), { target: { value: "Login" } });
    const none = await view.findByTestId("rail-no-match");
    expect(none.textContent).toContain("No open or recent session matches “Login”");
    fireEvent.click(view.getByTestId("rail-no-match-history"));
    expect(view.getByTestId("rail-no-match-history").textContent).toContain("Search all history for “login”");
    expect(screens).toEqual([{ id: "history", query: "login" }]);
  });

  it("opens History with the words in its search", () => {
    const { result } = renderHook(() => useNavigation());
    act(() => void window.dispatchEvent(new CustomEvent("aij:open-screen", { detail: { id: "history", query: "login" } })));
    expect(result.current.historyQuery).toBe("login");
  });
});

describe("History's search that finds nothing", () => {
  it("links to Settings › Privacy's search switch, and turns nothing on", async () => {
    const opened = heard("aij:open-settings");
    setSeam("__AIJ_FIXTURE__", { conversations: [] });
    setSeam("__AIJ_OUTSIDE_SEARCH__", { enabled: false, size_bytes: null, sessions: 3, indexed: 0 } satisfies OutsideSearchInfo);
    const view = render(
      <ConversationsProvider>
        <History sessions={[]} projects={projects} onChanged={() => {}} initialQuery="login" />
      </ConversationsProvider>,
    );
    const link = await view.findByTestId("history-privacy-link");
    expect(view.getByTestId("page-sessions-empty").textContent).toContain("aren't searched (Settings › Privacy).");
    fireEvent.click(link);
    expect(opened).toEqual([{ section: "search" }]);
    expect(Reflect.get(window, "__AIJ_OUTSIDE_SEARCH_CALLS__")).toBeUndefined();
  });

  it("lands on the switch's own row, with its explanation", async () => {
    setSeam("__AIJ_OUTSIDE_SEARCH__", { enabled: false, size_bytes: null, sessions: 3, indexed: 0 } satisfies OutsideSearchInfo);
    const target = targetOf("search")!;
    expect(target.pane).toBe("privacy");
    const { section, words } = splitAnchor(target.anchor!);
    expect(section).toBe("stats");
    const view = render(
      <section id="settings-stats">
        <fieldset>Crash reports stay on this computer.</fieldset>
        <OutsideSearchSetting />
      </section>,
    );
    await waitFor(() => expect(view.getByTestId("privacy-outside-search")).toBeTruthy());
    const row = matchedRow(view.container.querySelector("section")!, words);
    expect(row?.dataset["testid"]).toBe("privacy-outside-search");
    expect(row?.textContent).toContain("Off by default; turning it off deletes the index.");
  });
});
