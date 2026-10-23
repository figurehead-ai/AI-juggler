//! AJ-E1518: the Privacy page's own checks.
//!
//! Every line on the Privacy page (`ui/src/lib/trust-claims.json`, drawn by
//! the app and by `docs/guide/trust.md`) names the tests that prove it. The
//! first test here proves each named test exists. The others are the
//! guarantees no earlier test made:
//! - where the network can be reached from, and which addresses;
//! - that the window reaches only the app;
//! - that no tool is pointed at another address;
//! - that crash reports have nowhere to go;
//! - that no switch starts by itself.

// Test crates are pragmatic by policy (QUALITY.md §2).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

mod common;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use common::source::{code_only, functions, product, sources, without_comments};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn read(path: &str) -> String {
    std::fs::read_to_string(repo().join(path)).unwrap_or_else(|error| panic!("{path}: {error}"))
}

/// A crate's shipped code, file by file: its `src/`, without test files
/// (`tests.rs`, `*_tests.rs`, anything under a `tests/` folder) or
/// `#[cfg(test)]` modules.
fn shipped(crate_name: &str) -> Vec<(PathBuf, String)> {
    let mut files = Vec::new();
    sources(
        &repo().join("crates").join(crate_name).join("src"),
        &mut files,
    );
    files.sort();
    files
        .into_iter()
        .filter(|path| {
            let name = path.file_name().unwrap().to_string_lossy();
            name != "tests.rs"
                && !name.ends_with("_tests.rs")
                && !path.components().any(|part| part.as_os_str() == "tests")
        })
        .map(|path| {
            let text = product(&std::fs::read_to_string(&path).unwrap());
            (path, text)
        })
        .collect()
}

fn workspace_crates() -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(repo().join("crates"))
        .unwrap()
        .flatten()
        .filter(|entry| entry.path().join("Cargo.toml").is_file())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

// ---------------------------------------------------------------------
// The claims

#[derive(serde::Deserialize)]
struct Claims {
    groups: Vec<Group>,
}

#[derive(serde::Deserialize)]
struct Group {
    title: String,
    claims: Vec<Claim>,
}

#[derive(serde::Deserialize)]
struct Claim {
    id: String,
    text: String,
    tests: Vec<String>,
}

fn claims() -> Claims {
    serde_json::from_str(&read("ui/src/lib/trust-claims.json")).unwrap()
}

/// Whether `source` defines a test named `name` that runs:
/// - in Rust, `fn name(` with `#[test]` or `#[tokio::test…]` among the
///   attribute and doc lines just above it, and no `#[ignore]` there;
/// - in TypeScript, `it("name"` or `test("name"`, not `.skip`ped.
fn defines_test(path: &str, source: &str, name: &str) -> bool {
    if path.ends_with(".ts") || path.ends_with(".tsx") {
        return [format!("it(\"{name}\""), format!("test(\"{name}\"")]
            .iter()
            .any(|call| {
                source
                    .match_indices(call.as_str())
                    .any(|(at, _)| !source[..at].ends_with('.') && !source[..at].ends_with("skip"))
            });
    }
    let lines: Vec<&str> = source.lines().collect();
    lines.iter().enumerate().any(|(n, line)| {
        let line = line.trim_start();
        if !(line.starts_with(&format!("fn {name}("))
            || line.starts_with(&format!("async fn {name}(")))
        {
            return false;
        }
        let above: Vec<&str> = lines[..n]
            .iter()
            .rev()
            .map(|line| line.trim())
            .take_while(|line| line.starts_with("#[") || line.starts_with("///"))
            .collect();
        above
            .iter()
            .any(|line| *line == "#[test]" || line.starts_with("#[tokio::test"))
            && !above.iter().any(|line| line.starts_with("#[ignore"))
    })
}

