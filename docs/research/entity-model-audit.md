# Account, App, Project, Session: an audit of how AI Juggler models them (0.3)

Research date: 2026-10-30
Author: research agent (read-only on the code; writes only this file)
Code read: `release/0.3` at `deb209e39` (what the maintainer runs, 0.3.0/0.3.1). Every `file:line` below is in that tree.
Companions: `relations-map.md` (which page links to what) and `pools-model.md` (pools), both in this folder. Section 9 lists where this audit and those two must agree.

How known:
- **Code** = read in the source, cited.
- **Seen** = run in the preview (`ui`, vite, mock data) on 2026-10-30.
- **Real setup** = the maintainer's Mac, read-only: directory names, and a `sqlite3 -readonly` count of non-secret columns. No file contents, no credentials.

---

## 0. Summary

The maintainer's model has four things: an **Account** (a subscription), an **App** (a desktop app or CLI in one of its profiles), a **Project** (a code folder) and a **Session** (a conversation in an App, on an Account). AI Juggler stores only two and a half of them. Projects and sessions are fine. An Account is never stored: it's worked out on each read from each folder's sign-in, and named after those folders. An App doesn't exist at all. What the code calls `Account` is a config folder: a Claude Code `CLAUDE_CONFIG_DIR`, a `CODEX_HOME`. It's what the UI calls a "profile" on the Accounts screen and an "Account" on its own page. The Claude app's own profiles (`ClaudeProfiles/b-anela` and five others on the real setup) are never stored. They appear only while running, as a place's "instance".

So the App grouping draws app-and-profile headers whose click opens that folder's page, titled **Account**, filed under the Accounts tab. That page mixes the Claude app's sessions with Claude Code's sessions in Terminal, because both write into the same folder. That is exactly what the maintainer hit (`ui/src/components/PlaceBoard.tsx:136-137`).

The fix:
- **An App page.**
- **A real Account page.**
- **One word per entity on screen.**
- **A small additive schema change in 0.4:** stored accounts and apps, with membership still read from the tools' own files.

There are 24 gaps. Three block a journey, 16 confuse and five are polish.

Section 11 covers each recommendation in turn: its user value, how it works with switching, pools, tasks, room and fees, notifications, Processes, Fleet and navigation (both ways), its dependencies, and what changes for existing users.

---

## 1. The model as the maintainer stated it (2026-10-30)

> "Account - Subscription. App - Physical desktop or CLI app or Profile. Projects - Git, code project. Session - Chat session belonging to App/Profile which also is connect to account. Multiple apps/profiles can have same account."

| Entity | Means | Has many | Belongs to |
|---|---|---|---|
| **Account** | a subscription you pay for (a Claude Max plan, a team seat, a ChatGPT plan, an API key through a service) | Apps, and through them sessions | — |
| **App** | one app you run, in one of its profiles: the Claude app in profile `b-anela`; Claude Code with folder `bookauth-anela`; Codex with `~/.codex`; the ChatGPT app's Codex | sessions, launch settings | one Account (the sign-in), its tool |
| **Project** | a git or code folder | sessions | — |
| **Session** | one conversation | — | one App, so one Account; one Project |

Two more words sit around these four:
- **Tool** is a family (Claude Code, Codex).
- **Launch settings** live under a CLI App.

### 1.1 Each tool on the four entities

| Tool | Account | App (one per…) | Session record | Stored today |
|---|---|---|---|---|
| Claude app (desktop) | the Anthropic sign-in (subject + organisation) | each app profile: `--user-data-dir` (`ClaudeProfiles/<name>`), or the default `~/Library/Application Support/Claude` (`crates/juggler-adapters/src/claude_live.rs:225-238`) | its Code sessions, `<instance>/claude-code-sessions/<account>/<org>/local_<id>.json` (`claude_live.rs:8-14`). Chats are not sessions | **No.** It's seen only while it runs, as `PlaceInfo.instance` (`crates/juggler-app/src/features/conversations/dto.rs:81-83`) |
| Claude Code (CLI) | the same sign-in | each config folder: `~/.claude`, `CLAUDE_CONFIG_DIR` (`ClaudeCodeProfiles/<name>`). It runs in Terminal, iTerm, an editor, or AI Juggler | `<folder>/projects/*.jsonl`; live: `<folder>/sessions/<pid>.json` (`claude_live.rs:3-7`) | Yes: a row in `accounts` (`crates/juggler-core/src/model.rs:182-226`) |
| Codex (CLI) | the ChatGPT sign-in | each `CODEX_HOME` | rollouts | Yes: an `accounts` row |
| ChatGPT app (Codex) | the same ChatGPT sign-in | the app (it runs one; `instance_arg: None`, `crates/juggler-adapters/src/codex_live.rs:74-80`) | the same rollouts | **No.** Only a place named "ChatGPT" |
| OpenCode, Grok Build, Kimi Code | their sign-ins or keys | each config or home folder | their stores | Yes: `accounts` rows |
| An account through a service (D-34) | the service's key | each folder that uses it | as its tool's | Folder rows. The account is worked out (`crates/juggler-app/src/features/subscriptions/mod.rs:107-121`) |

**The Claude app's Code sessions run Claude Code with a config folder.** The scan lab models it: app profile `b-work` runs a `claude` whose session file sits in folder `profiles/work` (`crates/juggler-app/src/features/conversations/scan/tests.rs:96-146`). On the real setup the rail named the place "Claude · bookauth-anela", not "Claude · b-anela". So the app profile `b-anela`'s sessions write into the Claude Code folder `bookauth-anela`. That folder is also what `claude` in Terminal uses. This is inferred from the place's name: `rail-items.ts:218` shows the folder's label before the instance. **One folder serves two Apps.** That is the shape the four-entity model handles and today's model doesn't.

