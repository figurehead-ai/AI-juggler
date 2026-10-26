//! The signal guardrail (D-01 zero false kills, AJ-E507 every signal
//! behind a same-tick verify): lists every Tauri command, and every
//! other entry point, that can send a signal, and asserts each goes
//! through a verified path.
//!
//! It reads the source, not a running app: the command registry in
//! `lib.rs`, then a call graph of the app crate's functions (a call
//! resolves by its module path, so `here::clean_here(` is the one in
//! `link/here.rs`). Three kinds of signal exist, and only these:
//!
//! - **verified ladder** — `terminate_group`, the TERM → grace → KILL
//!   ladder over pids. Called ONLY from `commands::kill_execute_as`,
//!   after the same-tick identity check (`verify_kill_target`: pid +
//!   start time + exe + boot, ledger-owned, this Mac) and after the
//!   audit intent is written; and from `features/processes/stop::execute`
//!   (D-41b, the user's own stop of another app's leftover): the same
//!   ladder, this pid only, after the identity check against its
//!   sighting (pid + start time + program + boot, its session's own
//!   process ended, never an app, a bundle's process, a ledger process or
//!   AI Juggler's) at click time and again as the ladder starts, and
//!   after its audit intent (`user:observed`). No rule ever calls it.
//! - **session stop** — `Supervisor::stop`, a session's own process
//!   group. It signals a pid only while it still names our process
//!   (`stop_targets`: an unreaped child, or an adopted root whose start
//!   time verifies).
//! - **own child** — `.kill()` on a `std::process::Child` this app
//!   spawned and has not waited on yet (its pid cannot be reused).
//!
//! Anything else — `libc::kill`, `nix::sys::signal`, a `kill` command,
//! the retired E410 `stop_pid_ladder` — fails the test. So does a new
//! command or call site that can signal: add it here only after
//! routing it through one of the paths above.

// Test crates are pragmatic by policy (QUALITY.md §2).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

const VERIFIED: &str = "verified ladder";
const SESSION: &str = "session stop";
const CHILD: &str = "own child";

/// What each kind of signal looks like in source (whitespace removed).
const SINKS: &[(&str, &str)] = &[
    (VERIFIED, "terminate_group("),
    (SESSION, "supervisor.stop("),
    (CHILD, ".kill()"),
];

/// Raw signal primitives the app crate never uses: every signal goes
/// through the supervisor's verified seams.
const FORBIDDEN_IN_APP: &[&str] = &[
    "libc::kill",
    "nix::sys::signal",
    "signal::kill",
    "killpg",
    "Command::new(\"kill\")",
    "Command::new(\"/bin/kill\")",
    "Command::new(\"pkill\")",
    "signal_pid(",
    "send_signal(",
    "stop_pid_ladder",
    "orphan_stop",
];

/// Every function in the app crate that signals directly, and how.
/// A new one fails the test until it is routed through a verified path.
const SINK_SITES: &[(&str, &str)] = &[
    ("commands::kill_execute_as", VERIFIED),
    // D-41b: another app's leftover, the user's click only (see above).
    ("features/processes/stop::execute", VERIFIED),
    ("commands::stop_session", SESSION),
    ("commands::freeze_source", SESSION),
    ("commands::stop_continuation", SESSION),
    ("commands::stop_away_instance", SESSION),
    ("crossmachine::build_bundle", SESSION),
    ("crossmachine/switch_back::conclude_back", SESSION),
    ("link::execute", SESSION),
    ("quit::app_quit", SESSION),
    ("terminal::terminal_kill_impl", SESSION),
    ("features/app/crash::end_watchdog", CHILD),
    ("features/conversations/scan::open_files", CHILD),
    ("features/quota/probe::run", CHILD),
    // E23 (ADR-0022): a driven session's own process, through the
    // supervisor: the one it just started when its protocol didn't answer
    // (the start falls back to the terminal), one handed over to the
    // tool's terminal, one released for a switch.
    ("features/runtime/launch::try_driven", SESSION),
    ("features/runtime/commands::handover_impl", SESSION),
    ("features/runtime::release_for_switch", SESSION),
];