#[test]
fn every_trust_claim_names_tests_that_exist() {
    let claims = claims();
    let doc = read("docs/guide/trust.md");
    let mut ids = BTreeSet::new();
    for group in &claims.groups {
        assert!(
            doc.contains(&group.title),
            "docs/guide/trust.md lacks the heading {:?}",
            group.title
        );
        for claim in &group.claims {
            assert!(
                ids.insert(claim.id.clone()),
                "two claims are {:?}",
                claim.id
            );
            assert!(!claim.tests.is_empty(), "{:?} names no test", claim.id);
            assert!(
                doc.contains(&claim.text),
                "docs/guide/trust.md doesn't say {:?} word for word",
                claim.text
            );
            for reference in &claim.tests {
                let (path, name) = reference
                    .split_once("::")
                    .unwrap_or_else(|| panic!("{reference}: not path::test"));
                let source = std::fs::read_to_string(repo().join(path))
                    .unwrap_or_else(|_| panic!("{:?} names {path}, which isn't there", claim.id));
                assert!(
                    defines_test(path, &source, name),
                    "{:?} names {name}, which isn't a running test in {path}",
                    claim.id
                );
                assert!(
                    doc.contains(name),
                    "docs/guide/trust.md doesn't cite {name}"
                );
            }
        }
    }
    assert!(ids.len() >= 15, "the page lost claims: {ids:?}");
}

/// The check above can tell a test from a helper, and an ignored or
/// skipped test from a running one.
#[test]
fn a_claim_needs_a_running_test() {
    let source = "#[test]\nfn real() {}\n\nfn helper() {}\n\n#[test]\n#[ignore]\nfn parked() {}\n\n/// Doc.\n#[tokio::test(flavor = \"multi_thread\")]\nasync fn live() {}\n";
    assert!(defines_test("a.rs", source, "real"));
    assert!(defines_test("a.rs", source, "live"));
    assert!(!defines_test("a.rs", source, "helper"));
    assert!(!defines_test("a.rs", source, "parked"));
    assert!(defines_test(
        "a.tsx",
        "it(\"does it\", () => {});",
        "does it"
    ));
    assert!(!defines_test(
        "a.tsx",
        "it.skip(\"does it\", () => {});",
        "does it"
    ));
}

// ---------------------------------------------------------------------
// The network

/// Crates that can open a connection: HTTP clients and servers, TLS,
/// WebSockets, gRPC, discovery, raw sockets.
const NETWORK_CRATES: &[&str] = &[
    "ureq",
    "reqwest",
    "hyper",
    "hyper-util",
    "isahc",
    "curl",
    "surf",
    "attohttpc",
    "h2",
    "h3",
    "quinn",
    "rustls",
    "tokio-rustls",
    "native-tls",
    "tokio-native-tls",
    "tungstenite",
    "tokio-tungstenite",
    "async-tungstenite",
    "websocket",
    "tonic",
    "mdns-sd",
    "libmdns",
    "zeroconf",
    "socket2",
    "tauri-plugin-http",
    "tauri-plugin-updater",
    "tauri-plugin-websocket",
    "tauri-plugin-upload",
];

/// Which crates may reach the network, through which, and why. A crate
/// added here is a change to the Privacy page: say what it reaches there.
fn network_allowed() -> BTreeMap<&'static str, BTreeSet<&'static str>> {
    BTreeMap::from([
        // The optional Figurehead account, and opt-in usage statistics
        // through it: figurehead.ai and stats.figurehead.ai.
        ("juggler-account", BTreeSet::from(["ureq", "rustls"])),
        // Fleet: only Macs the user paired, with pinned keys (TLS, gRPC),
        // found by announcing on the local network (mDNS); tokio's sockets.
        (
            "juggler-link",
            BTreeSet::from([
                "hyper-util",
                "rustls",
                "tokio-rustls",
                "tonic",
                "mdns-sd",
                "socket2",
            ]),
        ),
        // The update check (GitHub); Tauri's own HTTP client; Fleet's
        // listener (`link.rs`, tokio's sockets). The test bridge's local
        // server (hyper) is off unless the dev-bridge feature is on.
        (
            "juggler-app",
            BTreeSet::from(["tauri-plugin-updater", "reqwest", "socket2"]),
        ),
    ])
}

