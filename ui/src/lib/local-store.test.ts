/**
 * rc32: what the window remembers is its data folder's. A lab's Home
 * listed another HOME's scan ("Kimi Code is installed"), and its Account
 * pages opened on the tab another lab left (Settings).
 */
import { afterEach, describe, expect, it } from "vitest";

import { rememberedTabId, rememberTab } from "../pages/registry";
import { rememberedFound, rememberFound } from "./found-tools";
import type { ScanReportData } from "./ipc/providers";
import { scopedKey } from "./local-store";

const KIMI_ONLY: ScanReportData = { installs: [], probes: [], seeds: [], desktop_apps: [], not_set_up: ["kimi-code"], cancelled: false };

/** The window as the backend starts it on a data folder's scope. */
function inFolder(scope: string | undefined): void {
  if (scope === undefined) Reflect.deleteProperty(window, "__AIJ_STORE_SCOPE__");
  else Reflect.set(window, "__AIJ_STORE_SCOPE__", scope);
}

afterEach(() => {
  inFolder(undefined);
  localStorage.clear();
});

describe("the window's storage, per data folder (rc32)", () => {
  it("one lab's remembered tab and scan aren't another's", () => {
    inFolder("aaaaaaaaaaaa");
    rememberTab("subscription", "settings");
    rememberFound(KIMI_ONLY);
    expect(rememberedTabId("subscription")).toBe("settings");
    expect(rememberedFound()).toEqual([{ tool: "kimi-code", setUp: false }]);

    inFolder("bbbbbbbbbbbb");
    expect(rememberedTabId("subscription")).toBeNull();
    expect(rememberedFound()).toEqual([]);
  });

  it("the user's own data folder keeps its keys as they always were", () => {
    inFolder("");
    expect(scopedKey("aij.page.tab.app")).toBe("aij.page.tab.app");
    inFolder(undefined);
    rememberTab("app", "sessions");
    expect(localStorage.getItem("aij.page.tab.app")).toBe("sessions");
    inFolder("aaaaaaaaaaaa");
    expect(scopedKey("aij.page.tab.app")).toBe("aij.page.tab.app@aaaaaaaaaaaa");
  });
});
