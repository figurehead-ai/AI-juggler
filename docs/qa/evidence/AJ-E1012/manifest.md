# Evidence manifest — AJ-E1012

Task: `docs/tasks/E10-derived-workflows-and-beta.md` AJ-E1012 (Settings —
the policies the user owns, wireframe 26). Status: **DONE-EVIDENCED**.
2026-10-19, reviewer claude-opus.

## What exists now

- **Section list** beside Settings: General, Account & Cloud, Context packs
  & secrets, Cleanup, Accounts & renewal dates, Notifications, Fleet
  permissions, Data. Every section stays on the page; the list jumps, moves
  focus to the section heading and marks where you are (a jump keeps its
  mark while the scroll settles).
- **Context packs & secrets**: the D-14 control, now one shared component
  used by Settings and Accounts.
- **Cleanup**: each Account's two D-08 toggles, the rules stated once, and
  a default for new Accounts ("Notify only" unless chosen; Tier 1 only;
  existing Accounts keep their own choice).
- **Accounts & renewal dates**: each Account's user-entered date ("Not
  set: no renewal reminders") and the lead time, 1–30 days, default 7.
- **Notifications**: quiet hours for the hourly digest (default 22:00–07:00,
  equal hours = none, deferred never dropped) and which Accounts join the
  digest. Actionable alerts stay at-most-once per window.
- **Fleet permissions**: paired Macs and their access, changed through the
  Fleet screen's Access sheet.
- Backend: `settings_policy` (notification preferences, cleanup default);
  `QuietHours` in the alert engine (`digest_flush` keeps its default).
- Accessibility: buttons whose hidden text started with a space now put
  the space outside it, so names read "Access… for Mac mini", not
  "Access…for" (also in onboarding, the rail and remote switch targets).

## Proof

| Requirement | Proof | Result |
|---|---|---|
| Req.1 section list, focus, current mark | vitest `settings-sections.test.tsx` (7); browser check: jump to Notifications and to the last section keeps the mark and focuses the heading | PASS |
| Req.2 pack secrets shared | `PackSecretsCard` used by Settings and Accounts; existing switch-redaction and first-run tests pass | PASS |
| Req.3 cleanup default | `settings_policy :: the_cleanup_default_is_off_unless_chosen_and_only_reaches_new_accounts`; vitest | PASS |
| Req.4 renewal | vitest (default 7, per-Account "Not set", saved lead time) | PASS |
| Req.5 quiet hours, digest mutes | `settings_policy :: notification_prefs_default_to_the_anti_fatigue_hours_and_persist`; unit `quiet_hours_wrap_past_midnight_and_equal_means_none`; `juggler-core --test ranking_alerts` unchanged (11/11); vitest | PASS |
| Req.6 Fleet permissions | vitest (row, Access sheet, empty state) | PASS |
| Gallery and axe | `65-settings-sections` (12 states) and `44-settings`: axe 0 violations | PASS |
| Core visual baselines | Settings cells re-rendered in the pinned container (`visual_gate.sh --update`); per-screen notes in the PR | PASS |