/// The first network crate on each path out of each workspace crate,
/// through its normal (shipped) dependencies, with default features, as
/// `cargo metadata` resolves them. Workspace crates are checked as
/// themselves, so a path stops at one.
fn network_reached() -> BTreeMap<String, BTreeMap<String, String>> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let output = std::process::Command::new(cargo)
        .args(["metadata", "--format-version", "1", "--offline"])
        .current_dir(repo())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let metadata: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let names: BTreeMap<&str, &str> = metadata["packages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|package| {
            (
                package["id"].as_str().unwrap(),
                package["name"].as_str().unwrap(),
            )
        })
        .collect();
    let members: BTreeSet<&str> = metadata["workspace_members"]
        .as_array()
        .unwrap()
        .iter()
        .map(|id| id.as_str().unwrap())
        .collect();
    let nodes: BTreeMap<&str, &serde_json::Value> = metadata["resolve"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|node| (node["id"].as_str().unwrap(), node))
        .collect();
    let mut reached = BTreeMap::new();
    for member in &members {
        let mut found = BTreeMap::new();
        let mut seen = BTreeSet::from([*member]);
        let mut stack = vec![(*member, names[member].to_owned())];
        while let Some((id, path)) = stack.pop() {
            for dep in nodes[id]["deps"].as_array().unwrap() {
                let shipped = dep["dep_kinds"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|kind| kind["kind"].is_null());
                let to = dep["pkg"].as_str().unwrap();
                if !shipped || members.contains(to) {
                    continue;
                }
                let name = names[to];
                let via = format!("{path} > {name}");
                if NETWORK_CRATES.contains(&name) {
                    found.entry(name.to_owned()).or_insert(via);
                } else if seen.insert(to) {
                    stack.push((to, via));
                }
            }
        }
        reached.insert(names[member].to_owned(), found);
    }
    reached
}

#[test]
fn only_the_listed_crates_can_reach_the_network() {
    let allowed = network_allowed();
    let reached = network_reached();
    for name in workspace_crates() {
        let found = reached
            .get(&name)
            .unwrap_or_else(|| panic!("{name} isn't a workspace member"));
        let may = allowed.get(name.as_str()).cloned().unwrap_or_default();
        let extra: Vec<&String> = found
            .iter()
            .filter(|(crate_name, _)| !may.contains(crate_name.as_str()))
            .map(|(_, via)| via)
            .collect();
        assert!(
            extra.is_empty(),
            "{name} can reach the network through {extra:?}: say what it reaches on the Privacy page, then list it here"
        );
    }
    // The test bridge's server is in no build that doesn't ask for it.
    assert!(
        !reached["juggler-app"].contains_key("hyper"),
        "the test bridge's hyper is in the default build: {:?}",
        reached["juggler-app"]
    );
    // And no other crate opens a socket of its own, with std or tokio.
    for name in workspace_crates() {
        if allowed.contains_key(name.as_str()) {
            continue;
        }
        for (path, text) in shipped(&name) {
            let code = code_only(&text);
            for socket in ["TcpStream", "TcpListener", "UdpSocket", "tokio::net"] {
                assert!(
                    !code.contains(socket),
                    "{} opens a socket ({socket})",
                    path.display()
                );
            }
        }
    }
}

fn host(url: &str) -> &str {
    url.split("://")
        .nth(1)
        .and_then(|rest| rest.split(['/', ':', '?', '"', ' ']).next())
        .unwrap_or("")
}

/// The hosts the network crates' own code may name: ours, and test or
/// internal names that never leave the Mac.
const OUR_HOSTS: &[&str] = &[
    "figurehead.ai",
    "stats.figurehead.ai",
    "github.com",
    // Debug builds' local endpoints.
    "localhost",
    // Fleet's HTTP authority over a pinned TLS connection to a paired Mac.
    "aijuggler.node",
];

/// Every `http://…` or `https://…` in `code`, by host.
fn hosts_in(code: &str) -> Vec<String> {
    let mut out = Vec::new();
    for scheme in ["https://", "http://"] {
        let mut from = 0;
        while let Some(at) = code[from..].find(scheme) {
            let start = from + at;
            out.push(host(&code[start..]).to_owned());
            from = start + scheme.len();
        }
    }
    out
}

