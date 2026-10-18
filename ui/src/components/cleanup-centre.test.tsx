import { describe, expect, it } from "vitest";
import { fireEvent, render, waitFor, within } from "@testing-library/react";
import {
  CleanupCentre,
  type CleanupBridge,
  type CleanupInventoryData,
  type CleanupStatusBridge,
} from "../screens/CleanupCentre";
import { ProcessRow, type CleanupRowInfo } from "./ProcessRow";

function row(overrides: Partial<CleanupRowInfo> = {}): CleanupRowInfo {
  return {
    machine_id: "machine-local",
    pid: 8123,
    pgid: 8100,
    role: "helper",
    state: "orphaned",
    state_label: "Still running after its Instance closed",
    role_guess: "python3 (guess)",
    exe_path: "/usr/bin/python3",
    cmdline: "python3 -m http.server 5173",
    age_ms: 9_000_000,
    rss_bytes: 48 * 1024 * 1024,
    cpu_pct: 1.4,
    ports: "5173",
    hog: false,
    hog_rules: [],
    evidence: {
      root_pid: 8100,
      parent_pid: 8100,
      pgid: 8100,
      escaped_group: false,
      verification: {
        verdict: "verified",
        detail: "start-time and exe match the live process",
      },
      first_seen: 1_728_800_000_000,
      last_seen: 1_728_809_000_000,
    },
    ...overrides,
  };
}

function data(overrides: Partial<CleanupInventoryData> = {}): CleanupInventoryData {
  return {
    groups: [
      {
        session: "sess-orphan",
        provider: "claude-code",
        account: "Work Max",
        project: "web",
        machine: "machine-local",
        healthy: false,
        rows: [row()],
      },
      {
        session: "sess-clean",
        provider: "claude-code",
        account: "Team Max",
        project: "api",
        machine: "machine-local",
        healthy: true,
        rows: [
          row({
            pid: 4400,
            state: "running",
            state_label: "Running",
            ports: null,
            cmdline: "node vite",
            evidence: {
              root_pid: 4400,
              parent_pid: null,
              pgid: 4400,
              escaped_group: false,
              verification: { verdict: "verified", detail: "match" },
              first_seen: 1,
              last_seen: 2,
            },
          }),
        ],
      },
    ],
    clean_instances: 2,
    local_machine: "machine-local",
    ledger_degraded: false,
    history: null,
    ...overrides,
  };
}

