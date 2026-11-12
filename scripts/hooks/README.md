# Git hooks

Installed by `just hooks` (`git config core.hooksPath scripts/hooks`).

| Hook | What it checks |
|---|---|
| `pre-commit` | The staged changes' added lines and paths hold no denylisted real name (`private_terms.py staged`). |
| `commit-msg` | The commit message holds none (`private_terms.py message`). |
| `pre-push` | Every outgoing commit's added lines, paths and message hold none (`private_terms.py push`); and a push to `main` or `release/*` is of a commit `scripts/preflight.sh` passed. |

## The private denylist (D-101)

The repository is public. Real names (the maintainer's profile folders,
people, the Mac's user name, private addresses) must never reach it, so
the list of them lives **outside** it:

    ~/.config/ai-juggler/private-terms     (or $AIJ_PRIVATE_TERMS)

One term per line, matched case-insensitively anywhere in a line; `#`
starts a comment. Never commit it, never paste it into a brief, an issue
or a pull request. A hit names the file and the term's line number in the
list, never the term itself. Replace the real name with a generic one:
`acme-*` for a profile, `/Users/you` for a home folder, `example.com` for
an address.

Without the file the hooks say so and let the change through; the public
check below still runs.

## The public check

`scripts/private_data_lint.py` runs in CI on every pull request. It holds
no real name: it fails on any `/Users/<name>` that isn't a placeholder,
any `ClaudeProfiles/<name>` or `ClaudeCodeProfiles/<name>` that isn't a
fixture, and any e-mail address outside example.com, GitHub's noreply
and the few published ones it lists.

## Recorders

A script that records a run on a real setup into the repository writes
through `scripts/qa/anonymize.py`, and has a `--selftest` that feeds it a
real-looking name and home folder and checks neither reaches the output
(`scripts/qa/presence_check.py --selftest`).
