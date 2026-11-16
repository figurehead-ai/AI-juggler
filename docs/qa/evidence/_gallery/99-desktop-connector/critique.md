<!-- captures: b951f9c97137 -->
# 99-desktop-connector: desktop apps in a pool through AI Juggler's connector (AJ-E3013)

Design critique (D-57, AJ-E2814) against 99-pool-selection (the canvas and inspector around it), 92-agent-tools (the team tools card, the same card shape) and the calm UI round 2 rules. The Checkout team pool with a Claude app profile as an App member (no connector), a Claude app session connected, another Watched, and a ChatGPT (Codex app) thread; dark and light, 1280×800. Fixtures only: nothing is written or run.

## Checklist

| Check | Finding |
|---|---|
| Grid | The consent card sits in the inspector's 340 px column on its 16 px gutter, the same card as the team tools question (surface-2, 12 px radius, card shadow); in New pool it sits under "In this pool" in the side column. |
| Hierarchy | Card title (14 px semibold, ink) → consequence and example (ink-2) → the guide's link (accent) → Add (the card's one filled button) and "Add it yourself instead" (ghost). The preview's file lines are ink, their text folds behind "Its text". |
| Spacing | 8 px inside the card, 12 px between the inspector's sections; the canvas column keeps its 12 px between cards. |
| Icons | None added: the file lines are words ("Creates ~/…"), the fold uses the app's chevron. |
| Shared controls | One ConnectorCard (inspector, New pool, the switch's set-up list); the guide is the switch's own Show me how, now with the command's Copy; Remove's confirm is the danger button. |
| Disabled reasons | The ChatGPT thread has no Add: its line says why ("Its app can't take AI Juggler's connector yet; it can't be woken"). |
| Plain words | "Watched: its app has no AI Juggler connector", "Reads messages on its next turn", "Creates ~/Profiles/Work/claude-code/skills/ai-juggler/.claude-plugin/plugin.json", "Add to Claude app · Work Max", "Watching for it while these steps are open." No "MCP", "plugin dir" or "attested" on screen. |
| Type sizes | 12 px floor (the file text in the fold, the card's chips). |
| Both themes | Checked dark and light: Add's fill, the accent Add on the canvas cards and the file text's surface read in both. |
| 1280×800 | The canvas column scrolls (the ChatGPT card is below the fold); the preview's confirm stays in view with the text folded. |

## Captures

- members: the column: Claude app · Work Max (App, Member), "Watched: its app has no AI Juggler connector" with Add under it; Plan the trip notes app (Reviewer), "Claude app · Reads messages on its next turn", no Watched chip; Migrate the billing webhooks, Watched with why and Add; the ChatGPT thread below the fold. The old "can't reply" and the card's "Continue in AI Juggler" are gone. Pass.
- card: the App selected: its inspector says Watched and why, then the consent card: "Add AI Juggler's connector to Claude app · Work Max so its sessions can talk in this pool", what it writes and what it never touches, the example, "How desktop apps join a pool ↗", Add and "Add it yourself instead". Pass.
- preview: "What AI Juggler writes into Claude app · Work Max", "Only these files…", the two files ("Creates …/plugin.json", "Creates …/SKILL.md"), each with "Its text", then "Add to Claude app · Work Max" and Cancel. preview-text: the manifest's text open (the bridge's path and socket, the two hooks; no secret). Pass.
- guide: "Add AI Juggler's connector to Claude app · Work Max yourself": the four steps, the command with Copy, what shows it's done, and "Watching for it while these steps are open." Pass.
- connected: the connected session selected: "Session · Claude app · Reviewer", its pill "Reads messages on its next turn", "It talks in this pool through AI Juggler's connector.", and "AI Juggler's connector is in this app. Remove". Pass.
- new-pool: Who's in with Claude app · Work Max ticked: the side shows who's in, then the same consent card. Pass.

## Weak

- The App card's Add sits under the card (like Start one now), so the column grows by a row per Watched App.
- At 1280×800 the column holds three desktop cards before it scrolls; the ChatGPT thread's card is below the fold in every capture.
- New pool's desktop candidates all read "with AI Juggler's connector in it (added with your OK) its sessions talk in the pool; without it they're Watched", ChatGPT too, which can't take it yet; its card says so once it's in.
- The guide's "Show me how" button keeps the switch list's plain outlined style, a step down from the card's buttons.

## Round 5 (Casey's PC1 walk)

- card: under its title, what the connector is ("AI Juggler's connector is a small helper AI Juggler adds to an app so its sessions can read and send the pool's messages, notes and tasks."); "Show me how to add it by hand" in place of "Add it yourself instead". The inspector's Watched sentence says the same meaning. Pass.
- preview: opens with "This adds a small helper to your Claude app (Work Max) so its sessions can talk in this pool: 2 new files in its own settings folder; nothing of yours changes.", then "Nothing is written until you add it.", the files under a closed "Show the files", and Add to Claude app · Work Max with Cancel, all in view. preview-text: "Show the files" open, the manifest's text open. Pass.
- guide: "Add AI Juggler's connector to Claude app · Work Max by hand". Pass.
- All: the inspector is 380 px.
