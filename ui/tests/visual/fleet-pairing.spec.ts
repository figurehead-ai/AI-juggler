import { expect, test, type Page } from "@playwright/test";
import { allStatuses, goTo } from "./fixtures";

/**
 * AJ-E704 quality gates — the pairing wizard end to end in the
 * production build (mock bridge; the real two-instance exchange is
 * proven in `crates/juggler-app/tests/fleet_link.rs`):
 *
 * - pairing completes keyboard-only (no mouse, no camera);
 * - the permissions step defaults to View only;
 * - a mismatched code can't complete the wizard and pins nothing;
 * - with Link off nothing can be paired.
 */

const openFleet = async (page: Page, seams: Record<string, unknown> = {}) => {
  await page.addInitScript(
    ({ fixture, extra }) => {
      const w = window as unknown as Record<string, unknown>;
      w.__AIJ_FIXTURE__ = fixture;
      w.__AIJ_FLEET_PEERS__ = [];
      Object.assign(w, extra);
      localStorage.setItem("aij.theme", "dark");
      localStorage.setItem("aij.onboarding.dismissed", "1");
      sessionStorage.setItem("aij.ready.skipped", "1");
    },
    { fixture: allStatuses, extra: seams },
  );
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto("/");
  await expect(page.locator("main")).toBeVisible();
  // ⌘⇧F opens Fleet from anywhere.
  await page.keyboard.press("ControlOrMeta+Shift+F");
  await expect(page.getByTestId("fleet-screen")).toBeVisible();
};

const calls = (page: Page) =>
  page.evaluate(() => (window as unknown as { __AIJ_FLEET_CALLS__?: string[] }).__AIJ_FLEET_CALLS__ ?? []);

test("Link off: Add and Join are disabled and say why", async ({ page }) => {
  await openFleet(page);
  await expect(page.getByRole("switch")).toHaveAttribute("aria-checked", "false");
  await expect(page.getByTestId("fleet-add")).toBeDisabled();
  await expect(page.getByTestId("fleet-join")).toBeDisabled();
  await expect(page.getByTestId("fleet-link-detail")).toContainText("isn't visible to, or reachable from");
});

test("host pairing completes keyboard-only, and View only is the default", async ({ page }) => {
  await openFleet(page);
  await page.getByRole("switch").focus();
  await page.keyboard.press("Space");
  await expect(page.getByRole("switch")).toHaveAttribute("aria-checked", "true");
  await page.getByTestId("fleet-add").focus();
  await page.keyboard.press("Enter");

  const wizard = page.getByRole("dialog", { name: "Add a machine" });
  await expect(wizard).toBeVisible();
  await expect(page.getByTestId("pairing-code")).toHaveText("482 913");
  await expect(page.getByTestId("pairing-code")).toHaveAttribute("aria-label", "Pairing code 4 8 2 9 1 3");
  // The other machine enters the code.
  await page.evaluate(() => (window as unknown as { __AIJ_PAIR_SIMULATE__: (e: string) => void }).__AIJ_PAIR_SIMULATE__("joiner"));

  await expect(page.getByTestId("pairing-verify")).toBeVisible();
  await expect(page.getByTestId("pairing-peer-fingerprint")).toHaveText("9B10 44CE 7A0F E2D3 51B8 06C9 3E7D A425");
  await expect(page.getByTestId("pairing-fingerprints-match")).toBeFocused();
  await page.keyboard.press("Enter");

  await expect(page.getByTestId("pairing-permissions")).toBeVisible();
  const viewOnly = page.getByRole("radio", { name: /View only/ });
  await expect(viewOnly).toBeChecked();
  await expect(viewOnly).toBeFocused();
  await expect(page.getByRole("radio", { name: /View \+ control/ })).not.toBeChecked();
  await expect(page.getByTestId("pairing-permission-control")).toContainText("never read credentials");
  // Back is always available: Permissions → Verify → Permissions.
  await page.keyboard.press("Tab");
  await expect(page.getByRole("button", { name: "Back" })).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(page.getByTestId("pairing-pair")).toBeFocused();
  await page.keyboard.press("Enter");

  await expect(page.getByTestId("pairing-done")).toContainText("Paired with Laptop");
  await expect(page.getByRole("button", { name: "Done" })).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(wizard).toBeHidden();
  await expect(page.getByTestId("fleet-peer")).toHaveCount(1);
  await expect(page.getByTestId("fleet-peer")).toContainText("View only");
  expect(await calls(page)).toContain("pair-host-decide:view");
});

