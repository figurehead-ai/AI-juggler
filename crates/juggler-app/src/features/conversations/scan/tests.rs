//! The live refresher over fixture files and a scripted process table.

use std::sync::{Arc, Mutex};

use super::*;

const CLAUDE_APP: &str = "/Applications/Claude.app/Contents/MacOS/Claude";
const SERVER: &str = "/Applications/ChatGPT.app/Contents/Resources/codex-cli/codex";
const THREAD: &str = "019a1b2c-3d4e-7f80-9a1b-2c3d4e5f6a7b";
/// `Sat Oct 24 08:12:34 2026` UTC.
const STARTED: i64 = 1_792_829_554;

#[derive(Default)]
struct FakeOs {
    table: Mutex<Table>,
    open: Mutex<HashMap<u32, Vec<PathBuf>>>,
    reads: Mutex<usize>,
}

impl FakeOs {
    fn put(&self, pid: u32, ppid: u32, exe: &str, args: &[&str], start: i64) {
        self.table.lock().unwrap().insert(
            pid,
            Proc {
                pid,
                ppid: Some(ppid),
                exe: Some(PathBuf::from(exe)),
                args: args.iter().map(|a| (*a).to_owned()).collect(),
                start: Some(start),
            },
        );
    }
}

impl Os for FakeOs {
    fn table(&self) -> Table {
        *self.reads.lock().unwrap() += 1;
        self.table.lock().unwrap().clone()
    }

    fn some(&self, pids: &[u32]) -> Table {
        let table = self.table.lock().unwrap();
        pids.iter()
            .filter_map(|pid| table.get(pid).map(|p| (*pid, p.clone())))
            .collect()
    }

    fn open_files(&self, pid: u32) -> Option<Vec<PathBuf>> {
        self.open.lock().unwrap().get(&pid).cloned()
    }
}

fn adapters() -> Vec<(String, SharedAdapter)> {
    vec![
        (
            juggler_adapters::CLAUDE_CODE_ID.to_owned(),
            Arc::new(juggler_adapters::ClaudeCodeAdapter::new()) as SharedAdapter,
        ),
        (
            juggler_adapters::CODEX_ID.to_owned(),
            Arc::new(juggler_adapters::CodexAdapter::new()) as SharedAdapter,
        ),
    ]
}

fn write(path: &Path, body: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, body).unwrap();
}

fn pid_file(profile: &Path, pid: u32, session: &str, status: &str, host: &str) {
    write(
        &profile.join(format!("sessions/{pid}.json")),
        &format!(
            r#"{{"pid":{pid},"sessionId":"{session}","cwd":"/w/p","status":"{status}","procStart":"Sat Oct 24 08:12:34 2026","hostSessionId":"{host}","entrypoint":"claude-desktop","kind":"interactive"}}"#
        ),
    );
}

fn app_file(instance: &Path, host: &str, cli: &str, category: &str, archived: bool) {
    write(
        &instance.join(format!("claude-code-sessions/a/o/{host}.json")),
        &format!(
            r#"{{"cliSessionId":"{cli}","title":"T {cli}","isArchived":{archived},"lastFocusedAt":1792829000000,"postTurnSummary":{{"status_category":"{category}","needs_action":"words","status_detail":"words"}}}}"#
        ),
    );
}

struct Lab {
    home: tempfile::TempDir,
    profile: PathBuf,
    instance: PathBuf,
    codex: PathBuf,
}

impl Lab {
    fn new() -> Self {
        let home = tempfile::tempdir().unwrap();
        let profile = home.path().join("profiles/work");
        let instance = home
            .path()
            .join("Library/Application Support/ClaudeProfiles/b-work");
        let codex = home.path().join(".codex");
        std::fs::create_dir_all(&profile).unwrap();
        std::fs::create_dir_all(&instance).unwrap();
        Self {
            home,
            profile,
            instance,
            codex,
        }
    }

