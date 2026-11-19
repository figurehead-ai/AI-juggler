/**
 * The 0.4.6 RC walk P8: OpenCode's and Kimi Code's sign-ins both read
 * "default needs its sign-in" in Needs you; only the grey line under each
 * told them apart. A sign-in's title stays short by its label, unless
 * another sign-in reads the same: then each of those names its App.
 */
import { describe, expect, it } from "vitest";
import { homeNeeds } from "./home-needs";
import type { AccountInfo } from "../types/generated/AccountInfo";

const folder = (id: string, provider: string, label: string): AccountInfo => ({
  id, provider, label, plan: null, login_state: "logged_out", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false,
});

const titles = (signIns: AccountInfo[]) =>
  homeNeeds({ items: [], signIns, signInDetail: () => "", mac: null, heavy: [], leftovers: 0 }).map((need) => need.title);

describe("sign-in needs never read the same", () => {
  it("two Apps' sign-ins with one label each name their App", () => {
    const shown = titles([folder("oc", "opencode", "default"), folder("kimi", "kimi-code", "default"), folder("w", "claude-code", "Work Max")]);
    expect(new Set(shown).size).toBe(3);
    expect(shown.filter((title) => title === "default needs its sign-in")).toEqual([]);
    expect(shown.every((title) => title.endsWith(" needs its sign-in"))).toBe(true);
    // The one whose label is its own stays short.
    expect(shown).toContain("Work Max needs its sign-in");
  });

  it("one sign-in alone stays short", () => {
    expect(titles([folder("oc", "opencode", "default")])).toEqual(["default needs its sign-in"]);
  });
});
