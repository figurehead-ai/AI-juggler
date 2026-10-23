//! AJ-E1518: the trust page's own checks.
//!
//! Every line on the trust page (`ui/src/lib/trust-claims.json`, drawn by
//! the app and by `docs/guide/trust.md`) names the tests that prove it. The
//! first test here proves each named test exists. The others are the
//! guarantees no earlier test made: where the network can be reached
//! from, which addresses, that the window reaches only the app, that no
//! tool is pointed at another address, and that no switch starts by
//! itself.

// Test crates are pragmatic by policy (QUALITY.md §2).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn read(path: &str) -> String {
    std::fs::read_to_string(repo().join(path)).unwrap_or_else(|error| panic!("{path}: {error}"))
}

/// The Rust source files under `dir`, each with its path from the repo.
fn rust_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

/// A source file without its test code: none for a file that is a test
/// module, else the file with each `#[cfg(test)] mod … { … }` taken out.
fn shipped_code(path: &Path) -> Option<String> {
    let name = path.file_name()?.to_string_lossy();
    if name == "tests.rs" || name.ends_with("_tests.rs") {
        return None;
    }
    let mut text = std::fs::read_to_string(path).ok()?;
    while let Some(at) = test_module(&text) {
        let end = block_end(&text, at).unwrap_or(text.len());
        text.replace_range(at..end, "");
    }
    Some(text)
}

/// Where the next `#[cfg(test)]` module starts.
fn test_module(text: &str) -> Option<usize> {
    let mut from = 0;
    while let Some(found) = text[from..].find("#[cfg(test)]") {
        let at = from + found;
        let rest = text[at + "#[cfg(test)]".len()..].trim_start();
        if rest.starts_with("mod ")
            || rest.starts_with("pub mod ")
            || rest.starts_with("pub(crate) mod ")
        {
            return Some(at);
        }
        from = at + 1;
    }
    None
}

