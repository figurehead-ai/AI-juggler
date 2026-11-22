/**
 * D-126: each setup step's value under its name, one function for New
 * pool's steps and the canvas's checklist (and New pool's preview rows),
 * read from one PoolShape; and the Messages ways as talk grids.
 */
import { describe, expect, it } from "vitest";

import { draftOf } from "./new-pool";
import type { PoolShape } from "./pool-sentences";
import { talkSentence } from "./pool-sentences";
import { templateById } from "./pool-templates";
import { leadValue, reviewLines, rolesLine, rolesValue, shortNames, stepValues, talkValue, whoLine, whoValue } from "./pool-values";
import { OWN_MIX, presetOf, TALK_PRESETS } from "./pool-words";

const shape = (over: Partial<PoolShape> = {}): PoolShape => ({
  members: [
    { ref: "app:claude", label: "Claude · Work", role: "member", level: null, later: true },
    { ref: "app:codex", label: "Codex · Personal", role: "reviewer", level: null, later: true },
    { ref: "all", label: "Everyone in web", role: "member", level: null, later: false },
  ],
  leadNext: "app:claude",
  talk: templateById("blank").talk,
  level: "work_together",
  sealed: false,
  ...over,
});
const apps = (labels: string[]) => labels.map((label, at) => ({ ref: `app:${at}`, label, role: "member" as const, level: null, later: true }));

describe("a step's value (D-126: the steps and the checklist say it once)", () => {
  it("reads as the mock-ups': where, who's in, who leads, roles, messages, on their own", () => {
    expect(stepValues(shape(), "web")).toEqual({
      where: "web",
      who: "Claude, Codex, everyone in web",
      lead: "Claude's next session",
      everyone: "Codex checks the work",
      talk: "Everyone, freely",
      own: "Work together",
    });
  });

  it("a member is short where nothing else shares its tool's name, whole where something does", () => {
    // D-126: two Apps sharing an app word go by their profiles; two
    // sessions sharing a name stay whole.
    expect(shortNames(["Codex · Personal", "Claude app · Work", "Claude app · Side"])).toEqual(["Codex", "Work", "Side"]);
    expect(shortNames(["web · Lab", "web · Personal"], ["session:a", "session:b"])).toEqual(["web · Lab", "web · Personal"]);
  });

  it("who's in: no one, only everyone, and past three names the rest counted", () => {
    expect(whoValue(shape({ members: [] }), "web")).toBe("No one yet");
    // Was "everyone in web" in New pool's steps (a list's lower case on its own).
    expect(whoValue(shape({ members: shape().members.slice(2) }), "web")).toBe("Everyone in web");
    // Was "A, B, C, +2" in New pool's steps, "A, B, C and 2 more" on the checklist.
    expect(whoValue(shape({ members: apps(["A · 1", "B · 2", "C · 3", "D · 4", "E · 5"]) }), "web")).toBe("A, B, C, 2 more");
  });

  it("who leads: the session, the App's next session (with where on the preview), or none yet", () => {
    expect(leadValue(shape())).toBe("Claude's next session");
    expect(leadValue(shape(), "web")).toBe("Claude's next session in web");
    expect(leadValue(shape({ members: [...shape().members, { ref: "session:s1", label: "Draft the page", role: "lead", level: null, later: false }] }))).toBe("Draft the page");
    // Was "No lead for now" on the checklist.
    expect(leadValue(shape({ leadNext: null }))).toBe("No lead yet");
  });

  it("roles: who checks, everyone in the projects too when it joins as a Reviewer", () => {
    expect(rolesValue(shape({ members: apps(["A", "B"]) }))).toBe("Everyone builds");
    // Was "Everyone builds" in New pool's steps: everyone in web reviews.
    expect(rolesValue(shape({ members: [{ ref: "all", label: "Everyone in web", role: "reviewer", level: null, later: false }] }))).toBe("Everyone in web checks the work");
    expect(rolesValue(shape({ members: [...shape().members.slice(0, 2), { ref: "all", label: "Everyone in web", role: "reviewer", level: null, later: false }] }))).toBe("Codex and everyone in web check the work");
    expect(rolesValue(shape({ members: apps(["A", "B", "C", "D"]).map((one) => ({ ...one, role: "reviewer" as const })) }))).toBe("A, B, C and 1 more check the work");
  });

  it("messages: the way's own words; an Observer's row is no exception; a cell changed by hand is its own mix", () => {
    // Was "Some ask you first" on the checklist.
    expect(talkValue(templateById("feature-team").talk)).toBe("Members ask you first");
    // A made pool with no talk rows at all: everyone, freely (was "Your own mix" in the steps).
    expect(talkValue({})).toBe("Everyone, freely");
    expect(talkValue({ ...templateById("blank").talk, "observer>lead": "ask" })).toBe("Everyone, freely");
    expect(talkValue({ ...templateById("blank").talk, "lead>member": "blocked" })).toBe(OWN_MIX);
    expect(talkValue(TALK_PRESETS.find((one) => one.id === "lead")!.grid)).toBe("Through the lead");
  });

  it("on their own: the level's name, Custom too (as the header's picker says it)", () => {
    expect(stepValues(shape({ level: "custom", picked: 3 }), "web").own).toBe("Custom");
    expect(stepValues(shape({ level: "full" }), "web").own).toBe("Work together and wake each other");
  });
});

