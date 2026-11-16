# AJ-E3011 done-gate walks on round 6 (2026-11-16)

Setup-only walks (main's ruling: ease ≥ 6/7, value ≥ 5/7, no screen scored "no" on "Do I know what this does and who it affects?"; each tester also says what would have made ease and value a 6 or a 7). The UI is a clean snapshot at f7435d2d1 (round 6, the section-title fix 6db085536, the danger tint d26547319) served with `scripts/qa/walk_bundle.sh`. The backend is the optimised `walk` profile on a fresh lab (`AIJ_LAB_PROFILE=walk scripts/qa/lab_scenario.sh lab-bridge`). Each tester is a fresh Sonnet agent with browser tools only; each end state was checked on screen by Opus.

## Sam (P10: a Claude Code CLI and a Codex CLI on project web, through the project's own pool)

**S** (no detour). **Ease 6/7, value 5/7**, feeling +1 ("guided, wordy, reassuring"). Problem solved: partly (set up, not seen working).

- Path: Projects → web → Open the pool → Set up web's own pool: Start from scratch, Where, Who's in (both Apps), Who leads (Claude), Everyone else (Codex → Reviewer), Who may message whom (left as is), On their own (Work together), Team tools (Yes), Review → Set up the pool.
- The key question: yes for every core control (Open the pool, Start from scratch, the Apps' boxes, Who leads, the Role menu, On their own, Team tools, Review's Change). "No" only for things off the path: the word "pool" before "as one team" explained it, an "Empty" pool from the lab's seed, Show the diagram, the Sessions/More tabs, "New sessions join as", Seal this pool.
- What would have made **ease** a 6 or 7 (setup): an immediate visible response on each Next, and calling the step where the checker is chosen "Who checks the work" rather than "Everyone else".
- What would have made **value** a 6 or 7 (after setup): seeing them work (a live line such as "Claude handed Codex a review, Codex said OK") and a sign that the roles and permission choice are enforced.
- End state, checked by Opus: "web · all sessions"; Claude Code CLI · Work leads when a session starts; Codex CLI · Personal is the Reviewer; Work together; team tools on; "This pool is ready" with the first prompt to copy.

## Casey (PC1: two Claude app profiles, Work Max and Side, never a terminal), New pool

**SD** (one detour: she reached Review, read that both profiles "can't talk here yet: it needs AI Juggler's connector first", went back to Who's in and added it). She called it S. **Ease 6/7, value 5/7**, feeling +1 ("calm, curious, slightly cautious"). Problem solved: partly.

- The connector: she added it to both profiles after the preview ("2 new files in its own settings folder; nothing of yours changes; nothing is written until you add it"). She opened "Show the files" and decided yes because it was specific, said nothing of hers changes, and Remove takes out exactly what it wrote. Comfort 4/5. The files landed only in the lab's two profile folders (checked by Opus).
- The key question: yes for every core control (Pools, New pool, Start from scratch, the Apps' boxes, Who leads, the Role menu, On their own, Team tools, Show the files, Add to Claude app, Make pool). "Partly" for the message grid, "On its own" next to the Role menu, and Add as a first press that only previews. "No" for the words Watched, wake each other, worktree, connector, plugin.json / SKILL.md, Seal this pool and Needs you; she guessed most of them right.
- What would have made **ease** a 6 or 7 (setup): ask for the connector as a clearly marked step where the profiles are picked, not only a note that Review catches later; instant feedback on clicks; fewer steps for a plain "boss and checker" team.
- What would have made **value** a 6 or 7 (after setup): seeing them work together.
- End state, checked by Opus: "web team" in web; Claude app · Work Max leads when a session starts; Claude app · Side is the Reviewer; both "added · joins at its next start · reads messages on its next turn"; Work together; team tools on; "This pool is ready".

## Verdict against the done-gate

Both walks meet main's bar: ease 6/7 (≥ 6), value 5/7 (≥ 5), and no core setup screen answered "no" on the key question. Both testers put value's missing point after setup (seeing the team work), which is AJ-E3307 R7 on the RC.

## Found in these walks, and where each goes

- **The ready line names "Claude app" for the reviewer when both members are Claude app profiles** ("…and ask Claude app to review"): it must name the member (Side). A bug, fixed in 0.4.6 (AJ-E3011 round 7).
- **The connector is asked for only as a note in Who's in**, and Review is the first place that says the team can't talk without it (Casey's worst moment and her ease 6→7 point). It is fixed in 0.4.6 (AJ-E3011 round 7): Who's in shows each desktop App's connector state on its own row, with Add, before Next.
- **"Everyone else" as the title of the step where the checker is chosen** (Sam's ease point). It is fixed in 0.4.6 (AJ-E3011 round 7) with a title that says what is chosen there.
- **Clicks that looked unchanged for 2–4 s.** Measured in the browser pane during Sam's run, with load average 12–23 from other work on the Mac: setup's Next took 400–416 ms to the first frame and the canvas's Diagram 1.0 s, with no long task on the page's main thread (List: 6–13 ms). That points at the pane's compositing under load, not the app's code. Round 5 measured 10–15 ms for the same steps in the real Dev window. This goes to the RC walk (AJ-E3307), which times them again in the Dev window.
- **"Show the files" doesn't open with an accessibility press** (the aij-test proof's driver; a `<summary>` styled `inline-flex`). It is checked and fixed in 0.4.6 with round 7.