    fn profiles(&self) -> Vec<Profile> {
        vec![
            Profile {
                tool: juggler_adapters::CLAUDE_CODE_ID.to_owned(),
                dir: self.profile.clone(),
                account_id: Some("acct-work".to_owned()),
            },
            Profile {
                tool: juggler_adapters::CODEX_ID.to_owned(),
                dir: self.codex.clone(),
                account_id: Some("acct-codex".to_owned()),
            },
        ]
    }

    /// The Claude app's `b-work` instance, its helper and a `claude` in it.
    fn desktop(&self, os: &FakeOs, pid: u32, start: i64) {
        let flag = format!("--user-data-dir={}", self.instance.display());
        os.put(1, 0, "/sbin/launchd", &[], 0);
        os.put(10, 1, CLAUDE_APP, &[CLAUDE_APP, &flag], STARTED - 100);
        os.put(
            11,
            10,
            "/Applications/Claude.app/Contents/Helpers/disclaimer",
            &[],
            STARTED - 50,
        );
        let exe = format!(
            "{}/claude-code/2.1/x/claude.app/Contents/MacOS/claude",
            self.instance.display()
        );
        os.put(pid, 11, &exe, &["claude", "--resume=s-1"], start);
    }

    fn tick(&self, scanner: &mut Scanner, os: &FakeOs, at: Instant) -> LiveModel {
        scanner.tick(os, &adapters(), &self.profiles(), self.home.path(), at)
    }
}

/// A lab with one desktop session open: `s-1`, pid 12, idle.
fn one_session() -> (Lab, FakeOs) {
    let lab = Lab::new();
    let os = FakeOs::default();
    lab.desktop(&os, 12, STARTED);
    pid_file(&lab.profile, 12, "s-1", "idle", "local_a");
    (lab, os)
}

#[test]
fn a_desktop_session_is_open_in_its_claude_instance_with_the_apps_word() {
    let (lab, os) = one_session();
    app_file(&lab.instance, "local_a", "s-1", "blocked", false);
    app_file(&lab.instance, "local_b", "s-2", "completed", false);
    app_file(&lab.instance, "local_c", "s-3", "completed", true);
    let model = lab.tick(&mut Scanner::default(), &os, Instant::now());
    assert_eq!(model.entries.len(), 1);
    let entry = &model.entries[0];
    assert_eq!(entry.account_id.as_deref(), Some("acct-work"));
    assert_eq!(entry.session_id, "s-1");
    assert_eq!(entry.state, Some(State::AsksYou));
    assert_eq!(entry.viewed_ms, Some(1_792_829_000_000));
    assert_eq!(entry.app_title.as_deref(), Some("T s-1"));
    let place = &entry.places[0];
    assert_eq!(place.pid, 12);
    // 0.4.1: the app's own id for it, what its session link takes.
    assert_eq!(place.app_session.as_deref(), Some("local_a"));
    let host = place.host.as_ref().unwrap();
    assert_eq!((host.name.as_str(), host.pid), ("Claude", 10));
    assert_eq!(host.instance.as_deref(), Some("b-work"));
    assert_eq!(model.pid_dirs.get(&12), Some(&lab.profile));
    // In the sidebar, not archived, no process: inactive.
    assert_eq!(
        model.inactive,
        vec![(
            juggler_adapters::CLAUDE_CODE_ID.to_owned(),
            "s-2".to_owned()
        )]
    );
}

#[test]
fn a_reused_pid_or_a_stale_file_is_not_open() {
    let lab = Lab::new();
    let os = FakeOs::default();
    // The pid file's process ended; the pid now runs something else.
    lab.desktop(&os, 12, STARTED);
    os.put(12, 11, "/bin/zsh", &["zsh"], STARTED + 3600);
    pid_file(&lab.profile, 12, "s-1", "busy", "local_a");
    let model = lab.tick(&mut Scanner::default(), &os, Instant::now());
    assert!(model.entries.is_empty());
    assert!(model.pid_dirs.is_empty());
    // A pid nobody runs: nothing.
    pid_file(&lab.profile, 99, "s-9", "busy", "local_z");
    assert!(
        lab.tick(&mut Scanner::default(), &os, Instant::now())
            .entries
            .is_empty()
    );
}