describe("CleanupCentre (AJ-E506)", () => {
  it("renders groups with the verbatim pass2 §2 state labels", async () => {
    const bridge: CleanupBridge = { load: () => Promise.resolve(data()) };
    const view = render(<CleanupCentre bridge={bridge} />);
    await waitFor(() => expect(view.getByTestId("cleanup-group-sess-orphan")).toBeTruthy());
    expect(view.getByTestId("cleanup-row-8123").textContent ?? "").toContain(
      "Still running after its Instance closed",
    );
    // Icon+text badge (never colour alone).
    expect(view.getByTestId("cleanup-state-8123").textContent ?? "").toMatch(/⚑/);
    // Attribution columns.
    expect(view.getByTestId("cleanup-group-sess-orphan").textContent ?? "").toContain(
      "claude-code · Work Max · web",
    );
    expect(view.getByTestId("cleanup-row-8123").textContent ?? "").toContain(":5173");
    view.unmount();
  });

  it("AJ-E507: kill affordance on verified live rows ONLY — never on unverifiable/exited", async () => {
    const withUnverifiable = data({
      groups: [
        {
          session: "sess-orphan",
          provider: "claude-code",
          account: "Work Max",
          project: "web",
          machine: "machine-local",
          healthy: false,
          rows: [
            row(),
            row({
              pid: 9100,
              state: "unverifiable",
              state_label: "Unverifiable — will not be touched",
              evidence: {
                root_pid: 8100,
                parent_pid: 8100,
                pgid: 9100,
                escaped_group: true,
                verification: { verdict: "unverifiable", detail: "differs" },
                first_seen: 1,
                last_seen: 2,
              },
            }),
            row({
              pid: 9200,
              state: "exited",
              state_label: "Exited",
              evidence: {
                root_pid: 8100,
                parent_pid: 8100,
                pgid: 8100,
                escaped_group: false,
                verification: { verdict: "unknown", detail: "exited" },
                first_seen: 1,
                last_seen: 2,
              },
            }),
          ],
        },
      ],
    });
    const bridge: CleanupBridge = { load: () => Promise.resolve(withUnverifiable) };
    const view = render(<CleanupCentre bridge={bridge} />);
    await waitFor(() => expect(view.getByTestId("cleanup-group-sess-orphan")).toBeTruthy());
    // The verified orphan offers the staged sheet…
    expect(view.getByTestId("cleanup-kill-8123").textContent).toContain("Terminate");
    // …the unverifiable and exited rows render NO affordance at all
    // (not disabled — absent; Req.2).
    expect(view.queryByTestId("cleanup-kill-9100")).toBeNull();
    expect(view.queryByTestId("cleanup-kill-9200")).toBeNull();
    view.unmount();
  });

  it("collapses healthy trees by default and expands the evidence chain on demand", async () => {
    const bridge: CleanupBridge = { load: () => Promise.resolve(data()) };
    const view = render(<CleanupCentre bridge={bridge} />);
    await waitFor(() => expect(view.getByTestId("cleanup-group-sess-clean")).toBeTruthy());
    // Healthy group collapsed: its running row is hidden.
    expect(view.queryByTestId("cleanup-row-4400")).toBeNull();
    // Orphan group expanded by default; the expansion is offered
    // (title affordance) and opens the evidence panel.
    const toggle = view
      .getByTestId("cleanup-row-8123")
      .querySelector("button") as HTMLButtonElement;
    expect(toggle.title).toContain("Why does Juggler think this is mine?");
    fireEvent.click(toggle);
    expect(view.getByTestId("cleanup-evidence-8123").textContent ?? "").toContain(
      "Why does Juggler think this is mine?",
    );
    view.unmount();
  });

  it("renders the copy-exact empty success and the degraded banner", async () => {
    const empty: CleanupBridge = {
      load: () =>
        Promise.resolve({
          local_machine: "machine-local",
          groups: [],
          clean_instances: 3,
          ledger_degraded: false,
          history: null,
        }),
    };
    const view = render(<CleanupCentre bridge={empty} />);
    await waitFor(() => expect(view.getByTestId("cleanup-empty")).toBeTruthy());
    expect(view.getByTestId("cleanup-empty").textContent ?? "").toContain(
      "Nothing left behind — 3 Instances running clean.",
    );
    view.unmount();

    const degraded: CleanupBridge = {
      load: () =>
      Promise.resolve({
          local_machine: "machine-local",
          groups: [],
          clean_instances: 0,
          ledger_degraded: true,
          history: null,
        }),
    };
    const view2 = render(<CleanupCentre bridge={degraded} />);
    await waitFor(() => expect(view2.getByTestId("cleanup-degraded-banner")).toBeTruthy());
    expect(view2.getByTestId("cleanup-degraded-banner").textContent ?? "").toContain(
      "Process tracking degraded — cleanup is read-only until restart.",
    );
    view2.unmount();
  });

  it("announces filter changes politely and exposes exactly the four filters", async () => {
    const bridge: CleanupBridge = { load: () => Promise.resolve(data()) };
    const view = render(<CleanupCentre bridge={bridge} />);
    await waitFor(() =>
      expect(view.getByTestId("cleanup-filter-announcement")).toBeTruthy(),
    );
    const group = view.getByRole("radiogroup", { name: "Cleanup filter" });
    const labels = within(group)
      .getAllByRole("radio")
      .map((radio) => radio.textContent);
    expect(labels).toEqual(["Needs attention", "All managed", "Re-adopted", "History (7 days)"]);
    expect(view.getByTestId("cleanup-filter-announcement").textContent ?? "").toContain(
      "Needs attention",
    );
    view.unmount();
  });

  it("AJ-E510: a down-period helper shows the distinct left-behind label, not the live-session orphan one", async () => {
    const bridge: CleanupBridge = {
      load: () =>
        Promise.resolve(
          data({
            groups: [
              {
                session: "sess-crash",
                provider: "claude-code",
                account: "Work Max",
                project: "web",
                machine: "machine-local",
                healthy: false,
                rows: [
                  row({
                    pid: 8301,
                    state: "orphaned",
                    down_period: true,
                    state_label: "left behind while Juggler was closed",
                  }),
                ],
              },
            ],
          }),
        ),
    };
    const view = render(<CleanupCentre bridge={bridge} />);
    await waitFor(() => expect(view.getByTestId("cleanup-row-8301")).toBeTruthy());
    expect(view.getByTestId("cleanup-state-8301").textContent ?? "").toContain(
      "left behind while Juggler was closed",
    );
    // Still the orphan icon class — the badge stays icon+text.
    expect(view.getByTestId("cleanup-state-8301").textContent ?? "").toMatch(/⚑/);
    view.unmount();
  });

  it("AJ-E510: a re-adopted Instance row reads Running — re-adopted with the ↻ badge", async () => {
    const bridge: CleanupBridge = {
      load: () =>
        Promise.resolve(
          data({
            groups: [
              {
                session: "sess-adopted",
                provider: "claude-code",
                account: "Work Max",
                project: "web",
                machine: "machine-local",
                healthy: false,
                rows: [
                  row({
                    pid: 8402,
                    state: "adopted",
                    state_label: "Running — re-adopted",
                  }),
                ],
              },
            ],
          }),
        ),
    };
    const view = render(<CleanupCentre bridge={bridge} />);
    await waitFor(() => expect(view.getByTestId("cleanup-state-8402")).toBeTruthy());
    expect(view.getByTestId("cleanup-state-8402").textContent ?? "").toContain(
      "Running — re-adopted",
    );
    expect(view.getByTestId("cleanup-state-8402").textContent ?? "").toMatch(/↻/);
    view.unmount();
  });
});

