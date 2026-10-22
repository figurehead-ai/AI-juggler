# AJ-E1006 — desk falsification (2026-10-20)

The task asks whether two guessed workflows need to be **built** in AI
Juggler. `protocol.md` set out a user round (≥ 5 dogfood participants)
to decide. That round has not run: the AJ-E809 cohort has no
participants yet (`../AJ-E809/report.md`: 0 runs, 0 participants), and
an agent can't recruit or stand in for users. This file records what can
be tested without users. Each hypothesis predicts that something is
missing today, so we look for it in the tools and in the app.

## The question each workflow has to survive

> Is there a step the user needs that neither the tools (Claude Code,
> Codex) nor the AI Juggler of today already gives them?

If the step already exists, a built workflow would duplicate it. If it's
missing, the user round still decides whether anyone wants it.

## W6 Model arbitrage: a cheap model drafts, a strong one finishes

**Prediction:** the user can't move a task from a cheap model to a strong
one without AI Juggler building it.

**Same account, two models.** Claude Code does this itself:

- Claude Code's model docs ([model-config](https://code.claude.com/docs/en/model-config)):
  `opusplan` is a "Special mode that uses `opus` during plan mode, then
  switches to `sonnet` for execution", and "use `/model <alias|name>` to
  switch immediately". That's the draft/finish split inside one session,
  with its context kept.
- Codex has its own model picker in the session.

**Cheap account to strong account** (W6's case: GLM Lite drafts, Claude
Max finishes). AI Juggler already does this. On a running session,
"Switch account…" lists every destination ranked by headroom. Each one
carries its fidelity badge ("F1 · Beta"), what doesn't carry over ("todo
list, subagent state") and its runway. Picking one hands the
conversation over. That is W6 steps 3–4 as written in
`docs/review/pass2/07-derived-workflows-and-scenarios.md` §W6. Walked by
hand on the current build, 2026-10-20: session → Switch account… →
destination, with the ranked list and badges shown before anything moves.

**Not served:** a per-leg cost split in Usage ("draft 82 % on Lite"), a
"Draft ready" row and a long-draft warning. Nothing anywhere asks for
these. The research marks them `[inferred]`, and the cost split depends
on the cost ledger reading per-leg burn.

**Result: falsified.** The workflow's core step exists twice over:
upstream for one account, and in the app's switch across accounts.

## W5 Pair review: one account writes, another reviews

**Prediction:** to have a second account, ideally another provider,
review the work, the user has to copy the diff into a second terminal by
hand (pass2/07 §W5: "15–30 min versus manual copy-the-diff into a second
terminal").

**The tools review on their own:**

- Codex ([developer commands](https://learn.chatgpt.com/docs/developer-commands?surface=cli)):
  `codex review` runs "a non-interactive review of uncommitted changes, a
  base branch diff, a commit, or custom review instructions". `/review` is
  in its slash commands.
- Claude Code bundles `/review` (an alias of `/code-review`,
  [slash commands](https://code.claude.com/docs/en/slash-commands)).
- OpenAI publishes a Claude Code plugin,
  [`openai/codex-plugin-cc`](https://github.com/openai/codex-plugin-cc)
  ("Use Codex from Claude Code to review code or delegate tasks"), whose
  `/codex:review` "Runs a normal Codex review on your current work". That
  is cross-provider pair review, from inside the author's own session.

**Reaching another account in AI Juggler.** New session → the project →
Customise → the other account's Launch: 4 clicks, walked by hand
2026-10-20. That session runs in the same project and reads the diff from
git, so nothing is copied. The author's session is untouched, which is
what W5 asks for (a copy, never a move; D-03/D-05 are kept because nothing
is transplanted).

**Not served:** "Send comments to author session" and a lineage pair
"authored by A, reviewed by B". Both are `[inferred]`, and nobody has
reported missing them.

**Result: falsified.** The step the prediction says is missing (getting
the diff to a second account or provider for review) needs no copying
today, in the app or upstream.

## What this does not settle

- Whether people do either workflow, and whether the residual steps
  would help. Only the user round in `protocol.md` can show that. If it
  runs and 3 or more participants show either habit **and** friction with
  the paths above, the workflow comes back as a new task (D-26).
- The fidelity of a cross-provider switch (Claude Code → Codex) is beta
  (AJ-E1304; F-badges shown per pair), so W6's cross-provider leg is only
  as good as that switch.

## Sources

- [Claude Code — model configuration](https://code.claude.com/docs/en/model-config) (`opusplan`, `/model`)
- [Claude Code — slash commands](https://code.claude.com/docs/en/slash-commands) (`/review`)
- [Codex — developer commands](https://learn.chatgpt.com/docs/developer-commands?surface=cli) (`codex review`)
- [openai/codex-plugin-cc](https://github.com/openai/codex-plugin-cc) (`/codex:review` from Claude Code)
- The app, walked by hand on the build of 2026-10-20 (mock bridge): New session → Customise; Switch account…
