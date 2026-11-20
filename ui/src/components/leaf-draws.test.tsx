/**
 * AJ-E812: the small parts every App tile and rail row draws (the App's
 * icon, its profile's badge, its kind) draw again only when their own
 * props change. With ten sessions at work, each list read and each 30 s
 * clock tick drew Home's tiles and the rail's rows, and every icon, badge
 * and kind mark in them with it: about 60 icons a minute (the probe's
 * exact counts) that showed what they showed before.
 *
 * Draws are counted through what each part calls once per draw: the
 * badge's marks hook, the kind's words, and the screen's pixel ratio,
 * which an icon reads to pick its size (`pixelsFor`). React's Profiler
 * reports nothing in this test environment (home-machine-level.test).
 */
import { act, cleanup, render } from "@testing-library/react";
import { useEffect, useState } from "react";
import { afterAll, afterEach, beforeAll, describe, expect, it, vi } from "vitest";
import { AppIcon, forgetAppIconsForTests } from "./AppIcon";
import { AppKindBadge } from "./AppKindBadge";
import { ProfileBadge } from "./ProfileBadge";

const draws = vi.hoisted(() => ({ badge: 0, kind: 0, icon: 0 }));
vi.mock("../lib/marks", async (original) => {
  const real = await original<typeof import("../lib/marks")>();
  return {
    ...real,
    useMarks: () => {
      draws.badge += 1;
      return real.useMarks();
    },
  };
});
vi.mock("../lib/app-kind", async (original) => {
  const real = await original<typeof import("../lib/app-kind")>();
  return {
    ...real,
    appKindWords: (...args: Parameters<typeof real.appKindWords>) => {
      draws.kind += 1;
      return real.appKindWords(...args);
    },
  };
});

const ratio = Object.getOwnPropertyDescriptor(window, "devicePixelRatio");
beforeAll(() => {
  Object.defineProperty(window, "devicePixelRatio", {
    configurable: true,
    get: () => {
      draws.icon += 1;
      return 2;
    },
  });
});
afterAll(() => {
  if (ratio) Object.defineProperty(window, "devicePixelRatio", ratio);
});
afterEach(() => {
  cleanup();
  forgetAppIconsForTests();
});

let redraw: () => void = () => {};

/** A tile that draws again on its own (a list read, the clock), its
 * parts' props unchanged unless `name` changes. */
function Tile({ name }: { name: string }) {
  const [, setDraws] = useState(0);
  useEffect(() => {
    redraw = () => setDraws((n) => n + 1);
  }, []);
  return (
    <>
      <AppIcon appId={null} appPid={null} name={name} size={20} />
      <ProfileBadge app="claude-code" name="Work" />
      <AppKindBadge kind="cli" variant="chip" />
    </>
  );
}

const settle = () => act(async () => new Promise((done) => setTimeout(done, 30)));

describe("AJ-E812 an App's icon, badge and kind mark", () => {
  it("draw nothing when the tile around them draws again, and again when their own props change", async () => {
    const { rerender } = render(<Tile name="Example" />);
    await settle();
    expect(draws.icon).toBeGreaterThan(0);
    expect(draws.badge).toBeGreaterThan(0);
    expect(draws.kind).toBeGreaterThan(0);

    const before = { ...draws };
    for (let tick = 0; tick < 5; tick += 1) await act(async () => redraw());
    await settle();
    expect(draws.icon, "the icon, its props unchanged").toBe(before.icon);
    expect(draws.badge, "the profile's badge, its props unchanged").toBe(before.badge);
    expect(draws.kind, "the kind mark, its props unchanged").toBe(before.kind);

    rerender(<Tile name="Other" />);
    await settle();
    expect(draws.icon, "the icon's name changed").toBeGreaterThan(before.icon);
    expect(draws.badge, "the badge's props didn't").toBe(before.badge);
    expect(draws.kind, "the kind's props didn't").toBe(before.kind);
  });
});
