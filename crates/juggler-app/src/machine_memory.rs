//! This Mac's memory beyond the AI tools (the 0.1.4 review: "Under memory
//! pressure" beside "AI tools: about 16 GB of 103 GB" read as plenty
//! left, and the alerts named the Claude app while other apps held most of
//! it). Two things: the biggest other apps by name, and whether the AI
//! tools are a small share of what's in use, so an alert names the real
//! cause.
//!
//! Names only: an app's name (its `.app` bundle's), else the process's.
//! Never a path or an argument, which can hold private folder names. From
//! the process table the collector already reads: nothing new is measured.
//! Read-only: another app gets no control of any kind (D-08).

use std::collections::HashMap;

use juggler_supervisor::live_stats::ProcessMemory;

use crate::machine_health::ThisMacInfo;

/// One other app (not an AI tool) and the memory its processes use.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct OtherAppInfo {
    /// The app's name ("Docker", from `Docker.app`), else the process's
    /// ("next-server"). Never a path.
    pub name: String,
    /// Resident memory summed over its processes, bytes.
    #[ts(type = "number")]
    pub rss_bytes: u64,
    /// How many processes the figure sums. Over one, it's an estimate:
    /// memory they share is counted once per process.
    pub processes: u32,
}

/// This Mac lists at most this many other apps.
pub const OTHER_APPS_SHOWN: usize = 6;
/// An alert names at most this many.
pub const OTHER_APPS_NAMED: usize = 3;
/// A name is cut at this many characters.
const NAME_MAX: usize = 40;

/// A name with no path in it, cut to [`NAME_MAX`]; `None` when empty.
fn plain_name(name: &str) -> Option<String> {
    let name = name.rsplit('/').next().unwrap_or(name).trim();
    (!name.is_empty()).then(|| name.chars().take(NAME_MAX).collect())
}

/// The name a process is listed under: its app's, when it runs from
/// inside one (a helper counts with its app), else its own.
fn listed_name(process: &ProcessMemory) -> Option<String> {
    process
        .exe
        .as_deref()
        .and_then(crate::features::conversations::places::outer_bundle)
        .and_then(|(_, bundle)| plain_name(bundle.trim_end_matches(".app")))
        .or_else(|| plain_name(&process.name))
}

/// Pure: the biggest other apps, heaviest first, at most `limit`; each
/// app's helpers summed under its name.
#[must_use]
pub fn other_apps(processes: &[ProcessMemory], limit: usize) -> Vec<OtherAppInfo> {
    let mut apps: HashMap<String, (u64, u32)> = HashMap::new();
    for process in processes {
        let Some(name) = listed_name(process) else {
            continue;
        };
        let entry = apps.entry(name).or_default();
        entry.0 = entry.0.saturating_add(process.rss_bytes);
        entry.1 = entry.1.saturating_add(1);
    }
    let mut out: Vec<OtherAppInfo> = apps
        .into_iter()
        .map(|(name, (rss_bytes, processes))| OtherAppInfo {
            name,
            rss_bytes,
            processes,
        })
        .collect();
    out.sort_by(|a, b| b.rss_bytes.cmp(&a.rss_bytes).then(a.name.cmp(&b.name)));
    out.truncate(limit);
    out
}

/// Pure: the AI tools hold under a third of the memory in use, so they are
/// not what's filling it. `false` when memory in use wasn't read.
#[must_use]
pub fn tools_a_small_share(mac: &ThisMacInfo) -> bool {
    mac.machine
        .memory_used
        .is_some_and(|used| used > 0 && mac.tools.rss_bytes.saturating_mul(3) < used)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    const G: u64 = 1_000_000_000;

    fn process(name: &str, exe: Option<&str>, rss: u64) -> ProcessMemory {
        ProcessMemory {
            name: name.to_string(),
            exe: exe.map(std::path::PathBuf::from),
            rss_bytes: rss,
        }
    }

    /// The review's Mac: a dev server, Docker's app and its helpers, a
    /// file watcher, a video render.
    fn table() -> Vec<ProcessMemory> {
        vec![
            process(
                "next-server",
                Some("/Users/you/Clients/Acme Secret/node_modules/.bin/node"),
                6_400_000_000,
            ),
            process(
                "com.docker.backend",
                Some("/Applications/Docker.app/Contents/MacOS/com.docker.backend"),
                3 * G,
            ),
            process(
                "Docker Desktop Helper",
                Some(
                    "/Applications/Docker.app/Contents/Frameworks/Docker Desktop Helper.app/Contents/MacOS/Docker Desktop Helper",
                ),
                G,
            ),
            process(
                "fseventsd",
                Some("/System/Library/Frameworks/CoreServices.framework/fseventsd"),
                1_500_000_000,
            ),
            process(
                "ffmpeg",
                Some("/Users/you/Private/render/ffmpeg"),
                900_000_000,
            ),
            process(
                "ffmpeg",
                Some("/Users/you/Private/render/ffmpeg"),
                300_000_000,
            ),
        ]
    }

    #[test]
    fn helpers_count_with_their_app_and_the_heaviest_come_first() {
        let apps = other_apps(&table(), OTHER_APPS_SHOWN);
        let names: Vec<(&str, u64, u32)> = apps
            .iter()
            .map(|app| (app.name.as_str(), app.rss_bytes, app.processes))
            .collect();
        assert_eq!(
            names,
            [
                ("next-server", 6_400_000_000, 1),
                ("Docker", 4 * G, 2),
                ("fseventsd", 1_500_000_000, 1),
                ("ffmpeg", 1_200_000_000, 2),
            ]
        );
        assert_eq!(other_apps(&table(), 2).len(), 2, "at most the limit");
        assert!(other_apps(&[], OTHER_APPS_SHOWN).is_empty());
    }

    /// Names only: a folder's name in a path (a client, a project) never
    /// reaches the list, and neither does a slash.
    #[test]
    fn other_apps_show_names_never_paths() {
        let mut processes = table();
        processes.push(process(
            "/Users/you/Clients/Acme Secret/run.sh",
            None,
            2 * G,
        ));
        processes.push(process("   ", None, 5 * G));
        let apps = other_apps(&processes, usize::MAX);
        let shown = serde_json::to_string(&apps).unwrap();
        for private in ["Acme", "Secret", "Clients", "/Users", "Private", "/"] {
            assert!(!shown.contains(private), "{private:?} in {shown}");
        }
        assert!(apps.iter().any(|app| app.name == "run.sh"));
        assert!(apps.iter().all(|app| !app.name.trim().is_empty()));
        let long = other_apps(&[process(&"x".repeat(200), None, G)], 1);
        assert_eq!(long[0].name.chars().count(), NAME_MAX);
    }
}
