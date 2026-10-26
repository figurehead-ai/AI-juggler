import { afterEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { forgetPaged, pageOf, prefetchPaged, refreshPaged, rememberedPage, usePagedList, type FetchPage, type PagedList as Paged } from "./paged";
import { PagedList } from "../components/PagedList";

afterEach(() => {
  cleanup();
  forgetPaged();
});

type Row = { id: string; title: string };
const rows = (n: number, prefix = "r"): Row[] => Array.from({ length: n }, (_, i) => ({ id: `${prefix}${i}`, title: `Session ${i}` }));

/** A backend whose answers wait until the test releases them. */
function backend(all: () => Row[]) {
  const asked: { cursor: string | null; limit: number | null }[] = [];
  const pending: (() => void)[] = [];
  const fetchPage: FetchPage<Row> = (request) => {
    asked.push({ cursor: request.cursor, limit: request.limit });
    return new Promise((resolve) => pending.push(() => resolve(pageOf(all(), request))));
  };
  const release = async () => {
    await act(async () => {
      while (pending.length) pending.shift()!();
      await Promise.resolve();
    });
  };
  return { asked, fetchPage, release };
}

function Probe({ listKey, fetchPage, pageSize, out }: { listKey: string | null; fetchPage: FetchPage<Row>; pageSize?: number; out: { list?: Paged<Row> } }) {
  out.list = usePagedList(listKey, fetchPage, { pageSize, keyOf: (row) => row.id });
  return null;
}

describe("pageOf (the backend's page shape)", () => {
  it("cuts pages at their boundaries and says when the last one is reached", () => {
    const all = rows(5);
    const first = pageOf(all, { cursor: null, limit: 2 });
    expect(first.items.map((r) => r.id)).toEqual(["r0", "r1"]);
    const second = pageOf(all, { cursor: first.next_cursor, limit: 2 });
    expect(second.items.map((r) => r.id)).toEqual(["r2", "r3"]);
    const last = pageOf(all, { cursor: second.next_cursor, limit: 2 });
    expect(last.items.map((r) => r.id)).toEqual(["r4"]);
    expect(last.next_cursor).toBeNull();
    expect(pageOf(all, { cursor: null, limit: 5000 }).items).toHaveLength(5);
    expect(pageOf([], { cursor: null, limit: null })).toEqual({ items: [], next_cursor: null, total: 0 });
  });
});

describe("usePagedList", () => {
  it("reads the first page, then the next on loadMore, without doubling a row", async () => {
    const all = rows(120);
    const api = backend(() => all);
    const out: { list?: Paged<Row> } = {};
    render(<Probe listKey="history" fetchPage={api.fetchPage} pageSize={50} out={out} />);
    expect(out.list!.loading).toBe(true);
    await api.release();
    expect(out.list!.items).toHaveLength(50);
    expect(out.list!.hasMore).toBe(true);
    expect(out.list!.total).toBe(120);
    act(() => out.list!.loadMore());
    act(() => out.list!.loadMore()); // a second ask while one is in flight is ignored
    expect(api.asked).toHaveLength(2);
    expect(out.list!.loadingMore).toBe(true);
    await api.release();
    expect(out.list!.items.map((r) => r.id).slice(48, 52)).toEqual(["r48", "r49", "r50", "r51"]);
    expect(new Set(out.list!.items.map((r) => r.id)).size).toBe(100);
  });

  it("shows a list opened again at once from memory, and reads it again behind it", async () => {
    let all = rows(3);
    const api = backend(() => all);
    const out: { list?: Paged<Row> } = {};
    const first = render(<Probe listKey="sessions:p1" fetchPage={api.fetchPage} out={out} />);
    await api.release();
    first.unmount();
    all = [{ id: "new", title: "New" }, ...all];
    render(<Probe listKey="sessions:p1" fetchPage={api.fetchPage} out={out} />);
    // The very first render has the remembered rows: no skeleton.
    expect(out.list!.loading).toBe(false);
    expect(out.list!.stale).toBe(true);
    expect(out.list!.items.map((r) => r.id)).toEqual(["r0", "r1", "r2"]);
    await api.release();
    expect(out.list!.stale).toBe(false);
    expect(out.list!.items[0]!.id).toBe("new");
  });

  it("drops an answer for a key it has moved on from", async () => {
    const api = backend(() => rows(3, "a"));
    const other = backend(() => rows(2, "b"));
    const out: { list?: Paged<Row> } = {};
    const view = render(<Probe listKey="a" fetchPage={api.fetchPage} out={out} />);
    view.rerender(<Probe listKey="b" fetchPage={other.fetchPage} out={out} />);
    await other.release();
    await api.release();
    expect(out.list!.items.map((r) => r.id)).toEqual(["b0", "b1"]);
  });

  it("is read again when refreshPaged names it, and prefetch fills memory for the first open", async () => {
    let all = rows(2);
    const api = backend(() => all);
    const out: { list?: Paged<Row> } = {};
    render(<Probe listKey="history:ended" fetchPage={api.fetchPage} out={out} />);
    await api.release();
    all = rows(4);
    act(() => refreshPaged("history:"));
    await api.release();
    expect(out.list!.items).toHaveLength(4);

    const fetchPage = vi.fn(async (request: { cursor: string | null; limit: number | null }) => pageOf(rows(80), request));
    await prefetchPaged("usage:rows", fetchPage, 30);
    await prefetchPaged("usage:rows", fetchPage, 30);
    expect(fetchPage).toHaveBeenCalledTimes(1);
    expect(rememberedPage<Row>("usage:rows")!.items).toHaveLength(30);
  });
});

describe("PagedList", () => {
  function List({ fetchPage, listKey = "list" }: { fetchPage: FetchPage<Row>; listKey?: string }) {
    const list = usePagedList(listKey, fetchPage, { keyOf: (row) => row.id });
    return (
      <PagedList
        list={list}
        keyOf={(row) => row.id}
        label="Sessions"
        empty={<p>No sessions</p>}
        rowHeight={40}
        renderRow={(row) => (
          <button type="button" data-testid="row">
            {row.title}
          </button>
        )}
      />
    );
  }

  it("shows its skeleton in the very first render", () => {
    const api = backend(() => rows(10));
    render(<List fetchPage={api.fetchPage} />);
    expect(screen.getByTestId("paged-list-skeleton").getAttribute("aria-busy")).toBe("true");
  });

  it("draws only the rows in view of a long list, and says how many there are", async () => {
    const all = rows(200);
    const api = backend(() => all);
    render(<List fetchPage={(request) => api.fetchPage({ ...request, limit: 200 })} />);
    await api.release();
    const drawn = screen.getAllByTestId("row");
    // jsdom's view is 768 px: 20 rows of 40 px and the overscan, not 200.
    expect(drawn.length).toBeGreaterThan(10);
    expect(drawn.length).toBeLessThan(40);
    const first = screen.getAllByRole("listitem")[0]!;
    expect(first.getAttribute("aria-setsize")).toBe("200");
    expect(first.getAttribute("aria-posinset")).toBe("1");
  });

  it("reads the next page from its Load more button, and moves by arrow key", async () => {
    const all = rows(60);
    const api = backend(() => all);
    render(<List fetchPage={(request) => api.fetchPage({ ...request, limit: 25 })} />);
    await api.release();
    // The end is near in a 768 px view: the next page is asked at once.
    await api.release();
    await waitFor(() => expect(api.asked.length).toBeGreaterThanOrEqual(2));
    await api.release();
    const more = screen.queryByTestId("paged-list-more");
    if (more) {
      fireEvent.click(more);
      await api.release();
    }
    expect(screen.queryByTestId("paged-list-more")).toBeNull();
    const buttons = screen.getAllByTestId("row");
    buttons[0]!.focus();
    fireEvent.keyDown(buttons[0]!, { key: "ArrowDown" });
    await waitFor(() => expect(document.activeElement?.textContent).toBe("Session 1"));
  });

  it("says when there's nothing, and never shows a skeleton for an empty answer", async () => {
    const api = backend(() => []);
    render(<List fetchPage={api.fetchPage} />);
    await api.release();
    expect(screen.getByText("No sessions")).toBeTruthy();
    expect(screen.queryByTestId("paged-list-skeleton")).toBeNull();
  });
});
