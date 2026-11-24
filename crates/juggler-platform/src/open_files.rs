//! Which programs have a file open to write it (AJ-E2324 R2, the move
//! review's B3): a move's guard against writers no tool's adapter names
//! (a sync tool, an editor, another app), asked right before its backup
//! and again right before it moves anything.
//!
//! Only this user's processes are asked about, and only which of their
//! open files are at or under the given paths and whether each is open
//! for writing: never their arguments or environment (D-37). Readers
//! (an editor showing a file, Spotlight, `tail -f`) are left out: a clone
//! of a file being read is whole.
//!
//! - **macOS**: one `lsof` listing of this user's open files (about a
//!   third of a second on a Mac with 900 processes), filtered here; no
//!   `unsafe`.
//! - **Linux**: `/proc/<pid>/fd` and its `fdinfo` flags.
//! - **Windows**: not available yet ([`crate::unsupported`]); the move's
//!   other checks still hold.

use std::path::{Path, PathBuf};

/// A program with a file open to write it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Writer {
    /// Its process.
    pub pid: u32,
    /// Its short name, as the system lists it.
    pub name: String,
    /// The file.
    pub path: PathBuf,
}

/// The programs of this user (this one aside) that have a file at or
/// under any of `paths` open for writing.
///
/// # Errors
///
/// The system can't be asked (or this system has no implementation yet).
pub fn writers(paths: &[PathBuf]) -> std::io::Result<Vec<Writer>> {
    // The real spelling of each (the listing names real paths:
    // `/private/tmp`, never `/tmp`).
    let roots: Vec<PathBuf> = paths
        .iter()
        .filter_map(|p| std::fs::canonicalize(p).ok())
        .collect();
    if roots.is_empty() {
        return Ok(Vec::new());
    }
    let me = std::process::id();
    Ok(imp::open_for_writing()?
        .into_iter()
        .filter(|w| w.pid != me && roots.iter().any(|root| w.path.starts_with(root)))
        .collect())
}

/// Reads `lsof -F pcan` output: each process's `p`id and `c`ommand, then
/// each of its files' `f`d, `a`ccess (`r`, `w`, `u`) and `n`ame. Keeps
/// the files open for writing (`w` or `u`).
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn parse_lsof(out: &str) -> Vec<Writer> {
    let mut found = Vec::new();
    let (mut pid, mut name, mut writes) = (0_u32, String::new(), false);
    for line in out.lines() {
        let Some(tag) = line.chars().next() else {
            continue;
        };
        let value = &line[tag.len_utf8()..];
        match tag {
            'p' => {
                pid = value.parse().unwrap_or(0);
                name.clear();
            }
            'c' => value.clone_into(&mut name),
            'f' => writes = false,
            'a' => writes = value.contains('w') || value.contains('u'),
            'n' if writes && pid != 0 => found.push(Writer {
                pid,
                name: name.clone(),
                path: Path::new(value).to_path_buf(),
            }),
            _ => {}
        }
    }
    found
}

#[cfg(target_os = "macos")]
mod imp {
    use super::{Writer, parse_lsof};

    pub(super) fn open_for_writing() -> std::io::Result<Vec<Writer>> {
        let uid = nix::unistd::getuid().as_raw().to_string();
        // -n -P: no name lookups; -w: no warnings; -a -u: this user only;
        // -d 0-999999: numbered files only (not the program's own code).
        let out = crate::process::helper("/usr/sbin/lsof")
            .args([
                "-n", "-P", "-w", "-a", "-u", &uid, "-d", "0-999999", "-F", "pcan",
            ])
            .stdin(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .output()?;
        // lsof says 1 when it found nothing to list.
        if !out.status.success() && !out.stdout.is_empty() {
            return Err(std::io::Error::other("lsof failed"));
        }
        Ok(parse_lsof(&String::from_utf8_lossy(&out.stdout)))
    }
}

#[cfg(target_os = "linux")]
mod imp {
    use std::os::unix::fs::MetadataExt as _;
    use std::path::Path;

    use super::Writer;

    pub(super) fn open_for_writing() -> std::io::Result<Vec<Writer>> {
        let me = nix::unistd::getuid().as_raw();
        let mut found = Vec::new();
        for proc_dir in std::fs::read_dir("/proc")?.flatten() {
            let Ok(pid) = proc_dir.file_name().to_string_lossy().parse::<u32>() else {
                continue;
            };
            if proc_dir.metadata().map(|m| m.uid()).ok() != Some(me) {
                continue;
            }
            let base = proc_dir.path();
            let name = std::fs::read_to_string(base.join("comm"))
                .map(|s| s.trim().to_owned())
                .unwrap_or_default();
            let Ok(fds) = std::fs::read_dir(base.join("fd")) else {
                continue;
            };
            for fd in fds.flatten() {
                let Ok(target) = std::fs::read_link(fd.path()) else {
                    continue;
                };
                if !target.is_absolute() {
                    continue;
                }
                let info = std::fs::read_to_string(base.join("fdinfo").join(fd.file_name()))
                    .unwrap_or_default();
                // O_WRONLY (1) or O_RDWR (2) in the low bits of the octal flags.
                let writes = info
                    .lines()
                    .find_map(|l| l.strip_prefix("flags:"))
                    .and_then(|f| u32::from_str_radix(f.trim(), 8).ok())
                    .is_some_and(|flags| flags & 3 != 0);
                if writes {
                    found.push(Writer {
                        pid,
                        name: name.clone(),
                        path: Path::new(&target).to_path_buf(),
                    });
                }
            }
        }
        Ok(found)
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
mod imp {
    use super::Writer;

    pub(super) fn open_for_writing() -> std::io::Result<Vec<Writer>> {
        Err(crate::unsupported("the open-file check"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_listing_keeps_writers_and_leaves_readers_out() {
        let out = "p42\ncvim\nf3\nar\nn/x/a.jsonl\nf4\naw\nn/x/b.jsonl\np43\ncsync\nf9\nau\nn/x/c.jsonl\n";
        let found = parse_lsof(out);
        assert_eq!(
            found,
            vec![
                Writer {
                    pid: 42,
                    name: "vim".into(),
                    path: "/x/b.jsonl".into()
                },
                Writer {
                    pid: 43,
                    name: "sync".into(),
                    path: "/x/c.jsonl".into()
                },
            ]
        );
    }

    /// A real program holding the file open to append is found, by name;
    /// one only reading it isn't; nothing outside the paths is.
    #[test]
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    fn a_program_writing_is_found_and_a_reader_is_not() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("s.jsonl");
        std::fs::write(&file, b"{}\n").unwrap();
        let other = tempfile::tempdir().unwrap();
        let shell = |redirect: &str| {
            std::process::Command::new("/bin/sh")
                .arg("-c")
                .arg(format!("exec /bin/sleep 20 {redirect}"))
                .spawn()
                .unwrap()
        };
        let mut writer = shell(&format!("3>>'{}'", file.display()));
        let mut reader = shell(&format!("3<'{}'", file.display()));
        // Let both open it.
        let until = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let mut found = Vec::new();
        while std::time::Instant::now() < until {
            found = writers(&[tmp.path().to_path_buf()]).unwrap();
            if !found.is_empty() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        let nobody = writers(&[other.path().to_path_buf()]).unwrap();
        for child in [&mut writer, &mut reader] {
            child.kill().unwrap();
            child.wait().unwrap();
        }
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].pid, writer.id());
        assert_eq!(found[0].name, "sleep");
        assert!(nobody.is_empty(), "{nobody:?}");
    }
}
