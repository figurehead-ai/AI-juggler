-- A 0.3 data folder's database, as 0.3.1 to 0.3.9 left it (schema v39),
-- with every kind of pool and a row of each kind of content a pool holds:
-- the P9 walk ("Moving from 0.3") opens 0.4 on it in a lab. Applied after
-- the first 39 migrations (`pools_v45.rs`); `{LAB}` is the lab's HOME, so
-- every folder named here is the lab's own. Generic names only.
--
-- Projects shop and blog; five ended sessions; pools:
--   pool-default:p-shop  shop's own pool (0.3 made it): s-1 lead
--   pool-default:p-blog  blog's own pool: s-4 joins as a member
--   pool-default:none    sessions with no project: s-5
--   pool-a  "Checkout"   a user pool in shop: s-2 lead, its rule, approval,
--                        thread, doc, open ask, tasks and canvas places
--   pool-b  "Notes"      a user pool with no project: s-3 observer, a work
--                        item, a task, paused
--   pool-gone "Old spike" deleted in 0.3

INSERT INTO accounts (id, provider_id, label, plan, state_dir, created_at)
    VALUES ('work', 'claude-code', 'Work Max', 'max-20x', '{LAB}/profiles/claude-work', 1792486800000),
           ('personal', 'codex', 'Personal', 'plus', '{LAB}/profiles/codex-personal', 1792486800000);
INSERT INTO launch_recipes (id, account_id, settings_json, default_cwd, created_at, name)
    VALUES ('rec-work', 'work', '{}', '{LAB}/code/shop', 1792486800000, 'Main'),
           ('rec-personal', 'personal', '{}', '{LAB}/code/blog', 1792486800000, 'Main');
INSERT INTO projects (id, root, name, created_at, last_activity)
    VALUES ('p-shop', '{LAB}/code/shop', 'shop', 1792486800000, 1792573200000),
           ('p-blog', '{LAB}/code/blog', 'blog', 1792486800000, 1792573200000);
INSERT INTO work_items (id, project, title, created_at)
    VALUES ('w-blog', 'p-blog', 'Blog relaunch', 1792490400000);
INSERT INTO instances (id, conversation_id, launch_recipe_id, machine, cwd, status, started_at,
                       last_activity, usage_json, kind, project, label)
    VALUES ('s-1', NULL, 'rec-work', 'this-mac', '{LAB}/code/shop', 'stopped', 1792490400000, 1792494000000, '{"tokens":{"input":18400,"output":2300,"cache_read":91000,"cache_write":4100,"reasoning":0},"confidence":"Estimated"}', 'terminal', 'p-shop', 'Checkout plan'),
           ('s-2', NULL, 'rec-work', 'this-mac', '{LAB}/code/shop', 'stopped', 1792490500000, 1792497600000, '{"tokens":{"input":18400,"output":2300,"cache_read":91000,"cache_write":4100,"reasoning":0},"confidence":"Estimated"}', 'terminal', 'p-shop', 'Retry banner copy'),
           ('s-3', NULL, 'rec-personal', 'this-mac', '{LAB}/notes', 'stopped', 1792490600000, 1792501200000, '{"tokens":{"input":18400,"output":2300,"cache_read":91000,"cache_write":4100,"reasoning":0},"confidence":"Estimated"}', 'terminal', NULL, 'Notes tidy-up'),
           ('s-4', NULL, 'rec-personal', 'this-mac', '{LAB}/code/blog', 'finished', 1792490700000, 1792504800000, '{"tokens":{"input":18400,"output":2300,"cache_read":91000,"cache_write":4100,"reasoning":0},"confidence":"Estimated"}', 'terminal', 'p-blog', 'Blog draft'),
           ('s-5', NULL, 'rec-work', 'this-mac', '{LAB}/scratch', 'stopped', 1792490800000, 1792508400000, '{"tokens":{"input":18400,"output":2300,"cache_read":91000,"cache_write":4100,"reasoning":0},"confidence":"Estimated"}', 'terminal', NULL, 'Shell spike');

INSERT INTO pools (id, name, kind, project, created_at, access_level, spans_macs)
    VALUES ('pool-a', 'Checkout', 'user', 'p-shop', 1792490000000, 'full', 0),
           ('pool-b', 'Notes', 'user', NULL, 1792490000000, 'read', 0),
           ('pool-gone', 'Old spike', 'user', NULL, 1792490000000, 'work_together', 0);
