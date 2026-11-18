# ADR-0037: One memory and context system for Chat now and the 0.6 bot later: a supersede-only store, live facts over stored ones, one assembler with per-job budgets, and an append-only log

**Status:** Accepted (2026-11-18, the coordinating session), with §1.8's backup rule (D-133's backup ruling) in place of the proposal that memory never travels in a backup. ADR-0036's building waits on this ADR's store and log (Order of work, step 1).
**Date:** 2026-11-18
**Deciders:** Opus (design); the coordinating session (review); the maintainer (D-133, and the open question at the end)
**Serves:** D-133 as extended (one memory and context system for 0.5 Chat and the 0.6 bot); D-130 as clarified (the read-only jobs' severity rules; the bot's own, stricter harness); D-132 and [ADR-0036](./0036-digest-and-watches.md) (the digest and watches, which live on this store, write this log and use this assembler); [ADR-0034](./0034-ai-jobs-router.md) (the router, the queue, Chat v1, its look-ups, §6.4's budget, §6.7's history); [ADR-0033](./0033-feed-summary-engines.md) (the guard, agent text as untrusted data); [ADR-0035](./0035-remote-config.md) (flags); [ADR-0019](./0019-extension-architecture.md) (features as modules); [ADR-0023](./0023-local-cache.md) (the cache registry); [ADR-0030](./0030-storage-concurrency.md) (one writer lock, short holds); [ADR-0016](./0016-backup-archive-format.md) (the backup's scrub); D-14 (secrets hidden); D-52 (OS seams); D-117 (levels from facts); D-123 (the bot in 0.6); D-126 (rich settings); D-127 (never show what can't be used); E38 ([`E38-chat.md`](../tasks/E38-chat.md), AJ-E3819)

## Context

