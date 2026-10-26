/**
 * AJ-E2108 gallery fixture: a session's transcript end as the backend
 * returns it (`conversations_last_messages`), long enough to look like
 * real work: 30-odd turns with commands, edits, markdown, code blocks,
 * a key taken out, and a question at the end. Invented text, no real
 * transcript.
 */

type Block = { kind: "text"; text: string; steps: string[] } | { kind: "steps"; text: string; steps: string[] };
export type FixtureTurn = { speaker: "you" | "assistant"; blocks: Block[]; at_ms: number | null; model: string | null };

const text = (value: string): Block => ({ kind: "text", text: value, steps: [] });
const steps = (...lines: string[]): Block => ({ kind: "steps", text: "", steps: lines });

/** [speaker, blocks] in order; times are spread back from `t0`. */
const SCRIPT: ["you" | "assistant", Block[]][] = [
  // Before the page's first 30: "Show earlier" brings them.
  ["you", [text("Morning. Can you pull main and check the build is green before we start?")]],
  ["assistant", [steps('ran "git pull"', 'ran "pnpm build"'), text("Pulled 6 commits; the build is green in 41 s.")]],
  ["you", [text("The checkout test fails about one run in five on CI. Find out why and fix it, please.")]],
  [
    "assistant",
    [
      text("I'll start by running it in a loop to see how often it fails and what the failures have in common."),
      steps('ran "pnpm vitest run checkout --repeat 40"', "Read checkout.test.ts", "Read payment-mock.ts"),
      text(
        "Ran it **40 times: 8 failures**. Every failure has the same shape:\n\n- the payment mock answers *after* the 2 s timeout\n- the order is left in `pending`\n- the assertion on `status === \"paid\"` then fails\n\nSo it's a race between the mock's delay and the timeout, not a bug in checkout itself.",
      ),
    ],
  ],
  ["you", [text("Why would the mock be slow? It's all in memory.")]],
  [
    "assistant",
    [
      steps('searched for "setTimeout"', "Read test/setup.ts"),
      text(
        "It isn't slow on purpose. `payment-mock.ts` waits a random jitter to look like a real gateway:\n\n```ts\nconst jitter = Math.random() * 2500; // up to 2.5 s\nawait sleep(jitter);\nreturn { status: \"captured\" };\n```\n\nWith a 2 s timeout, anything over 2000 ms loses: about 20 % of runs, which matches the 8 in 40.",
      ),
    ],
  ],
  ["you", [text("Makes sense. What are the options?")]],
  [
    "assistant",
    [
      text(
        "### Three ways out\n\n1. **Make the mock deterministic** in tests: no jitter unless a test asks for it.\n2. **Raise the timeout** to 5 s. Simple, but slows every failing run and hides real slowness.\n3. **Use fake timers** so the test controls time, and assert the timeout path separately.\n\nI'd go with 1 and 3 together: the happy path stays fast and exact, and the timeout gets its own test.",
      ),
    ],
  ],
  ["you", [text("Go with 1 and 3. Keep the jitter available for the soak tests.")]],
  [
    "assistant",
    [
      steps("Edit payment-mock.ts", "Edit checkout.test.ts", "Write checkout-timeout.test.ts"),
      text(
        "Done. The mock now takes an option:\n\n```ts\nexport function paymentMock({ jitterMs = 0 } = {}) {\n  return async () => {\n    if (jitterMs > 0) await sleep(Math.random() * jitterMs);\n    return { status: \"captured\" } as const;\n  };\n}\n```\n\nThe soak suite passes `jitterMs: 2500`, everything else gets none.",
      ),
    ],
  ],
  ["you", [text("Run the whole suite.")]],
  [
    "assistant",
    [
      steps('ran "pnpm vitest run"'),
      text("**412 passed**, 0 failed, in 38 s. The checkout file alone went from 6.1 s to 0.9 s now that it doesn't wait on jitter."),
    ],
  ],
  ["you", [text("And the loop? 40 runs again.")]],
  [
    "assistant",
    [
      steps('ran "pnpm vitest run checkout --repeat 40"'),
      text("40 of 40 passed. The timeout test passes on its own too, and it fails as it should if I break the timeout handler."),
    ],
  ],
  ["you", [text("Nice. While you're there, the soak config still points at the old staging key, it's •••, swap it for the env var.")]],
  [
    "assistant",
    [
      steps("Read soak.config.ts", "Edit soak.config.ts"),
      text("Swapped it for `process.env.PAYMENT_SANDBOX_KEY`, with a clear error if it's missing:\n\n```ts\nconst key = process.env.PAYMENT_SANDBOX_KEY;\nif (!key) throw new Error(\"Set PAYMENT_SANDBOX_KEY to run the soak tests\");\n```"),
    ],
  ],
  ["you", [text("Add that to the README's testing section.")]],
  [
    "assistant",
    [
      steps("Read README.md", "Edit README.md"),
      text("Added a short **Soak tests** paragraph under *Testing*: what they're for, the env var, and that they're not part of `pnpm test`."),
    ],
  ],
  ["you", [text("Does CI run the soak suite anywhere?")]],
  [
    "assistant",
    [
      steps('searched for "soak"', "Read .github/workflows/nightly.yml"),
      text("Only in `nightly.yml`, once a night, against the sandbox. It reads the key from the repo's secrets as `PAYMENT_SANDBOX_KEY`, so the rename lines up and nothing else needs to change."),
    ],
  ],
  ["you", [text("Good. Commit this as one change.")]],
  [
    "assistant",
    [
      steps('ran "git add -p"', 'ran "git commit"'),
      text("Committed on `fix/flaky-checkout`:\n\n> test(checkout): a deterministic payment mock; the timeout gets its own test\n\nFour files, +61 −18."),
    ],
  ],
  ["you", [text("Can you also look at why the orders page test is slow? It takes 12 s.")]],
  [
    "assistant",
    [
      steps('ran "pnpm vitest run orders --reporter verbose"', "Read orders.test.tsx", "Read test/server.ts"),
      text(
        "Most of the 12 s is one thing: each of its 14 tests starts a **fresh mock server** and seeds 2,000 orders.\n\n| Step | Time |\n|---|---|\n| server start | 0.3 s × 14 |\n| seeding | 0.5 s × 14 |\n| the tests | 1.1 s |\n\nSeeding once per file and resetting between tests would bring it to about 2 s.",
      ),
    ],
  ],
  ["you", [text("Do it.")]],
  [
    "assistant",
    [
      steps("Edit test/server.ts", "Edit orders.test.tsx", 'ran "pnpm vitest run orders"'),
      text("Seeded once in `beforeAll`, with a cheap `reset()` in `beforeEach` that only truncates what the tests write. **12.4 s → 1.8 s**, all 14 still pass."),
    ],
  ],
  ["you", [text("Run everything once more and then we're done with tests.")]],
  [
    "assistant",
    [
      steps('ran "pnpm vitest run"', 'ran "pnpm typecheck"', 'ran "pnpm lint"'),
      text("All green: 413 tests in 27 s (was 38 s), typecheck clean, lint clean."),
    ],
  ],
  ["you", [text("Great. Now the PR description, short.")]],
  [
    "assistant",
    [
      text(
        "Here's a draft:\n\n**Fix the flaky checkout test, speed up the orders tests**\n\n- The payment mock no longer adds random delay unless a test asks for it; the timeout path has its own test.\n- The orders tests seed once per file: 12 s → 2 s.\n- The soak tests read the sandbox key from `PAYMENT_SANDBOX_KEY`.",
      ),
    ],
  ],
  ["you", [text("Open the PR against main.")]],
  [
    "assistant",
    [
      steps('ran "git push -u origin fix/flaky-checkout"'),
      text(
        "Pushed. Before I open it: the branch is 3 commits behind `main`, and one of them touches `payment-mock.ts` too.\n\nShould I **rebase onto main first**, or open the PR as it is and let the merge sort it out?",
      ),
    ],
  ],
];

