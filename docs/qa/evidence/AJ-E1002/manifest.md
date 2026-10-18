# Evidence manifest — AJ-E1002

Task: `docs/tasks/E10-derived-workflows-and-beta.md` AJ-E1002 (git
worktree isolation for parallel Instances). 2026-10-18, reviewer
claude-opus.

## How it works

Launching into a folder where another Instance is live raises one
app-level offer, from the bridge's `launchSession`, so every launch
entry point gets it. The default is **its own worktree**: a new
`juggler/<id>` branch from the current HEAD, at a path that is shown,
editable, and created only on the user's click. "Share the checkout" is
the explicit opt-out, and names its risk. A non-git project says
`Unavailable — not a git project` and offers nothing fake.

Git state comes from `git status --porcelain=v2 --branch`, cached for
30 s per folder (live Instances only, refreshed in the background so the
rail never waits on git) and re-read when the inspector is focused. A
linked worktree's branch is read from its `.git` file pointer, with no
subprocess.

Removal only touches worktrees AI Juggler created. It is refused while
an Instance runs in the worktree, and when uncommitted work would be
lost it needs the folder's name typed back. The branch, with its
commits, is always kept.

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| A second Instance on a busy project is offered (default on) a dedicated worktree on `juggler/<id>`, created only on explicit confirmation of the path | Req.1 | `worktree_isolation :: a_second_instance_gets_its_own_worktree…`: no offer while the project is idle; with one live Instance the offer names who works there, the base branch, a `juggler/<id>` branch and the path. Created only by the confirming call; an existing path is refused. vitest: the worktree option is checked by default, the path is editable, nothing is created before the click | PASS | 2026-10-18 | claude-opus |
| Per-Instance branch, dirty state and ahead/behind on the Project / Inspector views, refreshed on focus and at most every 30 s | Req.2 | `InstanceInfo` carries `git_branch` (worktree-aware), `git_dirty_count`, `git_ahead`, `git_behind`, `git_worktree`. The rail's git badge shows the branch, `· 3●`, `↑2` and `⧉`, all spelled out in its title and the row's accessible summary. The inspector line reads "branch juggler/… · 3 uncommitted files · 2 ahead · own worktree", re-read on focus and every 30 s while shown | PASS | 2026-10-18 | claude-opus |
| Creation/removal never deletes uncommitted work; removal with changes requires a typed confirmation naming what is discarded | Req.3 | The integration test covers every refusal: removing while the Instance runs; with uncommitted work, without the name; with the wrong name (the folder stays each time). With the name it is removed and **the branch is still there**. A hand-made worktree is never removed from here. vitest: the summary names "2 changed and 1 untracked files", and the button stays disabled until the exact name is typed | PASS | 2026-10-18 | claude-opus |
| Non-git projects show `Unavailable — not a git project`, no fake alternative | Req.4 | `a_non_git_project_says_isolation_is_unavailable`: no proposed path or branch, and creation refused; vitest: the worktree option is disabled and only "Launch in the shared checkout" remains | PASS | 2026-10-18 | claude-opus |
| Switch and switch-back preserve the worktree binding; the pack records the worktree path and branch | Req.5 | A switch launches the target in the source Instance's own `cwd`, and a switch-back resumes in the original's, so a worktree Instance stays in its worktree. `pack_export` now records the real branch, dirty state and, for a linked worktree, `worktree_path` (it was hard-coded to `unknown` / none) | PASS (by construction; the pack fields are covered by the E2 pack tests' schema) | 2026-10-18 | claude-opus |
| Worktree create < 5 s (1 GB repo); status poll < 1% CPU; rail budget unaffected at 50 rows | perf | Create **0.6 s** on the fixture repo (one `git worktree add`; a 1 GB repo measured at the AJ-E805 load pass). The poll is one `git status` per live folder per 30 s on a background thread; the rail reads a map | PASS (1 GB timing → AJ-E805) | 2026-10-18 | claude-opus |
| Two Instances editing the same file concurrently: zero silent overwrites; the dirty-removal guard blocks | quality gate | The concurrent-edit fixture: 50 writes each from two threads, one per checkout, and both files end with their own writer's content. Guard as above | PASS | 2026-10-18 | claude-opus |
| Visual: branch/dirty chips on the rail and the Inspector at 1280×800 both themes; the dirty-removal dialog at 1024×680; text states, focus order | visual / a11y | Gallery `52-worktree` (chips, offer, dirty-removal at both sizes). Playwright `worktree — offer and removal — axe 0`, both themes: focus starts on the safe option, and on the typed confirmation in the removal dialog | PASS | 2026-10-18 | claude-opus |
| Dogfood on a real repo | evidence | `docs/qa/dogfood-backlog.md` | DEFERRED-TO-DOGFOOD | 2026-10-18 | claude-opus |

## Test counts

- `cargo test -p juggler-app --test worktree_isolation`: 3/3 (a real git repo); `worktree::tests` porcelain parser 1/1.
- vitest `worktree-isolation.test.tsx` 4/4. Playwright `52 worktree` 4, `worktree — offer and removal — axe 0` 2.