test("a mismatched code can't complete the wizard and pins nothing", async ({ page }) => {
  await openFleet(page, {
    __AIJ_LINK__: { enabled: true, port: 47821, error: null },
    __AIJ_DISCOVERED__: [{ instance_id: "a1b2c3d4", label: "Unnamed machine (a1b2c3d4)", version: "1.0" }],
  });
  await page.getByTestId("fleet-join").click();
  const wizard = page.getByRole("dialog", { name: "Join a machine" });
  await expect(wizard).toBeVisible();
  // Discovery never shows a name: labels arrive only inside pairing.
  await expect(page.getByTestId("pairing-machine")).toHaveText(/Unnamed machine \(a1b2c3d4\)/);
  await expect(page.getByRole("radio", { name: /Unnamed machine/ })).toBeChecked();
  await expect(page.getByTestId("pairing-connect")).toBeDisabled();
  await page.getByRole("textbox", { name: "Digit 1 of 6" }).focus();
  await page.keyboard.type("000000");
  await expect(page.getByTestId("pairing-connect")).toBeEnabled();
  await page.keyboard.press("Enter");

  await expect(page.getByTestId("pairing-failure")).toHaveAttribute("data-reason", "mismatch");
  await expect(page.getByTestId("pairing-failure")).toContainText("The code didn't match");
  await expect(page.getByTestId("pairing-verify")).toHaveCount(0);
  await expect(page.getByTestId("pairing-permissions")).toHaveCount(0);
  expect((await calls(page)).some((call) => call.startsWith("pair-join-decide"))).toBe(false);
  await page.getByTestId("pairing-retry").click();
  await expect(page.getByTestId("pairing-join-find")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(wizard).toBeHidden();
  await expect(page.getByTestId("fleet-peer")).toHaveCount(0);
});

test("a pasted pairing link fills the code and is sent whole", async ({ page }) => {
  await openFleet(page, { __AIJ_LINK__: { enabled: true, port: 47821, error: null }, __AIJ_DISCOVERED__: [] });
  await page.getByTestId("fleet-join").click();
  await expect(page.getByTestId("pairing-none-found")).toContainText(
    "No machines found. Check both machines are on the same network with Link on.",
  );
  await page.getByTestId("pairing-toggle-address").click();
  await page.getByTestId("pairing-address").fill("100.64.0.2");
  const link = "aijuggler-pair:v1:482913:C2E91F7A0B368D54E1A07C2946BF3D18";
  await page.getByRole("textbox", { name: "Digit 1 of 6" }).focus();
  await page.evaluate((text) => {
    const target = document.activeElement as HTMLInputElement;
    const data = new DataTransfer();
    data.setData("text", text);
    target.dispatchEvent(new ClipboardEvent("paste", { clipboardData: data, bubbles: true, cancelable: true }));
  }, link);
  await expect(page.getByTestId("pairing-link-pasted")).toBeVisible();
  await page.getByTestId("pairing-connect").click();
  await expect(page.getByTestId("pairing-verify")).toBeVisible();
  expect(await calls(page)).toContain("pair-join:100.64.0.2");
});

// AJ-E705: a remote selection uses the same six tabs; an offline peer
// shows its stale state without a spinner deadlock; ⌘[ goes back.
test("a remote Instance fills the same six-tab inspector and offline never deadlocks", async ({ page }) => {
  const now = Date.now();
  const machine = {
    machine_id: "machine-mini", label: "Mac mini", connection: "connected", freshness: "live",
    last_heard_ms: now, permission: "view", resources: null,
    instances: [{
      id: "inst-1", provider: "claude-code", account_label: "Work Max", status: "running",
      model: null, model_source: "unknown", project_name: "web-shop", started_at_ms: now - 60_000,
      last_activity_ms: null, cpu_percent: null, rss_bytes: null, process_count: null, measured_at_ms: now,
    }],
  };
  await openFleet(page, {
    __AIJ_LINK__: { enabled: true, port: 47821, error: null },
    __AIJ_FLEET_PEERS__: [{
      machine_id: "machine-mini", label: "Mac mini", fingerprint: "11AA 22BB 33CC 44DD 55EE 66FF 7788 99AA",
      permission: "view", permission_label: "View only", ask_each_time: false, paired_via: "code",
      paired_at_ms: now, last_seen_ms: now, connection: "connected", protocol: "v1.0", connection_detail: null,
    }],
    __AIJ_FLEET_REMOTE__: [machine],
  });
  await page.getByTestId("fleet-remote-instance").click();
  const tabs = page.getByRole("tablist", { name: /inspector tabs/ }).getByRole("tab");
  await expect(tabs).toHaveText(["Overview", "Usage", "History", "Quality / Performance", "Processes", "Actions"]);
  for (const name of ["Overview", "Usage", "History", "Quality / Performance", "Processes", "Actions"]) {
    await page.getByRole("tab", { name }).click();
    await expect(page.getByRole("tabpanel", { name })).toBeVisible();
  }
  await expect(page.getByTestId("remote-stop")).toBeDisabled();
  await expect(page.getByTestId("remote-stop-reason")).toHaveText("View-only pairing — change in Fleet settings on Mac mini");
  // The peer drops off: the same view shows Offline, no spinner, no hang.
  await page.evaluate(() => {
    const w = window as unknown as { __AIJ_FLEET_REMOTE__: Array<Record<string, unknown>> };
    w.__AIJ_FLEET_REMOTE__ = w.__AIJ_FLEET_REMOTE__.map((m) => ({ ...m, connection: "offline", freshness: "offline" }));
  });
  await expect(page.getByTestId("remote-freshness")).toHaveAttribute("data-freshness", "offline", { timeout: 5000 });
  await expect(page.getByTestId("remote-freshness")).toContainText("Offline — Mac mini last seen");
  await expect(page.getByText("Loading…")).toHaveCount(0);
  await page.keyboard.press("ControlOrMeta+BracketLeft");
  await expect(page.getByTestId("fleet-screen")).toBeVisible();
});

// AJ-E709: the machine filter persists across restarts; a Fleet alert
// deep-links to that machine's inspector; access changes only on this Mac.
test("machine filter persists, a Fleet alert opens its machine, access is changed here", async ({ page }) => {
  const now = Date.now();
  const peer = {
    machine_id: "machine-mini", label: "Mac mini", fingerprint: "11AA 22BB 33CC 44DD 55EE 66FF 7788 99AA",
    permission: "view", permission_label: "View only", ask_each_time: false, paired_via: "code",
    paired_at_ms: now, last_seen_ms: now, connection: "connected", protocol: "v1.0", connection_detail: null,
  };
  const machine = {
    machine_id: "machine-mini", label: "Mac mini", connection: "connected", freshness: "live",
    last_heard_ms: now, permission: "view", resources: null,
    instances: [{
      id: "inst-1", provider: "claude-code", account_label: "Work Max", status: "waiting_user",
      model: null, model_source: "unknown", project_name: "web-shop", started_at_ms: now - 60_000,
      last_activity_ms: null, cpu_percent: null, rss_bytes: null, process_count: null, measured_at_ms: now,
    }],
    alerts: [{ kind: "exhausted", account: "Work Max", text: "Work Max hit its 5-hour limit.", at_ms: now }],
  };
  await openFleet(page, { __AIJ_LINK__: { enabled: true, port: 47821, error: null }, __AIJ_FLEET_PEERS__: [peer], __AIJ_FLEET_REMOTE__: [machine] });

  // Access: changed in a sheet on this Mac (Cancel focused first).
  await page.getByTestId("fleet-access").click();
  await expect(page.getByRole("dialog", { name: "What may Mac mini do on this Mac?" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Cancel" })).toBeFocused();
  await page.getByTestId("fleet-access-ask").click();
  await page.getByTestId("fleet-access-save").click();
  await expect(page.getByTestId("fleet-peer")).toContainText("asks each time");
  expect(await calls(page)).toContain("permission:machine-mini:control:true");

  // The board: All machines by default; filter to Mac mini; it persists.
  await goTo(page, "Fleet");
  await expect(page.getByTestId("instance-board")).toBeVisible(); // D-25: one rail, always the list
  await expect(page.getByTestId("board-remote-row")).toHaveAccessibleName(/^Work Max on Mac mini.*waiting user/);
  await page.getByLabel("Filter by machine").selectOption("machine-mini");
  await page.reload();
  await expect(page.getByTestId("instance-board")).toBeVisible(); // D-25: one rail, always the list
  await expect(page.getByLabel("Filter by machine")).toHaveValue("machine-mini");
  await expect(page.getByTestId("board-machine-local")).toHaveCount(0);

  // The alert names the machine and opens its inspector.
  await expect(page.getByTestId("remote-alert-exhausted")).toContainText("Mac mini");
  await page.getByTestId("remote-alert-exhausted").getByRole("button", { name: "Open" }).click();
  await expect(page.getByTestId("remote-machine-overview")).toBeVisible();
});
