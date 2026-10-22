# Changelog

All notable changes to AI Juggler will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- Opening AI Juggler while it is already open brings the open one
  forward without touching its data first (it used to open the data,
  run its upgrades and recovery, and only then hand over).
- Updates (beta): a failed update says why in plain words — no
  connection, a download that didn't pass the signature check, one signed
  for another version, the backup — with the technical reason behind
  Details and **Try again**; the running version is never touched.
  **Restart to update** (Settings and a prompt on Home) appears only when
  no session is running, no switch is in flight and no bring-in is under
  way, and Settings says what it is waiting for; nothing restarts by
  itself. Every update prompt carries the channel's label (**Beta**).
  Review on the update notice opens Settings on the update itself.
- Going back to an earlier version works: before AI Juggler upgrades its
  database it keeps a copy of it (the two newest are kept), and Settings ›
  About & updates › **Go back to an earlier version** puts that copy back
  — after saying what changed since won't be in it — keeps today's data
  beside it, and quits so the earlier version can be installed. An older
  AI Juggler that opens newer data now says so in a window (on every
  desktop platform) and offers the copy or the pre-update backup, instead
  of quitting at launch without a word; it waits while another AI Juggler
  has the data open. A release whose database changes can't ship without a downgrade
  note in its release notes.
- The session, account, project and tool views say what they show in
  plain words: no task codes ("lands with AJ-E606+"), no "No data"
  placeholders, tools by name ("Claude Code") and states in the rail's
  words. Actions that don't work yet aren't shown; ones that don't apply
  right now stay greyed with the reason. The tool chip in a session's or
  an account's header opens the tool's own view: what's installed and its
  accounts' quota. An account's history says how long ago each session
  started (it said "now ago"), in days once it's more than two. A quota
  with no reading says "Not measured yet" everywhere, as Accounts does.