#[test]
fn the_app_reaches_only_its_own_addresses() {
    // What the network crates are pointed at.
    assert_eq!(host(juggler_account::PRODUCTION_URL), "figurehead.ai");
    assert_eq!(
        host(juggler_stats::cycle::REPORTS_URL),
        "stats.figurehead.ai"
    );
    assert_eq!(
        host(juggler_stats::cycle::CONFIG_URL),
        "stats.figurehead.ai"
    );
    assert!(
        juggler_app::updates::FEED_BASE.starts_with("https://github.com/figurehead-ai/"),
        "{}",
        juggler_app::updates::FEED_BASE
    );
    // Every address the network code names is one of ours: the crates
    // that make requests, and the app's files that hand requests to them.
    let mut checked = 0;
    for name in ["juggler-account", "juggler-stats", "juggler-link"] {
        for (path, text) in shipped(name) {
            for found in hosts_in(&without_comments(&text)) {
                assert!(
                    found.is_empty() || OUR_HOSTS.contains(&found.as_str()),
                    "{} names {found}",
                    path.display()
                );
                checked += 1;
            }
        }
    }
    for (path, text) in shipped("juggler-app") {
        if !(text.contains("HttpRequest") || text.contains("Transport")) {
            continue;
        }
        for found in hosts_in(&without_comments(&text)) {
            assert!(
                found.is_empty() || OUR_HOSTS.contains(&found.as_str()),
                "{} hands a request {found}",
                path.display()
            );
            checked += 1;
        }
    }
    assert!(
        checked >= 4,
        "the scan found almost no addresses ({checked})"
    );
    // The builds use those: no workflow, script, Cargo config or the
    // justfile points a build at another address.
    for name in [
        "AIJ_UPDATER_ENDPOINT",
        "AIJ_UPDATER_FEED_BASE",
        "AI_JUGGLER_FIGUREHEAD_URL",
    ] {
        assert_eq!(build_settings_naming(name), Vec::<String>::new(), "{name}");
    }
}

/// The build's own settings (CI workflows, scripts, Cargo config, the
/// justfile) that name `variable`.
fn build_settings_naming(variable: &str) -> Vec<String> {
    let mut found = Vec::new();
    for dir in [".github", "scripts", ".cargo"] {
        let mut stack = vec![repo().join(dir)];
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if std::fs::read_to_string(&path).is_ok_and(|text| text.contains(variable)) {
                    found.push(path.display().to_string());
                }
            }
        }
    }
    if read("justfile").contains(variable) {
        found.push("justfile".to_owned());
    }
    found
}

#[test]
fn crash_reports_have_nowhere_to_go() {
    // No build is given the reporting address.
    assert_eq!(
        build_settings_naming("AI_JUGGLER_CRASH_ENDPOINT"),
        Vec::<String>::new()
    );
    // The address is read only to say whether sending is possible, and the
    // crash code holds no way to send.
    let crash = product(&read("crates/juggler-app/src/crash.rs"));
    let code = code_only(&crash);
    let uses: Vec<usize> = code
        .match_indices("service_address()")
        .map(|(at, _)| at)
        .filter(|at| !code[..*at].ends_with("fn "))
        .collect();
    assert_eq!(
        uses.len(),
        1,
        "crash.rs uses the reporting address beyond `can_send`"
    );
    assert!(code[uses[0]..].starts_with("service_address().is_some()"));
    for sender in ["Transport", "HttpRequest", "ureq", "reqwest", "TcpStream"] {
        assert!(!code.contains(sender), "crash.rs can send: {sender}");
    }
    for (path, text) in shipped("juggler-app") {
        if path.ends_with("crash.rs") {
            continue;
        }
        assert!(
            !code_only(&text).contains("service_address"),
            "{} uses the crash reporting address",
            path.display()
        );
    }
}