/// The Tauri commands that can send a signal, and the paths they take.
const SIGNALLING_COMMANDS: &[(&str, &[&str])] = &[
    ("kill_execute", &[VERIFIED]),
    ("processes_observed_stop", &[VERIFIED]),
    ("stop_session", &[SESSION]),
    ("terminal_kill", &[SESSION]),
    ("app_quit", &[SESSION]),
    ("session_handover", &[SESSION]),
    ("switch_execute", &[SESSION]),
    ("switch_undo", &[SESSION]),
    ("switch_back_execute", &[SESSION]),
    ("fleet_switch_execute", &[SESSION]),
    ("fleet_switch_back_execute", &[SESSION]),
    ("app_relaunch", &[CHILD]),
    ("update_restart", &[CHILD]),
    ("session_prefs_set", &[CHILD]),
    ("quota_probe_turn_on", &[CHILD]),
    ("quota_probe_run", &[CHILD]),
    // E23: every start goes through the one spawn path, whose driven try
    // stops the process it just started when its protocol doesn't answer
    // (`try_driven`, through `Supervisor::stop`); a hand-over and a resume
    // stop the driven session's own process the same way.
    ("launch_session", &[SESSION]),
    ("epoch_resume", &[SESSION]),
    ("parked_resume", &[SESSION]),
    ("packs_open_start", &[SESSION]),
    ("runtime_handover", &[SESSION]),
    ("runtime_resume", &[SESSION]),
];

