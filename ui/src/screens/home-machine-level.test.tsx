/**
 * AJ-E813: Home reads only This Mac's summary level (a Mac notice is
 * severe under pressure), yet it drew itself again on every 5 s reading.
 * Now it draws again only when the level changes.
 *
 * Home's own draws are counted through a hook it calls once per draw
 * (`useKeys("home-action")`); React's Profiler reports nothing in this
 * test environment.
 */
import { act, cleanup } from "@testing-library/react";
import { useState, type ReactElement } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { HomeScreen } from "./Home";
import { ThisMacValue } from "../lib/machine";
import { renderWithNeeds } from "../test/needs-kit";
import type { ThisMacInfo } from "../types/generated/ThisMacInfo";

const draws = vi.hoisted(() => ({ home: 0 }));
vi.mock("../lib/use-keys", async (original) => {
  const real = await original<typeof import("../lib/use-keys")>();
  return {
    ...real,
    useKeys: (...args: Parameters<typeof real.useKeys>) => {
      if (args[0] === "home-action") draws.home += 1;
      return real.useKeys(...args);
    },
  };
});
vi.mock("../components/TerminalView", () => ({ TerminalView: () => null }));

afterEach(cleanup);

const NOW = 1_728_800_000_000;

/** A reading: its level and a CPU figure that changes on every reading. */
function reading(level: string, cpu: number): ThisMacInfo {
  return { machine: { sampled: true, cpu_percent: cpu, summary: { level, words: level, cause: null } }, heavy: [] } as unknown as ThisMacInfo;
}

let give: (info: ThisMacInfo) => void = () => {};

/** The app's provider, holding Home as one unchanging element (as the
 * shell does): only what reads This Mac can draw again on a reading. */
function Host({ home }: { home: ReactElement }) {
  const [info, setInfo] = useState<ThisMacInfo>(reading("plenty", 4));
  give = setInfo;
  return <ThisMacValue info={info}>{home}</ThisMacValue>;
}

const settle = () => act(async () => new Promise((done) => setTimeout(done, 60)));

describe("AJ-E813 Home and This Mac's readings", () => {
  it("draws nothing on a reading at the same level, and again when the level changes", async () => {
    const home = <HomeScreen sessions={[]} accounts={[]} projects={[]} now={NOW} onOpenSession={() => {}} onNewSession={() => null} onAddAccounts={() => null} />;
    renderWithNeeds({ sessions: [] }, <Host home={home} />);
    await settle();
    expect(draws.home).toBeGreaterThan(0);

    draws.home = 0;
    await act(async () => give(reading("plenty", 37)));
    await act(async () => give(reading("plenty", 81)));
    await settle();
    expect(draws.home, "readings at the same level").toBe(0);

    await act(async () => give(reading("pressure", 92)));
    await settle();
    expect(draws.home, "the level changed").toBeGreaterThan(0);
  });
});
