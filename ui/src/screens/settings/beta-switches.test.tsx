/**
 * D-127: Settings › Switching › Beta, the one door to switches still in
 * beta: off by default, a real switch (role and name), stored as the
 * backend reads it, in plain words (never "proven").
 */
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { appSettingGet, appSettingSet } from "../../lib/ipc/settings";
import { BETA_SWITCHES_KEY, BetaSwitchesSection } from "./BetaSwitchesSection";
import { SwitchingPane } from "./SwitchingPane";

afterEach(async () => {
  cleanup();
  await appSettingSet(BETA_SWITCHES_KEY, "");
});

const stored = () => appSettingGet(BETA_SWITCHES_KEY);

describe("Settings › Switching › Beta", () => {
  it("is off by default and turns switches still in beta on and off", async () => {
    render(<BetaSwitchesSection />);
    const toggle = await screen.findByRole("switch", { name: "Show switches still in beta" });
    expect(toggle.getAttribute("aria-checked")).toBe("false");
    fireEvent.click(toggle);
    await waitFor(async () => expect(await stored()).toBe("1"));
    expect(screen.getByRole("switch", { name: "Show switches still in beta" }).getAttribute("aria-checked")).toBe("true");
    expect(screen.getByText("Switches still in beta are listed, marked Beta.")).toBeTruthy();
    fireEvent.click(screen.getByRole("switch", { name: "Show switches still in beta" }));
    await waitFor(async () => expect(await stored()).toBeNull());
  });

  it("reads a stored choice, in the Switching pane, in plain words", async () => {
    await appSettingSet(BETA_SWITCHES_KEY, "1");
    const { container } = render(<SwitchingPane />);
    const toggle = await screen.findByRole("switch", { name: "Show switches still in beta" });
    expect(toggle.getAttribute("aria-checked")).toBe("true");
    expect(container.querySelector('[data-settings-section="beta"]')).not.toBeNull();
    expect(container.textContent).not.toMatch(/proven/i);
  });

  it("is the backend's key", () => {
    expect(BETA_SWITCHES_KEY).toBe("switching.beta");
  });
});
