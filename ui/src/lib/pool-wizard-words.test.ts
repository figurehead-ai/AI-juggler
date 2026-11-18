/**
 * D-126: New pool's own words — the steps' questions and groups, each
 * step's value under its name, and the Messages presets as talk grids.
 */
import { describe, expect, it } from "vitest";

import { draftOf } from "./new-pool";
import { templateById } from "./pool-templates";
import { talkSentence } from "./pool-sentences";
import { eyebrowOf, leadShort, OWN_MIX, presetOf, rolesLine, rolesShort, shortNames, stepValue, TALK_PRESETS, talkShort, whoLine, whoShort, type DraftView } from "./pool-wizard-words";

const view = (over: Partial<DraftView> = {}): DraftView => ({
  named: [
    { ref: "app:claude", name: "Claude · Work", role: "member" },
    { ref: "app:codex", name: "Codex · Personal", role: "reviewer" },
  ],
  everyone: true,
  where: "web",
  lead: { name: "Claude · Work", next: true },
  level: "work_together",
  talk: templateById("blank").talk,
  ...over,
});

describe("the wizard's words (D-126)", () => {
  it("each step says where it is: its number of all and its group", () => {
    const steps = ["template", "where", "who", "lead", "everyone", "talk", "own", "tools", "review"] as const;
    expect(eyebrowOf(steps, "who")).toBe("Step 3 of 9 · The basics");
    expect(eyebrowOf(steps, "talk")).toBe("Step 6 of 9 · How they work");
    expect(eyebrowOf(steps, "review")).toBe("Step 9 of 9 · Finish");
  });

  it("a member is short where nothing else shares its tool's name, whole where something does", () => {
    expect(shortNames(["Codex · Personal", "Claude app · Work", "Claude app · Side"])).toEqual(["Codex", "Claude app · Work", "Claude app · Side"]);
  });

  it("each step's value, as the steps and the preview say it", () => {
    expect(whoShort(view())).toBe("Claude, Codex, everyone in web");
    expect(whoLine(view())).toBe("Claude and Codex, and every new session in web");
    expect(whoLine(view({ named: [] }))).toBe("Every new session in web");
    expect(whoShort(view({ named: [], everyone: false }))).toBe("No one yet");
    expect(leadShort(view())).toBe("Claude's next session");
    expect(leadShort(view({ lead: null }))).toBe("No lead yet");
    expect(rolesShort(view())).toBe("Codex checks the work");
    expect(rolesLine(view())).toBe("Codex checks the work; the rest build");
    expect(rolesLine(view({ named: [] }))).toBe("Everyone builds (Member)");
    expect(stepValue("own", { ...view(), template: "From scratch", tools: null })).toBe("Work together");
    expect(stepValue("review", { ...view(), template: null, tools: null })).toBeNull();
  });

  it("past three names, the rest are counted", () => {
    const many = ["A · 1", "B · 2", "C · 3", "D · 4", "E · 5"].map((name, at) => ({ ref: `app:${at}`, name, role: "member" as const }));
    expect(whoShort(view({ named: many, everyone: false }))).toBe("A, B, C, +2");
  });

  it("the Messages presets are talk grids there are: Feature team's is Members ask you first", () => {
    expect(presetOf(templateById("blank").talk)).toBe("free");
    expect(presetOf(templateById("feature-team").talk)).toBe("ask");
    const lead = TALK_PRESETS.find((one) => one.id === "lead")!;
    expect(talkSentence({ talk: lead.grid })).toBe("Member to Member: blocked; Member to Observer: blocked. The rest is allowed; an Observer only reads.");
    // A cell changed by hand: no preset, its own mix.
    expect(presetOf({ ...templateById("blank").talk, "lead>member": "ask" })).toBeNull();
    expect(talkShort({ ...templateById("blank").talk, "lead>member": "ask" })).toBe(OWN_MIX);
    expect(presetOf(draftOf(templateById("reviewer-for-everything"), null).talk)).toBeNull();
  });
});
