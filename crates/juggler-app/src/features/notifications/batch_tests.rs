//! The batch on a fake clock: the 60-second burst, consolidation, quiet
//! hours released as one, the breakthrough switch, told-once keys and a
//! watch folded into its need.

use super::*;

const SEC: i64 = 1_000;

fn s(id: &str) -> Target {
    Target::Session(id.into())
}

fn finished(id: &str) -> Item {
    Item {
        name: id.into(),
        phrase: "finished".into(),
        phrase_many: "finished".into(),
        title: id.into(),
        body: "Finished".into(),
        ..Item::new(format!("finished:{id}"), Kind::Finished, s(id), 1)
    }
}

fn asks(id: &str) -> Item {
    Item {
        name: id.into(),
        phrase: "asks you".into(),
        phrase_many: "ask you".into(),
        title: id.into(),
        body: "Waiting for your reply".into(),
        ..Item::new(format!("asks:{id}"), Kind::Waiting, s(id), 2)
    }
}

fn watch(id: &str, key: &str, echoes: Option<Kind>, level: u8) -> Item {
    Item {
        name: id.into(),
        phrase: "finished".into(),
        phrase_many: "finished".into(),
        title: id.into(),
        body: "Finished · your watch".into(),
        watch: true,
        echoes,
        ..Item::new(key, Kind::Watch, s(id), level)
    }
}

#[test]
fn seven_agents_finishing_in_a_minute_are_one_notification_naming_them() {
    let mut batch = Batcher::default();
    let names = ["api", "web", "docs", "infra", "ui", "db", "cli"];
    let mut sent = Vec::new();
    for (i, name) in names.iter().enumerate() {
        let at = i64::try_from(i).unwrap() * 8 * SEC;
        sent.extend(batch.offer(vec![finished(name)], false, at));
    }
    assert!(sent.is_empty(), "calm items wait for the batch");
    assert_eq!(batch.next_flush(), Some(60 * SEC));
    assert!(
        batch.flush(59 * SEC).is_none(),
        "not before its minute is up"
    );
    let group = batch.flush(60 * SEC).expect("one at the end");
    assert_eq!(group.items.len(), 7);
    let words = group.words();
    assert_eq!(words.title, "api finished · 6 more");
    assert_eq!(words.body, "web, docs, infra, ui and 2 more finished");
    assert_eq!(words.opens, Opens::Feed, "no need among them: the feed");
    assert!(batch.flush(120 * SEC).is_none(), "sent once");
}

#[test]
fn the_first_need_of_a_burst_goes_at_once_and_the_rest_wait_for_its_end() {
    let mut batch = Batcher::default();
    let now = batch.offer(vec![asks("web")], false, 0);
    assert_eq!(now.len(), 1, "the first L2 at once");
    assert_eq!(now[0].words().title, "web");
    assert_eq!(now[0].words().opens, Opens::Target(s("web")));
    // A second need and two finishes within the minute: held.
    assert!(batch.offer(vec![asks("api")], false, 10 * SEC).is_empty());
    assert!(
        batch
            .offer(vec![finished("docs"), finished("infra")], false, 20 * SEC)
            .is_empty()
    );
    // The next offer after the minute sends the burst first.
    let later = batch.offer(Vec::new(), false, 61 * SEC);
    assert_eq!(later.len(), 1);
    let words = later[0].words();
    assert_eq!(
        words.title, "api asks you · 2 more",
        "titled from the highest level"
    );
    assert_eq!(words.body, "docs and infra finished");
    assert_eq!(words.opens, Opens::Centre, "a need among them: the centre");
}

#[test]
fn a_burst_with_one_item_keeps_todays_notification() {
    let mut batch = Batcher::default();
    assert!(batch.offer(vec![finished("api")], false, 0).is_empty());
    let group = batch.flush(BATCH_MS).unwrap();
    let words = group.words();
    assert_eq!(
        (words.title.as_str(), words.body.as_str()),
        ("api", "Finished")
    );
    assert_eq!(words.opens, Opens::Target(s("api")));
}

#[test]
fn a_key_is_told_once() {
    let mut batch = Batcher::default();
    assert_eq!(batch.offer(vec![asks("web")], false, 0).len(), 1);
    assert!(batch.offer(vec![asks("web")], false, 61 * SEC).is_empty());
    let mut items = vec![finished("api"), finished("api")];
    items[1].key = "finished:api".into();
    batch.offer(items, false, 200 * SEC);
    assert_eq!(batch.flush(400 * SEC).unwrap().items.len(), 1);
}

