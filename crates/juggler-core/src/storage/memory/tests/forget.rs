#![allow(clippy::unwrap_used, reason = "a test: a failed step fails the test")]

use super::*;

/// `n` notes written in one transaction, so their index is one segment of
/// many pages: each holds a long word, `{word}{n:05}q`, and a longer tail.
fn bulk_notes(storage: &Storage, word: &str, n: usize) {
    bulk(storage, word, n, usize::MAX);
}

/// [`bulk_notes`], every `stated`th row a stated preference instead (by
/// the user, from Settings, naming a session, "web pref {i}"): in the same
/// segment as the notes, as a store that grew over time has them.
fn bulk(storage: &Storage, word: &str, n: usize, stated: usize) {
    let tx = crate::storage::write_transaction(&storage.conn).unwrap();
    for i in 0..n {
        let id = format!("bulk-{i}-{}", word.len());
        let preference = i % stated == stated - 1;
        let (kind, text) = if preference {
            ("preference", format!("web pref {i}"))
        } else {
            (
                "note",
                format!("{word}{i:05}q and a longer tail of other words {i}"),
            )
        };
        tx.execute(
            "INSERT INTO memory_records (id, subject_key, subject_id, version, kind, scope_kind,
                 tier, priority, authority, untrusted, source_kind, confidence, as_of_ms, text,
                 created_ms)
             VALUES (?1, ?2, ?3, 1, ?4, 'global', 'long', 1, 'user', 0, 'settings', 1, 0,
                     ?5, 0)",
            rusqlite::params![
                id,
                format!("{kind}:bulk{i}{}", word.len()),
                format!("sub-{i}-{}", word.len()),
                kind,
                text
            ],
        )
        .unwrap();
        if preference {
            tx.execute(
                "INSERT INTO memory_entities (record_id, entity_kind, entity_id)
                 VALUES (?1, 'session', 's1')",
                [&id],
            )
            .unwrap();
        }
    }
    tx.commit().unwrap();
}

/// The FTS index's page separators (`memory_fts_idx`) holding `word`.
fn separators(storage: &Storage, word: &str) -> Vec<String> {
    let mut stmt = storage
        .conn
        .prepare("SELECT term FROM memory_fts_idx")
        .unwrap();
    stmt.query_map([], |row| row.get::<_, Vec<u8>>(0))
        .unwrap()
        .map(|term| String::from_utf8_lossy(&term.unwrap()).into_owned())
        .filter_map(|term| term.find(word).map(|at| term[at..].to_owned()))
        .collect()
}

/// Picks a page separator holding `word` from the middle of the index and
/// forgets every subject whose words it starts; returns it.
fn forget_a_separator(storage: &Storage, word: &str) -> String {
    let found = separators(storage, word);
    assert!(found.len() >= 3, "a multi-page index: {}", found.len());
    let separator = found[found.len() / 2].clone();
    assert!(separator.len() > word.len() + 2, "{separator}");
    for key in keys_holding(storage, &separator) {
        storage.memory().forget(&key, &Actor::User, NOW).unwrap();
    }
    separator
}

