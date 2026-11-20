/**
 * The 0.4.6 RC walks K3 (F-A8-1's other surfaces): a switch's targets are
 * named by the App each is reached through. A Claude app profile's folder
 * is a real target (the switch continues in that profile, or a terminal
 * on its folder, the switch's own choice), but it isn't a CLI App unless
 * the Apps list says so (`lib/apps`, D-84): the switch dialog's "Ranked
 * for you" read "Claude Code CLI · Work Max" and "Claude Code CLI · Side",
 * Apps the lab didn't have. The same targets on the session page's card
 * and in the limit dialog. The rule is `targetApp` (`lib/apps.test.ts`).
 */
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { SwitchTargetRows } from "./SwitchTargetRows";
import { LimitDialog } from "./LimitDialog";
import { targetsOf } from "../pages/session/ContinueOn";
import type { FolderTarget } from "../lib/apps";
import type { StoredAppInfo } from "../lib/ipc/apps";
import type { KnownPlaceInfo } from "../lib/ipc/places";
import type { SwitchTargetData } from "../lib/ipc/switch-targets";
import { clearSeams, instance, setSeam } from "../test/fixtures";

const NOW = Date.now();

// K3: Claude Code on "work" (Work) and "personal" (Personal), Codex, and the
// Claude app's profiles Work Max (Work) and Side (Personal) on their own folders.
const profile = (name: string, folder: string): KnownPlaceInfo => ({
  id: `app:claude-app:/p/${name}`, app_id: "claude-app", app: "Claude", instance: name, profile: name, label: `Claude · ${name}`, app_pid: null, lost: false,
  keep_alive: false, paused: null, account_id: folder, seen_ms: NOW, cannot_start: null, recent: [], found: false,
});
const PLACES = [profile("Work Max", "app-work"), profile("Side", "app-side")];
const stored = (id: string, folder_id: string, tool = "claude-code"): StoredAppInfo => ({ id, kind: id.startsWith("cli:") ? "cli" : "desktop", tool, host_app_id: null, label: null, folder_id, account_id: null, last_seen_ms: 0 });
// The backend's list: no CLI App on either profile's folder.
const STORED = [stored("cli:cc-work", "cc-work"), stored("cli:cc-personal", "cc-personal"), stored("cli:codex", "codex", "codex"), ...PLACES.map((place) => stored(place.id, place.account_id!))];

// The live app's Apps, as `useTargetApp` reads them with the backend's list.
vi.mock("../lib/apps", async (original) => {
  const real = await original<typeof import("../lib/apps")>();
  const appOf = (target: FolderTarget) => real.targetApp(target, { stored: STORED, places: PLACES, listed: true });
  return { ...real, useTargetApp: () => appOf };
});

const target = (folder: string, label: string, provider: string, extra: Partial<SwitchTargetData> = {}): SwitchTargetData => ({
  recipe_id: `r-${folder}`,
  provider,
  account_id: folder,
  account_label: label,
  badge: "Same conversation, another app · Beta",
  validated: false,
  cross_provider: provider !== "claude-code",
  drops: [],
  top_factors: [],
  factors: [],
  confidence: "computed",
  score: null,
  filtered_reason: null,
  tte_min: null,
  suggested: false,
  ...extra,
});
const TARGETS = [
  target("cc-work", "work", "claude-code"),
  target("app-work", "Work Max", "claude-code", { desktop: true }),
  target("app-side", "Side", "claude-code", { desktop: true }),
  target("codex", "default", "codex"),
];
const NAMES = ["Claude Code CLI · work", "Claude app · Work Max", "Claude app · Side", "Codex CLI · default"];

afterEach(() => {
  cleanup();
  clearSeams("__AIJ_SWITCH_TARGETS__");
});

describe("K3: a switch's targets are named by their real Apps", () => {
  it("the switch dialog's Ranked for you", () => {
    render(<SwitchTargetRows targets={TARGETS} onPick={() => {}} />);
    const rows = screen.getAllByRole("button").filter((button) => button.hasAttribute("data-row"));
    expect(rows.map((row) => NAMES.find((name) => row.textContent?.startsWith(name)) ?? row.textContent)).toEqual(NAMES);
    expect(screen.queryByText(/Claude Code CLI · (Work Max|Side)/)).toBeNull();
  });

  it("the session page's Continue in another app: the name, and the profile's own mark", async () => {
    const { targetApp } = await vi.importActual<typeof import("../lib/apps")>("../lib/apps");
    const rows = targetsOf(TARGETS, "cc-personal", (one) => targetApp(one, { stored: STORED, places: PLACES, listed: true }));
    expect(rows.map((row) => row.name)).toEqual(NAMES);
    expect(rows.map((row) => row.mark?.app ?? null)).toEqual([null, "Claude app", "Claude app", null]);
  });

  it("the limit dialog's pick", async () => {
    setSeam("__AIJ_SWITCH_TARGETS__", [target("app-work", "Work Max", "claude-code", { desktop: true, suggested: true })]);
    const blocked = instance({ id: "s-1", status: "quota_blocked", account_id: "cc-personal", account_label: "personal", started_at_ms: NOW - 3_600_000 });
    render(<LimitDialog session={blocked} sessions={[blocked]} now={NOW} onContinue={() => {}} onChooseAnother={() => {}} onParked={() => {}} onClose={() => {}} />);
    expect(await screen.findByRole("button", { name: "Continue in Claude app · Work Max" })).toBeTruthy();
    expect(screen.getByTestId("limit-continue").textContent).not.toContain("Claude Code CLI · Work Max");
  });
});
