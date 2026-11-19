import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { AiJobInfo } from "./ipc/ai";

/** What the backend is asked, and the change listeners it was given. */
const calls: Array<{ name: string; args: unknown }> = [];
const heard = new Map<string, Array<(message: { payload: null }) => void>>();

const job = (over: Partial<AiJobInfo> & Pick<AiJobInfo, "job" | "state">): AiJobInfo => ({
  beta: false,
  beta_on: false,
  model_id: null,
  model_name: null,
  why: null,
  ...over,
});

let answer: AiJobInfo[] = [];

vi.mock("./ipc/transport", async (importOriginal) => ({
  ...(await importOriginal<typeof import("./ipc/transport")>()),
  liveBackend: true,
  call: async (name: string, args: unknown) => {
    calls.push({ name, args });
    if (name === "ai_jobs") return answer.map((info) => ({ ...info }));
    if (name === "ai_beta_set") {
      const which = Reflect.get(Object(args), "job");
      const on = Reflect.get(Object(args), "on") === true;
      answer = answer.map((info) => (info.job === which ? { ...info, beta_on: on, state: on ? "ready" : "beta_off" } : info));
      return answer;
    }
    throw new Error(`unexpected ${name}`);
  },
  listen: async (event: string, handler: (message: { payload: null }) => void) => {
    heard.set(event, [...(heard.get(event) ?? []), handler]);
    return () => heard.set(event, (heard.get(event) ?? []).filter((h) => h !== handler));
  },
}));
vi.mock("./bridge", async (importOriginal) => ({ ...(await importOriginal<typeof import("./bridge")>()), hasLiveBackend: () => true }));

beforeEach(() => {
  calls.length = 0;
  heard.clear();
  answer = [
    job({ job: "feed", state: "off" }),
    job({ job: "chat", state: "ready", beta: true, beta_on: true, model_id: "built-in", model_name: "Built-in model", why: "recommended" }),
    job({ job: "review", state: "beta_off" }),
    job({ job: "digest", state: "no_engine", why: "turned_off" }),
  ];
  vi.useRealTimers();
});

describe("useAiJob(job) (ADR-0034 §3.4, AJ-E3802)", () => {
  it("asks nothing until a view wants it", async () => {
    vi.resetModules();
    await import("./ai-availability");
    expect(calls).toEqual([]);
  });

  it("shows a job only when it runs or waits: one answer shared by every view", async () => {
    vi.resetModules();
    const { useAiJob, isShown } = await import("./ai-availability");
    const chat = renderHook(() => useAiJob("chat"));
    const review = renderHook(() => useAiJob("review"));
    expect(chat.result.current).toBeNull();
    await waitFor(() => expect(chat.result.current?.state).toBe("ready"));
    await waitFor(() => expect(review.result.current?.state).toBe("beta_off"));
    expect(calls).toEqual([{ name: "ai_jobs", args: undefined }]);
    expect(isShown(chat.result.current)).toBe(true);
    expect(isShown(review.result.current)).toBe(false);
    expect(isShown(job({ job: "digest", state: "no_engine" }))).toBe(false);
    expect(isShown(job({ job: "digest", state: "off" }))).toBe(false);
    expect(isShown(job({ job: "chat", state: "waiting" }))).toBe(true);
    expect(isShown(null)).toBe(false);
  });

  it("is asked again when the backend says the answer changed, never on a timer", async () => {
    vi.resetModules();
    const { useAiJob } = await import("./ai-availability");
    const chat = renderHook(() => useAiJob("chat"));
    await waitFor(() => expect(chat.result.current?.state).toBe("ready"));
    await waitFor(() => expect(heard.get("ai://jobs-changed")).toHaveLength(1));
    vi.useFakeTimers();
    await act(async () => void vi.advanceTimersByTime(24 * 60 * 60 * 1000));
    vi.useRealTimers();
    expect(calls).toHaveLength(1);
    // Apple Intelligence turned off: the backend says so, the view hides.
    answer = answer.map((info) => (info.job === "chat" ? { ...info, state: "no_engine", why: "turned_off", model_id: null } : info));
    act(() => heard.get("ai://jobs-changed")?.forEach((on) => on({ payload: null })));
    await waitFor(() => expect(chat.result.current?.state).toBe("no_engine"));
    expect(chat.result.current?.why).toBe("turned_off");
    expect(calls).toHaveLength(2);
  });

  it("turns a job's Beta on through the backend", async () => {
    vi.resetModules();
    const { aiBetaSet } = await import("./ipc/ai");
    const jobs = await aiBetaSet("review", true);
    expect(calls).toEqual([{ name: "ai_beta_set", args: { job: "review", on: true } }]);
    expect(jobs.find((info) => info.job === "review")?.state).toBe("ready");
  });
});