describe("Cleanup history (AJ-E509)", () => {
  const historyBridge = (rows: unknown[], emptyNote: string | null): CleanupBridge => ({
    load: (filter) =>
      Promise.resolve({
        local_machine: "machine-local",
        groups: [],
        clean_instances: 0,
        ledger_degraded: false,
        history:
          filter === "history"
            ? { rows: rows as never[], empty_note: emptyNote }
            : null,
      }),
  });

  it("renders audit rows with actor icon+text and the outcome vocabulary", async () => {
    const rows = [
      {
        id: 1,
        at_ms: 1_728_810_000_000,
        actor: "user:orphan_stop",
        automatic: false,
        pid: 8123,
        cmdline_trunc: "python3 -m http.server 5173",
        session_id: "s-1",
        provider: "claude-code",
        account_id: "Work Max",
        machine_id: "machine-local",
        outcome: "exited on TERM",
        ports_before: "5173",
        ports_after: "",
      },
      {
        id: 2,
        at_ms: 1_728_809_000_000,
        actor: "auto:tier-1",
        automatic: true,
        pid: 4400,
        cmdline_trunc: "node vite",
        session_id: "s-2",
        provider: "claude-code",
        account_id: null,
        machine_id: "machine-local",
        outcome: "TERM'd, outcome unknown",
        ports_before: null,
        ports_after: null,
      },
    ];
    const view = render(
      <CleanupCentre bridge={historyBridge(rows, null)} />,
    );
    // History is a filter, not the default — switch to it.
    fireEvent.click(view.getByRole("radio", { name: "History (7 days)" }));
    await waitFor(() => expect(view.getByTestId("cleanup-history")).toBeTruthy());
    expect(view.getByTestId("cleanup-history-row-1").textContent ?? "").toContain(
      "exited on TERM",
    );
    expect(view.getByTestId("cleanup-history-row-2").textContent ?? "").toContain(
      "TERM'd, outcome unknown",
    );
    // Icon+text actors (never colour alone).
    expect(view.getByTestId("cleanup-history-actor-1").textContent ?? "").toMatch(/👤/);
    expect(view.getByTestId("cleanup-history-actor-1").textContent ?? "").toContain("you");
    expect(view.getByTestId("cleanup-history-actor-2").textContent ?? '').toMatch(/⚙/);
    expect(view.getByTestId("cleanup-history-actor-2").textContent ?? "").toContain("automatic");
    // Export is user-initiated only — present, not automatic.
    expect(view.getByRole("button", { name: /Export audit log/ })).toBeTruthy();
    view.unmount();
  });

  it("renders the copy-exact empty-history note", async () => {
    const view = render(
      <CleanupCentre
        bridge={historyBridge([], "No cleanups yet — automatic cleanup is off by default.")}
      />,
    );
    fireEvent.click(view.getByRole("radio", { name: "History (7 days)" }));
    await waitFor(() => expect(view.getByTestId("cleanup-history-empty")).toBeTruthy());
    expect(view.getByTestId("cleanup-history-empty").textContent ?? "").toContain(
      "No cleanups yet — automatic cleanup is off by default.",
    );
    view.unmount();
  });
});