fn keys_holding(storage: &Storage, text: &str) -> Vec<String> {
    let keys: Vec<String> = storage
        .conn
        .prepare("SELECT subject_key FROM memory_records WHERE instr(text, ?1) > 0")
        .unwrap()
        .query_map([text], |row| row.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert!(!keys.is_empty());
    keys
}

/// Neither the index nor the file (with its WAL) holds `separator`.
fn assert_separator_gone(storage: &Storage, path: &std::path::Path, word: &str, separator: &str) {
    assert!(
        !separators(storage, word)
            .iter()
            .any(|s| s.starts_with(separator)),
        "{separator} is still in the index"
    );
    assert!(
        !spot(&file_bytes(path), separator),
        "{separator} is still in the file"
    );
}

fn store_on_disk(dir: &std::path::Path, name: &str) -> (Storage, std::path::PathBuf) {
    let path = dir.join(name);
    let storage = Storage::open(&path).unwrap();
    storage.conn.execute_batch(V54).unwrap();
    (storage, path)
}

/// The re-review's RX1: a second forget with no write since the first,
/// when the first left the index one segment. Fails with `optimize`.
#[test]
fn a_second_forget_in_a_row_clears_its_separator() {
    const WORD: &str = "secondword";
    let dir = tempfile::tempdir().unwrap();
    let (storage, path) = store_on_disk(dir.path(), "juggler.sqlite");
    bulk_notes(&storage, WORD, 4000);
    forget_a_separator(&storage, WORD);
    let second = forget_a_separator(&storage, WORD);
    assert_separator_gone(&storage, &path, WORD, &second);
}

/// The re-review's RX2: a store whose index is one segment already (after
/// an automerge or an earlier forget). Fails with `optimize`.
#[test]
fn a_forget_on_a_one_segment_index_clears_its_separator() {
    const WORD: &str = "onesegmentword";
    let dir = tempfile::tempdir().unwrap();
    let (storage, path) = store_on_disk(dir.path(), "juggler.sqlite");
    bulk_notes(&storage, WORD, 4000);
    storage
        .conn
        .execute_batch("INSERT INTO memory_fts(memory_fts) VALUES ('optimize');")
        .unwrap();
    let separator = forget_a_separator(&storage, WORD);
    assert_separator_gone(&storage, &path, WORD, &separator);
}

/// The re-review's RX3: an export (its index built fresh, one segment)
/// restored on this Mac after a forget: the tombstone pass drops the
/// subject and its separator from the file that replaces the live one.
/// Fails with `optimize`.
#[test]
fn a_restore_of_an_older_export_drops_the_forgotten_separator() {
    const WORD: &str = "restoredword";
    let dir = tempfile::tempdir().unwrap();
    let (live, live_path) = store_on_disk(dir.path(), "live.sqlite");
    bulk_notes(&live, WORD, 4000);
    let export_path = dir.path().join("export.sqlite");
    live.snapshot_to(&export_path).unwrap();
    let export = Storage::open(&export_path).unwrap();
    scrub_snapshot(&export.conn, Destination::Local).unwrap();
    // This Mac forgets, after the export, a word that starts an index page
    // of the export.
    let found = separators(&export, WORD);
    assert!(found.len() >= 3, "a multi-page export index");
    let separator = found[found.len() / 2].clone();
    for key in keys_holding(&live, &separator) {
        live.memory().forget(&key, &Actor::User, NOW).unwrap();
    }
    export
        .conn
        .execute(
            "ATTACH DATABASE ?1 AS prior",
            [live_path.to_string_lossy().as_ref()],
        )
        .unwrap();
    assert!(after_restore(&export.conn, "prior").unwrap() > 0);
    export.conn.execute("DETACH DATABASE prior", []).unwrap();
    assert_separator_gone(&export, &export_path, WORD, &separator);
}

/// H1 of the security review: FTS5's `secure-delete` leaves a page's
/// separator (the prefix of its first term) in `memory_fts_idx`. Forget
/// rewrites the index, so a forgotten word that started a page is gone
/// from the index and the file. Fails without the `optimize` in forget.
#[test]
fn forget_clears_the_index_page_separators() {
    const WORD: &str = "diagnosisword";
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("juggler.sqlite");
    let storage = Storage::open(&path).unwrap();
    storage.conn.execute_batch(V54).unwrap();
    bulk_notes(&storage, WORD, 4000);
    storage
        .conn
        .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
        .unwrap();
    let found = separators(&storage, WORD);
    assert!(found.len() >= 5, "a multi-page index: {}", found.len());
    let separator = found[found.len() / 2].clone();
    assert!(separator.len() > WORD.len() + 2, "{separator}");
    assert!(
        spot(&file_bytes(&path), &separator),
        "the search finds it before"
    );
    // Forget every subject whose word the separator starts.
    let keys: Vec<String> = storage
        .conn
        .prepare("SELECT subject_key FROM memory_records WHERE instr(text, ?1) > 0")
        .unwrap()
        .query_map([&separator], |row| row.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert!(!keys.is_empty());
    for key in &keys {
        storage.memory().forget(key, &Actor::User, NOW).unwrap();
    }
    assert!(
        !separators(&storage, WORD)
            .iter()
            .any(|s| s.starts_with(&separator)),
        "the separator is still in the index"
    );
    storage
        .conn
        .execute_batch("INSERT INTO memory_fts(memory_fts) VALUES ('integrity-check')")
        .unwrap();
    drop(storage);
    assert!(
        !spot(&file_bytes(&path), &separator),
        "the separator {separator} is still in the file"
    );
}

/// H1 of the security review, the backups: the cloud copy, a filtered
/// export rebuilt by the backup's own steps, holds no local-only or
/// forgotten word, not even as an index separator; a local backup holds
/// no forgotten one (fails without the scrub's fresh index).
#[test]
fn the_cloud_copy_holds_no_local_only_or_forgotten_word() {
    const LOCAL: &str = "localonlyword";
    const FORGOTTEN: &str = "forgottencanary";
    let dir = tempfile::tempdir().unwrap();
    let live_path = dir.path().join("live.sqlite");
    let live = Storage::open(&live_path).unwrap();
    live.conn.execute_batch(V54).unwrap();
    bulk(&live, LOCAL, 3000, 50);
    let gone = remember(
        &live,
        &record("note:gone", Kind::Note, &format!("{FORGOTTEN} words")),
    );
    live.conn
        .execute(
            "INSERT INTO memory_forgotten (subject_id, forgotten_ms) VALUES (?1, 1)",
            [&gone.subject_id],
        )
        .unwrap();
    let mut watch = record("watch:w1", Kind::StandingRequest, "watching web: finishes");
    watch.source = SourceKind::WatchesView;
    remember(&live, &watch);
    let mut alias = record("alias:session:s1", Kind::Preference, "the web agent");
    alias.source = SourceKind::Settings;
    remember(&live, &alias);
    // A forgotten word that starts an index page (a restored older copy
    // still holds it, with this Mac's tombstone).
    let found = separators(&live, LOCAL);
    assert!(found.len() >= 3, "a multi-page index");
    let separator = found[found.len() / 2].clone();
    live.conn
        .execute(
            "INSERT INTO memory_forgotten (subject_id, forgotten_ms)
             SELECT subject_id, 1 FROM memory_records WHERE instr(text, ?1) > 0",
            [&separator],
        )
        .unwrap();

    for (name, dest) in [
        ("cloud.sqlite", Destination::Cloud),
        ("local.sqlite", Destination::Local),
    ] {
        let path = dir.path().join(name);
        live.snapshot_to(&path).unwrap();
        let copy = Storage::open(&path).unwrap();
        scrub_snapshot(&copy.conn, dest).unwrap();
        // The app's own last steps (`backup::scrub`).
        copy.conn
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE); PRAGMA journal_mode = DELETE; VACUUM;")
            .unwrap();
        copy.conn
            .execute_batch("INSERT INTO memory_fts(memory_fts) VALUES ('integrity-check')")
            .unwrap();
        assert_eq!(
            count(
                &copy,
                "SELECT COUNT(*) FROM memory_fts WHERE memory_fts MATCH '\"web\"'"
            ),
            62,
            "{name}: the stated settings are found"
        );
        if dest == Destination::Cloud {
            assert!(separators(&copy, LOCAL).is_empty());
            assert_eq!(count(&copy, "SELECT COUNT(*) FROM memory_records"), 62);
        }
        drop(copy);
        let bytes = file_bytes(&path);
        assert!(!spot(&bytes, FORGOTTEN), "{name} holds the forgotten word");
        assert!(
            !spot(&bytes, &separator),
            "{name} holds the forgotten separator {separator}"
        );
        assert_eq!(
            spot(&bytes, LOCAL),
            dest == Destination::Local,
            "{name}: local-only words go to local backups only"
        );
    }
}

/// The cloud filter's own checks (the re-review's M2 and M3 at the cloud
/// layer): rows the gate would refuse, written straight to the table (an
/// older build, a bug), stay out of the cloud copy unless each part of a
/// stated setting holds.
#[test]
fn the_cloud_filter_takes_only_typed_trusted_stated_settings() {
    let dir = tempfile::tempdir().unwrap();
    let live = installed_storage();
    let row = |id: &str,
               kind: &str,
               authority: &str,
               untrusted: i64,
               source: &str,
               text: &str,
               entity: bool| {
        live.conn
            .execute(
                "INSERT INTO memory_records (id, subject_key, subject_id, version, kind, scope_kind,
                     tier, priority, authority, untrusted, source_kind, confidence, as_of_ms, text,
                     created_ms)
                 VALUES (?1, ?1, ?1, 1, ?2, 'global', 'long', 1, ?3, ?4, ?5, 1, 0, ?6, 0)",
                rusqlite::params![id, kind, authority, untrusted, source, text],
            )
            .unwrap();
        if entity {
            live.conn
                .execute(
                    "INSERT INTO memory_entities VALUES (?1, 'session', 's1')",
                    [id],
                )
                .unwrap();
        }
    };
    row(
        "pref:ok",
        "preference",
        "user",
        0,
        "settings",
        "the web agent",
        true,
    );
    row(
        "watch:ok",
        "standing_request",
        "user",
        0,
        "watches_view",
        "watching web",
        false,
    );
    row(
        "pref:untrusted",
        "preference",
        "user",
        1,
        "settings",
        "the web agent",
        true,
    );
    row(
        "pref:free",
        "preference",
        "user",
        0,
        "settings",
        &"words ".repeat(12),
        true,
    );
    row(
        "pref:no-entity",
        "preference",
        "user",
        0,
        "settings",
        "the web agent",
        false,
    );
    row(
        "pref:note-job",
        "preference",
        "user",
        0,
        "note_job",
        "the web agent",
        true,
    );
    row(
        "pref:model",
        "preference",
        "model",
        0,
        "settings",
        "the web agent",
        true,
    );
    row(
        "watch:settings",
        "standing_request",
        "user",
        0,
        "settings",
        "watching web",
        false,
    );
    row(
        "watch:untrusted",
        "standing_request",
        "user",
        1,
        "watches_view",
        "watching web",
        false,
    );
    let cloud = snapshot(&live, dir.path(), "cloud.sqlite");
    scrub_snapshot(&cloud.conn, Destination::Cloud).unwrap();
    assert_eq!(ids_in(&cloud), ["pref:ok", "watch:ok"]);
}