#[test]
fn a_running_turn_is_working_whatever_the_last_word_and_open_twice_is_one_row() {
    let lab = Lab::new();
    let os = FakeOs::default();
    lab.desktop(&os, 12, STARTED);
    os.put(
        30,
        1,
        "/System/Applications/Utilities/Terminal.app/Contents/MacOS/Terminal",
        &[],
        5,
    );
    os.put(31, 30, "/bin/zsh", &[], 6);
    os.put(
        32,
        31,
        "/Users/you/.local/bin/claude",
        &["claude", "--resume", "s-1"],
        STARTED,
    );
    pid_file(&lab.profile, 12, "s-1", "idle", "local_a");
    pid_file(&lab.profile, 32, "s-1", "busy", "");
    app_file(&lab.instance, "local_a", "s-1", "review_ready", false);
    let model = lab.tick(&mut Scanner::default(), &os, Instant::now());
    assert_eq!(model.entries.len(), 1, "one conversation, one row");
    let entry = &model.entries[0];
    assert_eq!(entry.state, Some(State::Working));
    let apps: Vec<&str> = entry
        .places
        .iter()
        .map(|place| place.host.as_ref().unwrap().name.as_str())
        .collect();
    assert_eq!(apps, ["Claude", "Terminal"]);
}

#[test]
fn the_codex_apps_loaded_threads_are_the_files_its_server_holds() {
    let lab = Lab::new();
    let os = FakeOs::default();
    os.put(1, 0, "/sbin/launchd", &[], 0);
    os.put(
        40,
        1,
        "/Applications/ChatGPT.app/Contents/MacOS/ChatGPT",
        &[],
        7,
    );
    os.put(41, 40, SERVER, &["codex", "app-server"], 8);
    write(
        &lab.codex.join(format!("thread-writer-locks/{THREAD}.lock")),
        "",
    );
    // A lock nobody holds is left over, not open.
    write(
        &lab.codex
            .join("thread-writer-locks/019a1b2c-3d4e-7f80-9a1b-2c3d4e5f6a7c.lock"),
        "",
    );
    os.open.lock().unwrap().insert(
        41,
        vec![
            lab.codex.join(format!("thread-writer-locks/{THREAD}.lock")),
            lab.codex.join("state_5.sqlite"),
        ],
    );
    let model = lab.tick(&mut Scanner::default(), &os, Instant::now());
    assert_eq!(model.entries.len(), 1);
    let entry = &model.entries[0];
    assert_eq!(entry.session_id, THREAD);
    assert_eq!(
        entry.places[0].profile.as_deref(),
        Some(lab.codex.as_path())
    );
    let host = entry.places[0].host.as_ref().unwrap();
    assert_eq!((host.app_id.as_deref(), host.pid), (Some("codex-app"), 40));
}

#[test]
fn a_codex_resume_with_nothing_held_names_its_thread() {
    let lab = Lab::new();
    let os = FakeOs::default();
    os.put(
        30,
        1,
        "/System/Applications/Utilities/Terminal.app/Contents/MacOS/Terminal",
        &[],
        5,
    );
    os.put(31, 30, "/bin/zsh", &[], 6);
    os.put(
        32,
        31,
        "/opt/homebrew/bin/codex",
        &["codex", "resume", THREAD],
        9,
    );
    let model = lab.tick(&mut Scanner::default(), &os, Instant::now());
    assert_eq!(model.entries.len(), 1);
    assert_eq!(
        model.entries[0].account_id, None,
        "the account comes from the index"
    );
    assert_eq!(model.entries[0].session_id, THREAD);
}

