# AJ-E2907 evidence: a confirm before deleting launch settings

- `cargo test -p juggler-app --test manage_accounts`
  ("deleting_launch_settings_keeps_their_sessions_and_waits_for_open_ones":
  refused through the command while a session is open, naming it; the
  sessions stay in History) and vitest `delete-launch-settings.test.tsx`
  — pass (2026-11-05).
- Gallery `41-policy`: the confirm (`accounts-delete-launch-settings-*`)
  and the refusal while three sessions are open
  (`accounts-delete-launch-settings-refused-*`); 1280×800 and 1024×680,
  dark and light; critique updated and stamped.
- The preview now refuses as the backend does, from its own sessions
  (`lib/ipc/manage.ts` `deleteProfile`), so the refusal can be seen and
  captured without a live backend.
- Verdict: PASS.
