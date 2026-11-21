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

    /// The `unsafe` keywords and the `unsafe_code` lint names in `src`,
    /// counted on Rust's own tokens (`proc-macro2`'s lexer): string
    /// literals, comments and doc comments are never code, and a macro's
    /// body is (the 2026-11-19 review's I1: a cut at `//` was fooled by a
    /// string holding `//`).
    fn unsafe_tokens(src: &str) -> (usize, usize) {
        fn walk(stream: proc_macro2::TokenStream, keyword: &mut usize, lint: &mut usize) {
            for tree in stream {
                match tree {
                    proc_macro2::TokenTree::Ident(ident) => {
                        let word = ident.to_string();
                        if word == ["un", "safe"].concat() {
                            *keyword += 1;
                        } else if word == ["un", "safe_code"].concat() {
                            *lint += 1;
                        }
                    }
                    proc_macro2::TokenTree::Group(group) => walk(group.stream(), keyword, lint),
                    _ => {}
                }
            }
        }
        let stream: proc_macro2::TokenStream = src.parse().expect("Rust source lexes");
        let (mut keyword, mut lint) = (0, 0);
        walk(stream, &mut keyword, &mut lint);
        (keyword, lint)
    }

    /// I1's mutations: each hides an `unsafe` the old line scan missed or
    /// would miss; the token count sees every one, and none of the decoys.
    #[test]
    fn the_unsafe_count_reads_tokens_not_text() {
        let word = ["un", "safe"].concat();
        let cases = [
            (
                format!("fn f() {{ let _u = \"a//b\"; {word} {{ }} }}"),
                (1, 0),
            ),
            (format!("fn f() {{ {word} {{ }} }}"), (1, 0)),
            (format!("{word} impl Send for X {{}}"), (1, 0)),
            (format!("m!({word} {{ x() }});"), (1, 0)),
            // (The attribute is assembled, so this file's own lint scan,
            // allow_reasons, doesn't read it as one.)
            (format!("#[{}({word}_code)] fn f() {{}}", "allow"), (0, 1)),
            (format!("/* {word} {{ }} */ fn f() {{}}"), (0, 0)),
            (format!("/// {word} {{ }}\nfn f() {{}}"), (0, 0)),
            (format!("fn f() {{ let _ = \"{word} {{ }}\"; }}"), (0, 0)),
            (
                format!("fn f() {{ let _ = r#\"{word}\"#; }} // {word}"),
                (0, 0),
            ),
        ];
        for (src, want) in cases {
            assert_eq!(unsafe_tokens(&src), want, "{src}");
        }
    }

    /// D-131: this crate's only `unsafe` is `background_io_macos`: one
    /// `unsafe extern` block and one `unsafe` block, under its one
    /// `allow(unsafe_code)`, in this file.
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
        let found: Vec<(String, (usize, usize))> = files
            .iter()
            .map(|file| {
                let text = std::fs::read_to_string(file).unwrap();
                (
                    file.file_name().unwrap().to_string_lossy().into_owned(),
                    unsafe_tokens(&text),
                )
            })
            .filter(|(_, counts)| *counts != (0, 0))
            .collect();
        assert_eq!(
            found,
            [("thread.rs".to_owned(), (2, 1))],
            "a new `{}` needs a recorded decision",
            ["un", "safe"].concat()
        );
    }
}
