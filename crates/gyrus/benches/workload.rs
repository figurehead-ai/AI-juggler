//! The benchmark's workload, shared by `benches/budgets.rs` (prints the
//! numbers) and `tests/budgets.rs` (fails past a budget): a graph of N
//! links over N ÷ 5 nodes of 15 kinds and 24 link kinds, hubs drawn from a
//! Zipf-like spread, weights random, labels from a 2,000-word vocabulary,
//! aliases on 1 % of nodes and 384-dimension vectors on up to 10,000.
//! Written straight into SQLite (a fixture), then indexed by a real build.

#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::disallowed_methods,
    clippy::too_many_lines,
    missing_docs,
    reason = "a benchmark fixture: fixed sizes, its own temp database"
)]

use std::time::{Duration, Instant};

use gyrus::ops::{self, Pattern, SeedQuery, Var, Walk};
use gyrus::{Graph, KindSpec, LinkSpec, NodeId, Pack, WalkRule};
use rusqlite::{Connection, params};

pub const KINDS: usize = 15;
pub const LINKS: usize = 24;
pub const DIM: usize = 384;
pub const NOW: i64 = 1_800_000_000_000;

/// The budgets (ADR-0038 §3), p95 in milliseconds.
pub const BUDGETS: &[(&str, f64)] = &[
    ("seed", 5.0),
    ("expand", 5.0),
    ("matches", 5.0),
    ("as_of", 5.0),
    ("rank", 1.0),
    ("touch", 3.0),
    ("forget", 20.0),
    ("dream", 200.0),
];

pub struct Rng(u64);

impl Rng {
    pub const fn new(seed: u64) -> Self {
        Self(seed | 1)
    }
    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    pub fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
    pub fn unit(&mut self) -> f32 {
        (self.next() % 1_000_000) as f32 / 1_000_000.0
    }
    /// Zipf-like: small indexes far more often (hubs).
    pub fn zipf(&mut self, n: usize) -> usize {
        let u = f64::from(self.unit()).max(1e-9);
        ((n as f64).powf(u) as usize).saturating_sub(1).min(n - 1)
    }
}

pub fn kind_name(i: usize) -> String {
    format!("k{i:02}")
}

pub fn link_name(i: usize) -> String {
    format!("l{i:02}")
}

pub fn pack() -> Pack {
    let mut b = Pack::builder();
    for k in 0..KINDS {
        let mut spec = KindSpec::new(&kind_name(k)).half_life(30.0 + k as f64 * 10.0);
        for j in 0..3 {
            let l = (k * 3 + j) % LINKS;
            spec = spec.walk(WalkRule::both(&link_name(l), 3, 8));
        }
        if k == 3 {
            spec = spec.mergeable();
        }
        b = b.kind(spec);
    }
    for l in 0..LINKS {
        b = b.link(LinkSpec::new(&link_name(l), &[], &[]));
    }
    b.rules(&kind_name(14), &link_name(23))
        .same_as(&link_name(22))
        .build()
        .unwrap()
}

pub fn word(i: usize) -> String {
    const SYL: [&str; 16] = [
        "ka", "lo", "mi", "ner", "po", "qua", "ri", "sen", "tu", "vex", "wo", "xan", "yel", "zo",
        "bri", "dun",
    ];
    format!(
        "{}{}{}",
        SYL[i % 16],
        SYL[(i / 16) % 16],
        SYL[(i / 256) % 16]
    )
}

pub struct World {
    pub dir: tempfile::TempDir,
    pub conn: Connection,
    pub graph: Graph,
    pub nodes: usize,
    pub links: usize,
    pub build: Duration,
    pub hubs: Vec<NodeId>,
}

pub fn open(path: &std::path::Path) -> Connection {
    let conn = Connection::open(path).unwrap();
    conn.pragma_update(None, "journal_mode", "WAL").unwrap();
    conn.pragma_update(None, "synchronous", "NORMAL").unwrap();
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    conn
}

