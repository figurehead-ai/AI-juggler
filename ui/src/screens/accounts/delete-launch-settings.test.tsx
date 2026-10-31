/**
 * D-60 (0.3.1): deleting launch settings asks first, says their sessions
 * stay in History, and shows the backend's refusal (an open session, by
 * name) in its own words instead of deleting.
 */
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { DeleteLaunchSettingsModal } from "./DeleteLaunchSettings";
import type { LaunchRecipeInfo } from "../../lib/bridge";
import profileRowSource from "./ProfileRow.tsx?raw";

const RECIPE = { id: "rec-1", name: "Deep work", account_id: "acc-1" } as unknown as LaunchRecipeInfo;

afterEach(cleanup);

describe("deleting launch settings", () => {
  it("asks first and says the sessions made with them stay", async () => {
    const onDelete = vi.fn(async () => undefined);
    const onClose = vi.fn();
    render(<DeleteLaunchSettingsModal recipe={RECIPE} onClose={onClose} onDelete={onDelete} />);
    expect(screen.getByTestId("delete-launch-settings").textContent).toContain("Sessions already made with it stay in History.");
    expect(onDelete).not.toHaveBeenCalled();
    fireEvent.click(screen.getByTestId("delete-launch-settings-confirm"));
    await waitFor(() => expect(onClose).toHaveBeenCalled());
    expect(onDelete).toHaveBeenCalledWith("rec-1");
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
  });
});
