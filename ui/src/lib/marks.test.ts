/**
 * AJ-E3206 req. 2 — the two helpers every mark goes through: an App's
 * colour and an Account's, each its stored slot, resolved by id (never by
 * a name on screen), with a fallback made from the id for one not stored.
 */
import { afterEach, describe, expect, it } from "vitest";
import { MARK_COLOURS, slotOklch, accountColor, accountSlot, appColor, appSlot, appSlotKnown, fallbackSlot, forgetMarksForTests, learnFolders, learnPlaces } from "./marks";
import { fillSlots, previewAccountSlots, previewFolderSlots, previewPlaceSlots } from "../preview/slots";
import { cliAppKey, desktopAppKey, ownDesktopAppKey } from "./app-name";
import { deltaE } from "./oklch";
import type { AccountInfo } from "../types/generated/AccountInfo";
import type { KnownPlaceInfo } from "../types/generated/KnownPlaceInfo";
import type { SubscriptionInfo } from "../types/generated/SubscriptionInfo";
import { previewNamed } from "../preview/names";

afterEach(() => forgetMarksForTests());

const folder = (id: string, label: string, over: Partial<AccountInfo> = {}): AccountInfo => ({
  id,
  provider: "claude-code",
  label,
  plan: null,
  login_state: "logged_in",
  cleanup_opt_in: false,
  renewal_date: null,
  preview_opt_in: false,
  in_place: false,
  ...over,
});

const place = (id: string, instance: string, slot: number | null): KnownPlaceInfo => ({
  id,
  lost: false,
  app_id: "claude-app",
  app: "Claude",
  instance,
  profile: instance,
  label: `Claude · ${instance}`,
  app_pid: null,
  keep_alive: false,
  paused: null,
  account_id: null,
  seen_ms: 0,
  cannot_start: null,
  recent: [],
  found: false,
  colour_slot: slot,
});

describe("AJ-E3206 the hues", () => {
  it("each slot its own colour (as many as the backend's: tests/bundle/mark-slots.test.ts)", () => {
    const colours = MARK_COLOURS.map((_, slot) => appColor(slot));
    expect(new Set(colours).size).toBe(MARK_COLOURS.length);
    expect(appColor(MARK_COLOURS.length)).toBe(appColor(0));
    // An Account's hues start half way round: the first Account never wears
    // the first four Apps' colours (most people's first folder is both).
    expect(accountColor(0)).toBe(appColor(MARK_COLOURS.length / 2));
    for (let app = 0; app < MARK_COLOURS.length / 2; app += 1) expect(accountColor(0)).not.toBe(appColor(app));
  });
});

describe("AJ-E3206 an App's colour", () => {
  it("a desktop session AI Juggler started wears its folder's App colour until a desktop App is remembered there", () => {
    learnFolders([folder("fake-desk", "Fake Desk", { app_colour_slot: 2 })]);
    const own = ownDesktopAppKey("dev.example.fake", "Fake Desk", "fake-desk", "Fake Desk");
    expect(appSlot(own)).toBe(2);
    expect(appSlotKnown(own)).toBe(true);
    learnPlaces([{ ...place("app:fake:/p/fake", "fake", 6), app_id: "dev.example.fake", app: "Fake Desk", account_id: "fake-desk" }]);
    expect(appSlot(own)).toBe(6);
  });

  it("is its stored slot, by its id or the rail's key for it, whatever its name", () => {
    learnFolders([folder("f-1", "Work", { app_colour_slot: 4 })]);
    expect(appSlot("cli:f-1")).toBe(4);
    expect(appSlot(cliAppKey("claude-code", "f-1", "Work"))).toBe(4);
    learnFolders([folder("f-1", "Renamed", { app_colour_slot: 4 })]);
    expect(appSlot(cliAppKey("claude-code", "f-1", "Renamed"))).toBe(4);
    learnPlaces([place("app:claude-app:/p/b-work", "b-work", 7)]);
    expect(appSlot("app:claude-app:/p/b-work")).toBe(7);
    expect(appSlot(desktopAppKey("claude-app", "Claude", "b-work"))).toBe(7);
    expect(appColor("app:claude-app:/p/b-work")).toBe(appColor(7));
  });

  it("not stored yet: a fallback from its id, the same each time and never from a name", () => {
    expect(appSlot("app:claude-app:/p/new")).toBe(fallbackSlot("app:claude-app:/p/new"));
    expect(appSlot("app:claude-app:/p/new")).toBe(appSlot("app:claude-app:/p/new"));
    expect(fallbackSlot("x")).toBeLessThan(MARK_COLOURS.length);
  });

  it("not stored yet: never the colour of an App that is, nor of another guess, while a colour is free", () => {
    learnFolders(MARK_COLOURS.slice(0, 5).map((_, slot) => folder(`f-${slot}`, `F${slot}`, { app_colour_slot: slot })));
    const keys = ["app|claude-app|Work", "app|claude-app|Home", "cli|codex|x"];
    const slots = keys.map((key) => appSlot(key));
    for (const slot of slots) expect(slot).toBeGreaterThanOrEqual(5);
    // Two Apps no list coloured never share one while slots are free (the
    // 0.4 review: two Claude app groups in one purple).
    expect(new Set(slots).size).toBe(slots.length);
    expect(keys.map((key) => appSlot(key))).toEqual(slots);
    // Every colour taken: any of them, still one per App each time.
    learnFolders(MARK_COLOURS.map((_, slot) => folder(`f-${slot}`, `F${slot}`, { app_colour_slot: slot })));
    expect(appSlot("app|claude-app|Work")).toBe(appSlot("app|claude-app|Work"));
  });

  it("not stored yet: the free colour most unlike every one in use, not a near-twin", () => {
    // Blue and green taken: never the olive beside the green, whatever the id.
    for (const id of ["app|fake-desktop|folder:f", "app|x|y", "cli|codex|z", "app:claude-app:/p/n"]) {
      const slot = fallbackSlot(id, new Set([0, 1]));
      expect([0, 1, 5]).not.toContain(slot);
      const nearest = Math.min(deltaE(slotOklch(slot), slotOklch(0)), deltaE(slotOklch(slot), slotOklch(1)));
      for (const other of [2, 3, 4, 6, 7]) expect(nearest).toBeGreaterThanOrEqual(Math.min(deltaE(slotOklch(other), slotOklch(0)), deltaE(slotOklch(other), slotOklch(1))) - 1e-9);
    }
  });

  it("a desktop session AI Juggler started wears the profile last seen on its folder", () => {
    learnPlaces([{ ...place("app:claude-app:/p/old", "old", 2), account_id: "f-1", seen_ms: 1 }, { ...place("app:claude-app:/p/new", "new", 6), account_id: "f-1", seen_ms: 2 }]);
    expect(appSlot(ownDesktopAppKey("claude-app", "Claude", "f-1", "Work"))).toBe(6);
  });
});