describe("ProcessRow evidence expansion (AJ-E506 Req.4)", () => {
  it("shows the complete chain: root, ppid, PGID containment, identity, seen", () => {
    const evidenceRow = row();
    const view = render(
      <ul>
        <ProcessRow row={evidenceRow} />
      </ul>,
    );
    // Rows start collapsed.
    expect(view.queryByTestId("cleanup-evidence-8123")).toBeNull();
    fireEvent.click(view.getByTestId("cleanup-row-8123").querySelector("button")!);
    const panel = view.getByTestId("cleanup-evidence-8123");
    const text = panel.textContent ?? "";
    expect(text).toContain("Instance root PID 8100");
    expect(text).toContain("first-seen parent 8100");
    expect(text).toContain("matches the root (contained)");
    expect(text).toContain("verified — start-time and exe match the live process");
    expect(text).toContain("never a one-click kill");
    view.unmount();
  });

  it("labels escaped groups honestly in the chain", async () => {
    const escaped = row({
      pid: 9100,
      evidence: {
        root_pid: 8100,
        parent_pid: 8100,
        pgid: 9100,
        escaped_group: true,
        verification: { verdict: "unverifiable", detail: "live start-time differs" },
        first_seen: 1,
        last_seen: 2,
      },
      state: "unverifiable",
      state_label: "Unverifiable — will not be touched",
    });
    const view = render(
      <ul>
        <ProcessRow row={escaped} />
      </ul>,
    );
    expect(view.getByTestId("cleanup-state-9100").textContent ?? "").toContain(
      "Unverifiable — will not be touched",
    );
    fireEvent.click(view.getByTestId("cleanup-row-9100").querySelector("button")!);
    expect(view.getByTestId("cleanup-evidence-9100").textContent ?? "").toContain(
      "escaped its process group",
    );
    view.unmount();
  });
});


