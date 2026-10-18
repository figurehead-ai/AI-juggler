/**
 * AJ-E702 vitest — the Fleet screen: this Mac's fingerprint (labelled,
 * copyable), local rename, the paired list with a confirmed Unpair
 * that names the machine (Cancel focused by default), Reset identity,
 * and the fail-closed "Trust broken" banner with an explicit reset.
 */
import { act, fireEvent, render, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";
import { FleetScreen } from "../screens/FleetScreen";

const w = () => window as unknown as Record<string, unknown>;

const PEER = {
  machine_id: "machine-mini",
  label: "Mac mini",
  fingerprint: "11AA 22BB 33CC 44DD 55EE 66FF 7788 99AA",
  permission: "view",
  permission_label: "View only",
  ask_each_time: false,
  paired_via: "code",
  paired_at_ms: 1,
  last_seen_ms: null,
};

beforeEach(() => {
  delete w().__AIJ_FLEET_SELF__;
  delete w().__AIJ_FLEET_CALLS__;
  w().__AIJ_FLEET_PEERS__ = [];
});

describe("AJ-E702 Fleet screen", () => {
  it("shows this Mac's fingerprint as labelled, focusable monospace groups", async () => {
    const { container } = render(<FleetScreen />);
    const fingerprint = await waitFor(() => {
      const found = container.querySelector("[data-testid='fleet-fingerprint']");
      expect(found).not.toBeNull();
      return found!;
    });
    expect(fingerprint.getAttribute("aria-label")).toBe("Machine fingerprint");
    expect(fingerprint.getAttribute("tabindex")).toBe("0");
    expect(fingerprint.textContent!.split(" ")).toHaveLength(8);
    expect(container.querySelector("[data-testid='fleet-peers-empty']")).not.toBeNull();
    expect(container.textContent).not.toMatch(/Coming soon|EPICS/);
  });

  it("renames this Mac locally", async () => {
    const { container } = render(<FleetScreen />);
    await waitFor(() => expect(container.querySelector("[data-testid='fleet-rename']")).not.toBeNull());
    fireEvent.click(container.querySelector("[data-testid='fleet-rename']")!);
    const input = container.querySelector<HTMLInputElement>("[data-testid='fleet-label-input']")!;
    fireEvent.change(input, { target: { value: "  Laptop  " } });
    await act(async () => {
      fireEvent.submit(input.form!);
    });
    await waitFor(() =>
      expect(container.querySelector("[data-testid='fleet-label']")!.textContent).toBe("Laptop"),
    );
    expect(w().__AIJ_FLEET_CALLS__).toEqual(["label:Laptop"]);
  });

  it("Unpair is confirmed in a sheet naming the machine, with Cancel focused", async () => {
    w().__AIJ_FLEET_PEERS__ = [PEER];
    const { container } = render(<FleetScreen />);
    await waitFor(() => expect(container.querySelector("[data-testid='fleet-peer']")).not.toBeNull());
    fireEvent.click(container.querySelector("[data-testid='fleet-unpair']")!);
    const sheet = container.querySelector("[data-testid='fleet-confirm']")!;
    expect(sheet.getAttribute("role")).toBe("alertdialog");
    expect(sheet.textContent).toContain("Unpair Mac mini?");
    expect(document.activeElement?.textContent).toBe("Cancel");
    // Esc cancels without touching anything.
    fireEvent.keyDown(window, { key: "Escape" });
    expect(container.querySelector("[data-testid='fleet-confirm']")).toBeNull();
    expect(w().__AIJ_FLEET_CALLS__).toBeUndefined();
    fireEvent.click(container.querySelector("[data-testid='fleet-unpair']")!);
    await act(async () => {
      fireEvent.click(container.querySelector("[data-testid='fleet-confirm-action']")!);
    });
    await waitFor(() => expect(container.querySelector("[data-testid='fleet-peer']")).toBeNull());
    expect(w().__AIJ_FLEET_CALLS__).toEqual(["unpair:machine-mini"]);
  });

  it("Reset identity warns that every machine must pair again", async () => {
    w().__AIJ_FLEET_PEERS__ = [PEER];
    const { container } = render(<FleetScreen />);
    await waitFor(() => expect(container.querySelector("[data-testid='fleet-reset-identity']")).not.toBeNull());
    const before = container.querySelector("[data-testid='fleet-fingerprint']")!.textContent;
    fireEvent.click(container.querySelector("[data-testid='fleet-reset-identity']")!);
    expect(container.querySelector("[data-testid='fleet-confirm']")!.textContent).toContain(
      "must pair again",
    );
    await act(async () => {
      fireEvent.click(container.querySelector("[data-testid='fleet-confirm-action']")!);
    });
    await waitFor(() =>
      expect(container.querySelector("[data-testid='fleet-fingerprint']")!.textContent).not.toBe(before),
    );
    expect(container.querySelector("[data-testid='fleet-peers-empty']")).not.toBeNull();
  });

  it("a broken registry shows the fail-closed banner and resets only on an explicit click", async () => {
    w().__AIJ_FLEET_SELF__ = {
      trust: "broken",
      trust_reason: "an older copy of the registry was restored (rollback)",
    };
    const { container } = render(<FleetScreen />);
    const banner = await waitFor(() => {
      const found = container.querySelector("[data-testid='fleet-trust-broken']");
      expect(found).not.toBeNull();
      return found!;
    });
    expect(banner.getAttribute("role")).toBe("alert");
    expect(banner.textContent).toContain("Trust broken — re-pair required");
    expect(banner.textContent).toContain("rollback");
    fireEvent.click(container.querySelector("[data-testid='fleet-reset-trust']")!);
    await act(async () => {
      fireEvent.click(container.querySelector("[data-testid='fleet-confirm-action']")!);
    });
    await waitFor(() => expect(container.querySelector("[data-testid='fleet-trust-broken']")).toBeNull());
    expect(w().__AIJ_FLEET_CALLS__).toEqual(["reset-trust"]);
  });
});
