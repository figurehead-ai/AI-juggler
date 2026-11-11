-- v51 (the usage holders keyed by number) undone first: the old table
-- back, empty (its rows are derived and come back on the next read).
DROP TRIGGER IF EXISTS usage_events_unheld_insert;
DROP TRIGGER IF EXISTS usage_events_unheld_move;
DROP TABLE usage_holds;
DROP TABLE usage_keys;
-- v50 (the quota samples' last_seen, a run of one reading as one row)
-- undone first: it sits on everything below.
ALTER TABLE quota_samples DROP COLUMN last_seen;
-- v47 (D-84's apps.added) undone first, then v46: each sits on the last.
ALTER TABLE apps DROP COLUMN added;
-- v46 (the Floor's areas, AJ-E3503): it sits on v45.
DROP TABLE floor_areas;
-- v45 (D-62's pools model, AJ-E3003) undone by hand, for the tests that
-- roll a database back to an older schema and let the migrations run
-- again. The trigger first: it writes to pool_scope.
DROP TRIGGER pools_default_for_new_project;
CREATE TRIGGER pools_default_for_new_project AFTER INSERT ON projects
BEGIN
    INSERT OR IGNORE INTO pools (id, name, kind, project, created_at)
        VALUES ('pool-default:' || NEW.id, NEW.name, 'default', NEW.id,
                CAST(strftime('%s', 'now') AS INTEGER) * 1000);
END;
DROP TABLE pool_talk;
DROP TABLE pool_scope;
DROP INDEX idx_pool_members_ref;
DROP INDEX idx_pool_members_active;
DELETE FROM pool_members WHERE member_ref = 'all';
CREATE UNIQUE INDEX idx_pool_members_active ON pool_members(member_ref) WHERE left_at IS NULL;
ALTER TABLE pool_members DROP COLUMN level;
ALTER TABLE pool_members DROP COLUMN custom_actions;
ALTER TABLE pools DROP COLUMN scope_all;
ALTER TABLE pools DROP COLUMN sealed;
ALTER TABLE pools DROP COLUMN skip_held;
ALTER TABLE tasks DROP COLUMN project;
