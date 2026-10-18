import { expect, test, type Page } from "@playwright/test";
import { allStatuses } from "./fixtures";

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
