//! Every process on this Mac, listed cheaply: its pid, its parent, when it
//! started, its short name and whether it is a zombie (decision D-110,
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
//! `kinfo_proc`, so the fields read are taken at their offsets in its
//! fixed 64-bit layout (`macos::LAYOUT`), and every read checks itself:
//! a record size other than the layout's, or a listing in which this
//! process isn't found under its own parent, is no listing (`None`, and
//! the callers read the table their old way).
//!
//! [`task`] reads one process's resident memory and the CPU time it has
//! used (`proc_pidinfo(PROC_PIDTASKINFO)`, the record `sysinfo` takes them
//! from, without the arguments `sysinfo` reads with them): This Mac's
//! other apps are measured from it on every full scan, and the observed
//! tools' trees (the Claude app's ~300 processes on a busy Mac) at every
//! listing. The record's CPU times are in Mach time units: nanoseconds on
//! an Intel Mac, 24 MHz ticks on Apple silicon. They're converted with
//! Mach's own timebase (`mach_timebase_info`: 125/3 nanoseconds a tick on
//! Apple silicon, 1/1 on Intel), read once.
//!
//! [`pid_version`] (AJ-E3013, L4) reads one process's pid version
//! (`proc_pidinfo(PROC_PIDUNIQIDENTIFIERINFO)`'s `p_idversion`): the
//! number the kernel bumps each time a pid is reused, which a local
//! socket's audit token (`LOCAL_PEERTOKEN`) carries, so the bridge can tell
//! the process that connected from one that took its pid since.
//!
//! THE AUDIT: seven `unsafe` blocks, each with its SAFETY note: the
//! `sysctl` asking how many bytes the records take (a null buffer: nothing
//! written), and the `sysctl` writing them into a byte buffer whose size
//! the kernel is told; zeroing one `proc_taskinfo`, and `proc_pidinfo`
//! writing into it at its size; zeroing one `proc_uniqidentifierinfo`, and
//! `proc_pidinfo` writing into it at its size (AJ-E3013 L4, under D-110's
//! rules: the coordinating session's ruling, 2026-11-15);
//! `mach_timebase_info` writing its two integers into a live record, once
//! per run. Reading the fields is safe code.
//!
//! Nothing here reads a process's arguments, environment, files or the
//! contents of its memory.
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
    allow(
        unsafe_code,
        reason = "D-110: the seven audited blocks (AJ-E3013 L4 added two)"
    )
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
    /// It has ended, or is ending (the kernel's `P_WEXIT`), and waits for
    /// its parent to collect it: not a running process any more.
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

/// One process's task, as the kernel counts it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Task {
    /// Its resident memory, bytes.
    pub resident: u64,
    /// The CPU time it has used, user and system, nanoseconds.
    pub cpu_ns: u64,
}

/// The resident memory and CPU time of `pid`. `None` when it's gone, isn't
/// this user's to read (the kernel refuses another user's), and on other
/// systems.
#[must_use]
pub fn task(pid: u32) -> Option<Task> {
    #[cfg(target_os = "macos")]
    {
        macos::task(pid)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = pid;
        None
    }
}

/// AJ-E3013 (L4): `pid`'s pid version (`p_idversion`): changes when the pid
/// is reused, and equals a local socket peer's audit-token pid version
/// while that peer is the process at `pid`. `None` when it's gone, isn't
/// this user's to read, and on other systems.
#[must_use]
pub fn pid_version(pid: u32) -> Option<u32> {
    #[cfg(target_os = "macos")]
    {
        macos::pid_version(pid)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = pid;
        None
    }
}