/// The end of the braced block after `at`: braces counted outside strings,
/// characters and comments.
fn block_end(text: &str, at: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut depth = 0usize;
    let mut i = at;
    let mut opened = false;
    while i < bytes.len() {
        match bytes[i] {
            b'/' if bytes.get(i + 1) == Some(&b'/') => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            b'"' => {
                i += 1;
                while i < bytes.len() && bytes[i] != b'"' {
                    if bytes[i] == b'\\' {
                        i += 1;
                    }
                    i += 1;
                }
            }
            b'\'' if bytes.get(i + 2) == Some(&b'\'') => i += 2,
            b'\'' if bytes.get(i + 1) == Some(&b'\\') => {
                while i + 1 < bytes.len() && bytes[i + 1] != b'\'' {
                    i += 1;
                }
                i += 1;
            }
            b'{' => {
                depth += 1;
                opened = true;
            }
            b'}' => {
                depth = depth.checked_sub(1)?;
                if opened && depth == 0 {
                    return Some(i + 1);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
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

/// Whether `source` defines a test named `name`: a `fn name(` with a test
/// attribute in the few lines above it.
fn defines_test(source: &str, name: &str) -> bool {
    let lines: Vec<&str> = source.lines().collect();
    lines.iter().enumerate().any(|(n, line)| {
        let line = line.trim_start();
        (line.starts_with(&format!("fn {name}(")) || line.starts_with(&format!("async fn {name}(")))
            && lines[n.saturating_sub(6)..n]
                .iter()
                .any(|above| above.contains("#[test]") || above.contains("#[tokio::test"))
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
                    defines_test(&source, name),
                    "{:?} names {name}, which isn't a test in {path}",
                    claim.id
                );
            }
        }
    }
    assert!(ids.len() >= 15, "the page lost claims: {ids:?}");
}

// ---------------------------------------------------------------------
// The network

/// Crates that can open a connection: HTTP clients and servers, TLS
/// streams, WebSockets.
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
    "tauri-plugin-http",
    "tauri-plugin-updater",
    "tauri-plugin-websocket",
    "tauri-plugin-upload",
];

/// Who may use them, and why. A crate added here is a change to the
/// trust page: say what it reaches there.
fn network_allowed() -> BTreeMap<&'static str, BTreeSet<&'static str>> {
    BTreeMap::from([
        // The optional Figurehead account, and opt-in usage statistics
        // through it: figurehead.ai and stats.figurehead.ai.
        ("juggler-account", BTreeSet::from(["ureq", "rustls"])),
        // Fleet: only Macs the user paired, with pinned keys.
        (
            "juggler-link",
            BTreeSet::from(["hyper-util", "rustls", "tokio-rustls"]),
        ),
        // The update check (GitHub), and the test bridge's local server,
        // which is optional and only in the dev-bridge feature.
        (
            "juggler-app",
            BTreeSet::from(["tauri-plugin-updater", "hyper", "hyper-util"]),
        ),
    ])
}

/// A crate's dependencies that ship (not dev- or build-only), from its
/// `Cargo.toml`, with whether each is optional.
fn shipped_dependencies(manifest: &toml::Table) -> BTreeMap<String, bool> {
    let mut deps = BTreeMap::new();
    let mut tables: Vec<&toml::Table> = Vec::new();
    if let Some(table) = manifest.get("dependencies").and_then(toml::Value::as_table) {
        tables.push(table);
    }
    if let Some(targets) = manifest.get("target").and_then(toml::Value::as_table) {
        for target in targets.values().filter_map(toml::Value::as_table) {
            if let Some(table) = target.get("dependencies").and_then(toml::Value::as_table) {
                tables.push(table);
            }
        }
    }
    for table in tables {
        for (name, spec) in table {
            let name = spec
                .get("package")
                .and_then(toml::Value::as_str)
                .unwrap_or(name)
                .to_owned();
            let optional = spec.get("optional").and_then(toml::Value::as_bool) == Some(true);
            deps.insert(name, optional);
        }
    }
    deps
}

#[test]
fn only_the_listed_crates_can_reach_the_network() {
    let allowed = network_allowed();
    let mut seen = BTreeMap::new();
    for entry in std::fs::read_dir(repo().join("crates")).unwrap().flatten() {
        let manifest = entry.path().join("Cargo.toml");
        let Ok(text) = std::fs::read_to_string(&manifest) else {
            continue;
        };
        let manifest: toml::Table = toml::from_str(&text).unwrap();
        let name = manifest["package"]["name"].as_str().unwrap().to_owned();
        let deps = shipped_dependencies(&manifest);
        let network: BTreeSet<String> = deps
            .keys()
            .filter(|dep| NETWORK_CRATES.contains(&dep.as_str()))
            .cloned()
            .collect();
        let may: BTreeSet<String> = allowed
            .get(name.as_str())
            .map(|set| set.iter().map(|dep| (*dep).to_owned()).collect())
            .unwrap_or_default();
        let extra: Vec<&String> = network.difference(&may).collect();
        assert!(
            extra.is_empty(),
            "{name} can now reach the network through {extra:?}: say what it reaches on the trust page, then list it here"
        );
        if name == "juggler-app" {
            for server in ["hyper", "hyper-util"] {
                assert_eq!(
                    deps.get(server),
                    Some(&true),
                    "juggler-app's {server} is the test bridge's, and must stay optional"
                );
            }
        }
        seen.insert(name, network);
    }
    for (name, _) in allowed {
        assert!(
            seen.contains_key(name),
            "{name} isn't in the workspace any more"
        );
    }
}

fn host(url: &str) -> &str {
    url.split("://")
        .nth(1)
        .and_then(|rest| rest.split(['/', ':', '?']).next())
        .unwrap_or("")
}

#[test]
fn the_app_reaches_only_its_own_addresses() {
    // What the network crates above are pointed at.
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
    // The builds use those: no workflow or script points a build
    // somewhere else, and none gives crash reports an address to go to
    // (so this version can't send them).
    for name in [
        "AIJ_UPDATER_ENDPOINT",
        "AIJ_UPDATER_FEED_BASE",
        "AI_JUGGLER_CRASH_ENDPOINT",
        "AI_JUGGLER_FIGUREHEAD_URL",
    ] {
        for dir in [".github", "scripts"] {
            let mut stack = vec![repo().join(dir)];
            while let Some(dir) = stack.pop() {
                for entry in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        stack.push(path);
                    } else if let Ok(text) = std::fs::read_to_string(&path) {
                        assert!(
                            !text.contains(name),
                            "{} sets {name}: the trust page says where the app connects",
                            path.display()
                        );
                    }
                }
            }
        }
        assert!(!read("justfile").contains(name), "the justfile sets {name}");
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
    // And no window is given a way out: no HTTP, shell, file or
    // WebSocket permission.
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
            for denied in ["http:", "shell:", "fs:", "websocket:", "upload:"] {
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
/// provider.
const REDIRECTS: &[&str] = &[
    "ANTHROPIC_BASE_URL",
    "ANTHROPIC_API_URL",
    "ANTHROPIC_BEDROCK_BASE_URL",
    "ANTHROPIC_VERTEX_BASE_URL",
    "OPENAI_BASE_URL",
    "OPENAI_API_BASE",
    "CODEX_BASE_URL",
    "HTTPS_PROXY",
    "HTTP_PROXY",
    "ALL_PROXY",
    "https_proxy",
    "http_proxy",
    "all_proxy",
];

#[test]
fn ai_juggler_never_points_a_tool_at_another_address() {
    // AI Juggler's own code never names one, so it can't set one. A value
    // the user typed into a launch setting's environment is the user's.
    let mut found = Vec::new();
    for crate_dir in std::fs::read_dir(repo().join("crates")).unwrap().flatten() {
        let src = crate_dir.path().join("src");
        if !src.is_dir() {
            continue;
        }
        for path in rust_files(&src) {
            let Some(code) = shipped_code(&path) else {
                continue;
            };
            for name in REDIRECTS {
                if code.contains(&format!("\"{name}\"")) {
                    found.push(format!("{} names {name}", path.display()));
                }
            }
        }
    }
    assert!(found.is_empty(), "{found:#?}");
}

// ---------------------------------------------------------------------
// Switching

/// What moves work to another account or Mac.
const MOVES: &[&str] = &[
    "session_handover_impl",
    "switch_execute_impl",
    "switch_execute_core",
    "switch_undo_impl",
    "switch_back_execute_impl",
    "switch_back_execute_to_impl",
    "fleet_switch_execute_impl",
    "fleet_switch_back_execute_impl",
];

/// The window's commands that call them: each runs when the user clicks.
const ON_CLICK: &[&str] = &[
    "session_handover",
    "switch_execute",
    "switch_undo",
    "switch_back_execute",
    "fleet_switch_execute",
    "fleet_switch_back_execute",
];

/// The `fn` a byte offset sits in: the nearest `fn name` before it.
fn enclosing_fn(code: &str, at: usize) -> Option<String> {
    let before = &code[..at];
    let start = before.rfind("fn ")? + 3;
    let name: String = code[start..]
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    Some(name)
}

#[test]
fn nothing_switches_on_its_own() {
    let mut calls = Vec::new();
    let mut commands = BTreeSet::new();
    for path in rust_files(&repo().join("crates/juggler-app/src")) {
        let Some(code) = shipped_code(&path) else {
            continue;
        };
        for command in ON_CLICK {
            if let Some(at) = code
                .find(&format!("pub async fn {command}("))
                .or_else(|| code.find(&format!("pub fn {command}(")))
            {
                let above = &code[code[..at].rfind("\n\n").unwrap_or(0)..at];
                assert!(
                    above.contains("#[tauri::command"),
                    "{command} in {} isn't a window command",
                    path.display()
                );
                commands.insert(*command);
            }
        }
        for name in MOVES {
            let call = format!("{name}(");
            let mut from = 0;
            while let Some(found) = code[from..].find(&call) {
                let at = from + found;
                from = at + call.len();
                let preceded = code[..at].chars().next_back();
                if preceded.is_some_and(|c| c.is_alphanumeric() || c == '_')
                    || code[..at].ends_with("fn ")
                    || code[..at].ends_with('`')
                    || code[..at].ends_with('[')
                {
                    continue;
                }
                let line = code[..at].lines().last().unwrap_or("");
                if line.trim_start().starts_with("//") {
                    continue;
                }
                let caller = enclosing_fn(&code, at).unwrap_or_default();
                calls.push((path.display().to_string(), caller, (*name).to_owned()));
            }
        }
    }
    assert_eq!(
        commands.len(),
        ON_CLICK.len(),
        "a window command moved: found {commands:?}"
    );
    let allowed: BTreeSet<&str> = MOVES.iter().chain(ON_CLICK).copied().collect();
    let strays: Vec<_> = calls
        .iter()
        .filter(|(_, caller, _)| !allowed.contains(caller.as_str()))
        .collect();
    assert!(
        strays.is_empty(),
        "work moves from something the user didn't click: {strays:#?}"
    );
    assert!(
        calls.len() >= ON_CLICK.len(),
        "the scan found no calls: {calls:?}"
    );
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
        // The preview and the gallery show what the app shows.
        for text in [&terms.name, &terms.url, &terms.accounts, &terms.checked] {
            assert!(
                preview.contains(text.as_str()),
                "ui/src/preview/tools.ts doesn't carry {}'s {text:?}",
                spec.id
            );
        }
        // Each line is research, recorded with its source and day.
        assert!(
            register.contains(&terms.url) || register.contains(host(&terms.url)),
            "the register doesn't record {}",
            terms.url
        );
        assert!(
            register.contains(&terms.checked),
            "the register has no check on {}",
            terms.checked
        );
    }
}
