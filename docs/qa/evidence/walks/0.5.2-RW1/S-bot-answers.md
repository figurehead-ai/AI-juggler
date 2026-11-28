# 0.5.2 RW1 Part S: Bot's answers, word for word

Dev 28204 (build 4622c72c9), lab /tmp/aij-lab-rw1. The text is the panel's accessibility text, copied unedited. Each model line is the panel's own.

## S2 (06:38, new topic): "what needs me"
- Answer: “Rewrite the date parser to accept two-digit years” failed in “web”.
- Chips: record 1 (Rewrite the date parser… · Session · Claude Code CLI · Work · work · web · Stopped with an error · 6:37 AM)
- "Checked against the record below"; model line "Apple's on-device model · ≈ 345 est. in · ≈ 31 est. out · 1.3 s · 1 record"; status "Bot: Here’s what I found: 1 session stopped with an error."

## S3, first ask (06:39, new topic): "what are my sessions"
- Answer: “Update the summary cell in the report notebook” is idle in “docs” and “Rewrite the date parser to accept two-digit years” failed in “web”.
- Chips: record 1, record 2; "Checked against the 4 records below": 1 Update the summary cell… (Work · docs, Idle), 2 Rewrite the date parser… (Work · web, Stopped with an error), 3 Change the date format in the API responses to ISO 8601 (Personal · api, Finished), 4 Add a retry helper to the API client (Codex CLI · ChatGPT · default · web, Ended)
- Model line: "Apple's on-device model · ≈ 547 est. in · ≈ 52 est. out · 1.3 s · 4 records"; status "Bot: Here’s what I found: 1 session stopped with an error, 1 is idle, 1 finished, and 1 ended."

## S3, second ask (06:39, another new topic): "what are my sessions"
- Answer (two lines, one chip each): “Update the summary cell in the report notebook” is idle in “docs”, in “Work · work”. / “Rewrite the date parser to accept two-digit years” failed in “web”, in “Work · work”.
- "Checked against the 4 records below" (same four records).

## S3, third and fourth ask (06:40, new topics): "what are my sessions"
- Answer, both times, word for word (two lines, one record chip each): “Update the summary cell in the report notebook” is idle in “docs”, in “Work · work”. / “Rewrite the date parser to accept two-digit years” failed in “web”, in “Work · work”.
- "Checked against the 4 records below"; model line "Apple's on-device model · ≈ 642 est. in · ≈ 68 est. out · 4.4 s (first), 2.0 s, 1.7 s · 4 records"; "Bot: Here’s what I found: 1 session stopped with an error, 1 is idle, 1 finished, and 1 ended."
- X1 (stray ", records." and the App joined to the folder by "and") did not show in four asks.

## S4 (06:40, new topic each): "what is web doing?"
- Answer (two lines): “Rewrite the date parser to accept two-digit years” failed in “web”, in “Work · work”. / “Add a retry helper to the API client” ended in “web”, in “ChatGPT · default”.
- "Checked against the 3 records below" (the third record, the finished "Split the cart total…", is not in a sentence); model line "… 1.5 s · 3 records"; "Bot: Here’s what I found: 1 session stopped with an error, 1 finished, and 1 ended."

## S4, "what is api doing?"
- Answer: “Change the date format in the API responses to ISO 8601” finished in “api”.
- "Checked against the 3 records below"; model line "… 1.0 s · 3 records"; "Bot: Here’s what I found: 1 session finished and 2 ended."

## S5, first try (06:41): Ask Bot on the revert post, then "did it finish cleanly?"
- Scope under the box: "About: Change the date format in the API responses to ISO 8601"; the user's line reads "About this post: “Claude Code CLI finished Change the date format in the API responses to ISO 8601 in api in under a minute.” did it finish cleanly?"
- Answer, word for word: "Bot’s answer didn’t check out against your records, so it isn’t shown. Try asking another way." Status: "Bot: I don’t know." Model line "Apple's on-device model · ≈ 479 est. in · ≈ 38 est. out · 1.5 s"; "Looked at · Change the date format in the API responses to ISO 8601 only · just now"; "memory 1". No record chips.

## S5, second try (06:41, New topic, then Ask Bot on the revert post, "did it finish cleanly?")
- Same result as the first try, word for word: "Bot’s answer didn’t check out against your records, so it isn’t shown. Try asking another way." Status "Bot: I don’t know." Model line "Apple's on-device model · ≈ 433 est. in · ≈ 38 est. out · 1.6 s"; "Looked at · Change the date format in the API responses to ISO 8601 only · just now". No record chip. (Expected: “Change the date format…” finished in “api”. with that one record.)

## S6 (06:42, same topic): "what is 3+5"
- Answer: "Bot’s own answer · not from your records" / "3+5 equals 8." / "Bot is here for your AI apps and sessions: ask what needs you, what finished, or how a session is going." Model line "… ≈ 228 est. in · ≈ 14 est. out · 0.6 s"; status "Bot answered." The answer carries no "About this post" scope (the composer fell back to "Ask about your sessions, tasks, pools or usage…").

