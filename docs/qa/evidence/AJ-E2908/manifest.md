# Evidence manifest — AJ-E2908

Task: `docs/tasks/E29-trust-and-the-finished-basics.md` AJ-E2908, an ended
session's page: Resume, Archive, Replay and Copy ID. Built in 0.4. Checked
requirement by requirement on 2026-11-06 (release/0.4.1). Status: **DONE,
6 gaps open** (agent self-check, 2026-11-06). Most are missing assertions
on code that exists. Still open from the task's own evidence list: the
live Resume on the real setup in AI Juggler Dev.

| Req | Result | Evidence |
|---|---|---|
| R1 | GAP | `ui/src/components/inspector-instance.test.tsx` "an ended session's header: Relaunch, then Resume last…" shows both on an ended session, Resume after Relaunch, with the lines "Fresh conversation" and "Same conversation"; `ui/src/pages/session/resume.test.tsx` "resumes the same conversation…" asserts Resume's tooltip "Resume — same session, same account.". Missing: no test asserts Relaunch's tooltip ("Relaunch — fresh session, same launch settings and folder.", `ItemHeader.tsx`) or that Resume is the primary button and Relaunch the secondary. |
| R2 | GAP | `crates/juggler-app/tests/session_resume.rs` `an_ended_session_resumes_its_own_conversation_on_its_own_account`: refused while running and without a conversation; then a new session with the same conversation id, the same launch settings and the same folder (Rust: run in the release preflight). `session_resume_impl` and `epoch_resume_impl` both call `resume_conversation` in `commands.rs`. Missing: whether a tool can resume isn't read from its spec; the UI offers Resume on every ended session, and a tool that can't resume fails only when the adapter's `launch_spec` refuses. No test covers such a tool. |
| R3 | GAP | `resume.test.tsx` "says why Resume can't run" (no conversation, "Sign in to Work first", "Work is out of room until 17:00", open in another app) and "is off, with its reason, without a conversation" (disabled, reason in `header-resume-why`). Missing: no test shows "Continue on another account…" beside Resume when the account is out of room (`header-resume-continue` in `ResumeButton.tsx`). |
| R4 | GAP | `resume.test.tsx` "adds Replay, Archive and Copy the conversation's id to an ended session's menu only": `resolveActions` gives `replay`, `archive`, `copy-id` on an ended session, none on a live one, and "Unarchive" on an archived one; Replay and Archive are History's specs (`action-specs.ts`). Missing: no test opens the session page's ⋯ and finds them, or checks that Archive's toast offers Undo (`ManageWork.tsx` `archiving`). |
| R5 | GAP | Missing: no test proves Copy the conversation's id writes the tool's own id to the clipboard and says "Copied." (`ManageWork.tsx` `copyConversationId`); the menu test uses a stub. |
| R6 | GAP | `resume.test.tsx` shows the ⋯ adds nothing on a live session; `inspector-instance.test.tsx` "a running session's header offers Continue in another app…; an ended one's, Relaunch" shows no Relaunch on a running one. Missing: no test asserts a running session's header has no Resume. |

Run 2026-11-06: `pnpm exec vitest run src/pages/session/resume.test.tsx src/components/inspector-instance.test.tsx` — pass.
Gallery: `ui/tests/visual/release-04-screens-gallery.spec.ts` "Resume" (`99-resume`: an ended session's page from Home; both themes, both sizes).
