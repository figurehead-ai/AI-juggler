//! AJ-E2402 (D-41a): sightings of the processes in sessions other apps run.
//! Each tick the graph's other-apps sessions are noted here in memory; a
//! process seen for the first time is written once (pid, start time, boot,
//! program name, session, first and last seen, a keyed hash of its command
//! line), its last seen at most once a minute after. A helper still alive
//! when its session's own process has ended is a leftover: "left behind
//! by" its session. Kept 7 days; never the environment, never the command
//! line itself, nothing sent anywhere.

use std::collections::{HashMap, HashSet};

use juggler_core::storage::{SIGHTING_KEEP_MS, Sighting};
use sha2::{Digest, Sha256};

use super::model::{LeftoverInput, Live, TheirsInput};

/// How often a known sighting's last seen is written.
pub const TOUCH_EVERY_MS: i64 = 60_000;
/// How often old sightings are forgotten.
pub const PRUNE_EVERY_MS: i64 = 3_600_000;
/// A helper is sighted once it has run this long: a process that lives a
/// moment (a hook, one command) never becomes a leftover worth showing.
pub const MIN_AGE_MS: i64 = 10_000;
/// The app setting holding the hash's key (random, made once, local).
pub const SALT_KEY: &str = "process_hash_salt";

/// A keyed hash of a command line: SHA-256 over the install's own random
/// key and the line, hex. The same line hashes the same on this Mac (so a
/// snapshot can tell two processes apart); the line can't be read back,
/// and without the key a guessed line can't be checked.
#[must_use]
pub fn cmd_hash(salt: &[u8], cmdline: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(salt);
    hasher.update([0_u8]);
    hasher.update(cmdline.as_bytes());
    hex(&hasher.finalize()[..16])
}

/// The install's hash key, made on first use.
#[must_use]
pub fn salt(storage: &juggler_core::storage::Storage) -> Vec<u8> {
    let settings = storage.app_settings();
    if let Ok(Some(hex)) = settings.get(SALT_KEY)
        && let Some(bytes) = unhex(&hex)
        && bytes.len() >= 16
    {
        return bytes;
    }
    let mut bytes = [0_u8; 32];
    if getrandom::fill(&mut bytes).is_err() {
        // No randomness: a key from the clock is still local and unknown.
        let now = crate::commands::now_millis().to_le_bytes();
        bytes[..now.len()].copy_from_slice(&now);
    }
    let _ = settings.set(SALT_KEY, &hex(&bytes));
    bytes.to_vec()
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut out, byte| {
        let _ = write!(out, "{byte:02x}");
        out
    })
}

fn unhex(text: &str) -> Option<Vec<u8>> {
    (0..text.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(text.get(at..at + 2)?, 16).ok())
        .collect()
}

/// The sightings in memory, and what the store still needs.
#[derive(Debug, Default)]
pub struct Memory {
    /// Every sighting of this boot, by `(pid, start)`, with when its last
    /// seen was last written.
    known: HashMap<(u32, i64), (Sighting, i64)>,
    /// Loaded from the store once.
    pub loaded: bool,
    /// When old ones were last forgotten.
    pub pruned_ms: i64,
}

/// One tick's outcome.
#[derive(Debug, Default)]
pub struct Noted {
    /// To write (new ones, and known ones whose last seen moved on).
    pub writes: Vec<Sighting>,
    /// Their leftovers: processes alive whose session's own process ended.
    pub leftovers: Vec<LeftoverInput>,
}

impl Memory {
    /// Loads this boot's sightings (from the store, once).
    pub fn load(&mut self, sightings: Vec<Sighting>, now_ms: i64) {
        for sighting in sightings {
            self.known
                .insert((sighting.pid, sighting.start_time), (sighting, now_ms));
        }
        self.loaded = true;
    }

    /// How many are held.
    #[must_use]
    pub fn len(&self) -> usize {
        self.known.len()
    }

