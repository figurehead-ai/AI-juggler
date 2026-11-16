/**
 * AJ-E3013 (D-116, D-30's amendment): the consent card for AI Juggler's
 * connector, in each state — none yet (Add, its preview, then the add),
 * there (Remove, showing what it deletes), changed (Remove names what it
 * leaves), an older edition's (Update), a preview that refuses (the
 * guide), and an app that can't take it (why, and no guide) — and the
 * guide, which reads the App again only while its steps are open.
 */
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { forgetSharedReads } from "../../../lib/shared-read";
import { CONNECTOR, fileLine, previewSentence, shortPath } from "../../../lib/connector";
import { resetPreviewConnector } from "../../../lib/ipc/connector";
import { ConnectorCard } from "./ConnectorCard";

const WORK = "app:claude-app:/Users/you/Profiles/Work";
const CODEX = "app:codex-app:";
const FOLDER = "~/Profiles/Work/claude-code";
const calls = (): string[] => {
  const seen: unknown = Reflect.get(window, "__AIJ_CONNECTOR_CALLS__");
  return Array.isArray(seen) ? seen.map(String) : [];
};

beforeEach(() => {
  forgetSharedReads();
  resetPreviewConnector();
  Object.assign(window, { __AIJ_CONNECTOR__: undefined, __AIJ_CONNECTOR_CALLS__: undefined, __AIJ_DESKTOP_MEMBERS__: undefined });
});
afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

const card = (appId = WORK) => render(<ConnectorCard appId={appId} app="Claude · Work" testId="c" />);

