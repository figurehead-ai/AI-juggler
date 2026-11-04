//! D-62 decision 12 (AJ-E3005 req. 2): the Reviewer.
//!
//! - It gives a verdict on a task in review that someone else holds:
//!   approved keeps it in review for the person's own Approve (D-44);
//!   sent back returns it to its holder with the note.
//! - It doesn't take build tasks, and a member can't give verdicts.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use juggler_core::storage::{NewTask, Storage, TASK_USER};
use juggler_core::tasks::{
    Priority, Role, TaskKind, TaskStatus, UserAction, Verdict, may_claim, may_review,
};

fn in_review(storage: &Storage, pool: &str, id: &str) {
    storage
        .tasks()
        .create(&NewTask {
            id,
            pool_id: pool,
            kind: TaskKind::Task,
            parent_id: None,
            title: "Fix refunds",
            body: "",
            acceptance: "",
            priority: Priority::Normal,
            status: TaskStatus::Ready,
            assignee: Some(("work:w-builder", "Builder")),
            created_by: TASK_USER,
            created_by_name: "You",
            external_url: None,
            blocked_by: &[],
            project: None,
        })
        .unwrap();
    storage
        .raw_conn()
        .execute("UPDATE tasks SET status = 'in_review' WHERE id = ?1", [id])
        .unwrap();
}

#[test]
fn the_rules_say_who_reviews_and_who_takes_build_tasks() {
    assert!(may_review(Role::Reviewer, TaskStatus::InReview, false).is_ok());
    assert!(may_review(Role::Lead, TaskStatus::InReview, false).is_ok());
    assert!(may_review(Role::Member, TaskStatus::InReview, false).is_err());
    assert!(may_review(Role::Reviewer, TaskStatus::InProgress, false).is_err());
    assert!(
        may_review(Role::Reviewer, TaskStatus::InReview, true).is_err(),
        "not its own"
    );
    assert!(may_claim(Role::Reviewer).is_err());
    assert!(may_claim(Role::Member).is_ok());
    assert_eq!(Verdict::parse("approved"), Some(Verdict::Approved));
    assert_eq!(Verdict::parse("sent_back"), Some(Verdict::SentBack));
    assert_eq!(Verdict::parse("done"), None);
}

#[test]
fn approved_waits_on_the_persons_approve_and_sent_back_goes_to_its_holder() {
    let storage = Storage::open_in_memory().unwrap();
    let pool = storage.pools().create("Reviews", None).unwrap().id;
    let reviewer = ("work:w-codex", "Codex reviewer");

    in_review(&storage, &pool, "aj-0001");
    let task = storage
        .tasks()
        .review(
            "aj-0001",
            Verdict::Approved,
            Some("Tests cover it"),
            reviewer,
        )
        .unwrap();
    assert_eq!(
        task.stored,
        TaskStatus::InReview,
        "the person's Approve is still theirs"
    );
    let done = storage
        .tasks()
        .user_action(
            "aj-0001",
            UserAction::Approve,
            None,
            None,
            false,
            (TASK_USER, "You"),
        )
        .unwrap();
    assert_eq!(done.stored, TaskStatus::Done);

    in_review(&storage, &pool, "aj-0002");
    assert!(
        storage
            .tasks()
            .review("aj-0002", Verdict::SentBack, None, reviewer)
            .is_err(),
        "sent back needs its note"
    );
    let back = storage
        .tasks()
        .review(
            "aj-0002",
            Verdict::SentBack,
            Some("Refund twice fails"),
            reviewer,
        )
        .unwrap();
    assert_eq!(back.stored, TaskStatus::InProgress);
    assert_eq!(back.assignee_ref.as_deref(), Some("work:w-builder"));
    let verbs: Vec<String> = storage
        .raw_conn()
        .prepare("SELECT verb FROM task_events WHERE task_id = 'aj-0002' ORDER BY rowid")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert!(verbs.contains(&"review_sent_back".to_owned()), "{verbs:?}");
    // A task no longer in review takes no verdict.
    assert!(
        storage
            .tasks()
            .review("aj-0002", Verdict::Approved, None, reviewer)
            .is_err()
    );
}