    /// None held.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.known.is_empty()
    }

    /// The `(tool, session)` of every held sighting.
    #[must_use]
    pub fn sessions(&self) -> HashSet<(String, String)> {
        self.known
            .values()
            .map(|(sighting, _)| (sighting.tool.clone(), sighting.session_id.clone()))
            .collect()
    }

    /// Every held identity `(pid, start)`.
    #[must_use]
    pub fn identities(&self) -> Vec<(u32, i64)> {
        self.known.keys().copied().collect()
    }

    /// The sighting of one process, by its identity.
    #[must_use]
    pub fn get(&self, pid: u32, start: i64) -> Option<&Sighting> {
        self.known.get(&(pid, start)).map(|(sighting, _)| sighting)
    }

    /// Notes this tick's sessions. `commands` names each process's program
    /// and command line (hashed here, never kept); `alive` is every
    /// process's live facts (the table); `live_sessions` the `(tool, id)`
    /// of the sessions still open. A process not seen for
    /// [`SIGHTING_KEEP_MS`] is forgotten.
    #[allow(clippy::too_many_arguments)] // one tick's inputs, each named
    pub fn note(
        &mut self,
        sessions: &[TheirsInput],
        commands: &HashMap<u32, (String, String, i64)>,
        alive: &dyn Fn(u32, i64) -> Option<Live>,
        live_sessions: &HashSet<(String, String)>,
        salt: &[u8],
        boot_id: &str,
        now_ms: i64,
    ) -> Noted {
        let mut noted = Noted::default();
        let mut seen_now: HashSet<(u32, i64)> = HashSet::new();
        for session in sessions {
            for process in &session.tree {
                let identity = (process.pid, process.start_time);
                seen_now.insert(identity);
                if let Some((sighting, written)) = self.known.get_mut(&identity) {
                    sighting.last_seen = now_ms;
                    if now_ms - *written >= TOUCH_EVERY_MS {
                        *written = now_ms;
                        noted.writes.push(sighting.clone());
                    }
                    continue;
                }
                if process.pid != session.root_pid
                    && now_ms - process.start_time.saturating_mul(1000) < MIN_AGE_MS
                {
                    continue;
                }
                // Hash the line only once the table has it for this very
                // process (a full scan reads it within 10 s).
                let Some((exe, cmd, start)) = commands.get(&process.pid) else {
                    continue;
                };
                if *start != process.start_time {
                    continue;
                }
                let sighting = Sighting {
                    pid: process.pid,
                    start_time: process.start_time,
                    boot_id: boot_id.to_owned(),
                    exe_name: super::roles::program_name(exe).to_owned(),
                    cmd_hash: cmd_hash(salt, cmd),
                    tool: session.tool.clone(),
                    session_id: session.session_id.clone(),
                    app: Some(session.app.name.clone()),
                    session_root: process.pid == session.root_pid,
                    parent_pid: process.parent_pid,
                    first_seen: now_ms,
                    last_seen: now_ms,
                };
                noted.writes.push(sighting.clone());
                self.known.insert(identity, (sighting, now_ms));
            }
        }
        // The sessions whose own process is gone.
        let mut ended: HashSet<(String, String)> = HashSet::new();
        let mut running: HashSet<(String, String)> = HashSet::new();
        for (sighting, _) in self.known.values() {
            if !sighting.session_root {
                continue;
            }
            let key = (sighting.tool.clone(), sighting.session_id.clone());
            let root_alive = seen_now.contains(&(sighting.pid, sighting.start_time))
                || alive(sighting.pid, sighting.start_time).is_some_and(|live| !live.zombie);
            if root_alive || live_sessions.contains(&key) {
                running.insert(key);
            } else {
                ended.insert(key);
            }
        }
        let mut gone: Vec<(u32, i64)> = Vec::new();
        let mut helpers_alive: HashSet<(String, String)> = HashSet::new();
        for (identity, (sighting, written)) in &mut self.known {
            let key = (sighting.tool.clone(), sighting.session_id.clone());
            if seen_now.contains(identity) {
                if !sighting.session_root {
                    helpers_alive.insert(key);
                }
                continue;
            }
            let Some(live) = alive(identity.0, identity.1) else {
                // Ended: the store keeps it its 7 days; memory holds a
                // session's own process only while it explains a leftover.
                if !sighting.session_root || now_ms - sighting.last_seen > SIGHTING_KEEP_MS {
                    gone.push(*identity);
                }
                continue;
            };
            if !sighting.session_root {
                helpers_alive.insert(key.clone());
            }
            sighting.last_seen = now_ms;
            if now_ms - *written >= TOUCH_EVERY_MS {
                *written = now_ms;
                noted.writes.push(sighting.clone());
            }
            if sighting.session_root || running.contains(&key) || !ended.contains(&key) {
                continue;
            }
            noted.leftovers.push(LeftoverInput {
                sighting: sighting.clone(),
                live,
            });
        }
        for identity in gone {
            self.known.remove(&identity);
        }
        // An ended session with no helper left needs no memory.
        self.known.retain(|identity, (sighting, _)| {
            !sighting.session_root
                || seen_now.contains(identity)
                || helpers_alive.contains(&(sighting.tool.clone(), sighting.session_id.clone()))
                || alive(identity.0, identity.1).is_some()
        });
        noted
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::processes::model::AppRef;

    fn live(pid: u32, start: i64, parent: u32) -> Live {
        Live {
            pid,
            start_time: start,
            parent_pid: Some(parent),
            name: format!("p{pid}"),
            ..Live::default()
        }
    }

    fn session(tree: Vec<Live>) -> TheirsInput {
        TheirsInput {
            tool: "tool-a".into(),
            session_id: "conv-1".into(),
            title: "Fix the login".into(),
            app: AppRef {
                name: "An app".into(),
                ..AppRef::default()
            },
            root_pid: 10,
            root_start: 100,
            tree,
            ..TheirsInput::default()
        }
    }

    fn commands() -> HashMap<u32, (String, String, i64)> {
        HashMap::from([
            (10, ("/x/tool".into(), "tool --resume abc".into(), 100)),
            (
                11,
                ("/usr/bin/node".into(), "node vite --port 5173".into(), 110),
            ),
        ])
    }

    #[test]
    fn the_hash_is_keyed_and_never_the_line() {
        let line = "node vite --port 5173 --token=secret";
        let a = cmd_hash(b"key-one-key-one-", line);
        let b = cmd_hash(b"key-two-key-two-", line);
        assert_ne!(a, b, "another install's key gives another hash");
        assert_eq!(a, cmd_hash(b"key-one-key-one-", line));
        assert_eq!(a.len(), 32);
        assert!(!a.contains("vite") && !a.contains("secret"));
    }

    #[test]
    fn a_helper_alive_after_its_session_ended_is_left_behind() {
        let mut memory = Memory::default();
        let tree = vec![live(10, 100, 1), live(11, 110, 10)];
        let both = |pid: u32, start: i64| match (pid, start) {
            (10, 100) => Some(live(10, 100, 1)),
            (11, 110) => Some(live(11, 110, 1)),
            _ => None,
        };
        let open = HashSet::from([("tool-a".to_owned(), "conv-1".to_owned())]);
        let first = memory.note(
            &[session(tree)],
            &commands(),
            &both,
            &open,
            b"k",
            "b1",
            200_000,
        );
        assert_eq!(first.writes.len(), 2, "two new sightings");
        assert!(first.writes.iter().all(|s| s.boot_id == "b1"));
        assert!(first.leftovers.is_empty());
        // The session closed; its helper still runs (reparented).
        let helper_only =
            |pid: u32, start: i64| (pid == 11 && start == 110).then(|| live(11, 110, 1));
        let later = memory.note(
            &[],
            &commands(),
            &helper_only,
            &HashSet::new(),
            b"k",
            "b1",
            205_000,
        );
        assert_eq!(later.leftovers.len(), 1);
        assert_eq!(later.leftovers[0].sighting.session_id, "conv-1");
        assert!(!later.leftovers[0].sighting.session_root);
        // A reused pid (another start time) is never the leftover.
        let reused = |pid: u32, start: i64| (pid == 11 && start == 999).then(|| live(11, 999, 1));
        let none = memory.note(
            &[],
            &commands(),
            &reused,
            &HashSet::new(),
            b"k",
            "b1",
            206_000,
        );
        assert!(none.leftovers.is_empty());
    }

    #[test]
    fn a_sighting_waits_for_its_own_command_line_and_is_written_once_a_minute() {
        let mut memory = Memory::default();
        let tree = vec![live(10, 100, 1)];
        let alive = |pid: u32, start: i64| (pid == 10 && start == 100).then(|| live(10, 100, 1));
        let open = HashSet::from([("tool-a".to_owned(), "conv-1".to_owned())]);
        let stale = HashMap::from([(10, ("/x/tool".into(), "other".into(), 55))]);
        let noted = memory.note(
            &[session(tree.clone())],
            &stale,
            &alive,
            &open,
            b"k",
            "b1",
            190_000,
        );
        assert!(noted.writes.is_empty(), "not this process's line: not yet");
        let noted = memory.note(
            &[session(tree.clone())],
            &commands(),
            &alive,
            &open,
            b"k",
            "b1",
            200_000,
        );
        assert_eq!(noted.writes.len(), 1);
        let noted = memory.note(
            &[session(tree.clone())],
            &commands(),
            &alive,
            &open,
            b"k",
            "b1",
            229_000,
        );
        assert!(noted.writes.is_empty(), "last seen waits a minute");
        let noted = memory.note(
            &[session(tree)],
            &commands(),
            &alive,
            &open,
            b"k",
            "b1",
            261_000,
        );
        assert_eq!(noted.writes.len(), 1);
        assert_eq!(noted.writes[0].first_seen, 200_000);
        assert_eq!(noted.writes[0].last_seen, 261_000);
    }
}