#[test]
fn quiet_hours_hold_watch_alerts_and_release_everything_as_one() {
    let mut batch = Batcher::default();
    let quiet = true;
    assert!(
        batch
            .offer(
                vec![
                    watch("api", "w1", Some(Kind::Finished), 1),
                    watch("web", "w2", None, 3),
                ],
                quiet,
                0
            )
            .is_empty(),
        "held through the night"
    );
    assert!(batch.waiting(SEC));
    // Quiet hours end: the engine releases what it held (flagged), and the
    // held alerts come with it, all in one notification, at once.
    let mut released = asks("docs");
    released.held = true;
    let out = batch.offer(vec![released], false, 8 * 3_600 * SEC);
    assert_eq!(out.len(), 1, "one notification");
    assert_eq!(out[0].items.len(), 3);
    assert_eq!(out[0].items[0].name, "web", "highest level first");
    assert!(!batch.waiting(8 * 3_600 * SEC));
}

#[test]
fn a_watch_set_to_go_through_quiet_hours_does() {
    let mut batch = Batcher::default();
    let mut loud = watch("web", "w1", None, 3);
    loud.breakthrough = true;
    let out = batch.offer(vec![loud, watch("api", "w2", None, 3)], true, 0);
    assert_eq!(out.len(), 1, "the switch's alert at once");
    assert_eq!(out[0].items[0].name, "web");
    assert!(batch.waiting(SEC), "the other waits for the morning");
}

#[test]
fn a_watch_on_a_need_just_told_is_not_a_second_notification() {
    let mut batch = Batcher::default();
    assert_eq!(batch.offer(vec![asks("web")], false, 0).len(), 1);
    let echo = watch("web", "w1", Some(Kind::Waiting), 2);
    assert!(batch.offer(vec![echo], false, 2 * SEC).is_empty());
    assert!(batch.flush(BATCH_MS).is_none(), "nothing held either");
}

#[test]
fn a_watch_and_its_need_in_one_burst_are_one_line_marked_asked() {
    let mut batch = Batcher::default();
    // Something loud went first, so the burst holds the rest.
    assert_eq!(batch.offer(vec![asks("db")], false, 0).len(), 1);
    batch.offer(vec![watch("web", "w1", Some(Kind::Waiting), 2)], false, SEC);
    batch.offer(vec![asks("web")], false, 2 * SEC);
    let group = batch.flush(BATCH_MS).unwrap();
    assert_eq!(group.items.len(), 1, "the need took the alert's place");
    assert!(group.items[0].watched);
    assert_eq!(group.items[0].kind, Kind::Waiting);
    assert_eq!(
        group.words().body,
        "Waiting for your reply · you asked to be told"
    );
}

#[test]
fn replayed_fires_come_as_one_notification_marked_closed() {
    let mut batch = Batcher::default();
    let mut a = watch("api", "w1", Some(Kind::Finished), 1);
    let mut b = watch("web", "w2", Some(Kind::Finished), 1);
    a.replayed = true;
    b.replayed = true;
    let out = batch.offer(vec![a, b], false, 0);
    assert_eq!(out.len(), 1);
    let words = out[0].words();
    assert_eq!(
        words.title,
        "While AI Juggler was closed: api finished · 1 more"
    );
    assert_eq!(words.body, "web finished");
}

#[test]
fn coming_to_the_front_drops_what_waits() {
    let mut batch = Batcher::default();
    batch.offer(vec![finished("api")], false, 0);
    batch.offer(vec![watch("web", "w1", None, 1)], true, 0);
    batch.drop_waiting();
    assert!(batch.flush(BATCH_MS).is_none());
    assert!(!batch.waiting(BATCH_MS));
}

#[test]
fn names_words_read_as_a_list() {
    let n =
        |names: &[&str]| names_words(&names.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>());
    assert_eq!(n(&["api"]), "api");
    assert_eq!(n(&["api", "web"]), "api and web");
    assert_eq!(n(&["api", "docs", "infra"]), "api, docs and infra");
    assert_eq!(n(&["a", "b", "c", "d", "e", "f"]), "a, b, c, d and 2 more");
}
