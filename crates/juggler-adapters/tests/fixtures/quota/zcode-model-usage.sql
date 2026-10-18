-- ZCode model_usage fixture (shape per AJ-E604 capture; ids redacted).
CREATE TABLE model_usage (
  rowid_for_doc INTEGER, -- real table uses implicit rowid
  model_id TEXT NOT NULL,
  provider_id TEXT,
  status TEXT NOT NULL,
  started_at INTEGER NOT NULL,
  input_tokens INTEGER NOT NULL DEFAULT 0,
  output_tokens INTEGER NOT NULL DEFAULT 0,
  reasoning_tokens INTEGER NOT NULL DEFAULT 0,
  cache_creation_input_tokens INTEGER NOT NULL DEFAULT 0,
  cache_read_input_tokens INTEGER NOT NULL DEFAULT 0,
  computed_total_tokens INTEGER NOT NULL DEFAULT 0
);
-- Peak row: 2025-10-13 (Monday) 15:00 UTC+8 — inside the weekday 14-18 peak.
INSERT INTO model_usage VALUES (NULL,'GLM-5.3','coding-plan account','completed',1760338800000,480347,479,0,0,479424,480826);
-- Off-peak row: 2025-10-14 02:40 UTC+8 — outside the peak window (50% rate).
INSERT INTO model_usage VALUES (NULL,'GLM-5.3','coding-plan account','completed',1760380800000,1000,100,0,0,0,1100);
-- Unknown model: no published multiplier — no credits, Estimated.
INSERT INTO model_usage VALUES (NULL,'GLM-6-X','coding-plan account','completed',1760380800000,100,10,0,0,0,110);
-- Not completed: excluded entirely.
INSERT INTO model_usage VALUES (NULL,'GLM-5.3','coding-plan account','failed',1760380800000,500,50,0,0,0,550);
