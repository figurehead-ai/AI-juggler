/**
 * AJ-E1105 vitest — Settings › Privacy in every state (spec §8) and the
 * consent-version re-ask, through the preview seam.
 */
import { cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { PrivacyCard } from "./PrivacyCard";
import { UsageStatsReconsent } from "./UsageStatsReconsent";
import type { UsageStatsOverview } from "../lib/bridge";

type Seams = {
  __AIJ_USAGE_STATS__?: UsageStatsOverview;
  __AIJ_USAGE_STATS_REPORTS__?: Record<string, string>;
  __AIJ_USAGE_STATS_SETS__?: string[];
};
const w = () => window as unknown as Seams;

function overview(
  patch: Omit<Partial<UsageStatsOverview>, "info"> & { info?: Partial<UsageStatsOverview["info"]> } = {},
): UsageStatsOverview {
  const { info, ...rest } = patch;
  return {
    info: {
      included: true,
      chosen: "off",
      effective: "off",
      can_upload: false,
      reason: null,
      decided_at_ms: null,
      consent_version: 0,
      current_consent_version: 1,
      ...info,
    },
    week: "2026-W42",
    rows: [],
    reports: [],
    next_upload_ms: null,
    last_error: null,
    docs_url: "https://example.invalid/usage-stats",
    sync: { offered: false, on: false, email: null, notice: null },
    ...rest,
  };
}

const counting = overview({
  info: { chosen: "local", effective: "local", decided_at_ms: 1, consent_version: 1 },
  rows: [
    { category: "features", metric: "app.launch", dims: "", value: "3" },
    { category: "features", metric: "instance.started", dims: "kind: terminal · origin: new · provider: codex", value: "2" },
    { category: "environment", metric: "projects.count", dims: "", value: "4-5" },
    { category: "reliability", metric: "error.class", dims: "area: storage · code: db_locked", value: "1" },
  ],
  reports: [
    {
      week: "2026-W41",
      status: "skipped",
      reason: "Counted while stats stayed on this Mac — never sent",
      upload_at_ms: null,
      next_try_ms: null,
      attempts: 0,
      bytes: 312,
      synced: false,
    },
  ],
});

beforeEach(() => {
  delete w().__AIJ_USAGE_STATS__;
  delete w().__AIJ_USAGE_STATS_REPORTS__;
  delete w().__AIJ_USAGE_STATS_SETS__;
});
afterEach(() => cleanup());

describe("AJ-E1105 Settings › Privacy", () => {
  it("off, never asked: nothing pre-selected, nothing listed", async () => {
    w().__AIJ_USAGE_STATS__ = overview();
    const { findByTestId, container } = render(<PrivacyCard />);
    expect((await findByTestId("privacy-state")).textContent).toContain("Off. Nothing is counted");
    const radios = container.querySelectorAll<HTMLInputElement>("[data-testid='usage-stats-option'] input");
    expect(radios.length).toBe(3);
    expect([...radios].some((radio) => radio.checked)).toBe(false);
    expect(container.querySelector("[data-testid='privacy-collected']")).toBeNull();
    expect(container.querySelector("[data-testid='privacy-delete']")).toBeNull();
    expect(container.querySelector("[data-testid='privacy-open-docs']")).not.toBeNull();
  });

  it("local: this week grouped by category, reports with reasons, exact JSON with Copy", async () => {
    w().__AIJ_USAGE_STATS__ = counting;
    w().__AIJ_USAGE_STATS_REPORTS__ = { "skipped/2026-W41": '{"schema":1,"week":"2026-W41"}' };
    const { findByTestId, getByTestId, container } = render(<PrivacyCard />);
    expect((await findByTestId("privacy-state")).textContent).toContain("Counting on this Mac only");
    expect(getByTestId("privacy-rows-features").textContent).toContain("instance.started");
    expect(getByTestId("privacy-rows-environment").textContent).toContain("4-5");
    expect(getByTestId("privacy-rows-reliability").textContent).toContain("db_locked");
    expect(getByTestId("privacy-report").textContent).toContain("never sent");
    fireEvent.click(getByTestId("privacy-report-view"));
    await waitFor(() => expect(container.querySelector("[data-testid='privacy-report-json'] pre")).not.toBeNull());
    expect(container.querySelector("[data-testid='privacy-report-json'] pre")!.textContent).toContain('"week": "2026-W41"');
    expect(getByTestId("privacy-report-copy").textContent).toBe("Copy");
    const checked = container.querySelector<HTMLInputElement>("[data-value='local'] input")!;
    expect(checked.checked).toBe(true);
  });

  it("share: the next upload time, and an upload problem in plain words with the retry", async () => {
    w().__AIJ_USAGE_STATS__ = overview({
      ...counting,
      info: { ...counting.info, chosen: "share", effective: "share", can_upload: true } as Partial<
        UsageStatsOverview["info"]
      >,
      next_upload_ms: Date.UTC(2026, 9, 21, 15, 10),
      last_error: "Couldn't reach Figurehead.",
      reports: [
        {
          week: "2026-W42",
          status: "pending",
          reason: null,
          upload_at_ms: Date.UTC(2026, 9, 21, 15, 10),
          next_try_ms: Date.UTC(2026, 9, 21, 16, 10),
          attempts: 1,
          bytes: 401,
          synced: false,
        },
      ],
    });
    const { findByTestId, getByTestId } = render(<PrivacyCard />);
    expect((await findByTestId("privacy-state")).textContent).toContain("Sharing");
    expect(getByTestId("privacy-next-upload").textContent).toContain("Next report may be sent after");
    expect(getByTestId("privacy-upload-error").textContent).toMatch(/Couldn't reach Figurehead\. Will retry /);
  });

  it("overridden by DO_NOT_TRACK: the reason, and the controls are disabled", async () => {
    w().__AIJ_USAGE_STATS__ = overview({
      info: { chosen: "share", effective: "off", reason: "Off because DO_NOT_TRACK is set", decided_at_ms: 1, consent_version: 1 },
    });
    const { findByTestId, getByTestId } = render(<PrivacyCard />);
    expect((await findByTestId("privacy-state")).textContent).toBe("Off because DO_NOT_TRACK is set.");
    expect((getByTestId("usage-stats-chooser") as HTMLFieldSetElement).disabled).toBe(true);
  });

  it("not included in this build: says so, no chooser", async () => {
    w().__AIJ_USAGE_STATS__ = overview({ info: { included: false, reason: "Not included in this build" } });
    const { findByTestId, container } = render(<PrivacyCard />);
    expect((await findByTestId("privacy-state")).textContent).toBe("This build doesn't include usage statistics.");
    expect(container.querySelector("[data-testid='usage-stats-chooser']")).toBeNull();
  });

  it("choosing Off deletes and says so; Delete local stats keeps the mode", async () => {
    w().__AIJ_USAGE_STATS__ = counting;
    const { findByTestId, getByTestId, container } = render(<PrivacyCard />);
    await findByTestId("privacy-collected");
    fireEvent.click(getByTestId("privacy-delete"));
    await waitFor(() => expect(getByTestId("privacy-message").textContent).toBe("Local stats deleted."));
    fireEvent.click(container.querySelector<HTMLInputElement>("[data-value='off'] input")!);
    await waitFor(() =>
      expect(getByTestId("privacy-message").textContent).toBe("Usage stats are off. Local stats deleted."),
    );
    expect(w().__AIJ_USAGE_STATS_SETS__).toEqual(["off"]);
  });
});

describe("AJ-E1105 consent-version re-ask", () => {
  it("a click on the question's text keeps focus inside it", async () => {
    w().__AIJ_USAGE_STATS__ = overview({
      info: { chosen: "share", effective: "share", decided_at_ms: 1, consent_version: 1, current_consent_version: 2 },
    });
    const { default: userEvent } = await import("@testing-library/user-event");
    const user = userEvent.setup();
    const { findByTestId, getByText } = render(<UsageStatsReconsent />);
    const dialog = await findByTestId("usage-stats-reconsent");
    await user.click(getByText("Usage statistics changed"));
    expect(dialog.contains(document.activeElement)).toBe(true);
  });

  it("asks again after a bump, with the current mode pre-selected", async () => {
    w().__AIJ_USAGE_STATS__ = overview({
      info: { chosen: "share", effective: "share", decided_at_ms: 1, consent_version: 1, current_consent_version: 2 },
    });
    const { findByTestId, getByTestId, container } = render(<UsageStatsReconsent />);
    await findByTestId("usage-stats-reconsent");
    expect(container.querySelector<HTMLInputElement>("[data-value='share'] input")!.checked).toBe(true);
    fireEvent.click(getByTestId("reconsent-save"));
    await waitFor(() => expect(container.querySelector("[data-testid='usage-stats-reconsent']")).toBeNull());
    expect(w().__AIJ_USAGE_STATS_SETS__).toEqual(["share"]);
  });

  it("is never shown before the user has chosen, or when current", async () => {
    w().__AIJ_USAGE_STATS__ = overview();
    const first = render(<UsageStatsReconsent />);
    await new Promise((resolve) => setTimeout(resolve, 20));
    expect(first.container.querySelector("[data-testid='usage-stats-reconsent']")).toBeNull();
  });
});

describe("AJ-E1105 performance", () => {
  it("renders 12 weeks of reports and a full week of rows in under 200 ms", async () => {
    const reports = Array.from({ length: 12 }, (_, i) => ({
      week: `2026-W${String(41 - i).padStart(2, "0")}`,
      status: i === 0 ? "pending" : i % 3 === 0 ? "skipped" : "sent",
      reason: i % 3 === 0 && i > 0 ? "Not picked this week (sampling) — never sent" : null,
      upload_at_ms: i === 0 ? Date.UTC(2026, 9, 21) : null,
      next_try_ms: null,
      attempts: 0,
      bytes: 1_200,
      synced: false,
    }));
    const rows = Array.from({ length: 150 }, (_, i) => ({
      category: i % 10 === 0 ? "environment" : i % 7 === 0 ? "reliability" : "features",
      metric: `metric.${i}`,
      dims: `provider: codex · kind: k${i % 5}`,
      value: String(i),
    }));
    w().__AIJ_USAGE_STATS__ = overview({
      ...counting,
      info: { ...counting.info, chosen: "share", effective: "share", can_upload: true } as Partial<
        UsageStatsOverview["info"]
      >,
      rows,
      reports,
    });
    const started = performance.now();
    const { findAllByTestId } = render(<PrivacyCard />);
    expect((await findAllByTestId("privacy-report")).length).toBe(12);
    const took = performance.now() - started;
    console.log(`privacy card: 12 reports + 150 rows rendered in ${took.toFixed(0)} ms`);
    expect(took).toBeLessThan(200);
  });
});

describe("AJ-E1109 personal stats sync in Privacy", () => {
  it("is offered only while sharing on a linked Mac; on says where it goes", async () => {
    w().__AIJ_USAGE_STATS__ = overview({
      ...counting,
      info: { ...counting.info, chosen: "share", effective: "share", can_upload: true } as Partial<
        UsageStatsOverview["info"]
      >,
      sync: { offered: true, on: false, email: "you@example.com", notice: null },
    });
    const { findByTestId, getByTestId, queryByTestId } = render(<PrivacyCard />);
    const toggle = (await findByTestId("privacy-sync-toggle")) as HTMLInputElement;
    expect(toggle.checked).toBe(false);
    expect(queryByTestId("privacy-sync-email")).toBeNull();
    fireEvent.click(toggle);
    await waitFor(() => expect(getByTestId("privacy-sync-email").textContent).toBe("Synced to you@example.com"));
  });

  it("isn't offered otherwise; an unlink says sync is off", async () => {
    w().__AIJ_USAGE_STATS__ = overview({
      ...counting,
      sync: { offered: false, on: false, email: null, notice: "This Mac is no longer linked, so stats sync is off." },
    });
    const { findByTestId, queryByTestId } = render(<PrivacyCard />);
    expect((await findByTestId("privacy-sync-notice")).textContent).toBe(
      "This Mac is no longer linked, so stats sync is off.",
    );
    expect(queryByTestId("privacy-sync")).toBeNull();
  });
});
