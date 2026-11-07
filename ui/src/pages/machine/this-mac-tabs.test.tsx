/**
 * AJ-E3104: This Mac's page leads to everything on it: Overview, then
 * Sessions, Apps, Projects and Pools, each counted as its list holds.
 */
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { ThisMacPage } from "./ThisMacPage";
import { ActionHostProvider } from "../../components/ItemMenu";
import { ConversationsProvider } from "../../lib/conversations";
import { knownPlacesPreview, placesConversations } from "../../preview/places";
import type { AccountInfo } from "../../types/generated/AccountInfo";

const NOW = Date.now();
const seams = () => window as unknown as Record<string, unknown>;
const folders = [
  { id: "claude-work", label: "Work Max", provider: "claude-code", plan: "max-20x", login_state: "logged_in" } as AccountInfo,
  { id: "claude-personal", label: "Personal", provider: "claude-code", plan: null, login_state: "logged_in" } as AccountInfo,
];

afterEach(() => {
  cleanup();
  delete seams().__AIJ_FIXTURE__;
  delete seams().__AIJ_PLACES__;
  localStorage.clear();
});

function page() {
  seams().__AIJ_FIXTURE__ = { conversations: placesConversations(NOW) };
  seams().__AIJ_PLACES__ = knownPlacesPreview(NOW);
  return render(
    <ConversationsProvider>
      <ActionHostProvider host={{}}>
        <ThisMacPage sessions={[]} projects={[]} accounts={folders} now={NOW} onOpenSession={() => {}} onOpenProject={() => {}} />
      </ActionHostProvider>
    </ConversationsProvider>,
  );
}

describe("AJ-E3104 This Mac's page", () => {
  it("opens on its Overview, then Sessions, Apps, Projects, Pools and Processes (AJ-E3104 R1)", async () => {
    page();
    expect((await screen.findByTestId("page-name")).textContent).toBe("This Mac");
    const tabs = screen.getAllByRole("tab").map((tab) => tab.textContent ?? "");
    expect(tabs.map((label) => label.replace(/\d+$/, "").trim())).toEqual(["Overview", "Sessions", "Apps", "Projects", "Pools", "Processes"]);
    expect(screen.getByTestId("this-mac-overview")).toBeTruthy();
    // Processes: every process here, in the Processes screen's own view.
    fireEvent.click(screen.getAllByRole("tab").find((tab) => tab.textContent === "Processes")!);
    expect(await screen.findByTestId("this-mac-processes")).toBeTruthy();
  });

  it("counts its Apps as the Apps tab lists them", async () => {
    page();
    const apps = await waitFor(() => {
      const tab = screen.getAllByRole("tab").find((one) => one.textContent?.startsWith("Apps"));
      expect(tab?.textContent).toMatch(/\d+$/);
      return tab!;
    });
    const count = Number(apps.textContent!.match(/(\d+)$/)![1]);
    fireEvent.click(apps);
    const list = await screen.findByTestId("this-mac-apps-tab");
    await waitFor(() => expect(within(list).getAllByTestId("account-app")).toHaveLength(count));
  });
});
