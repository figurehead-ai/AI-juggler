# What relates to what: the relation map, each page's tabs, one list for all (AJ-E3101)

Research date: 2026-10-30
Author: research agent (read-only on the repo)
Scope: E31 (`docs/tasks/E31-everything-links.md`). This file is the evidence; the proposal page (handed over with E30's) holds the wireframes.
Abbreviations:
- **M** = `crates/juggler-core/src/storage/mod.rs`
- **CMD** = `crates/juggler-app/src/commands.rs`
- **RI** = `ui/src/components/RightInspector.tsx`

How known:
- **Stored** = a column or table.
- **Read** = from a tool's own files.
- **Inferred** = worked out. The UI says "probably" where D-36 asks.

---

## 1. Words in code vs on screen (ADR-0008 A-2)

| On screen | In code today | Note |
|---|---|---|
| Profile (an app's setup folder: a Claude Code folder, a `CODEX_HOME`, a Claude app profile) | the `accounts` table, keyed by `state_dir` (M:103-109); `provider_id` is the tool (M:105, renamed at M:270) | page kind `account` (`ui/src/pages/registry.ts:127-128`) is really the profile |
| Launch settings | `launch_recipes` (was `profiles`, M:112-117, renamed M:274) | under a profile |
| Account (subscription) | not stored; `subscriptions_list` works it out on each read from every profile's sign-in (`crates/juggler-app/src/features/subscriptions/mod.rs:259`, grouped at `:101`); UI `useSubscriptions` / `accountOfProfile` (`ui/src/lib/subscriptions.ts:44`) | one login on two tools counts as two accounts (test at `subscriptions/mod.rs:416`) |
| Session (AI Juggler's) | `instances` (was `sessions`, M:120-127, renamed M:271) | |
| Session another app runs | a conversation: no `session_id`; in memory (`ConversationIndex`, `crates/juggler-app/src/features/conversations/mod.rs:182`), filled from the tools' files | `ui/src/lib/conversations.tsx:67-76` |
| Mac | `instances.machine` (M:124); paired Macs through Fleet (`fleet_replica`, M:171) | |

## 2. The map (each pair)

| Pair | How known (file:line) | List today (file:line) | Cost today | Shown today | Both ways? |
|---|---|---|---|---|---|
| project ↔ session | **Stored** `instances.project` (M:141), set from the cwd at launch (CMD:2947, 3065). Conversations: **Inferred** from the cwd's git root, with worktrees folded in (`crates/juggler-core/src/storage/projects.rs:377,410`) | `sessions_page` / `sessions_counts` by `project_id` (`features/sessions/commands.rs:52,70`; `rows.rs:297-388`); `projects_list` counts (CMD:7405) | full scan of instances and conversations in memory, then paged (`crates/juggler-app/src/paging.rs:186`) | Project › Sessions, History; session header project chip | yes |
| project ↔ profile | **Stored** `projects.default_account` (M:563); otherwise **Inferred** through its sessions | `ProjectInfo.accounts_used` (CMD:7430-7455); `projectAccounts` (`ui/src/lib/project-accounts.ts:14`) | scan | Projects avatars; Project Overview | project → profile only |
| project ↔ account | **Inferred** (profile → account) | client-side only | scan, plus the sign-in reads (cached 30 s) | not shown | no |
| project ↔ pool | **Stored** `pools.project` (`pools.rs:50`); one default pool per project (`pools.rs:59-69`) | `pools_list` (`features/pools/commands.rs:19`) | small | Project › Pool; Pools screen | yes (one pool) |
| project ↔ Mac | local projects are this Mac's; remote from the peer (`features/fleet/mod.rs:75`) | `fleet_launch_options` (`fleet/mod.rs:236`) | live call | Fleet only | no |
| project ↔ tool | **Inferred** from its sessions' tool | none | scan | not shown | no |
| session ↔ profile | **Stored** `instances.launch_recipe_id` (M:123, M:273) → `launch_recipes.account_id` (M:114). Conversations: **Read** (the folder the file came from, e.g. `crates/juggler-adapters/src/claude_conversations.rs:20`) | `sessions_page` by `account_id` (`rows.rs:328,353`); `AccountSessions` (`ui/src/pages/AccountPage.tsx:109-134`) | scan (no index on `launch_recipe_id`) | profile page › Sessions; session header badge | yes |
| session ↔ account | **Inferred** (profile → account) | `accountOfProfile` (`subscriptions.ts:44`) | sign-in reads, cached | session header chip; rail "account" lens | session → account only |
| session ↔ pool | **Stored** `pool_members` (`pools.rs:70-79`), else the project's default (`pools.rs:113`); conversations are in none | `pools_place` (`features/pools/commands.rs:47`), `pools_members` (`members.rs:184`) | members: scan of briefs | Pool canvas; Project › Pool; rail "pool" lens | yes |
| session ↔ Mac | **Stored** `instances.machine` (M:124); remote through Fleet | `fleet_remote` (`fleet/remote.rs:925`) | scan / peer | session header Mac chip; Fleet | session → Mac only |
| session ↔ tool | **Inferred** by join (recipe → profile `provider_id`); conversations **Read** `tool` | `InstanceInfo.provider` | scan | app icon on every row; tool page | partly |
| profile ↔ account | **Inferred** from the sign-in (`subscriptions/mod.rs:259-297`) | `subscriptions_list` | sign-in read per profile | Accounts screen | yes |
| profile ↔ tool | **Stored** `accounts.provider_id` (M:105, M:270) | `accounts_list` (CMD:1280) | small | tool page lists profiles (RI:913-923) | yes |
| profile ↔ pool | none (proposed: profile members, E30) | none | n/a | not shown | no |
| profile ↔ Mac | implicit (each Mac its own); remote `RemoteRecipeOption.account` (`fleet/mod.rs:62`) | `fleet_launch_options` | peer | New session (remote) | no |
| account ↔ pool / Mac / tool | none / none / **Inferred** (`SubscriptionInfo.tool`) | `subscriptions_list` | sign-in reads | Accounts screen | no |
| pool ↔ Mac | **Stored** flag `pools.spans_macs` only (`pools.rs:57`) | `pools_set_spans_macs` | n/a | pool settings | no |
| pool ↔ tool | none | none | n/a | not shown | no |
| Mac ↔ tool | this Mac: observed processes and the install scan (CMD:1425,1472); remote from the peer | `fleet_remote` | live | This Mac; Fleet | no |
| task ↔ pool | **Stored** `tasks.pool_id` (`crates/juggler-core/src/storage/tasks.rs:40`), indexed (`:60`) | `tasks_list` etc., SQL keyset paging (`storage/tasks.rs:228-237,675-713`) | indexed | Pool › Tasks; Tasks screen | yes |
| task ↔ session | **Stored** `task_sessions` (`tasks.rs:72`, index `:80`) | `tasks_session` → `useSessionTasks` (`ui/src/pages/tasks/useSessionTasks.ts:9`) | indexed | Session › Tasks | yes |
| task ↔ project | **Inferred** through `pools.project` | `counts_by_project` (`features/tasks/across.rs:217`) | pool scan, then an indexed count | Project › Tasks | yes |
| Work item ↔ project / profile / Mac | **Stored** `work_items.project` (indexed, M:483-489); `epochs(account, provider, machine_id)` (M:491-500, M:191) | switch graph | indexed | session History | yes |

**What's missing.**
- No index on `instances.project`, `instances.launch_recipe_id` or `instances.machine` (searched every `CREATE INDEX` in M).
- Conversations are walked in full on each list.
- No stored account entity.
- No summary table of counts. The D-58 cache (`crates/juggler-app/src/features/cache/mod.rs:123`, ADR-0023) holds subagent lists, icons and the search index only.

## 3. Today's tabs (`ui/src/pages/registry.ts:117-138`)

| Page | Tabs |
|---|---|
| session | (desktop) Window, Overview · (terminal) Overview; then Subagents*, Messages*, Tasks*, Usage, Processes, History* (* when there's any) |
| project | Overview, Sessions, Tasks, Pool, Subagents, Usage*, History |
| profile (kind `account`) | Overview, Sessions* |
| conversation | Overview, Subagents*, Usage*, Processes (live) |
| tool | Overview |
| machine | Overview, Processes, Actions |
| remote session | Overview, Quality, Processes, Actions |
| pool | Members, Tasks, Messages, Agent access (the canvas folds Members and access in, D-55) |

## 4. Proposed tabs (counts on every related tab; singular relations as header chips)

The rule: a relation to **one** thing is a chip in the page header (one click). A relation to **many** is a tab with its count, opening on the active ones first. Existing tabs are kept unless named as replaced.

| Page | Header chips (one each) | Tabs (new in bold) |
|---|---|---|
| Project | default profile; Macs it's on (Fleet) | Overview · Sessions (n) · **Apps (n)** (profiles, grouped by account, app icons) · **Pools (n)** (replaces Pool: a project can be in several pools) · Tasks · Subagents · Usage · History |
| Session | project · profile · account · Mac · Work item | Overview · Subagents · Messages · Tasks · **Pools (n)** · Usage · Processes · History |
| Profile | app · account · Mac | Overview · Sessions (n) · **Projects (n)** · **Pools (n)** · **Launch settings (n)** · Usage |
| Account (new page) | plan (if known) | Overview (limits, runway) · **Profiles (n)** · **Sessions (n)** · **Projects (n)** · **Pools (n)** · Usage |
| Pool | scope (projects) · sealed | Canvas · Tasks · Messages (unchanged, D-55); inspector sections Projects · Members by type · Macs |
| Mac | — | Overview · **Sessions (n)** · **Profiles (n)** · **Projects (n)** · **Pools (n)** · Processes · Actions |
| Tool | — | Overview · **Profiles (n)** · **Sessions (n)** · **Projects (n)** · **Pools (n)** |

**Default order everywhere:**
1. Needs you
2. Working
3. Open (idle)
4. Active in the last 7 days
5. The rest, by last activity

**Filters (chips):**
- state (Open now · Recent · Ended);
- app;
- profile or account;
- project;
- Mac (only when paired).

Search appears once the list can pass 8 items.

**Empty states teach.** Some examples:
- A project's Pools tab: "No pool names this project yet. Its sessions are watched together in its project pool. Make a pool to give them roles."
- A profile's Pools tab: "Add this profile to a pool, and every session it starts there joins by itself."

## 5. One list for every related tab: `RelatedList`

- **Input:** the related kind and the anchor (e.g. sessions of profile P), the default order, the filters allowed.
- **Data:** one paged command per related kind, using `paging.rs`'s `PageRequest` / `Page` (`crates/juggler-app/src/paging.rs:25,76`) with a keyset cursor (`page_of_fetched`, `:214`). The UI uses `usePagedList` (`ui/src/lib/paged.ts:103`) and `PagedList` (`ui/src/components/PagedList.tsx:64`).
- **Row:** one anatomy shared with the rail and Home (D-47): app icon with a state dot, title, state · detail, a profile badge, a project mark, when.
  - Today there are three: `SessionRowBody` (`ui/src/pages/SessionRow.tsx:50`), `PlaceRow` (`ui/src/components/PlaceRows.tsx:289`), and the Home tiles.
  - The build unifies them into one row with density variants. The fields are not merged per screen.
- **Counts:** from a relation-count cache (a D-58 `LocalCache` entry, invalidated by session, profile, pool and Fleet change events), never by re-reading transcripts.
- **Budget:** a related tab opens in ≤ 150 ms from the cache. The first page is 50 rows (`DEFAULT_LIMIT`, `paging.rs:17`).
- **Pickers** use the same order with sections ("Open now", "Recent", "In this project", then All), per `docs/plan/QUALITY.md:70`.

## 6. Build needs found

1. Indexes on `instances(project)`, `instances(launch_recipe_id)` and `instances(machine)`.
2. A conversations index by project, profile and tool, kept as the scan fills it.
3. An account page from the computed grouping (cached), with no stored entity yet.
4. The relation-count cache.
5. One `RelatedList` and one row.

---

## 7. Fit: how linking serves the goal and works with everything else

### 7.1 The user value and the goal it serves

| Goal | What links add |
|---|---|
| See what's open, where | From any page, the open things related to it come first, one click away. "Which apps work on this project right now?" is one tab. |
| Juggle many subscriptions | The Account page (and the entity audit's App page) shows each subscription's apps, sessions, projects and room together. The question "which account is this on, and what else is using it?" no longer needs the Accounts screen. |
| Switch | The session header's line-of-work chip and History tab show every account and app a piece of work passed through. Every page those touched links back to it. |
| Keep agents working | Needs you comes first in every list, so what blocks an agent is never below the fold. Pools tabs show which team a session works in. |

**Words.** If the entity audit's decisions 1 and 4 pass (`docs/research/entity-model-audit.md` §10), the "Profile page" here *is* its **App page**. Its tab set merges with audit §5.1: Overview, Sessions, Projects, Usage, Settings, Processes, plus Pools from this map. The **Account page** reads the stored `subscriptions` row instead of the computed grouping. Either way the tabs and the list are the same.

### 7.2 How it works with every related capability (both ways)

| Capability | What links use | What it gets from links |
|---|---|---|
| **Switching** (ADR-0011) | `work_items` and `epochs` (indexed, M:483-500): every account, app and Mac a line of work used. | A session's History tab lists those hops, each linking to its App, account and Mac. Counts on those pages include the sessions that passed through. |
| **Tasks** (E19) | `tasks.pool_id` and `task_sessions` (indexed). | Tasks tabs on project, pool and session, and task counts on Apps and accounts later if wanted. A task row links to its holder's session, and the session links back. |
| **Messaging** (E18) | Threads per pool. | A session's Messages tab links each sender's session. A pool's Messages links every member. |
| **Usage, room and credits** (E11, E22, D-45) | `usage_spans(account, session)`, the usage history, the account's room. | A Usage tab scoped to each entity, and the account's room on the Account and App pages. "Suggested" by room stays only on your own launch and switch sheets (D-45). |
| **Notifications** (E25) | A notice's `data` says what a click opens (AJ-E2501 req. 1). | The click opens the entity page at the right tab (the session's Tasks, the pool's Messages). Each page's Needs you section counts the same notices as the bell, never a second count (AJ-E2502). |
| **Processes** (E24) | The process ledger by session and Mac. | A Processes tab on session, App and Mac, each process linking to the session it serves. |
| **Fleet** (E7) | The `fleet_replica` and the peer's live answers. | A Mac chip and a Mac filter on lists. Remote rows link to the remote session page. Counts from an offline Mac say "as of" when they were last seen. |
| **Local cache** (D-58) | The cache registry and its invalidation by stamp. | A relation-count entry and the conversation index by project, App and tool, rebuilt on session, App, sign-in, pool and Fleet events. A cleared cache shows the page at once and the counts a moment later. |
| **Pools** (E30) | Pools by member type (pools-model §4.2). | Pools tabs everywhere, with how each session got in. The wizard's pickers are this map's pickers. |
| **The rail and Home** (D-47, D-36) | One row anatomy. | The same row in the rail, on Home and in every related list. Group headers open the App page (fixing the audit's G1). |

### 7.3 Dependencies

- **This needs:**
  - the instance indexes;
  - the conversation index by project, App and tool;
  - the relation-count cache;
  - the entity audit's App and Account (stored, or computed as today);
  - E30 for pools by App, account and Mac;
  - E28's pages, which carry today's tabs.
- **This is needed by:**
  - E30's wizard (pickers);
  - the App and Account pages;
  - every picker the pickers rule names (`docs/plan/QUALITY.md:70`);
  - the 5-second test in the walk.

### 7.4 End to end, with edge cases

**A walk through.**
1. Home → project *AI Juggler* → **Apps (5)**.
2. Open *Claude Code · Work* → **Sessions**: open first.
3. Open one session → its header chip **Account: Work** → the Account page.
4. **Pools (1)** → the pool's canvas.

Each step is one click, and each page links back.

**Edge cases.**

| Case | What happens |
|---|---|
| A session another app runs | It appears in its project's and App's lists as that app's row, with "probably" where its state is inferred (D-36). |
| A session with no project | Listed under "No project" on its App and account. The project filter offers "No project". |
| A worktree | Folded into its repository's project (`crates/juggler-core/src/storage/projects.rs:410`). The row shows the worktree's branch. |
| An unreadable sign-in | The App shows "account unknown", and the account's counts leave it out with a note. |
| Thousands of ended sessions | Paged by keyset, 50 at a time. Counts come from the cache, never from re-reading transcripts. |
| Counts change while a page is open | The cached count shows first and updates on the next event. A list never jumps while you scroll: new rows wait behind "3 new". |
| A remote Mac offline | Its rows and counts show "as of" when it was last seen. |
| A project renamed or moved | Keyed by id, so the links hold. |

### 7.5 What changes for existing users

- Nothing is removed.
- New tabs appear with counts: Apps and Pools on projects; Projects, Pools and Launch settings on the App (profile) page; a new Account page; related tabs on Mac and tool pages.
- The project's **Pool** tab becomes **Pools**.
- Rows look the same everywhere.
- Long lists open on what's active, with search, instead of in plain date order.
- The App grouping's header opens the App page, not a page titled "Account" (audit G1).