describe("AJ-E3206 an Account's colour", () => {
  const sub = (over: Partial<SubscriptionInfo> = {}): SubscriptionInfo =>
    // Named as the backend names it (the preview's stand-in for its rule).
    previewNamed([
      {
        key: "k",
        account_id: "acct:a",
        colour_slot: 6,
        tool: "claude-code",
        state: "signed_in",
        email: null,
        organisation: null,
        reason: null,
        profiles: [],
        plan: null,
        renewal_date: null,
        fee_usd: null,
        differs: [],
        choices: [],
        ...over,
      },
    ])[0]!;

  it("is its row's slot, and the same by each folder on it", () => {
    learnFolders([folder("f-1", "Work", { subscription_id: "acct:a", account_colour_slot: 6 }), folder("f-2", "Work 2", { subscription_id: "acct:a", account_colour_slot: 6 })]);
    expect(accountSlot(sub())).toBe(6);
    expect(accountSlot({ folder: "f-1" })).toBe(6);
    expect(accountSlot({ folder: "f-2" })).toBe(6);
    expect(accountColor({ folder: "f-2" })).toBe(accountColor(sub()));
  });

  it("one not stored falls back by its id: a folder's by its Account, else by itself", () => {
    learnFolders([folder("f-1", "Work", { subscription_id: "acct:b" }), folder("f-2", "Work 2", { subscription_id: "acct:b" })]);
    expect(accountSlot({ folder: "f-1" })).toBe(accountSlot({ folder: "f-2" }));
    expect(accountSlot({ folder: "f-1" })).toBe(fallbackSlot("acct:b"));
    expect(accountSlot({ folder: "unknown" })).toBe(fallbackSlot("folder:unknown"));
    expect(accountSlot(sub({ colour_slot: null, account_id: "acct:b" }))).toBe(fallbackSlot("acct:b"));
    expect(accountSlot(sub({ colour_slot: null, account_id: null }))).toBe(fallbackSlot("k"));
  });
});

describe("AJ-E3206 the preview gives slots as the backend does", () => {
  it("the least-used slot, the lowest on a tie, after the ones taken", () => {
    const rows: { slot: number | null }[] = [{ slot: null }, { slot: 0 }, { slot: null }, { slot: null }];
    fillSlots(rows, (row) => row.slot, (row, slot) => (row.slot = slot), [1]);
    expect(rows.map((row) => row.slot)).toEqual([2, 0, 3, 4]);
    const many = Array.from({ length: 10 }, () => ({ slot: null as number | null }));
    fillSlots(many, (row) => row.slot, (row, slot) => (row.slot = slot));
    expect(many.map((row) => row.slot)).toEqual([0, 1, 2, 3, 4, 5, 6, 7, 0, 1]);
  });

  it("folders, their Accounts and the desktop Apps agree, and a slot given once stays", () => {
    const folders = [folder("a", "A"), folder("b", "B", { login_state: "logged_out" }), folder("c", "C")];
    previewFolderSlots(folders, undefined, (state) => state === "logged_in");
    expect(folders.map((row) => row.app_colour_slot)).toEqual([0, 1, 2]);
    expect(folders.map((row) => row.subscription_id)).toEqual(["acct:preview:k-a", null, "acct:preview:k-c"]);
    expect(folders.map((row) => row.account_colour_slot)).toEqual([0, null, 1]);
    const subs = previewAccountSlots([{ key: "k-c", account_id: "acct:preview:k-c", state: "signed_in" } as SubscriptionInfo], folders);
    expect(subs[0]!.colour_slot).toBe(1);
    const places = [place("app:x:/p/1", "1", null), place("app:x:/p/2", "2", null)];
    previewPlaceSlots(places, folders);
    expect(places.map((row) => row.colour_slot)).toEqual([3, 4]);
    previewPlaceSlots(places, folders);
    expect(places.map((row) => row.colour_slot)).toEqual([3, 4]);
  });
});
