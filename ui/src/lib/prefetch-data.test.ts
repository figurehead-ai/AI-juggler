import { describe, expect, it, vi } from "vitest";

const queued: (() => Promise<unknown>)[] = [];
vi.mock("./bridge", async (importOriginal) => ({ ...(await importOriginal<typeof import("./bridge")>()), hasLiveBackend: () => true }));
vi.mock("./lazy", () => ({ prefetchData: (load: () => Promise<unknown>) => queued.push(load) }));
vi.mock("./ipc/sessions", async (importOriginal) => ({
  ...(await importOriginal<typeof import("./ipc/sessions")>()),
  sessionsPage: async () => ({ items: [{ key: "s/1", session: null, conversation: null, snippet: null }], next_cursor: "c", total: 900 }),
}));
vi.mock("./ipc/costs", async (importOriginal) => ({
  ...(await importOriginal<typeof import("./ipc/costs")>()),
  costRowsPage: async () => ({ items: [], next_cursor: null, total: 0 }),
}));

const { historyKey, prefetchMainScreens } = await import("./prefetch-data");
const { rememberedPage } = await import("./paged");
const { costRowsKey } = await import("./ipc/costs");

describe("the main screens' first pages, after first paint", () => {
  it("are queued in idle time and land under the keys their screens read", async () => {
    prefetchMainScreens();
    expect(queued).toHaveLength(2);
    expect(rememberedPage(historyKey("ended", ""))).toBeUndefined();
    for (const load of queued) await load();
    expect(rememberedPage(historyKey("ended", ""))!.total).toBe(900);
    expect(rememberedPage(costRowsKey(null))!.items).toEqual([]);
  });
});