/// L3 of the security review: a `REPLACE` over a record inside the store's
/// hold fires the index's delete trigger, so the old words leave the
/// index. Fails without `recursive_triggers` in `SecureHold`.
#[test]
fn a_replace_inside_the_hold_leaves_no_old_words_in_the_index() {
    let storage = installed_storage();
    let kept = remember(
        &storage,
        &record("note:r", Kind::Note, "alphaword old words"),
    );
    {
        let _hold = super::secure::SecureHold::on(&storage.conn).unwrap();
        storage
            .conn
            .execute(
                "INSERT OR REPLACE INTO memory_records (id, subject_key, subject_id, version, kind,
                     scope_kind, tier, priority, authority, untrusted, source_kind, confidence,
                     as_of_ms, text, created_ms)
                 VALUES (?1, 'note:r', ?2, 1, 'note', 'global', 'long', 1, 'user', 0, 'settings',
                         1, 0, 'betaword new words', 0)",
                [&kept.id, &kept.subject_id],
            )
            .unwrap();
    }
    storage
        .conn
        .execute_batch("CREATE VIRTUAL TABLE temp.v3 USING fts5vocab(main, memory_fts, row);")
        .unwrap();
    assert_eq!(
        count(
            &storage,
            "SELECT COUNT(*) FROM temp.v3 WHERE term = 'alphaword'"
        ),
        0,
        "the replaced words are still indexed"
    );
    assert_eq!(
        count(
            &storage,
            "SELECT COUNT(*) FROM temp.v3 WHERE term = 'betaword'"
        ),
        1
    );
}