UPDATE pools SET deleted_at = 1792500000000 WHERE id = 'pool-gone';
INSERT INTO pool_members (pool_id, member_ref, role, joined_at, left_at)
    VALUES ('pool-default:p-shop', 'session:s-1', 'lead', 1792490400000, NULL),
           ('pool-a', 'session:s-2', 'lead', 1792490500000, NULL),
           ('pool-b', 'session:s-3', 'observer', 1792490600000, NULL),
           ('pool-b', 'work:w-blog', 'member', 1792490650000, NULL),
           ('pool-gone', 'session:s-5', 'member', 1792490800000, 1792500000000);

-- Checkout's rule and approval, a thread with a question and its answer,
-- a doc, an open ask, and where its members sat on the canvas.
INSERT INTO agent_rules (pool_id, action, made_by, created_at)
    VALUES ('pool-a', 'post_message', 'user', 1792491000000);
INSERT INTO agent_approvals (pool_id, action, at)
    VALUES ('pool-a', 'share_doc', 1792491100000);
INSERT INTO agent_threads (id, pool_id, subject, started_by, created_at, updated_at, last_seq, count, hop, state)
    VALUES ('t-copy', 'pool-a', 'Copy for the retry banner', 'session:s-2', 1792492000000, 1792492600000, 2, 2, 1, 'open');
INSERT INTO agent_messages (id, thread_id, pool_id, sender_ref, sender_kind, sender_name, kind, subject, body, body_sha256, hop, reply_to, to_all, created_at)
    VALUES ('m-1', 't-copy', 'pool-a', 'session:s-2', 'session', 'Retry banner copy', 'question', 'Copy for the retry banner',
            'Should the banner say "Try again" or "Retry payment"?', '0000000000000000000000000000000000000000000000000000000000000001', 0, NULL, 1, 1792492000000),
           ('m-2', 't-copy', 'pool-a', 'user', 'user', 'You', 'answer', 'Copy for the retry banner',
            '"Retry payment" — it says what happens.', '0000000000000000000000000000000000000000000000000000000000000002', 1, 'm-1', 0, 1792492600000);
INSERT INTO agent_docs (id, pool_id, sha256, name, mime, size_bytes, shared_by, shared_by_name, policy, withheld, created_at)
    VALUES ('d-copy', 'pool-a', '0000000000000000000000000000000000000000000000000000000000000003', 'retry-copy.md', 'text/markdown', 412,
            'session:s-2', 'Retry banner copy', 'strip', '[]', 1792493000000);
INSERT INTO agent_asks (id, pool_id, actor_ref, actor_name, action, object_ref, summary, created_at)
    VALUES ('a-task', 'pool-a', 'session:s-2', 'Retry banner copy', 'create_task', 'task:new', 'Make a task: check the banner on a phone', 1792494000000);
INSERT INTO pool_layout (pool_id, member_ref, x, y, placed_at)
    VALUES ('pool-a', 'session:s-2', 140.0, 90.0, 1792491500000),
           ('pool-default:p-shop', 'session:s-1', 320.0, 160.0, 1792491500000);

INSERT INTO tasks (id, pool_id, kind, parent_id, title, body, status, assignee_ref, assignee_name, created_by, created_by_name, created_at, updated_at)
    VALUES ('k-epic', 'pool-default:p-shop', 'epic', NULL, 'Checkout v2', 'The new checkout, end to end.', 'ready', NULL, NULL, 'user', 'You', 1792491000000, 1792491000000),
           ('k-copy', 'pool-a', 'task', 'k-epic', 'Write the retry banner copy', '', 'in_progress', 'session:s-2', 'Retry banner copy', 'user', 'You', 1792491200000, 1792492600000),
           ('k-notes', 'pool-b', 'task', NULL, 'Tidy the meeting notes', '', 'todo', NULL, NULL, 'session:s-3', 'Notes tidy-up', 1792491300000, 1792491300000);
INSERT INTO task_events (at, pool_id, task_id, actor_ref, actor_name, verb, detail)
    VALUES (1792491200000, 'pool-a', 'k-copy', 'user', 'You', 'created', NULL),
           (1792492600000, 'pool-a', 'k-copy', 'session:s-2', 'Retry banner copy', 'claimed', NULL);
INSERT INTO agent_audit (at, pool_id, actor_ref, action, decision)
    VALUES (1792492000000, 'pool-a', 'session:s-2', 'post_message', 'allow');
INSERT INTO agent_events (at, pool_id, actor_ref, verb, object_ref, detail)
    VALUES (1792492000000, 'pool-a', 'session:s-2', 'posted', 'thread:t-copy', NULL);
INSERT INTO pool_pauses (pool_id, paused_at, reason)
    VALUES ('pool-b', 1792500000000, 'You paused it');
