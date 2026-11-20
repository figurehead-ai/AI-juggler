# AJ-E3011 newcomer walk, after round 2: Sam (2026-11-15)

- **Build:** release/0.4.6 at 67af48eb9, from a clean snapshot. The P10 lab (seed state) through the lab write-mode bridge, in the built-in browser at 1280×800.
- **Tester:** a fresh Sonnet tester with the same brief and persona; browser tools only (firewall checked: 91 calls, all `mcp__Claude_Browser__*`).
- **Path:** Projects › web › its Pools tab › "Open the pool". The pool was set up on the diagram, not in New pool.

## Result

**SD**: ease 4/7, value 4/7, problem solved partly, feeling 0 ("patient, unsure, wary"). Worse than round 1's two runs through New pool (S, 5/7). The diagram path had not received round 2's care.

| Step | Finding (verified on screen by Opus unless noted) |
|---|---|
| Make Claude the boss | An App's "Its role" picker shows **Lead greyed**, with its reason only in the hint. It was clicked three times. The Who leads checklist step then worked ("Leads next"). |
| Make Codex the checker | Reviewer, with its line in plain words: **yes** |
| What they may do without asking | The pool inspector's "On their own" choices, each with its line: mostly yes. The header "On their own: Work together" didn't say whom it affects. |
| Team tools | "Turn on team tools / Not now" in the diagram's checklist: **no**, ignored. |
| The pool inspector | A wall of rules (the Allowed/Ask first/Blocked grid, "Always allowed here", "Messages into sessions / Deliver / Pause and ask after 5"): **no**, skipped. |
| Latency | ~5 s after clicks. Not confirmed as the product's: the page runs on the dev server, which compiles each screen's modules on first open (its resource buffer was full of 250 module loads), behind a pane that throttles frames when hidden. To be measured in the bundled Dev window. |
| Words not understood | Pool, Lead, Observer, Needs you, wake each other, team tools, Messages into sessions, Deliver, Always allowed here, Everyone in web. |

**What the tester thinks the pool does** (correct): Claude's next session in web leads; Codex reviews and doesn't build; Work together, anything more waits for the user's OK; every new web session joins as Member.

## Round 3 (sent to the builder)

1. An App's Lead in its role picker becomes "Lead: its next session leads", which sets the same lead_next. A wider group shows its reason as visible text.
2. The pool inspector shows a summary in sentences first; its rules sit in closed collapsibles, in plain words.
3. The header's "On their own" says it is for everyone in this pool.
4. The diagram checklist's team-tools card says everything the setup's card does.
5. "Needs you" is explained on its first mention.
6. The "Everyone in web" card's line is rewritten.
7. "Open the pool" never loses a click while loading.
8. The ungated drag test expects `join:`.