/// L4 of the security review: a sidecar on an event that names one of the
/// subject's versions (and not the subject) goes with the forget. Fails
/// without the by-version clause in `erase_subject`.
#[test]
fn a_sidecar_naming_a_version_goes_with_the_forget() {
    let storage = installed_storage();
    let kept = remember(&storage, &record("note:v", Kind::Note, "a note"));
    let mut event = NewEvent::new(NOW, Stream::Feedback, "wrong", Actor::User);
    event.subject = Some(("memory", kept.id.clone()));
    let seq = storage
        .ai_log()
        .append(
            &event,
            Some((
                &NewSidecar {
                    body: "quoted words",
                    untrusted: false,
                    expires_ms: NOW + DAY,
                },
                &Secrets,
            )),
        )
        .unwrap();
    assert!(storage.ai_log().sidecar(seq).unwrap().is_some());
    let gone = storage
        .memory()
        .forget("note:v", &Actor::User, NOW)
        .unwrap();
    assert_eq!(gone.sidecars, 1);
    assert!(storage.ai_log().sidecar(seq).unwrap().is_none());
}

fn spot(bytes: &[u8], needle: &str) -> bool {
    bytes
        .windows(needle.len())
        .any(|window| window == needle.as_bytes())
}

fn file_bytes(path: &std::path::Path) -> Vec<u8> {
    let mut out = std::fs::read(path).unwrap_or_default();
    for suffix in ["-wal", "-shm", "-journal"] {
        let mut name = path.as_os_str().to_owned();
        name.push(suffix);
        out.extend(std::fs::read(std::path::PathBuf::from(name)).unwrap_or_default());
    }
    out
}