#[test]
fn the_whole_table_is_read_once_a_minute_and_files_only_when_changed() {
    let (lab, os) = one_session();
    let mut scanner = Scanner::default();
    let start = Instant::now();
    lab.tick(&mut scanner, &os, start);
    lab.tick(&mut scanner, &os, start + TICK);
    lab.tick(&mut scanner, &os, start + ALIVE_EVERY);
    assert_eq!(*os.reads.lock().unwrap(), 1);
    // A new session within a minute is found by its own pid and chain.
    os.put(13, 11, "/x/claude", &["claude"], STARTED);
    pid_file(&lab.profile, 13, "s-2", "busy", "local_b");
    let model = lab.tick(&mut scanner, &os, start + TICK * 4);
    assert_eq!(model.entries.len(), 2);
    assert_eq!(*os.reads.lock().unwrap(), 1);
    // The session ends: its pid goes; within the liveness check it's gone.
    os.table.lock().unwrap().remove(&13);
    let model = lab.tick(&mut scanner, &os, start + TICK * 4 + ALIVE_EVERY);
    assert_eq!(model.entries.len(), 1);
    lab.tick(&mut scanner, &os, start + FULL_EVERY);
    assert_eq!(*os.reads.lock().unwrap(), 2);
}

#[test]
fn the_states_map_to_e21s_words() {
    use juggler_adapter::live::{LiveStatus as S, TurnOutcome as O};
    assert_eq!(state_of(S::Busy, Some(O::Blocked)), Some(State::Working));
    assert_eq!(state_of(S::Idle, Some(O::Blocked)), Some(State::AsksYou));
    assert_eq!(
        state_of(S::Idle, Some(O::ReviewReady)),
        Some(State::DoneReview)
    );
    assert_eq!(state_of(S::Idle, Some(O::Completed)), Some(State::Idle));
    assert_eq!(state_of(S::Idle, None), Some(State::Idle));
    assert_eq!(state_of(S::Unknown, None), None);
    // AJ-E2107: the tool's own "waiting" asks you, whatever the app says.
    assert_eq!(state_of(S::Waiting, None), Some(State::AsksYou));
    assert_eq!(
        state_of(S::Waiting, Some(O::Completed)),
        Some(State::AsksYou)
    );
    assert_eq!(State::DoneReview.word(), "done_review");
}

/// The read-only count on a real Mac (AJ-E2101's evidence): counts only,
/// never an id, a path, a title or any other value.
/// `cargo test -p juggler-app --lib scan::tests::count_on_this_mac -- --ignored --nocapture`
#[test]
#[ignore = "reads this Mac's real session files and process table; run by hand"]
fn count_on_this_mac() {
    let home = juggler_platform::user::home().unwrap();
    let mut profiles: Vec<Profile> = Vec::new();
    let claude = juggler_adapters::CLAUDE_CODE_ID.to_owned();
    let mut dirs = vec![home.join(".claude")];
    if let Ok(entries) =
        std::fs::read_dir(home.join("Library/Application Support/ClaudeCodeProfiles"))
    {
        dirs.extend(entries.flatten().map(|entry| entry.path()));
    }
    for dir in dirs {
        profiles.push(Profile {
            tool: claude.clone(),
            dir,
            account_id: None,
        });
    }
    profiles.push(Profile {
        tool: juggler_adapters::CODEX_ID.to_owned(),
        dir: home.join(".codex"),
        account_id: None,
    });
    let adapters = adapters();
    let files: usize = profiles
        .iter()
        .filter_map(|p| {
            adapters
                .iter()
                .find(|(id, _)| *id == p.tool)
                .map(|(_, a)| a.live_session_files(&p.dir).len())
        })
        .sum();
    let mut scanner = Scanner::default();
    let started = Instant::now();
    let model = scanner.tick(&RealOs, &adapters, &profiles, &home, started);
    let first = started.elapsed();
    let mut costs = Vec::new();
    for step in 1..=5 {
        let at = Instant::now();
        let _ = scanner.tick(&RealOs, &adapters, &profiles, &home, started + TICK * step);
        costs.push(at.elapsed());
    }
    costs.sort();
    let places: usize = model.entries.iter().map(|entry| entry.places.len()).sum();
    let mut apps: BTreeMap<String, usize> = BTreeMap::new();
    let mut states: BTreeMap<&str, usize> = BTreeMap::new();
    let mut instances = HashSet::new();
    for entry in &model.entries {
        *states
            .entry(entry.state.map_or("unknown", State::word))
            .or_default() += 1;
        for place in &entry.places {
            let host = place.host.as_ref();
            *apps
                .entry(host.map_or_else(|| "(no app)".to_owned(), |h| h.name.clone()))
                .or_default() += 1;
            if let Some(dir) = host.and_then(|h| h.instance_dir.clone()) {
                instances.insert(dir);
            }
        }
    }
    let viewed = model
        .entries
        .iter()
        .filter(|e| e.viewed_ms.is_some())
        .count();
    let titled = model
        .entries
        .iter()
        .filter(|e| e.app_title.is_some())
        .count();
    println!("session files listed: {files}");
    println!(
        "open sessions (verified pid + start, or held): {}",
        model.entries.len()
    );
    println!(
        "places: {places}; by app: {apps:?}; app instances: {}",
        instances.len()
    );
    println!("states: {states:?}; with the app's record: {titled}; with viewed time: {viewed}");
    println!(
        "inactive in a running app's sidebar: {}",
        model.inactive.len()
    );
    println!(
        "first refresh: {first:?}; next refreshes median {:?}, max {:?}",
        costs[2], costs[4]
    );
}

