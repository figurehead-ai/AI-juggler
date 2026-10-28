import { cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { OutsideSearchSetting, indexWords } from "./OutsideSearchSetting";
import { hitKey } from "../screens/History";
import { type OutsideSearchEstimateInfo, type OutsideSearchInfo } from "../lib/ipc/outsideSearch";
import { byteSize } from "../lib/compact";

/**
 * AJ-E1538: searching the words of every session is off by default; the
 * size of its index is said before it's turned on, and its real size
 * while it's on; turning it off deletes the index. History keys an
 * outside session's hit by its account and conversation, as its rows do.
 */

type Seams = {
  __AIJ_OUTSIDE_SEARCH__?: OutsideSearchInfo;
  __AIJ_OUTSIDE_SEARCH_ESTIMATE__?: OutsideSearchEstimateInfo;
  __AIJ_OUTSIDE_SEARCH_CALLS__?: boolean[];
};
const seams = () => window as unknown as Seams;

beforeEach(() => {
  delete seams().__AIJ_OUTSIDE_SEARCH__;
  delete seams().__AIJ_OUTSIDE_SEARCH_ESTIMATE__;
  delete seams().__AIJ_OUTSIDE_SEARCH_CALLS__;
});
afterEach(cleanup);

describe("AJ-E1538 Settings › Privacy", () => {
  it("is off by default and says how big its index would be", async () => {
    seams().__AIJ_OUTSIDE_SEARCH__ = { enabled: false, size_bytes: null, sessions: 3400, indexed: 0 };
    seams().__AIJ_OUTSIDE_SEARCH_ESTIMATE__ = { size_bytes: 640_000_000, sessions: 3400 };
    const view = render(<OutsideSearchSetting />);
    await waitFor(() =>
      expect(view.getByTestId("outside-search-size").textContent).toBe(
        "Its index would take about 640 MB on this computer, for 3400 sessions.",
      ),
    );
    expect((view.getByTestId("outside-search-switch") as HTMLInputElement).checked).toBe(false);
    expect(view.getByTestId("privacy-outside-search").textContent).toContain("Off by default; turning it off deletes the index.");
  });

  it("turns on, then off, and says where the index stands", async () => {
    seams().__AIJ_OUTSIDE_SEARCH__ = { enabled: false, size_bytes: null, sessions: 12, indexed: 0 };
    seams().__AIJ_OUTSIDE_SEARCH_ESTIMATE__ = { size_bytes: 3_000_000, sessions: 12 };
    const view = render(<OutsideSearchSetting />);
    await waitFor(() => expect(view.getByTestId("outside-search-switch")).toBeTruthy());
    fireEvent.click(view.getByTestId("outside-search-switch"));
    await waitFor(() => expect((view.getByTestId("outside-search-switch") as HTMLInputElement).checked).toBe(true));
    fireEvent.click(view.getByTestId("outside-search-switch"));
    await waitFor(() => expect((view.getByTestId("outside-search-switch") as HTMLInputElement).checked).toBe(false));
    expect(seams().__AIJ_OUTSIDE_SEARCH_CALLS__).toEqual([true, false]);
  });

  it("says how far the index has got while it builds, and when it's done", () => {
    const on = (indexed: number): OutsideSearchInfo => ({ enabled: true, size_bytes: 210_000_000, sessions: 3400, indexed });
    expect(indexWords(on(900), null)).toBe("Index: 210 MB so far · 900 of 3400 sessions. It builds in the background.");
    expect(indexWords(on(3400), null)).toBe("Index: 210 MB · all 3400 sessions.");
    expect(indexWords({ enabled: false, size_bytes: null, sessions: 0, indexed: 0 }, { size_bytes: 0, sessions: 0 })).toBe(
      "There are no sessions started outside AI Juggler to index yet.",
    );
    expect(indexWords({ enabled: false, size_bytes: null, sessions: 5, indexed: 0 }, null)).toBe(
      "Working out how big its index would be…",
    );
    expect([byteSize(1_234_000_000), byteSize(640_400_000), byteSize(2_000), byteSize(0)]).toEqual(["1.2 GB", "640 MB", "2 KB", "0 B"]);
  });
});

describe("AJ-E1538 History", () => {
  it("keys an outside session's hit by its account and conversation", () => {
    expect(hitKey({ session_id: "sess-1", account_id: null, conversation_id: null, snippet: "x" })).toBe("sess-1");
    expect(hitKey({ session_id: null, account_id: "claude-code-1", conversation_id: "c-9", snippet: "x" })).toBe(
      "claude-code-1:c-9",
    );
  });
});
