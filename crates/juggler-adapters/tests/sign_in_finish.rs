//! AJ-E1545 R8 and AJ-E1553, per adapter, on fixtures only: each tool's
//! sign-in output is read by its own adapter (the page and the device code
//! it shows, a page on another host never), a sign-in is seen to finish by
//! the adapter's own "signed in?" check (`SignInWatch`), one that ends
//! early keeps its last lines (made safe), and an entry left for a folder
//! that now reads signed in is stale (cleared on restart).
//!
//! The outputs are the shapes each tool prints (its docs and the research
//! notes in `docs/research/`); every code, address and sign-in is made up.
//! No tool is run and no real folder or credential is read.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]

mod grok_fixture;
mod hermes_fixture;
mod kimi_fixture;
mod muse_fixture;
mod zcode_fixture;

use std::path::Path;

use juggler_adapter::prelude::*;
use juggler_adapter::sign_in_page::{SignInWatch, last_lines, plain_lines, signed_in};
use juggler_adapters::{
    ClaudeCodeAdapter, CodexAdapter, GrokBuildAdapter, HermesAdapter, KimiCodeAdapter, MuseAdapter,
    OpenCodeAdapter, ZCodeAdapter,
};

/// The three checks every adapter passes: not done while its folder reads
/// signed out, done once `sign_in` writes its sign-in, and stale after
/// (an entry for it is cleared on restart).
fn finishes<A: ToolSignIn + ?Sized>(adapter: &A, state: &Path, sign_in: impl FnOnce()) {
    let watch = SignInWatch::start(adapter, state);
    assert!(!watch.done(adapter, state), "done before any sign-in");
    assert!(!signed_in(adapter, state), "stale before any sign-in");
    sign_in();
    assert!(watch.done(adapter, state), "the sign-in wasn't seen");
    assert!(
        signed_in(adapter, state),
        "a signed-in folder's entry isn't stale"
    );
}

/// A sign-in that ends early: never done, and its last lines are kept,
/// a page's query and a token-like word taken out.
fn ends_early<A: ToolSignIn + ?Sized>(adapter: &A, state: &Path, output: &str) {
    let watch = SignInWatch::start(adapter, state);
    let text = plain_lines(output.as_bytes());
    assert!(!watch.done(adapter, state));
    let kept = last_lines(&text, 4);
    assert!(!kept.is_empty(), "no lines kept");
    assert!(
        kept.iter().all(|line| !line.contains('?')),
        "a query was kept: {kept:?}"
    );
    assert!(kept.iter().any(|line| line.contains("cancel")), "{kept:?}");
}

const EARLY: &str =
    "\r\nSigning in was cancelled (exit 1)\r\nsession abcdefghijklmnopqrstuvwxyz012345 closed\r\n";

#[test]
fn claude_code_reads_its_page_and_a_code_to_paste_back() {
    let adapter = ClaudeCodeAdapter::new();
    let url = "https://claude.ai/oauth/authorize?code=true&client_id=00000000-0000-0000-0000-000000000000&response_type=code&redirect_uri=https%3A%2F%2Fconsole.anthropic.com%2Foauth%2Fcode%2Fcallback&state=made-up";
    let output = format!(
        "\x1b[2J Browser didn't open? Use the url below to sign in (c to copy)\r\n\r\n{url}\r\n\r\n Paste code here if prompted >\r\n"
    );
    let page = adapter
        .sign_in_page(&plain_lines(output.as_bytes()))
        .unwrap();
    assert_eq!(page.url, url);
    assert_eq!(page.code, None, "Claude Code shows no device code");
    assert!(page.pastes_code);
    assert_eq!(
        adapter.sign_in_page("https://auth.openai.com/oauth/authorize?x=1\n"),
        None,
        "another tool's page"
    );

    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("acct");
    adapter.prepare_account(&state).unwrap();
    finishes(&adapter, &state, || {
        std::fs::write(state.join(".credentials.json"), b"made-up").unwrap();
    });
    // Signing in again over a sign-in: only a new one written counts.
    let again = SignInWatch::start(&adapter, &state);
    assert!(
        !again.done(&adapter, &state),
        "the old sign-in ended the new one"
    );
    std::thread::sleep(std::time::Duration::from_millis(20));
    std::fs::write(state.join(".credentials.json"), b"made-up, written again").unwrap();
    assert!(again.done(&adapter, &state));

    let fresh = dir.path().join("b");
    adapter.prepare_account(&fresh).unwrap();
    ends_early(&adapter, &fresh, &format!("{output}{EARLY}"));
}

