/**
 * AJ-E3206 req. 2 — one mark and one colour per App, and one per Account
 * (audit G12: one folder got two marks and three colours). Every surface
 * that draws an App's mark draws the same colour for the same App: the
 * rail's App header, its rows and their edge, an idle app, Home's tiles,
 * the session header and its App chip, the Account page's Apps, the
 * Accounts screen's rows (and the other rows naming a CLI App), the
 * folded rail and a chart by folder. Its colour is the App's stored slot,
 * so a rename or a new App never moves it. An Account's round avatar is
 * the Account's colour wherever it shows, by its own slot.
 */
import { cleanup, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { CliAppMark, PlaceMark, markPlace } from "./PlaceMarks";
import { PlaceHeader } from "./PlaceHeader";
import { edgeOf } from "./PlaceRows";
import { RailIdleApp } from "./RailIdleApp";
import { HomeApps } from "./HomeApps";
import { AccountAvatar, AccountStack } from "./Avatar";
import { FoldedRail } from "./RailFolded";
import { ActionHostProvider } from "./ItemMenu";
import { seriesColor } from "./charts/colors";
import { accountGroups } from "./usage/accounts";
import { AccountAppRows } from "../pages/subscription/AccountApps";
import { SessionMark } from "../pages/session/SessionHead";
import { AccountMenu } from "../pages/session/AccountMenu";
import { fromConversation, fromInstance } from "../pages/session/model";
import { ConversationsProvider } from "../lib/conversations";
import { accountColor, appColor, forgetMarksForTests, learnFolders, learnPlaces } from "../lib/marks";
import { appsOf } from "../lib/apps";
import { cliAppKey, desktopAppKey } from "../lib/app-name";
import { cliPlace, itemsOf } from "../lib/rail-items";
import { resetSeen } from "../lib/seen";
import { resetRailOrderCache } from "../lib/rail-order-store";
import { placesConversations } from "../preview/places";
import type { AccountInfo, InstanceInfo } from "../lib/bridge";
import type { KnownPlaceInfo } from "../types/generated/KnownPlaceInfo";
import type { SubscriptionInfo } from "../types/generated/SubscriptionInfo";

const NOW = new Date(2026, 10, 2, 10, 0, 0).getTime();
type Seams = { __AIJ_FIXTURE__?: unknown; __AIJ_PLACES__?: unknown };
const seams = () => window as unknown as Seams;

const folder = (id: string, label: string, app: number, account: number | null): AccountInfo => ({
  id,
  provider: "claude-code",
  label,
  plan: null,
  login_state: "logged_in",
  cleanup_opt_in: false,
  renewal_date: null,
  preview_opt_in: false,
  in_place: false,
  app_colour_slot: app,
  subscription_id: account === null ? null : "acct:anthropic:work",
  account_colour_slot: account,
});

/** Two folders on one Account; their CLI Apps slots 5 and 3, the Account 2.
 * Slot order is the backend's (first stored first), never the names'. */
const WORK = folder("claude-work", "Work Max", 5, 2);
const HOME = folder("claude-personal", "Personal", 3, 2);

const WORK_APP: KnownPlaceInfo = {
  id: "app:claude-app:/Users/you/Profiles/Work",
  app_id: "claude-app",
  app: "Claude",
  instance: "Work",
  profile: "Work Max",
  label: "Claude · Work Max",
  app_pid: 4100,
  keep_alive: false,
  paused: null,
  account_id: "claude-work",
  seen_ms: NOW,
  cannot_start: null,
  recent: [],
  found: false,
  colour_slot: 6,
};
const LAB_APP: KnownPlaceInfo = { ...WORK_APP, id: "app:claude-app:/Users/you/Profiles/Lab", instance: "Lab", profile: "Lab", label: "Claude · Lab", app_pid: null, account_id: null, colour_slot: 1 };

const ACCOUNT: SubscriptionInfo = {
  key: "k-work",
  account_id: "acct:anthropic:work",
  display_name: "Claude Max",
  colour_slot: 2,
  tool: "claude-code",
  state: "signed_in",
  email: "w…@example.com",
  organisation: null,
  reason: null,
  profiles: [WORK, HOME].map((row) => ({ id: row.id, label: row.label, folder: `~/.claude-${row.id}`, plan: null, renewal_date: null, fee_usd: null })),
  plan: "max",
  renewal_date: null,
  fee_usd: null,
  differs: [],
  choices: [],
};

const colorOf = (element: Element) => element.getAttribute("data-color");
const badgeIn = (element: HTMLElement) => colorOf(within(element).getByTestId("profile-badge"));

beforeEach(() => {
  forgetMarksForTests();
  resetSeen();
  resetRailOrderCache();
  learnFolders([WORK, HOME]);
  learnPlaces([WORK_APP, LAB_APP]);
});
afterEach(() => {
  cleanup();
  localStorage.clear();
  delete seams().__AIJ_FIXTURE__;
  delete seams().__AIJ_PLACES__;
});

describe("AJ-E3206 a CLI App wears one colour on every surface", () => {
  const work = appColor(5);
  const place = cliPlace("claude-code", WORK.id, WORK.label, { name: "Terminal", kind: "terminal", pid: 5100 });

  it("the rail's row mark, its App header, its rows' edge and an idle app", () => {
    expect(place.key).toBe(cliAppKey("claude-code", WORK.id, WORK.label));
    const row = render(<PlaceMark place={place} />);
    expect(badgeIn(row.container)).toBe(work);
    cleanup();
    const header = render(<PlaceHeader place={place} label="Claude Code" count={2} needs={0} folded={null} onToggle={() => {}} />);
    expect(badgeIn(header.container)).toBe(work);
    cleanup();
    expect(edgeOf(place)).toBe(work);
    const idle = render(
      <RailIdleApp
        app={{ key: place.key, kind: "cli", name: "Claude Code · Work Max", app: "Claude Code", profile: WORK.label, appId: null, lastMs: NOW, appPid: null, placeId: null, cannotStart: null, accountId: WORK.id, tool: "claude-code", sessions: [] }}
        open={false}
        onToggle={() => {}}
        now={NOW}
      />,
    );
    expect(badgeIn(idle.container)).toBe(work);
  });

  it("the Accounts screen's row (and every row naming a CLI App), the Account page's Apps, and a chart by folder", () => {
    const mark = render(<CliAppMark tool="claude-code" folderId={WORK.id} folder={WORK.label} />);
    expect(badgeIn(mark.container)).toBe(work);
    cleanup();
    const [app] = appsOf({ stored: [], places: [], folders: [WORK] });
    expect(app!.slot).toBe(5);
    const rows = render(<AccountAppRows apps={[{ app: app!, sessions: 1, open: 0, lastMs: NOW }]} now={NOW} />);
    expect(badgeIn(rows.container)).toBe(work);
    expect(seriesColor("profile", WORK.id)).toBe(work);
  });

  it("the session header's mark and its App chip", () => {
    const session = { id: "s-1", provider: "claude-code", account_id: WORK.id, account_label: WORK.label, status: "running", kind: "terminal", started_at_ms: NOW, last_activity_ms: NOW, conversation_id: null, cwd: null, model: null } as unknown as InstanceInfo;
    const model = fromInstance(session);
    const head = render(<SessionMark model={model} />);
    expect(colorOf(head.getByTestId("session-mark-profile").querySelector("[data-testid=profile-badge]")!)).toBe(work);
    cleanup();
    const chip = render(<AccountMenu model={model} account={ACCOUNT} />);
    const button = chip.getByTestId("session-account-menu");
    expect(badgeIn(button)).toBe(work);
    // Its dot alone wears the colour: a coloured border made it a pill like
    // the state's beside it (the 0.4 review).
    expect(button.style.borderColor).toBe("");
  });

  it("a rename, another App or a reload never moves it; another App is another colour", () => {
    learnFolders([{ ...WORK, label: "Aardvark" }, HOME, folder("claude-new", "A new one", 4, null)]);
    expect(appColor(cliPlace("claude-code", WORK.id, "Aardvark", { name: null, kind: null, pid: null }).key)).toBe(work);
    expect(appColor(cliAppKey("claude-code", HOME.id, HOME.label))).toBe(appColor(3));
    expect(appColor(3)).not.toBe(work);
  });
});

describe("AJ-E3206 a desktop App wears one colour on every surface", () => {
  const work = appColor(6);

  it("its row mark, an idle app, the Account page's Apps and the session header", () => {
    const conversation = placesConversations(NOW).find((row) => row.places?.[0]?.instance === "Work" && row.places[0].kind === "desktop")!;
    const place = markPlace(conversation.places![0], false, conversation.tool);
    expect(place.key).toBe(desktopAppKey("claude-app", "Claude", "Work"));
    const row = render(<PlaceMark place={place} />);
    expect(badgeIn(row.container)).toBe(work);
    cleanup();
    const idle = render(
      <RailIdleApp
        app={{ key: place.key, kind: "desktop", name: "Claude app · Work Max", app: "Claude app", profile: "Work Max", appId: "claude-app", lastMs: NOW, appPid: null, placeId: WORK_APP.id, cannotStart: null, accountId: null, tool: null, sessions: [] }}
        open={false}
        onToggle={() => {}}
        now={NOW}
      />,
    );
    expect(badgeIn(idle.container)).toBe(work);
    cleanup();
    const app = appsOf({ stored: [], places: [WORK_APP], folders: [] })[0]!;
    expect(app.slot).toBe(6);
    const rows = render(<AccountAppRows apps={[{ app, sessions: 1, open: 1, lastMs: NOW }]} now={NOW} />);
    expect(badgeIn(rows.container)).toBe(work);
    cleanup();
    const head = render(<SessionMark model={fromConversation(conversation)} />);
    expect(colorOf(head.getByTestId("session-mark-profile").querySelector("[data-testid=profile-badge]")!)).toBe(work);
  });

  it("Home's tiles: an open App's and a remembered one's", async () => {
    const conversations = placesConversations(NOW);
    seams().__AIJ_PLACES__ = { places: [WORK_APP, LAB_APP] };
    const items = itemsOf({ sessions: [], conversations, projects: [], seen: {}, snoozed: () => false, pins: new Set(), needOf: new Map() });
    render(<HomeApps items={items} runwayOf={() => null} />);
    const open = await screen.findAllByTestId("home-app-tile");
    const tile = open.find((one) => one.dataset.place === desktopAppKey("claude-app", "Claude", "Work"))!;
    expect(badgeIn(tile)).toBe(work);
    const kept = await screen.findByTestId("home-app-closed");
    expect(kept.dataset.place).toBe(LAB_APP.id);
    expect(badgeIn(kept)).toBe(appColor(1));
  });

  it("the folded rail: an AI Juggler session's mark is its CLI App's", async () => {
    seams().__AIJ_FIXTURE__ = { conversations: [] };
    const session = { id: "s-own", provider: "claude-code", account_id: WORK.id, account_label: WORK.label, status: "running", kind: "terminal", started_at_ms: NOW, last_activity_ms: NOW, project_id: null, needs_attention: null } as unknown as InstanceInfo;
    const view = render(
      <ActionHostProvider host={{}}>
        <ConversationsProvider>
          <FoldedRail sessions={[session]} now={NOW} selectedId={null} onSelect={() => {}} />
        </ConversationsProvider>
      </ActionHostProvider>,
    );
    const mark = await view.findByTestId("folded-tool-mark");
    await waitFor(() => expect(badgeIn(mark)).toBe(appColor(5)));
  });
});

describe("AJ-E3206 an Account wears one colour, round, wherever it shows", () => {
  const account = accountColor(2);

  it("its own colour where Accounts stand alone (the Runway, Usage), by either folder on it", () => {
    render(
      <>
        <AccountAvatar account={ACCOUNT} label="Work" tone="colour" />
        <AccountAvatar account={{ folder: WORK.id }} label={WORK.label} tone="colour" />
        <AccountAvatar account={{ folder: HOME.id }} label={HOME.label} tone="colour" />
      </>,
    );
    const avatars = screen.getAllByTestId("account-avatar");
    expect(avatars).toHaveLength(3);
    for (const avatar of avatars) {
      expect(colorOf(avatar)).toBe(account);
      expect(avatar.className).toContain("rounded-full");
    }
    expect(accountGroups([ACCOUNT], new Map()).get(WORK.id)?.color).toBe(account);
  });

  it("neutral beside an App (a row, a stack, the rail): a row wears one colour, its App's", () => {
    render(
      <>
        <AccountAvatar account={{ folder: WORK.id }} label={WORK.label} />
        <AccountAvatar account={{ folder: WORK.id }} label={WORK.label} size={28} />
        <AccountStack accounts={[{ id: WORK.id, label: WORK.label, live: false }]} />
      </>,
    );
    const [ring, large, stacked] = screen.getAllByTestId("account-avatar");
    for (const avatar of [ring!, large!, stacked!]) {
      expect(avatar.dataset.tone).toBe("neutral");
      expect(avatar.style.background).toBe("");
      expect(avatar.dataset.color).toBeUndefined();
    }
    // Small: a ring; large: grey with its initials.
    expect(ring!.className).toContain("border-muted");
    expect(large!.textContent).not.toBe("");
  });

  it("is told apart from its Apps' marks: another shape, and its own slot", () => {
    expect(account).not.toBe(appColor(5));
    render(<CliAppMark tool="claude-code" folderId={WORK.id} folder={WORK.label} />);
    expect(screen.queryByTestId("account-avatar")).toBeNull();
    expect(screen.getByTestId("profile-badge").dataset.variant).toBe("dot");
  });

  it("a folder whose Account isn't known gets a colour made from its id, the same each time", () => {
    const loose = accountColor({ folder: "claude-loose" });
    forgetMarksForTests();
    expect(accountColor({ folder: "claude-loose" })).toBe(loose);
    // Not stored yet: an Account row without a slot falls back by its id.
    expect(accountColor({ ...ACCOUNT, colour_slot: null })).toBe(accountColor({ ...ACCOUNT, colour_slot: null, key: "another-read" }));
  });
});
