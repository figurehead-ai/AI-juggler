/**
 * 0.3.5 gallery fixture: a Claude Code session's conversation as the feed
 * sends it (ADR-0024): stretches of text with each run of work as one
 * titled line (the titles as the Claude adapter writes them), narration,
 * a message the person sent mid-turn, and a turn still working. Invented
 * text, no real transcript.
 */

type Block = { kind: string; text?: string; steps?: string[]; title?: string; title_running?: string; calls?: { tool: string; line: string; failed: boolean; done: boolean }[] };
export type FeedFixtureTurn = { speaker: "you" | "assistant"; blocks: Block[]; at_ms: number | null; model: string | null };

const text = (value: string): Block => ({ kind: "text", text: value });
const narration = (value: string): Block => ({ kind: "narration", text: value });
const run = (title: string, running: string, ...lines: string[]): Block => ({
  kind: "steps",
  steps: lines,
  title,
  title_running: running,
  calls: lines.map((line) => ({ tool: "Bash", line, failed: false, done: true })),
});

const SCRIPT: ["you" | "assistant", Block[]][] = [
  ["you", [text("The checkout test fails about one run in five on CI. Find out why and fix it, please.")]],
  [
    "assistant",
    [
      narration("Running it in a loop first, to see what the failures have in common."),
      run("Ran the checkout test 40 times", "Running the checkout test 40 times", 'ran "pnpm vitest run checkout --repeat 40"'),
      text("Ran it **40 times: 8 failures**. Every failure has the same shape: the payment mock answers *after* the 2 s timeout, and the order is left in `pending`."),
      run("Read 2 files, searched code", "Reading payment-mock.ts", "Read checkout.test.ts", "Read payment-mock.ts", 'searched for "setTimeout"'),
      text("It's the mock's jitter:\n\n```ts\nconst jitter = Math.random() * 2500; // up to 2.5 s\nawait sleep(jitter);\n```\n\nWith a 2 s timeout, about 20 % of runs lose the race. I'll make the jitter an option that is off by default."),
    ],
  ],
  ["you", [text("Keep the jitter for the soak suite, though.")]],
  [
    "assistant",
    [
      text("Keeping it there: the soak suite passes `jitterMs: 2500`, everything else gets none."),
      run("Updated payment-mock.ts, ran a command", "Running the unit tests", "Edit payment-mock.ts", 'ran "pnpm vitest run"'),
    ],
  ],
];

/** The session's turns, the newest last (`t0` is now). */
export function feedTurns(t0: number): FeedFixtureTurn[] {
  const step = 90_000;
  return SCRIPT.map(([speaker, blocks], index) => ({
    speaker,
    blocks,
    at_ms: t0 - (SCRIPT.length - index) * step + 30_000,
    model: speaker === "assistant" ? "claude-opus-4-5" : null,
  }));
}

/** What arrives while the reader looks further up: the run finishes and
 * the reply comes. */
export const ARRIVING: Block[] = [text("All green: 214 passed. The checkout test passed 200 runs in a row with the jitter off.")];
