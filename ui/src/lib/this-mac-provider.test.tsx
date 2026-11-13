import { act, render, screen, waitFor } from "@testing-library/react";
import { useContext, type JSX } from "react";
import { describe, expect, it, vi } from "vitest";
import type { ThisMacInfo } from "./machine";

/** The backend's 5 s This Mac emission, as the page hears it. */
let tell: ((info: ThisMacInfo) => void) | null = null;
vi.mock("./ipc/this-mac", () => ({
  thisMac: () => new Promise(() => undefined),
  onThisMac: (handler: (info: ThisMacInfo) => void) => {
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
    const drawn = { reader: 0, rest: 0 };
    function Reader(): JSX.Element {
      drawn.reader += 1;
      const info = useContext(ThisMacContext);
      return <span data-testid="tools">{info?.tools.count ?? "none"}</span>;
    }
    function Rest(): JSX.Element {
      drawn.rest += 1;
      return <span>rail</span>;
    }
    render(
      <ThisMacProvider>
        <Rest />
        <Reader />
      </ThisMacProvider>,
    );
    await waitFor(() => expect(tell).not.toBeNull());
    const rest = drawn.rest;
    for (const count of [1, 2, 3]) act(() => tell!({ tools: { count } } as unknown as ThisMacInfo));
    expect(screen.getByTestId("tools").textContent).toBe("3");
    expect(drawn.rest).toBe(rest);
    expect(drawn.reader).toBeGreaterThan(3);
  });
});
