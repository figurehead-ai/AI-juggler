/**
 * D-97 (3): an empty Accounts page offers "Find my apps", first run's own
 * scan and found step (never a second scan), said in Account words; Add
 * an app (the tool list) stays beside it.
 */
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ProfilesScreen } from "../Profiles";

afterEach(cleanup);

describe("D-97 the empty Accounts page", () => {
  it("offers Find my apps, which opens first run's scan, and Add an app beside it", async () => {
    const onFindApps = vi.fn();
    const onAddAccounts = vi.fn();
    render(<ProfilesScreen knownAccounts={[]} knownProfiles={[]} openTerminal={() => {}} onAddAccounts={onAddAccounts} onFindApps={onFindApps} />);
    const empty = await screen.findByTestId("accounts-empty");
    expect(empty.textContent).toContain("No accounts yet");
    expect(empty.textContent).not.toContain("No apps yet");
    fireEvent.click(screen.getByTestId("accounts-find-apps"));
    expect(onFindApps).toHaveBeenCalledOnce();
    expect(onAddAccounts).not.toHaveBeenCalled();
    expect(screen.getByTestId("accounts-find-apps").textContent).toBe("Find my apps");
    fireEvent.click(screen.getByTestId("accounts-empty-add"));
    expect(onAddAccounts).toHaveBeenCalledOnce();
  });
});