## S7 (06:42, one new topic)
1. "call api the payments agent" → "I’ll remember: you call “api” the payments agent." / "Stays on this Mac." with What Bot remembers, Edit, Undo (no separate Remember press was offered; it remembered at once). Status "Bot answered."
2. "what is the payments agent doing?" → “Change the date format in the API responses to ISO 8601” finished in “api”. (record 1; "Checked against the 3 records below"; "Looked at · every session, task and pool, and 1 memory"; model line "… 1.1 s · 3 records"; "Bot: Here’s what I found: 1 session finished and 2 ended.")
3. "what did I ask you to remember about api?" → "You call “api” “the payments agent” (you said, just now)." (record 1, the memory chip "1 the payments agent What Bot remembers · your name for api") then, as a second line with record 2: "The session “Change the date format in the API responses to ISO 8601” is finished in project “api”." "Checked against the 4 records below"; model line "… 2.0 s · 4 records". Not refused.

## S8 (06:43): the reminder on the Codex post
- Toast: "Reminder set for 9:43 AM. About Add a retry helper to the API client." Reminders list (Watches page): "Look at “Add a retry helper to the API client”" / "About Add a retry helper to the API client" / 9:43 AM / In 3 h / Once, then done. The Notifications priming sheet ("Tell you when a session needs you", Not now → "Nothing was asked… Done") came once, on this first Remind me.

## S10 (06:46): "tell me when web finishes", Watches Beta off (ai_beta_watches=0)
- Bot, word for word: "More than one matches that name, so the Watches form asks which one." [Open the form]; "Bot answered."
- Form: "Filled in by Bot · More than one matches that name: pick which one."; Watch what "Bot couldn't tell which one" with the field "Add more, or search…" (the picker NOT open on arrival; pressing the field opens it with "YOUR WORDS MATCH": web · started 6:45 AM (open, Idle) first, then the project web, then its other sessions); For what "Pick what to watch first."; Until nothing chosen, "How long is suggested once you pick what to watch."; footer none of "Pick how long first"/"Check it" (Watch button live only after a pick).
- After picking the web session: note "Bot read every part of your words. Check it, then click Watch."; For what "Its turn ends" ticked; Until Once "Suggested for “its turn ends”" / "Tells you once, then ends. You'll find it under Ended." Removing the pick again leaves the note "Bot read every part of your words. Check it, then click Watch." with nothing picked. Cancel: nothing saved.

## S11 (06:48): "tell me when web finishes unless it's after dark"
- Bot: "I couldn't fill the form from your words. The Watches form is open." [Open the form].
- Form note: "Bot couldn't fill in any part from your words. Pick each one. It couldn't read “unless it’s after dark”."; target empty ("Bot couldn't tell which one"); after picking the web session: note "Bot filled in what it could read. What to watch for, how long and quiet hours are left for you. It couldn't read “unless it’s after dark”."; "For what: Bot couldn't tell what for", the card "Its turn ends" tagged "Nearest", not ticked; Until "Bot couldn't tell how long from your words. Pick one."; quiet hours "Bot couldn't tell whether this should wake you. Check it."; footer "Pick how long first." Cancel wrote nothing.

## S12 (06:49): Ask Bot on the revert post, "tell me when it finishes"
- Bot, word for word: "I filled in the Watches form. Check it and click Watch." / "Tell you when “Change the date format in the API responses to ISO 8601” finishes its turn. Once." [Open the form]
- Form target: Change the date format in the API responses to ISO 8601 (Claude Code CLI · Personal), the post's session; nothing else filled. Saved once (Even during quiet hours ticked, as the quiet hours rule says) for S13.

## S13 (06:5x): the head with the watch count
- Head: "Bot · Beta · 👁 1 (1 watch) · + · search · expand · ⋯ · close" with the line "Apple on-device · Running" whole under it, at 1755×1348 (both themes).

## S14 (06:51): the three reviews, 6 s each
- Edits post: cue "AI · Apple's on-device model is writing lines on this turn…" shown at the first poll (≈1.5 s after the press), gone by ≈3 s; no AI line followed (code-only): Risk Low, Changed checkout/cart.ts +3 −1, totals.ts +5 −0, totals.test.ts +9 −0, Tests Pass "The tests pass: 12 passed, 0 failed."
- Revert: Risk Medium · tests not run · part undone; Changed src/format.ts +1 −1 with AI line "switched the API dates to ISO 8601"; Reverted src/locale.ts (agent's sentence); Tests Not run; "Lines by Apple's on-device model · Beta · Show its words · This is wrong".
- Failed: Risk High · it failed · tests fail; Changed parse/dates.ts +5 −3 with AI line "rewrote the date parser in dates.ts to accept two-digit years"; Tests Fail "The tests fail: 3 failed in dates.test.ts, all on years before 1970."; same footer.
- No line opened on and/or/but or ended on of/in/for.
