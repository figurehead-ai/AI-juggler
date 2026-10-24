/**
 * AJ-E2102 — Go to it selects a terminal's tab only when the user turned
 * that on in Settings › Desktop apps (off by default: a click never
 * raises macOS's question about controlling the terminal).
 */
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, expect, it } from "vitest";
import { DesktopAppsPane, TERMINAL_TABS_KEY } from "../screens/settings/DesktopAppsPane";

type Seams = { __AIJ_APP_SETTINGS__?: Record<string, string> };
const seams = () => window as unknown as Seams;

afterEach(() => {
  cleanup();
  delete seams().__AIJ_APP_SETTINGS__;
  localStorage.clear();
});

it("is off until turned on, and saves at once", async () => {
  render(<DesktopAppsPane />);
  const tabs = await screen.findByRole("switch", { name: "Select the right terminal tab" });
  expect(tabs.getAttribute("aria-checked")).toBe("false");
  expect(document.getElementById(tabs.getAttribute("aria-describedby")!)!.textContent).toContain("macOS asks once per terminal");
  await act(async () => {
    fireEvent.click(tabs);
  });
  await waitFor(() => expect(seams().__AIJ_APP_SETTINGS__?.[TERMINAL_TABS_KEY]).toBe("1"));
  expect(tabs.getAttribute("aria-checked")).toBe("true");
  await act(async () => {
    fireEvent.click(tabs);
  });
  await waitFor(() => expect(seams().__AIJ_APP_SETTINGS__?.[TERMINAL_TABS_KEY]).toBe(""));
});