/** The asking session's turns, the newest last. */
export function askingTurns(t0: number): FixtureTurn[] {
  const step = 75_000;
  return SCRIPT.map(([speaker, blocks], index) => ({
    speaker,
    blocks,
    at_ms: t0 - (SCRIPT.length - index) * step + 15_000,
    model: speaker === "assistant" ? "claude-opus-4-5" : null,
  }));
}

/** A Codex session still working: its last turn is the user's ask, then
 * the tool calls it is making. */
export function workingTurns(t0: number): FixtureTurn[] {
  const at = (minutes: number) => t0 - minutes * 60_000;
  return [
    { speaker: "you", blocks: [text("Add pagination to the orders endpoint: page and limit, default limit 50, max 200.")], at_ms: at(14), model: null },
    {
      speaker: "assistant",
      blocks: [
        steps("Read src/routes/orders.ts", "Read src/db/orders.ts", 'searched for "LIMIT"'),
        text("The endpoint returns every order in one go. I'll add `page` and `limit` query parameters, validate them, and push the limit into the SQL instead of slicing in memory."),
        steps("edited src/routes/orders.ts, src/db/orders.ts"),
        text("```ts\nconst limit = Math.min(Number(query.limit ?? 50), 200);\nconst page = Math.max(Number(query.page ?? 1), 1);\nconst rows = await db.orders.list({ limit, offset: (page - 1) * limit });\n```"),
      ],
      at_ms: at(11),
      model: "gpt-5-codex",
    },
    { speaker: "you", blocks: [text("Return the total too, so the UI can show page counts.")], at_ms: at(6), model: null },
    {
      speaker: "assistant",
      blocks: [
        text("Adding a `total` from a `COUNT(*)` over the same filter, in one round trip with a window function so it stays one query."),
        steps("edited src/db/orders.ts", 'ran "pnpm test orders"'),
      ],
      at_ms: at(1),
      model: "gpt-5-codex",
    },
  ];
}

/** The old shape (speaker and text only), for the "before" captures. */
export function asTextOnly(turns: FixtureTurn[]): { speaker: string; text: string }[] {
  return turns.map((turn) => ({ speaker: turn.speaker, text: turn.blocks.filter((block) => block.kind === "text").map((block) => block.text).join("\n\n") }));
}
