//! Every process on this Mac, listed cheaply: its pid, its parent, when it
//! started, its short name and whether it is a zombie (decision D-108,
//! 2026-11-13, AJ-E805's idle CPU).
//!
//! WHY: `sysinfo` reads a process's arguments and environment
//! (`KERN_PROCARGS2`, twice) and three more kernel records on every
//! refresh, whatever it is asked for, so reading "which processes are
//! there" meant reading everything about all of them (about 850 on the
//! reference Mac). AI Juggler did that several times every 10 s (the 2 s
//! collector's listing and full scan, presence's 5 s look, the live view's
//! scan): half of its idle CPU in the lab (`docs/qa/evidence/AJ-E805/
//! idle-0.4.5.md`). With this listing, a process's arguments are read once,
//! when it first appears, and only the processes AI Juggler measures are
//! read again. It also lists the zombies `sysinfo` never lists on a Mac
//! (which took a `/bin/ps` before).
//!
//! HOW: one `sysctl(CTL_KERN, KERN_PROC, KERN_PROC_ALL)`, the call `ps`
//! makes: the kernel's `kinfo_proc` record of every process, whoever owns
//! it, zombies too (`proc_pidinfo` refuses both). `libc` has no
//! `kinfo_proc`, so the four fields read are taken at their offsets in its
//! fixed 64-bit layout ([`macos::LAYOUT`]), and every read checks itself:
//! a record size other than the layout's, or a listing in which this
//! process isn't found under its own parent, is no listing (`None`, and
//! the callers read the table their old way).
//!
//! THE AUDIT: two `unsafe` blocks, each with its SAFETY note: the
//! `sysctl` asking how many bytes the records take (a null buffer: nothing
//! written), and the `sysctl` writing them into a byte buffer whose size
//! the kernel is told. Reading the fields is safe code over those bytes.
//!
//! Nothing here reads a process's arguments, environment, memory or files.
//!
//! What this crate is NOT: a general unsafe escape hatch. It exists so the
//! workspace law `unsafe_code = "forbid"` stays absolute for every other
//! crate; it doesn't inherit the workspace lint baseline (see
//! `Cargo.toml`). Any other `unsafe` anywhere needs a new recorded
//! decision.
//!
//! Must not know: anything of AI Juggler: it is an audited shim.
#![deny(missing_docs, unsafe_op_in_unsafe_fn)]
#![cfg_attr(
    target_os = "macos",
    allow(unsafe_code, reason = "D-108: the two audited blocks")
)]

/// One process, as the kernel lists it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Listed {
    /// Its pid.
    pub pid: u32,
    /// Its parent's pid (0 for the kernel's own).
    pub ppid: u32,
    /// When it started, epoch seconds.
    pub start_secs: u64,
    /// The microseconds within that second (a pid reused within one
    /// second is still told apart).
    pub start_micros: u64,
    /// Its short name (the kernel's `p_comm`: the program's file name, up to 16 bytes).
    pub name: String,
    /// It has ended and waits for its parent to collect it.
    pub zombie: bool,
}

