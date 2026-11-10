/**
 * D-96 B (AJ-E3307 M2) — the heading's "more things" by name: every part
 * the heading counts as "more", in its order, the rest one "N more".
 */
import { describe, expect, it } from "vitest";
import { headlineExtras, needsHeading, type HomeNeed } from "./home-needs";

const need = (key: string, kind: HomeNeed["kind"], title: string): HomeNeed => ({ key, kind, title, detail: "", severe: false });

describe("D-96 B — the heading's extras", () => {
  it("names each thing the heading counts as more, sessions left out", () => {
    const needs = [need("s:1", "session", "api"), need("quota:work", "quota", "Work is almost out of room"), need("signin:codex", "signin", "ChatGPT needs its sign-in")];
    const extras = headlineExtras(needs, { caps: [], messages: 0, tasks: 0 });
    expect(needsHeading({ sessions: 1, others: 2 }, { runOut: [], running: true, anything: true })).toBe("1 session and 2 more things need you");
    expect(extras.map((extra) => extra.words)).toEqual(["Work is almost out of room", "ChatGPT needs its sign-in"]);
    expect(extras[1]!.target).toBe('[data-need-key="signin\\:codex"]');
  });

  it("counts caps, agents' messages and tasks, and folds a long list into \"N more\"", () => {
    const needs = [need("mac", "mac", "This Mac is working hard"), need("leftovers", "leftovers", "2 processes left running")];
    const extras = headlineExtras(needs, { caps: ["Work's monthly cap is reached"], messages: 2, tasks: 1 });
    expect(extras.map((extra) => extra.words)).toEqual(["This Mac is working hard", "2 processes left running", "Work's monthly cap is reached", "2 more"]);
    expect(headlineExtras([], { caps: [], messages: 1, tasks: 1 }).map((extra) => extra.words)).toEqual(["1 message from an agent", "1 task waits on you"]);
  });
});