#[test]
fn the_window_can_reach_nothing_but_the_app() {
    let conf: serde_json::Value =
        serde_json::from_str(&read("crates/juggler-app/tauri.conf.json")).unwrap();
    let csp = conf["app"]["security"]["csp"].as_str().unwrap();
    let directives: BTreeMap<&str, &str> = csp
        .split(';')
        .filter_map(|part| part.trim().split_once(' '))
        .collect();
    assert_eq!(directives.get("default-src"), Some(&"'self'"), "{csp}");
    assert_eq!(
        directives.get("connect-src"),
        Some(&"ipc: http://ipc.localhost"),
        "the window may talk to the app and nothing else: {csp}"
    );
    // The other editions change names and icons, never the policy.
    for edition in ["tauri.dev.conf.json", "tauri.nightly.conf.json"] {
        let other: serde_json::Value =
            serde_json::from_str(&read(&format!("crates/juggler-app/{edition}"))).unwrap();
        assert!(
            other["app"]["security"].is_null(),
            "{edition} overrides the window's security"
        );
    }
    // And no window is given a way out: no HTTP, shell, file, WebSocket or
    // opener permission (links out go through the app's own commands).
    for entry in std::fs::read_dir(repo().join("crates/juggler-app/capabilities"))
        .unwrap()
        .flatten()
    {
        let capability: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(entry.path()).unwrap()).unwrap();
        for permission in capability["permissions"].as_array().unwrap() {
            let permission = permission
                .as_str()
                .or_else(|| permission["identifier"].as_str())
                .unwrap();
            for denied in ["http:", "shell:", "fs:", "websocket:", "upload:", "opener:"] {
                assert!(
                    !permission.starts_with(denied),
                    "{} grants {permission}",
                    entry.path().display()
                );
            }
        }
    }
}

/// The settings that send a tool's traffic somewhere other than its
/// provider: environment variables and Codex config keys.
const REDIRECTS: &[&str] = &[
    "ANTHROPIC_BASE_URL",
    "ANTHROPIC_API_URL",
    "ANTHROPIC_BEDROCK_BASE_URL",
    "ANTHROPIC_VERTEX_BASE_URL",
    "CLAUDE_CODE_USE_BEDROCK",
    "CLAUDE_CODE_USE_VERTEX",
    "apiKeyHelper",
    "OPENAI_BASE_URL",
    "OPENAI_API_BASE",
    "CODEX_BASE_URL",
    "openai_base_url",
    "chatgpt_base_url",
    "model_providers",
    "HTTPS_PROXY",
    "HTTP_PROXY",
    "ALL_PROXY",
    "https_proxy",
    "http_proxy",
    "all_proxy",
];

#[test]
fn ai_juggler_never_points_a_tool_at_another_address() {
    // AI Juggler's own shipped code never names one, in any file under a
    // crate's `src/` (code, and files it includes), so it can't set one. A
    // value the user typed into a launch setting's environment is theirs.
    let mut found = Vec::new();
    for name in workspace_crates() {
        let mut files = Vec::new();
        let mut stack = vec![repo().join("crates").join(&name).join("src")];
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if path.file_name().is_some_and(|dir| dir != "tests") {
                        stack.push(path);
                    }
                } else {
                    files.push(path);
                }
            }
        }
        for path in files {
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            let file = path.file_name().unwrap().to_string_lossy();
            if file == "tests.rs" || file.ends_with("_tests.rs") {
                continue;
            }
            let text = if file.ends_with(".rs") {
                product(&text)
            } else {
                text
            };
            for redirect in REDIRECTS {
                if without_comments(&text).contains(redirect) {
                    found.push(format!("{} names {redirect}", path.display()));
                }
            }
        }
    }
    assert!(found.is_empty(), "{found:#?}");
}

// ---------------------------------------------------------------------
// Switching

/// What moves work to another account or Mac: on this Mac, and on this
/// Mac when a paired Mac sends it.
const MOVES: &[&str] = &[
    "session_handover_impl",
    "switch_execute_impl",
    "switch_execute_core",
    "switch_undo_impl",
    "switch_back_execute_impl",
    "switch_back_execute_to_impl",
    "fleet_switch_execute_impl",
    "fleet_switch_back_execute_impl",
    "take_handover_here",
    "switch_back_here",
];

/// Where they may be called from: the window's commands, each run by a
/// click here; and Fleet's node handler, run by a paired Mac's request
/// (made by a click there).
const ON_CLICK: &[&str] = &[
    "session_handover",
    "switch_execute",
    "switch_undo",
    "switch_back_execute",
    "fleet_switch_execute",
    "fleet_switch_back_execute",
];
const FROM_A_PAIRED_MAC: &[&str] = &["take_handover"];

