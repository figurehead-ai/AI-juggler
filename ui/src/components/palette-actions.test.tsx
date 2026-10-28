/**
 * AJ-E2106 — ⌘K's action panel: with an item selected, its actions come
 * first (the registry's, in its order, with their icons and shortcut
 * hints), then sessions, then projects once something is typed. A greyed
 * action says why and does nothing; one that runs closes the palette.
 */
import { cleanup, fireEvent, render, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { GoToPalette, paletteSelection } from "./GoToPalette";
import { ActionHostProvider } from "./ItemMenu";
import type { ActionHost } from "../lib/actions";
import type { InstanceInfo, ProjectInfo } from "../lib/bridge";

afterEach(cleanup);

const live = (overrides: Partial<InstanceInfo> = {}): InstanceInfo =>
  ({
    id: "s1",
    provider: "claude-code",
    account_id: "a1",
    account_label: "Work Max",
    status: "running",
    kind: "terminal",
    git_branch: "login-fix",
    cwd: "/w/alpha",
    project_id: "p1",
    conversation_id: null,
    quota: null,
    label: null,
    ...overrides,
  }) as InstanceInfo;
const PROJECTS = [{ id: "p1", name: "alpha", root: "/w/alpha", missing: false } as ProjectInfo];

function open(selected: InstanceInfo | null, host: ActionHost, onClose = vi.fn()) {
  const view = render(
    <ActionHostProvider host={host}>
      <GoToPalette
        open
        onClose={onClose}
        sessions={[live({ id: "s2", kind: "desktop" })]}
        onFocusWindow={() => {}}
        onOpenSession={() => {}}
        hotkey="Alt+Space"
        onHotkeyChange={async () => {}}
        selected={paletteSelection(selected, null, null, null)}
        projects={PROJECTS}
      />
    </ActionHostProvider>,
  );
  const options = () => [...view.container.querySelectorAll("li[role='option']")];
  return { view, onClose, options, input: view.container.querySelector("input")! };
}

const appHost = (): ActionHost => ({
  openSession: vi.fn(),
  focusSession: vi.fn(),
  continueSession: vi.fn(),
  copyResumeCommand: vi.fn(),
  revealSessionProject: vi.fn(),
  stopSession: vi.fn(),
  openProjectById: vi.fn(),
});

describe("AJ-E2106 ⌘K on a selected session", () => {
  it("lists its actions first, with their shortcuts, then the sessions", () => {
    const { options, input } = open(live(), appHost());
    const ids = options().map((option) => option.getAttribute("data-testid"));
    expect(ids.slice(0, 7)).toEqual([
      "goto-row-action::focus",
      "goto-row-action::reveal-project",
      "goto-row-action::continue",
      "goto-row-action::rename",
      "goto-row-action::copy-resume",
      "goto-row-action::pin",
      "goto-row-action::stop",
    ]);
    expect(ids.at(-1)).toBe("goto-row-s2");
    // On its page already: no "Open its page".
    expect(ids).not.toContain("goto-row-action::open");
    const stop = options()[6]!;
    expect(stop.textContent).toContain("Stop this session");
    expect(stop.querySelector("kbd")?.textContent).toBe("⌘.");
    expect(stop.querySelector("svg")?.getAttribute("aria-hidden")).toBe("true");
    expect(options()[2]!.querySelector("kbd")?.textContent).toBe("⌘⇧H");
    expect(input.getAttribute("placeholder")).toBe("Act on login-fix, or search…");
  });

  it("a greyed action says why and does nothing; Enter on another runs it and closes", () => {
    const host = appHost();
    const { options, input, onClose } = open(live(), host);
    const carry = options().find((option) => option.getAttribute("data-testid") === "goto-row-action::continue")!;
    expect(carry.getAttribute("aria-disabled")).toBe("true");
    expect(carry.textContent).toContain("It has no conversation to carry over yet");
    fireEvent.click(carry);
    expect(host.continueSession).not.toHaveBeenCalled();
    expect(onClose).not.toHaveBeenCalled();
    // The first option is Go to it: Enter runs it.
    fireEvent.keyDown(input, { key: "Enter" });
    expect(host.focusSession).toHaveBeenCalledTimes(1);
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("typing narrows the actions and finds projects", () => {
    const host = appHost();
    const { options, input, view } = open(live(), host);
    fireEvent.change(input, { target: { value: "alpha" } });
    const ids = options().map((option) => option.getAttribute("data-testid"));
    expect(ids).toContain("goto-row-project::p1");
    expect(ids.some((id) => id?.startsWith("goto-row-action::"))).toBe(false);
    fireEvent.change(input, { target: { value: "stop" } });
    expect(options().map((option) => option.getAttribute("data-testid"))).toEqual(["goto-row-action::stop"]);
    fireEvent.change(input, { target: { value: "alp" } });
    fireEvent.click(within(view.container).getByText("/w/alpha").closest("li")!);
    expect(host.openProjectById).toHaveBeenCalledWith("p1");
  });

  it("an ended session has no Go to it and no Stop; nothing selected, no actions", () => {
    const ended = open(live({ status: "finished" }), appHost());
    const ids = ended.options().map((option) => option.getAttribute("data-testid"));
    expect(ids).not.toContain("goto-row-action::focus");
    expect(ids).not.toContain("goto-row-action::stop");
    cleanup();
    const none = open(null, appHost());
    expect(none.options().map((option) => option.getAttribute("data-testid"))).toEqual(["goto-row-s2"]);
    expect(none.input.getAttribute("placeholder")).toBe("Go to a session or screen, or type what to do…");
  });
});
