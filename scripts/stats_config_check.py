#!/usr/bin/env python3
"""Usage stats are collected for a version before it's promoted.

  stats_config_check.py check X.Y.Z [--config FILE]   exit 0 = collected
  stats_config_check.py --selftest

promote.yml runs it for every beta and stable promotion. It reads the
usage-stats service's config (CONFIG_URL, the file the app itself reads
before it sends a weekly report, crates/juggler-stats/src/config.rs) and
fails, saying why, when the version's reports would be refused:

- the config can't be fetched, isn't JSON, or a field the app reads has
  the wrong type (a sample_rate outside (0, 1] makes it unusable);
- `enabled` is false or missing (the app reads a missing one as off);
- the version is below `min_app_version` or above `max_app_version`
  (strict SemVer, scripts/release_version.py);
- `known_versions` lists versions and this isn't one of them (the
  service keeps its reports out of the dashboards).

This changes nothing in the app: collection stays Off until the user
chooses. `--config FILE` reads a local copy instead of fetching (the
self-test's fixtures, scripts/fixtures/stats-config/). Only a GET, with a
timeout; nothing is sent.
"""
from __future__ import annotations

import json
import os
import sys
import urllib.error
import urllib.request
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from release_version import semver  # noqa: E402

CONFIG_URL = "https://stats.figurehead.ai/v1/config.json"
TIMEOUT_S = 15
MAX_BYTES = 64 * 1024
FIXTURES = Path(__file__).resolve().parent / "fixtures" / "stats-config"


class Unavailable(Exception):
    """The config couldn't be read."""


def fetch(url: str = CONFIG_URL, timeout: float = TIMEOUT_S) -> bytes:
    """The config's bytes: one HTTPS GET, redirects included only to HTTPS."""
    if not url.startswith("https://"):
        raise Unavailable(f"{url} isn't HTTPS")
    request = urllib.request.Request(url, headers={"Accept": "application/json", "User-Agent": "ai-juggler-promote"})
    try:
        with urllib.request.urlopen(request, timeout=timeout) as response:  # noqa: S310 (HTTPS only, above)
            if not response.geturl().startswith("https://"):
                raise Unavailable(f"{url} redirected off HTTPS")
            body = response.read(MAX_BYTES + 1)
    except (urllib.error.URLError, TimeoutError, OSError) as err:
        raise Unavailable(f"couldn't fetch {url}: {err}") from err
    if len(body) > MAX_BYTES:
        raise Unavailable(f"{url} is over {MAX_BYTES} bytes")
    return body


def failures(body: bytes, version: str, source: str = CONFIG_URL) -> list[str]:
    """Why `version`'s usage stats wouldn't be collected (empty: they would)."""
    semver(version)  # the caller's version is a real one (ValueError otherwise)
    try:
        config = json.loads(body)
    except (UnicodeDecodeError, json.JSONDecodeError) as err:
        return [f"the config at {source} isn't JSON ({err})"]
    if not isinstance(config, dict):
        return [f"the config at {source} isn't a JSON object"]
    malformed: list[str] = []
    enabled = config.get("enabled", False)
    if not isinstance(enabled, bool):
        malformed.append(f'"enabled" is {enabled!r}, not true or false')
    rate = config.get("sample_rate", 1.0)
    if isinstance(rate, bool) or not isinstance(rate, (int, float)) or not 0 < rate <= 1:
        malformed.append(f'"sample_rate" is {rate!r}, not a number in (0, 1]')
    bounds = {}
    for field in ("min_app_version", "max_app_version"):
        value = config.get(field)
        if value is None:
            continue
        try:
            bounds[field] = semver(value)
        except ValueError:
            malformed.append(f'"{field}" is {value!r}, not a SemVer version')
    known = config.get("known_versions", [])
    if not isinstance(known, list) or not all(isinstance(v, str) for v in known):
        malformed.append(f'"known_versions" is {known!r}, not a list of versions')
    if malformed:
        return [f"the config at {source} isn't valid: {m}" for m in malformed]

    problems: list[str] = []
    if not enabled:
        problems.append(
            f'collection is off at {source} ("enabled": false), so every report would be refused: '
            "turn it on in the service's config before promoting"
        )
    key = semver(version)
    if "min_app_version" in bounds and key < bounds["min_app_version"]:
        problems.append(
            f"{version} is below the service's min_app_version {config['min_app_version']}, "
            "so its reports would be refused: lower min_app_version or promote a newer version"
        )
    if "max_app_version" in bounds and key > bounds["max_app_version"]:
        problems.append(
            f"{version} is above the service's max_app_version {config['max_app_version']}, "
            "so its reports would be refused: raise max_app_version first"
        )
    if known and version not in known:
        problems.append(
            f"{version} isn't in the service's known_versions, so its reports would be kept "
            "out of the dashboards: add it there first"
        )
    return problems


