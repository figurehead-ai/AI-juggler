# Evidence manifest — AJ-E1006

Task: AJ-E1006 (pair review and model arbitrage, `[inferred — falsify
first]`). Status: **REJECTED (D-26, 2026-10-20)** — both workflows
fail a desk falsification ([`desk-falsification.md`](desk-falsification.md)):
the step each one says is missing is already served, upstream by the
tools and in the app. Nothing is built. The user round in
[`protocol.md`](protocol.md) didn't run (the AJ-E809 cohort has no
participants yet); if it runs and shows the habit plus friction, a
workflow comes back as a new task. 2026-10-18, updated 2026-10-20,
reviewer claude-opus.

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| A falsification round (≥ 5 dogfood users): diary and interview evidence for or against each workflow, with persona and verbatim notes | Req.1 | `protocol.md`: participants, a two-week diary, an interview script, coding, and the packet layout | INSTRUMENT READY (round awaits the cohort) | 2026-10-18 | claude-opus |
| Only a workflow with ≥ 3 independent users graduates to a build task | Req.2 | The decision rule is written into the protocol before any data, so the bar cannot move afterwards | PENDING EVIDENCE | 2026-10-18 | claude-opus |
| Negative evidence closes the task as REJECTED; the roadmap does not carry the feature | Req.3 | Same rule. No UI, code or roadmap entry exists for either workflow | PENDING EVIDENCE | 2026-10-18 | claude-opus |
| A graduated design respects D-03/D-05 (review is a switch or fork with lineage) | Req.4 | Stated as a condition of graduation | PENDING EVIDENCE | 2026-10-18 | claude-opus |
| Desk falsification: is the step each workflow needs already served? | added 2026-10-20 | W6: `opusplan` and `/model` upstream; across accounts, the app's Switch account… with fidelity badges and the carry list. W5: `/review` in Claude Code, `codex review`, OpenAI's Claude Code plugin `/codex:review`; in the app, New session on another account in the same project (4 clicks). Both paths walked by hand. Sources and quotes in `desk-falsification.md` | FALSIFIED (both) | 2026-10-20 | claude-opus |
| The outcome is a new decision-log entry | evidence | D-26 in `docs/review/decision-log.md` | PASS | 2026-10-20 | claude-opus |
