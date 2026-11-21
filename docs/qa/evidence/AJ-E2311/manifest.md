# Evidence manifest — AJ-E2311

Task: `docs/tasks/E23-session-runtime.md` AJ-E2311, 0.4.6's deferred session findings, built in 0.4.7's batch 1 on release/0.4.7 (D-138). Source list: `docs/research/0.4.6-deferred-to-0.5.md`.
Status: each fixed item has a test that fails before its fix; the screens are walked on 0.4.7's nightly.

| Req | Result | Evidence | Walked |
|---|---|---|---|
| R1 | PASS (test) | 24d5375e3: Open in terminal is one row (`overnight_summary::a_session_handed_over_to_its_terminal_is_one_row`, `rail-model.test.ts`) | V2 |
| R2 | PASS (test) | ad02ebc83: New session opens in the page's project, else where you last started one, on the App used there; Recent holds its order; no shift (`new-session-fits-here.test`) | V2 |
| R3 | PASS (test) | 828b832fb: "Stopping p1-writer." then where it runs (`stop-names-session.test`) | V2 |
| R4 | PASS (test) | 1bbbc8f47: the limit dialog fits down to 1024×680 with its title and Not now in sight; the other-apps list has Cancel (`switch-dialogs-fit.test`) | V2 |
| R5 | PASS (0.4.8) | f1c0cb6a8 (one press, the step's reason) and 0.4.8's daa91bd43/08acacf40: the full-window setup renders in a portal under the body, so its frames are where it's drawn; anything that hit-tests by frame (Voice Control's "click", Switch Control's point scanning, our driver) missed the rail and Cancel before. A system-wide AX hit-test at the rail's "Where" returns `AXButton «2 Where» AXPress`, at Cancel `AXButton «Cancel» AXPress` (0.4.8 walk W2) | W2 |
| R6 | PARTIAL | Done: a deleted conversation file starts again after a restart, saying why (db3e12e33, `a_driven_session_whose_conversation_file_is_gone_starts_again_after_a_restart`); a switch back from the other Mac keeps the name given there (b2e11a329, `work_renamed_on_the_other_mac_comes_back_under_that_name`); a look running when a process ends never writes it back (141c9d17c). Open: a no-turn session quit under a build before d4c840a46 (only a direct upgrade from 0.4.5 or older, once) needs an adapter question ("is this conversation written?"): batch 2 | V2 |
| R7 | OPEN | Needs a live Codex turn in a lab profile, which waits on the maintainer's lab Codex sign-in | — |
| R8 | PASS (test) | 421afb7e6: Claude Code's `utilization` is a fraction of its window (the installed CLI's schema: values above 1 when usage runs past a cap), read ×100 and capped at 100; a week on one model reads "Weekly (Opus)" (`codec::limits::a_use_is_always_a_fraction_of_its_window`) | V2 |