describe("AJ-E3013: the connector's consent card", () => {
  it("none yet: the ask, its consequence, an example and the guide's section; Add shows the preview first, then adds", async () => {
    card();
    const ask = await screen.findByTestId("c");
    expect(ask.querySelector("h3")?.textContent).toBe("Add AI Juggler's connector to Claude · Work so its sessions can talk in this pool");
    expect(ask.textContent).toContain(CONNECTOR.consequence);
    expect(ask.textContent).toContain("For example:");
    expect(screen.getByTestId("c-docs").textContent).toBe("How desktop apps join a pool ↗");
    await act(async () => fireEvent.click(screen.getByTestId("c-add")));
    // The preview: each file in a line, its exact text in a fold; nothing written yet.
    const files = screen.getAllByTestId("c-file");
    expect(files.map((one) => one.firstElementChild?.textContent)).toEqual([`Creates ${FOLDER}/skills/ai-juggler/.claude-plugin/plugin.json`, `Creates ${FOLDER}/skills/ai-juggler/skills/pool-tasks/SKILL.md`]);
    expect(screen.getAllByTestId("c-file-text")[0]!.textContent).toContain('"mcpServers"');
    expect(screen.getAllByTestId("c-file-text")[0]!.textContent).not.toMatch(/token|secret|password/i);
    expect(calls()).toEqual([]);
    await act(async () => fireEvent.click(screen.getByTestId("c-confirm")));
    expect(calls()).toEqual([`add:${WORK}`]);
    expect(screen.getByTestId("c-added").textContent).toBe("Added. A running session picks it up at its next start.");
    // No restart is offered: nothing says one is safe.
    expect(screen.queryByText(/restart/i)).toBeNull();
  });

  it("Cancel on the preview writes nothing", async () => {
    card();
    await act(async () => fireEvent.click(await screen.findByTestId("c-add")));
    fireEvent.click(screen.getByTestId("c-cancel"));
    expect(await screen.findByTestId("c-add")).toBeTruthy();
    expect(calls()).toEqual([]);
  });

  it("there: one line, and Remove shows exactly what it deletes before it does", async () => {
    Object.assign(window, { __AIJ_CONNECTOR__: { [WORK]: { state: "current" } } });
    card();
    expect((await screen.findByTestId("c-there")).textContent).toContain("AI Juggler's connector is in this app.");
    await act(async () => fireEvent.click(screen.getByTestId("c-remove")));
    expect(screen.getAllByTestId("c-remove-file").map((one) => one.textContent)).toEqual([`Deletes ${FOLDER}/skills/ai-juggler/.claude-plugin/plugin.json`, `Deletes ${FOLDER}/skills/ai-juggler/skills/pool-tasks/SKILL.md`]);
    expect(calls()).toEqual([]);
    await act(async () => fireEvent.click(screen.getByTestId("c-confirm-remove")));
    expect(calls()).toEqual([`remove:${WORK}`]);
    expect(screen.getByTestId("c-removed").textContent).toBe("A running session keeps it until it next starts.");
  });

  it("changed: it says so, and Remove names the file it would delete", async () => {
    const changed = "/Users/you/Profiles/Work/claude-code/skills/ai-juggler/skills/pool-tasks/SKILL.md";
    Object.assign(window, { __AIJ_CONNECTOR__: { [WORK]: { state: "changed", changed: [changed] } } });
    card();
    expect((await screen.findByTestId("c-changed")).textContent).toContain(shortPath(changed));
    await act(async () => fireEvent.click(screen.getByTestId("c-remove")));
    const lines = screen.getAllByTestId("c-remove-file").map((one) => one.textContent);
    expect(lines).toContain(`${FOLDER}/skills/ai-juggler/skills/pool-tasks/SKILL.md changed since AI Juggler wrote it`);
  });

  it("an older edition's: Update, through the same preview", async () => {
    Object.assign(window, { __AIJ_CONNECTOR__: { [WORK]: { state: "outdated" } } });
    card();
    expect((await screen.findByTestId("c-outdated")).textContent).toBe(CONNECTOR.outdated);
    expect(screen.getByTestId("c-add").textContent).toBe("Update");
    await act(async () => fireEvent.click(screen.getByTestId("c-add")));
    expect(screen.getAllByTestId("c-file")[0]!.getAttribute("data-action")).toBe("replace");
    expect(fileLine("merge", "/Users/you/.codex/config.toml")).toBe("Adds AI Juggler's own table to ~/.codex/config.toml; the rest stays as it is");
  });

  it("a preview that refuses: why, what's in the way, and the guide instead, with Copy; no Add", async () => {
    Object.assign(window, { __AIJ_CONNECTOR__: { [WORK]: { state: "absent", refused: "~/Profiles/Work/claude-code/skills/ai-juggler/.claude-plugin/plugin.json is already there and isn't AI Juggler's as it wrote it, so Add won't replace it" } } });
    card();
    await act(async () => fireEvent.click(await screen.findByTestId("c-add")));
    expect(screen.getByTestId("c-refused").textContent).toContain("so Add won't replace it.");
    expect(screen.getAllByTestId("c-file")[0]!.textContent).toContain("Won't replace");
    expect(screen.queryByTestId("c-confirm")).toBeNull();
    expect(screen.getByTestId("turn-on-steps").textContent).toContain("Add plugins");
    expect(screen.getByTestId("turn-on-command").textContent).toBe("/plugin marketplace add figurehead-ai/AI-juggler");
    expect(screen.getByTestId("turn-on-copy")).toBeTruthy();
  });

  it("an app that can't take it (the Codex app): why, plainly, and that no guide can do it either", async () => {
    card(CODEX);
    // Never the backend's own text: its reason code, worded.
    expect((await screen.findByTestId("c-why-not")).textContent).toBe("AI Juggler can't add its connector to this app yet: it can't tell this app's sessions apart.");
    expect(screen.getByTestId("c-guide-none").textContent).toContain("no guide for it either");
    expect(screen.queryByTestId("c-add")).toBeNull();
  });
});

describe("AJ-E3013: the guide", () => {
  it("watches for it only while its steps are open, and says when it's done", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    card();
    await act(async () => fireEvent.click(await screen.findByTestId("c-guide-open")));
    expect(screen.getByTestId("c-guide-watching")).toBeTruthy();
    // Done elsewhere (the user installed it): seen on the next read.
    Object.assign(window, { __AIJ_CONNECTOR__: { [WORK]: { state: "current" } } });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(13_000);
    });
    await waitFor(() => expect(screen.getByTestId("c-guide-done").textContent).toBe("Done: AI Juggler's connector is in Claude · Work."));
  });

  it("with its steps closed, it reads nothing", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    card();
    await act(async () => fireEvent.click(await screen.findByTestId("c-guide-open")));
    fireEvent.click(screen.getByRole("button", { name: "Show me how to turn on AI Juggler's connector" }));
    expect(screen.queryByTestId("c-guide-watching")).toBeNull();
    Object.assign(window, { __AIJ_CONNECTOR__: { [WORK]: { state: "current" } } });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(13_000);
    });
    expect(screen.queryByTestId("c-guide-done")).toBeNull();
  });
});