describe("New pool's preview rows", () => {
  it("say who's in and the roles as a line", () => {
    expect(whoLine(shape(), "web")).toBe("Claude and Codex, and every new session in web");
    expect(whoLine(shape({ members: [] }), "web")).toBe("No one yet");
    expect(rolesLine(shape())).toBe("Codex checks the work; the rest build");
    expect(rolesLine(shape({ members: [] }))).toBe("Everyone builds (Member)");
  });
});

describe("the Messages ways (D-126: talk grids there are, no new backend)", () => {
  it("Feature team's is Members ask you first; Through the lead in a sentence", () => {
    expect(presetOf(templateById("blank").talk)).toBe("free");
    expect(presetOf(templateById("feature-team").talk)).toBe("ask");
    const lead = TALK_PRESETS.find((one) => one.id === "lead")!;
    expect(talkSentence({ talk: lead.grid })).toBe("Member to Member: blocked; Member to Observer: blocked. The rest is allowed; an Observer only reads.");
    expect(presetOf({ ...templateById("blank").talk, "lead>member": "ask" })).toBeNull();
    expect(presetOf(draftOf(templateById("reviewer-for-everything"), null).talk)).toBeNull();
  });

  it("an Observer's cell changed by hand keeps the way: it only reads either way (was its own mix)", () => {
    expect(presetOf({ ...templateById("feature-team").talk, "observer>member": "ask" })).toBe("ask");
  });
});

// The 0.4.6 rc3 critique (98-pool-setup at 1280x800): two CLI Apps' full
// names ran Who's in over four lines and cut the preview's cards.
describe("one short identity for an App (D-126 critique)", () => {
  const two = shape({
    members: [
      { ref: "app:cli:work", label: "Claude Code CLI · Work Max", role: "member", level: null, later: true },
      { ref: "app:cli:personal", label: "Claude Code CLI · Personal", role: "reviewer", level: null, later: true },
      { ref: "all", label: "Everyone in storefront", role: "member", level: null, later: false },
    ],
    leadNext: "app:cli:work",
  });

  it("two Apps that share an app word go by their profiles, in the steps and the lead", () => {
    expect(stepValues(two, "storefront").who).toBe("Work Max, Personal, everyone in storefront");
    expect(leadValue(two)).toBe("Work Max's next session");
    expect(rolesValue(two)).toBe("Personal checks the work");
  });

  it("Review says each row in one short sentence", () => {
    expect(reviewLines(shape(), "web", "web team")).toEqual({
      where: "“web team” works in web.",
      who: "Claude and Codex, and every new session in web.",
      lead: "Claude's next session in web leads.",
      everyone: "Codex checks the work. Everyone else builds.",
      talk: "Everyone may message everyone.",
      own: "They work together. Anything more asks you.",
      sealed: "No. It can link to your other pools.",
    });
    const lines = reviewLines(shape({ leadNext: null, members: apps(["A"]), talk: TALK_PRESETS[1]!.grid, level: "read", sealed: true }), "web", "x");
    expect([lines.lead, lines.everyone, lines.talk, lines.own, lines.sealed]).toEqual([
      "No lead yet: each session works on what you give it.",
      "Everyone builds.",
      "Between members, each message waits for your OK.",
      "They look only. Anything more asks you.",
      "Yes. Nothing goes in or out, and its sessions are in no other pool.",
    ]);
  });
});
