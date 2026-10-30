# ADR-0023: Local cache — one system, sized by the machine, the user's to clear

- Status: Accepted (D-58, 2026-10-30)
- Date: 2026-10-30
- Deciders: the maintainer (ruling of 2026-10-30); cache builder (draft)
- Relates to: D-14 (secrets redacted), D-37 (what is read and kept), D-38 (app icons), D-52 (Windows and Linux), AJ-E1520 (subagent index), AJ-E1538 (search index)

## Context

AI Juggler runs on the user's own Mac. Several features already kept files to be quick, each in its own way:

- the subagent index (`subagents/<hash>.idx`): a reader's checkpoint per session, the newest 200 kept by its own prune;
- the app icons (`icons/*.png`): kept per bundle version, old versions removed, open to other users of the Mac;
- the search index of outside sessions (`outside-search.sqlite`): opt-in, its own switch, removed whole when turned off.

Nothing measured them together, nothing knew how much room the Mac had, and Settings said Last messages were "kept nowhere" while the subagent index kept prompts and results whenever they were on.

The maintainer, 2026-10-30: "remember our tool is installed on users machines. We are not exporting that data. So if you need to store certain data for local caching purposes so our app is faster or more capable, so be it. We can always options in settings where users can disable that if they choose or if they want clear cache. But we should be caching as much as possible to provide smooth experience, but at the same time we need to have solid invalidation system of updating it and managing. Like if our cache files get too big, our system should automatically monitor that. Our system is going to be used on multiple types of machines some lots of resource some very little so our system should be smart about the usage of resources."

## Decision

1. **What the cache is.**
   - It lives only on this machine and is never sent anywhere.
   - Every entry can be rebuilt from its source (the tools' own files, the apps' bundles), so none is a record: losing it costs time, never data.
   - Files are owner-only (`0600` in a `0700` folder where the folder is new) and written whole through a temporary file. Words are redacted before they are written (D-14).
   - Settings › Advanced › Local cache turns it off and clears it.

2. **One module, a registry of caches** (`crates/juggler-app/src/features/cache/`). A cache implements `LocalCache`:
   - what it is in plain words (name, what it speeds up, what is slower once cleared), where it lives;
   - its entries with size and last use, `remove` one, `clear` all, `purge_stale`;
   - whether it may be trimmed an entry at a time (`Trim::Partly`) or only whole (`Trim::WholeOnly`);
   - an optional entry cap, the settings that shape it, and whether it has its own switch.

   A folder of entry files registers with `FolderCache::new` in a few lines. Registered today:

   | Cache | Trim | Cap | Shaped by | Own switch |
   |---|---|---|---|---|
   | Subagent lists | partly | 200 sessions | Last messages | — |
   | App icons | partly | — | — | — |
   | Search index | whole only | — | — | Search the words of every session |

   No feature prunes on its own any more: the subagent index's 200 is the manager's cap.

3. **Invalidation.** Every entry's first line names its cache, its format version, a print of the settings that shaped it, and its source's identity (the bundle's version for an icon; for a subagent list, its session's key, while the adapter's checkpoint carries each transcript's own offset and is read again from the start when a file was replaced or shrank).
   - A read that finds any of them different drops the file and reads the source again: stale words are never shown as current.
   - A corrupt, unreadable, oversized or non-regular file is dropped the same way.
   - A setting change drops at once the entries it shaped (`setting_changed`, called from `app_setting_set`).
   - The first walk after a start drops entries of an older format; the icons' plain `.png` files of before are removed.

