# AJ-E3011 done-gate walk, after round 3: Sam through the diagram path (2026-11-15)

- **Build:** release/0.4.6 at 2f3467475. Both the UI (vite from the snapshot) and the bridge come from a clean snapshot. An earlier run on the same day was **void**: the dev server hot-reloaded an uncommitted edit, and every pool crashed with "InspectPool.tsx does not provide an export named 'Section'". The setup was fixed and the run repeated.
- **Lab:** P10 (seed state, with the "Empty" pool scoped to web).
- **Tester:** a fresh Sonnet tester with the same persona. The brief pointed at the project, as main asked for the diagram path. Browser tools only.

## Result

**SD**, with small detours. Ease 5/7, value 5/7, solved partly ("configured, but nothing is running yet"), feeling +1 ("hopeful, mildly unsure, busy"). That is up from round 2's 4/7, but short of the 6/7 bar.

| # | Step | Worked | Value | Clarity | Feeling | Note |
|---|---|---|---|---|---|---|
| 1–2 | Add Claude, then Codex | 5 | 4 | 4 | 4/3 | The toast "is in the pool" helps. Codex showed ~2–3 s late, which looked like a failed click. |
| 3 | Who leads › Change › Claude's next session | 4 | 4 | 4 | 4 | "Pick one to be in charge" confirmed it. "Leads next / next session" was confusing: nobody is in charge until a session starts. |
| 4 | Codex card › Its role › Reviewer | 5 | 4 | 3 | 4 | Found only through the hint text; the menu once showed late. |
| 5 | Header "On their own (everyone in this pool)": kept Work together | 4 | 4 | 3 | 4 | "Anything more waits for your OK" matched the goal. "Wake each other" sounded like it costs money. |
| 6 | "Turn on team tools" | 5 | 4 | 3 | 4 | It was low in a long side panel and found by searching. The checklist still said "3 of 6 done", with defaults shown as to do. |

## Opus verification

The run's end state was checked against what the panel would show (the team tools On, Claude "Leads next", Codex Reviewer, Work together) and the tester's account of it. That account is correct. The delays are still to be measured in the bundled Dev window: this page is the dev server behind a pane that may be hidden.

## Round 4

1. When the team tools are off, say so at the top of the pool page, with its Turn on next to it. Don't leave it low in the side panel.
2. The checklist counts a step left at its default as done ("default ✓"). Once the essentials are set (who's in, team tools on), it says the pool is ready.
3. "What happens next" in one line once ready: "Open Claude Code in web: it leads; Codex checks its work; their messages show here".
4. The lead line reads "Claude Code · Work takes charge as soon as one of its sessions starts in web".
5. Add and role changes show at once (a pending look within 100 ms), then settle.
6. The inspector's rarer settings (seal, "Pools its agents may write to", "Your other Macs", "Kept apart") go under one closed "More settings".