- Plain words everywhere: a quota reads "measured", "worked out" or
  "estimate" (never "MEAS"/"COMP"/"EST"), token counts say "tokens", and
  switching says what carries ("Full story, fresh start", "Same
  conversation, new account", "Switched back") instead of "F1",
  "transplant" or "switch-back ↺". A switch is "proven" or "not proven
  yet", and Accounts › "Which switches between accounts are proven"
  explains each one with "automatic tests" and "tried by hand". Sessions
  still running after a restart are "picked up after restart" (not
  "re-adopted"). The launch settings each account starts with are
  "Default" everywhere, also in a project's actions and New session.
  New session's footer speaks only when a folder is picked. A lint keeps
  developer words off the screen.
- Home and Accounts never recommend an account that's low for new work.
  When only a low one is measured, they say AI Juggler can't tell which
  account has room (Home used to say both "start new work on another
  account" and "Best for new work" for the same account at 6% left).
- On a large setup, sessions running outside AI Juggler show as running
  within seconds of the first launch: the newest files are read first,
  and what's live is worked out as the first pass goes, not only at its
  end (it took minutes).
- Opening AI Juggler right after it was force-quit (or crashed) starts
  it, instead of saying another one "is already open and isn't
  answering" when none was. When one really is open and stuck, the
  message offers **Try again**, and **Quit** quits. On macOS the message
  is AI Juggler's own window, so it never stays on screen after AI
  Juggler is gone.
- At a start, the conversation list and the projects no longer wait on
  the usage history being written: it's written in small steps, and the
  first read of your conversations runs once, not three times at once.
  Large conversation files are listed from their start and end first,
  so your conversations and running sessions show within seconds even
  with gigabytes of history; their token counts follow.
- Home never says "All clear" while an account has run out; it says so.
- History lists every session that has ended, including the ones you
  ran outside AI Juggler, each by what it was about (its title or first
  question), with its tool, project and account, and opens its page.
  Replay, Archive and Delete are in the row's ⋯ menu (and on
  right-click). Sessions everywhere are named by what they were about
  when you haven't named them.
- **Claude: measure limits for sessions started elsewhere (opt-in).** On
  an account used where it is, AI Juggler can add its capture to the
  account's status line, so sessions you start in the Claude app or a
  terminal report their limits too. It's off until you turn it on, after
  you see the exact one-line change. Your own status line keeps showing,
  nothing else in the file changes, and turning it off puts the file
  back as it was.
- A finished session's row no longer shows its account's "% left";
  only a running one does, once the account runs low.
- This Mac no longer lists a session as "under 1 MB": that was a session
  whose process had already exited, so nothing was measured. Such a
  session isn't listed, and its view shows its memory as unknown.
- New session opens without waiting: its launch settings and ranking
  load off the app's main thread, and it reuses the sessions the app
  already has. Its account lines say what Home says (an account with no
  reading of its own shows its latest session's), and Customise keeps
  keyboard focus, so Esc still closes it.
- On Home, J/K keep working after you mark a session reviewed or resume
  it, and A works from a row's own Mark reviewed button. Handed-over
  sessions no longer show under "Pick up where you left off". Home's
  header has the legend "?" too. "Find my accounts" on Home and Usage
  asks which tool when more than one is installed, like Accounts does.
  The tool choice keeps keyboard focus where you put it.
- Settings › Cleanup has the "Pause automatic cleanup" switch too. In
  Cleanup, keyboard focus stays on the ⋯ button after you pause, resume
  or press Escape. In a narrow window, ⌘B and the density shortcut only
  open or close the list for now; they never change your setting.
- Adding an account is short: name it and sign in, or pick a folder,
  see what's in it (size, sign-in, a too-large history offers settings
  only) and bring it in as a copy. First run, Accounts › Add account and
  New project's "Add an account" all use these steps; the old setup
  wizard is gone. Add account asks which tool only when more than one is
  installed, so the separate "Add Provider" button is gone too.
- Bringing in your Claude setups now **uses them where they are** by
  default: your Claude app and AI Juggler share one sign-in, one history
  and one set of settings, and AI Juggler doesn't change your files.
  Signed-in setups stay signed in. **Make separate copies** is right
  next to it if you want an isolated sandbox. Each account in Accounts
  says which it is.
- When some setups come in and some don't, you choose: **Continue** with
  the ones that came in, **Try again** for the others, or — for a copy
  that failed — **Use it where it is instead**. Close or Esc while
  bringing them in means **Stop**: the one in progress finishes and the
  rest wait.
- An account you brought in as a copy can switch to the folder it came
  from: **Use the original instead** on its card in Accounts. It then
  shares that folder's sign-in and history; the copy stays on disk.
- Cleanup says what it is in one line instead of "Tier 0 observe ·
  verified manual kills". Pause automatic cleanup and Cleanup settings
  sit behind ⋯, and Resume is on the "paused" banner. The machine filter
  only appears once another Mac is paired. Groups name the tool ("Claude
  Code"), and alerts point to "Cleanup".
- The app says **tool** where it said "Provider": History's search,
  Settings, pack secrets, the rail's filters, the session details and the
  backup's "not included" list.
- In a window narrower than 900 px, the rail folds to its icon column by
  itself and comes back when the window is wider. Your density setting
  doesn't change; ⌘B opens the list while the window is narrow.
- Home is also the morning triage. Sessions parked until a reset have
  their own section, with Resume once the reset is reached (they no
  longer read as "hit its limit"). "Pick up where you left off" lists
  what ended this week that you haven't marked reviewed. J/K or the
  arrows move between rows, Enter opens, A runs the row's action. Triage
  and Overview, which repeated Home, are gone from More.
- A calmer app, rebuilt around what you need first (decision D-25). The
  top bar is Home, Accounts, History and Settings; Usage, Fleet and
  Cleanup are under More, and Cleanup and Fleet show up by
  themselves when they have something for you. The rail is one list of
  your projects and their sessions: search, Needs you · Running · All,
  and the rest behind a Filters button that says when one is on. A row
  shows its state in words and adds a note only when it matters (low
  quota, parked, uncommitted files, a sign-in to check, a stale value).
- The app says **session** for what it used to call an Instance — in
  every screen, message and the guide (D-25). A session's conversation
  record is a "conversation". The code keeps its names.
- Home answers "what needs me?": sessions that need you, what's running,
  each account's runway, and where you left off.
- The session view opens on where the session runs (its branch or
  folder), its state, project, account and how long; Switch account… sits
  beside Stop, and the ranked list of accounts is one click (or ⌘⇧H)
  away instead of always open. Everything else is under Details. Announcements say what happened ("waiting for your reply"),
  not rule codes.
- Accounts shows each account's runway in words — the 5-hour window
  and the week, when each resets, and when this pace would run out — plus
  what runs on it and the best account for new work. Usage leads with the
  same runway and explains itself when nothing is set up yet.
- Add account finds what's already on this Mac and isn't in AI Juggler
  yet, the same way first run does, and goes straight to a fresh account
  when there's nothing new.
- When an account hits its limit, one decision: continue now on the best
  other account, park the session until the reset (it's offered back;
  nothing restarts on its own), or choose another.
- List density moved to Settings › Appearance, with each choice
  described.
- First run, when there are no accounts yet: what's already on this Mac
  (Claude Code in the terminal and the Claude desktop app's profiles),
  brought in as copies with one button; a sign-in list with "Sign in to
  all"; then the computer access this OS needs (macOS: Accessibility and
  Automation, Screen Recording), each with why, its live status and Allow.

- The project moved to its public home,
  [figurehead-ai/AI-juggler](https://github.com/figurehead-ai/AI-juggler),
  as a clean snapshot. Issue and PR numbers mentioned in earlier docs
  (`#8`, `#37`, …) refer to the pre-public development repository.
- Open-source setup: rewritten README, CONTRIBUTING (real dev setup and
  gates), SECURITY (private reporting, scope), new SUPPORT and
  GOVERNANCE, Dependabot, privacy reminders in the issue and PR templates.
- Dependencies brought current before launch: React 19, Vite 8 (Rolldown),
  TypeScript 7, Playwright 1.64.1, axe-core 4.14; in Rust, ed25519-dalek and
  x25519-dalek 3, chacha20poly1305 0.11, getrandom 0.4 and zstd 0.14 (keys,
  signatures and backup containers are byte-compatible: the known-answer
  tests pass unchanged).

### Fixed

- The whole window no longer slides up when a button low on Home takes
  focus: screen-reader words hung below the window, so the app itself
  could scroll.
- On Windows and Linux, Ctrl+Shift shortcuts (Ctrl+Shift+H, the switch
  and the others) work while you type in a session's terminal, as ⌘⇧ ones
  do on a Mac. Ctrl+Shift+C and Ctrl+Shift+V stay the terminal's copy and
  paste.
- This Mac counts what a session starts as soon as it starts (within about
  4 seconds). A build fanning out under a Claude app session could be
  missed for up to 10 seconds, so the app's figure read far too low.
- This Mac never drops a tool without a word: past the three heaviest, the
  rest are behind "and N more". Search helpers a finished Claude Code
  session left running are no longer counted as Claude in a terminal.
- Codex's context compactions now count, once: the usage of the compaction
  call, which Codex records outside its turns, was missed, and could count
  twice when AI Juggler read the file just as the compaction was written.
- A conversation resumed into a new session repeats the original's
  history. Its cost now stays with the original session, whatever order AI
  Juggler reads the files in, including a copy of a copy and a fork that
  both go on. In a few cases the copy is still credited with the shared
  part: an original that writes first after a fork, a fork resumed again
  before it wrote anything, and a resume that never wrote. An account's
  total is right either way. Counts already stored are read again once, so
  these fixes reach them.
- The tools AI Juggler runs are found in fixed system places, never by
  searching your `PATH` (git included).

- An account's limits no longer read "100% left" when nothing has been
  measured yet: on Home, Accounts, Usage, a session's details and New
  session, a window with no reading says "Not measured yet". It also
  shows the tokens used in that window, and an empty bar instead of a full
  one. A reset time shows only when the tool itself reported it; before,
  one was worked out from your own use, and under steady use it read
  "now".
- **Codex**: AI Juggler now shows Codex's limits and its credit balance,
  as measured. Codex records them in its own session files, so "Codex
  doesn't report its limits" is gone. It shows the windows Codex reports
  (on many plans only the weekly one), never a 5-hour window Codex
  doesn't have.
- A measured limit says when it was read ("as of 12 min ago", then a
  time, then a date). It no longer jumps back when a session resumed
  days later reports its old week, or when two sessions report a point
  apart. Once a window resets with no newer reading, it says "Not
  measured since the reset" instead of 0% or the old figure. Credits say
  who reported them and when.
- Usage no longer turns the whole window blank. A cost with no value
  yet (no fee set, a model with no price, Codex's credits) crashed it;
  each now reads as missing, and a model with no price says "No price
  for <model> yet", never $0.
- A screen that hits a problem shows "This screen hit a problem" with
  **Try again** and **Copy details**, while the sidebar and the header
  keep working. Before, one screen's error blanked the whole window. The
  error goes to the crash report only if crash reports are on.
- Token counts are right. Claude's were counted two or three times: one
  reply is written as several lines, one per part, and each line was
  counted. Codex's were counted twice, because its input includes its
  cached input and its output its reasoning. Counts already stored are
  read again once from your history files, so the totals correct
  themselves.
- Usage puts a price on the models you use. It knows the current Claude
  models and the GPT-5 models Codex runs on, from Anthropic's and
  OpenAI's own price pages. Codex rows now carry their model too.
- Home under memory pressure: the warning banner no longer covers Home's
  headline, and Home shows the warning once, under Needs you. The figure
  is the same everywhere: the whole Claude app across its windows ("about
  16 GB across 3 windows"), not one window. While the warning is on,
  This Mac says "Under memory pressure" too, and so does the sidebar's
  footer, which used to say "healthy" whatever the Mac's state. The
  collapsed sidebar says "All clear" only while the Mac has room.
- A tool that doesn't report its limits says so on every screen. Before,
  Usage could show it at "100% left" while Home said it doesn't report.
- Suggested accounts and switch targets show the account's name, not
  its internal id, and say "Not enough use to forecast" instead of
  "exhausts ≈0 min". New session's account list says one thing per
  account, in the same words as Home.
- A screen's "Copy details" says "Copied", or shows the details to
  select when the copy fails.
- First run's "See a live preview of each session" (Screen Recording) no
  longer stays stuck after you allow it in System Settings. macOS only
  shows the new answer once AI Juggler reopens, so the card now says so
  and offers **Reopen AI Juggler**, telling you first if sessions are
  running. After the reopen, first run comes back to the same step. The
  page also checks again as soon as you return from System Settings, and
  it says "Allowed" only once macOS does.
- Opened from Finder, AI Juggler now finds tools installed with npm (like
  Codex) and can start them: it uses the same PATH as your Terminal, so
  `node` is found. Before, Add account only offered Claude, and a Codex
  session or sign-in couldn't start. A tool it finds but whose version it
  can't read is still offered, marked "Found; couldn't check its
  version" — never "not installed".
- A switch that replays a conversation always carries at least its
  newest turn (cut to fit when it alone is too long) and refuses — before
  starting anything — when there is no conversation to carry; each
  replayed turn is quoted, so text inside a turn can't pose as another
  turn. A session copied in from another Mac can only land as a plain
  session file in its account, and a copy never overwrites a file.
- The session list no longer shows every project from your Claude
  history as a bare row with a "0". Projects you haven't run anything in
  yet sit under a short **Projects** heading. Each reads like a session:
  a folder mark, the name, and when you last worked there, how many
  conversations, and with which accounts. Worktrees count as their
  repository, clones of one repository are grouped, and every group
  folds (and stays folded).

- Switching a session you started in the app no longer stops at
  "Couldn't tell which file is this session": AI Juggler finds the
  session's own file (same folder, started after the launch) when you
  switch and remembers it — for Claude Code and Codex alike. It never
  guesses: when two sessions of one account share a folder, or two files
  could be the one, it still asks.

- A message to Codex that starts with "-" (a list item, say) is passed as
  text, never read as an option; a session id that looks like an option
  is refused instead of resuming something else (Codex and Claude Code).
  The Expert-setup line for a Codex account now pins its SQLite folder
  too, and First run finds a `CODEX_HOME=` line in your shell files.
- Measured quota from Claude's status line works on a real Mac. The
  status line command pointed at a path under "Application Support"
  without quotes, so the shell split it at the space and the capture
  never ran. Existing installs are repaired the next time AI Juggler
  checks them, and your own status line stays chained.
- Started from a terminal that exports `ANTHROPIC_API_KEY` (or an auth
  token), AI Juggler ran every account on that one key. Each account now
  signs in as itself; a key set in an account's own settings still works.
- A launch setting that picks a model no longer rewrites the account's
  `settings.json` (it replaced the whole file, losing hooks and
  permissions); the model goes in as Claude's own `--model` option.
- Bringing in a Claude setup that has a linked folder (for example a
  skill under `skills/` that is a link to somewhere else) no longer fails
  with "cannot copy skills/…". The folder is kept as a link that points
  where it did — AI Juggler never follows it or copies what it points to
  — and the import says which links it kept. Linked files are still
  copied, so the copy never shares a file with your original. A broken
  link, or one this system won't let AI Juggler make, is left out with a
  note instead of failing the import.
- Everything you can click shows the pointing hand: buttons, rows,
  menus, filters, checkboxes and links all showed the arrow.
- The bring-in screen can always be closed: **Not now** at first run,
  **Close** from Accounts › Add account, or Esc.
- Security: a switch arriving from a paired Mac can no longer write files
  outside the target account's folder or replace a file it already has —
  names and session ids in a switch bundle are now checked before
  anything is written. (Only possible with Fleet pairing, which is beta.)

- The "?" at the bottom of the session list now opens the guide to what
  the icons and words mean (it opened out of sight), in the list's own
  words.
- First run says where each Claude setup is open right now (the Claude
  app or a terminal) and calls the default ~/.claude "Default"; it no
  longer guesses "Terminal" for a folder the desktop app uses.
- An account that needs its sign-in shows on Home under Needs you, with
  Sign in right there, instead of replacing Home with a separate screen.

- Session rows are readable at the WCAG contrast minimum in both themes:
  their second line and time use the stronger grey, and ended sessions
  are shown by a quieter title instead of being faded.

- Release builds ride out a short outage at Apple's signing services: a
  missing timestamp or an unanswered notary or ticket request is retried
  (up to four tries); any other signing failure still stops the build.

- A session's resource history stopped updating after five minutes, so
  its sparkline froze and the "this uses a lot" check looked at old
  numbers. It now keeps the latest five minutes.
- When a session ended, a helper it left running could drop out of the
  snapshot that protects heavy helpers from automatic cleanup. The last
  live view is now kept until the session is fully finished.
- Project names in the top bar show as written ("ai-juggler", not
  "Ai-Juggler").

- The More menu opened underneath the page, so only its first item
  could be clicked; it now paints above everything and closes on Escape
  or a click elsewhere.

- Screen readers and voice control: buttons are now named by the words they
  show (WCAG 2.5.3). Instance rows, filter chips, project tiles, remote rows
  and switch targets read their visible text first; the rest of a row's
  details is its description, so nothing is lost.

### Added

- **Rules and MCP servers the same across tools** (AJ-E1507), in one
  reviewed click. The project's header says **Rules in sync** or
  **1 rules file differs** (a missing file counts only for a tool that
  works there), and opens one sheet, also in every project's menu
  (**Compare rules and servers…**). Pick the master copy, see the diff
  applying it would make, and apply it to one file; a tool that can read
  another file (Claude Code's `@AGENTS.md`) can read the master instead
  of a copy. A link, or a file that changed since you looked, is never
  written. Each account's MCP servers are listed too, and one can be
  copied to another account in that tool's own form; secrets stay
  behind, named. Every change can be undone there or in Settings.

- **A This Mac page** (AJ-E1528). "See what's using it", the This Mac
  panel's "See all", More › This Mac and the rail's "This Mac" line open
  it. It shows the Mac's state in words and meters, then what's using it:
  each tool with its sessions or windows (the Claude app's marked "not
  started by AI Juggler", with nothing to click), what's working the Mac
  hard with links, recent alerts and advice, and anything left behind.
  Home's panel stays a summary.
- **Make it smoother** (AJ-E1527). The welcome lists the optional
  permissions (today, jumping to a session's own window), each with its
  state and its Allow or Open System Settings, in the same rows as first
  run's. They're never required: "Not now" puts them away. Afterwards,
  while one is still off, Home shows a small reminder row, never a
  modal, and its own "Not now".
- **Telling projects and sessions apart** (AJ-E1532). Every project has
  a mark, the same on the rail, Home, History and its page: its own icon
  when its folder has one (a Tauri, Electron or Xcode app icon, a
  favicon or a logo, read and never changed), else two letters on one of
  twelve colours that stay apart in both themes. Clones of one
  repository share their mark and carry their number. **Change its
  mark…** in a project's menu picks the colour, letters or an emoji.
  Every account now has a round avatar in its colour, with initials that
  tell accounts sharing a first word apart ("Wo" and "Pe" for
  claude-work and claude-personal). A session row shows its tool and the
  account serving it, and says "on Work Max" to a screen reader. A
  project's row shows the accounts that worked there, a ring on any
  running there now. A folded project shows as a stripe in its colour.
- **A welcome for new installs** (AJ-E1524). With no accounts yet, Home
  now explains what AI Juggler is for and what it found on this Mac
  ("We found Claude Code and Codex on this Mac", or install links when
  nothing is there). It shows four things it does for you and a trust
  line: it never stores your passwords or keys, and never sits between
  you and the tool. Two ways in, **Find my accounts** and **Add a fresh
  account**, and a short "How it works". Usage and Fleet point back to
  it while no account is in.
- **One kind of page for everything** (AJ-E1533). A session, a project,
  an account and a tool now open the same kind of page. It says what the
  item is and what it belongs to, shows its tabs (only those with
  something in them), and keeps its actions at the top.
  - A project's page lists every session that worked there, yours from
    your own terminal too, live first. It shows its branch and what
    isn't committed, and what each account spent there.
  - A session's numbers agree from its header to every tab.
  - Open an account from its name on Accounts, and a tool from More ›
    Tools.
- **Always know where you are** (AJ-E1534). The open item is selected
  in the list (and scrolled to), whatever it is, and the top bar follows
  the page you're on. Each kind of item has its own mark. Folders show
  as `~/…`, never the full path.
- **Sessions started outside AI Juggler** (AJ-E1529). Every account's
  conversations now show, the ones you started in your own terminal or
  in the Claude app too: under their project in the list (live ones first, with a
  quiet "outside AI Juggler" mark), on Home, in each account's running
  count and in the go-to palette. Open one to see where it ran and on
  which account, then continue it in AI Juggler or copy the command that
  resumes it. Removed and pruned worktrees fold into their repository
  instead of showing as moved projects, and Codex conversations count
  too, once each, however often they were resumed. Each session in the
  list ends in its account's initials, in that account's own colour,
  the same on every screen. AI Juggler never stops or changes what it
  didn't start.
- **Right-click menus** (AJ-E1537). Right-click a session, a project, an
  account, a history entry or a line on This Mac for its actions, in the
  Mac's own menu. The same menu opens from the row's ⋯ button, a page's
  header and Shift+F10. A greyed-out item says why, and Stop, Forget,
  Remove and Delete sit apart at the end. New: copy a session's resume
  command, start a new session in a project, copy a project's path.
- **Name, pin and tidy sessions and projects** (AJ-E1502). Rename a
  session or a project (the folder keeps its name), pin it to the top,
  open a project in your editor, point a moved project at its new
  folder, or forget one you're done with until you use it again.
- **Tool marks** (AJ-E1526). Wherever a tool is named (Add account, first
  run, account cards, the rail, the inspector, Usage and This Mac), a
  small mark sits beside its name. Claude Code and Codex show a neutral
  letter mark for now, because Anthropic and OpenAI ask for approval
  before their logos are used. Other companies' names and logos belong to
  their owners (see TRADEMARKS.md).
- Tools are described by their adapters (AJ-E1525, ADR-0019). Every
  screen reads a tool's name, sign-in words, install page and first run's
  groups from one catalog. A new tool needs no change to the interface,
  and a check keeps tool names out of shared code. Nothing you see
  changes, apart from one wording: Home's sign-in line now says "Claude
  Code's own sign-in".
- **Settings › Sessions** (AJ-E1505). **Stuck after** sets how long a
  session may go without output (with its CPU near idle) before it's
  marked Looks stuck. Raise it if your builds run quietly. It applies to
  running sessions at once, no restart. Under **Terminal**: the font size
  and scrollback, applied to open terminals at once. Each has a one-line
  reason and **Reset**.
- **Keys** (AJ-E1504). Press **?** to see every shortcut, by area.
  **⌘1–4** open Home, Accounts, Usage and Fleet. **⌘⇧I** folds the
  inspector to its header line (the open tab keeps its room), and the
  window remembers it. The inspector's tabs moved to **⌥⌘1–6**. While
  you type in a field or the terminal, plain keys and Ctrl-combos are
  yours: Ctrl+K, Ctrl+B, Ctrl+N and Ctrl+F now reach the shell instead of
  the app. ⌘ shortcuts still work there.
- **One way every action answers** (AJ-E1503). A toast at the bottom of
  the window says what happened, with **Undo** where it can be undone
  (or ⌘Z outside a text field). Stopping one session is held for 5
  seconds: Undo in time and the stop is never sent. Mark reviewed,
  snooze, mute, archive and unarchive can be undone; the switch's undo
  window is the same toast. Toasts are read out by screen readers and
  never take focus.
- Quitting (⌘Q or the window's close button) while sessions are live
  says how many are running and what quitting does to them: a terminal
  session ends with AI Juggler, and a desktop session keeps running. Pick
  **Quit anyway**, **Stop them first** or Cancel. With nothing live,
  AI Juggler quits at once.
- While the rail, Home and Accounts load they show rows shaped like their
  own, rather than "No sessions yet" or "Bring in your accounts" for a
  moment.
- **Codex** is the second Provider (AJ-E1301). AI Juggler finds the Codex
  CLI, gives each Codex account its own folder (`CODEX_HOME`), signs it
  in with OpenAI's own `codex login`, and starts, resumes or runs Codex
  in it. Sessions run without Codex's background server, so stopping a
  session stops all of it. Verified against Codex CLI 0.163.0. Watching
  sessions and switching follow (E13).
- First run finds your existing Codex setups (`~/.codex`, `$CODEX_HOME`,
  any `~/.codex-…` folder, `CODEX_HOME=` in your shell files) and brings
  one in as a copy — the original keeps working, untouched (AJ-E1302).
  Your sign-in stays with the original (a Codex sign-in can't be shared
  between two folders without signing one out), so each copy signs in
  once. Codex's own databases stay behind (Codex rebuilds them from the
  copied history); the light import leaves the history behind too and
  lists what it skipped.
- Codex sessions read like Claude ones (AJ-E1303): the conversation,
  the model each turn ran, tokens, compaction and errors come from
  Codex's own session files, in both of its history formats. The
  account's identity check uses Codex's account id (never a token).
- Switching with Codex, in beta (AJ-E1304). Between two Codex accounts
  the session's file is copied into the other account and resumed
  there — the original is never changed, and a session that already
  exists there gets a new id instead of overwriting. Between Claude Code
  and Codex the conversation is replayed into a new session on the other
  side (with secrets handled the way you chose — a replay waits until
  you have chosen). Every Codex switch is labelled Beta: none has been
  checked on real accounts yet.
- Codex setups can be used where they are, like Claude Code ones
  (AJ-E1306, ADR-0018). Bringing in `~/.codex` (or another Codex home)
  no longer has to make a copy: AI Juggler runs your own home, so it
  stays signed in and Codex outside AI Juggler sees the same sessions.
  Nothing of AI Juggler's is written into it. `~/.codex` itself runs with
  `CODEX_HOME` unset, as a plain `codex` does. A Codex copy can switch to
  its original ("Use the original instead"), and a failed Codex copy can
  come in where it is instead. Copies stay available for anyone who
  wants a sandbox.
- AI Juggler now watches the Mac itself, not only its sessions (E14,
  AJ-E1401): processor and memory use, macOS's own memory-pressure level,
  swap, free space on the disks your sessions use, power and thermal
  state, sampled every 2 seconds with five minutes of history. One plain
  summary — "Plenty of room", "Getting tight" or "Under pressure", with
  the reasons — is what Home, the launch advisor and the alerts will show.
  Old swap doesn't count against you; swap that keeps growing does. Memory
  in use counts app memory, wired and compressed data, close to the
  figure Activity Monitor shows.
- AI Juggler now also measures the Claude tools it didn't start (E14,
  AJ-E1402/E1403): each Claude desktop app window and each `claude` in a
  terminal, over all the processes it runs, matched to your account by the
  folder it uses. They count in "what your AI tools use" and are always
  labelled as not started by AI Juggler. They are never offered stop or
  cleanup. A session is called heavy when it stays above 90 % of a core
  or 2 GB for a minute.
- Home has a **This Mac** panel beside Runway (E14, AJ-E1404). It says how
  your Mac is doing ("Plenty of room", "Getting tight", "Under pressure",
  and why), how much memory and CPU your AI tools use right now, the
  heaviest three, and how many processes were left running after their
  session closed, with a Clean up button. A session working your Mac hard,
  or a Mac under pressure, shows up under Needs you with one action.
- A rail row now notes a session's memory or CPU only while that session
  is heavy. The session view shows "CPU 12% · 1.1 GB · 3 helpers" under
  its title instead of keeping it under Details (AJ-E1405).
- New session now answers "Has this Mac got room?" beside the account's
  own verdict (E14, AJ-E1406/E1407): OK, Tight or Not now, with the
  reasons and what would make room ("Closing feat/read-model frees about
  3.2 GB"). It learns from your own sessions how heavy each launch setting
  is (the median peak of sessions that ran at least a minute and didn't
  fail, after three; until then it says "not measured yet", with a Tight
  or Not now verdict too). A deleted launch setting's or Account's
  measurements go with it. It never stops you starting a session.
  **Downgrade note:** this adds database schema v29. An older build
  refuses a v29 database (`SchemaTooNew`) rather than risk it, so going
  back to a build before this one needs the database backup taken before
  the upgrade.
- AI Juggler tells you when your Mac is struggling (E14, AJ-E1408): memory
  pressure that lasts, swap that keeps growing, a processor flat out for
  minutes, a hot Mac, a disk running out of space, or a session working
  it hard. Each is said once (at most one of a kind every 30 minutes),
  with what's using the most and what of it is high, and stays under
  Needs you while it lasts. A Mac hovering near a line doesn't flap: an
  alert, and a heavy session, clear only after 45 s back under it. A
  notice goes once its condition has ended. It never stops anything by
  itself.
- This Mac gives advice from your own sessions (AJ-E1409): how many
  sessions it comfortably runs at once, and which launch setting is the
  heaviest. It shows only once there are enough sessions to go on.
- Three update channels (decision D-20): **Nightly** (built from the latest
  code every day), **Beta** and **Stable** (each promoted with the
  maintainer's approval). Choose Stable or Beta in Settings › About &
  updates; moving to a steadier channel never installs an older version.
- Settings, organised as in the approved design: a section list beside
  every policy you own — Context packs & secrets, Cleanup (with a default
  for new Accounts, off unless you choose it), Accounts & renewal dates
  (with how early to remind you), Notifications (quiet hours for the
  hourly digest and which Accounts it includes), Fleet permissions, and
  Data.
- Editions that install side by side (decision D-23): **AI Juggler** for
  everyday work, **AI Juggler Nightly** for trying the newest build, and
  **AI Juggler Dev** for development (`just dev`). Each has its own data,
  Keychain items, Fleet port and icon, so none can change another's data.
- Derived workflows for the beta (E10):
  - an identity guard so an Instance never runs on the wrong account;
  - git worktree isolation for a second Instance in a project;
  - parking a quota-blocked Instance until its reset;
  - a morning summary from a timestamped status journal;
  - a Triage view;
  - History: transcript search, archive, read-only replay, and per-Account
    retention with a dry run;
  - backup and restore of AI Juggler's own data (ADR-0016);
  - announced, verified, user-initiated updates for release builds, with
    release and rollback workflows (docs/RELEASING.md).
- Quality-signals engine (AJ-E409): per-Instance tokens/min, error rate,
  context report, session cost from a versioned in-repo pricing table
  (`crates/juggler-core/src/quality/pricing.v1.json`, **pricing table v1**
  — captured 2026-10-13 from anthropic.com/pricing; future row changes bump
  the version and land here).
- Repository bootstrap: license (MIT), contribution guide, code of conduct,
  security policy, issue/PR templates, placeholder CI.
- Deep-dive research notes for Claude Code, Codex, ZCode, and the tooling
  landscape (`docs/research/`).
- Product proposal (`docs/PROPOSAL.md`) and technical specification
  (`docs/SPEC.md`) — published for review.
- CONTRIBUTING guidance: only final artifacts belong in the repo (no
  conversation logs or exploration transcripts).
- Machine health subsystem (spec §5): system + per-session resource
  monitoring, overload alerts, launch admission advisor, crash recovery,
  and optimization recommendations.
- Fleet / multi-machine architecture (spec §11): mDNS discovery, code
  pairing, gRPC over mutual TLS, telemetry/control sync with
  credentials-never-leave-the-machine invariant; fleet milestones M6/M7
  toward 1.0.
- Release & platform policy: macOS-first signed + notarized builds
  (Apple Developer via Vuxtra), strict semver with 0.x beta channel,
  stable channel at 1.0.
- Execution plan (docs/plan/EPICS.md): epics → tasks → acceptance
  criteria for M0 through 1.0, per-milestone validation gates with
  committed gate reports and user confirmation.
- Quality standards (docs/plan/QUALITY.md): engineering standards and
  the per-task Definition of Done applied to every requirement.
- Architecture decision records (docs/adr/): Tauri 2, adapter
  architecture, automation policy, fleet transport, MIT license.
- Autonomous build runbook (docs/plan/RUNBOOK.md): the operating
  contract for unattended build runs — PR flow, DoD, gate stops, hard
  rails.
- First run and Add accounts list the Codex setups found on this Mac in
  their own **Codex** group, beside the Claude ones, and say when the
  Codex CLI still needs installing (AJ-E1301). Machines with only Claude
  setups see the screen exactly as before. Codex setups always come in
  as copies: the "use them where they are" choice shows only when a
  Claude Code setup is on the list, and a Codex-only first run says
  copies throughout. Sign-in steps (First run, Home and Accounts) name a
  Codex account's sign-in the same way everywhere: "Codex's own sign-in
  (OpenAI)".

### Added (M0-1)

- Cargo workspace with the seven `juggler-*` crates; workspace lint
  baseline (clippy pedantic subset, `unwrap/expect/panic/todo` warnings,
  `missing_docs` deny, `unsafe_code` forbid); pinned toolchain
  (1.99.0, MSRV 1.99); cargo-deny policy; `just` task runner
  (`just verify` = the CI gate, locally).

### Added (M0-2)

- CI pipeline: `verify` job (fmt, clippy `-D warnings`, tests, docs with
  `-D warnings`, cargo-deny, llvm-cov coverage artifact) plus a
  cross-platform build matrix (macOS arm64 + x86_64 cross-check, Linux,
  Windows); required status checks enforced on `main`.

### Added (M0-3)

- Domain model in `juggler-core`: typed identifiers, `Account`, `Profile`,
  `SessionRecord`, session statuses (incl. `QuotaBlocked`), usage snapshots
  with confidence levels, quota windows, transcript refs; typed
  `CoreError`.
- SQLite storage: WAL + foreign keys, forward-only versioned migrations
  (newer-schema databases rejected, corrupt files degrade to
  `NotADatabase`), repositories for accounts/profiles/sessions with
  cascade deletes.
- Crash-safe `atomic_write` helper.
- Property-based round-trip tests (proptest) for every entity plus
  migration, corruption, cascade, and atomic-write edge cases.

### Added (M0-4)

- `juggler-adapter`: the `HarnessAdapter` contract — capability flags,
  discovery reports with `verified_against` version pinning, launch
  specs (interactive/headless, account-aware), the read-only
  `SessionWatcher` event interface, and an id-keyed adapter registry.
- `juggler-adapters`: the scriptable `fake-harness` binary (JSONL
  transcripts, turns/stall/burn/exit fixtures, resume) and the
  `FakeHarnessAdapter` reference implementation with a polling watcher.
- End-to-end adapter test: discover → prepare account → launch spec →
  run → watch (discovered/appended/ended) → resume.

### Added (Gate V0)

- Gate V0 validation report (docs/qa/gate-M0.md): CI matrix, coverage
  baseline meeting QUALITY.md targets, functional acceptance evidence,
  quality audit with fixes applied in-PR, and carried-forward items.
- Coverage-closing tests: repository update/delete paths, launch-spec
  constructors, registry trait-object usage, adapter degrade paths.

### Added (M0-5)

- `juggler-supervisor`: PTY (interactive) and piped (headless) spawning
  with per-profile env overlays; lifecycle event bus
  (Started/Exited with signal detail); bounded output ring buffer
  (512 KiB/session); SIGTERM→SIGKILL stop ladder; process-tree
  descendant tracking via sysinfo ppid walks; append-only supervision
  journal with crash re-adoption (`adopt_orphans`, zombies treated as
  dead); children survive supervisor/app death by design.
- Integration tests: spawn/capture/stop, external SIGKILL → journaled
  exit, drop-survival + journal re-adoption, process-tree tracking,
  PTY output capture, unknown-session errors.

### Added (M1-1)

- Tauri 2 application shell (`juggler-app`): strict CSP (self only),
  minimal core capability, generated brand icon placeholder, `app_info`
  command + `app://ready` event, typed both sides via ts-rs (Rust is
  the single source for TS types).
- Webview UI scaffold (`ui/`): React 18 + TypeScript + Vite + Tailwind
  v4; dark-first design tokens; base primitives (Button/Card/Chip/
  StatCard/QuotaRing); inline SVG icons; app shell (rail nav, status
  bar, dashboard placeholder); typed bridge with browser fallback.
- CI: `ui` job (typecheck + build) as a required check; Tauri Linux
  system dependencies for cargo jobs.

### Added (M1-2)

- Claude Code adapter: PATH + installer-default + override binary
  discovery with `--version` verification against pinned releases
  (drift warns, never fails); isolated `CLAUDE_CONFIG_DIR` account
  preparation (0700 dirs, 0600 managed-marker, idempotent); metadata-only
  login detection (credentials-file presence + macOS namespaced keychain
  probe — never reads token contents); guided vendor-login launch spec;
  interactive/headless launch specs with settings-overlay env merging.

### Changed

- Handover model upgraded from summary packs to tiered session sync:
  native resume (T1), cross-account session transplant (T2), cross-harness
  transcript replay (T3); context packs remain as preview/export format (T4).
- Explicit automation policy in spec and proposal: monitoring, alerts,
  suggestions, and staging are automated; every state-changing action is
  executed by an explicit user click.

### Added (M1-5)

- Claude Code transcript parser: version-tolerant JSONL extraction
  (uuid/parentUuid/sessionId, roles, text content, token usage) with
  per-kind counters, malformed-line skip-and-count (never panics), and
  resume dedupe by message uuid (first occurrence wins).
- Committed anonymized fixture corpus: basic conversation with
  tool_use/tool_result, subagent sidechains, resumed-session duplicates,
  and malformed damage cases.
- notify-based transcript watcher: fs events wake the watcher; a bounded
  recursive walk is the self-healing diff source (lost/coalesced fs
  events recover on the next poll); emits Discovered/MessageAppended.
- Fuzz scaffold (`fuzz/`): `transcript_parse` target exercising parser +
  dedupe on arbitrary bytes (cargo-fuzz, nightly; outside the workspace).

### Added (M1-3)

- App backend: `AppState` (SQLite storage, adapter registry, journaling
  supervisor) with per-platform data-dir resolution; 13 typed Tauri
  commands (accounts, profiles, sessions, launch/stop, login state and
  copyable vendor-login command, dashboard stats); lifecycle forwarder
  persisting session status transitions; `Profile.name` (schema v2
  migration).
- Supervisor fix surfaced by integration testing: the PTY path now
  inherits the base environment and applies the profile overlay on top
  (spec §4 env semantics).
- Launcher UI (Profiles screen): account cards with harness glyph, plan
  and login chips; per-profile rows with model chips and one-click
  Launch; add-account and new-profile modals; guided-login modal with
  copyable isolated login command (external-terminal fallback) and
  re-check; empty states; dashboard shows live counts. Typed bridge with
  plain-browser design-preview mocks.
- Integration tests (unix): two accounts launch in parallel through the
  full app stack with complete config-dir isolation and per-account
  settings overlays; write containment verified across the sandbox
  (zero writes outside the app data dir).

### Added (M1-3b)

- Smart onboarding foundation (ADR-0006, spec §12.1): adapter-level
  `scan_environment` finds existing setups — the vendor default
  (`~/.claude`), `$CLAUDE_CONFIG_DIR` overrides (other manager tools),
  and `.claude*` home-pattern dirs (manual or AI-created) — with
  metadata-only login probes, dedup, and most-useful-first ordering.
- Config import: `import_account` byte-copies a discovered setup into a
  fresh managed dir (0700 + marker), skipping rebuildable caches, with a
  2 GiB fail-fast ceiling and a read-only guarantee toward the source
  (tree-snapshot tested). Keychain-locked logins honestly report the
  one-time re-login path; file-based logins carry over.
- Login aids capped at "we open things": per-adapter `login_url`, opened
  in the system browser via tauri-plugin-opener.
- App commands `environment_scan`, `account_import` (creates a default
  `Main` profile so imports are immediately launchable), and
  `login_open_web`; typed bridge entries with browser-preview mocks.
- Integration tests: scan classification/dedup/sorting, import
  correctness + source immutability + ceiling, and the command-level
  onboarding path (import → launch → session recorded).

### Added (M1-4)

- Embedded terminal (xterm.js + PTY streaming, issue #20): tabbed
  terminal dock at the bottom of the window — drag-resizable, collapsible
  with ⌘J, layout persisted. Live sessions and guided logins stream into
  it; closing a tab's panel and reopening replays the retained tail.
- Supervisor: PTY keyboard input (`write_input`), resize (`resize_pty`),
  live output taps (ring listeners, one forwarder per terminal), and
  exit probes (`is_finished`).
- App surface: `terminal_spawn_login` (vendor login in an isolated PTY),
  `terminal_attach` (idempotent streaming), `terminal_write/resize/kill/
  snapshot`; base64 chunk events batched at a 16 ms flush — the 60 fps
  UI budget. Guided login now runs fully in-app; the login modal also
  opens the vendor's sign-in page (claude.ai) or copies the command.
- Launch flows land in the dock: clicking Launch opens a terminal tab
  attached to the interactive session.
- Tests: PTY input/resize/tap round-trip; 20 concurrent streaming
  sessions fan-out without deadlock (M1-4 perf floor); app-level
  terminal cores (echo → snapshot replay → stop ladder); login spec
  env-scoping proof.

### Changed (design iteration 4 — instance-centric IA, ADR-0007)

- Reframed the product around what it manages — **instances of AI coding
  tools (CLI today, desktop next) working on tracked projects** — instead
  of a CLI-session board.
- Domain (schema v3): `sessions.kind` (`terminal` | `desktop`) and a
  first-class `projects` table keyed by canonical root; launches touch
  their project and link the session to it — the lineage that powers
  switch/hand-over.
- UI rebuilt to two modes: left rail = **Projects / Instances / Sessions**
  (project panel: who's on it now, account history, launch-with-another-
  account; instance panel: status, kind badge, open-terminal/stop);
  global menus top-right (Overview, Accounts, Usage, Fleet) with a
  primary Add action.
- `projects_list` command with active-instance and account-history
  rollup; ts-rs `ProjectInfo`.
- Tests: projects repo roundtrip, session kind/project-link persistence,
  and the juggling integration proof — two accounts, one shared
  directory, one tracked project with both accounts in its history and
  the live instance surfaced.

### Changed (design iteration 5 — workflow navigator)

- The left rail is now the workflow itself (ADR-0007): three tabs —
  Projects / Instances / Sessions — whose live list lives inside the
  rail (collapsing to icons with ⌘B), plus a New session action
  (⌘N) anchored in the navigator.
- Selecting anything loads its panel on the right; selecting a session
  or instance embeds its **terminal directly in the panel** — the
  bottom dock is gone. Panels cross-link: session → its project,
  project → its sessions, project → launch-with-another-account (the
  one-click juggle), all drill-in-place.
- New-session modal: profile picker + optional directory (launching
  into a directory creates/reuses its project), lands selected with
  its terminal open.
- Guided logins join the Sessions rail list under "Sign-ins" and open
  in the same panel; ended items keep replayable scrollback.

### Changed (design iteration 6 — rail polish: responsive by construction)

- Tabs reflow instead of clipping: labels truncate cleanly down to a
  threshold, then fade to icon-only with tooltips (rail widths
  208–360px); collapsed shows stacked icon tabs. Nothing overflows at
  any width.
- Drag-resize restored on the rail (hairline handle that lights up on
  hover/drag, persisted width; double-click collapses), with the
  layout audit-driven at default/narrow/collapsed states.
- Tighter design-system pass on the rail: 7px row rhythm, 12.5/10.5px
  type pair, Linear-style 2px accent selection bars, focus-visible
  rings, proper aria tablist semantics, tooltips only when labels are
  hidden.
- Thin overlay-style scrollbars across rail and panels.

### Added (M1-3b Phase C — onboarding wizard)

- The zero-knowledge add-account flow (ADR-0006, spec §12.1): Detect
  (auto environment scan, findings in plain language with origin and
  sign-in state) → Identity (plain-words purpose + plan tier, or
  one-tap import of a discovered setup) → Login (sign in inside AI
  Juggler / open claude.ai / copyable command / re-check, with the
  honest keychain note when import can't carry the login) → Summary
  (what was created, what was skipped, original untouched).
- First run with zero accounts opens the wizard automatically (once
  per explicit dismissal); Accounts → Add account uses the same flow.
- Replaces the old add-account form modal; fully reviewable in a
  plain browser via mocks.

### Added (scanner: other manager tools, verified against t3.codes)

- Environment scan now covers **other manager tools' profile roots**
  (new `manager-tool` origin): t3.codes' `ClaudeCodeProfiles/` verified
  on a real machine with five live profiles. Manager-tool findings say
  plainly that the tool and its sessions are never touched.
- Login honesty for pre-2.1.56 setups: when only the legacy shared
  keychain entry exists, manager-tool imports note that a one-time
  re-login inside AI Juggler is likely.
- Operational runner (ignored test) importing real profiles into the
  app's data dir; used to import the user's five t3.codes profiles
  read-only (originals manifest-verified untouched; live session
  transcripts kept growing in the source, proving t3 undisturbed).

### Added (Gate V1 prep)

- `docs/qa/gate-M1.md`: dogfood environment prepared with the five
  imported t3.codes accounts, per-account re-login expectations (shared
  legacy keychain), acceptance checklist, and manifest re-verification
  step.
- UI honesty pass: the header chip names only Claude Code (Codex/ZCode
  ship with their adapters); quota rings say usage tracking lands in M2.

### Fixed (Gate V1 dogfood findings)

- User-stopped sessions now record `stopped`, not `failed`: portable-pty
  surfaces our stop-ladder kills as plain non-zero exit codes, so the
  supervisor tracks stop intent (`was_stopped`) and the lifecycle
  forwarder maps it to the new `SessionStatus::Stopped` (schema-neutral
  wire value `stopped`; UI chip included).
- `dogfood_v1` operational runner: the agent-run mechanics dogfood
  (two imported t3 accounts in parallel on one project, real claude
  under PTY, streaming + stop + project lineage verified; evidence in
  docs/qa/gate-M1.md).

### Added (the golden path — from the experience review)

- **Single instance**: a second launch focuses the running app (no more
  ghost windows); startup reconciles stale "running" sessions whose
  processes died with the last app run (marked stopped — journal
  adoption keeps true survivors live).
- **Your real projects, day one**: `projects_backfill` parses every
  imported transcript (cwd read from transcript content, never guessed
  from encoded dir names) and surfaces the user's actual project
  history with per-account attribution — schema v4 (`history_json`).
  On this machine: 6 real projects (two app repos, a code-graph tool, t3
  worktrees…) with account history, idempotent, run at startup.
- **Get-ready home**: any account not signed in makes the app open on a
  checklist — one click per account opens Claude's own sign-in in an
  embedded terminal; progress "N of M ready"; skippable.
- **New session without typing**: recent projects (real history) +
  native macOS folder picker + drop-a-folder-anywhere; account list
  shows sign-in state before launch (unusable accounts disabled with
  the reason, not silent).
- **The juggle (v0)**: `session_handover` — moves a session's transcript
  to another account (byte copy, same project subpath) and resumes it
  there (`claude --resume <id>`), recording lineage (source
  HandedOver → continuation with native id + transcript ref). "Continue
  on another account" chips on every session panel.
- Login-state banner on sign-in terminals; `LaunchMode::Resume` in the
  adapter contract.

### Stage 1 — plan documents rewritten to the settled decisions (D-01..D-15)

- Decision log marked **Final** on maintainer instruction (2026-10-13);
  second-opinion review corpus committed (two-pass reports, competitive
  reviews, 107-task E-catalog with evidence contracts, 27 wireframes).
- SPEC rewritten where the decisions demanded it: §3 domain
  (Provider/Account/Instance vocabulary, Switch Graph, spawn ledger,
  pack v2, honesty states), §5 (ledger-owned verified cleanup, D-08
  tiers), §6 (two clocks, D-12), §7 (unlimited bidirectional
  Switching, hybrid door, switch-back resume+delta, per-pair fidelity
  badges), §9 (Inspector-primary UX, indicator grammar, Cleanup
  Centre), §11 (Fleet at launch, D-13), §12.1 (cross-Provider
  onboarding).
- PROPOSAL re-centered (Switch → See → Clean; D-09 tagline); EPICS
  resequenced with phase frame, slip rule, and expanded gate evidence.
- ADR-0008..0013 accepted (terminology, pack v2 + user-owned
  redaction, spawn ledger, Switch Graph, two clocks, Fleet at launch);
  ADR-0003's automation invariant gains its two recorded amendments.
- README truth-fixed (shipped vs specified vs labelled-beta table);
  docs/qa/beta-exit.md defines the eight beta-exit gates.
- Billing/renewal API research recorded (docs/research/billing-apis.md):
  no sanctioned public renewal endpoints found for any Provider —
  subscription clocks stay user-entered (D-12).
- AJ-E901..E910 DONE-EVIDENCED with per-task evidence manifests and
  the Stage-1 acceptance packet (docs/qa/evidence/AJ-E910/). No
  product code changed (scope proof in the packet).

### Stage 2 begins — E8 evidence machinery (AJ-E801/E802/E806) + D-16 themes

- **Evidence store (AJ-E801)**: `docs/qa/evidence/<task-id>/` with the
  manifest template (6 fields + redaction checklist), a generated
  `_index.md`, and `scripts/evidence_check.sh` (gate-doc link
  resolution, credential-pattern secret scan, index freshness) wired
  into `just verify` and the CI verify job.
- **Visual harness (AJ-E802)**: Playwright against the production
  build with seedable bridge fixtures — all 9 SessionStatus values,
  honest-empty and ready-home sets; capture matrix 4 viewports
  (incl. Tauri's 1024×680 minimum and 390×844) × 2 themes; 41
  captures; browsable generated gallery; byte-identical determinism
  self-test (pinned clock + reduced-motion); not-yet-existing states
  recorded as NOT CAPTURED with owning task IDs, never omitted.
- **Gates (AJ-E806)**: single gate vocabulary across justfile /
  QUALITY.md / CI (verify, ui, evidence now; visual/a11y/perf named
  with owners); seeded-failure proofs for unit and visual gates
  (captured red, then green after removal); a11y seeded failure
  recorded DEFERRED → AJ-E804.
- **D-16 dual theme implemented**: Light + Dark token layer (values
  only — token names unchanged), System/Light/Dark choice persisted,
  `⌘⇧L` cycles, flash-free init, WCAG-minded light palette; both
  themes captured in the gallery.

### E1 naming (Phase A + B1 + B2) — Providers/Accounts/Instances begin

- **AJ-E101**: ADR-0008 confirmed Accepted + Amendment A-1 recorded
  (identifier semantics: `SessionId` stays the *Instance* id; new
  `ConversationId` newtype wraps the *native* conversation id; wire
  and storage columns unchanged).
- **AJ-E102 (Phase A — UI copy)**: user-facing copy swept to the
  charter vocabulary ("provider:", "Launch settings", honest cwd
  placeholder); gallery re-captured green in both themes.
- **AJ-E103 (Phase B1 — domain aliases)**: `ProviderId = HarnessId`
  and `ProviderAdapter` additive aliases; zero wire change.
- **AJ-E104 (Phase B2 — Instance/Session split)**: `SessionRecord` →
  `InstanceRecord` (old name aliased one minor version);
  `native_id: Option<String>` → `conversation: Option<ConversationId>`
  (DB column and values untouched — proven by a raw-SQL v4-fixture
  round-trip test, user_version unchanged); `InstanceStatus` alias;
  `launch_settings()` accessor pending AJ-E105's field rename;
  `SessionInfo.conversation_id` exposes the conversation in the
  view-model; 83 tests green.

### E1 Phase B3 (AJ-E105) — Launch settings demotion

- `Profile` → `LaunchRecipe` canonically (`Profile`/`LaunchRecipeId`
  aliases retained one minor version); stored values untouched
  (alias round-trip test green).
- Accounts screen: per-Account "Start on <label>" primary (disabled
  with reason while signed out) + "Edit launch settings" section,
  collapsed by default, with the spec's helper copy; recipe rows show
  "Default" for the auto-created Main. No new window.alert.

### E1 Phase B4 (AJ-E106) — schema v5

- Forward-only V5 migration: `accounts.harness → provider_id`,
  `sessions → instances` (+ `native_id → conversation_id`,
  `profile_id → launch_recipe_id`), `profiles → launch_recipes` —
  pure `ALTER … RENAME` inside the per-version transaction;
  user_version 4→5 exactly once on open; zero stored values
  rewritten (proven by an in-place v4-file upgrade test with real
  values); downgrade still fails `SchemaTooNew`. Repositories query
  only the new names.

### E1 Phase B5 (AJ-E107) — bridge + generated-types rename

- DTOs: `ProviderInfo` / `LaunchRecipeInfo` / `InstanceInfo` canonical
  (`HarnessInfo`/`ProfileInfo`/`SessionInfo` deprecated aliases one
  version; `#[serde(alias = "harness")]` keeps old payloads
  deserialising). Wire fields: `provider`, `launch_recipe_id`,
  `conversation_id`.
- Tauri commands gain canonical names (`providers_list`,
  `launch_recipes_list`, `instances_list`) with legacy names registered
  as one-version aliases; arguments/returns unchanged.
- ts-rs regenerated (never hand-edited beyond the generated re-export
  alias files); bridge exposes canonical functions with `@deprecated`
  legacy aliases; all UI call sites on canonical names; gallery green
  end-to-end with the new wire fields.

### Renamed (E1, AJ-E101–E108) — terminology per ADR-0008

Old → new: harness → **Provider**; profile (as account) → **Account**;
profile (as launch config) → **Launch settings** (`LaunchRecipe`
internal); session (running process) → **Instance**; handover →
**Switch**. Stored data and wire IDs are unchanged; old databases
migrate automatically (schema v5). Deprecated aliases (Rust types,
serde fields, Tauri command names, bridge functions, TS types) remain
for one version. The terminology lint gate (`just terminology`) now
enforces the vocabulary in SPEC/PROPOSAL/EPICS and JSX copy.