#[test]
fn forget_leaves_no_trace_in_the_file() {
    const CANARY: &str = "qzxwvcanary";
    const KEY_CANARY: &str = "qzxwvkeyword";
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("juggler.sqlite");
    let storage = Storage::open(&path).unwrap();
    storage.conn.execute_batch(V54).unwrap();
    // Neighbours on the same pages, so the forgotten cells sit among live
    // ones; enough long words for an index of many pages.
    bulk_notes(&storage, "keep note neighbourword", 3000);
    // The conversation the note came from, and one that used it.
    storage
        .conn
        .execute_batch(&format!(
            "INSERT INTO chat_conversations (id, started_ms, last_ms, title, title_turn) VALUES
                 ('c1', 1, 4, 'remember {CANARY} for payments', 't1'),
                 ('c2', 1, 1, 'is {CANARY} stuck?', 't3'),
                 ('c3', 1, 1, 'the {CANARY} chat', NULL);
             INSERT INTO chat_turns (id, conversation_id, seq, at_ms, question, outcome, shown) VALUES
                 ('t1', 'c1', 1, 1, 'remember {CANARY} for payments', 'remembered',
                  'I''ll remember: {CANARY} for payments'),
                 ('t2', 'c1', 2, 2, 'how is web', 'rows', NULL),
                 ('t4', 'c1', 3, 3, 'and {CANARY} again?', 'rows', 'about {CANARY}'),
                 ('t3', 'c2', 1, 1, 'is {CANARY} stuck?', 'rows', NULL),
                 ('t5', 'c3', 1, 1, 'the {CANARY} chat', 'rows', NULL);"
        ))
        .unwrap();
    let key = format!("note:{KEY_CANARY}");
    let mut from_turn = record(&key, Kind::Note, &format!("remember {CANARY} for payments"));
    from_turn.source_id = Some("t1".into());
    let first = remember(&storage, &from_turn);
    storage.memory().link_turn("t4", &first.subject_id).unwrap();
    storage.memory().link_turn("t5", &first.subject_id).unwrap();
    let mut turn_event = NewEvent::new(NOW, Stream::Chat, "turn", Actor::App);
    turn_event.subject = Some(("turn", "t4".into()));
    storage
        .ai_log()
        .append(
            &turn_event,
            Some((
                &NewSidecar {
                    body: &format!("the turn quoted {CANARY}"),
                    untrusted: false,
                    expires_ms: NOW + DAY,
                },
                &Secrets,
            )),
        )
        .unwrap();
    let edited = remember(
        &storage,
        &record(
            &key,
            Kind::Note,
            &format!("remember {CANARY} for payments and docs"),
        ),
    );
    let mut asked = record(
        &format!("asked:{KEY_CANARY}"),
        Kind::Asked,
        &format!("is {CANARY} stuck?"),
    );
    asked.authority = Authority::App;
    asked.source_id = Some("t3".into());
    remember(&storage, &asked);
    let mut answer = record(
        &format!("answer:{KEY_CANARY}"),
        Kind::Answer,
        &format!("{CANARY} was stuck"),
    );
    answer.authority = Authority::App;
    remember(&storage, &answer);
    let mut feedback = NewEvent::new(NOW, Stream::Feedback, "wrong", Actor::User);
    feedback.memory_subject = Some(edited.subject_id.clone());
    storage
        .ai_log()
        .append(
            &feedback,
            Some((
                &NewSidecar {
                    body: &format!("it said {CANARY} wrongly"),
                    untrusted: false,
                    expires_ms: NOW + DAY,
                },
                &Secrets,
            )),
        )
        .unwrap();
    storage
        .conn
        .execute_batch(
            "CREATE VIRTUAL TABLE temp.vocab USING fts5vocab(main, memory_fts, row);
             PRAGMA wal_checkpoint(TRUNCATE);",
        )
        .unwrap();
    let has_term = |storage: &Storage| {
        count(
            storage,
            &format!("SELECT COUNT(*) FROM temp.vocab WHERE term = '{CANARY}'"),
        )
    };
    // The search finds it before: the proof that it would find a leftover.
    assert_eq!(has_term(&storage), 1);
    let before = file_bytes(&path);
    assert!(spot(&before, CANARY) && spot(&before, KEY_CANARY));

    let gone = storage
        .memory()
        .forget(&key, &Actor::User, NOW + 1)
        .unwrap();
    assert_eq!(
        (
            gone.subjects,
            gone.versions,
            gone.sidecars,
            gone.turns,
            gone.titles
        ),
        (1, 2, 2, 3, 2)
    );
    assert!(gone.wal_folded);
    let asked_gone = storage
        .memory()
        .forget(&format!("asked:{KEY_CANARY}"), &Actor::User, NOW + 2)
        .unwrap();
    assert_eq!(
        (asked_gone.subjects, asked_gone.versions),
        (2, 2),
        "asked takes its answer"
    );
    // The turns it came from or was used in are redacted; a title taken
    // from one is renamed from what's left, else neutral.
    let turns = |sql: &str| -> Vec<(String, String)> {
        let mut stmt = storage.conn.prepare(sql).unwrap();
        stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    };
    assert_eq!(
        turns("SELECT id, question FROM chat_turns WHERE redacted = 1 ORDER BY id"),
        [("t1", ""), ("t3", ""), ("t4", ""), ("t5", "")].map(|(a, b)| (a.to_owned(), b.to_owned()))
    );
    assert_eq!(
        turns("SELECT id, title FROM chat_conversations ORDER BY id"),
        [
            ("c1", "how is web"),
            ("c2", NEUTRAL_TITLE),
            ("c3", NEUTRAL_TITLE)
        ]
        .map(|(a, b)| (a.to_owned(), b.to_owned()))
    );
    assert_eq!(
        count(
            &storage,
            "SELECT COUNT(*) FROM chat_turns WHERE shown IS NOT NULL"
        ),
        0
    );
    assert_eq!(count(&storage, "SELECT COUNT(*) FROM chat_turn_memory"), 0);

    assert_eq!(
        count(
            &storage,
            &format!("SELECT COUNT(*) FROM memory_records WHERE text LIKE '%{CANARY}%'")
        ),
        0
    );
    assert_eq!(
        count(
            &storage,
            &format!("SELECT COUNT(*) FROM memory_fts WHERE memory_fts MATCH '\"{CANARY}\"'")
        ),
        0
    );
    assert_eq!(has_term(&storage), 0);
    assert_eq!(count(&storage, "SELECT COUNT(*) FROM ai_log_text"), 0);
    assert_eq!(count(&storage, "SELECT COUNT(*) FROM memory_forgotten"), 3);
    assert_eq!(
        count(
            &storage,
            "SELECT COUNT(*) FROM memory_fts WHERE memory_fts MATCH '\"keep\"'"
        ),
        3000,
        "the neighbours are still found"
    );
    storage
        .conn
        .execute_batch("INSERT INTO memory_fts(memory_fts) VALUES ('integrity-check')")
        .unwrap();
    let recall = RecallQuery {
        text: CANARY,
        entities: &[],
        scopes: &[Scope::Global],
        kinds: &[Kind::Note, Kind::Asked, Kind::Answer],
        min_authority: Authority::Model,
        limit: 10,
    };
    assert!(
        KeywordRetriever
            .recall(&storage.memory(), &recall, NOW)
            .unwrap()
            .is_empty()
    );

    // The raw file: neither the words nor the subject key's.
    let after = file_bytes(&path);
    assert!(
        !spot(&after, CANARY),
        "the forgotten words are still in the file"
    );
    assert!(
        !spot(&after, KEY_CANARY),
        "the forgotten key is still in the file"
    );
    drop(storage);
    let closed = file_bytes(&path);
    assert!(!spot(&closed, CANARY) && !spot(&closed, KEY_CANARY));
}

