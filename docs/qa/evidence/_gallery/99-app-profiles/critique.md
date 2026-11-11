<!-- captures: fc603170d733 -->
# 99-app-profiles: Claude app profiles found when they aren't running (0.4, D-62, AJ-E3203)

Design critique (D-57, AJ-E2814). First critique of this family, written at 0.4 from its captures.

## 0.4

The audit's decision 5 and scenario A6: someone with six Claude app profiles saw only the ones running today, so the App list hid what they have. Five screens, both themes, on the places desktop (the Work Max and Personal profiles running, Lab kept alive and down, Old seen 9 days ago), with two more profile folders, Design and Research, beside them in `~/Profiles`. Each is reached the way a user would: Work Max's page from the rail's App header, then Home, then Settings › Desktop apps from the menu.

**ask** (1280) and **ask-1024.** Work Max's App page, Overview. One card above "Open now" says what would be read and where: "Find this app's other profiles. Claude app keeps each profile in a folder of its own, in `~/Profiles`. AI Juggler can read the names of the folders there, and nothing inside them, so the profiles that aren't running show too." Under it, quieter: "You're asked once. Settings › Desktop apps can turn it off later." Two buttons at its right, "No thanks" (ghost) then "Find them" (secondary): the page's one filled button stays "Go to it". The folder is in the mono face so the path reads as a path. At 1024 the card's text wraps to two lines and the buttons stay beside it. The header reads "Claude app · Work Max": the profile is named by the Claude Code folder its sessions use, as the user names it there, not by its own folder ("Work").

**found.** After "Find them": the card becomes one line, "Found 2 more profiles: Design, Research. Each is an app of its own now, with Start.", with OK. The rail's Inactive apps lists four now: Lab and Old (seen, with when), then Design and Research, "Not running", each with its Start button. They come after the ones used, since they were never seen running.

**found-app.** Design's own page, from its name in the rail: "Claude app · Design", facts "Claude app · Not running · found with your other profiles · No sessions yet", the primary button "Start". One card says what will fill the page. No Usage or Processes tab (nothing to show yet).

**home.** Home's Your apps after "Show 2 more apps": the running apps' tiles, then the closed ones, Lab (kept alive) and Old first, then Design and Research, each "Not running · found with your other profiles" and "Start Claude app".

**settings.** Settings › Desktop apps, Your apps: Design and Research are rows like the others, with Start and Keep alive but no Forget (a found profile would come straight back; its folder or the switch decides). Below the rows, the switch "Find profiles that aren't running": "Reads the names of the folders beside a profile that ran (`~/Profiles`), and nothing inside them. Off forgets the profiles found that way." It's on after the yes.

## Fixed while critiquing

- The result line ("Found 2 more profiles…") stayed on screen when moving from Work Max's page to Design's: it's now the page's own, and a new page starts without it.
- The rail said "Not running · just now" for a profile only found by its folder, as if it had just been seen; it says "Not running" and sorts after the apps you've used. Home's closed tiles put them after the used ones too.
- Settings offered Forget on a found profile, which would come back a minute later; it's gone for those rows.
- "Asked once." read like a fragment; it says "You're asked once. Settings › Desktop apps can turn it off later."

## Weak

- The ask sits above "Open now", the page's most used card. It's shown once, until answered, and it's the only thing on the page that needs an answer, so it leads; it's still one more card the first time.
- A found profile has no Account until its sessions run once (nothing inside its folder is read to learn it), so its page shows no Limits, and its name is its folder's ("Design") until then.
- The Settings switch is a row without an icon below rows with app icons; it governs the list, so it stays in the same section.
- The rail's open App headers still cut "Claude app · Personal" to "Claude app · P…" at 1280 (from before this change).

## 0.4.1 re-score (2026-11-07, AJ-E3305)

The whole family was retaken on release/0.4.1 for the 14-line re-score (AJ-E3305, `docs/qa/0.4-walks/design.md`) and opened on contact sheets, both themes, 1280 and 1024. Shared with every screen since the last shoot: the header's New session is tinted (one filled button per view), the Floor button is a split control with its own menu, This Mac's line is its dot below 1120 px, and the rail's App headers put the profile first.

Settings › Desktop apps now lists running apps first, then by last seen, then the ones found (the preview sorted by Keep alive and name before; the backend already did this): ChatGPT, Personal and Work Max running, then Lab, Old, then Design and Research found (9d96ccd06).