/// 0.3.4: an app session started from another, by the app's own ids.
fn app(host: &str, session: &str, from: Option<&str>, archived: bool, at: i64) -> AppEntry {
    (
        "claude-code".to_owned(),
        PathBuf::from("/i/b-work"),
        AppSession {
            host_session_id: host.to_owned(),
            session_id: Some(session.to_owned()),
            cwd: None,
            title: Some(format!("Title {session}")),
            archived,
            last_activity_ms: Some(at),
            last_focused_ms: None,
            outcome: None,
            spawned_from: from.map(str::to_owned),
        },
    )
}

#[test]
fn the_sessions_a_session_started_come_from_its_apps_record_newest_first_archived_left_out() {
    let links = started_links(&[
        app("local_p", "parent", None, false, 1),
        app("local_a", "child-a", Some("local_p"), false, 10),
        app("local_b", "child-b", Some("local_p"), false, 30),
        app("local_c", "child-c", Some("local_p"), true, 40),
        app("local_d", "child-d", Some("local_unknown"), false, 50),
    ]);
    let seen: Vec<(&str, &str)> = links
        .iter()
        .map(|link| (link.parent.as_str(), link.session_id.as_str()))
        .collect();
    assert_eq!(seen, [("parent", "child-b"), ("parent", "child-a")]);
    assert_eq!(links[0].title.as_deref(), Some("Title child-b"));
}

