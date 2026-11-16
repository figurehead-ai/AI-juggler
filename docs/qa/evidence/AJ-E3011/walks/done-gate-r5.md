# AJ-E3011 done-gate walks on round 5 (2026-11-16)

Both walks are setup-only (main's ruling: ease ≥ 6/7, value ≥ 5/7, no screen scored "no" on the key question). The UI is a clean snapshot served with `scripts/qa/walk_bundle.sh`; each tester is a fresh Sonnet agent using browser tools only (firewall checked for each).

## Sam (P10, through the project's own pool) at ad964b48b, debug-build bridge

**S** (no detour). Ease 5/7, value 5/7, feeling +1 ("guided, wordy, reassured"). Every core control scored yes on the key question:
- "Who leads: Pick one to be in charge"
- the role list and its menu
- "On their own"
- the team-tools question ("costs nothing extra", "changes nothing in your apps")
- Review in sentences, and the ready line naming whom to open first

What held ease at 5:
- the nine template cards and their jargon at Start;
- a member's name squeezed into a narrow column in Everyone else;
- "On its own: Same as the pool" left unexplained;
- 1–3 s waits. The bridge's backend was a debug build: an optimised `walk` profile was added afterwards (e4f931d62, 023a529db).

## Casey (PC1: two Claude app profiles, never a terminal) at e4f931d62, walk-profile bridge

An earlier run on the walk-profile bridge was **void**. The profile's build had no `juggler-agent` beside the app, so the connector read as "can't take AI Juggler's connector yet" (fixed: the bridge builds the agent for its own profile, 023a529db).

It still exposed product bugs, all sent to the UI builder as round 6:
- the developer-facing words when the connector can't be added;
- "6 of 6 done" while nothing could work;
- Apps silently dropped on Make pool;
- the lead step's wrong hint;
- the lab strip covering Next.

**The valid run: SD** (one detour). Ease 5/7, value 5/7, feeling +1 ("guided, cautious, hopeful"). Casey added the connector to both profiles after opening "Show the files": "The preview is what made me comfortable … I knew exactly which files would be added". Checked after the run: each lab profile holds `skills/ai-juggler/.claude-plugin/plugin.json` (2 of 2). Both Apps were in the pool, Work Max leading from its next session and Side reviewing, with Work together and the team tools on, and the ready line correct.

The detour: on the project page, "New pool for web…" was clicked as the layout shifted under the pointer, and opened a session page.

## Where it stands

Both personas are at 5/7 on ease and value, short of the 6/7 ease bar. Round 6 is with the UI builder:
- from a project's own pool, the templates sit under a closed "Or start from a template";
- the member name gets the row's width, and "On its own" gets a hint;
- the ready line gets a first prompt to copy;
- a skeleton shows at once in place of a blank panel;
- "New pool for web…" opens the setup;
- no layout shift on the project page;
- Casey's run's bugs.

Both walks repeat on round 6 with the walk-profile bridge.
