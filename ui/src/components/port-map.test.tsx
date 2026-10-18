import { describe, expect, it, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import { PortInventoryList, PortWarningSheet, type PortsBridge } from "./PortMap";
import type { PortInventoryInfo } from "../lib/bridge";

function inventoryFixture(): PortInventoryInfo {
  return {
    rows: [
      {
        port: 5173,
        pid: 8123,
        class: "managed-orphan",
        session: "sess-a",
        provider: "claude-code",
        account: "acc-1",
        project: "proj-1",
        listener_age_ms: 9_000_000,
        name: "/usr/bin/python3",
        hint: "vite dev server port",
        killable: true,
      },
      {
        port: 3000,
        pid: 4400,
        class: "managed-running",
        session: "sess-b",
        provider: "claude-code",
        account: "acc-2",
        project: "proj-2",
        listener_age_ms: 65_000,
        name: "/usr/local/bin/node",
        hint: "common dev server port",
        killable: true,
      },
      {
        port: 8080,
        pid: 5500,
        class: "re-adopted",
        session: "sess-c",
        provider: "claude-code",
        account: null,
        project: null,
        listener_age_ms: 250_000,
        name: "/usr/bin/python3",
        hint: "common dev server port",
        killable: true,
      },
      {
        port: 8000,
        pid: 6600,
        class: "not-managed",
        session: null,
        provider: null,
        account: null,
        project: null,
        listener_age_ms: null,
        name: null,
        hint: "common dev server port",
        killable: false,
      },
    ],
    note: null,
    hogs: [],
  };
}

describe("PortInventoryList (AJ-E505)", () => {
  it("renders all four classifications with icon+text labels", async () => {
    const bridge: PortsBridge = { load: () => Promise.resolve(inventoryFixture()) };
    render(<PortInventoryList bridge={bridge} />);
    await waitFor(() => expect(screen.getByTestId("port-inventory")).toBeTruthy());
    expect((screen.getByTestId("port-row-5173")).textContent ?? "").toContain("Managed orphan");
    expect(screen.getByTestId("port-class-5173").textContent ?? "").toMatch(/⚑/);
    expect((screen.getByTestId("port-row-3000")).textContent ?? "").toContain("Managed running");
    expect((screen.getByTestId("port-row-8080")).textContent ?? "").toContain("Re-adopted");
    expect((screen.getByTestId("port-row-8000")).textContent ?? "").toContain("Not managed by Juggler");
    // Unmanaged rows say WHY they are muted — icon+text, not colour.
    expect((screen.getByTestId("port-row-8000")).textContent ?? "").toContain(
      "listener outside Juggler's ledger",
    );
    // The port is the headline; the PID is evidence (Nielsen H2).
    expect((screen.getByTestId("port-row-5173")).textContent ?? "").toContain(":5173");
    expect((screen.getByTestId("port-row-5173")).textContent ?? "").toContain("PID 8123");
    // Attribution + hint + humanized ages.
    expect((screen.getByTestId("port-row-5173")).textContent ?? "").toContain("claude-code · acc-1 · proj-1");
    expect((screen.getByTestId("port-row-5173")).textContent ?? "").toContain("2h");
    expect((screen.getByTestId("port-row-5173")).textContent ?? "").toContain("vite dev server port");
  });

  it("carries ZERO action controls — kills belong to the AJ-E507 sheet", async () => {
    const bridge: PortsBridge = { load: () => Promise.resolve(inventoryFixture()) };
    render(<PortInventoryList bridge={bridge} />);
    await waitFor(() => expect(screen.getByTestId("port-inventory")).toBeTruthy());
    expect(screen.queryByRole("button")).toBeNull();
  });

  it("renders the honest platform note instead of an empty truth", async () => {
    const bridge: PortsBridge = {
      load: () => Promise.resolve({ rows: [], note: "ports unknown on this platform", hogs: [] }),
    };
    render(<PortInventoryList bridge={bridge} />);
    await waitFor(() => expect(screen.getByTestId("port-inventory-note")).toBeTruthy());
    expect((screen.getByTestId("port-inventory-note")).textContent ?? "").toContain(
      "ports unknown on this platform",
    );
  });

  it("renders the empty state and surfaces load failures", async () => {
    const empty: PortsBridge = { load: () => Promise.resolve({ rows: [], note: null, hogs: [] }) };
    render(<PortInventoryList bridge={empty} />);
    await waitFor(() => expect(screen.getByTestId("port-inventory-empty")).toBeTruthy());

    const failing: PortsBridge = {
      load: () => Promise.reject(new Error("netlink busy")),
    };
    render(<PortInventoryList bridge={failing} />);
    await waitFor(() => expect(screen.getByTestId("port-inventory-error")).toBeTruthy());
    expect((screen.getByTestId("port-inventory-error")).textContent ?? "").toContain("netlink busy");
  });
});

describe("PortWarningSheet (AJ-E505 Req.4)", () => {
  const warning = {
    port: 5173,
    pid: 8123,
    instance: "claude-code (sess-a1b2)",
    age_ms: 9_000_000,
    message: ":5173 is already held by an orphan from claude-code (sess-a1b2) (2h).",
  };

  it("renders the copy-exact message with both actions", () => {
    const onFreePort = vi.fn();
    const onLaunchAnyway = vi.fn();
    const view = render(
      <PortWarningSheet
        warning={warning}
        onFreePort={onFreePort}
        onLaunchAnyway={onLaunchAnyway}
      />,
    );
    expect(view.getByTestId("port-warning-copy").textContent ?? "").toContain(warning.message);
    expect(view.getByRole("button", { name: "Free this port" })).toBeTruthy();
    expect(view.getByRole("button", { name: "Launch anyway" })).toBeTruthy();
    view.unmount();
  });

  it("Launch anyway proceeds without blocking; Free dismisses to the sheet route", () => {
    const onFreePort = vi.fn();
    const onLaunchAnyway = vi.fn();
    const view = render(
      <PortWarningSheet
        warning={warning}
        onFreePort={onFreePort}
        onLaunchAnyway={onLaunchAnyway}
      />,
    );
    view.getByRole("button", { name: "Launch anyway" }).click();
    expect(onLaunchAnyway).toHaveBeenCalledTimes(1);
    view.getByRole("button", { name: "Free this port" }).click();
    expect(onFreePort).toHaveBeenCalledTimes(1);
    view.unmount();
    // The previous describe's render leaked into the document body
    // (no auto-cleanup between describes without the globals): scope
    // lookups per-render as above, proven by the second sheet.
    const second = render(
      <PortWarningSheet warning={warning} onFreePort={vi.fn()} onLaunchAnyway={vi.fn()} />,
    );
    expect(second.getAllByRole("button")).toHaveLength(2);
  });
});
