import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

/**
 * Lazy loading (2026-10-23, "lazy load as much as possible"): a part loads
 * on first use and waits on its own, so what's around it never blanks; and
 * after the first paint every part is fetched, the likeliest first.
 */
afterEach(cleanup);

describe("lazyNamed and prefetchLazy", () => {
  it("renders the part once loaded, and only the part waits", async () => {
    vi.resetModules();
    const { lazyNamed } = await import("./lazy");
    let release!: () => void;
    const gate = new Promise<void>((resolve) => (release = resolve));
    const Slow = lazyNamed(
      () => gate.then(() => ({ Panel: ({ label }: { label: string }) => <p data-testid="panel">{label}</p> })),
      "Panel",
    );
    const view = render(
      <div>
        <h1 data-testid="beside">Home</h1>
        <Slow label="Settings" />
      </div>,
    );
    // While it loads, what's beside it is there, and the part isn't.
    expect(view.getByTestId("beside").textContent).toBe("Home");
    expect(view.queryByTestId("panel")).toBeNull();
    // The page says a part is loading (the visual gate waits on it).
    expect(document.documentElement.hasAttribute("data-loading-parts")).toBe(true);
    release();
    expect((await view.findByTestId("panel")).textContent).toBe("Settings");
    expect(document.documentElement.hasAttribute("data-loading-parts")).toBe(false);
  });

  it("renders a part already fetched in its first frame, never through Suspense (0.3.4)", async () => {
    vi.resetModules();
    vi.useFakeTimers();
    const { lazyNamed, prefetchLazy } = await import("./lazy");
    const Part = lazyNamed(() => Promise.resolve({ Panel: ({ label }: { label: string }) => <p data-testid="panel">{label}</p> }), "Panel");
    prefetchLazy();
    for (let i = 0; i < 10; i++) await vi.advanceTimersByTimeAsync(100);
    vi.useRealTimers();
    // The idle prefetch fetched it: the first open shows it at once (React
    // holds a Suspense reveal 300 ms: a blank screen on first open).
    const view = render(<Part label="Usage" />);
    expect(view.getByTestId("panel").textContent).toBe("Usage");
  });

  it("keeps a part shown while it loaded in its Suspense (no remount when it arrives)", async () => {
    vi.resetModules();
    const { lazyNamed } = await import("./lazy");
    const { useEffect } = await import("react");
    let mounts = 0;
    let release!: () => void;
    const gate = new Promise<void>((resolve) => (release = resolve));
    const Part = lazyNamed(
      () =>
        gate.then(() => ({
          Panel: ({ label }: { label: string }) => {
            useEffect(() => void (mounts += 1), []);
            return <p data-testid="panel">{label}</p>;
          },
        })),
      "Panel",
    );
    const view = render(<Part label="One" />);
    release();
    await view.findByTestId("panel");
    view.rerender(<Part label="Two" />);
    expect(view.getByTestId("panel").textContent).toBe("Two");
    expect(mounts).toBe(1);
  });

  it("fetches every part after the first paint, the likeliest first, once", async () => {
    vi.resetModules();
    vi.useFakeTimers();
    const { lazyNamed, prefetchLazy } = await import("./lazy");
    const order: string[] = [];
    const part = (name: string) => () => {
      order.push(name);
      return Promise.resolve({ Part: () => null });
    };
    lazyNamed(part("settings"), "Part");
    lazyNamed(part("inspector"), "Part", true);
    lazyNamed(part("usage"), "Part");
    prefetchLazy();
    prefetchLazy(); // a second call does nothing
    expect(order).toEqual([]); // not during the first paint
    for (let i = 0; i < 10; i++) await vi.advanceTimersByTimeAsync(100);
    expect(order).toEqual(["inspector", "settings", "usage"]);
    vi.useRealTimers();
  });
});