/// Fills a store with one of each class (ADR-0037 §1.8) and returns it.
fn every_class(storage: &Storage) -> Vec<(&'static str, String)> {
    let mut ids = Vec::new();
    // A preference the model noted, then the user restated.
    let mut noted = record("focus:global", Kind::Preference, "the api project");
    noted.authority = Authority::Model;
    noted.confidence = 0.9;
    noted.tier = Tier::Short;
    ids.push(("model_noted", remember(storage, &noted).id));
    let mut stated = record("focus:global", Kind::Preference, "the web project");
    stated.entities = vec![EntityRef {
        kind: EntityKind::Project,
        id: "p1".into(),
    }];
    ids.push(("user_preference", remember(storage, &stated).id));
    let mut watch = record("watch:w1", Kind::StandingRequest, "watching api: finishes");
    watch.source = SourceKind::WatchesView;
    ids.push(("user_watch", remember(storage, &watch).id));
    let mut only_model = record("alias:session:s9", Kind::Preference, "the docs agent");
    only_model.authority = Authority::Model;
    only_model.confidence = 0.95;
    ids.push(("model_preference", remember(storage, &only_model).id));
    ids.push((
        "note",
        remember(storage, &record("note:n1", Kind::Note, "payments first")).id,
    ));
    let mut asked = record("asked:h1", Kind::Asked, "is web stuck?");
    asked.authority = Authority::App;
    ids.push(("asked", remember(storage, &asked).id));
    let mut answer = record("answer:h1", Kind::Answer, "web was stuck");
    answer.authority = Authority::App;
    ids.push(("answer", remember(storage, &answer).id));
    let forgotten = remember(storage, &record("note:gone", Kind::Note, "forget me"));
    ids.push(("forgotten", forgotten.id));
    storage
        .conn
        .execute_batch(
            "INSERT INTO chat_conversations (id, started_ms, last_ms, title)
                 VALUES ('c1', 1, 2, 'is web stuck?');
             INSERT INTO chat_turns (id, conversation_id, seq, at_ms, question, outcome)
                 VALUES ('t1', 'c1', 1, 1, 'is web stuck?', 'rows');",
        )
        .unwrap();
    let mut wrong = NewEvent::new(NOW, Stream::Feedback, "wrong", Actor::User);
    wrong.memory_subject = Some(forgotten.subject_id.clone());
    let sidecar = NewSidecar {
        body: "it was wrong",
        untrusted: false,
        expires_ms: NOW + DAY,
    };
    storage
        .ai_log()
        .append(&wrong, Some((&sidecar, &Secrets)))
        .unwrap();
    // Forgotten on the live database after a backup was made: model it by
    // writing the tombstone alone, as a restored older copy would carry it.
    storage
        .conn
        .execute(
            "INSERT INTO memory_forgotten (subject_id, forgotten_ms) VALUES (?1, ?2)",
            rusqlite::params![forgotten.subject_id, NOW],
        )
        .unwrap();
    ids
}

