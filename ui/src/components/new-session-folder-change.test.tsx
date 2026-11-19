/**
 * F-rc3f-1 (the 0.4.6 rc3f walk; persona Priya): in New session, picking
 * another folder moved the App to the one last used there, with no sign
 * it changed ("Start in AIJuggler on Claude Code CLI · aij-test", then
 * "team" gave "… on Claude Code CLI · Lab — the only account used here").
 * The ruling: New session shows which App and Account it's about to use,
 * clearly, and never preselects silently.
 *
 * - A suggested App is suggested again for the new folder, and when it
 *   moves the card says so, naming both Apps and why, with Undo.
 * - An App the person picked (Change app, the door, the binding they just
 *   set) stays; where the folder's own App is another, it's named and
 *   offered, never taken.
 * - One line shows the App and its Account; the live region says them
 *   when they change.
 * - Change app lists the folder's suggestion first, marked, then the rest.
 */
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { NewSessionModal } from "../screens/NewSession";
import * as launchModule from "../lib/ipc/launch";
import * as accountsModule from "../lib/ipc/accounts";
import type { AccountInfo } from "../types/generated/AccountInfo";
import type { LaunchRecipeInfo } from "../types/generated/LaunchRecipeInfo";
import type { ProjectInfo } from "../types/generated/ProjectInfo";

const seam = (values: Record<string, unknown>) => Object.assign(window, values);

const folder = (id: string, label: string): AccountInfo => ({ id, provider: "claude-code", label, plan: null, login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false });
const ACCOUNTS = [folder("claude-work", "Work Max"), folder("claude-side", "Side")];
const recipe = (id: string, account_id: string, account_label: string): LaunchRecipeInfo => ({ id, name: "Main", account_id, account_label, provider: "claude-code", model: null, default_cwd: null, restart_on_crash: null });
const RECIPES = [recipe("p-1", "claude-work", "Work Max"), recipe("p-3", "claude-side", "Side")];

const project = (name: string, ago: number, used: string[]): ProjectInfo => ({
  id: `proj-${name}`,
  root: `/Users/dev/${name}`,
  name,
  last_activity_ms: Date.now() - ago,
  accounts_used: used,
  history_sessions: 3,
  history_last_used_ms: null,
  repo: null,
  active_instance: null,
  instance_count: 0,
  missing: false,
});

// api: only Work Max ever ran there (worked in last); web: only Side;
// docs: only Work Max.
const PROJECTS = [project("api", 12 * 60_000, ["Work Max"]), project("web", 86_400_000, ["Side"]), project("docs", 2 * 86_400_000, ["Work Max"])];

const member = (id: string, label: string) => ({ id, label, folder: `~/.config/${id}`, plan: null, renewal_date: null, fee_usd: null });
const SUBSCRIPTIONS = [
  { key: "k-work", account_id: "acct-work", name: "Work", tool: "claude-code", state: "signed_in", email: null, organisation: null, reason: null, plan: "max", renewal_date: null, fee_usd: null, differs: [], profiles: [member("claude-work", "Work Max")] },
  { key: "k-side", account_id: "acct-side", name: "Side", tool: "claude-code", state: "signed_in", email: null, organisation: null, reason: null, plan: "pro", renewal_date: null, fee_usd: null, differs: [], profiles: [member("claude-side", "Side")] },
];

// The quota seam's reading, as the backend sends it.
const display = (account: string, left: number) => ({
  account,
  weekly: { state: "ok", percentage: 100 - left, resets_at_ms: Date.now() + 3 * 3_600_000, badge: "measured", measured_at_ms: Date.now() },
  five_hour: null,
  forecast: { state: "learning", badge: "learning" },
  renewal: { state: "none", label: "", entered_by_user: false },
});

beforeEach(() => seam({ __AIJ_RECIPES__: RECIPES, __AIJ_QUOTA_DISPLAY__: [], __AIJ_RANKING__: [], __AIJ_FIXTURE__: { subscriptions: SUBSCRIPTIONS } }));

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  Reflect.deleteProperty(window, "__AIJ_FIXTURE__");
});

const mountSheet = ({ account = null as string | null } = {}) => {
  const launchSpy = vi.spyOn(launchModule, "launchSession").mockResolvedValue({ session_id: "s1", pid: 1 });
  render(<NewSessionModal projects={PROJECTS} accounts={ACCOUNTS} recipes={RECIPES} initialDirectory={null} initialAccount={account} onClose={() => {}} onLaunched={() => {}} openLoginTerminal={() => {}} />);
  return { launchSpy };
};

const sentence = () => screen.getByTestId("new-session-sentence").textContent;
const appLine = () => screen.getByTestId("launch-app-line");
const changeFolder = async (name: string) => {
  fireEvent.click(screen.getByTestId("change-folder"));
  fireEvent.click(await screen.findByTestId(`change-folder-/Users/dev/${name}`));
};
const changeApp = async (id: string) => {
  fireEvent.click(screen.getByTestId("change-app"));
  fireEvent.click(await screen.findByTestId(`change-app-${id}`));
};