// Casey's PC1 walk: the preview led with long paths, and "connector" wasn't
// explained where it first appeared; "Add it yourself" sounded like a
// terminal.
describe("AJ-E3013: the card in plain words first (Casey's PC1 walk)", () => {
  it("says what the connector is under its title; the preview opens with one plain sentence, the files under a closed Show the files", async () => {
    render(<ConnectorCard appId={WORK} app="Claude app · Work Max" kind="Claude app" profile="Work Max" testId="c" />);
    expect((await screen.findByTestId("c-meaning")).textContent).toBe("AI Juggler's connector is a small helper AI Juggler adds to an app so its sessions can read and send the pool's messages, notes and tasks.");
    expect(screen.getByTestId("c-guide-open").textContent).toBe("Show me how to add it by hand");
    await act(async () => fireEvent.click(screen.getByTestId("c-add")));
    expect(screen.getByTestId("c-preview-sentence").textContent).toBe("This adds a small helper to your Claude app (Work Max) so its sessions can talk in this pool: 2 new files in its own settings folder; nothing of yours changes.");
    const fold = screen.getByTestId("c-files-fold");
    expect(fold.hasAttribute("open")).toBe(false);
    expect(fold.textContent).toContain("Show the files");
    expect(fold.contains(screen.getAllByTestId("c-file")[0]!)).toBe(true);
  });

  it("the sentence counts what each file is", () => {
    expect(previewSentence([{ action: "merge" }], { name: "ChatGPT" })).toBe("This adds a small helper to ChatGPT so its sessions can talk in this pool: its own table added to 1 settings file; nothing of yours changes.");
    expect(previewSentence([{ action: "replace" }, { action: "replace" }], { name: "x", kind: "Claude app", profile: "Side" }, "pools")).toBe("This adds a small helper to your Claude app (Side) so its sessions can talk in pools: 2 older AI Juggler files updated; nothing of yours changes.");
  });
});

// 93b495f28: Remove is all or nothing; a changed file refuses it, named,
// and the card offers Remove anyway, with Cancel focused.
describe("AJ-E3013: Remove anyway", () => {
  it("a file changed since Add: Remove would take nothing out; Remove anyway names it and takes it out whole", async () => {
    const changed = "/Users/you/Profiles/Work/claude-code/skills/ai-juggler/skills/pool-tasks/SKILL.md";
    Object.assign(window, { __AIJ_CONNECTOR__: { [WORK]: { state: "changed", changed: [changed] } } });
    card();
    await act(async () => fireEvent.click(await screen.findByTestId("c-remove")));
    // Round 7 #8: it says nothing was taken out, then what Remove anyway
    // would delete; never "Deletes …" as if Remove will.
    expect(screen.getByTestId("c").querySelector("h3")!.textContent).toBe("Nothing was taken out of Claude · Work");
    expect(screen.getByTestId("c-remove-changed").textContent).toBe("SKILL.md changed since AI Juggler added it, so Remove leaves the connector as it is.");
    expect(screen.getByTestId("c").textContent).toContain("Remove anyway would delete:");
    expect(screen.getAllByTestId("c-remove-file").map((one) => one.textContent)).toEqual([`${FOLDER}/skills/ai-juggler/.claude-plugin/plugin.json`, `${FOLDER}/skills/ai-juggler/skills/pool-tasks/SKILL.md changed since AI Juggler wrote it`]);
    expect(screen.getByTestId("c").textContent).not.toMatch(/Deletes /);
    const anyway = screen.getByTestId("c-remove-anyway");
    expect(anyway.textContent).toBe("Remove anyway (also deletes SKILL.md)");
    expect(document.activeElement).toBe(screen.getByTestId("c-cancel"));
    expect(screen.queryByTestId("c-confirm-remove")).toBeNull();
    await act(async () => fireEvent.click(anyway));
    expect(calls()).toEqual([`remove-anyway:${WORK}`]);
    expect(screen.getByTestId("c-removed").textContent).toBe("A running session keeps it until it next starts.");
  });

  it("a Remove refused for a file changed meanwhile turns into Remove anyway", async () => {
    Object.assign(window, { __AIJ_CONNECTOR__: { [WORK]: { state: "current" } } });
    card();
    await act(async () => fireEvent.click(await screen.findByTestId("c-remove")));
    // Someone changes a file while the confirmation is open.
    resetPreviewConnector();
    Object.assign(window, { __AIJ_CONNECTOR__: { [WORK]: { state: "changed", changed: ["/Users/you/Profiles/Work/claude-code/skills/ai-juggler/.claude-plugin/plugin.json"] } } });
    await act(async () => fireEvent.click(screen.getByTestId("c-confirm-remove")));
    expect(screen.getByTestId("c-remove-anyway").textContent).toBe("Remove anyway (also deletes plugin.json)");
    expect(calls()).toEqual([]);
  });
});