fn snapshot(storage: &Storage, dir: &std::path::Path, name: &str) -> Storage {
    let path = dir.join(name);
    storage.snapshot_to(&path).unwrap();
    Storage::open(&path).unwrap()
}

fn ids_in(storage: &Storage) -> Vec<String> {
    let mut stmt = storage
        .conn
        .prepare("SELECT id FROM memory_records ORDER BY id")
        .unwrap();
    stmt.query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

#[test]
fn backup_inclusion_follows_the_rule() {
    let dir = tempfile::tempdir().unwrap();
    let live = installed_storage();
    let ids = every_class(&live);
    let id = |class: &str| ids.iter().find(|(c, _)| *c == class).unwrap().1.clone();

    // A local backup: everything but the forgotten subject and the tombstones.
    let local = snapshot(&live, dir.path(), "local.sqlite");
    scrub_snapshot(&local.conn, Destination::Local).unwrap();
    let mut want: Vec<String> = ids
        .iter()
        .filter(|(class, _)| *class != "forgotten")
        .map(|(_, id)| id.clone())
        .collect();
    want.sort();
    assert_eq!(ids_in(&local), want);
    assert_eq!(count(&local, "SELECT COUNT(*) FROM memory_forgotten"), 0);
    assert_eq!(count(&local, "SELECT COUNT(*) FROM chat_turns"), 1);
    assert!(count(&local, "SELECT COUNT(*) FROM ai_log") > 0);
    assert_eq!(
        count(&local, "SELECT COUNT(*) FROM ai_log_text"),
        0,
        "the forgotten one's"
    );

    // The cloud: the stated settings only.
    let cloud = snapshot(&live, dir.path(), "cloud.sqlite");
    scrub_snapshot(&cloud.conn, Destination::Cloud).unwrap();
    let mut want = vec![id("user_preference"), id("user_watch")];
    want.sort();
    assert_eq!(ids_in(&cloud), want);
    let preference = cloud.memory().current("focus:global").unwrap().unwrap();
    assert_eq!(
        preference.supersedes, None,
        "the link to the model's version is cut"
    );
    assert_eq!(preference.version, 2);
    assert_eq!(
        cloud.memory().entities(&preference.id).unwrap().len(),
        1,
        "its entities travel with it"
    );
    assert_eq!(count(&cloud, "SELECT COUNT(*) FROM memory_entities"), 1);
    for table in [
        "chat_conversations",
        "chat_turns",
        "ai_log",
        "ai_log_text",
        "memory_forgotten",
    ] {
        assert_eq!(
            count(&cloud, &format!("SELECT COUNT(*) FROM {table}")),
            0,
            "{table}"
        );
    }
    assert!(count(&cloud, "SELECT COUNT(*) FROM ai_log_prune") > 0);
    cloud
        .conn
        .execute_batch("INSERT INTO memory_fts(memory_fts) VALUES ('integrity-check')")
        .unwrap();
    assert_eq!(
        count(
            &cloud,
            "SELECT COUNT(*) FROM memory_fts WHERE memory_fts MATCH '\"project\"'"
        ),
        1
    );
    // The copy's own triggers still hold.
    assert!(
        cloud
            .conn
            .execute("UPDATE memory_records SET text = 'x'", [])
            .is_err()
    );

    // AI Juggler's own copy (before an update): no memory at all.
    let own = snapshot(&live, dir.path(), "own.sqlite");
    scrub_snapshot(&own.conn, Destination::OwnCopy).unwrap();
    for table in [
        "memory_records",
        "memory_entities",
        "chat_turns",
        "ai_log_text",
    ] {
        assert_eq!(
            count(&own, &format!("SELECT COUNT(*) FROM {table}")),
            0,
            "{table}"
        );
    }
    assert!(
        count(&own, "SELECT COUNT(*) FROM ai_log") > 0,
        "the spine holds no words"
    );

    // A restore on this Mac drops what this Mac forgot.
    let restored = snapshot(&live, dir.path(), "restored.sqlite");
    restored
        .conn
        .execute("DELETE FROM memory_forgotten", [])
        .unwrap();
    restored
        .conn
        .execute(
            "ATTACH DATABASE ?1 AS prior",
            [dir.path().join("local.sqlite").to_string_lossy().as_ref()],
        )
        .unwrap();
    // `prior` (the local copy) has no tombstones: nothing dropped.
    assert_eq!(after_restore(&restored.conn, "prior").unwrap(), 0);
    restored.conn.execute("DETACH DATABASE prior", []).unwrap();
    let live_path = dir.path().join("live.sqlite");
    live.snapshot_to(&live_path).unwrap();
    restored
        .conn
        .execute(
            "ATTACH DATABASE ?1 AS prior",
            [live_path.to_string_lossy().as_ref()],
        )
        .unwrap();
    assert_eq!(after_restore(&restored.conn, "prior").unwrap(), 1);
    assert!(restored.memory().current("note:gone").unwrap().is_none());
    assert_eq!(
        count(&restored, "SELECT COUNT(*) FROM main.memory_forgotten"),
        1
    );
    restored.conn.execute("DETACH DATABASE prior", []).unwrap();
}

#[test]
fn a_pre_migration_copy_holds_no_memory() {
    let dir = tempfile::tempdir().unwrap();
    let live = installed_storage();
    every_class(&live);
    let path = dir.path().join("juggler.sqlite.before-v54.partial");
    live.snapshot_to(&path).unwrap();
    scrub_copy(&path).unwrap();
    let copy = Storage::open(&path).unwrap();
    for table in [
        "memory_records",
        "memory_entities",
        "chat_turns",
        "ai_log_text",
    ] {
        assert_eq!(
            count(&copy, &format!("SELECT COUNT(*) FROM {table}")),
            0,
            "{table}"
        );
    }
    assert!(
        count(&copy, "SELECT COUNT(*) FROM ai_log") > 0,
        "the spine holds no words"
    );
    drop(copy);
    let bytes = file_bytes(&path);
    assert!(!spot(&bytes, "payments first"));
    // A copy without the store is left as it is.
    let plain = dir.path().join("plain.sqlite");
    drop(Storage::open(&plain).unwrap());
    scrub_copy(&plain).unwrap();
}

/// L1 of the security review: a forget whose WAL fold another
/// connection's open read blocks is folded again by the next write once
/// the read ends, so the forgotten words leave the file and its WAL; and
/// every connection the store writes through cuts its WAL on restart
/// (`journal_size_limit = 0`). Fails without the retry, or the limit.
#[test]
fn a_blocked_wal_fold_is_retried_by_the_next_write() {
    const CANARY: &str = "qzxwvblockedfold";
    let dir = tempfile::tempdir().unwrap();
    let (storage, path) = store_on_disk(dir.path(), "juggler.sqlite");
    remember(
        &storage,
        &record("note:blocked", Kind::Note, &format!("remember {CANARY}")),
    );
    assert_eq!(
        count(&storage, "PRAGMA journal_size_limit"),
        0,
        "the WAL is cut on restart"
    );
    // Another connection holds a read open across the forget.
    let reader = crate::storage::open_hardened(&path).unwrap();
    reader
        .execute_batch("BEGIN; SELECT COUNT(*) FROM memory_records;")
        .unwrap();
    let gone = storage
        .memory()
        .forget("note:blocked", &Actor::User, NOW)
        .unwrap();
    assert!(!gone.wal_folded, "the read should have blocked the fold");
    assert!(
        spot(&file_bytes(&path), CANARY),
        "the words wait in the file or WAL"
    );
    reader.execute_batch("COMMIT;").unwrap();
    drop(reader);
    // The next write folds it.
    storage.memory().touch(&[], NOW + 1).unwrap();
    assert!(
        !spot(&file_bytes(&path), CANARY),
        "the forgotten words are still in the file or WAL"
    );
    assert!(
        retry_pending_fold(&storage.conn).unwrap(),
        "still marked pending"
    );
}