def check(version: str, config_file: Path | None = None) -> tuple[bool, str]:
    """(stats are collected for `version`, the report to print)."""
    source = str(config_file) if config_file else CONFIG_URL
    try:
        body = config_file.read_bytes() if config_file else fetch()
    except (Unavailable, OSError) as err:
        return False, f"usage stats: not promoting — {err}; the promotion needs the service's answer"
    problems = failures(body, version, source)
    if not problems:
        return True, f"usage stats: collected for {version} ({source})"
    return False, "\n".join(f"usage stats: not promoting — {p}" for p in problems)


def selftest() -> int:
    fixture = lambda name: FIXTURES / f"{name}.json"  # noqa: E731
    ok, report = check("0.4.1", fixture("enabled"))
    assert ok and "collected for 0.4.1" in report, report
    ok, report = check("0.4.1", fixture("disabled"))
    assert not ok and '("enabled": false)' in report and "turn it on" in report, report
    ok, report = check("0.4.1", fixture("missing-enabled"))
    assert not ok and "collection is off" in report, "a missing enabled is off, as in the app"
    ok, report = check("0.4.0", fixture("min-0.4.1"))
    assert not ok and "0.4.0 is below the service's min_app_version 0.4.1" in report, report
    ok, report = check("0.4.1", fixture("min-0.4.1"))
    assert ok, f"equal to min_app_version passes: {report}"
    ok, report = check("0.10.0", fixture("min-0.4.1"))
    assert ok, f"0.10.0 is above 0.4.1 (numbers, not text): {report}"
    ok, report = check("0.4.1", fixture("min-0.4.1-rc.1"))
    assert ok, f"a release is above its own pre-release: {report}"
    ok, report = check("0.5.0", fixture("max-0.4.9"))
    assert not ok and "above the service's max_app_version 0.4.9" in report, report
    ok, report = check("0.4.1", fixture("known-versions"))
    assert ok, report
    ok, report = check("0.4.2", fixture("known-versions"))
    assert not ok and "known_versions" in report, report
    for name, words in (
        ("malformed", "isn't JSON"),
        ("not-an-object", "isn't a JSON object"),
        ("bad-enabled", '"enabled" is \'yes\''),
        ("bad-min", '"min_app_version" is \'v0.1\''),
        ("bad-rate", '"sample_rate" is 0'),
    ):
        ok, report = check("0.4.1", fixture(name))
        assert not ok and words in report, (name, report)
    ok, report = check("0.4.1", FIXTURES / "absent.json")
    assert not ok and "not promoting" in report, report
    try:
        fetch("http://stats.figurehead.ai/v1/config.json")
    except Unavailable as err:
        assert "isn't HTTPS" in str(err)
    else:
        raise AssertionError("fetched over plain HTTP")
    def unreachable() -> bytes:
        raise Unavailable("couldn't fetch it: timed out")

    real_fetch = globals()["fetch"]
    try:
        globals()["fetch"] = unreachable  # no network in the self-test
        ok, report = check("0.4.1")
        assert not ok and "couldn't fetch it: timed out" in report, report
    finally:
        globals()["fetch"] = real_fetch
    for bad in ("v0.4.1", "0.4"):
        try:
            check(bad, fixture("enabled"))
        except ValueError:
            continue
        raise AssertionError(f"accepted the version {bad!r}")
    print("stats-config-check selftest: OK")
    return 0


def main(args: list[str]) -> int:
    if args == ["--selftest"]:
        return selftest()
    if len(args) in (2, 4) and args[0] == "check" and (len(args) == 2 or args[2] == "--config"):
        try:
            ok, report = check(args[1], Path(args[3]) if len(args) == 4 else None)
        except ValueError as err:
            ok, report = False, f"usage stats: not promoting — {err}"
        prefix = "::error::" if not ok and os.environ.get("GITHUB_ACTIONS") == "true" else ""
        print("\n".join(prefix + line for line in report.splitlines()))
        return 0 if ok else 1
    print(__doc__, file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
