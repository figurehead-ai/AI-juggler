/**
 * ADR-0020 — the chart kit's contracts: axes name values the chart
 * reaches, numbers read short with the exact one on hover, every chart is
 * one keyboard stop that reads its marks aloud, the empty state keeps the
 * frame, "Show as table" shows the numbers, and colour follows the entity.
 */
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { ChartBody, Legend, TableView } from "./ChartCard";
import { Donut } from "./Donut";
import { Gauge } from "./Gauge";
import { Heatmap, hourCells } from "./Heatmap";
import { ShareBar, StatTile } from "./StatTile";
import { TimeChart, timeTable } from "./TimeChart";
import { change, exact, niceAxis, percentOf, short, stackOf, usdWords } from "./scale";
import { modelName, seriesColor } from "./colors";
import { accountColor } from "../../lib/palette";

afterEach(cleanup);

describe("scale", () => {
  it("an axis's top is a clean step at or above the largest value", () => {
    expect(niceAxis(83)).toEqual({ top: 100, ticks: [0, 25, 50, 75, 100] });
    expect(niceAxis(41_000_000)).toEqual({ top: 60_000_000, ticks: [0, 20_000_000, 40_000_000, 60_000_000] });
    expect(niceAxis(0)).toEqual({ top: 1, ticks: [0, 1] });
    for (const max of [3, 17, 260, 9_999, 1.2e9]) {
      const { top, ticks } = niceAxis(max);
      expect(top).toBeGreaterThanOrEqual(max);
      expect(ticks[ticks.length - 1]).toBe(top);
    }
  });

  it("numbers read short, in full on hover", () => {
    expect(short(8_012_345, "tokens")).toBe("8.0M");
    expect(exact(8_012_345, "tokens")).toBe("8,012,345 tokens");
    expect(usdWords(490.12)).toBe("$490");
    expect(usdWords(1284.4)).toBe("$1,284");
    expect(usdWords(0.42)).toBe("$0.42");
    expect(usdWords(12_300)).toBe("$12.3k");
    expect(percentOf(1, 1000)).toBe("<1%");
    expect(percentOf(250, 1000)).toBe("25%");
  });

  it("the change says up, down, the same, or new", () => {
    expect(change(118, 100)).toEqual({ dir: "up", words: "18%" });
    expect(change(80, 100)).toEqual({ dir: "down", words: "20%" });
    expect(change(100.4, 100)).toEqual({ dir: "same", words: "same" });
    expect(change(5, 0)).toEqual({ dir: "new", words: "new" });
  });

  it("a stack leaves zeros out and goes bottom-up in the series' order", () => {
    const stack = stackOf(
      [
        { key: "a", label: "A", values: [3], color: "red" },
        { key: "b", label: "B", values: [0], color: "blue" },
        { key: "c", label: "C", values: [2], color: "green" },
      ],
      0,
    );
    expect(stack).toEqual([
      { key: "a", from: 0, to: 3 },
      { key: "c", from: 3, to: 5 },
    ]);
  });
});

describe("colour follows the entity", () => {
  it("a profile's series wears its badge's colour; Other is neutral", () => {
    expect(seriesColor("profile", "claude-work")).toBe(accountColor("claude-work"));
    expect(seriesColor("model", "__other")).toBe("var(--color-faint)");
    expect(seriesColor("model", "claude-opus-4-5")).toBe(seriesColor("model", "claude-opus-4-1-20250805"));
  });

  it("models read as people say them", () => {
    expect(modelName("claude-opus-4-5-20251101")).toBe("Opus 4.5");
    expect(modelName("claude-sonnet-4-5")).toBe("Sonnet 4.5");
    expect(modelName("gpt-5-codex")).toBe("GPT-5 Codex");
    expect(modelName("")).toBe("Model not recorded");
  });
});

const edges = [0, 10, 20, 30];
const series = [
  { key: "work", label: "Work", values: [5_000_000, 0, 1_000_000], color: "red" },
  { key: "codex", label: "Codex Main", values: [1_000_000, 2_000_000, 0], color: "blue" },
];