describe("Cleanup automation status (AJ-E508)", () => {
  const statusBridgeOf = (
    status: import("../lib/bridge").CleanupStatusData,
    mutations: { pause?: boolean; resume?: boolean; decide?: [number, boolean] } = {},
  ): CleanupStatusBridge => ({
    status: () => Promise.resolve(status),
    pause: (paused) =>
      Promise.resolve({
        ...status,
        paused: mutations.pause ?? paused,
      }),
    resume: () =>
      Promise.resolve({
        ...status,
        suspended: false,
        suspended_until_ms: null,
      }),
    tier2Decide: (pid, keep) => {
      mutations.decide = [pid, keep];
      return Promise.resolve({ ...status, tier2_pending: [] });
    },
  });

  const tier0: import("../lib/bridge").CleanupStatusData = {
    paused: false,
    suspended: false,
    suspended_until_ms: null,
    tier2_pending: [],
  };

  it("shows the pause kill-switch; pausing shows the exact banner", async () => {
    const view = render(
      <CleanupCentre
        bridge={{ load: () => Promise.resolve(data()) }}
        statusBridge={statusBridgeOf({ ...tier0, paused: true })}
      />,
    );
    await waitFor(() => expect(view.getByTestId("cleanup-paused-banner")).toBeTruthy());
    expect(view.getByTestId("cleanup-paused-banner").textContent).toBe(
      "Automatic cleanup is paused.",
    );
    expect(view.getByTestId("cleanup-pause-toggle").textContent).toContain("Resume");
    view.unmount();
  });

  it("default state: no banners, the pause button offered", async () => {
    const view = render(
      <CleanupCentre
        bridge={{ load: () => Promise.resolve(data()) }}
        statusBridge={statusBridgeOf({ ...tier0 })}
      />,
    );
    await waitFor(() => expect(view.getByTestId("cleanup-pause-toggle")).toBeTruthy());
    expect(view.getByTestId("cleanup-pause-toggle").textContent).toContain("Pause");
    expect(view.queryByTestId("cleanup-paused-banner")).toBeNull();
    expect(view.queryByTestId("cleanup-suspended-banner")).toBeNull();
    expect(view.queryByTestId("cleanup-tier2-notices")).toBeNull();
    view.unmount();
  });

  it("suspension banner names the 24 h window and re-confirm clears it", async () => {
    const bridge = statusBridgeOf({
      ...tier0,
      suspended: true,
      suspended_until_ms: 1_728_810_000_000,
    });
    const view = render(
      <CleanupCentre bridge={{ load: () => Promise.resolve(data()) }} statusBridge={bridge} />,
    );
    await waitFor(() => expect(view.getByTestId("cleanup-suspended-banner")).toBeTruthy());
    expect(view.getByTestId("cleanup-suspended-banner").textContent).toContain(
      "suspended for 24 h",
    );
    fireEvent.click(view.getByTestId("cleanup-resume-button"));
    await waitFor(() => expect(view.queryByTestId("cleanup-suspended-banner")).toBeNull());
    view.unmount();
  });

  it("the Tier-2 notification names the process, counts down, and Undo keeps it running", async () => {
    const mutations: { decide?: [number, boolean] } = {};
    const bridge: CleanupStatusBridge = {
      status: () =>
        Promise.resolve({
          paused: false,
          suspended: false,
          suspended_until_ms: null,
          tier2_pending: [
            {
              pid: 8123,
              session: "s-1",
              cmdline_trunc: "node vite dev",
              deadline_ms: Date.now() + 30_000,
            },
          ],
        }),
      pause: () => Promise.resolve({ ...tier0 } as never),
      resume: () => Promise.resolve({ ...tier0 } as never),
      tier2Decide: (pid, keep) => {
        mutations.decide = [pid, keep];
        return Promise.resolve({ ...tier0 } as never);
      },
    };
    const view = render(
      <CleanupCentre bridge={{ load: () => Promise.resolve(data()) }} statusBridge={bridge} />,
    );
    await waitFor(() => expect(view.getByTestId("cleanup-tier2-notice-8123")).toBeTruthy());
    expect(view.getByTestId("cleanup-tier2-notice-8123").textContent).toContain(
      "node vite dev (PID 8123) survived a crash",
    );
    expect(view.getByTestId("cleanup-tier2-countdown-8123").textContent ?? "").toMatch(
      /\d+ seconds until the automatic tidy/,
    );
    fireEvent.click(view.getByTestId("cleanup-tier2-undo-8123"));
    await waitFor(() => expect(view.queryByTestId("cleanup-tier2-notice-8123")).toBeNull());
    expect(mutations.decide).toEqual([8123, true]);
    view.unmount();
  });
});

