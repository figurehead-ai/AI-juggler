import { act, render, screen, waitFor } from "@testing-library/react";
import { useContext, type JSX } from "react";
import { describe, expect, it, vi } from "vitest";

/** The backend's 5 s This Mac emission, as the page hears it (the page
 * takes whatever reading it is given: the parts a test needs). */
let tell: ((info: object) => void) | null = null;
vi.mock("./ipc/this-mac", () => ({
  thisMac: () => new Promise(() => undefined),
  onThisMac: (handler: (info: object) => void) => {
    tell = handler;
    return () => (tell = null);
  },
}));
vi.mock("./bridge", async (importOriginal) => ({ ...(await importOriginal<typeof import("./bridge")>()), hasLiveBackend: () => true }));

describe("This Mac's readings (AJ-E805)", () => {
  // It was the shell's state: each 5 s reading, while an AI tool runs, drew
  // the whole app again, the rail included. Only what reads it draws now.
  it("draw again only what reads This Mac", async () => {
    const { ThisMacContext, ThisMacProvider } = await import("./machine");
    const reader = vi.fn();
    const rest = vi.fn();
    function Reader(): JSX.Element {
      reader();
      const info = useContext(ThisMacContext);
      return <span data-testid="tools">{info?.tools.count ?? "none"}</span>;
    }
    function Rest(): JSX.Element {
      rest();
      return <span>rail</span>;
    }
    render(
      <ThisMacProvider>
        <Rest />
        <Reader />
      </ThisMacProvider>,
    );
    await waitFor(() => expect(tell).not.toBeNull());
    const restDrawn = rest.mock.calls.length;
    for (const count of [1, 2, 3]) act(() => tell!({ tools: { count } }));
    expect(screen.getByTestId("tools").textContent).toBe("3");
    expect(rest).toHaveBeenCalledTimes(restDrawn);
    expect(reader.mock.calls.length).toBeGreaterThan(3);
  });

  // A rail row read its heavy note from the whole reading, so each 5 s
  // reading drew every row again. A row draws again only when its note does.
  it("draw a row again only when its own heavy note changes", async () => {
    const { ThisMacProvider, useHeavyNote } = await import("./machine");
    const drawn = vi.fn();
    function Row({ id }: { id: string }): JSX.Element {
      drawn(id);
      return <span data-testid={`heavy-${id}`}>{useHeavyNote(id) ?? "calm"}</span>;
    }
    const rows = (id: string) => drawn.mock.calls.filter(([which]) => which === id).length;
    const reading = (a: string | null, b: string | null, count: number) => ({
      tools: { count },
      heavy: [
        { session_id: "a", heavy_note: a },
        { session_id: "b", heavy_note: b },
      ].filter((tree) => tree.heavy_note !== null),
    });
    render(
      <ThisMacProvider>
        <Row id="a" />
        <Row id="b" />
      </ThisMacProvider>,
    );
    await waitFor(() => expect(tell).not.toBeNull());
    act(() => tell!(reading(null, "3.2 GB", 1)));
    const a = rows("a");
    const b = rows("b");
    // Five more readings: a's note comes and goes, b's stays the same.
    for (const [note, count] of [["CPU 96%", 2], ["CPU 97%", 3], [null, 4], [null, 5], [null, 6]] as const) {
      act(() => tell!(reading(note, "3.2 GB", count)));
    }
    expect(screen.getByTestId("heavy-a").textContent).toBe("calm");
    expect(screen.getByTestId("heavy-b").textContent).toBe("3.2 GB");
    expect(rows("b")).toBe(b);
    expect(rows("a") - a).toBe(3);
  });
});
