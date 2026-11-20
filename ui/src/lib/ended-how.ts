/**
 * D-96 C (AJ-E3307 D2) — how an ended run ended, in one vocabulary: the
 * session's `ending` (the backend's, from its process's exit and the
 * user's Stop: `features/sessions/read_model.rs`, `with_endings`), never
 * a guess. Nothing said: only "Ended".
 *
 * Per adapter: every session AI Juggler runs (Claude Code, Codex and any
 * tool in its terminal, driven or not) has its process's exit and the
 * user's Stop; a session another app runs (the Claude app, Codex's app, a
 * CLI outside AI Juggler) has neither: its reader sees it close, never
 * how, so it reads "Ended".
 */
const WORDS: Record<string, string> = {
  finished: "Finished",
  error: "Error",
  stopped: "Stopped by you",
  crashed: "Crashed",
  // The rc27 P1 walk: a switch stopped it, not the user.
  switched: "Stopped by the switch",
  // The rc29 walk: it ended with AI Juggler (a quit, a reopen that
  // couldn't bring it back); it read a bare "Ended".
  with_app: "Ended with AI Juggler",
};

/** "Finished", "Error", "Stopped by you", "Crashed", "Stopped by the
 * switch", "Ended with AI Juggler"; else "Ended". */
export function endedWord(how: string | null | undefined): string {
  return (how && WORDS[how]) || "Ended";
}

/** With its detail when its signal had one: "Error · exit code 1". */
export function endedWords(how: string | null | undefined, detail: string | null | undefined): string {
  return detail ? `${endedWord(how)} · ${detail}` : endedWord(how);
}

/** SK-5: what a few exit codes mean, as shells use them. */
const EXIT_MEANING: Record<number, string> = {
  126: "its command couldn't run",
  127: "its command wasn't found",
  130: "it was interrupted",
};

/** SK-5: what a few signals did, in plain words. */
const SIGNAL_DID: Record<number, string> = {
  1: "hung up (its terminal closed)",
  2: "interrupted",
  6: "aborted",
  9: "killed",
  11: "crashed (a memory fault)",
  15: "asked to quit",
};

/** SK-5 (the 0.4.6 soak, K3): why an ended run stopped, in plain words,
 * from its ending's detail (`exit code 1`, `signal 9`): "Claude Code
 * exited with code 1", "Claude Code was killed (signal 9)"; `null` when
 * nothing said why. */
export function endedCause(detail: string | null | undefined, tool: string): string | null {
  if (!detail) return null;
  const exit = /^exit code (-?\d+)$/.exec(detail);
  if (exit) {
    const meaning = EXIT_MEANING[Number(exit[1])];
    return `${tool} exited with code ${exit[1]}${meaning ? ` (${meaning})` : ""}`;
  }
  const signal = /^signal (\d+)$/.exec(detail);
  if (signal) return `${tool} was ${SIGNAL_DID[Number(signal[1])] ?? "stopped"} (signal ${signal[1]})`;
  return `${tool} ended: ${detail}`;
}

/** D-96 C: the groups an ended run lands in, as the headline counts them
 * ("60 ended: 57 finished, 2 errors, 1 crashed"). */
const ENDED_PARTS: Array<{ id: string; one: string; many: string }> = [
  { id: "finished", one: "finished", many: "finished" },
  { id: "error", one: "error", many: "errors" },
  { id: "crashed", one: "crashed", many: "crashed" },
  { id: "stopped", one: "stopped by you", many: "stopped by you" },
  { id: "switched", one: "stopped by a switch", many: "stopped by switches" },
  { id: "ended", one: "not known how", many: "not known how" },
];

/** The headline: what needs you, what still runs, then what ended and how. */
export function summaryHeadline(counts: Record<string, number | undefined>): string {
  const parts: string[] = [];
  if (counts.needs_you) parts.push(`${counts.needs_you} needs you`);
  if (counts.running) parts.push(`${counts.running} still running`);
  const ended = ENDED_PARTS.map((part) => ({ part, n: counts[part.id] ?? 0 })).filter(({ n }) => n > 0);
  const total = ended.reduce((sum, { n }) => sum + n, 0);
  if (total > 0) {
    const how = ended.map(({ part, n }) => `${n} ${n === 1 ? part.one : part.many}`);
    // Nothing said for any of them: only "ended".
    parts.push(ended.length === 1 && ended[0]!.part.id === "ended" ? `${total} ended` : `${total} ended: ${how.join(", ")}`);
  }
  return parts.join(" · ");
}