describe("charts", () => {
  it("a time chart is one keyboard stop; the arrows read each bucket aloud", () => {
    render(<TimeChart edges={edges} series={series} unit="tokens" label="Tokens per day" bucketWords={(start) => `Day ${start / 10 + 1}`} tickWords={(start) => `D${start / 10 + 1}`} />);
    const chart = screen.getByTestId("time-chart");
    expect(chart.getAttribute("tabindex")).toBe("0");
    expect(chart.getAttribute("aria-label")).toContain("Tokens per day");
    expect(chart.querySelectorAll("[data-testid='time-bar']").length).toBe(3);
    fireEvent.keyDown(chart, { key: "ArrowRight" });
    expect(screen.getByText(/Day 1: 6\.0M in all; Work 5\.0M, Codex Main 1\.0M/)).toBeTruthy();
    expect(screen.getByTestId("chart-tooltip").textContent).toContain("5.0M");
    fireEvent.keyDown(chart, { key: "End" });
    expect(screen.getByText(/Day 3: 1\.0M in all/)).toBeTruthy();
    fireEvent.keyDown(chart, { key: "Escape" });
    expect(screen.queryByTestId("chart-tooltip")).toBeNull();
  });

  it("its table has a row per bucket, newest first, exact numbers", () => {
    const table = timeTable(edges, series, "tokens", (start) => `Day ${start / 10 + 1}`);
    expect(table.rows[0]).toEqual(["Day 3", "1,000,000 tokens", "0 tokens", "1,000,000 tokens"]);
    render(<TableView table={table} caption="Usage" />);
    expect(screen.getAllByRole("row").length).toBe(4);
  });

  it("the empty state keeps the chart's height and says why", () => {
    render(
      <ChartBody empty emptyWords="No usage in this range yet" height={240} table={{ columns: [], rows: [] }} showTable={false} caption="">
        <span>chart</span>
      </ChartBody>,
    );
    const empty = screen.getByTestId("chart-empty");
    expect(empty.style.height).toBe("240px");
    expect(empty.textContent).toBe("No usage in this range yet");
    expect(screen.queryByText("chart")).toBeNull();
  });

  it("a legend names two or more series, never one", () => {
    const { rerender } = render(<Legend label="Series" items={[{ key: "a", label: "A", color: "red" }]} />);
    expect(screen.queryByTestId("chart-legend")).toBeNull();
    rerender(<Legend label="Series" items={[{ key: "a", label: "A", color: "red" }, { key: "b", label: "B", color: "blue", value: "2M" }]} />);
    expect(screen.getByTestId("chart-legend").textContent).toContain("B2M");
  });

  it("a donut lists each part with its value and share; picking a row opens it", () => {
    const picked: string[] = [];
    render(
      <Donut
        items={[
          { key: "web", label: "web-app", value: 750, color: "red" },
          { key: "api", label: "api", value: 250, color: "blue" },
        ]}
        unit="tokens"
        label="Tokens by project"
        centre={{ value: "1k", words: "tokens" }}
        onPick={(key) => picked.push(key)}
      />,
    );
    const legend = screen.getByTestId("donut-legend");
    expect(legend.textContent).toContain("75%");
    expect(legend.textContent).toContain("25%");
    fireEvent.click(screen.getByRole("button", { name: /api/ }));
    expect(picked).toEqual(["api"]);
    expect(screen.getAllByTestId("donut-segment").length).toBe(2);
  });

  it("a heatmap folds hours into weekday and hour, and reads its cells", () => {
    const monday9 = new Date(2026, 9, 19, 9).getTime();
    const cells = hourCells([monday9, monday9 + 7 * 86_400_000]);
    expect(cells.find((cell) => cell.row === 0 && cell.col === 9)!.count).toBe(2);
    render(<Heatmap cells={cells} columns={24} label="When" columnLabels={[]} />);
    const map = screen.getByTestId("heatmap");
    fireEvent.keyDown(map, { key: "ArrowDown" });
    expect(screen.getByText("Mon 00:00: no usage")).toBeTruthy();
  });

  it("a gauge says not measured in words, never as an empty or full ring", () => {
    render(<Gauge name="5-hour window" left={null} state="unmeasured" />);
    expect(screen.getByTestId("gauge").textContent).toContain("Not measured yet");
    expect(screen.getByTestId("gauge").querySelector("circle[stroke-dasharray]")).toBeNull();
    cleanup();
    render(<Gauge name="This week" left={41} resets="Mon 10:40 AM" resetsIn="in 3 days" />);
    expect(screen.getByRole("img").getAttribute("aria-label")).toBe("This week: 41% left, resets Mon 10:40 AM, in 3 days");
  });

  it("a stat tile shows the number short, exact on hover, and its change", () => {
    render(<StatTile label="Tokens" value={8_012_345} unit="tokens" before={10_000_000} period="the 30 days before" better="down" />);
    const value = screen.getByTestId("stat-value");
    expect(value.textContent).toBe("8.0M");
    expect(value.getAttribute("title")).toBe("8,012,345 tokens");
    expect(screen.getByTestId("stat-delta").dataset.dir).toBe("down");
    expect(screen.getByTestId("stat-delta").textContent).toContain("down 20% on the 30 days before");
  });

  it("a share bar is the item against the biggest, its percentage of the total", () => {
    render(<ShareBar parts={[{ key: "w", label: "Work", value: 50, color: "red" }]} total={200} scale={100} unit="tokens" label="web-app" />);
    const bar = screen.getByTestId("share-bar");
    expect(bar.textContent).toBe("25%");
    expect((bar.querySelector("span[title]") as HTMLElement).style.width).toBe("50%");
  });
});
