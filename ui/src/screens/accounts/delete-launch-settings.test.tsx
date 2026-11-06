/**
 * D-60 (0.3.1): deleting launch settings asks first, says their sessions
 * stay in History, and shows the backend's refusal (an open session, by
 * name) in its own words instead of deleting.
 */
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { DeleteLaunchSettingsModal } from "./DeleteLaunchSettings";
import { ToastHost, resetToasts } from "../../components/Toast";
import type { LaunchRecipeInfo } from "../../types/generated/LaunchRecipeInfo";
import profileRowSource from "./ProfileRow.tsx?raw";
import { account, asButton, instance } from "../../test/fixtures";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";

const RECIPE = { id: "rec-1", name: "Deep work", account_id: "acc-1" } as unknown as LaunchRecipeInfo;

afterEach(cleanup);

describe("deleting launch settings", () => {
  it("asks first and says the sessions made with them stay", async () => {
    const onDelete = vi.fn(async () => undefined);
    const onClose = vi.fn();
    resetToasts();
    render(
      <>
        <DeleteLaunchSettingsModal recipe={RECIPE} onClose={onClose} onDelete={onDelete} />
        <ToastHost />
      </>,
    );
    expect(screen.getByTestId("delete-launch-settings").textContent).toContain("Sessions already made with it stay in History.");
    expect(onDelete).not.toHaveBeenCalled();
    fireEvent.click(screen.getByTestId("delete-launch-settings-confirm"));
    await waitFor(() => expect(onClose).toHaveBeenCalled());
    expect(onDelete).toHaveBeenCalledWith("rec-1");
    // AJ-E2907 Req.5: said once it's done.
    await waitFor(() => expect(screen.getByText("Deleted “Deep work”.")).toBeTruthy());
    resetToasts();
  });

  it("shows a refusal in the backend's words and keeps the settings", async () => {
    const words = "«Deep work» has a session open: api. Stop it first, then delete the launch settings";
    const onClose = vi.fn();
    render(<DeleteLaunchSettingsModal recipe={RECIPE} onClose={onClose} onDelete={async () => Promise.reject(new Error(words))} />);
    fireEvent.click(screen.getByTestId("delete-launch-settings-confirm"));
    expect((await screen.findByTestId("delete-launch-settings-error")).textContent).toContain("Stop it first");
    expect(screen.queryByTestId("delete-launch-settings-confirm")).toBeNull();
    expect(onClose).not.toHaveBeenCalled();
  });

  it("the row's delete button opens the confirm, never deletes on the click", () => {
    expect(profileRowSource).toContain("onClick={() => setDeleting(recipe)}");
    expect(profileRowSource).not.toMatch(/onClick=\{\(\) => void props\.onDeleteRecipe/);
    // AJ-E2907 Req.4: the app's own delete icon, not a drawn one.
    expect(profileRowSource).toContain('<ActionIcon name="delete" size={13} />');
  });
});

describe("AJ-E2907 R1 and R4 — the confirm, from the row's bin", () => {
  const ACCOUNT = account({ id: "acc-1", label: "Work", service: null });
  async function openRow(onDeleteRecipe = vi.fn(async () => undefined)) {
    const { ProfileRow } = await import("./ProfileRow");
    const view = render(
      <ul>
        <ProfileRow account={ACCOUNT} subscription={undefined} recipes={[RECIPE]} openNow={0} apps={[]} canUseInPlace={false} reportsLimits={false} now={Date.now()} onLaunch={vi.fn()} onDeleteRecipe={onDeleteRecipe} onNewRecipe={vi.fn()} onLogin={vi.fn()} onRechecked={async () => undefined} />
      </ul>,
    );
    fireEvent.contextMenu(screen.getByTestId("profile-row").firstElementChild!, { clientX: 10, clientY: 10 });
    fireEvent.click(await screen.findByRole("menuitem", { name: /Edit launch settings/ }));
    return { view, onDeleteRecipe, bin: await screen.findByTestId("recipe-delete") };
  }

  it("R4: the bin is the app's delete icon and keeps its name", async () => {
    const { bin } = await openRow();
    expect(bin.getAttribute("aria-label")).toBe("Delete Deep work");
    expect(screen.getByRole("button", { name: "Delete Deep work" })).toBe(bin);
    expect(bin.querySelector("svg")).not.toBeNull();
  });

  it("R1: “Delete the launch settings “Deep work”?”, Cancel focused, Esc cancels, focus back on the bin", async () => {
    const { bin, onDeleteRecipe } = await openRow();
    bin.focus();
    fireEvent.click(bin);
    const dialog = await screen.findByRole("dialog", { name: "Delete the launch settings “Deep work”?" });
    expect(dialog.getAttribute("aria-modal")).toBe("true");
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Cancel" }));
    expect(screen.getByTestId("delete-launch-settings-confirm").className).toMatch(/danger/);
    fireEvent.keyDown(dialog, { key: "Escape" });
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    expect(onDeleteRecipe).not.toHaveBeenCalled();
    expect(document.activeElement).toBe(screen.getByTestId("recipe-delete"));
    // Cancel does the same.
    fireEvent.click(screen.getByTestId("recipe-delete"));
    fireEvent.click(await screen.findByRole("button", { name: "Cancel" }));
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    expect(onDeleteRecipe).not.toHaveBeenCalled();
  });
});

describe("AJ-E2907 R3 — refused in the window before the click while a session runs", () => {
  const ACCOUNT = account({ id: "acc-1", label: "Work", service: null });

  it("greys the bin with why while one of its sessions is live, and counts AI Juggler's live ones per settings", async () => {
    const { ProfileRow, liveWords } = await import("./ProfileRow");
    const { liveByRecipe } = await import("./AccountCard");
    const onDeleteRecipe = vi.fn(async () => undefined);
    render(
      <ul>
        <ProfileRow account={ACCOUNT} subscription={undefined} recipes={[RECIPE]} openNow={1} liveByRecipe={{ "rec-1": 1 }} apps={[]} canUseInPlace={false} reportsLimits={false} now={Date.now()} onLaunch={vi.fn()} onDeleteRecipe={onDeleteRecipe} onNewRecipe={vi.fn()} onLogin={vi.fn()} onRechecked={async () => undefined} />
      </ul>,
    );
    fireEvent.contextMenu(screen.getByTestId("profile-row").firstElementChild!, { clientX: 10, clientY: 10 });
    fireEvent.click(await screen.findByRole("menuitem", { name: /Edit launch settings/ }));
    const bin = await screen.findByTestId("recipe-delete");
    expect(asButton(bin).disabled).toBe(true);
    expect(bin.getAttribute("title")).toBe("1 live session uses these launch settings — stop it first.");
    fireEvent.click(bin);
    expect(screen.queryByTestId("delete-launch-settings")).toBeNull();
    expect(onDeleteRecipe).not.toHaveBeenCalled();
    expect(liveWords(2)).toBe("2 live sessions use these launch settings — stop them first.");
    expect(liveWords(0)).toBeNull();
    const at = (status: InstanceInfo["status"], recipe: string) => instance({ status, launch_recipe_id: recipe });
    expect(liveByRecipe([at("running", "rec-1"), at("waiting_user", "rec-1"), at("finished", "rec-1"), at("starting", "rec-2")])).toEqual({ "rec-1": 2, "rec-2": 1 });
  });
});
