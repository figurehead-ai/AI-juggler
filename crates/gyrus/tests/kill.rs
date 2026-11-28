//! ADR-0038 §1's crash check: a writer process is killed (SIGKILL, no
//! clean-up) at every step of a write, then the index rebuilt from SQLite
//! must equal SQLite, and the batch must be there whole or not at all.
//!
//! The test runs this binary again as the writer (`writer_until_killed`,
//! ignored in a normal run), which says where it is by a marker file and
//! waits there to be killed.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    missing_docs,
    reason = "tests: a failed step fails the test"
)]

mod common;

use std::collections::BTreeSet;
use std::path::Path;
use std::time::{Duration, Instant};

use common::{link, node, open, pack, small};
use gyrus::{Batch, Graph, Step};

const DB: &str = "GYRUS_KILL_DB";
const AT: &str = "GYRUS_KILL_AT";

fn steps() -> [(&'static str, Step); 3] {
    [
        ("staged", Step::Staged),
        ("committed", Step::Committed),
        ("published", Step::Published),
    ]
}

fn second_batch() -> Batch {
    let mut b = Batch::new();
    for i in 0..50 {
        b.node(node(
            "note",
            &format!("note:k{i}"),
            &format!("kill test note {i}"),
        ))
        .link(link(&format!("note:k{i}"), "about", "session:web"));
    }
    b
}

/// The writer: the small world committed, then a second batch written,
/// stopping at the step `GYRUS_KILL_AT` names until it is killed.
#[test]
#[ignore = "run by killed_at_every_step_the_rebuild_equals_sqlite as its writer"]
fn writer_until_killed() {
    let (Ok(path), Ok(at)) = (std::env::var(DB), std::env::var(AT)) else {
        return;
    };
    let conn = open(Path::new(&path));
    let graph = Graph::new(pack());
    graph.build(&conn).unwrap();
    graph.write(&conn, &small(), 1_000).unwrap();
    let wanted = steps()
        .into_iter()
        .find(|(name, _)| *name == at)
        .map(|(_, step)| step)
        .unwrap();
    let marker = format!("{path}.{at}");
    let _ = graph.write_observed(&conn, &second_batch(), 2_000, &mut |step| {
        if step == wanted {
            std::fs::write(&marker, b"here").unwrap();
            std::thread::sleep(Duration::from_secs(60));
        }
    });
}

/// What SQLite holds, as keys and (src, kind, dst) triples.
fn stored(path: &Path) -> BTreeSet<String> {
    let conn = open(path);
    let mut out = BTreeSet::new();
    let mut stmt = conn.prepare("SELECT key FROM gyrus_nodes").unwrap();
    for key in stmt.query_map([], |row| row.get::<_, String>(0)).unwrap() {
        out.insert(format!("n {}", key.unwrap()));
    }
    let mut stmt = conn
        .prepare(
            "SELECT s.key, l.kind, d.key FROM gyrus_links l
             JOIN gyrus_nodes s ON s.id = l.src JOIN gyrus_nodes d ON d.id = l.dst",
        )
        .unwrap();
    let rows = stmt
        .query_map([], |row| {
            Ok(format!(
                "l {} {} {}",
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?
            ))
        })
        .unwrap();
    for row in rows {
        out.insert(row.unwrap());
    }
    out
}

/// What a rebuilt index holds, in the same terms.
fn indexed(path: &Path) -> BTreeSet<String> {
    let conn = open(path);
    let graph = Graph::new(pack());
    graph.build(&conn).unwrap();
    let snap = graph.snapshot().unwrap();
    let mut out = BTreeSet::new();
    for id in snap.ids() {
        let key = &snap.node(id).unwrap().key;
        out.insert(format!("n {key}"));
        for group in snap.out_groups(id) {
            for edge in &group.edges {
                out.insert(format!(
                    "l {key} {} {}",
                    graph.pack().link_name(group.link),
                    snap.node(edge.other).unwrap().key
                ));
            }
        }
    }
    out
}

#[test]
fn killed_at_every_step_the_rebuild_equals_sqlite() {
    let exe = std::env::current_exe().unwrap();
    for (name, step) in steps() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("world.sqlite");
        gyrus::schema::install(&open(&path)).unwrap();
        let mut child = std::process::Command::new(&exe)
            .args([
                "writer_until_killed",
                "--exact",
                "--ignored",
                "--test-threads=1",
            ])
            .env(DB, &path)
            .env(AT, name)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let marker = dir.path().join(format!("world.sqlite.{name}"));
        let until = Instant::now() + Duration::from_secs(30);
        while !marker.exists() {
            assert!(Instant::now() < until, "the writer never reached {name}");
            if let Some(status) = child.try_wait().unwrap() {
                panic!("the writer ended at {name} before it was killed: {status}");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        child.kill().unwrap();
        let status = child.wait().unwrap();
        assert!(!status.success(), "{name}: the writer wasn't killed");
        let sql = stored(&path);
        assert_eq!(
            indexed(&path),
            sql,
            "{name}: the rebuilt index differs from SQLite"
        );
        let second = sql.iter().filter(|e| e.contains("note:k")).count();
        let whole = second_batch().ops.len();
        match step {
            Step::Staged => assert_eq!(second, 0, "{name}: an uncommitted batch survived"),
            Step::Committed | Step::Published => {
                assert_eq!(second, whole, "{name}: a committed batch is partly missing");
            }
        }
        assert!(
            sql.contains("n session:web"),
            "{name}: the first batch is gone"
        );
    }
}
