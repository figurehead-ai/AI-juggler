/**
 * The 0.4.6 RC walk A8 (F-A8-2): a Rename sheet opens with the name
 * selected, so typing replaces it. An Account named "Work", renamed by
 * typing "Acme", was saved as "WorkAcme": the field opened focused with
 * the caret after the name. One sheet for sessions, projects, Apps and
 * Accounts (`RenameSheet`); it selects once, on open, never again on
 * focus.
 */
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { RenameSheet } from "./ManageWork";
import { RenameAccountSheet } from "../pages/subscription/AccountSettings";
import * as subscriptionsModule from "../lib/ipc/subscriptions";
import { asInput, subscription } from "../test/fixtures";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

const field = (id: string) => asInput(screen.getByTestId(id));

describe("F-A8-2 a Rename sheet opens with the name selected", () => {
  it("an Account's: typing replaces its name, and Save names it that", async () => {
    const rename = vi.spyOn(subscriptionsModule, "subscriptionsRename").mockResolvedValue();
    const done = vi.fn();
    render(<RenameAccountSheet sub={subscription({ account_id: "acct:work", name: "Work" })} onClose={() => {}} onDone={done} />);
    const name = field("account-rename-sheet-name");
    expect(document.activeElement).toBe(name);
    expect([name.selectionStart, name.selectionEnd]).toEqual([0, "Work".length]);
    const user = userEvent.setup();
    await user.keyboard("Acme");
    expect(name.value).toBe("Acme");
    await user.click(screen.getByTestId("account-rename-sheet-save"));
    await waitFor(() => expect(rename).toHaveBeenCalledWith("acct:work", "Acme"));
    expect(done).toHaveBeenCalled();
  });

  it("selected once, on open: focus coming back keeps the caret where the user left it", async () => {
    render(<RenameSheet title="Rename web" initial="web" placeholder="web" hint="" testId="rename-project" onSave={async () => {}} onClose={() => {}} />);
    const name = field("rename-project-name");
    expect([name.selectionStart, name.selectionEnd]).toEqual([0, 3]);
    const user = userEvent.setup();
    await user.keyboard("{ArrowRight}-shop");
    expect(name.value).toBe("web-shop");
    name.blur();
    name.focus();
    expect(name.selectionStart).toBe(name.selectionEnd);
  });

  it("an empty name (the default shown as its placeholder) has nothing to select, and typing names it", async () => {
    render(<RenameSheet title="Rename Claude Code CLI · work" initial="" placeholder="Claude Code CLI · work" hint="" testId="app-rename-sheet" onSave={async () => {}} onClose={() => {}} />);
    const name = field("app-rename-sheet-name");
    expect(document.activeElement).toBe(name);
    await userEvent.setup().keyboard("Day job");
    expect(name.value).toBe("Day job");
  });
});