- **The ruling ([D-133](../review/decision-log.md), 2026-11-18).** The maintainer: Chat in 0.5 "should have a memory, short and long term, and being able to remember what user has previously asked the chat. But we have to be very smart with context." D-133 sets short-term memory (the conversation and a rolling summary, within a fixed budget), long-term memory (what the user asked, their standing requests and watches, their stated preferences, each with where and when it came from, local, never synced, visible and clearable), entity facts read live rather than remembered, and a context budget that fits Apple's model (about 4K tokens on macOS 26, 8K on 27).
- **The extension, the same day.** "Bot is not going to be a separate feature it is going to be an extension of our existing chat… We don't want to be constantly rewriting our memory and context system." So: one store and schema from day one (scope, kind, authority, provenance, confidence, as-of, superseded-by on every record; agent text never without "untrusted" provenance), one context assembler with pluggable sections and per-job budget data, one retrieval interface, an append-only event and audit log from 0.5, versioned migrations so 0.6 only adds, and the bot's own parts designed but not built. (D-133's extension text first named this ADR-0036; the decision log was corrected to ADR-0037 in 162488e3c.)
- **The acceptance rule ([D-130](../review/decision-log.md) as clarified).** For 0.5's read-only jobs, *critical* means fabrication: an invented fact, a contradiction of the facts, a confident answer to another question, an obeyed planted order. At most 1 % may reach the user, none systematic; omissions are tolerated up to about 10 %. For memory this means: **a remembered thing must never be invented or outdated** (D-133 point 5), and forgetting one is an omission, not a failure. The 0.6 bot, which decides and acts, gets its own much stricter harness; these criteria never carry over to it.
- **What ADR-0036 needs from this ADR** (it defines none of it): the **store** for digests (§1.9 there) and watches (§2.7), with scope, authority, provenance, "untrusted" marking, confidence, as-of and superseded-by; the **append-only log** for watch changes and fires, alerts sent, held and consolidated, digests made with their verdicts, and every "this is wrong"; the **assembler** for the `Digest` and `WatchDraft` jobs, with its budget table per engine tier and a `memory` section that is empty in 0.5.
- **What AI Juggler has today, which this design rests on:**
  - **One SQLite database** in `juggler-core` (`storage/mod.rs`): forward-only migrations as an ordered array of SQL batches (`MIGRATIONS`), the schema version in `PRAGMA user_version`, each migration in its own `IMMEDIATE` transaction, `SchemaTooNew` when an older build meets newer data. **release/0.5 is at V52; release/0.4.6 is at V53** (`pools::LEAD_NEXT_MIGRATION`, AJ-E3011), and no other branch goes past V52. Before an upgrade a consistent copy is kept beside the file (`<file>.before-v<N>`, the newest 2 kept) so a user can go back.
  - **FTS5 is in use** (the transcript index, V27, contentless; the agents' messages, V34, external content with triggers), and so is an **append-only table guarded by triggers** (`agent_events`: no update, no delete).
  - **The backup** (ADR-0016, `backup.rs`) snapshots the database and deletes `MACHINE_ONLY_TABLES` from the snapshot before export; a restore keeps this Mac's own rows of those tables.
  - **One writer lock with short holds** (ADR-0030).
  - **Chat v1** (ADR-0034, rescoped 2026-11-18 in AJ-E3808 and AJ-E3809): the model understands the question and picks read-only look-ups, and the panel shows the found records with links; written answers ship only when their harness passes. Its follow-up context is the last two exchanges (§6.7 there). Its history was placed in the registered cache (AJ-E3808), which is not built yet.
- **The research pass D-133 asked for** looked at another agent system's memory and context-assembly design (notes kept outside the repo). What carries over, in general terms: memory kinds with their own rules; tiers with promotion by repetition and decay before deletion; supersede rather than overwrite, with one current record per subject; a freshness check so stale derived facts are never shown as current; ranking by keyword relevance × priority × recency half-life, touched on use; a budgeted assembler with a never-truncated core that fails loudly; one fact, one slot; small per-turn receipts that record why each item was chosen, without prompt bodies; a confidence gate on inferred memories; small maintenance batches. What does not fit 0.5, and where some of it returns for the bot, is in §8.3.

## Decision

### 1. The store

One store, in AI Juggler's existing database, written only through one service (`features/ai/memory/store.rs`, the **write gate**). Every record has the same columns; a record's *kind* decides its payload, its rules and who may write it.

#### 1.1 Kinds

| Kind | Built in | What it holds | Authority | Written only by | Subject key |
|---|---|---|---|---|---|
| `preference` | 0.5 | a stated preference from a closed set of slots: `alias` (the user's own name for a session, App, project or pool: "the api agent") and `focus` (what the user cares about most, per scope) | user; model only through §2.4's gate | `features/ai/memory/` (Chat's remember flow, Settings) | `alias:<entity kind>:<id>`, `focus:<scope>` |
| `note` | 0.5 | the user's own words after "remember…" | user; model only through §2.4's gate | `features/ai/memory/` | `note:<hash of the normalised text>` |
| `standing_request` | 0.5 | a confirmed watch; the payload is ADR-0036's typed `Watch` | user | `features/watches/` (its three commands) | `watch:<WatchId>` |
| `asked` | 0.5 | a question the user asked, in their words, with the entities it resolved to | app (recorded by code) | `features/ai/chat/` | `asked:<normalised question hash>:<entity ids>` |
| `answer` | 0.5 | what was shown for that question: the cites, and each cited row's title, state and level *as they were then* | app | `features/ai/chat/` | `answer:<the same suffix as its asked>` |
| `digest` | 0.5 | ADR-0036 §1.9's digest: its rollup and AI items, each item with its own authority and cites | app for the rollup; model for AI items; the record takes the weakest | `features/digest/` | `digest:<window end>` |
| `policy` | 0.6, reserved | a rule the user set for the bot | user | the bot's rules editor | `policy:<scope>:<slot>` |
| `decision` | 0.6, reserved | a bot decision and its grounds | bot | the bot | `decision:<id>` |
| `action` | 0.6, reserved | an act the bot took, through the existing commands | bot | the bot | `action:<id>` |
| `outcome` | 0.6, reserved | what came of an action, observed from AI Juggler's facts | app | the outcome observer | `outcome:<action id>` |

**There is no "fact" kind.** Sessions, Apps, projects, usage, runway, states and levels are never stored as memories (§3). The `answer` kind keeps a snapshot only to show "then" beside "now".

**Each kind has one writer.** At start each owning module receives a `KindWriter<K>` for its kinds and no other; the write gate refuses a write without the right one. This makes ADR-0036's "a watch is written only by its three commands" a property of the types, and the architecture lint checks that `features/watches/` alone imports the watch writer.

#### 1.2 The columns (migration V54)

```sql
CREATE TABLE memory_records (
    id            TEXT PRIMARY KEY,          -- one id per version (a ULID)
    subject_key   TEXT NOT NULL,             -- §1.1; the unit of supersede and forget
    version       INTEGER NOT NULL,          -- 1, 2, … per subject key
    kind          TEXT NOT NULL,             -- §1.1, checked in Rust; reserved kinds refused by a trigger (§1.7)
    scope_kind    TEXT NOT NULL,             -- global | project | pool | session
    scope_id      TEXT,                      -- NULL for global
    tier          TEXT NOT NULL,             -- short | mid | long (§2)
    priority      INTEGER NOT NULL,          -- 0 low, 1 normal, 2 high, 3 pinned
    authority     TEXT NOT NULL,             -- user | app | model | agent | bot (reserved)
    untrusted     INTEGER NOT NULL,          -- 1 when any part holds agent-written text
    source_kind   TEXT NOT NULL,             -- chat_turn | watches_view | settings | digest_job | note_job | maintenance
    source_id     TEXT,                      -- the turn, the window, the view action
    confidence    REAL NOT NULL,             -- 0..1; 1 for user and app
    as_of_ms      INTEGER NOT NULL,          -- when it was said, asked or observed
    source_rev    TEXT,                      -- answer: a hash of the cited rows' state then (§3)
    supersedes    TEXT REFERENCES memory_records(id) DEFERRABLE INITIALLY DEFERRED,
    superseded_by TEXT REFERENCES memory_records(id) DEFERRABLE INITIALLY DEFERRED,  -- NULL: current
    text          TEXT NOT NULL,             -- the label shown and indexed, at most 280 characters
    payload       TEXT NOT NULL DEFAULT '{}',-- typed per kind (Watch, digest items, cites), at most 16 KB, valid JSON
    use_count     INTEGER NOT NULL DEFAULT 0,
    last_used_ms  INTEGER,
    expires_ms    INTEGER,                   -- digests, short-tier rows
    created_ms    INTEGER NOT NULL,
    UNIQUE (subject_key, version)
);
CREATE UNIQUE INDEX memory_one_current ON memory_records(subject_key) WHERE superseded_by IS NULL;
CREATE INDEX memory_by_kind_scope ON memory_records(kind, scope_kind, scope_id) WHERE superseded_by IS NULL;
CREATE INDEX memory_by_tier_use ON memory_records(tier, last_used_ms);
CREATE TABLE memory_entities (               -- the entities a record names, for recall and for "forget everything about web"
    record_id   TEXT NOT NULL REFERENCES memory_records(id) ON DELETE CASCADE,
    entity_kind TEXT NOT NULL,               -- session | app | project | pool | account
    entity_id   TEXT NOT NULL,
    PRIMARY KEY (record_id, entity_kind, entity_id)
) WITHOUT ROWID;
CREATE INDEX memory_entities_by_entity ON memory_entities(entity_kind, entity_id);
CREATE VIRTUAL TABLE memory_fts USING fts5(
    text, content = 'memory_records', content_rowid = 'rowid',
    tokenize = 'unicode61 remove_diacritics 2'
);
INSERT INTO memory_fts(memory_fts, rank) VALUES ('secure-delete', 1);
-- Triggers: a current, trusted, retrievable row is indexed; superseding or deleting it removes its entry.
-- Triggers: no update of id, subject_key, version, kind, scope, authority, untrusted, source, as_of, text or payload;
--           superseded_by may go from NULL to a value once; tier, priority, use_count, last_used_ms and expires_ms are
--           the maintenance columns (§2).
```

Plus the conversation tables (§1.6) and the log (§7), in the same migration.

**As built (AJ-E3819 step 1, `juggler-core/src/storage/memory/schema.rs`),** three changes to the sketch above, each for a reason found while building:
- **An explicit `seq INTEGER PRIMARY KEY`, with `id TEXT NOT NULL UNIQUE`,** and `memory_fts` keyed by `seq`: `VACUUM` (the backup's snapshot and scrub) may renumber an implicit rowid, which would point the external-content index at other rows.
- **`subject_id`, a random id per subject** shared by its versions: the log, its sidecars (§7.1's `subject_key` column is `memory_subject` and holds this id) and the forget tombstones (`memory_forgotten`, §1.5) name a subject by it, never by its key. A note's key is a hash of its words, so a key kept in the 90-day append-only spine would outlive the forget as a fingerprint.
- **`supersedes … ON DELETE SET NULL`:** removing an old version (the 20-version bound, the cloud scrub, §1.8) cuts the newer version's link to it, the one change the content trigger allows on that column.

**`untrusted` is set by type, not by care.** Payload types hold agent-written text only as `Untrusted<String>` (a session's name inside a watch, an excerpt inside a digest); serialising a payload that contains one sets the column, and the write gate refuses a record whose `authority` is `agent` with `untrusted = 0`. Untrusted rows are never indexed in `memory_fts` and never retrieved into a prompt (§4); the one place their text reaches a model is the digest's own guarded `agent_words` section (ADR-0036 §1.2), which reads the journals, not this store.

#### 1.3 The write gate

Every write passes, in order, in one short hold (ADR-0030), with the record's log event (§7) in the same transaction:

1. **The writer** holds the `KindWriter` for the kind (§1.1).
2. **The kind is built:** a reserved kind or the `bot` authority is refused in 0.5 (and by a trigger, §1.7).
3. **Authority and supersede** (§1.4): a write may supersede the current row of its subject key only when its authority is at least the current row's (user > app > model > agent); the user may supersede anything. A lower-authority write against a higher current row is refused and logged (0.6 adds a review path, §8.2).
4. **No secrets:** the D-14 secret-hiding pass runs over `text` and every string in `payload`. A hit refuses the record (the user sees "That looks like a key or password, so I won't keep it.") rather than storing a masked copy. Conversation turns (§1.6) store the question with the secret masked.
5. **Bounds:** `text` at most 280 characters, `payload` at most 16 KB, at most 20 versions kept per subject key (§1.4), and the store's total within §2's caps.
6. **Untrusted by type** (§1.2).
7. **The confidence gate** for model-authored rows (§2.4).

#### 1.4 Supersede, never overwrite

- **One current row per subject key,** enforced by the partial unique index `memory_one_current`. A change writes version *n*+1 with `supersedes` set and sets the old row's `superseded_by`, in one transaction (the deferred foreign keys let both point at each other).
- **No row's content changes after it is written:** triggers refuse updates of the content columns. A user's edit in Settings, an edited watch (ADR-0036 §2.7), a restated preference: each is a new version.
- **History is kept and bounded:** the newest 20 versions of a subject key; older superseded versions are deleted by maintenance (§2.3). Retrieval and the views read only current rows; Settings shows a row's history on request ("changed 2 times").
- **Contradictions resolve by rule, not by a model:** "call the api session *backend*" after "call it *api*" supersedes `alias:session:<id>`; nothing has to decide which of two coexisting notes is true.

#### 1.5 "Forget" really deletes

Forgetting a record forgets its **subject**: every version of that subject key. One transaction, run with `PRAGMA secure_delete = ON` (SQLite overwrites the freed cells and pages with zeros) and `recursive_triggers` on for that hold, then a `wal_checkpoint(TRUNCATE)`. As built after the independent security review (`docs/security/review-memory-forget-step1.md`, its highs and mediums fixed):

1. the rows of every version, and their `memory_entities` rows;
2. their `memory_fts` entries, removed at once (FTS5's `secure-delete` option); **then the index is rewritten** (`optimize`) in the same transaction, because `secure-delete` leaves each page's separator in `memory_fts_idx`, the prefix of the page's first term, so a forgotten word that started a page would otherwise survive almost whole (the review's H1);
3. **the Chat turns it came from or was used in** (its versions' `source_id`, and `chat_turn_memory`, the link each turn's receipt writes): their words are redacted (the question emptied, the shown answer cleared, the turn marked `redacted`), and a conversation whose title was taken from one (`title_turn`) is renamed from its first turn left, else "Earlier conversation" (H2). The turns stay as rows, so a conversation keeps its shape and the ledger (§5.4) counts what's left;
4. the log's text sidecars about it: by its random `subject_id` (`memory_subject`, §7.1), by an event naming one of its versions, and by an event naming one of its turns. The spine's rows keep only ids and codes, so they say "a memory was forgotten" and nothing of what it said;
5. for an `asked` subject, its `answer` subject too; "Forget this conversation" removes the conversation's turns as well;
6. a **tombstone** (`memory_forgotten`: the subject's random id and when, never its key or text, on this Mac only).

**The same deletion reaches every copy AI Juggler keeps, or the copy keeps no memory at all:**
- **AI Juggler's own copies hold no memory:** the copy before a migration (`<file>.before-v<N>`), the backup before an update, and the copy before a restore are emptied of memory, conversations and sidecars, with `secure_delete` on, before they are kept. A later forget couldn't reach them, so they never hold what it would need to reach; going back costs the memory, which is the safe direction.
- **The copy before a restore** (`juggler.sqlite.before-restore-<stamp>`) keeps no memory however the restore ends, and is deleted after a day (at start and at each restore), H3. A restore that fails its count check keeps its copy under its own name, as the way back, unpruned.
- **A restore** applies this Mac's tombstones to the staged backup **before** it replaces anything (M1), then again after, as a check, so a backup made before a forget brings nothing forgotten back, turns included.
- **Backups** (ADR-0016): by §1.8's rule. Each is a snapshot taken now, so a forgotten subject is in none made after the forget; its scrub builds the FTS index fresh from the rows left and compacts the file, so neither a freed page nor an index separator carries it.
- **Backup files the user exported before the forget** are theirs: AI Juggler can't reach a file once it's saved, and such files don't rotate. The backup page and the forget sheet say so plainly (`features::ai::memory::FORGET_AND_BACKUPS`: "A backup file you exported before still holds them until you delete that file"). Scrubbing them was rejected: they may be anywhere (another disk, a cloud folder), and rewriting a user's archive behind their back could break the one copy they trusted.
- **Out of reach by design:** the file system's own copies of freed blocks (APFS snapshots, Time Machine). A WAL fold blocked by another connection's open read leaves the frames until the next full checkpoint (`Forgotten::wal_folded`; maintenance retries it, step 3).

**Clear actions** (Settings › AI › Memory, §9) are forgets over sets of subjects: "Forget everything about web" (by `memory_entities`), "Forget this conversation", "Forget all memory".

#### 1.6 Conversations: the short tier's storage (amends ADR-0034 §6.7's location)

```sql
CREATE TABLE chat_conversations (id TEXT PRIMARY KEY, started_ms INTEGER NOT NULL, last_ms INTEGER NOT NULL,
                                 title TEXT NOT NULL, has_agent_words INTEGER NOT NULL DEFAULT 0);
CREATE TABLE chat_turns (id TEXT PRIMARY KEY,
                         conversation_id TEXT NOT NULL REFERENCES chat_conversations(id) ON DELETE CASCADE,
                         seq INTEGER NOT NULL, at_ms INTEGER NOT NULL,
                         question TEXT NOT NULL,            -- the user's words, secrets masked, at most 600 characters
                         outcome TEXT NOT NULL,             -- rows | answered | dont_know | out_of_scope | watch_drafted | watch_set | remembered | stopped
                         shown TEXT,                        -- a written answer, when one was shown; NULL in the look-up view
                         cites TEXT NOT NULL DEFAULT '[]',  -- the ids the shown result rests on
                         entities TEXT NOT NULL DEFAULT '[]', -- the entities the question resolved to
                         has_agent_words INTEGER NOT NULL DEFAULT 0,
                         UNIQUE (conversation_id, seq));
```

- **History moves from the cache to the store.** ADR-0023 defines the cache as rebuildable, and a user's conversations are not; and forgetting must reach turns, the records derived from them and the summary built from them in one transaction. What ADR-0034 §6.7 and AJ-E3808 promise stays: on this Mac only, at most 20 MB with the oldest conversations removed first, listed by first question and date and paged, never raw rows, tool calls, rejected drafts or transcripts. The memory module registers a `LocalCache` entry, "Chat history", only so Settings › Local cache shows its size and **Clear cache still clears it** (AJ-E3808 point 2).
- **What clearing history removes:** Clear chat history and Clear cache delete every conversation, and every record whose only source is one of those turns and that the user didn't state (`asked`, `answer`, and model-noted rows still in the short tier). The user's own notes, preferences and watches stay, and the confirmation sheet names them ("Your 4 notes and 2 watches stay. Forget them in Memory."). Removing the oldest conversations for the 20 MB cap does **not** remove the `asked` records made from them: that is what long-term memory is for.
- **Last messages off** deletes conversations' turns that carry agent words (`has_agent_words`, as ADR-0034 §6.7 says) and every `untrusted` record, digests included (ADR-0036 §1.9).
- **Follow-ups get more than two exchanges** (amends ADR-0034 §6.7's "never older chats"): the recent turns, the conversation ledger (§5.4) and recalled memories, each within its share (§5.3). A follow-up still runs its own look-ups; old rows are never reused as facts (§3).

#### 1.7 One migration in 0.5, and 0.6 only adds

- **The tables take V54**: the next free version after release/0.5 (V52) and release/0.4.6 (V53, which reaches release/0.5 when 0.4.6 merges). **The runner needs contiguous versions** (a version is its index in `MIGRATIONS` + 1), so V54 is built and tested on release/0.5 but listed only by the 0.4.6 → 0.5 merge, right after V53; the test `v54_waits_for_v53` fails the moment V53 arrives, as the reminder, and `scripts/migration_clash.py` (in `just verify-static`) fails if another branch claims a version this one does. One migration holds every 0.5 table of this ADR: `memory_records`, `memory_entities`, `memory_fts` with its triggers, `chat_conversations`, `chat_turns`, `ai_log`, `ai_log_text`, `ai_log_prune` and their triggers. The number is checked again across every local and remote branch when AJ-E3819's first step is built, as D-numbers are; if a newer migration has landed by then, this one takes the next free number.
- **0.6 adds and never rewrites:** new tables (`memory_links`, §8.1), new indexes and triggers, new kinds and log streams (the `kind`, `authority` and `stream` columns are text, checked in Rust, so no table rebuild is needed), and dropping the 0.5 trigger `memory_reserved_kinds`, which refuses the reserved kinds and the `bot` authority until then. No 0.5 column changes meaning.
- **A forward-only test** opens a V53 fixture database, migrates it, and checks the new tables are empty and every existing table is byte-identical.

#### 1.8 Backups: settings travel, Chat's words stay on this Mac's own backups (D-133's backup ruling)

The coordinating session's call on this ADR's open question (2026-11-18), recorded in [D-133](../review/decision-log.md). Memory is split by what it is; the split is by row, decided by the record's kind and authority, and lives in one place (`storage::memory::backup`), which the backup's scrub calls for every destination:

| What | The user's export | AI Juggler's own copies (before an update, a migration, a restore) | The encrypted cloud backup (when it is on) |
|---|---|---|---|
| **Stated settings:** a `standing_request`, or a `preference` that names an entity in a label of at most 60 characters; by the user, trusted, from a setting's source (Settings, the Watches view, a confirmed Chat draft); with its `memory_entities` rows | yes | no | yes |
| **Chat's words and what was derived from them:** every other record (`note`, `asked`, `answer`, `digest`, anything `model`- or `agent`-authored, an earlier model-noted version of a preference), `chat_conversations`, `chat_turns`, the log's sidecars | yes | no | **never** (D-121: Chat's content never leaves the Mac) |
| **The log's spine** (ids and codes) | yes | yes | no |
| **Forget tombstones** (`memory_forgotten`) | no: this Mac's own, kept on restore | no | no |

- **A note is Chat's words.** The user stated it, but in free words typed into Chat, which can hold anything; so it travels as chat history does: local backups only. Watches and preferences are typed settings: a preference must name an entity in a short label, and the write gate refuses any other shape (`PreferenceShape`), so the remember flow keeps free words as a note (the review's M3).
- **Who may claim a setting** (the review's M2): a `KindWriter` can't be made outside the store; `Writers::issue` hands each owner its one writer at start, and the gate refuses a kind, authority or source it doesn't cover. A label holding agent words is `RecordText::Untrusted`, by type, and an untrusted record is never a stated setting.
- **A stated setting goes with its history, minus what isn't a setting:** an earlier, model-noted version of a preference the user then restated stays out of the cloud copy, and the link to it is cut (`supersedes` set to NULL, the one change the content triggers allow, §1.4).
- **The cloud copy is a filtered export:** the stated settings are read out, every memory table is emptied, and they are written back as new rows into an empty index (only current, trusted rows are indexed), so nothing of the source file's memory or its index structure travels; the log's watermarks move past what was removed (as Clear AI activity does, §7.3), so the copy's own triggers hold; and the backup's compaction rebuilds every page. The user's export gets the same fresh index.
- **Forget reaches the next backup** (§1.5): the backup page and the forget sheet say "Forgotten memories are left out of every backup made after you forget them. A backup file you exported before still holds them until you delete that file: AI Juggler can't reach a file once it's saved. AI Juggler's own copies, kept before an update or a restore, hold no memories at all." Restoring an older export on this Mac doesn't bring them back (the tombstones).
- **Restoring a cloud backup:** what it carries replaces the stated settings; whether this Mac's own Chat history is kept beside them, rather than replaced by the backup's (which has none), is decided with the backup page (AJ-E3819 step 4), and until then a restore replaces everything, as it does today.

### 2. Tiers, promotion, decay and maintenance

#### 2.1 The three tiers

| Tier | What lives there | How it ends |
|---|---|---|
| **Short** | the conversation (§1.6), and model-noted rows not yet confirmed or reused | with the conversation, or after 7 days unless promoted |
| **Mid** | `asked` and `answer` rows; digests (until they expire at 30 days) | idle 30 days: priority decays to low; low and idle 30 more days: deleted (§1.5's deletion, without the user's sheet) |
| **Long** | what the user stated (`note`, `preference`), active watches, pinned rows, and what was promoted | never deleted automatically; only by the user's forget, or as an old superseded version (§1.4) |

*Idle* means not used: not included in an assembled context, not shown by a look-up, not restated.

#### 2.2 Promotion by repetition

- **Short → mid:** a model-noted row is restated (the same subject key written again) or used twice, or the user taps **Keep** on it.
- **Mid → long:** the same `asked` subject on 3 different days within 30 days, or used in 5 assembled contexts, or pinned by the user. Its priority rises one step at promotion.
- **An explicit "remember this" is long at once** and is always accepted (§2.4).
- **A watch is long while active.** When it ends (ADR-0036 §2.3's lifetimes), it becomes mid and ages like an `asked` row, so "what did I ask you to watch last week?" can still be answered.

#### 2.3 Maintenance in bounded idle batches

`memory::maintain(now)` is the only place tiers, priorities and expiry change:

- **When:** 2 minutes after start, then at most hourly, only while no AI job is running or queued and the Mac is not battery-critical (ADR-0036 §1.6's reading). Never during a turn.
- **How much:** batches of at most 200 rows, each one short writer hold (ADR-0030), at most 2,000 rows a pass; it stops at once when a job arrives and resumes next time.
- **What:** promote (§2.2); decay idle mid rows to low; delete low idle mid rows and expired short rows and digests; delete superseded versions past the 20th; enforce the caps; apply the touches queued after each turn (§4.3); prune the log by its watermarks (§7.3).
- **Caps:** records and their index at most 20 MB (AJ-E3819); conversations at most 20 MB (§1.6). Over the cap, the oldest low mid rows go first, then the oldest mid; long rows are never removed for space, and Settings says so when they alone fill it.
- Each pass writes one `memory.maintained` log event with counts only.

#### 2.4 The confidence gate, and "remember this"

- **Explicit, always accepted.** A message whose words match the remember cues ("remember", "don't forget", "from now on", "always", "call … …", in English and German in 0.5) is handled by code: the memory's text is the user's own words after the cue, trimmed; the kind and slot come from the cue (`call X Y` is an `alias`); authority user, confidence 1, tier long. Chat shows the app's echo: "I'll remember: “the api project matters most”." with **Edit** and **Undo**. No model is involved, so it works with no engine.
- **Inferred, only through the gate.** A model may propose a `note` or `preference` from a message that had no cue (job `MemoryNote`, §5.3), filling a guided shape: kind, slot, the span of the user's message it rests on, and a confidence. It is written only when all hold: the span is a verbatim part of the user's message (extractive; the agents' text is never a source); the slot is in the closed set; confidence at least 0.8; and nothing of higher authority holds the subject key. It is written in the short tier, authority model, and shown once, quietly ("Noted: you call web *the api agent* · Keep · Undo"). Below the gate it is dropped and counted.
- **The inferred path is a model job, so it ships under D-130:** opt-in Beta behind `memory_note_beta` (ADR-0035, `OffOnly`), only after its harness run passes (§10.2). Until then only explicit memories and what code records exist.

### 3. Live facts over stored facts

- **Entity facts are read at query time.** Sessions, Apps, projects, pools, tasks, usage and runway come from the owning modules' read services, through Chat's look-ups (ADR-0034 §6.2) and the digest's and watches' section providers. No memory kind holds them (§1.1).
- **A remembered answer is a pointer and a snapshot, never a fact source.** An `answer` row holds the question, its cites, and per cite the title, state and level then, with `source_rev` (a hash of those states) and `as_of`. It is shown by code, never by a model:
  - "You asked this 2 days ago." Then, per cite, the live row read now. When the live state equals the snapshot, one line. When it differs, both: "*then:* stuck · *now:* finished", the live one in normal ink, the old one in secondary ink with its age. When the entity is gone: "no longer here".
  - A remembered answer's content **never enters a model's context.** An `asked` row may (§5.3's `recalled` section), as "On Nov 16 you asked: “is web stuck?” (about web [m2])": the user's question and the entities, no states.
- **One fact, one slot, and live wins.** Every assembled item carries an optional fact key (`entity:<kind>:<id>:<attribute>`, or the record's subject key). The assembler keeps the first item per fact key in section precedence order, and live sections come before memory sections (§5.2). A memory that would restate an entity's attribute is dropped in favour of the live row, with the receipt reason `live_won`.
- **A memory ref can back only what the user said or asked.** When written answers ship, ADR-0034 §7.2's check gains one rule: a sentence citing a memory ref may not carry a state, level, count or outcome word (step 3's lists); those need a live row. The age phrase ("you asked 2 days ago") is composed by code from `as_of`, never by the model.
- **Age is always shown where it matters:** a remembered answer always carries its age; any memory used in an answer or a look-up result that is older than 7 days carries it ("you told me 2 months ago"); Settings shows every row's age.

### 4. Retrieval: one trait

#### 4.1 The trait

```rust
pub trait Retriever: Send + Sync {
    /// Pure over the store's read handle; no writes (touches are queued, §4.3).
    fn recall(&self, q: &RecallQuery<'_>, now_ms: i64) -> Result<Vec<Recalled>, MemoryError>;
}

pub struct RecallQuery<'a> {
    pub job: JobKind,
    pub text: &'a str,                 // the user's own words; never agent text
    pub entities: &'a [EntityRef],     // resolved by code from the question, aliases included
    pub scopes: &'a [Scope],           // global, plus the project, pool or session in play
    pub kinds: KindSet,                // from the job's budget row
    pub min_authority: Authority,      // 0.5: model; agent-authored rows are never recalled
    pub limit: u16,
}

pub struct Recalled {
    pub record: RecordId, pub version: u32, pub fact_key: FactKey,
    pub score: f32, pub reason: RecallReason,  // AlwaysOn | Matched | SameEntity
}
```

Callers (the `always_on` and `recalled` section providers, the `memory` look-up) see only this trait. **v1 is `KeywordRetriever`.**

#### 4.2 v1: keyword × priority × recency, no embeddings

1. **Always-on pass:** current long-tier `preference` rows in the query's scopes (the user's aliases and focus), with no similarity needed, capped by the `always_on` section's share. Reason `AlwaysOn`.
2. **Matched pass:** `memory_fts MATCH` on the question's words, each word quoted (the transcript index's rule, so user words are never FTS syntax), joined to current, trusted rows of the job's kinds, scopes and authority; plus rows whose `memory_entities` name an entity of the question (reason `SameEntity`, relevance floor 0.5).
3. **Score** = relevance × priority weight × 0.5^(age ÷ half-life):
   - relevance: *b* ÷ (1 + *b*), where *b* = −`bm25(memory_fts)` (FTS5's BM25 is negative, better is lower), so 0 < relevance < 1; **at least 0.25 for a word match** (as built: FTS5 gives a word found in half the rows or more an IDF of almost 0, so in a small store, the usual case, a real match would otherwise score nothing);
   - priority weight: low 0.5, normal 1.0, high 1.5, pinned 2.0;
   - age in days since the later of `last_used_ms` and `as_of_ms`; half-life 14 days for `asked`, 90 for `note` and `preference`, 30 for an ended watch.
4. **One per fact key;** below 0.05 dropped; the top `limit` returned.

Under 10 ms on 5,000 records (AJ-E3819 gives the whole assembly 50 ms).

#### 4.3 Touch on use

A record is *used* only when it is included in an assembled context or shown in a look-up result, not merely recalled. The turn queues its touches (`use_count + 1`, `last_used_ms`), and maintenance's first batch, or the next writer hold after the turn, applies them. Assembly never writes.

#### 4.4 Later, behind the same trait

- **Embeddings:** an on-device embedder's vectors beside FTS5, fused with BM25 by reciprocal rank fusion. Added only if the recall harness (§10.2) shows keyword recall missing what users ask, and never by a download without the user's click (D-131).
- **Entity relations:** recall widened through AI Juggler's own `relations` tables (a pool's members, a project's sessions), no graph store.
- Each is a new `Retriever` impl chosen per job by its budget row; the trait's contract tests run over every impl, so callers never change.

### 5. One context assembler

#### 5.1 Sections and providers

`features/ai/context/` owns the assembler. Modules register **section providers** (ADR-0019: features as modules); a job's budget row says which sections it has, and an unlisted provider never runs for that job.

```rust
pub trait SectionProvider: Send + Sync {
    fn id(&self) -> SectionId;
    fn provide(&self, cx: &AssemblyCx<'_>, cap: u32) -> Result<Vec<Item>, ProvideError>;
}

pub struct Item {
    pub text: String,              // already framed and guarded by the provider (agent text quoted, ADR-0034 §7.1)
    pub tokens: u32,
    pub rank: f32,                 // the provider's own order; higher kept first
    pub fact_key: Option<FactKey>,
    pub source: ItemSource,        // Record(id, version) | Cite(kind, id) | Turn(id) | Fixed
    pub trust: Trust,              // App | User | Untrusted (only from guarded providers)
}

pub struct BudgetRow {
    pub job: JobKind,
    pub window: u32,               // the engine's context: 4,096 | 8,192 | 32,768 (0.6) …
    pub sections: &'static [SectionBudget],
    pub output: u32, pub margin: u32,
    pub retriever: RetrieverId,    // KeywordRetriever in 0.5
}

pub struct SectionBudget {
    pub id: SectionId,
    pub core: bool,                // never truncated
    pub max: u32, pub floor: u32,  // the share, and what it keeps before it may be dropped whole
    pub drop_order: u8,            // 1 is dropped first
    pub group: Option<GroupId>,    // sections sharing one cap (the digest's agents and events)
    pub precedence: u8,            // for one-fact-one-slot: live sections first
}

pub fn assemble(job: JobKind, card: &EngineCard, cx: &AssemblyCx<'_>, reg: &Registry)
    -> Result<Assembled, AssemblyError>;
pub enum AssemblyError { CoreOverflow { need: u32, room: u32 }, NoBudgetRow, Provider(SectionId) }
```

`Assembled` holds the prompt parts in the job's fixed frame order and the receipt (§6).

#### 5.2 How it assembles

1. **The row:** the job's row with the largest `window` not above the routed engine's `context_tokens` (ADR-0034 §2). None: `NoBudgetRow`, and the job's ladder runs.
2. **The core first, never truncated:** the core sections are provided whole. If core + output + margin exceeds the window: **`CoreOverflow`**, logged as `assembly.core_overflow`, a failed test in every build, and in a user's build the job's own ladder (the found rows, the rollup, the form). Core content is never cut to fit. Its size is bounded at the source: the question box takes at most 600 characters (the smallest row's current-turn share, with a counter in the panel), and each core section's largest possible size is a build-time test per row.
3. **The rest, by share:** each droppable section gets its items up to `max`, kept by `rank`. A share a section didn't use is lent to the sections dropped last, up to their own `max`.
4. **One fact, one slot:** items are walked in `precedence` order; an item whose fact key is already held is dropped (`live_won` when the holder is live).
5. **Fit, dropping lowest first:** if the total (counted as §5.5) is over the window, items are removed from the section with the lowest `drop_order`, lowest rank first, down to its floor, then the next section; then sections are dropped whole in the same order. The core is never touched.
6. **If the engine still reports `TooLong`** (its own count disagrees), the job reassembles once at 85 % of every droppable share; a second `TooLong` runs the job's ladder.

Pure over the provided items: the fitting takes under 5 ms; the whole assembly, providers and retrieval included, under 50 ms (AJ-E3819).

**Budgets are compiled data, not remote** (`features/ai/context/budgets.rs`). A change to a row is a prompt change: it bumps the job's prompt version and re-runs that job's harness, as ADR-0034 §3.2 requires. Build-time tests check every row: core maximum + Σ droppable `max` + output + margin ≤ window, every listed section has a provider, every group's cap fits.

#### 5.3 Budget rows in 0.5, and the shape of the larger ones

**`ChatAnswer`** (the look-up view now, written answers when they pass; replaces ADR-0034 §6.4's table, whose figures it keeps for instructions, rows and answer):

| Section | Core | Drop order | 4,096 (Apple, macOS 26) | 8,192 (Apple on 27; the local GGUF model) |
|---|---|---|---|---|
| `instructions` (with the look-up definitions, eight, and the answer shape) | yes | — | about 1,000 | about 1,000 |
| `always_on` (aliases and focus, at most 5) | yes | — | at most 150 | at most 250 |
| `current_turn` (the question) | yes | — | at most 150 | at most 200 |
| `recalled` (memories, §4) | | 1 | at most 250 | at most 600 |
| `ledger` (the conversation ledger, §5.4) | | 2 | at most 150 | at most 300 |
| `recent_turns` (the last exchanges, newest kept) | | 3 | at most 350 | at most 800 |
| `rows` (look-up rows, reserved; the tool host cuts rows to what remains, newest first, `more` set) | | 4 | at most 1,650 | at most 4,600 |
| Output | | | 200 | 200 |
| Margin | | | 196 | 242 |

**`WatchDraft`** (ADR-0036 §2.4):

| Section | Core | Drop order | 4,096 | 8,192 |
|---|---|---|---|---|
| `instructions` with fresh examples and the draft shape | yes | — | about 1,000 | about 1,300 |
| `conditions` (the catalogue, only what this Mac's Apps report) | yes | — | at most 500 | at most 500 |
| `always_on` (aliases) | yes | — | at most 150 | at most 250 |
| `current_turn` | yes | — | at most 150 | at most 200 |
| `recent_turns` (the last two exchanges) | | 1 | at most 300 | at most 600 |
| `current_watches` (as refs) | | 2 | at most 350 | at most 800 |
| `targets` (ranked suggested first) | | 3 | at most 1,150 | at most 3,900 |
| Output | | | 200 | 200 |
| Margin | | | 296 | 442 |

**`Digest`:** ADR-0036 §1.2's table, as this assembler's row: `instructions` (with examples and the output shape) is core; `agents` and `events` share one group cap; `agent_words` is dropped first; `memory` has a share of 0 in 0.5. An agent its sections leave out still gets its rollup line there.

**`MemoryNote`** (§2.4; built only for its harness until it passes): `instructions` core about 600, `current_turn` core at most 150, `notes_in_scope` (current notes and preferences of that scope, so it doesn't re-propose them) at most 300, output 120.

**The larger rows (0.6, designed here, added with the bot's jobs):** a 32,768-token row (a larger local model or a cloud engine, ADR-0034's `Tier::Strong`) has the same sections with larger shares, plus `policies` (core: the user's rules for the bot in scope) and `decisions` (the bot's recent decisions and outcomes in scope). The code is the same; only rows are added.

#### 5.4 The rolling summary: a conversation ledger built by code

D-133 asks for "a rolling summary of earlier turns, kept within a fixed token budget". It is **not model prose.** A small model's free summaries change verbs and drop caveats (ADR-0033's and ADR-0036's evidence); a summary that feeds every later turn would carry one such error forward as context. The ledger is built by code from `chat_turns` at each assembly, so it is exact, free, and nothing to forget separately:

- **Subjects in play:** the entities resolved in earlier turns, newest first, with the user's alias where one exists: "web (session) [s3] · api (session) [s7] · Payments (project) [p1]". This is what lets "is it done yet?" resolve "it" by code before the look-ups.
- **Asked, one line per earlier turn** not in `recent_turns`: "10:42 · asked “is web stuck?” · found 1 session". The question in the user's own words (at most 20 words), the outcome kind and counts. **No states and no answer content:** whatever the earlier result said may have changed, and a follow-up looks again (§3).
- **Set in this chat:** watches confirmed and notes remembered, by their code-written echo ("watching api: finishes").
- **Within its share,** newest lines first; older ones fold into one counted line ("and 6 earlier questions about web and api").

It is a pure function of the turns: deleting a turn or a conversation changes it at once, and the test `the_ledger_holds_no_state_words` runs it over the fixture. In 0.6 a larger engine may add a model-written abstract under the same section id, checked by the shared guard against the source turns, with authority model, never replacing the ledger.

#### 5.5 Counting tokens

The assembler counts with the routed engine's own counter where it has one (Apple's `tokenCount(for:)`; the local helper's tokenizer, through the helper protocol). Where it must estimate, it counts 3.5 characters a token, rounded up: a budget must over-count, so this is tighter than ADR-0034 §6.4's 4 characters, which stays only for display and logs. §5.2 step 6 catches what an estimate still misses.

### 6. Receipts and "What I used"

Every assembled job writes a receipt as the payload of its log event (`chat.turn`, `digest.made`, `watch.drafted`; §7):

```rust
pub struct Receipt {
    pub job: JobKind, pub engine: EngineId, pub row: BudgetRowId,
    pub prompt_version: u32, pub guard_version: u32,
    pub sections: Vec<SectionUse>,  // id, cap, used tokens, items in, items dropped
    pub used: Vec<Used>,            // at most 16: ItemSource, version, reason, score (thousandths), age in days
    pub dropped: Vec<Dropped>,      // at most 16: ItemSource, reason
}
// reasons: AlwaysOn | Matched | SameEntity | RecentTurn | Ledger | LiveRow
// drop reasons: NoRoom | LiveWon | Superseded | BelowScore | Authority
```

- **No text, by type:** a receipt holds ids, codes and numbers only, so it never needs forgetting and never repeats a memory or a question. A test walks the payload types and fails on any `String` field.
- **"What I used"** under each Chat result, folded: plain lines resolved at view time from the ids, for example "Your name for web: *the api agent* (you said, 12 days ago)", "3 sessions, read just now", "Earlier in this chat: 2 questions", and "Left out for room: 2 older memories". A record forgotten since shows as "something you've since asked me to forget". Each memory line opens it in Settings › AI › Memory.
- **The digest's Show source** (ADR-0036 §1.8) and a watch draft's details read the same receipts.
- **Settings** shows per memory "used 4 times, last 2 days ago" from `use_count` and `last_used_ms`.

### 7. The append-only event and audit log

#### 7.1 Shape

```sql
CREATE TABLE ai_log (
    seq          INTEGER PRIMARY KEY AUTOINCREMENT,
    at_ms        INTEGER NOT NULL,
    stream       TEXT NOT NULL,     -- watch | alert | digest | feedback | chat | memory | assembly; 0.6: bot.decision | bot.approval | bot.action | bot.outcome
    verb         TEXT NOT NULL,     -- per stream, below
    actor        TEXT NOT NULL,     -- user | app | model:<engine id> | bot (0.6)
    subject_kind TEXT,              -- watch | digest | memory | conversation | notice …
    subject_id   TEXT,              -- the record, watch, digest or notice id
    subject_key  TEXT,              -- a memory's subject key, so forget can find its sidecars
    version      INTEGER,           -- the record version it concerns
    correlation  TEXT,              -- groups one turn, one digest, one fire and its alert; 0.6: one decision's chain
    cause_seq    INTEGER,           -- the event that caused this one (a fire's observation → its alert); 0.6: causal links
    schema       INTEGER NOT NULL,  -- the payload type's version
    payload      TEXT NOT NULL DEFAULT '{}'
                 CHECK (json_valid(payload) AND length(payload) <= 4096)   -- typed, text-free
);
CREATE INDEX ai_log_stream_at ON ai_log(stream, at_ms);
CREATE INDEX ai_log_subject ON ai_log(subject_kind, subject_id);
CREATE TABLE ai_log_text (                 -- the erasable sidecar for the few events that carry words
    seq         INTEGER PRIMARY KEY REFERENCES ai_log(seq),
    subject_key TEXT,
    body        TEXT NOT NULL,             -- at most 2 KB; secrets hidden; agent words marked by `untrusted`
    untrusted   INTEGER NOT NULL,
    expires_ms  INTEGER NOT NULL
);
CREATE TABLE ai_log_prune (stream TEXT PRIMARY KEY, before_ms INTEGER NOT NULL);
-- Triggers: no UPDATE on ai_log, ever. DELETE on ai_log only for a row older than its stream's before_ms.
--           before_ms only moves forward. ai_log_text may be deleted freely (forget, Last messages off, expiry).
```

**The spine is append-only and text-free; words live in a sidecar that can be erased.** That is how an audit log and "forget really deletes" coexist: the spine records that something happened, by whom, to which id, under which versions; a sidecar holds the few words an event needs (a "this is wrong" item, a watch's echo at the time it fired) and goes with its subject. Payloads are typed structs with ids, enums, numbers and versions only, checked like receipts (§6).

**A record change and its log event commit together:** the write gate (§1.3), the watch commands and the digest job append in the same transaction as the change they record.

#### 7.2 What 0.5 logs

| Stream | Verbs | Payload |
|---|---|---|
| `watch` | created, edited, paused, resumed, removed, expired, ended (target gone), fired | the watch id and version; for `fired`, the typed observation (kind, session, state, at) and the replay flag ("while AI Juggler was closed") |
| `alert` | sent, held (quiet hours), released, consolidated, broke_through ("Even during quiet hours") | the notice ids; for `consolidated`, what it folded |
| `digest` | made | the window, AI or rollup, the engine, prompt and guard versions, per item its check verdict and fallback reason, the receipt |
| `feedback` | wrong | the digest or turn id, the item, the engine, versions, the chosen reason; the item's words in the sidecar |
| `chat` | turn, stopped | the conversation and turn ids, the outcome, the receipt |
| `memory` | remembered, edited, forgot, cleared, promoted, decayed, deleted, refused, maintained | ids, kinds, counts, the refusal reason; never the text |
| `assembly` | core_overflow, too_long | the job, the row, the sizes |

ADR-0036 §2.7's list is the `watch`, `alert`, `digest` and `feedback` streams.

#### 7.3 Retention and reading

- **Kept:** the spine 90 days per stream in 0.5; sidecars 30 days; at most 200,000 spine rows, the oldest day of the largest stream pruned first. Pruning moves the stream's `before_ms` forward, then deletes, in maintenance's batches (§2.3).
- **Cleared:** Settings › AI › Memory's **Forget all memory** also erases every sidecar; the spine's ids and codes stay until their retention, since they hold nothing the user said. **Clear AI activity** (Settings › AI on this Mac, folded) moves every stream's watermark to now and deletes the spine too.
- **Read by:** the Watches view's "told 2 times", Settings › AI's "this is wrong" count, the receipts' views, the harness's replay, and, in 0.6, the bot's audit view. The log is never a source of facts for a model in 0.5.

### 8. Designed for 0.6, not built in 0.5

D-123 places the bot in 0.6 as an extension of Chat. It uses this store, this log, this assembler and the observation bus (ADR-0036 §2.3). What it adds is designed here so that 0.6 only adds; **none of it is built in 0.5**, and the bot gets **its own, much stricter harness**: D-130's chat criteria (≤ 1 % fabrication, omissions tolerated) never apply to a job that decides or acts.

#### 8.1 Policy memory, decisions, causal links and approvals

- **`policy`:** the user's rules for the bot ("never approve deletions in Payments", "continue tasks in the docs pool on my behalf"), each in a closed set of slots per scope, authority user only, written only from the bot's rules editor or a confirmed Chat draft (as watches are). A policy is core context for every bot job in its scope (§5.3's larger rows), never retrieved by score.
- **`decision`:** each bot decision as a record: the inputs it rested on (receipt ids), the policies it applied, the option chosen and those it rejected, its confidence. Written by the bot, authority bot, never superseding anything of the user's.
- **`action` and `outcome`:** the act, through the existing commands and the pool levels (ADR-0034 §8's `ActTool` registry), and what AI Juggler observed afterwards, written by the outcome observer with authority app.
- **Causal links,** a narrow table added by 0.6's migration: `memory_links(from_id, to_id, relation)` with relations `applied_policy`, `rested_on`, `approved_by`, `caused`, `outcome_of`. With the log's `correlation` and `cause_seq` (present from 0.5), a decision's chain is a few indexed joins, from the observation through the decision, the approval and the action to its outcome.
- **Approvals:** an approval is a `bot.approval` log event (actor user) linked to its decision by `cause_seq` and `approved_by`. An action that needs one cannot be written without the link; the write gate checks it.

#### 8.2 Authority and supersede governance

- **Stricter write rules for the bot:** the bot may write only `decision` and `action`; it may never write or supersede `policy`, `preference`, `note` or `standing_request`; it may propose them, as a draft the user confirms. Its writes are rate-bounded per hour and per scope.
- **A review path for conflicts:** a lower-authority write against a higher current row, refused in 0.5 (§1.3), becomes in 0.6 a proposal the user resolves in a review list, never a silent override.
- **Policy conflicts** (two policies in scope disagreeing) are found by slot when written and refused until the user resolves them.
- **Confidence from outcomes:** the bot's decisions gain a signal 0.5 lacks, the observed outcome. A decision kind whose outcomes fail lowers its confidence gently (divided, never zeroed) and raises its approval need; this never changes a user's rows.

#### 8.3 The research ideas that don't fit now, and where the bot may use them

- **A graph store and many typed edge kinds:** AI Juggler's entity set is closed and relational. The bot gets the narrow `memory_links` table (§8.1), not a graph database.
- **Dense embeddings and a re-ranking model:** they need extra models and memory that a 4K budget can't spare. Behind the `Retriever` trait if recall tests fail, and on larger engine rows for the bot (§4.4).
- **Multi-writer authority classes and a supersede review:** Chat has one user, and the user's edits rule. The bot is a second writer, so §8.2 brings them in.
- **Confidence driven by verification:** Chat has no verifier. The bot's outcomes are one (§8.2).
- **Shared or team memory layers, synced files:** never. Memory stays on this Mac (§9).
- **A cache of past answers served as answers:** usage and states change by the minute. Never; remembered answers are pointers (§3).

### 9. Privacy

- **Everything on this Mac.** The store, conversations and log are in AI Juggler's local database: never in the Fleet outbox or on a paired Mac, in a cloud backup only as the stated watches and preferences (§1.8), never in usage statistics, crash reports or the app's logs (tracing gets ids and codes; a test greps captured logs for fixture memory text). The AI helpers keep ADR-0033 §4's no-network sandbox and never read the database (ADR-0034 §6.3).
- **Settings › AI › Memory** (D-126's rich mode, reached also from Chat's panel and from each "What I used" line):
  - every current memory, grouped by kind (Your notes, Names you use, What matters to you, Watches, What you asked), each with its text, who it came from in plain words ("you said", "noted by AI (Beta)", "AI Juggler recorded"), its scope, its age, "used N times", and **Edit** (a new version), **Pin**, **Forget**; paged, never loaded all at once;
  - **Remember what I ask** (on by default): off stops new `asked` and `answer` records and forgets the existing ones, after a sheet that says so;
  - **Let Chat note things on its own (Beta)**, off by default, shown only when the `MemoryNote` job passed and its flag is on (D-127);
  - **Forget all memory**, with a sheet naming what goes and what stays (active watches stay, with a link to the Watches view);
  - the space used, against its 20 MB.
- **No secrets stored** (§1.3 step 4): a record holding one is refused, a question holding one is stored masked.
- **Agent text is untrusted** everywhere it lands (§1.2): not indexed, not recalled, guarded where a job reads it, and gone when Last messages goes off.
- **The trust guide's "AI on this Mac"** gains one row per claim, each with its named test (§10.1).
- **STRIDE, briefly:** spoofing (a "What I used" line or a memory link opening something else: links come from ids, never from text); tampering (a planted order writing a memory: only the user's words after a cue, or a verbatim span of the user's message, can become one; agent text never); repudiation (every memory change, watch change, fire, alert, digest and turn is logged with its actor); information disclosure (no secrets kept; Chat's words reach only this Mac's own backups, the cloud backup carrying only the stated settings, §1.8; forget scrubs pages, the index, the WAL and the pre-migration copies, and no backup made after it holds the subject); denial of service (the caps, the batches, the 600-character question); elevation (memory can't act; a remembered note is the user's words, framed as the user's, and a memory ref can't back a state claim, §3).

### 10. Tests

#### 10.1 Tests in code (each named in the trust guide or the task's gates)

- **`a_remembered_answer_never_contradicts_live_facts`:** a fixture of `answer` rows; every cited entity's state is changed, removed or kept, in every combination; for each, the rendered recall shows the live state as *now* (and the snapshot only labelled *then* with its age, or "no longer here"), and the assembled context for a follow-up holds no snapshot state token for any entity. Over the AJ-E3805 fixture database.
- **`a_stale_memory_is_shown_with_its_age`:** a remembered answer always carries its age; any memory older than 7 days used in a result carries it; the age is composed by code from `as_of`.
- **`forget_really_deletes`:** a canary string in a note, an `asked` row, a turn and a feedback sidecar; Forget; then the canary is absent from the database file's bytes, the WAL, the FTS shadow tables, both pre-migration copies made after it was written, and a backup archive's snapshot, and `recall` returns nothing for it. The same for Forget this conversation, Clear chat history, Clear cache and Forget all memory.
- **`the_core_is_never_truncated`:** a property test over random section sizes for every budget row: either the core reaches the prompt byte-identical, or the result is `CoreOverflow`; never a cut core.
- **`supersede_keeps_history`:** n edits to one subject leave n+1 versions linked both ways, exactly one current (the partial index refuses a second), the newest 20 kept, and retrieval returns only the current one.
- Also: `one_current_row_per_subject_key`, `a_lower_authority_write_never_supersedes_the_users`, `agent_text_is_always_marked_untrusted`, `untrusted_rows_are_never_indexed_or_recalled`, `no_secret_is_ever_stored`, `the_log_spine_is_append_only` (update and early delete refused by the triggers), `log_and_receipt_payloads_hold_no_text`, `a_record_change_and_its_log_event_commit_together`, `live_wins_on_a_fact_key`, `the_ledger_holds_no_state_words`, `remember_this_is_always_kept`, `an_inferred_note_below_the_gate_is_never_written`, `an_inferred_note_rests_on_the_users_own_words`, `a_watch_is_written_only_by_its_three_commands` (ADR-0036's, now through `KindWriter`), `maintenance_holds_stay_short` (no hold over one frame on the 5,000-record fixture), `every_budget_row_fits_its_window`, `memory_never_leaves_the_mac` (network stubbed; Fleet outbox, stats and crash-report paths checked), `v54_migrates_a_v53_database_and_changes_nothing_else`.

#### 10.2 The D-130 harness (AJ-E3818's, extended for AJ-E3819)

Synthetic multi-conversation histories, written blind to the prompts, in English and German, generic names only (D-124: no real transcripts). Practice sets for tuning; held-out sets judged once with the prompts frozen; a Sonnet session runs it, Opus checks every PASS or FAIL before it is recorded.

| Bar | Target | Measured on |
|---|---|---|
| **Invented recall** (a claim that the user asked, said or set something they didn't) | critical: at most 1 % of shown results, none systematic | held-out recall questions ("what did I ask about web yesterday?", "what do I call the api session?") |
| **Outdated stated as current** (a remembered state shown or used as now) | **0**, structural (§3), and checked on every held-out item | every shown result and every assembled context |
| Recall omissions (a relevant memory not used) | tolerated, recorded; about 10 % | the same set |
| A forgotten memory recalled | **0** | a forget-then-ask set |
| `MemoryNote` (if built for Beta): notes written that the user didn't say | critical, at most 1 %, none systematic; below that the job doesn't ship | messages with and without implicit preferences, with planted orders in quoted agent text |
| Value | the panel with memory preferred in at least 60 % of decided blind pairs against the same panel without it | follow-up conversations |

A failed round means the next optimisation in D-130's order (facts first: more of the work done by code, then the retry, then adapters, then larger models), never dropping memory.

### 11. Windows and Linux (D-52)

The store, the log, the retriever, the assembler and the ledger are OS-neutral SQL and Rust. Token counting falls back to the estimate where an engine has no counter; `secure_delete` and FTS5 are SQLite's own, so forget behaves the same on every OS.

## Alternatives considered

- **A separate memory database file.** It would keep memory out of backups and pre-migration copies by construction. But watches, digests and the feed read across it and the sessions tables, it adds a second writer lock against ADR-0030, and a second migration chain. The same guarantees come from the scrub of copies and backups, checked by `forget_really_deletes`.
- **A table per kind** (watches, preferences, history, digests apart). Each with its own provenance, supersede and forget code, and 0.6 would add three more. Rejected for one table, one write gate and per-kind payloads.
- **Updating rows in place.** Loses what the user said before, and an edit can't be audited or undone. Rejected for supersede.
- **Soft delete for forget** (a `forgotten` flag). The words stay on disk. Rejected: forget deletes, scrubs freed pages and the index, and reaches copies and backups.
- **Storing entity facts as memories** ("web is stuck"). They go stale within minutes; the user would see an old state as current. Rejected for live reads and snapshot-only answers.
- **A model-written rolling summary.** The evidence shows small models flipping verbs and dropping caveats in free prose; a summary carries one such error into every later turn. Rejected for the code-built ledger; a checked abstract may join it on larger engines (§5.4).
- **Embeddings from day one.** An extra model and memory for a store of a few thousand short rows in a closed vocabulary. Behind the trait, if the harness shows the need.
- **Proportional budgets** (percentages of the window). With 4K, every token is placed by hand; absolute shares per row are explicit and testable, and the larger rows are just more rows.
- **Truncating the core to fit.** A cut instruction or question changes what the model is asked. Rejected: the core is bounded at the source and overflow fails loudly.
- **Remote-tunable budgets.** A budget change changes what was tested. Rejected: budgets ship with the prompt version (ADR-0035 selects behaviour, never changes a tested one).
- **A mutable log, or text in the log.** An audit trail that can be edited is not one, and text in an append-only table can't be forgotten. Rejected for an append-only, text-free spine with erasable sidecars.
- **Keeping chat history in the registered cache** (ADR-0034 §6.7). The cache is for rebuildable data (ADR-0023), and forget must reach turns and what was derived from them in one transaction. Moved to the store; its Settings line and Clear cache stay.

## Consequences

- **Easier:**
  - The digest, watches, Chat and the 0.6 bot share one store, one log and one assembler; a new job is a budget row and its providers, a new kind is a payload type and a writer.
  - A user sees everything remembered, where it came from and how old it is, and forgetting is real.
  - "What I used" and the digest's sources come from the same receipts, which also explain a small model's failures when testing.
- **Harder:**
  - Every write goes through the gate, with a log event in the same transaction.
  - Forget has to reach the copies and the backup scrub, and its test reads raw file bytes.
  - Budget rows are a release artefact: a change re-runs the job's harness.
- **Accepted:**
  - Going back to an older build keeps no memory from the pre-migration copy. A backup made before a forget still holds what was forgotten (only a restore on this Mac drops it, §1.5).
  - The ledger is plainer than a written summary; it keeps no answer content, so a follow-up looks again.
  - Inferred notes wait for their harness; until then Chat remembers only what the user said, asked and set.
  - The spine of the log keeps ids and codes for 90 days after the words are forgotten.
- **To verify when building** (design facts, not questions): FTS5's `secure-delete` option in the bundled SQLite (it needs 3.42 or later; the contentless delete already in V27 needs 3.43); the token cost of eight look-up definitions and the watch catalogues in Apple's 4K context; that `secure_delete` toggled per hold has no effect on the other holds; the scrub's cost on a large pre-migration copy (the memory tables are small, so it should be milliseconds).

## Order of work

0.5 builds §§1 to 7, §9 and §10 for the kinds marked 0.5; §8 only as reserved names and the `memory_reserved_kinds` trigger. AJ-E3819 is one XL task in E38; these steps are its parts, ordered with ADR-0036's tasks:

1. **AJ-E3819, step 1: the store and the log.** Migration V54 (number re-checked across all branches); the write gate with `KindWriter`s, supersede, authority, untrusted, the secret refusal; forget, with the copies' and the backup's scrub; the log, its sidecar and watermarks; `forget_really_deletes`, `supersede_keeps_history`, `the_log_spine_is_append_only`. **AJ-E3712 (the digest's records), AJ-E3814 (watches) and AJ-E3817 (alert events) build on this step**, so they depend on it (their task files list AJ-E3702 and AJ-E3802 today; the dependency on this step is to be added).
2. **AJ-E3819, step 2: the assembler, budgets and receipts.** `features/ai/context/` with the providers' registry, the rows of §5.3, `CoreOverflow`, the counting, the receipts. **AJ-E3712's job (e) and AJ-E3815's job (f) assemble through it.** `the_core_is_never_truncated`, `every_budget_row_fits_its_window`.
3. **AJ-E3814 to AJ-E3817 and AJ-E3712, on steps 1 and 2** (ADR-0036's order): watches as `standing_request` records through their three commands, fires and alerts logged, digests as records with their `digest.made` events.
4. **AJ-E3819, step 3: Chat's memory.** Conversations in the store (AJ-E3808's history moves here), the ledger, `asked` and `answer` recording, the remember flow (explicit only), `KeywordRetriever`, the `always_on` and `recalled` providers, the `memory` look-up (Chat's eighth, read-only; ADR-0036 made `watches` the seventh), live-over-stored rendering, maintenance. `a_remembered_answer_never_contradicts_live_facts`, `a_stale_memory_is_shown_with_its_age`.
5. **AJ-E3819, step 4: Settings › AI › Memory and "What I used".** Mock-ups in both themes first (D-126); the a11y gate, the design critique, and a Sonnet walk (ask, recall with its age, edit, forget, clear) verified by Opus.
6. **AJ-E3818, extended: §10.2's recall run.** Then, only if the maintainer's D-130 bars allow and the run passes, **`MemoryNote`** as opt-in Beta; otherwise explicit memory alone ships, and the next optimisation round follows.

## Open questions for the maintainer

1. ~~**Should memory travel in your backups?**~~ **Decided** (the coordinating session, 2026-11-18, D-133's backup ruling): the stated watches and preferences travel as settings, locally and to the encrypted cloud backup; Chat's history, remembered answers, notes and inferred memories go to local backups only; Forget reaches the next backup and the backup page says so (§1.8).

For the coordinating session: ADR-0036's seams (§1.2's `memory` section, §1.9's digest records, §2.4's assembler, §2.7's watch records and log, §4's untrusted marking) are all met here as written, so ADR-0036 needs no change. Two task-file notes: AJ-E3712, AJ-E3814 and AJ-E3817 should depend on AJ-E3819's step 1 (and AJ-E3815 on its step 2), and ADR-0036's order of work still names "AJ-E3713: job (e)", which has no task file; job (e)'s run is AJ-E3818.