#[test]
fn codex_reads_its_browser_page_and_its_device_code() {
    let adapter = CodexAdapter::with_bin("/bin/echo");
    let browser = "Starting local login server on http://localhost:1455.\nIf your browser did not open, navigate to this URL to authenticate:\n\nhttps://auth.openai.com/oauth/authorize?response_type=code&client_id=app_madeup&state=s\n";
    let page = adapter.sign_in_page(browser).unwrap();
    assert!(
        page.url
            .starts_with("https://auth.openai.com/oauth/authorize?")
    );
    assert_eq!(page.code, None);
    let device = "Follow these steps to sign in with ChatGPT using device code authorization:\n\n1. Open this link in your browser and sign in to your account\n   https://auth.openai.com/codex/device\n\n2. Enter this one-time code (expires in 15 minutes)\n   WDJB-MJHTK\n";
    let page = adapter.sign_in_page(device).unwrap();
    assert_eq!(page.url, "https://auth.openai.com/codex/device");
    assert_eq!(page.code.as_deref(), Some("WDJB-MJHTK"));
    assert!(!page.pastes_code);

    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("acct");
    adapter.prepare_account(&state).unwrap();
    finishes(&adapter, &state, || {
        std::fs::write(
            state.join("auth.json"),
            br#"{"auth_mode":"chatgpt","tokens":{"access_token":"made-up","account_id":"acct-made-up"}}"#,
        )
        .unwrap();
    });
    let fresh = dir.path().join("b");
    adapter.prepare_account(&fresh).unwrap();
    ends_early(&adapter, &fresh, &format!("{device}{EARLY}"));
}

#[test]
fn grok_build_reads_xais_page_and_code() {
    let adapter = GrokBuildAdapter::with_bin("/bin/echo");
    let output = "Sign in to Grok Build\nOpen https://auth.x.ai/oauth2/device?user_code=QWER-TYUP to continue\nYour code: QWER-TYUP\n";
    let page = adapter.sign_in_page(output).unwrap();
    assert!(page.url.starts_with("https://auth.x.ai/oauth2/device"));
    assert_eq!(page.code.as_deref(), Some("QWER-TYUP"));
    assert_eq!(
        adapter.sign_in_page("https://x.ai.evil.test/device\n"),
        None
    );

    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    finishes(&adapter, &home, || grok_fixture::sign_in(&home, "made-up"));
    let fresh = dir.path().join("b");
    std::fs::create_dir_all(&fresh).unwrap();
    ends_early(&adapter, &fresh, &format!("{output}{EARLY}"));
}

#[test]
fn kimi_code_reads_its_device_page_and_code() {
    let adapter = KimiCodeAdapter::with_bin("/bin/echo");
    let output = "Please visit the following URL to finish authorization:\nhttps://www.kimi.com/code/authorize_device?user_code=WDJB-MJHT\nVerification code: WDJB-MJHT\n";
    let page = adapter.sign_in_page(output).unwrap();
    assert!(
        page.url
            .starts_with("https://www.kimi.com/code/authorize_device")
    );
    assert_eq!(page.code.as_deref(), Some("WDJB-MJHT"));
    let global = adapter
        .sign_in_page("Open https://auth.kimi.ai/device\nCode: ABCD-2345\n")
        .unwrap();
    assert_eq!(global.code.as_deref(), Some("ABCD-2345"));

    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    finishes(&adapter, &home, || {
        kimi_fixture::sign_in_membership(&home, "made-up")
    });
    let fresh = dir.path().join("b");
    std::fs::create_dir_all(&fresh).unwrap();
    ends_early(&adapter, &fresh, &format!("{output}{EARLY}"));
}

