/**
 * ADR-0023 vitest — Settings › Advanced › Local cache: one line with its
 * size and limit and the switch; the limit picker; Clear cache…, which
 * says what gets slower before it clears; each cache, folded; the
 * nearly-full-disk line; and the Privacy pane's door to it.
 */
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { LocalCacheSection, cacheSummary } from "./LocalCacheSection";
import { PrivacyPane } from "./PrivacyPane";
import { targetOf } from "./panes";
import { SAMPLE_CACHE, type CacheInfo } from "../../lib/ipc/cache";

type Seams = { __AIJ_CACHE__?: CacheInfo; __AIJ_CACHE_CALLS__?: unknown[] };
const seams = () => window as unknown as Seams;
const calls = () => seams().__AIJ_CACHE_CALLS__ ?? [];

afterEach(() => {
  cleanup();
  delete seams().__AIJ_CACHE__;
  delete seams().__AIJ_CACHE_CALLS__;
});

const section = async () => {
  render(
    <ul>
      <LocalCacheSection />
    </ul>,
  );
  return screen.findByTestId("cache-switch-row");
};

describe("Local cache", () => {
  it("says on one line whether it's on, what it uses and its limit", async () => {
    const row = await section();
    expect(row.textContent).toContain("Keep a cache on this computer");
    expect(row.textContent).toContain("Using 47 MB of 1.3 GB.");
    expect(within(row).getByRole("switch", { name: "Keep a cache on this computer" }).getAttribute("aria-checked")).toBe("true");
    expect(screen.getByTestId("cache-limit-row").textContent).toContain("Automatic: 1.3 GB now, from this computer's free space.");
    expect(screen.getByRole("button", { name: "Size limit: Automatic" })).toBeTruthy();
    expect(screen.getByTestId("cache-clear-row").textContent).toContain("Frees 47 MB now.");
    // Plain words, never the caches' keys.
    expect(document.body.textContent).not.toContain("search-index");
  });

  it("lists each cache, folded, with its size and what it speeds up", async () => {
    await section();
    const contents = screen.getByTestId("cache-contents-toggle") as HTMLDetailsElement;
    expect(contents.open).toBe(false);
    fireEvent.click(within(contents).getByText("What's in it"));
    const rows = screen.getAllByTestId("cache-row");
    expect(rows.map((row) => row.querySelector("span")?.textContent)).toEqual(["Subagent lists", "App icons", "Search index"]);
    expect(rows[0]!.textContent).toContain("A session's Subagents tab opens at once");
    expect(within(rows[0]!).getByTestId("cache-row-size").textContent).toBe("45 MB · 126 items");
    expect(within(rows[2]!).getByTestId("cache-row-size").textContent).toBe("Empty");
    expect(rows[2]!.textContent).toContain("Kept whole, with its own switch: Search the words of every session, in Privacy.");
  });

  it("turned off, says so, keeps nothing and explains what is greyed", async () => {
    const row = await section();
    await act(async () => {
      fireEvent.click(within(row).getByRole("switch"));
    });
    await waitFor(() => expect(row.textContent).toContain("Off: nothing is kept"));
    expect(calls()).toEqual([{ on: false }]);
    expect(row.textContent).toContain("Turned off. Its files were removed.");
    const limit = screen.getByRole("button", { name: /Size limit/ }) as HTMLButtonElement;
    expect(limit.disabled).toBe(true);
    expect(screen.getByTestId("cache-limit-row").textContent).toContain("Turn the cache on to set its limit.");
    expect((screen.getByTestId("cache-clear") as HTMLButtonElement).disabled).toBe(true);
    expect(screen.getByTestId("cache-clear-row").textContent).toContain("Nothing to clear: the cache is empty.");
  });

  it("clears only after saying what gets slower", async () => {
    await section();
    fireEvent.click(screen.getByTestId("cache-clear"));
    const sheet = await screen.findByTestId("cache-clear-sheet");
    expect(sheet.textContent).toContain("Frees 47 MB on this computer. Nothing of yours is lost.");
    const slower = within(sheet).getByTestId("cache-clear-slower");
    // Only what's in the cache now: the search index is empty here.
    expect(slower.querySelectorAll("li")).toHaveLength(2);
    expect(slower.textContent).toContain("Subagent lists: Each session's Subagents tab reads its subagents' files again");
    expect(calls()).toEqual([]);
    // Escape keeps it.
    fireEvent.keyDown(sheet, { key: "Escape" });
    expect(screen.queryByTestId("cache-clear-sheet")).toBeNull();
    expect(calls()).toEqual([]);
    fireEvent.click(screen.getByTestId("cache-clear"));
    await act(async () => {
      fireEvent.click(await screen.findByTestId("cache-clear-go"));
    });
    await waitFor(() => expect(screen.getByTestId("cache-switch-row").textContent).toContain("Cleared 47 MB."));
    expect(calls()).toEqual([{ clear: true }]);
    expect(screen.getByTestId("cache-switch-row").textContent).toContain("Using 0 B of 1.3 GB.");
  });

  it("a fixed limit is picked from the menu and said in plain words", async () => {
    await section();
    fireEvent.click(screen.getByRole("button", { name: "Size limit: Automatic" }));
    await act(async () => {
      fireEvent.click(await screen.findByTestId("cache-limit-1000"));
    });
    await waitFor(() => expect(screen.getByTestId("cache-limit-row").textContent).toContain("Fixed at 1 GB."));
    expect(calls()).toEqual([{ limit_mb: 1000 }]);
    expect(screen.getByRole("button", { name: "Size limit: 1 GB" })).toBeTruthy();
  });

  it("on a nearly full disk, says it keeps to its floor until there's room", async () => {
    seams().__AIJ_CACHE__ = { ...SAMPLE_CACHE, low_disk: true, budget_bytes: 64_000_000, used_bytes: 12_000_000, free_bytes: 3_100_000_000 };
    const row = await section();
    expect(row.textContent).toContain("Using 12 MB of 64 MB.");
    expect(within(row).getByRole("status").textContent).toBe("Your disk is nearly full, so the cache keeps to 64 MB and adds nothing until there's room.");
  });

  it("summarises off and on in one line", () => {
    expect(cacheSummary({ ...SAMPLE_CACHE, on: false })).toBe("Off: nothing is kept, so pages read your tools' files each time they open.");
    expect(cacheSummary(SAMPLE_CACHE)).toBe("Using 47 MB of 1.3 GB. Pages that read many files open at once.");
  });
});

describe("Privacy's door to it", () => {
  it("says what is kept and opens Local cache", () => {
    const opened = vi.fn();
    window.addEventListener("aij:open-settings", opened);
    render(<PrivacyPane />);
    const row = screen.getByTestId("privacy-reads-cache");
    expect(row.textContent).toContain("A cache on this computer, never sent");
    expect(screen.getByTestId("privacy-reads-last").textContent).toContain("kept nowhere");
    fireEvent.click(within(row).getByRole("button", { name: "Local cache…" }));
    expect((opened.mock.calls[0]![0] as CustomEvent).detail).toEqual({ section: "local-cache" });
    window.removeEventListener("aij:open-settings", opened);
    expect(targetOf("local-cache")).toEqual({ pane: "advanced", anchor: "local-cache" });
  });
});