/// A graph of `links` links, built and indexed.
pub fn world(links: usize) -> World {
    let dir = tempfile::tempdir().unwrap();
    let conn = open(&dir.path().join("bench.sqlite"));
    gyrus::schema::install(&conn).unwrap();
    let nodes = (links / 5).max(100);
    let mut rng = Rng::new(0x5eed_0000 + links as u64);
    conn.execute_batch("BEGIN IMMEDIATE").unwrap();
    {
        let mut node = conn
            .prepare(
                "INSERT INTO gyrus_nodes (id, kind, key, label, indexed, tier, priority, salience,
                     confidence, authority, as_of, last_used, use_count, valid_from, created)
                 VALUES (?1, ?2, ?3, ?4, 1, ?5, ?6, ?7, ?8, 'app', ?9, ?10, ?11, ?12, ?9)",
            )
            .unwrap();
        let mut alias = conn
            .prepare("INSERT OR IGNORE INTO gyrus_aliases (alias, node) VALUES (?1, ?2)")
            .unwrap();
        let vectors = nodes.min(10_000);
        let mut blocks: std::collections::BTreeMap<i64, (Vec<u8>, Vec<u8>)> =
            std::collections::BTreeMap::new();
        for i in 1..=nodes {
            let id = i as i64;
            let kind = kind_name(i % KINDS);
            let words = 3 + rng.below(4);
            let label: Vec<String> = (0..words).map(|_| word(rng.zipf(2_000))).collect();
            let label = label.join(" ");
            let tier = ["short", "mid", "long"][rng.below(3)];
            let age = (rng.below(120) as i64) * 86_400_000;
            node.execute(params![
                id,
                kind,
                format!("{kind}:{i}"),
                label,
                tier,
                rng.below(4) as i64,
                f64::from(rng.unit()),
                f64::from(0.5 + rng.unit() / 2.0),
                NOW - age,
                if rng.below(2) == 0 {
                    Some(NOW - age / 2)
                } else {
                    None
                },
                rng.below(20) as i64,
                NOW - age - 86_400_000,
            ])
            .unwrap();
            let table = gyrus::schema::fts_table(gyrus::schema::bucket_of(id));
            conn.prepare_cached(&format!(
                "INSERT INTO {table}(rowid, label) VALUES (?1, ?2)"
            ))
            .unwrap()
            .execute(params![id, label])
            .unwrap();
            if i % 100 == 0 {
                alias.execute(params![format!("agent{i}"), id]).unwrap();
            }
            if i <= vectors {
                let v: Vec<f32> = (0..DIM).map(|_| rng.unit() - 0.5).collect();
                let block = blocks
                    .entry(id.div_euclid(gyrus::schema::VECTOR_BLOCK))
                    .or_default();
                block.0.extend(id.to_le_bytes());
                block.1.extend(gyrus::store::vector_blob(&v));
            }
        }
        for (block, (ids, v)) in &blocks {
            conn.execute(
                "INSERT INTO gyrus_vectors (dim, block, ids, v) VALUES (?1, ?2, ?3, ?4)",
                params![DIM as i64, block, ids, v],
            )
            .unwrap();
        }
        let mut link = conn
            .prepare(
                "INSERT OR IGNORE INTO gyrus_links (src, dst, kind, weight, confidence,
                     valid_from, created)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            )
            .unwrap();
        let mut made = 0;
        while made < links {
            let src = 1 + rng.below(nodes);
            let dst = 1 + rng.zipf(nodes);
            if src == dst {
                continue;
            }
            let kind = link_name(rng.below(LINKS));
            made += link
                .execute(params![
                    src as i64,
                    dst as i64,
                    kind,
                    f64::from(0.05 + 0.95 * rng.unit()),
                    f64::from(0.5 + rng.unit() / 2.0),
                    NOW - (rng.below(200) as i64) * 86_400_000,
                    NOW,
                ])
                .unwrap();
        }
    }
    conn.execute_batch("UPDATE gyrus_meta SET v = 1 WHERE k = 'gen'; COMMIT;")
        .unwrap();
    let graph = Graph::new(pack());
    let began = Instant::now();
    graph.build(&conn).unwrap();
    let build = began.elapsed();
    let hubs: Vec<NodeId> = (1..=5).map(NodeId::from).collect();
    World {
        dir,
        conn,
        graph,
        nodes,
        links,
        build,
        hubs,
    }
}

/// p50 and p95 of `runs` timings, in ms.
pub fn percentiles(mut ms: Vec<f64>) -> (f64, f64) {
    ms.sort_by(f64::total_cmp);
    let at = |q: f64| ms[((ms.len() as f64 - 1.0) * q).round() as usize];
    (at(0.5), at(0.95))
}

fn time(runs: usize, mut f: impl FnMut(usize)) -> (f64, f64) {
    let mut out = Vec::with_capacity(runs);
    for i in 0..runs {
        let t = Instant::now();
        f(i);
        out.push(t.elapsed().as_secs_f64() * 1000.0);
    }
    percentiles(out)
}

