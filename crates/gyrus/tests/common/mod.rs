//! What the tests share: a small generic pack, a database on disk, and a
//! scan of every file for a phrase.

#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::disallowed_methods,
    missing_docs,
    reason = "test helpers: each test file uses a part; gyrus's own temp databases"
)]

use std::path::{Path, PathBuf};

use gyrus::{
    Authority, Batch, Graph, KindSpec, Label, LinkSpec, NewLink, NewNode, Pack, Tier, WalkRule,
};
use rusqlite::Connection;

/// A pack of people, projects, sessions, notes, topics and rules.
pub fn pack() -> Pack {
    Pack::builder()
        .kind(
            KindSpec::new("person")
                .half_life(365.0)
                .walk(WalkRule::out("prefers", 2, 4))
                .walk(WalkRule::out("works_on", 2, 4))
                .question("Is this still who they are?"),
        )
        .kind(
            KindSpec::new("project")
                .walk(WalkRule::inward("part_of", 2, 5))
                .walk(WalkRule::inward("about", 2, 5)),
        )
        .kind(
            KindSpec::new("session")
                .walk(WalkRule::out("part_of", 2, 3))
                .walk(WalkRule::out("runs_in", 1, 2))
                .walk(WalkRule::inward("about", 2, 5))
                .question("Is it running right now? Read its live state.")
                .not_searchable(),
        )
        .kind(KindSpec::new("app").walk(WalkRule::inward("runs_in", 1, 5)))
        .kind(
            KindSpec::new("note")
                .half_life(90.0)
                .walk(WalkRule::out("about", 2, 4))
                .mergeable()
                .max_links("about", 8),
        )
        .kind(KindSpec::new("preference").walk(WalkRule::out("about", 1, 3)))
        .kind(KindSpec::new("topic").walk(WalkRule::inward("part_of", 1, 6)))
        .kind(KindSpec::new("rule"))
        .link(LinkSpec::new("prefers", &["person"], &["preference"]))
        .link(LinkSpec::new("works_on", &["person"], &["project"]))
        .link(LinkSpec::new(
            "part_of",
            &["session", "note"],
            &["project", "topic"],
        ))
        .link(LinkSpec::new("runs_in", &["session"], &["app"]))
        .link(LinkSpec::new("about", &["note", "preference"], &[]))
        .link(LinkSpec::new("governs", &["rule"], &[]))
        .link(LinkSpec::new("same_as", &["note"], &["note"]))
        .rules("rule", "governs")
        .same_as("same_as")
        .build()
        .unwrap()
}

pub struct Db {
    pub dir: tempfile::TempDir,
    pub path: PathBuf,
    pub conn: Connection,
}

/// A database on disk in WAL mode, with gyrus's tables.
pub fn db() -> Db {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("world.sqlite");
    let conn = open(&path);
    gyrus::schema::install(&conn).unwrap();
    Db { dir, path, conn }
}

pub fn open(path: &Path) -> Connection {
    let conn = Connection::open(path).unwrap();
    conn.pragma_update(None, "journal_mode", "WAL").unwrap();
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    conn
}

pub fn node(kind: &str, key: &str, label: &str) -> NewNode {
    NewNode::new(kind, key, Label::Trusted(label.to_owned()), 1_000)
}

pub fn link(src: &str, kind: &str, dst: &str) -> NewLink {
    NewLink::new(src, kind, dst, "test")
}

/// A small world: alice works on api; sessions web and docs in api, in
/// two apps; notes about web.
pub fn small() -> Batch {
    let mut b = Batch::new();
    b.node(NewNode {
        tier: Tier::Long,
        authority: Authority::User,
        ..node("person", "person:alice", "Alice")
    })
    .node(node("project", "project:api", "api backend"))
    .node(NewNode {
        aliases: vec!["the api agent".into()],
        ..node("session", "session:web", "")
    })
    .node(node("session", "session:docs", ""))
    .node(node("app", "app:term", "Terminal app"))
    .node(node("app", "app:desk", "Desktop app"))
    .node(node("note", "note:1", "web deploys need the staging flag"))
    .node(node("note", "note:2", "docs wait on the api review"))
    .node(node("preference", "pref:short", "short answers"))
    .link(link("person:alice", "works_on", "project:api"))
    .link(link("person:alice", "prefers", "pref:short"))
    .link(link("session:web", "part_of", "project:api"))
    .link(link("session:docs", "part_of", "project:api"))
    .link(link("session:web", "runs_in", "app:term"))
    .link(link("session:docs", "runs_in", "app:desk"))
    .link(link("note:1", "about", "session:web"))
    .link(link("note:2", "about", "session:docs"))
    .link(link("note:2", "about", "project:api"));
    b
}

/// A built graph over the small world.
pub fn small_graph(db: &Db) -> Graph {
    let graph = Graph::new(pack());
    graph.build(&db.conn).unwrap();
    graph.write(&db.conn, &small(), 2_000).unwrap();
    graph
}

/// Every file beside the database (the file, its WAL and shm) that holds
/// `needle`'s bytes.
pub fn holding(dir: &Path, needle: &str) -> Vec<String> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if !path.is_file() {
            continue;
        }
        let bytes = std::fs::read(&path).unwrap();
        if bytes.windows(needle.len()).any(|w| w == needle.as_bytes()) {
            out.push(path.file_name().unwrap().to_string_lossy().into_owned());
        }
    }
    out
}

/// Every keyword-index term, in every bucket, that holds `needle`.
pub fn fts_terms(conn: &Connection, needle: &str) -> Vec<String> {
    let mut out = Vec::new();
    for bucket in 0..gyrus::FTS_BUCKETS {
        let table = gyrus::schema::fts_table(bucket);
        conn.execute_batch(&format!(
            "CREATE VIRTUAL TABLE IF NOT EXISTS temp.v_{table} USING fts5vocab(main, {table}, row);"
        ))
        .unwrap();
        let mut stmt = conn
            .prepare(&format!("SELECT term FROM temp.v_{table}"))
            .unwrap();
        let terms: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .unwrap()
            .map(Result::unwrap)
            .filter(|t: &String| t.contains(needle))
            .collect();
        out.extend(terms);
    }
    out
}