/// Mach time `ticks` in nanoseconds, at `numer`/`denom` nanoseconds a
/// tick (Apple silicon 125/3, Intel 1/1); past `u64`, `u64::MAX`.
#[cfg_attr(
    not(target_os = "macos"),
    allow(dead_code, reason = "macOS's task read")
)]
fn nanos(ticks: u128, numer: u32, denom: u32) -> u64 {
    let nanos = ticks * u128::from(numer) / u128::from(denom.max(1));
    u64::try_from(nanos).unwrap_or(u64::MAX)
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
        /// `kp_proc.p_flag` (`i32`).
        pub flag: usize,
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
        flag: 32,
        stat: 36,
        pid: 40,
        comm: 243,
        ppid: 560,
    };

    /// The kernel's zombie state (`SZOMB` in `sys/proc.h`).
    const ZOMBIE: u8 = 5;
    /// The kernel's "working on exiting" flag (`P_WEXIT` in `sys/proc.h`).
    const EXITING: i32 = 0x0000_2000;
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
            zombie: *record.get(LAYOUT.stat)? == ZOMBIE
                || i32_at(record, LAYOUT.flag)? & EXITING != 0,
        })
    }

    pub(super) fn task(pid: u32) -> Option<super::Task> {
        let pid = libc::c_int::try_from(pid).ok()?;
        let (numer, denom) = timebase()?;
        // SAFETY: an all-zero `proc_taskinfo` is a valid value (plain
        // integers).
        let mut info: libc::proc_taskinfo = unsafe { std::mem::zeroed() };
        let size = libc::c_int::try_from(std::mem::size_of::<libc::proc_taskinfo>()).ok()?;
        // SAFETY: `info` is a live `proc_taskinfo` and the kernel is told
        // its size, so it writes at most that much; it returns how many
        // bytes it wrote (anything else: nothing usable). Nothing is kept
        // past the call.
        let wrote = unsafe {
            libc::proc_pidinfo(pid, libc::PROC_PIDTASKINFO, 0, (&raw mut info).cast(), size)
        };
        if wrote != size {
            return None;
        }
        let ticks = u128::from(info.pti_total_user) + u128::from(info.pti_total_system);
        Some(super::Task {
            resident: info.pti_resident_size,
            cpu_ns: super::nanos(ticks, numer, denom),
        })
    }

    /// `proc_uniqidentifierinfo` (`sys/proc_info.h`), which libc doesn't
    /// declare: the executable's UUID, the process's and its parent's
    /// unique ids, the pid version, and reserved words.
    #[repr(C)]
    struct UniqIdentifierInfo {
        p_uuid: [u8; 16],
        p_uniqueid: u64,
        p_puniqueid: u64,
        p_idversion: i32,
        p_reserve2: u32,
        p_reserve3: u64,
        p_reserve4: u64,
    }

    /// `PROC_PIDUNIQIDENTIFIERINFO` (`sys/proc_info.h`).
    const PROC_PIDUNIQIDENTIFIERINFO: libc::c_int = 17;

    pub(super) fn pid_version(pid: u32) -> Option<u32> {
        let pid = libc::c_int::try_from(pid).ok()?;
        // SAFETY: an all-zero `UniqIdentifierInfo` is a valid value (plain
        // integers and bytes).
        let mut info: UniqIdentifierInfo = unsafe { std::mem::zeroed() };
        let size = libc::c_int::try_from(std::mem::size_of::<UniqIdentifierInfo>()).ok()?;
        // SAFETY: `info` is a live `UniqIdentifierInfo` laid out as the
        // kernel's `proc_uniqidentifierinfo`, and the kernel is told its
        // size, so it writes at most that much; it returns how many bytes it
        // wrote (anything else: nothing usable). Nothing is kept past the
        // call.
        let wrote = unsafe {
            libc::proc_pidinfo(
                pid,
                PROC_PIDUNIQIDENTIFIERINFO,
                0,
                (&raw mut info).cast(),
                size,
            )
        };
        if wrote != size {
            return None;
        }
        u32::try_from(info.p_idversion).ok()
    }

    /// Mach time's nanoseconds a tick, as `(numer, denom)`
    /// (`mach_timebase_info`), read once; `None` when the kernel doesn't
    /// say (then no CPU time is read).
    #[allow(
        deprecated,
        reason = "libc points Mach calls at the mach2 crate; this one call is \
                  the whole need, and libc's own declaration is the audited one"
    )]
    pub(super) fn timebase() -> Option<(u32, u32)> {
        static TIMEBASE: std::sync::OnceLock<Option<(u32, u32)>> = std::sync::OnceLock::new();
        *TIMEBASE.get_or_init(|| {
            let mut info = libc::mach_timebase_info { numer: 0, denom: 0 };
            // SAFETY: `info` is a live `mach_timebase_info` (two `u32`s),
            // the record the call fills; it writes nothing else and keeps
            // nothing past the call.
            let read = unsafe { libc::mach_timebase_info(&raw mut info) };
            (read == 0 && info.numer > 0 && info.denom > 0).then_some((info.numer, info.denom))
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

    /// AJ-E3013 (L4): a live process has a pid version; a gone pid none.
    #[test]
    #[cfg(target_os = "macos")]
    fn a_live_process_has_a_pid_version() {
        assert!(super::pid_version(std::process::id()).is_some());
        assert_eq!(super::pid_version(u32::MAX), None);
    }

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

    /// This process's resident memory is what `ps` says (within its
    /// rounding and what moved between the two reads); a gone pid has none.
    #[test]
    #[cfg(target_os = "macos")]
    fn this_process_resident_memory_is_as_ps_says() {
        let ours = super::task(std::process::id())
            .expect("our own task info")
            .resident;
        let out = std::process::Command::new("/bin/ps")
            .args(["-o", "rss=", "-p", &std::process::id().to_string()])
            .output()
            .unwrap();
        let theirs: u64 = String::from_utf8_lossy(&out.stdout)
            .trim()
            .parse::<u64>()
            .unwrap()
            * 1024;
        assert!(
            ours.abs_diff(theirs) < theirs / 4 + 4 * 1024 * 1024,
            "ours {ours}, ps {theirs}"
        );
        let mut child = std::process::Command::new("/usr/bin/true").spawn().unwrap();
        let pid = child.id();
        child.wait().unwrap();
        assert_eq!(super::task(pid), None, "gone");
    }

    /// Ticks to nanoseconds at each Mac's ratio: Apple silicon's 24 MHz
    /// tick is 125/3 ns, Intel's is a nanosecond; past `u64`, saturated.
    #[test]
    fn mach_ticks_are_converted_at_each_macs_ratio() {
        assert_eq!(super::nanos(24_000_000, 125, 3), 1_000_000_000);
        assert_eq!(super::nanos(3, 125, 3), 125);
        assert_eq!(super::nanos(1_000_000_000, 1, 1), 1_000_000_000);
        assert_eq!(super::nanos(7, 1, 1), 7);
        assert_eq!(super::nanos(u128::MAX / 200, 125, 3), u64::MAX);
        assert_eq!(super::nanos(5, 1, 0), 5, "a zero denominator reads as one");
    }

    /// This Mac's timebase is Mach's (`mach_timebase_info`), and agrees
    /// with the tick rate the kernel names (`sysctl hw.tbfrequency`).
    #[test]
    #[cfg(target_os = "macos")]
    fn the_timebase_agrees_with_the_kernels_tick_rate() {
        let (numer, denom) = super::macos::timebase().expect("Mach's timebase");
        let out = std::process::Command::new("/usr/sbin/sysctl")
            .args(["-n", "hw.tbfrequency"])
            .output()
            .unwrap();
        let ticks_a_second: u64 = String::from_utf8_lossy(&out.stdout).trim().parse().unwrap();
        assert_eq!(
            super::nanos(u128::from(ticks_a_second), numer, denom),
            1_000_000_000,
            "{numer}/{denom} ns a tick at {ticks_a_second} ticks a second"
        );
        if cfg!(target_arch = "aarch64") {
            assert_eq!((numer, denom), (125, 3));
        }
    }

    /// A busy child's CPU time, read through [`super::task`], grows with
    /// the wall clock (a spinning process takes up to one core): the
    /// record's Mach ticks are converted to nanoseconds (unconverted, a
    /// 24 MHz tick reads 42 times too low), and its total is what `ps`
    /// says.
    #[test]
    #[cfg(target_os = "macos")]
    fn a_busy_childs_cpu_time_follows_the_wall_clock() {
        use std::time::{Duration, Instant};
        // Busy (bounded, no forks), killed however the checks end.
        let mut child = std::process::Command::new("/bin/sh")
            .args(["-c", "i=0; while [ $i -lt 30000000 ]; do i=$((i+1)); done"])
            .spawn()
            .unwrap();
        let pid = child.id();
        let checked = std::panic::catch_unwind(|| {
            let read = || {
                (
                    Instant::now(),
                    super::task(pid).expect("the child's task").cpu_ns,
                )
            };
            std::thread::sleep(Duration::from_millis(300));
            let (from_at, from_ns) = read();
            std::thread::sleep(Duration::from_millis(1_500));
            let (to_at, to_ns) = read();
            let wall = to_at.duration_since(from_at).as_nanos() as f64;
            let share = (to_ns - from_ns) as f64 / wall;
            assert!(
                (0.5..=1.1).contains(&share),
                "CPU {} ns over {wall} ns of wall time: {share}",
                to_ns - from_ns
            );
            // Its total, as `ps` says it (`[mm:]ss.cc`, to the hundredth).
            let out = std::process::Command::new("/bin/ps")
                .args(["-o", "time=", "-p", &pid.to_string()])
                .env("LC_ALL", "C")
                .output()
                .unwrap();
            let ours = super::task(pid).unwrap().cpu_ns as f64 / 1e9;
            let theirs = String::from_utf8_lossy(&out.stdout)
                .trim()
                .split(':')
                .fold(0.0, |total, part| {
                    total * 60.0 + part.parse::<f64>().unwrap()
                });
            assert!(
                (ours - theirs).abs() < 0.1 + theirs * 0.05,
                "ours {ours} s, ps {theirs} s"
            );
        });
        let _ = child.kill();
        let _ = child.wait();
        if let Err(failed) = checked {
            std::panic::resume_unwind(failed);
        }
    }

    #[test]
    #[cfg(not(target_os = "macos"))]
    fn elsewhere_it_lists_nothing() {
        assert_eq!(super::list(), None);
        assert_eq!(super::task(1), None);
    }
}
