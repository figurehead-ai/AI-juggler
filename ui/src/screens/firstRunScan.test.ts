/**
 * AJ-E2906 R5 — Look again while a scan still runs: only the newest
 * scan's answer lands. PR #218's `ui` job lost the race on a slow runner:
 * the first scan's older answer finished first, was shown, and the newer
 * one (with the new setup) was dropped.
 */
import { act, renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ScanReportData } from "../lib/ipc/providers";

const pending: Array<{ resolve: (report: ScanReportData) => void; reject: (error: Error) => void }> = [];
vi.mock("../lib/ipc/providers", async (original) => ({
  ...(await original<typeof import("../lib/ipc/providers")>()),
  environmentScanFull: () =>
    new Promise<ScanReportData>((resolve, reject) => {
      pending.push({ resolve, reject });
    }),
}));

const { useSetupScan } = await import("./firstRunScan");

/** A report told apart by its one project seed. */
const named = (seed: string): ScanReportData => ({ installs: [], probes: [], seeds: [seed], desktop_apps: [], cancelled: false });

afterEach(() => {
  pending.length = 0;
});

describe("useSetupScan", () => {
  it("an older scan that finishes after a newer one started never shows", async () => {
    const { result } = renderHook(() => useSetupScan());
    act(() => result.current.scan());
    act(() => result.current.scan());
    expect(pending).toHaveLength(2);
    await act(async () => pending[0]!.resolve(named("/old")));
    expect(result.current.report).toBeNull();
    await act(async () => pending[1]!.resolve(named("/new")));
    expect(result.current.report?.seeds).toEqual(["/new"]);
  });

  it("an older scan that fails after a newer one started never shows its error", async () => {
    const { result } = renderHook(() => useSetupScan());
    act(() => result.current.scan());
    act(() => result.current.scan());
    await act(async () => pending[0]!.reject(new Error("old failure")));
    expect(result.current.scanError).toBeNull();
    expect(result.current.report).toBeNull();
    await act(async () => pending[1]!.resolve(named("/new")));
    expect(result.current.report?.seeds).toEqual(["/new"]);
  });
});
