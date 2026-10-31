<!-- captures: e98db6e4ab14 -->
# 98-local-cache: Settings › Advanced › Local cache, and Privacy's line about it

Design critique (D-57, AJ-E2814) against the reference screens: the row menus (77-menus), the rail's order menu, Usage and Processes. The baseline note says what changed on purpose; this says whether it is good. New family (ADR-0023, D-58).

## Checklist

| Check | Finding |
|---|---|
| Grid | One settings card: the switch row, Size limit, Clear cache, then What's in it as a fold; values and controls on the card's right edge. |
| Hierarchy | Section title "Local cache", then each row's name over its one-line explanation; the footnote under the card says where it lives. |
| Spacing | Rows on the settings cards' rhythm; What's in it's rows carry name, size · items on the right, one line of what it speeds up. |
| Icons | Clear cache… carries the broom; the warning carries the app's warning icon; "Turn it on in Privacy" carries the link-out mark. |
| Shared controls | The shared switch, the shared Picker for Size limit (a hint under Automatic only), the shared sheet for Clear the cache?. |
| Disabled reasons | Off: "Turn the cache on to set its limit", "Nothing to clear: the cache is empty" beside greyed controls. |
| Plain words | "Using 47 MB of 1.3 GB. Subagent lists and app icons open without re-reading files."; "Nothing of yours is lost." |
| Type sizes | Row names 14 px, explanations and hints 13 and 12 px. |
| Both themes | Checked dark and light; the low-disk warning reads in both. |
| 1280×800 and 1024×700 | Both; the menu and the sheet fit at 1024. |

## Good

- Every row says what it does in this user's numbers (47 MB, 1.3 GB, 64 MB on a full disk) and what clearing costs, item by item, in the sheet.
- The search index's row says it has its own switch in Privacy and links there, instead of a second switch here.
- Privacy's "What it keeps to be quick" line names everything the cache holds and links to Local cache.

## Weak

- The card above (Data: backup & restore) says its purpose twice, inside and under the card.

## Fixed

- Second re-render (seen by eye): an empty cache said "Using 0 B of 1.3 GB. Search index opens without re-reading files." (a machine's "0 B", and a claim that something is faster while nothing is kept). It now says "Empty for now. It fills as you use AI Juggler, up to 1.3 GB." Vitest: "summarises off and on in one line" and the clear test.
- The search index's line read "Search the words of every session answers at once.", a broken sentence. It is now "A search of every session's words answers at once." (the Rust registry, the preview and this gallery).
- Size limit's menu said "Trims the least recently used first" under each of 250 MB, 1 GB and 4 GB. Only Automatic has a hint now; once a size is picked, its row says "The least recently used goes first."
- The low-disk capture warned "Your disk is nearly full" under a header saying "This Mac · Plenty of room". The gallery seeded only the cache. A real disk that full (under 5 GB) also makes This Mac say "Under pressure", so the capture now seeds the machine with the same short disk, and the two agree.

## Captures

| Capture | What is on it |
|---|---|
| on-1280x800, -1024x700 | The cache on: "Using 47 MB of 1.3 GB…", Size limit Automatic (1.3 GB now), Clear cache (frees 47 MB), What's in it folded. |
| contents-1280x800, -1024x700 | What's in it open: Subagent lists 45 MB · 126 items, App icons 2 MB · 38 items, Search index Off with "Turn it on in Privacy". |
| search-empty-1280x800, -1024x700 | Search on but empty: "Empty for now. It fills as you use AI Juggler, up to 1.3 GB.", Clear cache greyed, the index's row Empty with "Change it in Privacy". |
| limit-menu-1280x800, -1024x700 | Size limit's menu: Automatic ✓ (2 % of free space, between 64 MB and 2 GB), 250 MB, 1 GB, 4 GB. |
| clear-sheet-1280x800, -1024x700 | Clear the cache?: frees 47 MB, nothing of yours is lost, what is slower until it fills again; Cancel · Clear cache. |
| off-1280x800, -1024x700 | The cache off: "Off: nothing is kept…", the limit and Clear cache greyed with their reasons. |
| low-disk-1280x800, -1024x700 | A nearly full disk: 12 MB of 64 MB and the warning "Your disk is nearly full, so the cache keeps to 64 MB and adds nothing until there's room." |
| privacy | Privacy › What AI Juggler reads: To know what's open, For Last messages, What it keeps to be quick (with Local cache… as its door). |

## 0.3.2

The Local cache card now stands clearly off the page in both themes (a lighter card on the darker canvas, a white card on grey), and its rows read in three levels: the row titles, the brighter sentence beneath ("Using 47 MB of 1.3 GB…"), and the quieter figures on the right ("45 MB · 126 items", "Off", "Empty"). The clear-sheet's "Clear cache" is now one solid violet next to a plain Cancel, so the dialog's one real action is obvious over the blurred page. low-disk is the strongest capture: the amber warning sits inside the toggle's row, and the header's "This Mac · Under pressure" with its red dot and the rail's "Your Mac is under pressure" agree with it. In off and search-empty the disabled Size limit and Clear cache… are faint, which reads as unavailable, though in light the off toggle's white track on a white card is the lightest thing on the page. In limit-menu the menu covers the end of the card's footnote ("Everything in it can b…") while open, and the 1024 captures open scrolled so the backup card's first line is cut at the top; both as before. Every capture was re-rendered this round.
