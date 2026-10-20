# Evidence manifest — AJ-E1024

Task: `docs/tasks/E10-derived-workflows-and-beta.md` AJ-E1024 (ADR-0018). Status: **DONE-EVIDENCED**. 2026-10-20, reviewer claude-opus.

## What the maintainer asked, and what changed

1. **"Most of people don't want 10s of different profiles confusing everywhere."**
   - Bringing in now defaults to **Use them where they are**. The account's folder is the user's own. Claude's Keychain sign-in is named after that exact path, so it carries, and history and settings are shared with the Claude app.
   - **Make separate copies** sits beside it, with one line each on what it means.
2. **AI Juggler writes nothing of its own into an in-place folder.**
   - No marker, no 0700, no `settings.json` edits, no status line wrapper or spool.
   - The status line capture for sessions AI Juggler starts lives in `overlays/<account-id>/` and goes in with `claude --settings`. It chains the user's own status line, read-only.
   - The usage pass reads the folder's transcripts and installs nothing there.
3. **A bug found on the way.** A profile's model rewrote the account's whole `settings.json` to `{"model": …}` on every launch, wiping hooks and permissions in copies. The model now goes in as `claude --model`, for every account.
4. **The default `~/.claude`.** In place, it launches and signs in with `CLAUDE_CONFIG_DIR` unset, which is how Claude runs it: un-namespaced Keychain entry, `~/.claude.json`. The login probe and identity read match. The supervisor removes the variable when told to (`ENV_UNSET`) in every spawn path.
5. **"We should always give option to the user if they want to continue with partial import or try again."**
   - Each pick comes in or doesn't on its own. The result lists what failed, in plain words.
   - The choices are **Try again** (those only), **Use it where it is instead** (a failed copy) and **Continue with N**. Sign-ins of the ones that came in survive retries.
6. **"What happens if I stop mid point."**
   - While bringing in, Close and Esc read **Stop**. The setup in progress finishes and the rest wait.
   - The result offers **Bring in the rest** or **Continue**. What came in stays and shows as "Already in AI Juggler" next time.
   - A copy whose record can't be saved removes its copied folder.
7. **Elsewhere.** The scan marks in-place folders as already in, from the account list. The isolation checklist has honest rows for in-place accounts. "Reveal" opens the user's folder. The Accounts card says "uses ~/x where it is" or "a copy of ~/x".

## Review findings fixed (independent review, before the PR)

- **The status line never ran** from a path with a space; this predates the change and broke copies too. It is now a quoted command, and old installs are repaired (see AJ-E806).
- **A folder already copied** was preselected, and "use it where it is" then reported it as brought in. Sources of copies now show "Already in AI Juggler", and an `already_imported` result is said as such.
- **The default folder** is the exact `~/.claude` only, not a link to it. Its state file prefers `~/.claude.json`.
- **The sign-in terminal** runs in an empty AI Juggler folder, not in the config folder (it would become a "project").
- **The Expert snippet** for the default folder is `env -u CLAUDE_CONFIG_DIR claude`.
- **The wrapper and launch settings** are written atomically, and only when changed.
- **A failed save** deletes the account row only if it was inserted.
- **The status line** is only ever installed into a folder with AI Juggler's marker.
- **The result panel** is a live region and takes focus.

## Proof

| Requirement | Proof | Result |
|---|---|---|
| Req.1–2 in place, nothing written | Rust `in_place_accounts :: a_setup_used_where_it_is_is_never_written_by_ai_juggler`. Bring in (with a trailing `/`), launch with a model, a usage pass, the checklist (all pass), then a second bring-in is refused. Every path's size and mtime in the folder is unchanged. The stub sees `CLAUDE_CONFIG_DIR` = the folder, `--model opus` and `--settings <overlays>/launch-settings.json`, and the wrapper chains `my-line`. | PASS |
| Req.3 the model | `a_copy_keeps_its_settings_file_when_a_profile_sets_a_model`: the copy's `settings.json` is byte-identical after a model launch (it is rewritten on the old code). | PASS |
| Req.4 the default folder | `vendor_default_folder` (`~/.claude` and a link to it run unset; others exact; `~/.claude.json`) and `supervisor :: the_unset_marker_removes_the_variable`. | PASS |
| Refusals | `ai_juggler_s_own_copy_or_a_non_setup_cannot_be_used_in_place` (our own copy, a non-setup, a missing folder). | PASS |
| Req.5–6 the user decides | vitest `first-run.test.tsx`: default in place and no sign-in step; Continue with the ones that came in; Try again only the failed, keeping sign-ins (3 rows); a failed copy in place instead; Stop via Esc ("Stopped. 1 brought in, 2 still to go."), then Bring in the rest. Plain error words. | PASS |
| Req.7 placement words | vitest `account-runway.test.tsx` "where an account lives". | PASS |
| Walked by hand | Browser preview (1280×800, dark): the choice in view with in place picked; a copy failing for one of two → "1 brought in. 1 couldn't come in." with Use it where it is instead / Try again / Continue; Use it where it is instead → on to sign-in for the remaining copy only; Stop with a real click mid-way → "Stopped. 1 brought in, 1 still to go." with Bring in the rest / Continue. | PASS |

## Not covered here

- No real Claude session was started on an in-place folder in this pass (that needs a real sign-in). The Keychain rule was checked read-only on the maintainer's Mac, by entry existence only (no `-w`, nothing read). The plain `Claude Code-credentials` entry exists and none is namespaced for `~/.claude`, so the default folder runs unset. Each `ClaudeCodeProfiles/<name>` has the entry namespaced by its exact path, which the in-place account keeps verbatim.
- Codex setups still come in as copies: the adapter's `adopt_account` returns "unsupported" until the Codex adapter implements it.
- Measured quota for sessions started outside AI Juggler on an in-place folder comes from transcripts only, by design (ADR-0018).
