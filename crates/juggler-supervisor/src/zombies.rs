//! AJ-E2401: the finished processes that wait for their parent to collect
//! them (zombies, `Z` in `ps`), on a Mac.
//!
//! WHY a read of its own: `sysinfo` on macOS never lists a zombie. It
//! needs a process's program path or arguments to keep it, and the kernel
//! gives neither for a zombie, so a session's zombie never reached the
//! Processes view (processes lab 2026-10-26, bug 4: `zombies: 0` while
//! `ps` showed one for minutes). The kernel still lists them, so the
//! system's own `/bin/ps` (C locale, names only, never an argument or the
//! environment) says which pids are zombies and whose children they are.
//! On other systems `sysinfo` reports zombies itself, and this reads
//! nothing.
//!
//! Cost: one `ps` every full scan (10 s), only while a session's or an
//! observed tool's tree is watched, bounded by [`READ_TIMEOUT`].
//!
//! What it is NOT: an identity. A zombie's start time is `ps`'s elapsed
//! time from now, kept as first seen so it stays the same between reads;
//! it is shown, never written to the spawn ledger, verified or stopped
//! (only its parent can clear it).

use std::collections::HashMap;
use std::time::Duration;

/// How long one `ps` may take before it is given up (the tick goes on).
pub const READ_TIMEOUT: Duration = Duration::from_secs(2);

/// One zombie: its pid, its parent (the only process that can clear it),
/// its start time (approximate, see the module note) and its name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Zombie {
    /// Its pid.
    pub pid: u32,
    /// Its parent's pid.
    pub ppid: u32,
    /// When it started, epoch seconds (from `ps`'s elapsed time).
    pub start_time: i64,
    /// Its program's short name (the kernel's, up to 16 characters).
    pub name: String,
}

/// The zombies as last read, by pid, and by parent.
#[derive(Debug, Default)]
pub struct Zombies {
    by_pid: HashMap<u32, Zombie>,
    by_parent: HashMap<u32, Vec<u32>>,
}

impl Zombies {
    /// Reads them again (a Mac only; elsewhere nothing). A zombie already
    /// known under the same parent keeps its first start time.
    pub fn refresh(&mut self, now_secs: i64) {
        let Some(out) = read_ps() else {
            return; // couldn't read this time: keep the last read
        };
        self.replace(parse(&out, now_secs));
    }

    /// Forgets them (nothing is watched).
    pub fn clear(&mut self) {
        self.by_pid.clear();
        self.by_parent.clear();
    }

    /// Takes a new read, keeping each known zombie's first start time.
    pub fn replace(&mut self, read: Vec<Zombie>) {
        let mut by_pid = HashMap::with_capacity(read.len());
        let mut by_parent: HashMap<u32, Vec<u32>> = HashMap::new();
        for mut zombie in read {
            if let Some(known) = self.by_pid.get(&zombie.pid)
                && known.ppid == zombie.ppid
            {
                zombie.start_time = known.start_time;
            }
            by_parent.entry(zombie.ppid).or_default().push(zombie.pid);
            by_pid.insert(zombie.pid, zombie);
        }
        self.by_pid = by_pid;
        self.by_parent = by_parent;
    }

    /// The zombie `pid` is, when it is one (under `parent`, when given).
    #[must_use]
    pub fn get(&self, pid: u32, parent: Option<u32>) -> Option<&Zombie> {
        self.by_pid
            .get(&pid)
            .filter(|zombie| parent.is_none_or(|parent| parent == zombie.ppid))
    }

    /// The zombies `pid` hasn't collected.
    pub fn children_of(&self, pid: u32) -> impl Iterator<Item = &Zombie> {
        self.by_parent
            .get(&pid)
            .into_iter()
            .flatten()
            .filter_map(|child| self.by_pid.get(child))
    }

    /// How many there are.
    #[must_use]
    pub fn len(&self) -> usize {
        self.by_pid.len()
    }

    /// None at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_pid.is_empty()
    }
}

/// `ps`'s elapsed time (`[[dd-]hh:]mm:ss`) in seconds.
#[must_use]
pub fn elapsed_secs(text: &str) -> Option<i64> {
    let (days, rest) = match text.split_once('-') {
        Some((days, rest)) => (days.parse::<i64>().ok()?, rest),
        None => (0, text),
    };
    let mut secs = 0_i64;
    let parts: Vec<&str> = rest.split(':').collect();
    if !(2..=3).contains(&parts.len()) {
        return None;
    }
    for part in parts {
        secs = secs * 60 + part.parse::<i64>().ok()?;
    }
    Some(days * 86_400 + secs)
}