fn crate_src(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(name)
        .join("src")
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

/// The source with `//` comments removed (doc comments name the
/// primitives they forbid) and string literals blanked, so neither
/// counts as code.
fn code_of(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let chars: Vec<char> = src.chars().collect();
    let mut i = 0;
    let mut in_str = false;
    while i < chars.len() {
        let c = chars[i];
        if in_str {
            if c == '\\' {
                out.push(' ');
                i += 1;
                if i < chars.len() {
                    out.push(if chars[i] == '\n' { '\n' } else { ' ' });
                }
            } else if c == '"' {
                in_str = false;
                out.push('"');
            } else {
                out.push(if c == '\n' { '\n' } else { ' ' });
            }
        } else if c == '/' && chars.get(i + 1) == Some(&'/') {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        } else if c == '\'' && chars.get(i + 2) == Some(&'\'') {
            // A char literal like '{' must not count as a brace.
            out.push_str("' '");
            i += 3;
            continue;
        } else if c == '"' {
            in_str = true;
            out.push('"');
        } else {
            out.push(c);
        }
        i += 1;
    }
    out
}

/// The raw text with only `//` comments removed (strings kept): the
/// forbidden-primitive scan must see `Command::new("kill")`.
fn without_comments(src: &str) -> String {
    src.lines()
        .map(|line| {
            let mut in_str = false;
            let bytes = line.as_bytes();
            let mut end = line.len();
            let mut i = 0;
            while i < bytes.len() {
                match bytes[i] {
                    b'\\' if in_str => i += 1,
                    b'"' => in_str = !in_str,
                    b'/' if !in_str && bytes.get(i + 1) == Some(&b'/') => {
                        end = i;
                        break;
                    }
                    _ => {}
                }
                i += 1;
            }
            &line[..end]
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn is_ident(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Every `fn name … { body }` in `code` (nested ones too).
fn functions(code: &str) -> Vec<(String, String)> {
    let bytes = code.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while let Some(at) = code[i..].find("fn ") {
        let start = i + at;
        i = start + 3;
        if start > 0 && is_ident(bytes[start - 1] as char) {
            continue;
        }
        let name: String = code[i..]
            .trim_start()
            .chars()
            .take_while(|c| is_ident(*c))
            .collect();
        if name.is_empty() {
            continue;
        }
        let Some(open_rel) = code[i..].find(['{', ';']) else {
            break;
        };
        let open = i + open_rel;
        if bytes[open] == b';' {
            continue;
        }
        let mut depth = 0_i32;
        let mut end = open;
        for (offset, b) in bytes[open..].iter().enumerate() {
            match b {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = open + offset;
                        break;
                    }
                }
                _ => {}
            }
        }
        out.push((name, code[open..=end].to_string()));
    }
    out
}

/// A file's module path from its crate's `src`: `link/here`,
/// `features/quota/probe`, `commands`.
fn module_of(src_root: &Path, file: &Path) -> String {
    let rel = file
        .strip_prefix(src_root)
        .unwrap()
        .with_extension("")
        .to_string_lossy()
        .replace('\\', "/");
    rel.strip_suffix("/mod").map_or(rel.clone(), str::to_string)
}

/// The free-function calls in `body`: `(path segments, name)` — method
/// calls (`.x(`) are left out (a receiver's type is not knowable here;
/// the sinks themselves are matched as text).
fn calls(body: &str) -> Vec<(Vec<String>, String)> {
    let chars: Vec<char> = body.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if !(chars[i].is_ascii_alphabetic() || chars[i] == '_')
            || (i > 0 && (is_ident(chars[i - 1]) || chars[i - 1] == '.' || chars[i - 1] == ':'))
        {
            i += 1;
            continue;
        }
        let mut segments = Vec::new();
        let mut j = i;
        loop {
            let seg_start = j;
            while j < chars.len() && is_ident(chars[j]) {
                j += 1;
            }
            segments.push(chars[seg_start..j].iter().collect::<String>());
            if chars.get(j) == Some(&':')
                && chars.get(j + 1) == Some(&':')
                && chars
                    .get(j + 2)
                    .is_some_and(|c| c.is_ascii_alphabetic() || *c == '_')
            {
                j += 2;
                continue;
            }
            break;
        }
        // An optional turbofish: `name::<T>(`.
        let mut k = j;
        if chars.get(k) == Some(&':')
            && chars.get(k + 1) == Some(&':')
            && chars.get(k + 2) == Some(&'<')
        {
            let mut depth = 0;
            k += 2;
            while k < chars.len() {
                match chars[k] {
                    '<' => depth += 1,
                    '>' => {
                        depth -= 1;
                        if depth == 0 {
                            k += 1;
                            break;
                        }
                    }
                    _ => {}
                }
                k += 1;
            }
        }
        while chars.get(k).is_some_and(|c| *c == ' ' || *c == '\n') {
            k += 1;
        }
        if chars.get(k) == Some(&'(') {
            let name = segments.pop().unwrap();
            out.push((segments, name));
        }
        i = j.max(i + 1);
    }
    out
}

struct Graph {
    /// `module::name` → body (same-named fns of one module are merged).
    bodies: BTreeMap<String, String>,
    /// fn name → the modules defining it.
    by_name: HashMap<String, Vec<String>>,
}

impl Graph {
    fn of_app() -> Self {
        let root = crate_src("juggler-app");
        let mut files = Vec::new();
        rust_files(&root, &mut files);
        let mut bodies: BTreeMap<String, String> = BTreeMap::new();
        let mut by_name: HashMap<String, Vec<String>> = HashMap::new();
        for file in files {
            let module = module_of(&root, &file);
            let code = code_of(&std::fs::read_to_string(&file).unwrap());
            for (name, body) in functions(&code) {
                let key = format!("{module}::{name}");
                if !bodies.contains_key(&key) {
                    by_name.entry(name).or_default().push(module.clone());
                }
                bodies.entry(key).or_default().push_str(&body);
            }
        }
        Self { bodies, by_name }
    }

    /// The functions a call resolves to: unqualified and `Self::` calls
    /// to the caller's module, `a::b::f(` to a module ending in `a/b`
    /// (or, for `module::Type::f(`, in `module`).
    fn resolve(&self, caller: &str, path: &[String], name: &str) -> Vec<String> {
        let Some(modules) = self.by_name.get(name) else {
            return Vec::new();
        };
        let segments: Vec<&str> = path
            .iter()
            .map(String::as_str)
            .filter(|seg| !matches!(*seg, "crate" | "super" | "self"))
            .collect();
        let ends = |tail: &str| -> Vec<String> {
            modules
                .iter()
                .filter(|module| *module == tail || module.ends_with(&format!("/{tail}")))
                .map(|module| format!("{module}::{name}"))
                .collect()
        };
        if segments.is_empty() || segments == ["Self"] {
            return modules
                .iter()
                .filter(|module| *module == caller)
                .map(|module| format!("{module}::{name}"))
                .collect();
        }
        let tail = segments.join("/");
        let hits = ends(&tail);
        if !hits.is_empty() {
            return hits;
        }
        // A module re-exporting its child's fns (`pub use commands::*`).
        let children = self.within(&tail, name);
        if !children.is_empty() || segments.len() < 2 {
            return children;
        }
        ends(&segments[..segments.len() - 1].join("/"))
    }

    /// The fns named `name` in `module` or in a module under it (a
    /// parent that re-exports its children's items).
    fn within(&self, module: &str, name: &str) -> Vec<String> {
        self.by_name
            .get(name)
            .into_iter()
            .flatten()
            .filter(|candidate| {
                *candidate == module
                    || candidate.starts_with(&format!("{module}/"))
                    || candidate.contains(&format!("/{module}/"))
                    || candidate.ends_with(&format!("/{module}"))
            })
            .map(|candidate| format!("{candidate}::{name}"))
            .collect()
    }

    /// For every function: the kinds of signal it can reach.
    fn reach(&self) -> BTreeMap<String, BTreeSet<&'static str>> {
        let mut reach: BTreeMap<String, BTreeSet<&'static str>> = BTreeMap::new();
        let mut edges: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for (key, body) in &self.bodies {
            let flat: String = body.chars().filter(|c| !c.is_whitespace()).collect();
            reach.insert(
                key.clone(),
                SINKS
                    .iter()
                    .filter(|(_, pattern)| flat.contains(pattern))
                    .map(|(kind, _)| *kind)
                    .collect(),
            );
            let caller = key.rsplit_once("::").unwrap().0;
            let targets = calls(body)
                .into_iter()
                .flat_map(|(path, name)| self.resolve(caller, &path, &name))
                .filter(|target| target != key)
                .collect();
            edges.insert(key.clone(), targets);
        }
        loop {
            let mut changed = false;
            for (key, targets) in &edges {
                let gained: BTreeSet<&'static str> = targets
                    .iter()
                    .flat_map(|target| reach[target].iter().copied())
                    .filter(|kind| !reach[key].contains(kind))
                    .collect();
                if !gained.is_empty() {
                    reach.get_mut(key).unwrap().extend(gained);
                    changed = true;
                }
            }
            if !changed {
                return reach;
            }
        }
    }
}

/// `(module path, command name)` for every registered Tauri command.
fn registered_commands() -> Vec<(String, String)> {
    let lib = std::fs::read_to_string(crate_src("juggler-app").join("lib.rs")).unwrap();
    lib.lines()
        .filter_map(|line| {
            let rest = line
                .trim()
                .strip_prefix("Read ")
                .or_else(|| line.trim().strip_prefix("Write "))?;
            let path = rest.trim().strip_suffix(',')?;
            let (module, name) = path.rsplit_once("::")?;
            Some((module.replace("::", "/"), name.to_string()))
        })
        .collect()
}

#[test]
fn the_app_never_uses_a_raw_signal() {
    let root = crate_src("juggler-app");
    let mut files = Vec::new();
    rust_files(&root, &mut files);
    let mut found = Vec::new();
    for file in &files {
        let text = without_comments(&std::fs::read_to_string(file).unwrap());
        for pattern in FORBIDDEN_IN_APP {
            if text.contains(pattern) {
                found.push(format!("{}: {pattern}", file.display()));
            }
        }
    }
    assert!(
        found.is_empty(),
        "raw signals outside the verified paths: {found:#?}"
    );
}

#[test]
fn every_signal_site_is_one_of_the_verified_paths() {
    let graph = Graph::of_app();
    let mut direct: BTreeMap<String, BTreeSet<&str>> = BTreeMap::new();
    for (key, body) in &graph.bodies {
        let flat: String = body.chars().filter(|c| !c.is_whitespace()).collect();
        for (kind, pattern) in SINKS {
            if flat.contains(pattern) {
                direct.entry(key.clone()).or_default().insert(kind);
            }
        }
    }
    let expected: BTreeMap<String, BTreeSet<&str>> = SINK_SITES
        .iter()
        .map(|(site, kind)| ((*site).to_string(), BTreeSet::from([*kind])))
        .collect();
    println!("[signal sites]");
    for (site, kinds) in &direct {
        println!("  {site}: {kinds:?}");
    }
    assert_eq!(
        direct, expected,
        "a function signals outside the reviewed list: route it through \
         commands::kill_execute_as (pids) or Supervisor::stop (a session)"
    );
}

#[test]
fn the_pid_ladder_runs_only_after_verify_and_audit() {
    let graph = Graph::of_app();
    let body = &graph.bodies["commands::kill_execute_as"];
    let at = |needle: &str| {
        body.find(needle)
            .unwrap_or_else(|| panic!("kill_execute_as lost `{needle}`"))
    };
    let machine = at("local_machine_id(");
    let verify = at("verify_kill_target(");
    let refusal = at("if !verified");
    let intent = at(".intent(");
    let ladder = at("terminate_group(");
    assert!(
        machine < verify && verify < refusal && refusal < intent && intent < ladder,
        "the order is: this Mac, identity, refusal, audit intent, then the ladder"
    );
    // The identity rule itself: pid + start time + exe + boot, from
    // the ledger row, this Mac only — never a bare pid.
    let rule = &graph.bodies["commands::verify_kill_target"];
    for needle in [
        "local_machine_id(",
        "verify_settling(",
        "verify_start_time(",
        "boot_id",
    ] {
        assert!(rule.contains(needle), "verify_kill_target lost `{needle}`");
    }
    // The tiers, the sheet and a paired Mac's clean all come here.
    let callers: BTreeSet<&String> = graph
        .bodies
        .iter()
        .filter(|(key, body)| {
            *key != "commands::kill_execute_as" && body.contains("kill_execute_as(")
        })
        .map(|(key, _)| key)
        .collect();
    let callers: Vec<&str> = callers.iter().map(|key| key.as_str()).collect();
    assert_eq!(
        callers,
        [
            "commands::auto_cleanup_sweep",
            "commands::kill_execute_audited",
            "link/here::clean_here",
        ],
        "every pid kill enters through the sheet, the opted-in tiers or a paired Mac's clean"
    );
}

#[test]
fn the_session_stop_checks_identity_before_any_signal() {
    let src = code_of(
        &std::fs::read_to_string(crate_src("juggler-supervisor").join("supervisor.rs")).unwrap(),
    );
    let fns: BTreeMap<String, String> = functions(&src).into_iter().collect();
    let stop = &fns["stop"];
    let targets = stop
        .find("stop_targets(")
        .expect("stop lost its identity guard");
    let first_signal = stop.find("send_terminate(").expect("stop sends TERM");
    assert!(
        targets < first_signal,
        "identity is read before the first TERM"
    );
    let guard_src = code_of(
        &std::fs::read_to_string(crate_src("juggler-supervisor").join("stop_guard.rs")).unwrap(),
    );
    let guard_fns: BTreeMap<String, String> = functions(&guard_src).into_iter().collect();
    let guard = &guard_fns["stop_targets"];
    assert!(
        guard.contains("verify_start_time("),
        "an adopted root's start time is verified"
    );
    assert!(
        guard.contains("Signal::Probe"),
        "a reaped child's number is checked before its group is signalled"
    );
}

#[test]
fn only_the_supervisor_seams_hold_raw_signals() {
    let root = crate_src("juggler-supervisor");
    let mut files = Vec::new();
    rust_files(&root, &mut files);
    let mut holders = BTreeSet::new();
    for file in &files {
        let text = without_comments(&std::fs::read_to_string(file).unwrap());
        if text.contains("nix::sys::signal") || text.contains("libc::kill") {
            holders.insert(module_of(&root, file));
        }
        assert!(
            !text.contains("stop_pid_ladder"),
            "the retired E410 ladder is back in {}",
            file.display()
        );
    }
    assert_eq!(
        holders,
        BTreeSet::from(["supervisor".to_string(), "tree".to_string()]),
        "raw signals live only in tree::signal_pid and the session stop"
    );
}

#[test]
fn every_signalling_command_is_listed_and_verified() {
    let graph = Graph::of_app();
    let reach = graph.reach();
    let mut signalling: BTreeMap<String, BTreeSet<&str>> = BTreeMap::new();
    for (module, name) in registered_commands() {
        // An inline `mod` lives in its parent's file.
        let mut scope = module.as_str();
        let mut defs = graph.within(scope, &name);
        while defs.is_empty() {
            let Some((parent, _)) = scope.rsplit_once('/') else {
                break;
            };
            scope = parent;
            defs = graph.within(scope, &name);
        }
        assert!(
            !defs.is_empty(),
            "registered command {module}::{name} not found in the source"
        );
        let kinds: BTreeSet<&str> = defs
            .iter()
            .flat_map(|key| reach[key].iter().copied())
            .collect();
        if !kinds.is_empty() {
            signalling.insert(name, kinds);
        }
    }
    println!("[commands that can send a signal]");
    for (command, kinds) in &signalling {
        println!("  {command}: {kinds:?}");
    }
    let expected: BTreeMap<String, BTreeSet<&str>> = SIGNALLING_COMMANDS
        .iter()
        .map(|(command, kinds)| ((*command).to_string(), kinds.iter().copied().collect()))
        .collect();
    assert_eq!(
        signalling, expected,
        "the commands that can signal changed: each must reach a signal only \
         through the verified ladder, the session stop or its own child"
    );
    // No registered command is the retired E410 stop.
    assert!(
        !registered_commands()
            .iter()
            .any(|(_, name)| name.starts_with("orphan_")),
        "the E410 orphan_* commands are retired"
    );
}
