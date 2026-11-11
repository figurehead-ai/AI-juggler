-- A 0.3 data folder's database (schema v39) for the A9 walk ("Upgrade to
-- accounts"): three Claude Code folders, two of them on one sign-in that
-- disagree on plan, renewal and fee (0.3 kept each on the folder: the
-- plan and renewal in `accounts`, the fee in `app_settings`), one on
-- another sign-in. `{LAB}` is the lab's HOME; generic names only. The
-- sign-ins are the lab's made-up ones (`pools_v45.rs` writes them).
INSERT INTO accounts (id, provider_id, label, plan, state_dir, created_at, renewal_date)
    VALUES ('work-a', 'claude-code', 'Work A', 'max-20x', '{LAB}/profiles/claude-work-a', 1792486800000, '2026-11-20'),
           ('work-b', 'claude-code', 'Work B', 'max-5x', '{LAB}/profiles/claude-work-b', 1792486800000, '2026-11-28'),
           ('personal', 'claude-code', 'Personal', 'pro', '{LAB}/profiles/claude-personal', 1792486800000, NULL);
INSERT INTO app_settings (key, value)
    VALUES ('subscription_fee:work-a', '200'),
           ('subscription_fee:work-b', '100'),
           ('subscription_fee:personal', '20');
INSERT INTO launch_recipes (id, account_id, settings_json, default_cwd, created_at, name)
    VALUES ('rec-work-a', 'work-a', '{}', '{LAB}/code/web', 1792486800000, 'Main'),
           ('rec-work-b', 'work-b', '{}', '{LAB}/code/web', 1792486800000, 'Main'),
           ('rec-personal', 'personal', '{}', '{LAB}/code/api', 1792486800000, 'Main');
INSERT INTO projects (id, root, name, created_at, last_activity)
    VALUES ('p-web', '{LAB}/code/web', 'web', 1792486800000, 1792573200000),
           ('p-api', '{LAB}/code/api', 'api', 1792486800000, 1792573200000);
INSERT INTO instances (id, conversation_id, launch_recipe_id, machine, cwd, status, started_at,
                       last_activity, usage_json, kind, project, label)
    VALUES ('s-a', NULL, 'rec-work-a', 'this-mac', '{LAB}/code/web', 'stopped', 1792490400000, 1792494000000, NULL, 'terminal', 'p-web', 'Checkout copy'),
           ('s-b', NULL, 'rec-work-b', 'this-mac', '{LAB}/code/web', 'stopped', 1792490500000, 1792497600000, NULL, 'terminal', 'p-web', 'Retry banner'),
           ('s-p', NULL, 'rec-personal', 'this-mac', '{LAB}/code/api', 'finished', 1792490600000, 1792501200000, NULL, 'terminal', 'p-api', 'API notes');
