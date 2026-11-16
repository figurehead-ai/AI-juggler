/**
 * The 0.4.6 RC walk A8 (D-62, D-84): New session offers a CLI App's
 * launch settings only. A Claude app profile's folder has launch settings
 * too, but it is a CLI App only once a terminal session ran there or the
 * user named it (`lib/apps`); the sheet's case is
 * `components/new-session-real-apps.test.tsx`.
 */
import { describe, expect, it } from "vitest";
import { offeredRecipes } from "./new-session-choice";
import { account, recipe } from "../test/fixtures";

const cc = recipe({ id: "r-cc", account_id: "cc-work", account_label: "work" });
const profile = recipe({ id: "r-app", account_id: "app-work", account_label: "Work Max" });
const gone = recipe({ id: "r-old", account_id: "removed", account_label: "Old" });
const folders = [account({ id: "cc-work" }), account({ id: "app-work" })];

describe("the Apps New session offers", () => {
  it("a folder a Claude app profile holds, with no CLI App on it, isn't offered", () => {
    expect(offeredRecipes([cc, profile], folders, new Set(["cc-work"]))).toEqual([cc]);
  });

  it("once it is a CLI App (a terminal session ran there, or you named it), it is", () => {
    const both = [cc, profile];
    expect(offeredRecipes(both, folders, new Set(["cc-work", "app-work"]))).toBe(both);
  });

  it("before the Apps are read, and for a folder not in the list, as before", () => {
    const all = [cc, profile, gone];
    expect(offeredRecipes(all, folders, null)).toBe(all);
    expect(offeredRecipes(all, folders, new Set(["cc-work"]))).toEqual([cc, gone]);
  });
});