describe("AJ-E512 passive alerts (hog + idle nudge)", () => {
  const KILL_VERIFY_OK = {
    pid: 8123,
    session: "sess-orphan",
    verified: true,
    refusal: null,
    exe_path: "/usr/local/bin/node",
    cmdline: "node vite dev",
    cwd: "/srv/web",
    ports: [5173],
    pgid: 8100,
    escaped_group: false,
    members: [
      {
        pid: 8123,
        pgid: 8100,
        exe: "node",
        cmdline: "node vite dev",
        role_guess: "DevServer",
        unsaved_state_warning: null,
        from_ledger: true,
      },
    ],
    tree_pids: [8100, 8123],
    platform: "TERM → grace → KILL",
    grace_secs: 5,
    provider: "claude-code",
    account_id: "Work Max",
    project_id: "web",
  };

  it("renders the hog alert verbatim as a status message and stages the verified sheet", async () => {
    const view = render(
      <CleanupCentre
        bridge={{ load: () => Promise.resolve(data()) }}
        alertsBridge={{
          load: () =>
            Promise.resolve({
              alerts: [
                {
                  kind: "hog",
                  text: "node (vite) under Work Max holds 3.1 GiB — review in Cleanup Centre.",
                  pid: 8123,
                  session: "sess-orphan",
                },
              ],
            }),
        }}
        killBridge={{
          verify: async () => KILL_VERIFY_OK as never,
          execute: async () =>
            ({ refusal: null, members: [], cancelled: false, all_succeeded: true, still_holding: [], reoffer_pid: null }) as never,
          cancel: async () => undefined,
          liveness: async (pids: number[]) => pids.map(() => false),
        }}
      />,
    );
    await waitFor(() => expect(view.getByTestId("cleanup-alerts")).toBeTruthy());
    // A status message, not a dialog — and the exact backend text.
    const region = view.getByRole("status", { name: "Passive cleanup alerts" });
    expect(region.getAttribute("aria-live")).toBe("polite");
    expect(view.getByTestId("hog-alert-8123").textContent).toContain(
      "node (vite) under Work Max holds 3.1 GiB — review in Cleanup Centre.",
    );
    // Staging opens the SAME verified sheet the rows use — the only
    // kill path (app prepares, user executes).
    fireEvent.click(view.getByTestId("hog-stage-8123"));
    await waitFor(() => expect(view.getByTestId("kill-sheet-8123")).toBeTruthy());
    view.unmount();
  });

  it("renders the idle nudge passively — no action inside it", async () => {
    const view = render(
      <CleanupCentre
        bridge={{ load: () => Promise.resolve(data()) }}
        alertsBridge={{
          load: () =>
            Promise.resolve({
              alerts: [
                {
                  kind: "idle",
                  text: "python3 has been idle for 2h — review it in the Cleanup Centre.",
                  pid: 8124,
                  session: "sess-orphan",
                },
              ],
            }),
        }}
      />,
    );
    await waitFor(() => expect(view.getByTestId("idle-nudge-8124")).toBeTruthy());
    expect(view.getByTestId("idle-nudge-8124").textContent).toContain(
      "has been idle for 2h",
    );
    expect(view.getByTestId("idle-nudge-8124").querySelector("button")).toBeNull();
    view.unmount();
  });

  it("hides the region entirely when there are no alerts", async () => {
    const view = render(
      <CleanupCentre
        bridge={{ load: () => Promise.resolve(data()) }}
        alertsBridge={{ load: () => Promise.resolve({ alerts: [] }) }}
      />,
    );
    await waitFor(() => expect(view.getByTestId("cleanup-group-sess-orphan")).toBeTruthy());
    expect(view.queryByTestId("cleanup-alerts")).toBeNull();
    view.unmount();
  });
});

