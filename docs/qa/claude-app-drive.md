# Driving the Claude app in lab proofs: `app_drive`

`scripts/qa/app_drive` (`app_drive.swift`) operates one Claude desktop app window by its process id. It is for the lab proofs that act inside the sanctioned **aij-test** profile (`docs/research/desktop-pool-members.md` §7.2 and §7.3) and the 7b walks' Claude app checks (`docs/qa/0.4-walks/7b-live-claude.md`). It's modelled on `floor_drive`. It never moves the real pointer and never sends a keystroke to any other app, so the Mac stays usable while it runs.

## Commands

```sh
export AIJ_APP_PROFILE=aij-test            # required: the profile it may act on
export AIJ_APP_OUT=<scratchpad>/claude     # where pictures go (never the repo)
scripts/qa/app_drive list                  # the Claude app pids; names only aij-test's
export AIJ_APP_PID=<aij-test's pid>
scripts/qa/app_drive shot
scripts/qa/app_drive click 640 720
scripts/qa/app_drive type echo lab
scripts/qa/app_drive key return
scripts/qa/app_drive menu                  # the menu bar's menus
scripts/qa/app_drive menu File             # one menu's items (› submenu, ✓ chosen)
scripts/qa/app_drive menu "File>New Code Session"   # presses that item (use the title `menu File` lists)
scripts/qa/app_drive front
scripts/qa/app_drive --selftest
```

| Verb | What it does |
|---|---|
| `list` | Each running Claude app: pid, "aij-test" or just "another profile" / "the default profile", and its window's size. Needs no `AIJ_APP_PID`. |
| `shot` | A picture of the pid's main window (`screencapture -l`, by its CGWindow id), at its size in points. Prints the path and the size. A picture's pixel is a click's point. |
| `click X Y` | X and Y are points from the window's top left. Presses the element there through accessibility (`AXPress`), or puts the cursor in the text field there. |
| `type TEXT` | Types into the field that has the cursor. The key events are posted to the pid (`CGEvent.postToPid`), one per character, carrying that exact character. |
| `key NAME` | `return`, `escape`, `tab`, `space`, `delete`, the arrows, `home`, `end`, `pageup`, `pagedown`, or a combination such as `cmd+n` or `shift+tab`. Posted to the pid. |
| `menu PATH` | Exact titles joined by `>`; never a prefix. A menu is listed; an item is pressed through accessibility. |
| `front` | Brings that window forward. The Mac's keyboard focus moves to it, so use it only where a step needs it. |

The verbs that act answer with the time until the window changed (up to 3 s), as `floor_drive` does.

The wrapper compiles the Swift file once per source (the cache in `$TMPDIR/aij-app-drive` is keyed by the file's hash, so another worktree's copy is never run).

## The guard

It's read at that moment, before every verb on a pid. Any refusal prints its reason and exits 2.

1. **A Claude app process.** `AIJ_APP_PID` is a running process whose executable is the main one of a bundle named exactly `Claude.app` (`…/Claude.app/Contents/MacOS/…`), with the Claude app's bundle id. AI Juggler (installed or Dev), a terminal, the Claude Code CLI (its bundle is a lowercase `claude.app`), a helper process and every other app are refused.
2. **The test profile.** `AIJ_APP_PROFILE` must be set. The process's `--user-data-dir` must end with that folder, by whole folder names. The default profile (started without `--user-data-dir`), a different profile, two values and a value in the next argument (which the app doesn't read) are all refused. So the driver can't act on a real profile, even by a wrong pid.
3. **The window.** The pid's main window exists and is on screen, for `shot`, `click`, `type`, `key` and a menu press. `front` is how a window off screen comes back.

Before each key event, rule 1 is read again: if the process is gone or changed, nothing more is sent.

It also refuses:

- clicks in the window's close, minimise and zoom buttons;
- presses labelled log out, sign out, quit or delete account;
- the keys that quit, close, hide or minimise (`cmd+q`, `cmd+w`, `cmd+h`, `cmd+m`), and `cmd+tab`, `cmd+space`;
- menu items that quit, close, hide, sign out, delete or reset, plus the Apple menu and Services;
- text it can't type exactly: line breaks, tabs, control and invisible format characters (use `key return`, `key tab`);
- a click where another window of the app covers the point.

## What it never does

- A global mouse or key event, moving the pointer, or a keystroke to the front app or any other process.
- Reading the app's profile files, any credential, Keychain item or token.
- Asking macOS for a permission. Accessibility and posting events are preflighted only; a prompt is the maintainer's.

The process's arguments are read from the kernel for `--user-data-dir` only. The environment that follows them in the same buffer is wiped unread. `list` names only the profile in `AIJ_APP_PROFILE`.

**Why keys need no frontmost check.** The 7b sheet's click guard exists because a global click lands in whatever app is in front. `app_drive`'s input never goes through the front app. Clicks are accessibility presses on the pid's own elements, and key events are posted to the pid. An app in the background may drop posted keys, though. If `type` or `key` answers "no change in 3 s", run `front`, then try again.

## Selftest and status

`--selftest` needs no Claude app. It checks these refusals:

- rule 1, on paths: AI Juggler, Terminal, the CLI's `claude.app`, a helper inside `Claude.app`, and a look-alike folder;
- rule 1, on live processes: its own pid, pids 0, 1 and -1, a pid past the range, a pid whose process ended, and Finder;
- rule 2: a mismatch, a name that only ends the same, a folder inside the profile, the default profile, the next-argument form, two values, a value after `--`, no `AIJ_APP_PROFILE`;
- the text, key and menu refusals.

It also runs the command end to end with a non-Claude pid and a missing pid for every verb, expecting exit 2 and the reason. If a Claude app is running, it checks read-only that rule 1 accepts it and rule 2 refuses a wrong profile. Nothing is ever sent.

**2026-11-15:** built clean, selftest 50/50 ok. `shot` was checked read-only on a running profile window: 1701 × 1348 points, captured and deleted unopened. The live refusals were also run with `shot`: a wrong profile, no profile, the default profile, the CLI's `claude.app`, and three AI Juggler processes. `click`, `type`, `key`, `menu` and `front` haven't yet been run against aij-test. The first §7.2 run checks them, in particular whether keys posted to the app reach its prompt while it's in the background, and records the result here.
