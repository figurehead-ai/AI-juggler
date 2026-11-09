# Evidence manifest — AJ-E2307

Task: `docs/tasks/E23-session-runtime.md` AJ-E2307, Answers from Needs you and the session page. Branch `e23-runtime`. Status: **DONE on fakes**: the ask card, the batch view in Needs you, keys 1–9 and Enter, the reply box, audited answers, and the guardrail.

## What exists now

`features/runtime/commands.rs` (`runtime_asks`, `runtime_answer`); `ui/src/components/AskCard.tsx`, `HomeNeeds.tsx`, `pages/session/DrivenPanel.tsx`, `lib/ipc/runtime.ts`.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 the tool's own options; a reply box with its choices | `cd ui && npx vitest run src/components/ask-card.test.tsx` ("offers the tool's own options and carries the one clicked", "takes the tool's choice or a reply, once every part has one") | words as the tool sends them | pass | PASS | e4b7e9af | 2026-10-25 | agent |
| R2 always-allow is the tool's own rule | `cargo test -p juggler-adapters --test adapter_conformance` (`keeps_the_tools_own_option_rules`) | rule carried, never made up | pass | PASS | e4b7e9af | 2026-10-25 | agent |
| R3 no path from a timer, rule, agent or default | `cargo test -p juggler-app --test runtime_guardrails` (`only_the_users_click_reaches_an_answer`, `agents_and_pools_never_reach_the_runtime`); `cd ui && npx vitest run src/components/ask-card.test.tsx` ("sends nothing without a click") | none | pass | PASS | e4b7e9af | 2026-10-25 | agent |
| R4 audited: session, kind, option, when (never a reply's text) | `cargo test -p juggler-app --test runtime` (`a_driven_session_is_exact_and_answered_only_by_the_user`) | one `answered` row | pass | PASS | e4b7e9af | 2026-10-25 | agent |
| Batch view and keyboard (lab: approvals are frequent) | `cd ui && npx vitest run src/components/ask-card.test.tsx` ("lists every open ask of the driven sessions together", "answers from the keyboard") | all asks; 2 picks the second | pass | PASS | e4b7e9af | 2026-10-25 | agent |
| Visual: approval, question, free reply; both themes | the gallery run | looked at | not run (no capture this branch) | UNPROVEN | — | 2026-10-25 | agent |
| Live: one approval and one question answered from AI Juggler | docs/qa/runtime-lab.md steps 1 and 4 in the app | answered by a click | BLOCKED: AI Juggler Dev not built (disk < 25 GB), no desktop control. The answer shapes themselves worked live ([lab 2](docs/qa/evidence/AJ-E2301/lab-2-2026-10-25.md)) | UNPROVEN | — | 2026-10-25 | agent |
| Live (lab 3): approvals answered from AI Juggler | [lab 3](docs/qa/evidence/AJ-E2301/lab-3-2026-11-07.md) step 1 | answered by a click and by a key | Allow once clicked on Home; the next from the keyboard (focus the ask, `1`); a question with options didn't reach AI Juggler on Codex 0.163.0 (plain text, recorded); Claude's question is step 4 | PASS (approvals) | 3d97d92cb | 2026-11-07 | agent |
| R5 the UI reuses the session page's components | review: one `ui/src/components/AskCard.tsx` serves the session page (`pages/session/DrivenPanel.tsx`), Needs you (`MessageNeeds.tsx`, `TaskNeeds.tsx`) and the pool's inspector (`pages/pool/InspectNode.tsx`), on the readability scale (`text-body`, `text-meta`) and the shared `primitives` Button; `cd ui && npx vitest run src/components/ask-card.test.tsx` | one component, the shared scale | pass | PASS | 3eaa7b923 | 2026-11-09 | Opus (verified) |

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here (secret scan ran)
- [x] Paths hashed or generalised where they reveal private layout
- [x] Transcript excerpts scrubbed of personal content (none: fixtures only)
