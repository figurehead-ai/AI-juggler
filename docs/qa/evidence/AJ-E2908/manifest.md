# Evidence manifest — AJ-E2908

Task: `docs/tasks/E29-trust-and-the-finished-basics.md` AJ-E2908, an ended
session's page: Resume, Archive, Replay and Copy ID. Built in 0.4. Checked
requirement by requirement on 2026-11-06 (release/0.4.1). Status: **DONE-EVIDENCED** (agent self-check, 2026-11-06; R1, R3–R6 closed the same day;
R2, the tool spec's resume, is left). Most are missing assertions
on code that exists. Still open from the task's own evidence list: the
live Resume on the real setup in AI Juggler Dev.

| Req | Result | Evidence |
|---|---|---|
| R1 | PASS | `ui/src/components/inspector-instance.test.tsx` "an ended session's header: Relaunch, then Resume last…" shows both on an ended session, Resume after Relaunch, with the lines "Fresh conversation" and "Same conversation"; `ui/src/pages/session/resume.test.tsx` "resumes the same conversation…" asserts Resume's tooltip "Resume — same session, same account.". **Closed (2026-11-06):** `ui/src/components/inspector-instance.test.tsx` "AJ-E2908 R1: Resume is the primary, Relaunch the secondary, each saying what it starts": Relaunch's tooltip "Relaunch — fresh session, same launch settings and folder.", Resume's "Resume — same session, same account.", Resume filled (the page's one primary), Relaunch secondary. |
| R2 | PASS | `crates/juggler-app/tests/session_resume.rs` `an_ended_session_resumes_its_own_conversation_on_its_own_account`: refused while running and without a conversation; then a new session with the same conversation id, launch settings and folder, through the same `resume_conversation` the Switch Graph's epoch resume uses (run in the release preflight). Whether a tool can resume is its adapter's (2026-11-06): the tool catalog's `supports.resume` is true when its adapter gives resume arguments (`features::tools::spec_info`), and `ResumeButton` reads it from the tool's spec, with no per-tool branch; `pages/session/resume.test.tsx` "R2: whether its tool can resume is the tool's own spec" shows a tool that can't greyed with why. |
| R3 | PASS | `resume.test.tsx` "says why Resume can't run" (no conversation, "Sign in to Work first", "Work is out of room until 17:00", open in another app) and "is off, with its reason, without a conversation" (disabled, reason in `header-resume-why`). **Closed (2026-11-06):** `ui/src/pages/session/resume.test.tsx` "says the account is out of room on Resume, and offers Continue on another account…" (out of room: Resume off with the reason, `header-resume-continue` beside it calls `onContinue`) and "with room, no Continue beside it". |
| R4 | PASS | `resume.test.tsx` "adds Replay, Archive and Copy the conversation's id to an ended session's menu only": `resolveActions` gives `replay`, `archive`, `copy-id` on an ended session, none on a live one, and "Unarchive" on an archived one; Replay and Archive are History's specs (`action-specs.ts`). **Closed (2026-11-06):** `resume.test.tsx` "R4: an ended session's ⋯ has Replay, Archive and Copy the conversation's id; Archive's toast offers Undo": the session page's real ⋯ (`InspectorHeaderMenu`) lists the three; Archive archives (`archive:s-9`) and its toast's Undo unarchives. |
| R5 | PASS |  **Closed (2026-11-06):** `resume.test.tsx` "R5: Copy the conversation's id writes the tool's own id and says “Copied.”": the clipboard gets `conv-tool-42` and the toast says "Copied.". |
| R6 | PASS | `resume.test.tsx` shows the ⋯ adds nothing on a live session; `inspector-instance.test.tsx` "a running session's header offers Continue in another app…; an ended one's, Relaunch" shows no Relaunch on a running one. **Closed (2026-11-06):** `inspector-instance.test.tsx` "AJ-E2908 R6: a running session's header has no Resume and no Relaunch" (running, starting and waiting). |

Run 2026-11-06: `pnpm exec vitest run src/pages/session/resume.test.tsx src/components/inspector-instance.test.tsx` — pass.
Gallery: `ui/tests/visual/release-04-screens-gallery.spec.ts` "Resume" (`99-resume`: an ended session's page from Home; both themes, both sizes).

Gaps closed on 2026-11-06: the whole vitest suite passed (`pnpm exec vitest run --maxWorkers=4`: 281 files, 2135 tests) with the tests named above.