describe("AJ-E511 fleet machine dimension", () => {
  const killBridge = {
    verify: async (session: string, pid: number) => ({
      machine_id: "machine-local",
      pid,
      session,
      verified: true,
      refusal: null,
      exe_path: "/usr/bin/python3",
      cmdline: "python3 -m http.server 5173",
      cwd: "/srv",
      ports: [5173],
      pgid: 8100,
      escaped_group: false,
      members: [
        {
          pid, pgid: 8100, exe: "python3", cmdline: "python3 -m http.server 5173",
          role_guess: "DevServer", unsaved_state_warning: null, from_ledger: true,
        },
      ],
      tree_pids: [8100, pid],
      platform: "TERM → grace → KILL",
      grace_secs: 5,
      provider: "claude-code",
      account_id: "Work Max",
      project_id: "web",
    }) as never,
    execute: async () =>
      ({ refusal: null, members: [], cancelled: false, all_succeeded: true, still_holding: [], reoffer_pid: null }) as never,
    cancel: async () => undefined,
    liveness: async (pids: number[]) => pids.map(() => false),
  };

  it("renders This Mac on local rows and the machine id on foreign rows", async () => {
    const foreign = data({
      groups: [
        {
          session: "sess-fleet",
          provider: "claude-code",
          account: "Work Max",
          project: "web",
          machine: "machine-other",
          healthy: false,
          rows: [row({ pid: 9001, machine_id: "machine-other" })],
        },
      ],
    });
    const view = render(<CleanupCentre bridge={{ load: async () => data() }} />);
    await waitFor(() => expect(view.getByTestId("cleanup-machine-8123")).toBeTruthy());
    expect(view.getByTestId("cleanup-machine-8123").textContent).toBe("This Mac");
    view.unmount();

    const view2 = render(<CleanupCentre bridge={{ load: async () => foreign }} />);
    await waitFor(() => expect(view2.getByTestId("cleanup-machine-9001")).toBeTruthy());
    expect(view2.getByTestId("cleanup-machine-9001").textContent).toBe("machine-other");
    view2.unmount();
  });

  it("foreign rows are display-only — no Terminate affordance; local rows keep theirs", async () => {
    const mixed = data({
      groups: [
        {
          session: "sess-fleet",
          provider: "claude-code",
          account: "Work Max",
          project: "web",
          machine: "machine-local",
          healthy: false,
          rows: [
            row({ pid: 8123 }),
            row({ pid: 9001, machine_id: "machine-other" }),
          ],
        },
      ],
    });
    const view = render(
      <CleanupCentre bridge={{ load: async () => mixed }} killBridge={killBridge} />,
    );
    await waitFor(() => expect(view.getByTestId("cleanup-row-9001")).toBeTruthy());
    expect(view.getByTestId("cleanup-kill-8123")).toBeTruthy();
    expect(view.queryByTestId("cleanup-kill-9001")).toBeNull();
    view.unmount();
  });

  it("the machine filter is a labelled combobox, defaults to This machine, and widens the load", async () => {
    const calls: Array<[string, "this" | "all"]> = [];
    const view = render(
      <CleanupCentre
        bridge={{
          load: async (filter, machineScope) => {
            calls.push([filter, machineScope]);
            return data();
          },
        }}
      />,
    );
    await waitFor(() => expect(view.getByTestId("cleanup-group-sess-orphan")).toBeTruthy());
    expect(calls[0][1]).toBe("this");
    const select = view.getByRole("combobox", { name: "Machine filter" });
    expect((select as HTMLSelectElement).value).toBe("this");
    fireEvent.change(select, { target: { value: "all" } });
    await waitFor(() => expect(calls.at(-1)).toEqual(["needs-attention", "all"]));
    view.unmount();
  });
});