describe("a suggested App moves with the folder, visibly", () => {
  it("the note names the old and the new App and why, and Undo takes the old one back", async () => {
    const { launchSpy } = mountSheet();
    expect(sentence()).toBe("Start in api on Claude Code CLI · Work Max — the only account used here");
    // Nothing changed yet: no note.
    expect(screen.queryByTestId("launch-app-changed")).toBeNull();
    await changeFolder("web");
    await waitFor(() => expect(sentence()).toBe("Start in web on Claude Code CLI · Side — the only account used here"));
    const note = screen.getByTestId("launch-app-changed");
    expect(note.textContent).toBe("App changed to Claude Code CLI · Side (was Claude Code CLI · Work Max): the only account used here. Undo");
    // On the card, next to the App it's about.
    expect(within(screen.getByTestId("launch-preselected")).getByTestId("launch-app-changed")).toBe(note);
    const undo = screen.getByTestId("launch-app-undo");
    expect(undo.getAttribute("aria-label")).toBe("Undo: back to Claude Code CLI · Work Max");
    fireEvent.click(undo);
    // Back on the old App, now the person's own: no why, no note.
    await waitFor(() => expect(sentence()).toBe("Start in web on Claude Code CLI · Work Max"));
    expect(screen.queryByTestId("launch-app-changed")).toBeNull();
    fireEvent.keyDown(window, { key: "Enter", metaKey: true });
    await waitFor(() => expect(launchSpy).toHaveBeenCalledWith("p-1", "/Users/dev/web", undefined));
  });

  it("the App it moved to, picked in Change app, is the person's: the note goes (F-rc3g-5)", async () => {
    // The 0.4.6 rc3g walk: after a folder change moved the suggestion,
    // Change app › the same App kept "App changed to … (was …) Undo",
    // a note on a choice the person had since made their own.
    const { launchSpy } = mountSheet();
    await changeFolder("web");
    await waitFor(() => expect(screen.getByTestId("launch-app-changed")).toBeTruthy());
    await changeApp("claude-side");
    await waitFor(() => expect(screen.queryByTestId("launch-app-changed")).toBeNull());
    // Theirs now: no why, no Suggested mark, and Start goes where it says.
    expect(sentence()).toBe("Start in web on Claude Code CLI · Side");
    expect(within(appLine()).queryByTestId("suggested-mark")).toBeNull();
    // Theirs in the next folder too: kept, never moved.
    await changeFolder("docs");
    await waitFor(() => expect(sentence()).toBe("Start in docs on Claude Code CLI · Side"));
    expect(screen.queryByTestId("launch-app-changed")).toBeNull();
    fireEvent.keyDown(window, { key: "Enter", metaKey: true });
    await waitFor(() => expect(launchSpy).toHaveBeenCalledWith("p-3", "/Users/dev/docs", undefined));
  });

  it("a folder that keeps the same suggestion says nothing changed", async () => {
    mountSheet();
    await changeFolder("docs");
    await waitFor(() => expect(sentence()).toBe("Start in docs on Claude Code CLI · Work Max — the only account used here"));
    expect(screen.queryByTestId("launch-app-changed")).toBeNull();
  });

  it("a dropped folder follows the same rule: the move is said", async () => {
    mountSheet();
    const zone = screen.getByTestId("new-session-drop");
    fireEvent.drop(zone, { dataTransfer: { getData: (type: string) => (type === "text/uri-list" ? "file:///Users/dev/web" : "") } });
    await waitFor(() => expect(screen.getByTestId("launch-app-changed").textContent).toContain("App changed to Claude Code CLI · Side (was Claude Code CLI · Work Max)"));
  });

  it("the App line names the App and its Account, marked Suggested while New session chose it", async () => {
    mountSheet();
    await waitFor(() => expect(within(appLine()).getByTestId("launch-account").textContent).toBe("Work"));
    expect(within(appLine()).getByTestId("launch-app").textContent).toBe("App: Claude Code CLI · Work Max");
    expect(within(appLine()).getByTestId("suggested-mark")).toBeTruthy();
    await changeFolder("web");
    await waitFor(() => expect(within(appLine()).getByTestId("launch-app").textContent).toBe("App: Claude Code CLI · Side"));
    expect(within(appLine()).getByTestId("launch-account").textContent).toBe("Side");
    expect(within(appLine()).getByTestId("suggested-mark")).toBeTruthy();
    // Picked by the person: no longer a suggestion.
    await changeApp("claude-work");
    await waitFor(() => expect(within(appLine()).getByTestId("launch-app").textContent).toBe("App: Claude Code CLI · Work Max"));
    expect(within(appLine()).queryByTestId("suggested-mark")).toBeNull();
  });
});

