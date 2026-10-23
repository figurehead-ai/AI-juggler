/**
 * AJ-E1007, AJ-E1536 vitest — History: one search over project, Account, Provider,
 * model, branch, date and transcript text (snippet shown); Ended and
 * Archived lists; archive/unarchive in place; delete has no single-click
 * path (preview, then the typed word, then the token); replay is
 * labelled read-only with its history, lineage and process record;
 * retention shows a dry run before anything is removed.
 */
import { act, cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { History, matchesFields } from "./History";
import { DeleteInstanceDialog } from "../components/DeleteInstanceDialog";
import { ReplayDialog } from "../components/ReplayDialog";
import { StorageRetentionCard } from "../components/StorageRetentionCard";
import type { InstanceInfo, ProjectInfo } from "../lib/bridge";
import type { ConversationInfo } from "../lib/ipc/conversations";
import { ConversationsProvider } from "../lib/conversations";
import { ActionHostProvider } from "../components/ItemMenu";
import type { ActionHost } from "../lib/actions";
import type { JSX, ReactNode } from "react";

vi.mock("../components/ReplayTerminal", () => ({
  ReplayTerminal: ({ entries }: { entries: Array<{ role: string; text: string }> }) => (
    <div data-testid="replay-terminal">{entries.map((entry) => `${entry.role}: ${entry.text}`).join("\n")}</div>
  ),
}));

const w = () => window as unknown as Record<string, unknown>;
const calls = () => (w().__AIJ_HISTORY_CALLS__ as string[] | undefined) ?? [];
const DAY = 24 * 3_600_000;
const T = new Date(2026, 9, 1, 12).getTime();

const row = (id: string, status: string, over: Partial<InstanceInfo> = {}): InstanceInfo =>
  ({
    id,
    status,
    provider: "claude-code",
    account_id: "acc",
    account_label: "Work Max",
    model: "opus",
    git_branch: "feat/invoices",
    started_at_ms: T,
    last_activity_ms: T + 3_600_000,
    project_id: "p1",
    quota: null,
    tokens: null,
    cpu_pct: null,
    rss_bytes: null,
    child_count: 0,
    attention: null,
    needs_attention: null,
    machine_id: "this-mac",
    kind: "terminal",
    cwd: "/w",
    ...over,
  }) as InstanceInfo;

const projects = [{ id: "p1", name: "storefront" }] as ProjectInfo[];

function outsideOne(id: string, over: Partial<ConversationInfo> = {}): ConversationInfo {
  return {
    id,
    tool: "codex",
    account_id: "acc2",
    account_label: "Personal",
    project_id: "p1",
    cwd: "/w",
    title: "Tidy the release notes",
    git_branch: "main",
    started_ms: T,
    last_activity_ms: T + 7_200_000,
    tokens: null,
    live: false,
    session_id: null,
    ...over,
  };
}

/** History as the app has it: the conversations and the app's actions. */
function inApp(children: ReactNode, conversations: ConversationInfo[] = [], host: ActionHost = {}): JSX.Element {
  w().__AIJ_FIXTURE__ = { conversations };
  return (
    <ConversationsProvider>
      <ActionHostProvider host={host}>{children}</ActionHostProvider>
    </ConversationsProvider>
  );
}

const keys = (container: HTMLElement) =>
  Array.from(container.querySelectorAll<HTMLElement>("[data-testid='page-session-item']")).map((r) => r.dataset.key);

afterEach(() => {
  cleanup();
  delete w().__AIJ_FIXTURE__;
});
beforeEach(() => {
  for (const key of ["__AIJ_HISTORY_CALLS__", "__AIJ_TRANSCRIPT_HITS__", "__AIJ_REPLAY__", "__AIJ_DELETE_PREVIEW__", "__AIJ_ACCOUNT_STORAGE__", "__AIJ_RETENTION_PREVIEW__"]) {
    delete w()[key];
  }
});

describe("AJ-E1007 search", () => {
  it("matches every field the task names", () => {
    const s = row("s1", "finished");
    for (const query of ["storefront", "work max", "claude", "opus", "invoices", "2026-10-01"]) {
      expect(matchesFields(s, "storefront", query)).toBe(true);
    }
    expect(matchesFields(s, "storefront", "storefront codex")).toBe(false);
  });

  it("finds transcript text with its snippet, and lists Ended and Archived apart", async () => {
    w().__AIJ_TRANSCRIPT_HITS__ = [{ session_id: "s2", snippet: "…traced the flaky Stripe webhook retry…" }];
    const sessions = [row("s1", "finished"), row("s2", "stopped", { git_branch: null }), row("s3", "finished", { lifecycle: "archived" }), row("s4", "running")];
    const { container } = render(inApp(<History sessions={sessions} projects={projects} onChanged={() => {}} />));
    const ids = () => keys(container);
    expect(ids()).toEqual(["s/s1", "s/s2"]);
    fireEvent.change(container.querySelector("[data-testid='history-search']")!, { target: { value: "stripe" } });
    await waitFor(() => expect(ids()).toEqual(["s/s2"]));
    expect(container.querySelector("[data-testid='history-snippet']")!.textContent).toContain("Stripe webhook");
    fireEvent.change(container.querySelector("[data-testid='history-search']")!, { target: { value: "" } });
    fireEvent.click(container.querySelector("[data-testid='history-tab-archived']")!);
    expect(ids()).toEqual(["s/s3"]);
    // AJ-E1536: Unarchive is in the row's ⋯ menu.
    fireEvent.click(container.querySelector("[aria-label^='Actions for']")!);
    expect(document.querySelector("[data-testid='action-archive']")!.textContent).toContain("Unarchive");
    await act(async () => {
      fireEvent.click(document.querySelector("[data-testid='action-archive']")!);
    });
    expect(calls()).toContain("unarchive:s3");
  });
});

describe("AJ-E1536 History you can scan", () => {
  it("lists AI Juggler's ended sessions and the ones started outside it, each by what it was about, with its marks", async () => {
    const sessions = [row("s1", "finished", { title: "Fix the invoice export" }), row("s2", "running")];
    const { container } = render(inApp(<History sessions={sessions} projects={projects} onChanged={() => {}} />, [outsideOne("c1")]));
    await waitFor(() => expect(keys(container)).toEqual(["c/acc2/c1", "s/s1"]));
    const [outside, mine] = Array.from(container.querySelectorAll<HTMLElement>("[data-testid='page-session-item']"));
    expect(outside!.textContent).toContain("Tidy the release notes");
    expect(outside!.textContent).toContain("outside AI Juggler");
    expect(mine!.textContent).toContain("Fix the invoice export");
    for (const item of [outside!, mine!]) {
      expect(item.querySelector("[data-testid='page-session-project']")!.textContent).toContain("storefront");
      expect(item.querySelector("[data-testid='mark-project']")).not.toBeNull();
    }
    // Searching by what it was about finds the outside one too.
    fireEvent.change(container.querySelector("[data-testid='history-search']")!, { target: { value: "release notes" } });
    await waitFor(() => expect(keys(container)).toEqual(["c/acc2/c1"]));
  });

  it("a row opens its page; Delete is only in its ⋯ menu, last", async () => {
    const opened: string[] = [];
    const { container } = render(
      inApp(<History sessions={[row("s1", "finished")]} projects={projects} onChanged={() => {}} />, [], {
        openSessionById: (id) => opened.push(id),
        openHistory: (s) => opened.push(s.id),
      }),
    );
    fireEvent.click(container.querySelector("[data-testid='page-session-row']")!);
    expect(opened).toEqual(["s1"]);
    expect(container.textContent).not.toContain("Delete…");
    fireEvent.click(container.querySelector("[aria-label^='Actions for']")!);
    const items = Array.from(document.querySelectorAll<HTMLElement>("[data-testid^='action-']")).map((item) => item.dataset.testid);
    expect(items.at(-1)).toBe("action-delete");
    expect(items).toContain("action-replay");
  });
});

describe("AJ-E1007 delete", () => {
  it("has no single-click path: preview, then the typed word, then the token", async () => {
    let closed: boolean | null = null;
    const { container } = render(<DeleteInstanceDialog sessionId="s1" onClose={(deleted) => (closed = deleted)} />);
    await waitFor(() => expect(container.querySelector("[data-testid='delete-removes']")).not.toBeNull());
    expect(container.textContent).toContain("This cannot be undone");
    expect(container.querySelector("[data-testid='delete-keeps']")!.textContent).toContain("Provider's own transcript file");
    const go = () => container.querySelector<HTMLButtonElement>("[data-testid='delete-go']")!;
    expect(go().disabled).toBe(true);
    fireEvent.click(go());
    expect(calls()).toEqual(["delete-preview:s1"]);
    fireEvent.change(container.querySelector("[data-testid='delete-confirm']")!, { target: { value: "delet" } });
    expect(go().disabled).toBe(true);
    fireEvent.change(container.querySelector("[data-testid='delete-confirm']")!, { target: { value: "delete" } });
    await act(async () => {
      fireEvent.click(go());
    });
    expect(calls()).toEqual(["delete-preview:s1", "delete:s1:mock-token"]);
    expect(closed).toBe(true);
  });

  it("a refused delete offers no delete button", async () => {
    w().__AIJ_DELETE_PREVIEW__ = {
      session_id: "s1", account_label: "Work Max", removes: [], keeps: [], token: null,
      refused: "It is part of a switch history — deleting it would break that lineage. Archive it instead.",
    };
    const { container } = render(<DeleteInstanceDialog sessionId="s1" onClose={() => {}} />);
    await waitFor(() => expect(container.querySelector("[data-testid='delete-refused']")).not.toBeNull());
    expect(container.querySelector("[data-testid='delete-go']")).toBeNull();
  });
});

describe("AJ-E1007 replay", () => {
  it("is read-only, with history, lineage and the process record", async () => {
    w().__AIJ_REPLAY__ = {
      session_id: "s1", transcript_path: "/t/s1.jsonl", transcript: "{}", truncated: false, bytes: 2048,
      sha256: "ab12cd34ef56ab12cd34ef56", entries: [{ role: "user", text: "fix the invoice export" }],
      history: [{ kind: "status", what: "finished", at_ms: T }, { kind: "lifecycle", what: "archived", at_ms: T + DAY }],
      lineage: [{ direction: "in", other_account: "Personal", other_provider: "codex", state: "confirmed", fidelity: "F2" }],
      processes: [{ pid: 4242, role: "root", state: "exited", exit_kind: "clean", first_seen_ms: T, last_seen_ms: T + 60_000 }],
    };
    const { container } = render(<ReplayDialog sessionId="s1" onClose={() => {}} />);
    await waitFor(() => expect(container.querySelector("[data-testid='replay-terminal']")).not.toBeNull());
    expect(container.querySelector("#replay-title")!.textContent).toBe("Replay · read-only");
    expect(container.querySelector("[data-testid='replay-proof']")!.textContent).toContain("sha256 ab12cd34ef56");
    expect(container.textContent).toContain("user: fix the invoice export");
    expect(container.querySelector("[data-testid='replay-history-list']")!.textContent).toContain("· archived");
    expect(container.querySelector("[data-testid='replay-lineage-list']")!.textContent).toContain("Arrived from Personal (codex)");
    expect(container.querySelector("[data-testid='replay-process-list']")!.textContent).toContain("root 4242 · exited · clean");
  });
});

describe("AJ-E1007 retention", () => {
  it("keeps forever by default; cleanup is a dry run first", async () => {
    w().__AIJ_ACCOUNT_STORAGE__ = [
      { account_id: "acc", label: "Work Max", retention_days: null, instances: 12, history_events: 40, index_lines: 3200, transcript_bytes: 5 * 1024 * 1024 },
    ];
    w().__AIJ_RETENTION_PREVIEW__ = {
      account_id: "acc", retention_days: 90, token: "dry-1",
      deletes: [{ session_id: "s-old", project: "storefront", last_active_ms: T - 100 * DAY, kept_because: null }],
      kept: [{ session_id: "s-lineage", project: null, last_active_ms: T - 120 * DAY, kept_because: "It is part of a switch history" }],
      keeps: ["The Provider's own transcript files"],
    };
    const { container } = render(<StorageRetentionCard />);
    await waitFor(() => expect(container.querySelector("[data-testid='storage-row']")).not.toBeNull());
    expect(container.querySelector("[data-testid='storage-usage']")!.textContent).toContain("transcripts 5.0 MB");
    expect((container.querySelector("[data-testid='storage-review']") as HTMLButtonElement).disabled).toBe(true);
    await act(async () => {
      fireEvent.change(container.querySelector("[data-testid='storage-retention']")!, { target: { value: "90" } });
    });
    expect(calls()).toContain("retention:acc:90");
    await waitFor(() => expect((container.querySelector("[data-testid='storage-review']") as HTMLButtonElement).disabled).toBe(false));
    await act(async () => {
      fireEvent.click(container.querySelector("[data-testid='storage-review']")!);
    });
    expect(container.querySelector("[data-testid='dry-run-deletes']")!.textContent).toContain("s-old");
    expect(container.querySelector("[data-testid='dry-run-kept']")!.textContent).toContain("switch history");
    expect(calls().some((call) => call.startsWith("retention-apply"))).toBe(false);
    await act(async () => {
      fireEvent.click(container.querySelector("[data-testid='dry-run-apply']")!);
    });
    expect(calls()).toContain("retention-apply:acc:dry-1");
  });
});
