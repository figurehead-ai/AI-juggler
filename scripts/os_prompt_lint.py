#!/usr/bin/env python3
"""AJ-E1547 (PAT-OSP-4): a system prompt is raised only through the gate.

AI Juggler never raises a system prompt the user wasn't told about first.
The one gate is `juggler_platform::os_prompt`
(`crates/juggler-platform/src/os_prompt/`) with the app's half,
`crates/juggler-app/src/os_prompts/`; the registry of every prompt is
`docs/design/os-prompts.md`. This lint fails when an API that can make the
system ask is called anywhere else:

- `ax-prompt`      the Accessibility ask (`AXIsProcessTrustedWithOptions`
                   with its prompt option, `application_is_trusted_with_prompt`)
- `screen`         Screen Recording's request (`ScreenCaptureAccess`,
                   `CGRequestScreenCaptureAccess`)
- `script`         `AppleScript` and Apple events that may ask
                   (`osascript`, `NSAppleScript`, `OSAScript`,
                   `AEDeterminePermission`)
- `link-event`     an "open this link" event that may ask
                   (`send_link(`, not `send_link_quietly`; defined in
                   `juggler_platform::apps`)
- `notify-auth`    notification authorisation (`requestAuthorization`,
                   `UNUserNotificationCenter`) and the notification
                   plugin's `NotificationExt` (registering it asks nothing)
- `login-item`     `SMAppService` (only its audited shim) and turning it on
                   (`juggler_login_item::set(`)
- `screen-read`    reading the screen's windows (`SCShareableContent::get`),
                   only after the gate's quiet status check
- `network`        a listener on every interface or Bonjour (the local
                   network prompt), only where Link is gated
- `tcc-reset`      `tccutil`: never, anywhere (a security setting)

And a **companion** rule: a file that starts one of AI Juggler's own
helpers (`features/ai/engines/helper.rs`) must ask the gate
(`helper_may_start`) before it spawns.

`scripts/os_prompts.allow` lists a call another session is still to route
through the gate (`path rule  # why`); it **only shrinks**: an entry that
no longer matches anything fails until it's removed. Test code is left out.

Usage: os_prompt_lint.py [--selftest]
"""
from __future__ import annotations

import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import rust_tests  # noqa: E402  (one test-code finder for every lint, D-94)

ALLOW = ROOT / "scripts" / "os_prompts.allow"
GATE = "crates/juggler-platform/src/os_prompt/"
APP_GATE = "crates/juggler-app/src/os_prompts/"

RULES: list[tuple[str, re.Pattern[str], tuple[str, ...]]] = [
    ("ax-prompt", re.compile(r"AXIsProcessTrustedWithOptions|kAXTrustedCheckOptionPrompt|application_is_trusted_with_prompt"), (GATE,)),
    ("screen", re.compile(r"ScreenCaptureAccess|CGRequestScreenCaptureAccess"), (GATE,)),
    ("script", re.compile(r"/usr/bin/osascript|\"osascript\"|\bNSAppleScript\b|\bOSAScript\b|\bAEDeterminePermission\b"), (GATE,)),
    # A call to the gate's own `os_prompt::send_link` is the routed one.
    ("link-event", re.compile(r"(?<!os_prompt::)\bsend_link\s*\("), (GATE, "crates/juggler-platform/src/apps.rs")),
    ("notify-auth", re.compile(r"requestAuthorization|UNUserNotificationCenter|\bNotificationExt\b"), (GATE, APP_GATE + "notify.rs")),
    ("login-item", re.compile(r"\bSMAppService\b|registerAndReturnError"), ("crates/juggler-login-item/src/lib.rs",)),
    ("login-item", re.compile(r"\bjuggler_login_item::set\s*\("), (GATE,)),
    ("screen-read", re.compile(r"SCShareableContent::get"), ("crates/juggler-app/src/preview.rs",)),
    ("network", re.compile(r"bind\(\(\s*\"0\.0\.0\.0\"|ServiceDaemon::new"), ("crates/juggler-app/src/link.rs", "crates/juggler-link/src/discovery.rs")),
    ("tcc-reset", re.compile(r"\btccutil\b"), ()),
]

# A file allowed a rule above must also go through the gate: what it must name.
COMPANIONS: dict[str, tuple[str, str]] = {
    "crates/juggler-app/src/preview.rs": ("screen-read", "screen_recording_status"),
    "crates/juggler-app/src/link.rs": ("network", "LOCAL_NETWORK"),
    "crates/juggler-app/src/features/ai/engines/helper.rs": ("helper-start", "helper_may_start"),
}


def code_lines(text: str) -> list[tuple[int, str]]:
    """Lines outside the test modules (rust_tests), comments stripped."""
    out = []
    for number, line in enumerate(rust_tests.blank_tests(text).splitlines(), 1):
        code = line.split("//", 1)[0]
        out.append((number, code))
    return out


def allowed_entries(path: pathlib.Path) -> set[tuple[str, str]]:
    entries = set()
    if not path.is_file():
        return entries
    for raw in path.read_text(encoding="utf-8").splitlines():
        line = raw.split("#", 1)[0].strip()
        if line:
            file, rule = line.split()
            entries.add((file, rule))
    return entries


