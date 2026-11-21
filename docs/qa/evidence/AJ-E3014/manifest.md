# Evidence manifest — AJ-E3014

Task: `docs/tasks/E30-pools-redesign.md` AJ-E3014, 0.4.6's deferred pool and message findings, built in 0.4.7's batch 1 on release/0.4.7 (D-138). Source list: `docs/research/0.4.6-deferred-to-0.5.md`; the soak's list: `docs/qa/0.4-walks/soak-0.4.6/findings.md`.
Status: DONE. Each item has a test that fails before its fix; walked on 0.4.7's nightly code (docs/qa/0.4-walks/nightly-0.4.7/V1.md, 2026-11-20), with 0.4.8's follow-ups (the header's ⋯ at the row's end, the grid after a delete) walked on 0.4.8.

| Req | Result | Evidence | Walked |
|---|---|---|---|
| R1 | PASS (test) | 0387809d9: `pools-screen.test` "a pool made since the last visit…" (the grid holds still; a new pool goes last) | PASS (V1) |
| R2 | PASS (test) | a3f118569: a session moved out of its project's pool stays out (the pools model), and the refusal names the pool that holds it and how to reach the other: `a_session_moved_out_of_its_projects_pool_is_told_where_it_is` | PASS (V1) |
| R3 | PASS (test) | 9442f801b: every surface reads "2 of 2 agent messages in a row without you; one more waits for you", and the limit holds for a desktop member and a terminal AI Juggler can't type into: `the_hop_limit_holds_for_a_recipient_ai_juggler_cant_wake` | PASS (V1) |
| R4 | PASS (test) | 7f39c1132: who joins now and later beside the form; "it joins" only when a new session would; the connector paragraph once (`pool-setup-round4`, `pool-open-now`, `new-pool-who`, `new-pool` tests) | PASS (V1) |
| R5 | PASS (test) | 05d44b1d6, 0dc62a453: "web (in lab)" in Settings › Agent access (`pools-scope.test`) | PASS (V1) |
| R6 | PASS (test, walked: the settings panel scrolls by wheel to Delete pool…) | 1994400b2: the pool header's ⋯ (Rename, Delete), the card's ⋯ always in sight (`canvas.test` "SK-1…", `pools-screen.test`); the side panel and List view fill the page so each can scroll by wheel: the wheel itself is walked live | PASS (V1) |
| R7 | PASS (test), ruled | D-139 (the maintainer: "One press"): 3cf020efb, e7cb04043, "Allow and deliver" / "Allow, deliver later" / Deny on Needs you, the thread and the diagram: `allow_and_deliver_puts_a_held_message_into_its_session_in_one_press`, `messages.test`, `canvas-answers.test` | PASS (V1) |
| R8 | PASS (test) | 071aa2bdf: Pause or Stop takes the hop-limit ask off Home, and no Deliver reopens the thread: `pause_or_stop_at_the_hop_limit_takes_the_ask_off_home_and_deliver_never_reopens` | PASS (V1) |
| R9 | PASS (test) | Re-checked on the beta build in a P7 lab (2026-11-20): b GONE; a, c, d PRESENT, fixed in f56f4d4e3 and d0a254ddd (a held session counted and named in the header, Who's in and the canvas, no lone "No lead" card, no held session drawn on a Pools card): `held-members.test` | PASS (V1) |

## Also in this batch, from the soak (same files)
- SK-1 (R6), SK-3 bc2c5529d, SK-7 b7e32cf5e, SK-8 99f9f5724/ab19ba31a, SK-9 e2fe33335, SK-12 51bc815ac, SK-13 c2b783525/0e123875f, SK-14 6cf919f92, SK-17 b85d6a327, SK-18 609e295d7, D-140 61be31566 (`a_woken_agent_writing_in_a_new_thread_counts_on_from_its_chain`, checked failing with the chain count disabled), the refusal's rule f671ceaa1.
