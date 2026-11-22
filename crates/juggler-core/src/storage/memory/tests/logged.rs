#![allow(clippy::unwrap_used, reason = "a test: a failed step fails the test")]

use super::*;

fn watch_record(key: &str, text: &str) -> NewRecord {
    let mut rec = record(key, Kind::StandingRequest, text);
    rec.source = SourceKind::WatchesView;
    rec.source_id = None;
    rec.payload = serde_json::json!({ "watch": { "id": key } });
    rec
}

fn created(at: i64, id: &str) -> NewEvent {
    let mut event = NewEvent::new(at, Stream::Watch, "created", Actor::User);
    event.subject = Some(("watch", id.to_owned()));
    event
}

/// The record and the feature's own event commit together, the event
/// naming the version written; the same content again writes neither.
#[test]
fn remember_logged_commits_the_event_with_its_record() {
    let storage = installed_storage();
    let writers = Writers::for_tests();
    let memory = storage.memory();
    let rec = watch_record("watch:w-1", "Tell you when a session finishes");
    let first = memory
        .remember_logged(&writers.watches, &Secrets, &rec, &created(NOW, "w-1"), NOW)
        .unwrap();
    let events = storage.ai_log().of_subject("watch", "w-1", 10).unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].verb, "created");
    assert_eq!(events[0].version, Some(first.version));
    assert_eq!(
        events[0].memory_subject.as_deref(),
        Some(first.subject_id.as_str())
    );
    let again = memory
        .remember_logged(
            &writers.watches,
            &Secrets,
            &rec,
            &created(NOW + 1, "w-1"),
            NOW + 1,
        )
        .unwrap();
    assert!(again.unchanged);
    assert_eq!(
        storage
            .ai_log()
            .of_subject("watch", "w-1", 10)
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        storage
            .ai_log()
            .count_of_subject("watch", "w-1", "created")
            .unwrap(),
        1
    );
}

/// An event with free text in it refuses the whole write: no record.
#[test]
fn a_wordy_event_writes_nothing() {
    let storage = installed_storage();
    let writers = Writers::for_tests();
    let rec = watch_record("watch:w-2", "Tell you when a session fails");
    let mut wordy = created(NOW, "w-2");
    wordy.payload = serde_json::json!({ "said": "the api agent finished its work" });
    let result = storage
        .memory()
        .remember_logged(&writers.watches, &Secrets, &rec, &wordy, NOW);
    assert!(matches!(
        result,
        Err(MemoryError::Refused(Refusal::NotTextFree))
    ));
    assert!(storage.memory().current("watch:w-2").unwrap().is_none());
}

/// The watches writer can't write as anyone but the user, nor any other kind.
#[test]
fn the_watches_writer_writes_only_the_users_watches() {
    let storage = installed_storage();
    let writers = Writers::for_tests();
    let mut rec = watch_record("watch:w-3", "Tell you when a session ends");
    rec.authority = Authority::App;
    let result = storage.memory().remember_logged(
        &writers.watches,
        &Secrets,
        &rec,
        &created(NOW, "w-3"),
        NOW,
    );
    assert!(matches!(
        result,
        Err(MemoryError::Refused(Refusal::WrongWriter))
    ));
    let note = record("note:x", Kind::Note, "remember the api");
    let result = storage.memory().remember_logged(
        &writers.watches,
        &Secrets,
        &note,
        &created(NOW, "w-3"),
        NOW,
    );
    assert!(matches!(
        result,
        Err(MemoryError::Refused(Refusal::WrongWriter))
    ));
}

/// Current versions of one kind, newest first, a keyset page at a time;
/// superseded ones left out.
#[test]
fn current_of_kind_lists_the_current_versions() {
    let storage = installed_storage();
    let writers = Writers::for_tests();
    let memory = storage.memory();
    for (i, id) in ["w-a", "w-b"].iter().enumerate() {
        let at = NOW + i64::try_from(i).unwrap();
        memory
            .remember_logged(
                &writers.watches,
                &Secrets,
                &watch_record(&format!("watch:{id}"), "Tell you when a session finishes"),
                &created(at, id),
                at,
            )
            .unwrap();
    }
    let edited = watch_record("watch:w-a", "Tell you when a session fails");
    memory
        .remember_logged(
            &writers.watches,
            &Secrets,
            &edited,
            &created(NOW + 5, "w-a"),
            NOW + 5,
        )
        .unwrap();
    remember(&storage, &record("note:n", Kind::Note, "a note"));
    let watches = memory
        .current_of_kind(Kind::StandingRequest, None, 10)
        .unwrap();
    let keys: Vec<&str> = watches.iter().map(|r| r.subject_key.as_str()).collect();
    assert_eq!(keys, ["watch:w-a", "watch:w-b"]);
    assert_eq!(watches[0].version, 2);
    let first = memory
        .current_of_kind(Kind::StandingRequest, None, 1)
        .unwrap();
    assert_eq!(first.len(), 1);
    let after = Some((first[0].created_ms, first[0].id.as_str()));
    let next = memory
        .current_of_kind(Kind::StandingRequest, after, 10)
        .unwrap();
    let keys: Vec<&str> = next.iter().map(|r| r.subject_key.as_str()).collect();
    assert_eq!(
        keys,
        ["watch:w-b"],
        "the next page starts after the last row"
    );
}