/// AJ-E2101 R5: with file events on, the session folders are listed when
/// one changes, not every tick: a quiet tick reuses the last listing.
/// Events are hints (the rc21 preflight, 2026-11-09: on a Mac whose event
/// service was saturated they came more than 60 s late): a new session no
/// event announces is seen at the fallback, within [`LIST_FALLBACK`]; one
/// an event announces, or a rescan, at the next tick. The events come by
/// hand, so nothing here waits on the system's.
#[test]
fn session_folders_are_watched_with_file_events_not_listed_every_tick() {
    let (lab, os) = one_session();
    let mut scanner = Scanner::default();
    let (hand, _) = scanner.use_file_events_by_hand();
    let start = Instant::now();
    let profiles = lab.profiles();
    // A new CLI session: its process and its pid file.
    let open = |pid: u32, session: &str, host: &str| {
        os.put(pid, 11, "/x/claude", &["claude"], STARTED);
        pid_file(&lab.profile, pid, session, "busy", host);
    };
    assert!(scanner.lists_now(&profiles, start), "the first tick lists");
    let model = lab.tick(&mut scanner, &os, start);
    assert_eq!(model.entries.len(), 1);
    let folders = scanner.events.as_ref().unwrap().wake.folders().clone();
    assert_eq!(folders.len(), 1, "the session folder is watched");
    lab.tick(&mut scanner, &os, start + TICK);
    assert!(
        !scanner.lists_now(&profiles, start + TICK * 2),
        "a quiet tick lists nothing"
    );
    // A new session whose event is late or lost: the ticks before the
    // fallback reuse the listing; the fallback sees it.
    open(13, "s-2", "local_b");
    let model = lab.tick(&mut scanner, &os, start + TICK * 2);
    assert_eq!(model.entries.len(), 1, "no event, no fallback yet");
    let model = lab.tick(&mut scanner, &os, start + LIST_FALLBACK);
    assert_eq!(
        model.entries.len(),
        2,
        "the fallback sees it without an event"
    );
    // One an event announces shows at the next tick.
    let at = start + LIST_FALLBACK + TICK;
    open(14, "s-3", "local_c");
    assert!(!scanner.lists_now(&profiles, at));
    let folder = folders.iter().next().unwrap();
    hand.wake(Woke::Paths(vec![folder.join("14.json")]));
    assert!(scanner.lists_now(&profiles, at));
    assert_eq!(lab.tick(&mut scanner, &os, at).entries.len(), 3);
    // So does one after a rescan (the system dropped events).
    let at = at + TICK;
    open(15, "s-4", "local_d");
    assert!(!scanner.lists_now(&profiles, at));
    hand.wake(Woke::Rescan);
    assert_eq!(lab.tick(&mut scanner, &os, at).entries.len(), 4);
    // Without events (the default) every tick lists, as before.
    assert!(Scanner::default().lists_now(&profiles, start));
}

/// The go sitting's rerun (2026-11-07): a profile started from its App
/// page was up in a second, but the page said Start until the next
/// minute's full read. After a start, each tick reads the whole table
/// for the window, then once a minute again.
#[test]
fn after_a_start_every_tick_looks_for_it_for_a_while() {
    let lab = Lab::new();
    let os = FakeOs::default();
    let mut scanner = Scanner::default();
    let t0 = Instant::now();
    lab.tick(&mut scanner, &os, t0);
    let reads = || *os.reads.lock().unwrap();
    let first = reads();
    lab.tick(&mut scanner, &os, t0 + TICK);
    assert_eq!(reads(), first, "no full read between minutes");

    scanner.full_for(t0 + TICK, Duration::from_secs(30));
    lab.tick(&mut scanner, &os, t0 + TICK * 2);
    assert_eq!(reads(), first + 1);
    assert!(
        scanner.take_apps().is_some(),
        "the apps found, for the places"
    );
    lab.tick(&mut scanner, &os, t0 + TICK * 3);
    assert_eq!(reads(), first + 2);
    // After the window: once a minute again.
    lab.tick(&mut scanner, &os, t0 + TICK * 2 + Duration::from_secs(31));
    let after = reads();
    lab.tick(&mut scanner, &os, t0 + TICK * 3 + Duration::from_secs(31));
    assert_eq!(reads(), after);
}

