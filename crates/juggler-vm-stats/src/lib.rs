//! The pages macOS's memory compressor occupies (decision D-27,
//! 2026-10-23, the maintainer's approval of one audited
//! `host_statistics64` call).
//!
//! WHY: This Mac's "memory used" is app memory + wired + compressed, as
//! Activity Monitor and `vm_stat` count it. The compressed part is the
//! pages the compressor occupies (`vm_statistics64.compressor_page_count`).
//! No sysctl gives that; `vm.compressor_bytes_used` is the compressed data
//! inside those pages, and reading it left memory used about 3 % of
//! physical memory low on the maintainer's Mac (the 0.1 real-data pass,
//! 3.4). Only `host_statistics64(HOST_VM_INFO64)` reports the page count.
//!
//! THE AUDIT: three `unsafe` blocks, each with its SAFETY note.
//! - `mach_host_self()` once, for the host's name port, kept for the
//!   process's life (one send right, never released: the port is the
//!   host's, it exists as long as the system does, and asking once avoids
//!   taking a new right on every read).
//! - zeroing the `vm_statistics64` it fills (plain integers);
//! - `host_statistics64` into it, at the size the kernel is told, which
//!   it fills or reports failing.
//!
//! What this crate is NOT: a general unsafe escape hatch. It exists so the
//! workspace law `unsafe_code = "forbid"` stays absolute for every other
//! crate; it doesn't inherit the workspace lint baseline (see
//! `Cargo.toml`). Any other `unsafe` anywhere needs a new recorded
//! decision.
#![deny(missing_docs, unsafe_op_in_unsafe_fn)]
#![cfg_attr(target_os = "macos", allow(unsafe_code))] // D-27: the three audited blocks

/// How many pages the memory compressor occupies now (`vm_stat`'s "Pages
/// occupied by compressor"), in the kernel's page size (`hw.pagesize`).
/// `None` when the kernel doesn't say, and on other systems.
#[must_use]
pub fn compressor_pages() -> Option<u64> {
    #[cfg(target_os = "macos")]
    {
        macos::compressor_pages()
    }
    #[cfg(not(target_os = "macos"))]
    {
        None
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use std::sync::OnceLock;

    /// The host's name port, asked for once.
    fn host() -> libc::mach_port_t {
        static HOST: OnceLock<libc::mach_port_t> = OnceLock::new();
        *HOST.get_or_init(|| {
            // SAFETY: `mach_host_self` takes no arguments and only returns
            // the caller's send right to the host port; it can't fail in a
            // way that touches memory (an invalid port is returned as
            // `MACH_PORT_NULL`, which `host_statistics64` then refuses).
            #[allow(deprecated)] // libc points at the mach2 crate; same call
            unsafe {
                libc::mach_host_self()
            }
        })
    }

    pub(super) fn compressor_pages() -> Option<u64> {
        // SAFETY: an all-zero `vm_statistics64` is a valid value (plain
        // integers).
        let mut stats: libc::vm_statistics64 = unsafe { std::mem::zeroed() };
        let mut count = libc::HOST_VM_INFO64_COUNT;
        // SAFETY: `stats` is a live `vm_statistics64` and `count` says its
        // size in `integer_t`s (`HOST_VM_INFO64_COUNT`, the kernel's own
        // constant for this flavor), so the kernel writes at most that
        // many into it; the pointer cast is the documented
        // `host_info64_t` out-parameter. Nothing is kept past the call.
        let status = unsafe {
            libc::host_statistics64(
                host(),
                libc::HOST_VM_INFO64,
                (&raw mut stats).cast::<libc::integer_t>(),
                &raw mut count,
            )
        };
        (status == libc::KERN_SUCCESS).then(|| u64::from(stats.compressor_page_count))
    }
}

#[cfg(test)]
mod tests {
    /// The kernel reports it on a Mac, and it is `vm_stat`'s "Pages
    /// occupied by compressor" (read just after; it moves, so within 2 %).
    #[test]
    #[cfg(target_os = "macos")]
    fn a_mac_reports_the_compressor_pages_vm_stat_does() {
        let ours = super::compressor_pages().expect("host_statistics64 refused");
        let out = std::process::Command::new("/usr/bin/vm_stat")
            .output()
            .unwrap();
        let theirs: u64 = String::from_utf8_lossy(&out.stdout)
            .lines()
            .find_map(|line| line.strip_prefix("Pages occupied by compressor:"))
            .and_then(|count| count.trim().trim_end_matches('.').parse().ok())
            .expect("vm_stat names the compressor's pages");
        let off = ours.abs_diff(theirs) as f64 / theirs.max(1) as f64;
        assert!(off <= 0.02, "ours {ours}, vm_stat {theirs}");
        // Asking again works (the port is kept, not used up).
        assert!(super::compressor_pages().is_some());
    }

    #[test]
    #[cfg(not(target_os = "macos"))]
    fn elsewhere_it_reads_nothing() {
        assert_eq!(super::compressor_pages(), None);
    }
}