### 1.2 The real setup (read-only)

- `~/Library/Application Support/ClaudeProfiles` has six Claude app profiles: `b-anela b-faruk b-luka b-mia b-olivia personal-max`.
- `ClaudeCodeProfiles` has five Claude Code folders: `bookauth-anela bookauth-luka bookauth-mia bookauth-olivia personal-max`.
- There is one `~/.codex`.
- The `accounts` table has seven rows: `Default (~/.claude)`, the five `ClaudeCodeProfiles` folders and `Codex (~/.codex)`. **None is a Claude app profile.**
- Each row has one launch setting, "Main".
- `instances` holds one session AI Juggler started. Everything else the maintainer sees is a session another app runs.

---

## 2. How the code represents them today (angle 1)

| Maintainer's entity | In code | In storage | On the wire / UI |
|---|---|---|---|
| Account | `SubscriptionInfo`: grouped on every read from each folder's sign-in, keyed by `tool + subject + org`, hashed (`subscriptions/mod.rs:1-13, 101-189, 135, 246-249`) | **Not stored.** Its plan, renewal and fee are stored **per folder**: `accounts.plan`, `accounts.renewal_date` (`model.rs:189-205`), and app setting `subscription_fee:<folder id>` (`subscriptions/mod.rs:262-266`) | `useSubscriptions`, `accountOfProfile` (`ui/src/lib/subscriptions.ts:44-46`). Its name is its folders' names (`subscriptions.ts:59-64`) |
| App | **None.** The pieces are scattered: <br>• `HostApp`, an app *type* (`places.rs:22-62`, `hosts.json`) <br>• `PlaceInfo {app, instance, profile}`, only while live (`dto.rs:68-93`) <br>• `RailPlace`, UI-only (`ui/src/lib/rail-items.ts:29-42`) <br>• the CLI folder row | Nothing for desktop app profiles. The CLI folder is the `accounts` row | Rail headers, Home's "Your apps" tiles |
| (the folder, no entity of the maintainer's) | `Account` struct: "a harness login … materialized as an isolated state directory" (`model.rs:182-184`) | `accounts` (`id, provider_id, label, plan, state_dir, …`) | Called **profile** on the Accounts screen (`ui/src/screens/accounts/ProfileRow.tsx:1-6`). Called **Account** on its page (`ui/src/pages/registry.ts:21`) |
| Launch settings | `LaunchRecipe` (`model.rs:251-266`), with the alias `pub type Profile = LaunchRecipe` (`model.rs:268`) | `launch_recipes.account_id` → folder | "Launch settings" (`ui/src/lib/account-words.ts:18-23`) |
| Project | `ProjectRecord` (`model.rs:470-500`) | `projects` | Good |
| Session | `InstanceRecord` (AI Juggler's, `model.rs:814-862`); `ConversationInfo` (other apps', `dto.rs:3-66`) | `instances.launch_recipe_id` → recipe → folder. Conversations in memory | `account_id` / `account_label` on every session = **the folder** |

**"Profile" means three things in this code.**
1. Launch settings: the `Profile` alias at `model.rs:268`, and `InstanceRecord.profile` at `model.rs:824`.
2. A config folder: the `accounts` row, per D-35 and the Accounts screen.
3. The folder's label shown as a place's profile (`PlaceInfo.profile`: "its account's name, else its folder's", `dto.rs:79`).

And "account" means two: a folder (`account_id` everywhere) and a subscription (`SubscriptionInfo`). That's how the click bug came about. `group.items[0].accountId` reads as "the account" but is a folder id.

**A place's key and its label come from different entities.** The key is `app | instance ?? profile` (`rail-items.ts:186-188`). The label shows `profile ?? instance` (`rail-items.ts:218`). On the session page, the place's own label is `app · instance` (`crates/juggler-app/src/features/conversations/commands.rs:160-166`). So one Claude app profile is "Claude · b-anela" on the session page and "Claude · bookauth-anela" in the rail.

---

## 3. Gaps

Severity:
- **blocks**: a journey can't be done.
- **confuses**: it can be done, but the words or targets mislead.
- **polish**: minor.

Release:
- **0.3.x**: a bug fix that needs no new entity.
- **0.4**: needs the App or Account entity, the new pages, or a schema change.

| id | angle | what's wrong | where | user impact | sev | release |
|---|---|---|---|---|---|---|
| G1 | clicks | In the App grouping, a place header's name opens the page of the **first session's folder**, titled "Account", under the Accounts tab. The tooltip promises "Open Claude · bookauth-anela's page" | `ui/src/components/PlaceBoard.tsx:127-138` (`pageOf`: `group.items[0]?.accountId` → `openAccountById`); `PlaceHeader.tsx:42-43, 66`; `ui/src/lib/nav.ts:43`. Seen: "Claude · Work Max" opened "Claude Code › Account · Work Max" | The maintainer's report. You expect the app and get a subscription-looking page that also lists Terminal sessions | blocks | 0.4 (App page); words 0.3.x |
| G2 | model | There is no App entity, no App page and no App selection | `ui/src/pages/registry.ts:12` (PageKind); `ui/src/App.tsx:98-108` (Selection); `places.rs:22-62` (types only) | Nothing answers "this app: its sessions, projects, settings" | blocks | 0.4 |
| G3 | model | The Claude app's profiles aren't stored. One that isn't running is invisible. One that is shows only as `instance` | `dto.rs:81-83`; `running_setups.rs:20-23`; real setup: 6 app profiles, 0 rows | You can't see, name, order or filter your Claude app profiles. The Accounts screen never lists them | blocks | 0.4 |
| G4 | model | The Account (subscription) isn't stored. It's grouped per read and named after its folders ("bookauth-anela", "Work and Home", "A, B and 2 more") | `subscriptions/mod.rs:1-13`; `ui/src/lib/subscriptions.ts:59-64` | The account and the folder share a name, so "bookauth-anela" means two things. An account can't be renamed. Its name changes when you rename a folder | confuses | 0.4 |
| G5 | pages | No Account page. Every "account" door opens the **first folder's** page | Account lens header: `PlaceBoard.tsx:136`. Usage runway card: `ui/src/components/usage/RunwayCards.tsx:81`. Home "Measure its limits": `ui/src/components/HomeRunway.tsx:94`. Accounts card title: no door, `ui/src/screens/accounts/AccountCard.tsx:114-116` | With two folders on one subscription, "the account" opens one of them at random | confuses | 0.4 |
| G6 | pages | The page kind `account` is a folder's page. It mixes words: header "Account", crumb "Claude Code" (the tool), "Usage on bookauth-anela", "where this profile works", "Nothing running on this profile now", "No sessions on this account yet", "Settings for this profile" | `registry.ts:21, 127-128` (type label, drawn at `ui/src/pages/PageShell.tsx:83`); `ui/src/pages/ItemHeader.tsx:151-160`; `ui/src/pages/AccountPage.tsx:75, 118, 136, 184` | Four words for one page | confuses | 0.3.x (words), 0.4 (page) |
| G7 | pages | The folder page lists every session written to the folder, from the Claude app and from Terminal alike. Nothing on it says which app ran each one | `AccountPage.tsx:108-140` (`sessionsQuery({account_id})`). Seen: "Running now: … Claude Code on Work Max" and "… in Claude · Work Max" | From the Claude app's header you get Terminal's sessions too | confuses | 0.4 |
| G8 | words | A place's label is the CLI folder, not the app's own profile. Its key is the instance | `rail-items.ts:186-188, 218`; `commands.rs:148-166` | "Claude · bookauth-anela" in the rail, "Claude · b-anela" as the session page's Window. Two Claude app profiles on one folder get two identical headers | confuses | 0.3.x |
| G9 | groupings | For CLIs the App grouping groups by **host** (Terminal, iTerm, VS Code), not by app. AI Juggler's own sessions are one place whatever their folder | `places.rs:122-190` (host = first app bundle up the chain); `rail-items.ts:147` (own sessions: `profile: null`) | Claude Code and Codex in Terminal are two "Terminal" headers. Work and Personal folders run from AI Juggler share one header | confuses | 0.4 (decision 2) |
| G10 | clicks | The session header's one chip is labelled **Account** and holds the folder's name. It opens "Open bookauth-anela's page", the folder page. Its "Folder" fact is the session's **working folder**, not the profile's | `ui/src/pages/session/AccountMenu.tsx:57, 65-66, 73, 77, 85-86, 91` | The account, the app and the cwd are blurred into one chip | confuses | 0.3.x (words), 0.4 (two chips) |
| G11 | clicks | "Where it runs" names App, Window, Profile and Account as plain text: no links, two different profile names | `ui/src/pages/session/WhereItRuns.tsx:28-35`; `SessionHead.tsx:63-70` ("Running in the Claude app", text) | The page that knows where a session runs can't take you there | confuses | 0.3.x (links to what exists), 0.4 |
| G12 | rows | One folder gets two marks and three colours: <br>• `ProfileBadge` (`profileColor(name)`) in the rail, Home and the session header <br>• `AccountAvatar` (`accountColor(id)`) on page rows, Accounts rows, the folder page and the Account lens <br>• the account's own colour (`accountKeyColor(sub)`) on cards and Usage | `ui/src/components/ProfileBadge.tsx:38, 51`; `ui/src/lib/profile-marks.ts:66`; `ui/src/components/Avatar.tsx:36`; `ui/src/pages/SessionRow.tsx:109`; `ProfileRow.tsx:87`; `ItemHeader.tsx:179`; `PlaceBoard.tsx:284`; `AccountCard.tsx:112` | Colour can't link a row to its app or account | confuses | 0.4 |
| G13 | rows | Page session rows always show the folder (avatar and name), even on that folder's own page. They say the app only for other apps' sessions ("in Claude") | `SessionRow.tsx:96-110` | "43 sessions all labelled with the account", and no app column | polish | 0.3.x |
| G14 | groupings | The Account grouping does group by subscription. But its header opens the first folder, has no ↗, and takes its colour from the first folder's id | `ui/src/lib/rail-places.ts:49-50, 69`; `PlaceBoard.tsx:136, 265-268, 284` | Half right | confuses | 0.4 |
| G15 | groupings | Only `account:` filters by the folder's label, the same as `profile:`. No filter takes a subscription | `ui/src/lib/rail-view.ts:196-205`; `ui/src/components/WorkflowRail.tsx:404` | `account:work` doesn't mean the work subscription | confuses | 0.4 |
| G16 | words | "Which account?" lists folders' launch settings | `ui/src/screens/NewSession.tsx:322` | Two folders on one plan look like two accounts with separate room | confuses | 0.3.x (words), 0.4 |
| G17 | words | "Set its account…" and "New sessions here start with" pick a folder | `ui/src/lib/action-specs-setup.ts:68`; `ui/src/components/ProjectDefaultAccount.tsx:27-47` | The same | confuses | 0.4 |
| G18 | words | The project page's "Accounts used" counts folder labels, then says "No moves between accounts … 3 profiles worked here" | `ui/src/pages/project/Overview.tsx:97-101, 177`; `crates/juggler-app/src/commands.rs:7433-7440` | A count of the wrong thing | confuses | 0.3.x (words), 0.4 |
| G19 | pages | Home's "Your apps" tile: the name opens nothing. Only "Open Claude" (bring it forward) | `ui/src/components/HomeApps.tsx:43-101` | The one place called "apps" has no app door | confuses | 0.4 |
| G20 | pages | The Accounts screen lists folders under each account. A Claude app profile shows only as a "Claude app" chip, never by its name. A folder's Open goes to the page titled Account | `AccountCard.tsx:30-46`; `ProfileRow.tsx:83, 91, 148` | `b-anela` never appears on the Accounts screen | confuses | 0.4 |
| G21 | model | Settings that belong to the subscription (plan, renewal, fee) are edited per folder. Settings that belong to an app (desktop window preview, tidy-up) also sit on the folder | `model.rs:189-205`; `AccountPage.tsx:97`; `ui/src/pages/account/Settings.tsx` | Two folders on one plan can disagree. A "differs" mark papers over it (`subscriptions/mod.rs:192-226`) | polish | 0.4 (migration) |
| G22 | specs | SPEC §3 still says profile "is demoted to launch settings … never a rail noun and never means Account". D-35, §9.1's amendment and the guide say otherwise. The guide promises "That profile's page: its account, and the sessions open on it" for a place's name | `docs/SPEC.md:101-105` vs `docs/review/decision-log.md:708-728`, `docs/SPEC.md:541-547`; `docs/guide/whats-open.md:94`; `docs/guide/accounts.md:5-8` (says Claude app profiles are profiles; none are stored) | The specs disagree. Agents follow the wrong one | confuses | 0.3.x (docs) |
| G23 | model | An account's key includes the tool (`in|tool|subject|org`, and the test "one login on two tools is two accounts"). Fine today. But once a Claude app profile is stored with its own tool id, one Anthropic plan would split in two | `subscriptions/mod.rs:135, 414-421` | A trap for the migration | polish | 0.4 |
| G24 | clicks | ⌘K lists sessions and projects, never apps or accounts, though AJ-E2106 Req.4 asks for apps | `ui/src/components/GoToPalette.tsx:103-109` | No keyboard way to an app or account | polish | 0.4 |

Not gaps (they work):
- the Project grouping (headers open the project);
- the Pool grouping (headers open the pool, `PlaceBoard.tsx:131-134`);
- the place ↗ (brings the app's process forward, never launches it, `PlaceBoard.tsx:139-157`);
- a row's click (the session page);
- the session's tool door (`ui/src/pages/session/HeaderDoors.tsx:48-70`);
- Account grouping's grouping itself (`rail-places.ts:49-50`).

---

## 4. Click targets: today and should (angle 3)

| Where | Goes to today | Should go to |
|---|---|---|
| Rail, App grouping, header name | the first session's folder page, "Account" (`PlaceBoard.tsx:136-137`) | **that App's page** |
| Rail, App grouping, header ↗ | brings the app forward (`PlaceBoard.tsx:139-157`) | the same |
| Rail, App grouping, "AI Juggler" header | Home (`PlaceBoard.tsx:135`) | Home (its sessions are listed there) |
| Rail, Account grouping, header | the first folder's page (`PlaceBoard.tsx:136`) | **that Account's page** |
| Rail, Project / Pool header | the project / pool page | the same |
| Session header, "Account" chip | a menu → the folder page (`AccountMenu.tsx:57, 91`) | two chips: **App** (→ App page) and **Account** (→ Account page) |
| Session "Where it runs": App, Profile, Account | text (`WhereItRuns.tsx:30-35`) | App → App page, Account → Account page, Project → project (already a link) |
| Folder page crumb "Claude Code" | the tool in Settings (`ItemHeader.tsx:159-160`) | App page crumb: "Accounts › *account* › *app*" |
| Home, "Your apps" tile name | nothing (`HomeApps.tsx:54`) | App page; "Open" stays "bring it forward" |
| Home, "Runway by account" name / "Measure its limits" | nothing / the first folder's page (`HomeRunway.tsx:71-74, 94`) | Account page / the Account page's Limits |
| Usage, runway card name | the first folder's page (`RunwayCards.tsx:81`) | Account page |
| Usage, the "who" bars (by profile) | the folder page (`ui/src/components/usage/Dashboard.tsx:114-119`) | App page |
| Accounts, card title | nothing (`AccountCard.tsx:114`) | Account page |
| Accounts, folder row name / Open | the folder page (`ProfileRow.tsx:91, 148`) | the App page (one row per App, its folder as a detail) |
| A notice about a limit (target `account_id`) | the folder page (`ui/src/lib/notice-actions.ts:58, 83`) | Account page |
| Project, "Accounts used" tile | nothing | Account pages; plus an "Apps" tab (as `relations-map.md` §4) |

---

## 5. Pages: what exists, what each misses (angle 4)

| Page | Exists | Holds | Misses against the four entities |
|---|---|---|---|
| Session | yes | Overview (Where it runs, last messages), Usage, Processes, … | App and Account as doors (G10, G11). Two profile names (G8) |
| Project | yes | Overview, Sessions, Tasks, Pool, Subagents, Usage, History | Apps that worked here. "Accounts used" counts folders (G18) |
| "Account" (a folder) | yes | Overview (limits, usage, running now, usual projects, settings), Sessions | It's a folder page: it can't say which App ran what (G7). Limits belong to the account (D-35) |
| **App** | **no** | — | (G2) |
| **Account** | **no** (only cards on the Accounts screen) | — | (G5) |
| Tool | folded into Settings › Accounts (`nav.ts:66-84`) | install, its folders | Fine: Tool is outside the four |

### 5.1 The App page (proposed)

Header:
- app icon, profile badge beside it (D-38), name "Claude app · b-anela" (renamable);
- facts line: tool · "on **Anela (bookauth)**" (a door to the Account) · "uses folder `~/…/bookauth-anela`" · open now / last used;
- actions: **Go to it** (bring it forward, as the ↗) or **New session here** (CLI apps), and ⋯ (rename, hide from the rail, show the folder in Finder, its launch settings).

Tabs, each with its count:

| Tab | What it holds |
|---|---|
| Overview | • what needs you here, what's open now <br>• the account's runway, saying when it's shared ("shared with Claude Code · bookauth-anela") <br>• this week's tokens <br>• the projects it was used for, most recent first |
| Sessions (n) | this App's sessions only, open first; columns project, state, when. Never the account repeated on every row |
| Projects (n) | the projects it worked in, with session counts and last use |
| Usage | the usage dashboard scoped to this App |
| Settings | launch settings (CLI apps); the desktop window preview and tidy-up (moved from the folder page); "Show its sessions in the rail" (Settings › Desktop apps, per app) |
| Processes | while it runs |

### 5.2 The Account page (proposed, as `relations-map.md` §4)

Header: name (renamable), the sign-in's masked email or team, plan, renewal, fee.

| Tab | What it holds |
|---|---|
| Overview | limits and runway (moved from the folder page), the burn-downs, "Continue elsewhere" when it's out of room |
| Apps (n) | each App on this account, with its icon, open count, and a door to it |
| Sessions (n) | every session on this account |
| Projects (n) | the projects worked on with this account |
| Usage | the usage dashboard scoped to this account |

The folder becomes a detail of its App(s): its path, sign-in state, Sign in, Use the original. It's no longer a page of its own.

---

## 6. Groupings and rows (angle 5)

**App grouping today.** Each group is a host process plus the instance, labelled by the folder:
- **Two Claude app profiles on one folder** give two headers with the same name (G8).
- **One Claude app profile on a folder also used in Terminal** gives two headers ("Claude · bookauth-anela", "Terminal · bookauth-anela"). Their name clicks both open the same folder page (G1).
- **Claude Code and Codex in Terminal** give "Terminal · bookauth-anela" and "Terminal · Codex". The host leads the name; the app doesn't.

**Should be.** One header per App: "Claude app · b-anela", "Claude Code · bookauth-anela", "Codex · ~/.codex". The terminal or editor is the row's small icon and the ↗'s target. Go to it still reaches the right window (D-37 is unchanged: the place is still read from the parent chain).

**Row anatomy.** The rail row, Needs-you row and Home tile use the app icon + `ProfileBadge`. Page rows and Accounts rows use `AccountAvatar` for the same folder (G12). Proposed:
- one mark per App: the app icon with its profile badge;
- one colour per Account: the account's colour, as a thin edge or dot;
- never a round "account" disc for a folder.

This is the one row `relations-map.md` §5 asks for.

---

## 7. Journeys (angle 6)

Each was walked through the code paths. J1 was also walked in the preview.

| # | The user wants | Today | Breaks at |
|---|---|---|---|
| J1 | (the report) to open the Claude app in profile b-anela from the App grouping and see its things | Click "Claude · bookauth-anela" → "Claude Code › Account · bookauth-anela" under the Accounts tab. Limits, then "Usage on bookauth-anela", then "Running now", which includes the Terminal sessions on that folder | G1, G2, G6, G7. Nothing on the page is about the Claude app |
| J2 | "everything running on my work subscription" | The Account grouping groups correctly (`rail-places.ts:49-50`). The header opens one folder (G14). The Accounts card has no page (G5). Usage's card opens one folder | G5, G14 |
| J3 | "open the Claude app profile this session runs in" | Session page: "Running in the Claude app" (text). The **Account** chip shows `bookauth-anela` and opens the folder page. "Where it runs" says Window "Claude · b-anela" and Profile "bookauth-anela". Go to it works | G8, G10, G11: two names, no door to the app |
| J4 | "which projects did I use this app for?" | No App page. The folder page's "Usual projects" (`AccountPage.tsx:46-48, 75-95`) mixes the Claude app and Terminal, from folder labels (`accounts_used`) | G2, G7 |
| J5 | "this subscription is out of room: which apps and sessions are affected, where do I continue?" | Needs you shows each session's limit. The Accounts card lists folders and an app chip per folder. Continue on another account works per session. No page lists the account's apps and sessions together | G5, G20 |
| J6 | "I have the Claude app with three profiles on two subscriptions and Claude Code in two folders; show me that" | Accounts shows two cards with the **two folders** under them. The three app profiles appear only as a "Claude app" chip on whichever folder they use, and only while they run. The App grouping shows only the running ones, named by folder | G3, G8, G20 |
| J7 | "start Claude Code on my personal subscription" | New session: "Which account?" lists folders' launch settings (`NewSession.tsx:322`). With two personal folders it looks like two accounts. It works, but the room check counts per folder | G16 |
| J8 | "rename my work subscription to Acme Team and set its renewal once" | The account's name is its folders' names (`subscriptions.ts:59-64`). Renewal and fee are per folder, with a "differs" mark | G4, G21 |

---

## 8. Specs and decisions (angle 7)

| Source | Agrees with the maintainer | Conflicts |
|---|---|---|
| **D-35** (`decision-log.md:708-728`) | Yes: "a profile is the setup an app runs on … an account is the subscription". "Profile / app is what each native app runs on" is his word | "The stored entity stays one per folder in 0.1.x". Still so in 0.3. So a Claude app profile, named a profile in D-35, has no row. "Accounts where limits and money live", yet limits and plan sit on the folder page |
| **D-36** (`decision-log.md:730-748`), **SPEC §9.1** (`SPEC.md:541-547`) | "place = an app + profile pair (D-35) such as 'Claude app · Work'" is the maintainer's App | The place is built from the **host** for CLIs (G9). No page for a place |
| **AJ-E2109** (`docs/tasks/E21-desktop-first.md:184-201`) | The Accounts screen groups folders under subscriptions | "a look on the real setup" is still open. The real setup is where it broke. "The stored entity stays per folder until 0.2" passed without a change |
| **SPEC §3** (`SPEC.md:101-105`) | "an Account is one subscription/login" | "Profile is demoted to launch settings … never a rail noun and never means Account" (D-02/D-10, pre-D-35). It was never updated (G22) |
| **D-10 / D-02** (`decision-log.md:22-31, 111-117`) | — | Superseded by D-35 in part, but still cited in code comments (`model.rs:251`) |
| **UX spec §4.5** (`docs/review/deep/11-ux-redesign-screen-spec.md:98-108`) | An "Account detail" page with health, quota, profiles and recent sessions is the Account page proposed here | Its "profiles" are launch settings (pre-D-35) |
| **Guide** (`docs/guide/accounts.md:5-10`, `docs/guide/whats-open.md:94`) | Profile vs account, in the maintainer's terms | It promises a place's name opens "that profile's page"; it opens a folder page titled Account. It lists the Claude app profile as a profile; none is stored |
| **`relations-map.md` §4** (0.4) | It proposes a Profile page and a new Account page | Its "Profile" is this audit's App. Keep one word (decision 1) |
| **`pools-model.md` §4.2** (0.4) | Members by profile and by account | Its "Profile" member is this audit's App. Its "App (a tool)" member is a **Tool** (decision 7) |

---

## 9. Recommendations

### 9.1 Words (one per entity, everywhere)

| Entity | On screen | Never |
|---|---|---|
| Account | **Account**: the subscription; its own name | a folder's name; "profile" |
| App | **App**, named "*app* · *its profile*": "Claude app · b-anela", "Claude Code · bookauth-anela", "Codex · ~/.codex", "ChatGPT · Codex" | "Account"; the host terminal as its name |
| the app's profile | "profile" only inside an App's name or facts ("its profile b-anela") | a page or a noun of its own |
| the config folder | "folder" (a fact on the App page) | "Account", "profile" |
| Tool | **Tool**: Claude Code, Codex | "App" |
| Launch settings | as now, under a CLI App | "Profile" (drop `pub type Profile = LaunchRecipe`, `model.rs:268`) |

Copy that can change now (0.3.x, no new entity):

| Where | Change |
|---|---|
| `PlaceHeader.tsx:66` | The tooltip stops promising the place's page |
| `AccountMenu.tsx:65-91` | The chip says "Profile", and its "Folder" fact says "Working folder" |
| `AccountPage.tsx` | One word, "profile", throughout, until the App page replaces it |
| `registry.ts:21` (`PAGE_LABELS.account`) | The page's type label "Profile", not "Account" |
| `NewSession.tsx:322` | "Which profile?" |
| `Overview.tsx:97, 177` | "Profiles used" |
| `SessionRow.tsx` | Drop the folder column on that folder's own page |
| docs | SPEC §3; `docs/guide/whats-open.md:94` |

### 9.2 Click targets

The "should" column of §4. The App and Account pages come first. Every door in §4 then points at them.

### 9.3 Data model (0.4, additive, with migration)

1. **`subscriptions`** (the Account):
   - columns: `id`, `vendor` (`anthropic`, `openai`, or a service id), `identity_hash` (sha256 of vendor + subject + organisation; never the raw ids), `name` (yours, renamable), `plan`, `renewal_date`, `fee_usd`, `created_at`, `removed_at`;
   - `accounts.subscription_id` (nullable) caches each folder's last-read sign-in. It's re-pointed when a sign-in changes, and the UI says so.
   - Key by **vendor**, not tool (G23, `subscriptions/mod.rs:135`).
2. **`apps`** (the App):
   - columns: `id`, `kind` (`desktop` | `cli`), `tool`, `host_app_id` (`claude-app`, `codex-app`; null for a CLI), `instance_dir` (the desktop profile's data folder; null for the default), `folder_id` → `accounts.id` (the config folder it uses, learned from its sessions' pid files), `label` (yours; default the app's own profile name, else the folder's label), `hidden`, `first_seen`, `last_seen`.
3. **Sessions get their App:**
   - `instances.app_id` (nullable). AI Juggler's sessions get the CLI App of their recipe's folder.
   - A session another app runs gets its App from its place while it's live. Once ended, it comes from the desktop app's own per-instance session list (`claude_live.rs:8-14` records an app profile's sessions even after they end), else the CLI App of its folder.
4. **Code names** (an ADR-0008-style phased rename; wire values unchanged for a minor version):
   - today's `Account` struct becomes `ConfigFolder` (alias kept);
   - `SubscriptionInfo` becomes `Account`;
   - `App` is new;
   - `account_id` on the wire keeps meaning the folder until a versioned DTO adds `app_id` and `account_id` (the subscription) beside it.
5. **Migration:**
   - Backfill one CLI App per existing folder row. On the real setup that's 7.
   - Desktop Apps are created when first seen running (`places.rs` `running_instances`). The Claude app profiles not running are found per decision 5.
   - Run today's grouping once to create the `subscriptions` rows. Copy plan, renewal and fee where the folders agree; leave them unset where they differ, and say so once.
   - Move `subscription_fee:<folder>` app settings onto the account.
   - Nothing is deleted. A downgrade ignores the new tables. Back up as for v32 and v37 (`juggler.sqlite.before-v*`).
6. **Membership stays read, not stored:**
   - which sessions an App has comes from the tools' files, as today (D-37);
   - the stored rows hold identity, names, order and settings only.

---

## 10. Decisions for the maintainer

1. **The middle entity's word on screen is "App"**, with the app's profile in its name ("Claude app · b-anela"). This replaces D-35's "profile" as the noun. *Recommend: yes.* It's your word, and "profile" already means three things in the code.
2. **A CLI App is the tool + its folder**: "Claude Code · bookauth-anela". The terminal or editor is where its window is (a row detail and the ↗'s target), not its name. This amends D-36's place for CLIs. *Recommend: yes.*
3. **A Claude app profile and Claude Code in Terminal on the same folder are two Apps** on one Account, each with its own sessions. They aren't one App. *Recommend: two.* You run them as two things, and the app's own session list tells them apart.
4. **Store Accounts and Apps** (two additive tables in 0.4, §9.3). Today they're worked out on each read. *Recommend: store them.* An account then gets its own name, renewal and fee, and an app is remembered when it isn't running. Which sessions belong where stays read from the tools' files.
5. **Find Claude app profiles that aren't running** by listing the folder **names** next to one seen running (e.g. `ClaudeProfiles/*`). Never read their contents. *Recommend: yes, behind "Find this app's other profiles" on the App page the first time.* Add a Privacy page line.
6. **An Account's name is its own**: renamable, by default the sign-in's team, else its plan ("Claude Max"), else the masked email. It's no longer its folders' names. *Recommend: yes.*
7. **Align the pools model** (`pools-model.md` §4.2): its "Profile" member becomes **App** and its "App (a tool)" member becomes **Tool**, before E30 is built. *Recommend: yes.* One set of words for pools, pages and the rail.
8. **Record it as one decision** (D-61, "Account, App, Project, Session"). It amends D-35, D-36, SPEC §3 and §9.1, and the guide. The App and Account pages ship in **0.4** with E31's pages. The 0.3.x copy fixes in §9.1 fold into 0.4 unless a 0.3.x is already going out (one release, not many). *Recommend: yes.*

---

## 11. How each recommendation fits the whole

The maintainer, 2026-10-30: every decision serves user value, and every feature flows together. For each recommendation, this section gives:
- what the user gets;
- how it works with each capability, both ways;
- what it depends on;
- what changes for someone already using AI Juggler.

**The goal they all serve** (D-36, E21's five-second test): what needs you, what's open, where (which app, on which account), and one click to get there. The App answers "where". The Account answers "how much room, and what it costs". Project and Session are already right. Today the "where" and the "room" sit on one folder page. That's why every door that should lead to one of them lands on the folder.

### 11.1 The App page (and the App entity behind it)

**User value.** One click from any place an app is named (rail header, Home tile, session header, Accounts) to everything about that app:
- what's open and what needs you there;
- its sessions and projects;
- its usage;
- its settings: launch settings, window preview, tidy-up;
- how much room its account has left, and who shares it.

**How it fits, both ways:**

| Capability | App → it | It → App |
|---|---|---|
| Rail and navigation | The App grouping's headers become Apps; their name opens the page, their ↗ brings the app forward, as today. ⌘1–9 still go to places, which are now Apps (stable order, D-47) | The rail's Only `app:` filter means an App. The App page's "Show in the rail" is Settings › Desktop apps' switch, per App instead of per app type |
| Switching (Continue on another account) | The App page offers "Continue elsewhere" for its open sessions when its account is out of room | Switch targets name the App they'd start in ("Claude Code · personal-max"). "No new room" is decided by account, as today (`ui/src/lib/room-targets.ts:37-48`). A switch's new session joins the target App |
| Pools | An App can be a pool member ("every session of Claude Code · Work in these projects", `pools-model.md` §4.2's Profile member, renamed). Its page lists the pools it's in | A pool's canvas names each member by App and Account. A pool member type "Tool" covers "every Codex session" |
| Tasks | The App page's Sessions show each session's task (as rows do now) | "Start a session for this task" (`ui/src/pages/tasks/StartSession.tsx:42-90`, which lists "Main · bookauth-anela" today) picks an App, then its launch settings |
| Usage and room | Its Usage tab is the dashboard scoped to the App. Its room tile is its Account's, marked "shared with n Apps" | Usage's "who" bars split by App (`ui/src/components/usage/Dashboard.tsx:114-119`). Each bar opens its App page |
| Credits and fees | none: money is the Account's | — |
| Notifications | "Asks you" and "done, review it" notices open the session (as today); the App page's Overview lists what needs you there | A notice about an app (it crashed, or its window can't be reached) opens the App page |
| Processes | Its Processes tab is the App's processes while it runs, read-only for other apps' sessions (D-41) | A process row on the Processes screen says which App it belongs to, with a door |
| Fleet | A paired Mac's Apps are listed on that Mac's page | A remote launch picks an App on that Mac (`crates/juggler-app/src/features/fleet/mod.rs:62-72` carries a folder label today) |
| This Mac | — | This Mac's "running tools" (`crates/juggler-app/src/running_setups.rs`) become running Apps, with doors |

**Depends on:**
- the stored `apps` table (§9.3);
- each session's App (from its place while live; once ended, the Claude app's own session list or the CLI folder);
- E31's one list and one row (`relations-map.md` §5);
- the app icon at run time (D-38, exists).

**Changes for existing users.** Nothing is lost.
- The rail's headers keep their places but get clearer names ("Claude app · b-anela" instead of "Claude · bookauth-anela").
- CLI sessions move from a "Terminal · …" header to a "Claude Code · …" header.
- The old folder page's content moves into App and Account pages. A link to it opens the App page of that folder's CLI App.

### 11.2 The Account page (and the stored Account)

**User value:**
- "How much room do I have, what does it cost, what runs on it", for the thing you pay for, once.
- You can rename it, and set its renewal and fee once instead of per folder.

**How it fits, both ways:**

| Capability | Account → it | It → Account |
|---|---|---|
| Rail and navigation | The Account grouping's header opens it (today: the first folder). Accounts › an account's card opens it | Every runway, "Measure its limits" and limit notice leads here, not to a folder |
| Switching | "Continue elsewhere" on its Overview when it's out of room, for all its open sessions at once | Switch targets are grouped by Account. "No new room" already uses the account (`room-targets.ts:48`). A stored Account makes that hold even when a sign-in can't be read right now |
| Pools | An Account can be a pool member ("Personal stays read-only", `pools-model.md` §1.3 case 4). Its page lists those pools | Account membership resolves from stored rows, not a live sign-in read, so a signed-out folder doesn't silently drop out (`pools-model.md` §4.8) |
| Tasks | Its Sessions tab shows their tasks | — |
| Usage and room | It owns limits, the runway, the burn-downs and usage; the Usage screen's cards are Accounts already (`ui/src/components/usage/RunwayCards.tsx`) | Usage's runway card name opens it |
| Credits and fees | Plan, renewal and fee live here; one value, no "differs". An account through a service (D-34) shows its key and estimate here | Renewal notices (D-12) target the Account |
| Notifications | — | "Limit reached" and "renews in n days" open the Account page (`ui/src/lib/notice-actions.ts:58, 83` open a folder today) |
| Processes | — | — |
| Fleet | — | A paired Mac's sessions show their Account by name, which matches across Macs when the sign-in hash is the same (opt-in, the hash only) |

**Depends on:**
- the stored `subscriptions` table and its backfill (§9.3);
- the sign-in reads already used (`probe_identity`, metadata only);
- the vendor-keyed identity (G23).

**Changes for existing users:**
- Plan, renewal and fee entered on folders are copied to the account where they agree. Where they disagree, the account says so once and you pick.
- The account's name starts as its team, plan or masked email, not the folder's.
- Anyone who prefers the old name renames it once.

### 11.3 The corrected clicks (§4)

**User value.** Clicking a name leads to that thing. That's what the maintainer expected, and it's the five-second test's "one click".

**How it fits:**
- **One rule for every surface.** An app's name opens its App. An account's name opens its Account. A ↗ or Go to it brings the real app forward (D-37, unchanged).
- **The rail, Home, the session header, Usage, Accounts, notices and ⌘K all follow it**, so the same name always leads to the same page.
- **Breadcrumbs** read Accounts › account › App, so the lit tab (Accounts) matches where you are (`ui/src/lib/nav.ts:43`).

**Depends on:** the two pages. Until they exist, only the words that promise the wrong page change (§9.1).

**Changes for existing users:** muscle memory. A header click that opened the folder page opens the App page, which holds the same sessions, now only that App's.

### 11.4 The words (§9.1)

**User value.** One name per thing. An account and an app no longer share the name "bookauth-anela", so you can tell them apart on every screen.

**How it fits:**
- The words are the same in the rail's Only filters, ⌘K, menus and pools' member types (decision 7).
- Switch sheets and New session use the same words: "Which app?", then its account's room beside it.
- Notices, the guide and the Privacy page use them too.
- The terminology lint (D-35's consequence) learns the new meanings, so no screen drifts back.

**Depends on:** decision 1 (the word "App"), and the D-61 record.

**Changes for existing users:**
- "Profile" disappears as a noun on screen. It stays only inside an App's name.
- "Account" stops naming folders.
- Nothing about their data changes.

### 11.5 The data model (§9.3)

**User value.** What makes the pages possible:
- Apps you don't have open are still listed.
- An account keeps its name and fees.
- An app's history stays its own after its window closes.

**How it fits:**
- **Additive.** Today's folder rows, launch settings, sessions, switch graph (`epochs.account`), pools and tasks keep their keys.
- **New references sit beside them.** `instances.app_id`, `accounts.subscription_id`, and pool members `app:` / `account:` as `pools-model.md` §4.7 already plans.
- **Fleet** sends App and Account names in its views (opt-in, as today).
- **The D-58 cache** holds the relation counts (`relations-map.md` §6) and is invalidated when an App or Account changes.

**Depends on:**
- one schema version (v4x);
- the backup before migration (as `before-v32` and `before-v37`);
- E30 and E31 using the same tables.

**Changes for existing users:**
- A one-time migration with a backup. A downgrade ignores the new tables.
- No data is deleted.
- On the real setup it makes 7 CLI Apps, up to 6 Claude app Apps (as they're seen, or found per decision 5), one ChatGPT App, and one Account per distinct sign-in.