/// The go sitting's steady-state profile (2026-11-07): an idle AI Juggler
/// listed every running Claude app profile's session records each tick.
/// With file events on, a quiet tick lists nothing; a record no event
/// announces (late or lost: rc21, 2026-11-09) is read at the safety pass,
/// within [`APP_LIST_FALLBACK`]; one an event announces, or a rescan, at
/// the next tick. The events come by hand.
#[test]
fn a_quiet_tick_lists_no_app_records() {
    let lab = Lab::new();
    let os = FakeOs::default();
    lab.desktop(&os, 12, STARTED);
    app_file(&lab.instance, "local_a", "s-1", "completed", false);
    let mut scanner = Scanner::default();
    let (_, hand) = scanner.use_file_events_by_hand();
    let t0 = Instant::now();
    lab.tick(&mut scanner, &os, t0);
    assert_eq!(scanner.app_listings, 1);
    // Quiet: nothing listed again.
    for n in 1..5 {
        lab.tick(&mut scanner, &os, t0 + TICK * n);
    }
    assert_eq!(scanner.app_listings, 1, "a quiet tick lists nothing");
    let inactive = |model: &LiveModel, id: &str| model.inactive.iter().any(|(_, s)| s == id);
    // A new record whose event is late or lost: read at the safety pass.
    app_file(&lab.instance, "local_b", "s-2", "blocked", false);
    let model = lab.tick(&mut scanner, &os, t0 + TICK * 6);
    assert_eq!(scanner.app_listings, 1, "no event, no safety pass yet");
    assert!(!inactive(&model, "s-2"));
    let model = lab.tick(&mut scanner, &os, t0 + APP_LIST_FALLBACK);
    assert_eq!(scanner.app_listings, 2, "the safety pass, with no event");
    assert!(inactive(&model, "s-2"), "read: {:?}", model.inactive);
    // One an event announces: read at the next tick.
    app_file(&lab.instance, "local_c", "s-3", "blocked", false);
    let record = lab.instance.join("claude-code-sessions/a/o/local_c.json");
    hand.wake(Woke::Paths(vec![record]));
    let model = lab.tick(&mut scanner, &os, t0 + APP_LIST_FALLBACK + TICK);
    assert_eq!(scanner.app_listings, 3, "the change was heard");
    assert!(inactive(&model, "s-3"), "read: {:?}", model.inactive);
    // And after a rescan (the system dropped events).
    app_file(&lab.instance, "local_d", "s-4", "blocked", false);
    hand.wake(Woke::Rescan);
    let model = lab.tick(&mut scanner, &os, t0 + APP_LIST_FALLBACK + TICK * 2);
    assert_eq!(scanner.app_listings, 4);
    assert!(inactive(&model, "s-4"), "read: {:?}", model.inactive);
}

/// rc5 walk: a family nested only while its Claude app profile ran. The
/// user's own profile's records, read while it's closed too, say who
/// started whom; they're read again only when the records are listed.
#[test]
fn a_closed_profiles_records_still_say_who_started_whom() {
    let lab = Lab::new();
    let os = FakeOs::default();
    let record = |host: &str, cli: &str, from: Option<&str>, archived: bool| {
        let spawned = from.map_or(String::new(), |parent| {
            format!(r#","spawnedFrom":{{"sessionId":"{parent}","sideSession":true}}"#)
        });
        write(
            &lab.instance
                .join(format!("claude-code-sessions/a/o/{host}.json")),
            &format!(
                r#"{{"cliSessionId":"{cli}","title":"T {cli}","isArchived":{archived},"lastFocusedAt":1792829000000{spawned}}}"#
            ),
        );
    };
    record("local_p", "parent", None, false);
    record("local_c", "child", Some("local_p"), false);
    record("local_b", "shelved", Some("local_p"), true);
    let mut scanner = Scanner::default();
    let t0 = Instant::now();
    // Not running, not remembered: nothing known.
    assert!(lab.tick(&mut scanner, &os, t0).started.is_empty());
    scanner.remember(vec![("claude-app".to_owned(), lab.instance.clone())]);
    let model = lab.tick(&mut scanner, &os, t0 + TICK);
    let seen: Vec<(&str, &str)> = model
        .started
        .iter()
        .map(|link| (link.parent.as_str(), link.session_id.as_str()))
        .collect();
    assert_eq!(
        seen,
        [("parent", "child")],
        "an archived one isn't nested, as in the app"
    );
    // rc7 walk: the archived child read as plainly ended; its row says so.
    let archived: Vec<&str> = model.archived.iter().map(|(_, id)| id.as_str()).collect();
    assert_eq!(archived, ["shelved"]);
    assert!(
        model.entries.is_empty(),
        "nothing is open: still only the live view's"
    );
}
