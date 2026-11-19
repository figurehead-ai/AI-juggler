//! 0.4.1: a background thread on the lowest priority the system gives
//! one, so catch-up work (the subagent count's first reads after a start)
//! never competes with the window. The safe API of `thread-priority`.
//!
//! D-131 (2026-11-19): a thread's disk I/O can be made low-priority too
//! ([`background_io`]), so a model download (gigabytes written over
//! minutes) never slows the user's own disk work. macOS has no safe
//! binding for it, so it is this crate's one audited `unsafe` call.

/// Puts the calling thread on the lowest priority. False where the system
/// refused (the work then runs as before, only not lowered).
#[must_use]
pub fn lowest_priority() -> bool {
    thread_priority::set_current_thread_priority(thread_priority::ThreadPriority::Min).is_ok()
}

/// Makes the calling thread's disk I/O low-priority (macOS's throttled
/// tier, `IOPOL_THROTTLE`, for this thread only), and confirms the system
/// took it. False where it didn't, and on other systems (no seam yet): the
/// work then runs as before, only not lowered.
#[must_use]
pub fn background_io() -> bool {
    #[cfg(target_os = "macos")]
    {
        background_io_macos()
    }
    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}

/// D-131's carve-out: the crate's ONLY `unsafe` (a source test,
/// `the_only_unsafe_is_background_io`, keeps it so).
#[cfg(target_os = "macos")]
#[allow(
    unsafe_code,
    reason = "D-131: setiopolicy_np/getiopolicy_np have no safe binding (not in libc or nix); one audited call, SAFETY below"
)]
fn background_io_macos() -> bool {
    use std::ffi::c_int;

    // <sys/resource.h>: the disk I/O type, this thread only, and the
    // throttled tier.
    const IOPOL_TYPE_DISK: c_int = 0;
    const IOPOL_SCOPE_THREAD: c_int = 1;
    const IOPOL_THROTTLE: c_int = 3;

    unsafe extern "C" {
        fn setiopolicy_np(iotype: c_int, scope: c_int, policy: c_int) -> c_int;
        fn getiopolicy_np(iotype: c_int, scope: c_int) -> c_int;
    }

    // SAFETY: both are libSystem functions with these exact C signatures
    // (<sys/resource.h>, macOS 10.5+): three and two `int`s by value, an
    // `int` back, no pointers, no memory handed over or kept, no callback.
    // The constants are the header's. With IOPOL_SCOPE_THREAD they change
    // or read only the calling thread's own I/O policy; a bad argument is
    // reported as -1 (EINVAL), never undefined behaviour. Nothing else in
    // this crate is unsafe.
    unsafe {
        setiopolicy_np(IOPOL_TYPE_DISK, IOPOL_SCOPE_THREAD, IOPOL_THROTTLE) == 0
            && getiopolicy_np(IOPOL_TYPE_DISK, IOPOL_SCOPE_THREAD) == IOPOL_THROTTLE
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_new_thread_can_be_lowered() {
        let lowered = std::thread::spawn(super::lowest_priority)
            .join()
            .unwrap_or(false);
        assert!(lowered);
    }

    #[test]
    fn a_thread_s_disk_io_can_be_made_low_priority() {
        let set = std::thread::spawn(super::background_io)
            .join()
            .unwrap_or(false);
        assert_eq!(set, cfg!(target_os = "macos"));
        // Asking twice is fine (a download thread may be reused).
        let twice = std::thread::spawn(|| super::background_io() && super::background_io())
            .join()
            .unwrap_or(false);
        assert_eq!(twice, cfg!(target_os = "macos"));
    }

    /// D-131: this crate's only `unsafe` is `background_io_macos`: one
    /// `unsafe extern` block and one `unsafe` block, in this file.
    #[test]
    fn the_only_unsafe_is_background_io() {
        fn rust_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
            for entry in std::fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    rust_files(&path, out);
                } else if path.extension().is_some_and(|e| e == "rs") {
                    out.push(path);
                }
            }
        }
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut files = Vec::new();
        rust_files(&src, &mut files);
        let word = ["un", "safe"].concat();
        let mut found = Vec::new();
        for file in files {
            let text = std::fs::read_to_string(&file).unwrap();
            for line in text.lines() {
                let code = line.split("//").next().unwrap_or("");
                let hits = code
                    .match_indices(&word)
                    .filter(|(i, _)| {
                        let after = &code[i + word.len()..];
                        let before = &code[..*i];
                        !before.ends_with(|c: char| c.is_alphanumeric() || c == '_')
                            && !after.starts_with(|c: char| c.is_alphanumeric() || c == '_')
                            && !before.ends_with('"')
                    })
                    .count();
                for _ in 0..hits {
                    found.push((
                        file.file_name().unwrap().to_string_lossy().into_owned(),
                        code.trim().to_owned(),
                    ));
                }
            }
        }
        let expected = [
            ("thread.rs".to_owned(), format!("{word} extern \"C\" {{")),
            ("thread.rs".to_owned(), format!("{word} {{")),
        ];
        assert_eq!(found, expected, "a new `{word}` needs a recorded decision");
    }
}