/// The zombies in `ps -A -o pid=,ppid=,stat=,etime=,ucomm=`'s output.
#[must_use]
pub fn parse(out: &str, now_secs: i64) -> Vec<Zombie> {
    out.lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let pid = fields.next()?.parse::<u32>().ok()?;
            let parent = fields.next()?.parse::<u32>().ok()?;
            let stat = fields.next()?;
            if !stat.starts_with('Z') {
                return None;
            }
            let elapsed = elapsed_secs(fields.next()?)?;
            let name = fields.collect::<Vec<_>>().join(" ");
            Some(Zombie {
                pid,
                ppid: parent,
                start_time: now_secs - elapsed,
                name,
            })
        })
        .collect()
}

/// One `ps` of every process's pid, parent, state, age and short name.
#[cfg(target_os = "macos")]
fn read_ps() -> Option<String> {
    use std::io::Read;
    let mut child = std::process::Command::new("/bin/ps")
        .args(["-A", "-o", "pid=,ppid=,stat=,etime=,ucomm="])
        .env_clear()
        .env("LC_ALL", "C")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .ok()?;
    let mut stdout = child.stdout.take()?;
    // Read while it writes (a full pipe would stall it), then collect it.
    let reader = std::thread::spawn(move || {
        let mut out = String::new();
        let _ = stdout.read_to_string(&mut out);
        out
    });
    let deadline = std::time::Instant::now() + READ_TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if std::time::Instant::now() < deadline => {
                // poll: one `ps`'s exit, bounded by READ_TIMEOUT.
                std::thread::sleep(Duration::from_millis(10));
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
    reader.join().ok()
}

/// Elsewhere `sysinfo` lists zombies itself.
#[cfg(not(target_os = "macos"))]
fn read_ps() -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn elapsed_times_read_as_ps_writes_them() {
        assert_eq!(elapsed_secs("00:02"), Some(2));
        assert_eq!(elapsed_secs("01:02:03"), Some(3_723));
        assert_eq!(elapsed_secs("2-01:02:03"), Some(2 * 86_400 + 3_723));
        assert_eq!(elapsed_secs("x"), None);
        assert_eq!(elapsed_secs("1:2:3:4"), None);
    }

    #[test]
    fn only_zombies_are_read_and_keep_their_first_start() {
        let out = "    1     0 Ss   08-23:17:13 launchd\n62659 62658 Z          00:01 true            \n  700   650 R+      01:00 python3\n";
        let read = parse(out, 1_000);
        assert_eq!(
            read,
            [Zombie {
                pid: 62_659,
                ppid: 62_658,
                start_time: 999,
                name: "true".into(),
            }]
        );
        let mut zombies = Zombies::default();
        zombies.replace(read);
        // A second read a second later says 2 s: the start stays.
        zombies.replace(parse(out.replace("00:01", "00:02").as_str(), 1_001));
        assert_eq!(zombies.get(62_659, Some(62_658)).unwrap().start_time, 999);
        assert_eq!(zombies.children_of(62_658).count(), 1);
        assert!(zombies.get(62_659, Some(1)).is_none(), "another parent");
    }

    /// The real `ps`, with a real zombie: a child of this test that has
    /// finished and isn't collected yet. Bounded: it is collected at once.
    #[cfg(target_os = "macos")]
    #[test]
    fn a_real_zombie_is_read_with_its_parent() {
        let mut child = std::process::Command::new("/usr/bin/true")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let pid = child.id();
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        let mut zombies = Zombies::default();
        while std::time::Instant::now() < deadline {
            zombies.refresh(0);
            if zombies.get(pid, Some(std::process::id())).is_some() {
                break;
            }
            // poll: the child's exit, bounded by the deadline above.
            std::thread::sleep(Duration::from_millis(50));
        }
        let found = zombies.get(pid, Some(std::process::id())).cloned();
        let _ = child.wait();
        let found = found.expect("the finished child is read as a zombie");
        assert_eq!(found.name, "true");
        zombies.refresh(0);
        assert!(zombies.get(pid, None).is_none(), "collected: gone");
    }
}
