/**
 * AJ-E708 vitest — the sync chip and the clock-skew note: state in words
 * + a shape (never colour alone); "N changes queued" counts what waits on
 * THIS Mac for an away machine; a clock more than 5 minutes off is flagged
 * and the note says order never follows it.
 */
import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { ClockSkewNote, SyncChip } from "../screens/FleetScreen";
import type { FleetPeerInfo } from "../lib/bridge";

afterEach(cleanup);

const peer = (over: Partial<FleetPeerInfo> = {}): FleetPeerInfo => ({
  machine_id: "machine-laptop",
  label: "Laptop",
  fingerprint: "11AA 22BB 33CC 44DD 55EE 66FF 7788 99AA",
  permission: "view",
  permission_label: "View only",
  ask_each_time: false,
  paired_via: "code",
  paired_at_ms: 1,
  last_seen_ms: 1,
  connection: "connected",
  protocol: "v1.0",
  connection_detail: null,
  queued_changes: 0,
  pairing_unfinished: false,
  ...over,
});

describe("AJ-E708 sync chip", () => {
  it("says Synced while connected", () => {
    const { container } = render(<SyncChip peer={peer()} />);
    const chip = container.querySelector("[data-testid='fleet-sync']")!;
    expect(chip.getAttribute("data-state")).toBe("synced");
    expect(chip.textContent).toBe("✓ Synced");
  });

  it("counts the changes queued for an away machine, singular and plural", () => {
    const { container, rerender } = render(<SyncChip peer={peer({ connection: "offline", queued_changes: 12 })} />);
    const chip = () => container.querySelector("[data-testid='fleet-sync']")!;
    expect(chip().textContent).toBe("⇡ 12 changes queued");
    expect(chip().getAttribute("data-state")).toBe("queued");
    expect(chip().getAttribute("title")).toMatch(/only those, nothing it already has/);
    rerender(<SyncChip peer={peer({ connection: "offline", queued_changes: 1 })} />);
    expect(chip().textContent).toBe("⇡ 1 change queued");
    rerender(<SyncChip peer={peer({ connection: "offline", queued_changes: 0 })} />);
    expect(chip().textContent).toBe("✓ Nothing queued");
  });

  it("stays hidden with Link off when nothing is known", () => {
    const { container } = render(<SyncChip peer={peer({ connection: "link_off", queued_changes: null })} />);
    expect(container.querySelector("[data-testid='fleet-sync']")).toBeNull();
  });
});

describe("AJ-E708 clock skew", () => {
  it("flags more than 5 minutes either way and says order follows the sequence", () => {
    const { container, rerender } = render(<ClockSkewNote label="Laptop" skewMs={10 * 60_000} />);
    const note = () => container.querySelector("[data-testid='fleet-clock-skew']");
    expect(note()!.textContent).toMatch(/Laptop's clock is 10 min ahead of this Mac's/);
    expect(note()!.textContent).toMatch(/never its clock/);
    rerender(<ClockSkewNote label="Laptop" skewMs={-10 * 60_000} />);
    expect(note()!.textContent).toMatch(/10 min behind/);
    rerender(<ClockSkewNote label="Laptop" skewMs={4 * 60_000} />);
    expect(note()).toBeNull();
    rerender(<ClockSkewNote label="Laptop" skewMs={null} />);
    expect(note()).toBeNull();
  });
});
