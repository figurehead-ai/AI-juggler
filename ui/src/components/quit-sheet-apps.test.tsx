/**
 * rc30: the quit sheet said "2 sessions are running. They end when AI
 * Juggler quits." and the driven Codex session came back after the
 * reopen: each app says what happens to it (quit.rs `QuitSessionInfo`).
 */
import { describe, expect, it } from "vitest";
import { quitEffectWords } from "./QuitSheet";
import type { QuitPlanInfo } from "../types/generated/QuitPlanInfo";
import type { QuitSessionInfo } from "../types/generated/QuitSessionInfo";

const at = (provider: string, account_label: string, fate: string, kind = "terminal"): QuitSessionInfo => ({ kind, provider, account_label, fate });
const plan = (sessions: QuitSessionInfo[]): QuitPlanInfo => ({
  live: sessions.length,
  end_on_quit: sessions.filter((one) => one.fate === "ends").length,
  keep_running: sessions.filter((one) => one.fate === "keeps_running").length,
  come_back: sessions.filter((one) => one.fate === "comes_back").length,
  sessions,
});

describe("rc30 the quit sheet, per app", () => {
  it("says what comes back and what ends, by its app", () => {
    expect(quitEffectWords(plan([at("codex", "Lab", "comes_back"), at("claude-code", "Lab", "ends")]))).toBe(
      "Codex · Lab comes back when you reopen AI Juggler. Claude Code · Lab ends when AI Juggler quits.",
    );
    expect(quitEffectWords(plan([at("claude-code", "Lab", "ends"), at("claude-code", "Lab", "ends")]))).toBe("2 in Claude Code · Lab end when AI Juggler quits.");
    expect(quitEffectWords(plan([at("claude-code", "Work", "keeps_running", "desktop")]))).toMatch(/ keeps running after AI Juggler quits\.$/);
  });

  it("without names, the counts say it as before", () => {
    expect(quitEffectWords({ live: 2, end_on_quit: 2, keep_running: 0, come_back: 0, sessions: [] })).toBe("They end when AI Juggler quits.");
  });
});