#[test]
fn nothing_switches_on_its_own() {
    let mut calls = Vec::new();
    let mut commands = BTreeSet::new();
    for (path, text) in shipped("juggler-app") {
        let code = code_only(&text);
        for (function, body) in functions(&code) {
            if ON_CLICK.contains(&function.as_str()) {
                let at = code.find(body).unwrap();
                let above = &code[code[..at].rfind("\n\n").unwrap_or(0)..at];
                assert!(
                    above.contains("#[tauri::command"),
                    "{function} in {} isn't a window command",
                    path.display()
                );
                commands.insert(function.clone());
            }
            for name in MOVES {
                let call = format!("{name}(");
                let mut from = 0;
                while let Some(found) = body[from..].find(&call) {
                    let at = from + found;
                    from = at + call.len();
                    let before = body[..at].chars().next_back();
                    if before.is_some_and(|c| c.is_alphanumeric() || c == '_')
                        || body[..at].trim_end().ends_with("fn")
                    {
                        continue;
                    }
                    calls.push((
                        path.display().to_string(),
                        function.clone(),
                        (*name).to_owned(),
                    ));
                }
            }
        }
    }
    assert_eq!(
        commands.len(),
        ON_CLICK.len(),
        "a window command moved: found {commands:?}"
    );
    let allowed: BTreeSet<&str> = MOVES
        .iter()
        .chain(ON_CLICK)
        .chain(FROM_A_PAIRED_MAC)
        .copied()
        .collect();
    let strays: Vec<_> = calls
        .iter()
        .filter(|(_, caller, _)| !allowed.contains(caller.as_str()))
        .collect();
    assert!(
        strays.is_empty(),
        "work moves from something nobody clicked: {strays:#?}"
    );
    assert!(
        calls.len() >= ON_CLICK.len(),
        "the scan found no calls: {calls:?}"
    );
}

/// The scan above sees past comments and strings, so neither can make a
/// stray call look allowed or hide one.
#[test]
fn the_switch_scan_sees_through_comments_and_strings() {
    let code = code_only(
        "fn ingest() {\n    // like fn switch_execute(\n    let _ = \"fn switch_execute(\";\n    switch_execute_impl(state);\n}\n",
    );
    let found: Vec<(String, bool)> = functions(&code)
        .into_iter()
        .map(|(name, body)| (name, body.contains("switch_execute_impl(")))
        .collect();
    assert_eq!(found, vec![("ingest".to_owned(), true)]);
}

// ---------------------------------------------------------------------
// The vendors' terms

#[test]
fn every_tool_names_its_terms_and_the_preview_matches() {
    let preview = read("ui/src/preview/tools.ts");
    let register = read("docs/qa/evidence/AJ-E1518/terms-claims-register.md");
    let registry = juggler_app::state::default_registry();
    let ids = registry.ids();
    assert!(!ids.is_empty());
    for spec in ids
        .iter()
        .filter_map(|id| registry.get(id))
        .map(|adapter| adapter.spec())
    {
        let terms = spec
            .terms
            .unwrap_or_else(|| panic!("{} names no terms", spec.id));
        assert!(terms.url.starts_with("https://"), "{}", terms.url);
        // The preview shows what the app shows, in the tool's own entry.
        let entry = preview
            .split("\n  {\n")
            .find(|entry| entry.contains(&format!("id: \"{}\"", spec.id)))
            .unwrap_or_else(|| panic!("the preview has no {}", spec.id));
        for text in [&terms.name, &terms.url, &terms.accounts, &terms.checked] {
            assert!(
                entry.contains(text.as_str()),
                "the preview's {} entry doesn't carry {text:?}",
                spec.id
            );
        }
        // The line is research, recorded word for word with its source.
        assert!(
            register.contains(&terms.accounts),
            "the register doesn't record {}'s line word for word",
            spec.id
        );
        assert!(
            register.contains(host(&terms.url)),
            "the register doesn't record {}",
            terms.url
        );
    }
}