describe("an App the person picked is never replaced by a folder change", () => {
  it("picked in Change app: it stays, and the folder's own App is named and offered, never taken", async () => {
    const { launchSpy } = mountSheet();
    await changeApp("claude-side");
    await waitFor(() => expect(sentence()).toBe("Start in api on Claude Code CLI · Side"));
    await changeFolder("docs");
    await waitFor(() => expect(sentence()).toBe("Start in docs on Claude Code CLI · Side"));
    expect(screen.queryByTestId("launch-app-changed")).toBeNull();
    const kept = screen.getByTestId("launch-app-kept");
    expect(kept.textContent).toBe("Kept Claude Code CLI · Side, as you chose. Claude Code CLI · Work Max is the only account used in docs. Use Claude Code CLI · Work Max");
    // Start goes where it says.
    fireEvent.keyDown(window, { key: "Enter", metaKey: true });
    await waitFor(() => expect(launchSpy).toHaveBeenCalledWith("p-3", "/Users/dev/docs", undefined));
  });

  it("taking the offer uses the folder's own App", async () => {
    mountSheet();
    await changeApp("claude-side");
    await changeFolder("docs");
    fireEvent.click(await screen.findByTestId("launch-use-folder-own"));
    await waitFor(() => expect(sentence()).toBe("Start in docs on Claude Code CLI · Work Max"));
    expect(screen.queryByTestId("launch-app-kept")).toBeNull();
  });

  it("chosen by the door that opened it: it stays", async () => {
    mountSheet({ account: "claude-side" });
    expect(sentence()).toBe("Start in api on Claude Code CLI · Side");
    await changeFolder("docs");
    await waitFor(() => expect(sentence()).toBe("Start in docs on Claude Code CLI · Side"));
    expect(screen.getByTestId("launch-app-kept").textContent).toContain("Claude Code CLI · Work Max is the only account used in docs");
  });

  it("bound to the folder just now (Always start …): it's the person's, and stays in the next folder", async () => {
    vi.spyOn(accountsModule, "projectSetDefaultAccount").mockResolvedValue();
    mountSheet();
    fireEvent.click(screen.getByTestId("launch-always"));
    await waitFor(() => expect(sentence()).toBe("Start in api on Claude Code CLI · Work Max — your default here"));
    await changeFolder("web");
    await waitFor(() => expect(sentence()).toBe("Start in web on Claude Code CLI · Work Max"));
    expect(screen.queryByTestId("launch-app-changed")).toBeNull();
    expect(screen.getByTestId("launch-app-kept").textContent).toBe("Kept Claude Code CLI · Work Max, as you chose. Claude Code CLI · Side is the only account used in web. Use Claude Code CLI · Side");
  });
});

describe("keyboard and screen reader", () => {
  it("a polite live region says the App and its Account when they change, and the note with them", async () => {
    mountSheet();
    const region = screen.getByTestId("new-session-told");
    // An <output>: a status region, there from the first frame.
    expect(region.tagName).toBe("OUTPUT");
    expect(screen.getAllByRole("status")).toContain(region);
    expect(region.getAttribute("aria-live")).toBe("polite");
    // Opening says nothing of its own (the sheet's name and Start do).
    expect(region.textContent).toBe("");
    await changeFolder("web");
    await waitFor(() => expect(screen.getByTestId("new-session-heard").textContent).toBe("Starts on Claude Code CLI · Side, account Side."));
    expect(region.textContent).toContain("App changed to Claude Code CLI · Side (was Claude Code CLI · Work Max)");
    await changeApp("claude-work");
    await waitFor(() => expect(screen.getByTestId("new-session-heard").textContent).toBe("Starts on Claude Code CLI · Work Max, account Work."));
  });
});

describe("Change app: suggested first, then the rest (the pickers rule)", () => {
  it("the folder's suggestion leads, marked, even after the person picked another with more room", async () => {
    seam({ __AIJ_QUOTA_DISPLAY__: [display("claude-work", 40), display("claude-side", 80)] });
    mountSheet();
    await changeApp("claude-side");
    await waitFor(() => expect(sentence()).toBe("Start in api on Claude Code CLI · Side"));
    fireEvent.click(screen.getByTestId("change-app"));
    await screen.findByTestId("change-app-claude-work");
    const apps = screen
      .getAllByRole("menuitemradio")
      .map((item) => item.getAttribute("data-testid"))
      .filter((id) => !id?.startsWith("change-app-always"));
    // By room alone Side would lead; api's own suggestion comes first.
    expect(apps).toEqual(["change-app-claude-work", "change-app-claude-side"]);
    await waitFor(() => expect(screen.getByTestId("change-app-claude-work").textContent).toMatch(/^Claude Code CLI · Work MaxSuggested · Work · /));
    expect(screen.getByTestId("change-app-claude-side").textContent).not.toContain("Suggested");
  });
});
