/**
 * D-62 (AJ-E3203): on a Claude app profile's page, "Find this app's
 * other profiles" asks once, naming the folder it reads; a yes finds the
 * profiles there (each an App with Start), a no is kept, and Settings ›
 * Desktop apps turns it off again, forgetting what it found.
 */
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { AppPage } from "./AppPage";
import { ActionHostProvider } from "../../components/ItemMenu";
import { ConversationsProvider } from "../../lib/conversations";
import { foundProfilesPreview, knownPlacesPreview, placesConversations } from "../../preview/places";
import { DesktopAppsPane } from "../../screens/settings/DesktopAppsPane";
import type { AccountInfo } from "../../lib/bridge";

const NOW = Date.now();
const WORK = "app:claude-app:/Users/you/Profiles/Work";
const DESIGN = "app:claude-app:/Users/you/Profiles/Design";
const seams = () => window as unknown as Record<string, unknown>;
const folders = [{ id: "claude-work", label: "Work Max", provider: "claude-code", login_state: "logged_in", renewal_date: null, cleanup_opt_in: false, preview_opt_in: false } as AccountInfo];

afterEach(() => {
  cleanup();
  delete seams().__AIJ_FIXTURE__;
  delete seams().__AIJ_PLACES__;
  localStorage.clear();
});

function seed() {
  seams().__AIJ_FIXTURE__ = { conversations: placesConversations(NOW) };
  seams().__AIJ_PLACES__ = { ...knownPlacesPreview(NOW), profiles: foundProfilesPreview(NOW) };
}

function page(id: string) {
  return render(
    <ConversationsProvider>
      <ActionHostProvider host={{}}>
        <AppPage id={id} sessions={[]} projects={[]} accounts={folders} now={NOW} onOpenSession={() => {}} onOpenProject={() => {}} />
      </ActionHostProvider>
    </ConversationsProvider>,
  );
}

describe("AJ-E3203 Claude app profiles found when they aren't running", () => {
  it("asks once on the App page, naming the folder; a yes makes each profile there an App with Start", async () => {
    seed();
    page(WORK);
    const ask = await screen.findByTestId("app-find-profiles");
    expect(ask.textContent).toContain("Find this app's other profiles");
    expect(ask.textContent).toContain("~/Profiles");
    expect(ask.textContent).toContain("and nothing inside them");
    fireEvent.click(screen.getByTestId("app-find-profiles-yes"));
    expect((await screen.findByTestId("app-find-profiles-done")).textContent).toContain("Found 2 more profiles: Design, Research");
    cleanup();
    // Remembered: no ask again; the found profile has its own page, with Start.
    page(DESIGN);
    expect(await screen.findByTestId("page-name")).toHaveProperty("textContent", "Claude app · Design");
    expect(screen.getByTestId("app-go").textContent).toBe("Start");
    expect(screen.getByTestId("page-facts").textContent).toContain("Found with your other profiles");
    expect(screen.queryByTestId("app-find-profiles")).toBeNull();
  });

  it("a no is kept, and nothing is read", async () => {
    seed();
    page(WORK);
    fireEvent.click(await screen.findByTestId("app-find-profiles-no"));
    await waitFor(() => expect(screen.queryByTestId("app-find-profiles")).toBeNull());
    const host = seams().__AIJ_PLACES__ as { places: { id: string }[]; profiles: { answer: string } };
    expect(host.profiles.answer).toBe("off");
    expect(host.places.some((place) => place.id === DESIGN)).toBe(false);
  });

  it("Settings › Desktop apps turns it off, and the profiles found go", async () => {
    seed();
    page(WORK);
    fireEvent.click(await screen.findByTestId("app-find-profiles-yes"));
    await screen.findByTestId("app-find-profiles-done");
    cleanup();
    render(<DesktopAppsPane />);
    const row = await screen.findByTestId("desktop-find-profiles");
    expect(row.textContent).toContain("~/Profiles");
    expect(await screen.findByText("Claude app · Design")).toBeTruthy();
    fireEvent.click(row.querySelector("[role='switch']")!);
    await waitFor(() => expect(screen.queryByText("Claude app · Design")).toBeNull());
    const host = seams().__AIJ_PLACES__ as { profiles: { answer: string } };
    expect(host.profiles.answer).toBe("off");
  });
});
