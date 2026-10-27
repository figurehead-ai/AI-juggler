/**
 * wf10, §4.8 (the 2026-10-27 audit: the sheet lit up and dropped
 * nothing): a folder dropped onto the open New session sheet becomes its
 * folder — the app's own drop (full paths) and a page drop that carries a
 * `file://` address alike.
 */
import { cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { droppedPath } from "../lib/use-folder-drop";

const native = vi.hoisted(() => ({ handler: null as ((path: string) => void) | null }));

vi.mock("../lib/bridge", async (original) => {
  const real = await original<typeof import("../lib/bridge")>();
  return {
    ...real,
    onFolderDropped: (handler: (path: string) => void) => {
      native.handler = handler;
      return () => {
        native.handler = null;
      };
    },
  };
});

const w = () => window as unknown as Record<string, unknown>;

beforeEach(() => {
  w().__AIJ_RECIPES__ = [];
  w().__AIJ_RANKING__ = [];
  w().__AIJ_QUOTA_DISPLAY__ = [];
});
afterEach(() => {
  cleanup();
  native.handler = null;
});

async function sheet() {
  const { NewSessionModal } = await import("../screens/NewSession");
  return render(<NewSessionModal projects={[]} accounts={[]} initialDirectory={null} onClose={() => {}} onLaunched={() => {}} openLoginTerminal={() => {}} />);
}

describe("dropping a folder on New session", () => {
  it("the app's own drop picks the folder while the sheet is open", async () => {
    const view = await sheet();
    await waitFor(() => expect(native.handler).not.toBeNull());
    native.handler!("/Users/me/Dev/payments-api");
    await waitFor(() => expect(view.container.textContent).toContain("payments-api"));
  });

  it("a page drop with a file address picks it too, and the highlight goes", async () => {
    const view = await sheet();
    const zone = view.getByTestId("new-session-drop");
    fireEvent.dragOver(zone, { dataTransfer: { getData: () => "" } });
    fireEvent.drop(zone, { dataTransfer: { getData: (type: string) => (type === "text/uri-list" ? "file:///Users/me/Dev/web%20app/" : "") } });
    await waitFor(() => expect(view.container.textContent).toContain("web app"));
  });

  it("reads only file addresses", () => {
    expect(droppedPath({ getData: () => "file:///Users/me/a%20b" })).toBe("/Users/me/a b");
    expect(droppedPath({ getData: () => "# comment\nfile:///tmp/x/" })).toBe("/tmp/x");
    expect(droppedPath({ getData: () => "https://example.com/x" })).toBeNull();
    expect(droppedPath(null)).toBeNull();
  });
});