def findings(root: pathlib.Path, allow_path: pathlib.Path | None = None) -> list[str]:
    allow = allowed_entries(allow_path or root / "scripts" / "os_prompts.allow")
    used: set[tuple[str, str]] = set()
    out = []
    tests = rust_tests.test_files(root)
    for path in sorted((root / "crates").rglob("*.rs")):
        rel = path.relative_to(root).as_posix()
        if "/tests/" in rel or "/target/" in rel or "/examples/" in rel or "/benches/" in rel or rel in tests:
            continue
        text = path.read_text(encoding="utf-8", errors="replace")
        for number, code in code_lines(text):
            for name, pattern, places in RULES:
                if not pattern.search(code) or any(rel.startswith(place) for place in places):
                    continue
                if (rel, name) in allow:
                    used.add((rel, name))
                    continue
                out.append(f"{rel}:{number}: {name}: only through the system prompts' gate (juggler_platform::os_prompt)")
        if rel in COMPANIONS:
            name, needed = COMPANIONS[rel]
            if needed not in text:
                if (rel, name) in allow:
                    used.add((rel, name))
                else:
                    out.append(f"{rel}: {name}: must go through the gate ({needed})")
    for script in sorted((root / "scripts").rglob("*")):
        rel = script.relative_to(root).as_posix()
        if not script.is_file() or rel.endswith("os_prompt_lint.py") or script.suffix not in {".py", ".sh", ".mjs", ".ts", ""}:
            continue
        for number, line in enumerate(script.read_text(encoding="utf-8", errors="replace").splitlines(), 1):
            if re.search(r"\btccutil\b", line):
                out.append(f"{rel}:{number}: tcc-reset: never reset the system's privacy settings")
    for file, rule in sorted(allow - used):
        out.append(f"scripts/os_prompts.allow: {file} {rule} matches nothing now: remove it (the list only shrinks)")
    return out


def selftest() -> None:
    import tempfile
    with tempfile.TemporaryDirectory() as tmp:
        root = pathlib.Path(tmp)

        def put(rel: str, text: str) -> None:
            path = root / rel
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text)

        put("crates/x/src/lib.rs",
            'fn a() { std::process::Command::new("/usr/bin/osascript"); }\n'
            "// application_is_trusted_with_prompt in a comment is fine\n"
            "fn b() { juggler_login_item::set(true); }\n"
            "#[cfg(test)]\nmod tests {\n    fn t() { let _ = \"osascript\"; }\n}\n")
        put("crates/juggler-platform/src/os_prompt/mac.rs", 'const OSASCRIPT: &str = "/usr/bin/osascript";\n')
        put("crates/juggler-app/src/os_prompts/notify.rs", "use tauri_plugin_notification::NotificationExt;\n")
        put("crates/y/src/lib.rs", "use tauri_plugin_notification::NotificationExt;\nfn c() { juggler_platform::apps::send_link(1, \"x\"); }\n"
            "fn g() { os_prompt::send_link(APP_LINKS, user(), 1, \"x\"); }\n")
        put("crates/juggler-app/src/preview.rs", "fn p() { SCShareableContent::get(); }\n")
        put("crates/juggler-app/src/features/ai/engines/helper.rs", "fn launch() { command.spawn(); }\n")
        put("crates/z/src/lib.rs", "fn d() { juggler_platform::apps::send_link(1, \"x\"); }\n")
        put("scripts/reset.sh", "tccutil reset All\n")
        allow = root / "scripts/os_prompts.allow"
        allow.write_text("crates/z/src/lib.rs link-event  # routed\ncrates/gone.rs script  # stale\n")
        found = findings(root, allow)
        assert found == [
            "crates/juggler-app/src/features/ai/engines/helper.rs: helper-start: must go through the gate (helper_may_start)",
            "crates/juggler-app/src/preview.rs: screen-read: must go through the gate (screen_recording_status)",
            "crates/x/src/lib.rs:1: script: only through the system prompts' gate (juggler_platform::os_prompt)",
            "crates/x/src/lib.rs:3: login-item: only through the system prompts' gate (juggler_platform::os_prompt)",
            "crates/y/src/lib.rs:1: notify-auth: only through the system prompts' gate (juggler_platform::os_prompt)",
            "crates/y/src/lib.rs:2: link-event: only through the system prompts' gate (juggler_platform::os_prompt)",
            "scripts/reset.sh:1: tcc-reset: never reset the system's privacy settings",
            "scripts/os_prompts.allow: crates/gone.rs script matches nothing now: remove it (the list only shrinks)",
        ], found
    print("os-prompt-lint: selftest ok")


def main() -> int:
    if "--selftest" in sys.argv:
        selftest()
        return 0
    found = findings(ROOT)
    for line in found:
        print(f"os-prompt-lint: {line}", file=sys.stderr)
    if not found:
        print("os-prompt-lint: OK (every system prompt goes through juggler_platform::os_prompt)")
    return 1 if found else 0


if __name__ == "__main__":
    sys.exit(main())
