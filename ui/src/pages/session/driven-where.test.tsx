/**
 * The 0.4.6 rc3g walk (p4-web1): a session AI Juggler drove, without a
 * terminal, before its first turn's conversation was read, said "Runs in:
 * AI Juggler's terminal" and "Model: not reported by Claude Code", while
 * its own card read "Idle · claude-opus-5-5". It runs in AI Juggler (its
 * process is AI Juggler's child), and its model is the one its tool
 * reports to AI Juggler until its transcript says which runs.
 */
import { cleanup, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { RightInspector } from "../../components/RightInspector";
import { instance } from "../../test/fixtures";
import type { RuntimeInfo, RuntimeViewInfo } from "../../lib/ipc/runtime";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";
import { fromInstance } from "./model";
import { WhereItRuns } from "./WhereItRuns";

const REPORTED = "claude-opus-5-5";

const runtime: RuntimeInfo = {
  kind: "driven",
  label: "Claude Code without a terminal",
  beta: true,
  state: "idle",
  asks: [],
  can_steer: false,
  fell_back: null,
  error: null,
  limits: [],
  cut_off: null,
};

/** p4-web1 before its conversation was read: running, driven, no model
 * observed, none launched with. */
const driven = (over: Partial<InstanceInfo> = {}): InstanceInfo =>
  instance({ id: "sess-p4", status: "running", model: null, launch_model: null, running_model: null, model_source: "unknown", conversation_id: null, runtime, ...over });

const view: RuntimeViewInfo = { session_id: "sess-p4", runtime, items: [], streaming: null, model: REPORTED };

/** The preview's runtime views (`runtimeGet` reads them with no backend). */
const views = (all: Record<string, RuntimeViewInfo>) => Reflect.set(window, "__AIJ_RUNTIME__", all);

beforeEach(() => {
  views({});
});
afterEach(cleanup);

/** Where it runs' rows, label → value. */
function rows(card: HTMLElement): Record<string, string> {
  const labels = [...card.querySelectorAll("dt")].map((dt) => dt.textContent ?? "");
  const values = [...card.querySelectorAll("dd")].map((dd) => dd.textContent ?? "");
  return Object.fromEntries(labels.map((label, at) => [label, values[at] ?? ""]));
}

describe("a driven session before its conversation is read", () => {
  it("runs in AI Juggler, never in a terminal it hasn't got", () => {
    const where = render(<WhereItRuns model={fromInstance(driven())} accountWords={null} outside={false} />);
    expect(rows(where.getByTestId("where-it-runs"))["Runs in"]).toBe("AI Juggler");
    cleanup();
    // One in AI Juggler's terminal still says so.
    const terminal = driven({ runtime: { ...runtime, kind: "terminal", label: null } });
    const there = render(<WhereItRuns model={fromInstance(terminal)} accountWords={null} outside={false} />);
    expect(rows(there.getByTestId("where-it-runs"))["Runs in"]).toBe("AI Juggler's terminal");
  });

  it("names the model its tool reports until its transcript says which runs", () => {
    const where = render(<WhereItRuns model={fromInstance(driven(), undefined, REPORTED)} accountWords={null} outside={false} />);
    expect(rows(where.getByTestId("where-it-runs")).Model).toBe("Opus 5.5");
    // Its transcript's, once read.
    expect(fromInstance(driven({ running_model: "claude-sonnet-5" }), undefined, REPORTED).model).toBe("claude-sonnet-5");
  });

  it("its page says the model its card says, never 'not reported'", async () => {
    views({ "sess-p4": view });
    const session = driven();
    const page = render(
      <RightInspector
        selection={{ kind: "instance", id: session.id }}
        data={{ instance: session, project: null, account: null, provider: null, machineId: "this-mac" }}
        sessions={[session]}
        projects={[]}
        accounts={[]}
        profiles={[]}
        terminalTitle={null}
        freshness={{}}
        now={Date.now()}
        onOpenProject={() => {}}
        onOpenSession={() => {}}
        onLaunch={() => {}}
        onStopped={() => {}}
        onHandover={() => {}}
        onNew={() => {}}
      />,
    );
    // Its card, from the same read.
    await waitFor(() => expect(page.getByTestId("driven-panel").textContent).toContain(`Idle · ${REPORTED}`));
    expect(page.container.textContent).not.toContain("not reported");
    await waitFor(() => expect(rows(page.getByTestId("where-it-runs")).Model).toBe("Opus 5.5"));
    expect(rows(page.getByTestId("where-it-runs"))["Runs in"]).toBe("AI Juggler");
  });
});
