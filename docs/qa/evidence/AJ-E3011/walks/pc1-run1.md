# PC1 Casey (Claude app only, two profiles): early walk of the connector UI (2026-11-16)

- **Build:** release/0.4.6 at 0557a46c6, the connector UI plus the lab write mode for the connector. A clean snapshot was served with `scripts/qa/walk_bundle.sh`; the bridge is from the same snapshot.
- **Lab:** PC1 (`/tmp/aij-lab-pc1`): two Claude app profiles (Work Max, Side), project web, no pools.
- **Tester:** a fresh Sonnet tester playing Casey, who has never used a terminal; browser tools only. Firewall checked: 68 calls.
- **Purpose:** an early check before round 5. The done-gate walk comes after it.

## Result

**SD**. Ease 5/7, value 5/7, solved partly to yes, feeling +1 ("guided, cautious, mostly clear"). Casey made "web team" through the project's "New pool for web…": Work Max leads from its next session, Side reviews, Work together, the team tools on.

**The connector's consent worked.** Casey added it to both profiles and said why: "needed for the two to talk; only new files, nothing existing touched; reversible through Remove". They felt safe because "Nothing is written until you add it", Cancel, and the Remove promise. Checked after the run: each lab profile holds `skills/ai-juggler/.claude-plugin/plugin.json` (2 of 2), and nothing was written outside the lab.

## What still costs ease

1. **The preview leads with long paths** (`/tmp/…/ClaudeCodeProfiles/…/plugin.json`), and "connector" isn't explained where it first appears. Lead with one plain sentence: "This adds a small helper to your Claude app (Side) so its sessions can talk in this pool: 2 new files in its own settings folder; nothing of yours changes." Put the files and their text under "Show the files".
2. **"Add it yourself instead" sounds like a terminal route.** Say "Show me how to add it by hand".
3. **The team-tools question, after Add:** "does this change my Claude app a second time?" One line: "This is a setting in AI Juggler; it changes nothing in your apps."
4. **"Who may message whom":** it isn't clear whether the cells can be changed. Make it visibly changeable (the cells as buttons, with "Click a cell to change it"), or say it is set on the diagram.
5. **A possible bug:** after Work Max was picked in Who leads, "No lead for now" kept its purple dot. Check the lead picker's checked state with a test.
6. Words still unclear: Pool (until Start explains it), Observer and Member as vague, Needs you, wake each other, Seal, Scope, worktree, and the diagram controls.

(The yellow "Not on the lab bridge's list" bar is the lab bridge's own note, not part of the app.)
