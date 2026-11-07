# Evidence manifest — AJ-E1305 Live validation on real accounts and the docs

Task: `docs/tasks/E13-codex-adapter.md` AJ-E1305. Written 2026-11-07 on
`release/0.4.1` from the live labs already on file; no new live run (agents
never sign in; the labs ran on the maintainer's lab profiles and the Mac's
Codex home, with the maintainer's permission, in throwaway folders).

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 launch and resume, live | AJ-E1539 labs 11, 14, 19 and 20 (`docs/qa/evidence/AJ-E1539/manifest.md`, `lab-2026-10-28.md`): each switch launches the target tool by AI Juggler's launch line (`codex resume <id>` / `claude --resume`), read and recorded; runtime lab 2 (`docs/qa/evidence/AJ-E2301/lab-2-2026-10-25.md`): Codex through its app-server, launched by AI Juggler | the real binaries start and resume the conversation | Codex resumed a rollout by id in another home (lab 11: "The third codename is PLOVER; power is the only open plan item"); every Claude ↔ Codex hop of labs 14, 19, 20 resumed and answered its probes; the app-server read limits before a turn and carried a real approval | PASS | see the labs | 2026-10-23 – 2026-10-28 | Claude (self) |
| R1 two accounts signed in in parallel | labs 1–11: Claude Code accounts A and B (two subscriptions) side by side; lab 11: the Mac's Codex home and a second Codex home signed in at once | two at once | Claude: two subscriptions at once, every lab. Codex: two homes at once in lab 11; a second Codex **account** has never been available | PASS (Claude); Codex: DEFERRED (D-91) | — | 2026-10-22 – 2026-10-23 | Claude (self) |
| R1 Codex → Codex switch | lab 11 step 8 (a task begun in the Mac's Codex home, switched to the second home) | every register area | transcript-resume and todos PASS (0.163.0); rules-carry, subagents, file-history, mcp-references, cwd-worktree-edges and switch-back UNVALIDATED: the second home was signed out and removed, and a new one needs a sign-in and a second Codex account | DEFERRED (D-91) | b5fc27ada | 2026-11-07 | Claude (self) |
| R1 Claude → Codex and Codex → Claude replays, live, register filled | lab 14 (J1, J2, J3; Claude Code 2.1.310, codex-cli 0.163.0, 2026-10-26), again labs 19 and 20 (2026-10-28, clean environment) | each area decided with its versions | Claude → Codex: transcript-resume, rules-carry, todos, mcp-references, cwd-worktree-edges, switch-back PASS; subagents and file-history FAIL with drop notes. Codex → Claude: transcript-resume, rules-carry, todos, mcp-references, cwd-worktree-edges, switch-back PASS; subagents, file-history UNVALIDATED. Each cell carries `run: live`, both versions, date and evidence in `crates/juggler-handover/data/fidelity-register.json` (`docs/qa/evidence/AJ-E808/register.md`) | PASS | see the labs | 2026-10-26 – 2026-10-28 | Claude (self) |
| R2 SPEC §4.2 (and §4.1) say what is verified live versus on fixtures | `docs/SPEC.md` §4.1 and §4.2, "Verified live, and what is on fixtures only (AJ-E1305, 2026-11-07)" | each pair and runtime claim placed as live or fixtures, with versions and labs | as listed above; the app-server's interrupt, steer and compact are the fake Codex's; Codex → Codex beyond transcript and plan deferred | PASS | 1f2ce795d | 2026-11-07 | Claude (self) |
| R2 the release notes say it | `docs/releases/0.4.1.md`, "What's checked on the real tools" | the same split, in users' words | Claude ↔ Claude, Claude ↔ Codex, Codex ↔ Codex once, Codex sessions; "treated as unchecked again" after an update | PASS | 1f2ce795d | 2026-11-07 | Claude (self) |
| R2 the beta-exit register says it | `docs/qa/beta-exit.md`, "Gate 1 for Claude Code and Codex: live versus fixtures" | a row per pair | five rows, live and fixtures columns | PASS | 1f2ce795d | 2026-11-07 | Claude (self) |

D-91 (2026-11-07): the six Codex→Codex cells stay UNVALIDATED in the
register, each marked `deferred: D-91` (b5fc27ada; `register.rs`
`a_deferred_cell_stays_unvalidated_and_names_its_decision`), so the switch
keeps saying "not proven yet" for the pair; rendered in
`docs/qa/evidence/AJ-E808/register.md` under "Deferred cells".

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here
- [x] Paths generalised (`<claude-a>`, `<codex-home>` in the labs)
- [x] No conversation content beyond the labs' own probe words