4. **The budget comes from the machine** (`budget.rs`, decimal units as Finder counts them):
   - **Automatic: 2 % of the free disk, between 64 MB and 2 GB.** 100 GB free gives 2 GB; 10 GB free gives 200 MB. 64 MB still holds every app icon and the indexes of a few dozen busy sessions, so a full disk keeps the app quick where it matters most. Above 2 GB nothing gets faster: today's caches come to a few hundred MB on a heavy setup.
   - **Fixed sizes** the user may pick instead: 250 MB, 1 GB, 4 GB (any value between the floor and 64 GB is accepted).
   - **Low disk: under 5 GB free, or under 5 % of a small disk, whichever is lower.** macOS starts purging its own caches near there. The cache trims to the 64 MB floor and writes nothing new (the search index's builder waits too) until free space is a fifth above the line, so it doesn't flap around it.
   - **Free space unknown** (no reading on this system yet, Windows today): 256 MB, what the 2 % share gives at 12.8 GB free.
   - **Memory.** 8 GB or less is a small Mac, up to 16 GB medium, more is large. What a cache keeps in memory scales by one half, one and two times its base: the Subagents tab's readers (4, 8 or 16 sessions) and the icons in memory (64, 128 or 256).
   - Disk and memory are read through the `juggler_platform::room` seam (`statvfs` on Unix, the system's memory count); a system without a reading says so.

5. **Monitoring that is cheap.** A background pass every minute (`ticks::every_while`):
   - reads the disk's free space (one call) and sets low-disk mode;
   - walks the folders only when due: every 30 minutes, 5 minutes after something was written, when the size counted on writes passes the budget, on low disk, or right after a setting changed;
   - walks at most 20,000 files per cache and removes at most 2,000 per pass, so a small Mac never stalls; it never runs on the UI's thread.
   - Trimming: each cache to its cap first, then the least recently used entries across all trimmable caches until the total fits. A read marks an entry used. A whole-only cache is never trimmed for the budget — dropping the search index would only start building it again — and is removed whole only on a nearly full disk.

6. **Subagent text.** The Subagents tab's prompts, results and the message that started them are kept, redacted, in the subagent lists whenever the cache is on and Last messages are on, so the tab and its search are instant. Showing text still follows Last messages: the cache follows what the app is allowed to read. **Turning Last messages off drops that text from the cache at once**, because the user asked the app not to read it; the lists are rebuilt without it. A session's own last messages are still read on demand and kept nowhere.

7. **Settings.** Settings › Advanced › Local cache, after the Data sections: one line ("Keep a cache on this computer", what it uses of its limit, the switch); Size limit (Automatic or a fixed size); Clear cache…, whose sheet says what is slower until it fills again; and, folded, each cache with its size and what it speeds up. Off removes the files of every cache without its own switch; the search index keeps to its own. Settings › Privacy says what is kept and opens it.

## Alternatives considered

- **Keep each feature's own pruning** — no total, no view of the machine, a fourth cache would bring a fourth policy. Rejected by the architecture standard (no per-feature hacks).
- **A fixed total (say 500 MB)** — too much for a nearly full 256 GB Mac, too little to matter on a 2 TB one. The maintainer asked for "smart about the usage of resources".
- **Low disk at 5 % of any disk** — on a 2 TB disk that is 100 GB free, far from full. The lower of 5 GB and 5 % keeps the line meaningful on large and small disks.
- **Trim the search index like the rest** — it is one SQLite file, slow to build and useless half-built; trimming it would loop (drop, rebuild, drop). It is shown and cleared on request, and removed only when the disk is nearly full.
- **Keep subagent text after Last messages are turned off, unshown** — the cache would hold what the user asked the app not to read. Dropped instead.
- **A database table for cache entries** — makes the cache a record, adds a schema change, and ties trimming to the store's lock. Files carry their own stamp.

## Consequences

- A new cache (previews, usage folds, conversation scans) registers in a few lines and gets invalidation, the budget, trimming, Settings' list and Clear cache for free.
- The cache's size is visible and bounded on every machine; a nearly full disk gets its room back within a minute.
- Entries written by 0.3's first nightlies (the index's own head, plain icon files) are dropped once and rebuilt.
- Windows has no owner-only files yet, so nothing is written there (as before), and free space isn't read: the limit is 256 MB.
- A write already under way when the cache is turned off can land; the next pass removes it, and nothing is read while it is off.