#[test]
fn opencode_reads_the_page_of_the_service_picked_in_its_list() {
    let dir = tempfile::tempdir().unwrap();
    let adapter = OpenCodeAdapter::with_bin("/bin/echo").with_home(dir.path());
    // Its list first: nothing to read yet (the terminal stays shown).
    assert_eq!(
        adapter
            .sign_in_page("┌  Add credential\n◆  Select provider\n│  ● OpenCode (recommended)\n"),
        None
    );
    let copilot = "◇  Select provider\n│  GitHub Copilot\n│\n◇  Go to: https://github.com/login/device\n│\n◇  Enter code: 8F43-6B2A\n";
    let page = adapter.sign_in_page(copilot).unwrap();
    assert_eq!(page.url, "https://github.com/login/device");
    assert_eq!(page.code.as_deref(), Some("8F43-6B2A"));
    let chatgpt =
        "◇  Go to: https://auth.openai.com/oauth/authorize?client_id=app_madeup&state=s\n";
    assert_eq!(adapter.sign_in_page(chatgpt).unwrap().code, None);

    let state = dir.path().join("a");
    adapter.prepare_account(&state).unwrap();
    finishes(&adapter, &state, || {
        std::fs::write(
            state.join("data/opencode/auth.json"),
            r#"{"openai":{"type":"oauth","access":"made-up","refresh":"made-up","expires":1}}"#,
        )
        .unwrap();
    });
    let fresh = dir.path().join("b");
    adapter.prepare_account(&fresh).unwrap();
    ends_early(&adapter, &fresh, &format!("{copilot}{EARLY}"));
}

#[test]
fn zcode_reads_zais_page() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().canonicalize().unwrap();
    let home = base.join("home");
    std::fs::create_dir_all(&home).unwrap();
    let engine = zcode_fixture::app(&base.join("Applications"), "dev.zcode.app");
    let adapter = ZCodeAdapter::with_engine(&engine).with_home(&home);
    let output = "Opening your browser to sign in to Z.ai…\nIf it didn't open: https://chat.z.ai/oauth/authorize?client_id=zcode&state=s\n";
    let page = adapter.sign_in_page(output).unwrap();
    assert!(page.url.starts_with("https://chat.z.ai/oauth/authorize"));

    let state = base.join("accounts/a");
    adapter.prepare_account(&state).unwrap();
    finishes(&adapter, &state, || {
        zcode_fixture::sign_in(&state.join(".zcode"), "made-up")
    });
    let fresh = base.join("accounts/b");
    adapter.prepare_account(&fresh).unwrap();
    ends_early(&adapter, &fresh, &format!("{output}{EARLY}"));
}

#[test]
fn muse_code_reads_metas_device_page() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().canonicalize().unwrap().join("home");
    let adapter = MuseAdapter::new().with_home(&home);
    let output =
        "To sign in, open https://www.meta.ai/device and enter the code: KQXZ-PLMN\nWaiting…\n";
    let page = adapter.sign_in_page(output).unwrap();
    assert_eq!(page.url, "https://www.meta.ai/device");
    assert_eq!(page.code.as_deref(), Some("KQXZ-PLMN"));

    let own = home.join(".config/muse");
    std::fs::create_dir_all(&own).unwrap();
    finishes(&adapter, &own, || muse_fixture::sign_in(&own, "made-up"));
}

#[test]
fn hermes_reads_the_page_of_the_service_picked() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().canonicalize().unwrap();
    hermes_fixture::stub(&base.join("home/.local/bin/hermes"));
    let adapter = HermesAdapter::new()
        .with_home(base.join("home"))
        .with_system_policy(base.join("etc-hermes"));
    let output = "Select provider: Nous Portal\nOpen https://portal.nousresearch.com/device/verify?user_code=ABCD-EFGH\nCode: ABCD-EFGH\n";
    let page = adapter.sign_in_page(output).unwrap();
    assert!(
        page.url
            .starts_with("https://portal.nousresearch.com/device/verify")
    );
    assert_eq!(page.code.as_deref(), Some("ABCD-EFGH"));

    let state = base.join("accounts/a");
    adapter.prepare_account(&state).unwrap();
    finishes(&adapter, &state, || {
        hermes_fixture::sign_in(&state, "made-up")
    });
    let fresh = base.join("accounts/b");
    adapter.prepare_account(&fresh).unwrap();
    ends_early(&adapter, &fresh, &format!("{output}{EARLY}"));
}
