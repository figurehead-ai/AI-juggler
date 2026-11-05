//! AJ-E2912: what a switch writes for debug mode, in its own test binary
//! (`tracing` caches an event site's interest per process: tests running
//! beside it without a subscriber would hide its events).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

mod support;

use juggler_adapter::model::HarnessId;
use juggler_core::model::TranscriptRef;
use juggler_handover::pipeline::{self, FreezeChoice, Mechanism};
use support::tree::{TreeSupport, seed_tree, target};

/// AJ-E2912: the steps a switch writes for debug mode carry words and
/// numbers only, never what's in the conversation (its text, its folder).
#[test]
fn the_switchs_debug_steps_never_carry_the_conversation() {
    use std::sync::{Arc, Mutex};
    use tracing_subscriber::layer::SubscriberExt as _;

    #[derive(Clone, Default)]
    struct Seen(Arc<Mutex<Vec<String>>>);
    struct Fields<'a>(&'a mut String);
    impl tracing::field::Visit for Fields<'_> {
        fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
            use std::fmt::Write as _;
            let _ = write!(self.0, " {}={value:?}", field.name());
        }
    }
    impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for Seen {
        fn on_event(
            &self,
            event: &tracing::Event<'_>,
            _: tracing_subscriber::layer::Context<'_, S>,
        ) {
            let mut line = event.metadata().target().to_owned();
            event.record(&mut Fields(&mut line));
            self.0.lock().unwrap().push(line);
        }
    }

    const MARK: &str = "MARKED-WORDS-NEVER-LOGGED";
    let work = tempfile::tempdir().unwrap();
    let cwd = format!("/w/{MARK}");
    let main = seed_tree(&work.path().join("a"), &cwd, "sess-9");
    std::fs::write(
        &main,
        format!(
            "{}\n",
            serde_json::json!({
                "uuid": "u1", "parentUuid": null, "sessionId": "sess-9", "cwd": cwd,
                "type": "user", "message": {"role": "user", "content": MARK},
            })
        ),
    )
    .unwrap();
    let support = TreeSupport {
        harness: HarnessId::new("claude-code"),
        partial_install: false,
        torn_tail: false,
    };
    let seen = Seen::default();
    let subscriber = tracing_subscriber::registry().with(seen.clone());
    tracing::subscriber::with_default(subscriber, || {
        pipeline::execute(
            &support,
            &TranscriptRef {
                path: main.clone(),
                native_id: Some("sess-9".into()),
            },
            &target(&work.path().join("b")),
            FreezeChoice::StopNowAndSwitch,
            Mechanism::Transplant,
            &mut |_| Ok(7),
        )
        .unwrap();
    });
    let lines = seen.0.lock().unwrap().clone();
    let steps: Vec<&str> = lines
        .iter()
        .filter_map(|l| l.split(" step=").nth(1)?.split(' ').next())
        .collect();
    assert_eq!(
        steps,
        vec![
            "\"start\"",
            "\"bundled\"",
            "\"installed\"",
            "\"launched\"",
            "\"done\""
        ],
        "{lines:?}"
    );
    assert!(lines.iter().all(|l| l.starts_with("aij::flow")));
    assert!(
        lines.iter().all(|l| !l.contains(MARK)),
        "no step carries the conversation: {lines:?}"
    );
}
