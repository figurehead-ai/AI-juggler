/** The boot screen's way out (public/boot-watch.js; the 0.4.1 persona walks). */
import { afterEach, describe, expect, it, vi } from "vitest";

import script from "../public/boot-watch.js?raw";

function boot(): HTMLElement {
  document.body.innerHTML = '<div id="root"><div id="boot" role="status"><svg></svg>\n        Opening AI Juggler…\n      </div></div>';
  // eslint-disable-next-line @typescript-eslint/no-implied-eval -- the page's own script, run as the page runs it
  new Function(script)();
  return document.getElementById("boot")!;
}

describe("the boot screen", () => {
  afterEach(() => {
    vi.useRealTimers();
    document.body.innerHTML = "";
  });

  it("says it's taking long, with Reload, after 20 s", () => {
    vi.useFakeTimers();
    const screen = boot();
    vi.advanceTimersByTime(19_000);
    expect(screen.querySelector("button")).toBeNull();
    vi.advanceTimersByTime(1_000);
    expect(screen.getAttribute("role")).toBe("alert");
    expect(screen.textContent).toContain("taking longer than usual");
    expect(screen.textContent).not.toContain("Opening AI Juggler");
    expect(screen.querySelector("button")?.textContent).toBe("Reload");
  });

  it("says it couldn't open when its code fails, and stays quiet once the app replaced it", () => {
    vi.useFakeTimers();
    const screen = boot();
    window.dispatchEvent(new ErrorEvent("error", { message: "a module failed" }));
    expect(screen.textContent).toContain("couldn't finish opening");
    document.body.innerHTML = '<div id="root">Home</div>';
    vi.advanceTimersByTime(25_000);
    expect(document.body.textContent).toBe("Home");
  });
});