/// Every process now, in no order. `None` when the kernel doesn't say,
/// and on other systems (the caller reads the table its old way).
#[must_use]
pub fn list() -> Option<Vec<Listed>> {
    #[cfg(target_os = "macos")]
    {
        macos::list()
    }
    #[cfg(not(target_os = "macos"))]
    {
        None
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use super::Listed;

    /// Where the fields read sit in a `kinfo_proc` (`sys/sysctl.h`:
    /// `struct extern_proc kp_proc; struct eproc kp_eproc;`), 64-bit.
    pub(super) struct Layout {
        /// The record's size.
        pub size: usize,
        /// `kp_proc.p_starttime.tv_sec` (`i64`; `p_un` is its first field).
        pub start_secs: usize,
        /// `kp_proc.p_starttime.tv_usec` (`i32`).
        pub start_micros: usize,
        /// `kp_proc.p_stat` (one byte).
        pub stat: usize,
        /// `kp_proc.p_pid` (`i32`).
        pub pid: usize,
        /// `kp_proc.p_comm` (`MAXCOMLEN + 1` bytes, NUL-padded).
        pub comm: usize,
        /// `kp_eproc.e_ppid` (`i32`).
        pub ppid: usize,
    }

    /// The layout on every 64-bit Mac (`arm64` and `x86_64` alike).
    pub(super) const LAYOUT: Layout = Layout {
        size: 648,
        start_secs: 0,
        start_micros: 8,
        stat: 36,
        pid: 40,
        comm: 243,
        ppid: 560,
    };

    /// The kernel's zombie state (`SZOMB` in `sys/proc.h`).
    const ZOMBIE: u8 = 5;
    /// `MAXCOMLEN + 1`.
    const COMM: usize = 17;

    /// Every process's record, as bytes: asked how much room they take,
    /// then read into that much and a little more (processes start between
    /// the two calls); a buffer too small is asked again, at most three
    /// times.
    fn records() -> Option<Vec<u8>> {
        let mut mib = [libc::CTL_KERN, libc::KERN_PROC, libc::KERN_PROC_ALL];
        let depth = libc::c_uint::try_from(mib.len()).ok()?;
        for _ in 0..3 {
            let mut size: libc::size_t = 0;
            // SAFETY: a null buffer only asks how many bytes the records
            // take; the kernel writes that into `size`, a live `size_t`.
            let asked = unsafe {
                libc::sysctl(
                    mib.as_mut_ptr(),
                    depth,
                    std::ptr::null_mut(),
                    &raw mut size,
                    std::ptr::null_mut(),
                    0,
                )
            };
            if asked != 0 {
                return None;
            }
            let room = size.checked_add(64 * LAYOUT.size)?;
            let mut bytes = vec![0_u8; room];
            let mut filled: libc::size_t = room;
            // SAFETY: `bytes` holds `room` live bytes and `filled` tells
            // the kernel so: it writes at most that many and sets `filled`
            // to how many it wrote (failing with ENOMEM, writing nothing
            // usable, when they don't fit). Nothing is kept past the call.
            let read = unsafe {
                libc::sysctl(
                    mib.as_mut_ptr(),
                    depth,
                    bytes.as_mut_ptr().cast(),
                    &raw mut filled,
                    std::ptr::null_mut(),
                    0,
                )
            };
            if read == 0 {
                bytes.truncate(filled);
                return Some(bytes);
            }
        }
        None
    }

    fn i32_at(record: &[u8], at: usize) -> Option<i32> {
        Some(i32::from_ne_bytes(record.get(at..at + 4)?.try_into().ok()?))
    }

    fn i64_at(record: &[u8], at: usize) -> Option<i64> {
        Some(i64::from_ne_bytes(record.get(at..at + 8)?.try_into().ok()?))
    }

    /// One record's fields.
    pub(super) fn parse(record: &[u8]) -> Option<Listed> {
        let comm = record.get(LAYOUT.comm..LAYOUT.comm + COMM)?;
        let comm: Vec<u8> = comm.iter().copied().take_while(|byte| *byte != 0).collect();
        Some(Listed {
            pid: u32::try_from(i32_at(record, LAYOUT.pid)?).ok()?,
            ppid: u32::try_from(i32_at(record, LAYOUT.ppid)?).ok()?,
            start_secs: u64::try_from(i64_at(record, LAYOUT.start_secs)?).ok()?,
            start_micros: u64::try_from(i32_at(record, LAYOUT.start_micros)?).ok()?,
            name: String::from_utf8_lossy(&comm).into_owned(),
            zombie: *record.get(LAYOUT.stat)? == ZOMBIE,
        })
    }

    pub(super) fn list() -> Option<Vec<Listed>> {
        let bytes = records()?;
        let (records, rest) = bytes.as_chunks::<{ LAYOUT.size }>();
        if !rest.is_empty() {
            return None;
        }
        let listed: Vec<Listed> = records.iter().filter_map(|record| parse(record)).collect();
        // The layout checks itself: this process, under its own parent.
        let me = std::process::id();
        let parent = std::os::unix::process::parent_id();
        listed
            .iter()
            .any(|p| p.pid == me && p.ppid == parent && p.start_secs > 0)
            .then_some(listed)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, reason = "test code")]

    /// This process is listed with its parent, its start time and its
    /// name, as `ps` says them.
    #[test]
    #[cfg(target_os = "macos")]
    fn this_process_is_listed_as_ps_says() {
        let me = std::process::id();
        let listed = super::list().expect("the kernel lists processes");
        let mine = listed
            .iter()
            .find(|p| p.pid == me)
            .expect("this process is listed");
        let out = std::process::Command::new("/bin/ps")
            .args(["-o", "ppid=,comm=", "-p", &me.to_string()])
            .env("LC_ALL", "C")
            .output()
            .unwrap();
        let text = String::from_utf8_lossy(&out.stdout);
        let (ppid, comm) = text.trim().split_once(char::is_whitespace).unwrap();
        assert_eq!(mine.ppid, ppid.trim().parse::<u32>().unwrap());
        let program = std::path::Path::new(comm.trim())
            .file_name()
            .unwrap()
            .to_string_lossy();
        assert!(
            program.starts_with(&mine.name) || mine.name.starts_with(program.as_ref()),
            "{} vs {program}",
            mine.name
        );
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert!(
            mine.start_secs <= now && now - mine.start_secs < 3600,
            "started {}",
            mine.start_secs
        );
        assert!(mine.start_micros < 1_000_000);
        assert!(!mine.zombie);
        // As many processes as `ps` sees, give or take what starts and ends.
        let ps = std::process::Command::new("/bin/ps")
            .args(["-ax", "-o", "pid="])
            .output()
            .unwrap();
        let theirs = String::from_utf8_lossy(&ps.stdout).lines().count();
        assert!(
            listed.len().abs_diff(theirs) < theirs / 10 + 20,
            "listed {}, ps {theirs}",
            listed.len()
        );
    }

    /// A child that ended and wasn't collected is listed as a zombie under
    /// this process (`sysinfo` never lists one on a Mac); once collected,
    /// it is gone.
    #[test]
    #[cfg(target_os = "macos")]
    fn an_uncollected_child_is_a_zombie_and_a_collected_one_is_gone() {
        let mut child = std::process::Command::new("/usr/bin/true").spawn().unwrap();
        let pid = child.id();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        let zombie = loop {
            let found = super::list().unwrap().into_iter().find(|p| p.pid == pid);
            if found.as_ref().is_some_and(|p| p.zombie) || std::time::Instant::now() > deadline {
                break found;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        };
        let zombie = zombie.expect("the ended child is still listed");
        assert!(zombie.zombie, "{zombie:?}");
        assert_eq!(zombie.ppid, std::process::id());
        child.wait().unwrap();
        assert!(super::list().unwrap().iter().all(|p| p.pid != pid));
    }

    #[test]
    #[cfg(not(target_os = "macos"))]
    fn elsewhere_it_lists_nothing() {
        assert_eq!(super::list(), None);
    }
}
