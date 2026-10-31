<!-- captures: 9177dc6cca29 -->
# 99-keep-alive: Open starts an app that isn't running, and Keep alive (0.3.3)

Design critique (D-57, AJ-E2814). First critique of this family, written at 0.3.3 from its captures.

## 0.3.3

Five screens, both themes, on the places desktop (two Claude profiles and ChatGPT running, a Lab profile kept alive that's down, an old one).

**home-kept.** Your apps gains a fifth tile after the four session tiles: Claude with the Lab badge, a dashed border, "Not running · keep alive starts it" in the quiet grey, and Start Claude in the same outline button as the other tiles' Open. The dashed edge says "not here" without a colour of its own, and the tile keeps the row's grid width. The Work Max tile carries a small violet "kept alive" pill under its count. A first pass wrote "· kept alive" inline, which wrapped to a line starting with a dot; the pill wraps as a unit.

**home-paused.** The same tile, its line in the warning colour: "Keep alive paused: it stopped 3 times in 10 minutes", on two lines, Start Claude under it. The tile reads as the one thing in Your apps to look at, which is right for a pause.

**settings-your-apps.** Settings › Desktop apps opens on the new section, its heading framed (the jump's focus) above five rows. Each row has the app's icon, its name with the profile as Home names it ("Claude · Work Max"), a grey state line, then Start or Open, the "Keep alive" word and its switch. Kept-alive rows come first. The paused Lab row adds the amber "Open it to start it and keep it alive again." with the warning icon, as other Settings warnings do. Forget shows only on the idle closed row (Old), as a quiet ghost button. Every switch and button lines up in one right-hand column. The footer explains Start and Keep alive and their limits in three lines.

**session-open-in.** The ended session's header: Continue in AI Juggler (the one primary), then "Open in Claude" and "Copy resume command" as secondaries. The button names the app only; the profile is the Personal chip under the title, and the full "Open in Claude · Personal" is its accessible name and tooltip. A first pass printed the profile in the button and cut the title to "Add pagination to the ord…"; now it reads "…to the orders end…". Where it runs says "App · Not open now · last in Claude" above "Profile · Personal", so the card and the button agree on where the session was.

**rail-menu-keep.** A right-click on Claude · Work Max in the rail opens its menu: Open its page, Bring it forward, Keep alive with a violet ✓ at the right, Fold. The ✓ is drawn as the rail's order menu draws its own. A first pass had the item ticked for screen readers but showed no tick.

## Weak

- settings-your-apps: the word "Keep alive" repeats on every row beside its switch. It names each switch, but a column heading would be quieter.
- home-kept: the closed tile shows no runway, though the session tiles do. Its account is known when its sessions ran on one.
