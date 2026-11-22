import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { RemoteNoticeInfo } from "../types/generated/RemoteNoticeInfo";

const calls: Array<{ name: string; args: unknown }> = [];
const heard: Array<(message: { payload: null }) => void> = [];
let shown: RemoteNoticeInfo[] = [];

const notice = (id: string, over: Partial<RemoteNoticeInfo> = {}): RemoteNoticeInfo => ({
  id,
  level: "info",
  title: `Notice ${id}`,
  body: null,
  link: null,
  link_label: null,
  dismissable: true,
  ...over,
});

vi.mock("./ipc/transport", async (importOriginal) => ({
  ...(await importOriginal<typeof import("./ipc/transport")>()),
  liveBackend: true,
  call: async (name: string, args: unknown) => {
    calls.push({ name, args });
    if (name === "notices_list") return [...shown];
    if (name === "notices_dismiss") {
      const id = typeof args === "object" && args !== null && "id" in args ? String(args.id) : "";
      if (shown.some((n) => n.id === id && !n.dismissable)) throw { code: "refused", retryable: false, user_message: "This notice stays." };
      shown = shown.filter((n) => n.id !== id);
      return [...shown];
    }
    throw new Error(`unexpected ${name}`);
  },
  listen: async (event: string, handler: (message: { payload: null }) => void) => {
    if (event === "remote-config://changed") heard.push(handler);
    return () => undefined;
  },
}));
const mockBackend = vi.hoisted(() => ({ live: true }));
vi.mock("./bridge", async (importOriginal) => ({ ...(await importOriginal<typeof import("./bridge")>()), hasLiveBackend: () => mockBackend.live }));

beforeEach(() => {
  calls.length = 0;
  heard.length = 0;
  shown = [];
  mockBackend.live = true;
});

describe("useNotices() (ADR-0035 §2, AJ-E2023)", () => {
  it("reads nothing until a view asks, then shows none when there are none", async () => {
    vi.resetModules();
    const { useNotices } = await import("./remote-notices");
    expect(calls).toEqual([]);
    const view = renderHook(() => useNotices());
    expect(view.result.current.notices).toBeNull();
    await waitFor(() => expect(view.result.current.notices).toEqual([]));
  });

  it("shows the notices and dismisses one", async () => {
    shown = [notice("a", { level: "needs_you" }), notice("b")];
    vi.resetModules();
    const { useNotices } = await import("./remote-notices");
    const view = renderHook(() => useNotices());
    await waitFor(() => expect(view.result.current.notices?.map((n) => n.id)).toEqual(["a", "b"]));
    await act(() => view.result.current.dismiss("a"));
    expect(calls.map((c) => c.name)).toEqual(["notices_list", "notices_dismiss", "notices_list"]);
    expect(calls[1]?.args).toEqual({ id: "a" });
    await waitFor(() => expect(view.result.current.notices?.map((n) => n.id)).toEqual(["b"]));
  });

  it("refuses to dismiss one that stays, and keeps it", async () => {
    shown = [notice("stays", { level: "broken", dismissable: false })];
    vi.resetModules();
    const { useNotices } = await import("./remote-notices");
    const view = renderHook(() => useNotices());
    await waitFor(() => expect(view.result.current.notices).toHaveLength(1));
    await expect(view.result.current.dismiss("stays")).rejects.toMatchObject({ code: "refused" });
    expect(view.result.current.notices?.map((n) => n.id)).toEqual(["stays"]);
  });

  it("follows the backend's change event (a new file, or a dismissal elsewhere)", async () => {
    vi.resetModules();
    const { useNotices } = await import("./remote-notices");
    const view = renderHook(() => useNotices());
    await waitFor(() => expect(view.result.current.notices).toEqual([]));
    await waitFor(() => expect(heard).toHaveLength(1));
    shown = [notice("new")];
    act(() => heard.forEach((on) => on({ payload: null })));
    await waitFor(() => expect(view.result.current.notices?.map((n) => n.id)).toEqual(["new"]));
  });
});

describe("the preview, with no backend", () => {
  it("shows the seeded notices and the built-in flags", async () => {
    mockBackend.live = false;
    Reflect.set(window, "__AIJ_REMOTE__", { notices: [notice("seeded")] });
    const { noticesList, noticesDismiss, remoteValues, remoteAbout } = await import("./ipc/remote");
    expect((await noticesList()).map((n) => n.id)).toEqual(["seeded"]);
    expect(await noticesDismiss("seeded")).toEqual([]);
    expect((await remoteValues("flags")).flags.notices_on).toBe(true);
    expect((await remoteAbout()).verified).toBe(false);
    expect(calls).toEqual([]);
    Reflect.deleteProperty(window, "__AIJ_REMOTE__");
  });
});