/// Each operation's p50 and p95 (ms) on `w`.
pub fn measure(w: &World, runs: usize) -> Vec<(&'static str, f64, f64)> {
    let mut rng = Rng::new(7);
    let pack = w.graph.pack();
    let queries: Vec<String> = (0..runs)
        .map(|i| {
            format!(
                "{} {} agent{}",
                word(rng.zipf(2_000)),
                word(rng.zipf(2_000)),
                (i % 9 + 1) * 100
            )
        })
        .collect();
    let vectors: Vec<Vec<f32>> = (0..runs)
        .map(|_| (0..DIM).map(|_| rng.unit() - 0.5).collect())
        .collect();
    let mut out = Vec::new();
    let snap = w.graph.snapshot().unwrap();
    let (p50, p95) = time(runs, |i| {
        let q = SeedQuery {
            text: &queries[i],
            vector: Some(&vectors[i]),
            k: 12,
            ..SeedQuery::default()
        };
        let seeds = ops::seed(&w.conn, &snap, pack, &q).unwrap();
        assert_ne!(seeds.len(), 0);
    });
    out.push(("seed", p50, p95));
    let seeds_for = |i: usize| -> Vec<ops::Seed> {
        (0..3)
            .map(|j| ops::Seed {
                node: if j == 0 {
                    w.hubs[i % w.hubs.len()]
                } else {
                    1 + ((i * 7919 + j * 104_729) % w.nodes) as NodeId
                },
                relevance: 1.0,
                via: ops::Via::Given,
            })
            .collect()
    };
    let mut expanded = Vec::new();
    let (p50, p95) = time(runs, |i| {
        let reached = ops::expand(&snap, pack, &seeds_for(i), &Walk::new(200, NOW));
        assert!(reached.len() > 3);
        if i == 0 {
            expanded = reached;
        }
    });
    out.push(("expand", p50, p95));
    let (p50, p95) = time(runs, |i| {
        let reached = ops::expand(
            &snap,
            pack,
            &seeds_for(i),
            &Walk::new(200, NOW).at(NOW - 100 * 86_400_000),
        );
        assert_ne!(reached.len(), 0);
    });
    out.push(("as_of", p50, p95));
    let (p50, p95) = time(runs, |_| {
        let top = ops::rank(&snap, &expanded, 50);
        assert_ne!(top.len(), 0);
    });
    out.push(("rank", p50, p95));
    let (p50, p95) = time(runs, |i| {
        let hub = w.hubs[i % w.hubs.len()];
        let pattern = Pattern {
            vars: vec![Var { kind: None }, Var { kind: None }, Var { kind: None }],
            needs: vec![
                ops::Need {
                    from: 1,
                    link: link_name(i % LINKS),
                    to: 0,
                },
                ops::Need {
                    from: 1,
                    link: link_name((i + 1) % LINKS),
                    to: 2,
                },
            ],
            distinct: vec![(0, 2)],
            anchor: Some((0, hub)),
            limit: 20,
            at: None,
        };
        let _ = ops::matches(&snap, pack, &pattern);
    });
    out.push(("matches", p50, p95));
    drop(snap);
    let (p50, p95) = time(runs, |i| {
        let ids: Vec<NodeId> = (0..50)
            .map(|j| 1 + ((i * 50 + j) % w.nodes) as NodeId)
            .collect();
        w.graph.touch(&ids);
        w.graph.flush_uses(&w.conn, NOW + i as i64).unwrap();
    });
    out.push(("touch", p50, p95));
    // The idle pass: one batch of at most 200 writes, planned from the
    // snapshot and written, from its cursor (months later, so most nodes
    // have something to change).
    let later = NOW + 90 * 86_400_000;
    let mut after = 0;
    let (p50, p95) = time(runs.min(40), |_| {
        let snap = w.graph.snapshot().unwrap();
        let planned = gyrus::dream::plan(
            &w.conn,
            &snap,
            w.graph.pack(),
            &gyrus::dream::DreamOpts {
                after,
                ..gyrus::dream::DreamOpts::new(later, 200)
            },
        )
        .unwrap();
        drop(snap);
        assert!(planned.batch.ops.len() <= 200);
        after = planned.next.unwrap_or(0);
        w.graph.write(&w.conn, &planned.batch, later).unwrap();
    });
    out.push(("dream", p50, p95));
    let forgets = runs.min(50);
    let (p50, p95) = time(forgets, |i| {
        let key = format!("{}:{}", kind_name((w.nodes - i) % KINDS), w.nodes - i);
        let erased = w.graph.forget(&w.conn, &[key.as_str()]).unwrap();
        assert_eq!(erased.nodes, 1);
    });
    out.push(("forget", p50, p95));
    out
}
