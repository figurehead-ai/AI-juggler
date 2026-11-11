/**
 * rc31 (and rc27 behind the switch modal): after switching sessions a
 * terminal re-rendered about 12 columns wide. A fit runs only on a host
 * shown with its real size, and never to a mid-layout handful of columns.
 */
import { afterEach, describe, expect, it } from "vitest";

import { MIN_FIT_COLS, fitSize, hostShown } from "./terminal-fit";

const hosts: HTMLElement[] = [];
afterEach(() => hosts.splice(0).forEach((made) => made.remove()));

function host({ shown = true, w = 800, h = 400 } = {}): HTMLElement {
  const el = document.createElement("div");
  document.body.append(el);
  hosts.push(el);
  Object.defineProperty(el, "offsetParent", { get: () => (shown ? document.body : null) });
  Object.defineProperty(el, "clientWidth", { get: () => w });
  Object.defineProperty(el, "clientHeight", { get: () => h });
  return el;
}

describe("fitting a terminal", () => {
  it("fits a host shown with its real size", () => {
    expect(hostShown(host())).toBe(true);
    expect(fitSize(host(), () => ({ cols: 120, rows: 30 }))).toEqual({ cols: 120, rows: 30 });
  });

  it("never measures a hidden host or one not laid out yet", () => {
    let asked = 0;
    const propose = () => (asked++, { cols: 12, rows: 5 });
    expect(fitSize(host({ shown: false }), propose)).toBeNull();
    expect(fitSize(host({ w: 0 }), propose)).toBeNull();
    expect(fitSize(host({ h: 0 }), propose)).toBeNull();
    expect(asked).toBe(0);
  });

  it("never resizes to the 12 columns a mid-layout measure gives", () => {
    expect(fitSize(host(), () => ({ cols: 12, rows: 30 }))).toBeNull();
    expect(fitSize(host(), () => ({ cols: MIN_FIT_COLS, rows: 30 }))).toEqual({ cols: MIN_FIT_COLS, rows: 30 });
    expect(fitSize(host(), () => undefined)).toBeNull();
  });
});
