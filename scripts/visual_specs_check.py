#!/usr/bin/env python3
"""Every visual spec is run by a CI job, or says why not (D-117, D-118).

Two specs went unchecked unnoticed: eleven gallery families tagged a gate
state (`gate(...)`, the `@gate` tag) that ui/playwright.regression.config.ts
never listed in GALLERY_SPECS, so CI never compared them (cb445514a); and
attention-budget.spec.ts, an assertion spec, ran in no job at all until
scripts/a11y_gate.sh named it (9814a659f). For each
ui/tests/visual/**/*.spec.ts this fails when:

1. it tags a gate state but isn't in GALLERY_SPECS, unless
   scripts/visual_specs.allow lists it as `elsewhere`, naming the job that
   runs it instead (and a CI step really names it);
2. no CI job runs it: not compared by the gallery gate (in GALLERY_SPECS
   with a gate state), not matched by the regression config's own
   `testMatch`, and not named by any step CI runs (a `tests/visual/<x>.spec.ts`
   path, or a `playwright test <filter>` argument, in .github/workflows/*.yml
   or a script they call, followed script to script), unless the allowlist
   says why: `local` (by design never in CI: a wall-clock budget, a run
   behind its own switch) or `gap` (could run in CI, no job does yet; the
   list only shrinks).

And, so the lists can't rot: a GALLERY_SPECS name with no spec, or with no
gate state (the gallery project's `grep: /@gate/` then runs nothing from
it), fails; so does an allowlist line for a spec that's gone, one CI now
runs, or one with no reason. A whole-folder run (the on-demand sweep, which
reports and gates nothing) names no spec, so it doesn't count.

Usage: scripts/visual_specs_check.py [--selftest]
"""
from __future__ import annotations

import re
import shlex
import sys
import tempfile
from dataclasses import dataclass, field
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
VISUAL = "ui/tests/visual"
REGRESSION_CONFIG = "ui/playwright.regression.config.ts"
ALLOW = "scripts/visual_specs.allow"
WORKFLOWS = ".github/workflows"
SELF = "scripts/visual_specs_check.py"
KINDS = ("elsewhere", "local", "gap")

GATE = re.compile(r"\bgate\(|@gate\b")
SPEC_PATH = re.compile(r"tests/visual/([\w./-]+?)\.spec\.ts")
SCRIPT_REF = re.compile(r"scripts/[\w./-]+\.(?:sh|py|mjs|js)\b")
# Playwright options that take their value as the next word.
VALUE_OPTS = {
    "-c", "--config", "-g", "--grep", "--grep-invert", "--shard", "--reporter",
    "--project", "-j", "--workers", "--retries", "--timeout", "--output",
    "--max-failures", "--repeat-each", "--global-timeout", "--trace", "--tsconfig",
}


@dataclass
class Coverage:
    gallery: set[str] = field(default_factory=set)      # GALLERY_SPECS
    config_runs: bool = False                            # a CI step runs the config
    regression: list[re.Pattern] = field(default_factory=list)
    named: dict[str, set[str]] = field(default_factory=dict)  # spec -> CI files


def strip_comments(text: str, suffix: str) -> str:
    """Whole-line comments out, so prose naming a spec isn't a run of it."""
    mark = "//" if suffix in (".mjs", ".js", ".ts") else "#"
    return "\n".join("" if line.lstrip().startswith(mark) else line for line in text.splitlines())


def gallery_specs(config: str) -> set[str]:
    match = re.search(r"const GALLERY_SPECS\s*=\s*\[(.*?)\];", config, re.S)
    if not match:
        return set()
    body = re.sub(r"//[^\n]*", "", match.group(1))
    return {a or b for a, b in re.findall(r"\"([^\"]+)\"|'([^']+)'", body)}


def literal_test_matches(config: str) -> list[re.Pattern]:
    """The config's `testMatch: /regex/` literals (GALLERY_SPECS' is built)."""
    return [re.compile(m) for m in re.findall(r"testMatch:\s*/((?:\\/|[^/\n])+)/[a-z]*", config)]


def playwright_filters(text: str) -> list[str]:
    """The file filters of each `playwright test` command in `text`."""
    text = text.replace("\\\n", " ")
    filters = []
    for line in text.splitlines():
        at = line.find("playwright test")
        if at < 0:
            continue
        rest = re.split(r"\s(?:\|\|?|&&|;|>|2>)\s", line[at + len("playwright test"):])[0]
        try:
            words = shlex.split(rest)
        except ValueError:
            words = rest.split()
        skip = False
        for word in words:
            if skip:
                skip = False
            elif word.startswith("-"):
                skip = "=" not in word and word in VALUE_OPTS
            elif "$" not in word:
                filters.append(word)
    return filters


def ci_files(root: Path) -> dict[str, str]:
    """Every workflow, and every script one runs (followed script to
    script), relative path -> text without whole-line comments."""
    seen: dict[str, str] = {}
    queue = sorted(p.relative_to(root).as_posix() for p in (root / WORKFLOWS).glob("*.y*ml"))
    while queue:
        rel = queue.pop(0)
        if rel in seen or rel == SELF:  # the check names specs; it runs none
            continue
        path = root / rel
        if not path.is_file():
            continue
        text = strip_comments(path.read_text(encoding="utf-8", errors="replace"), path.suffix)
        seen[rel] = text
        queue.extend(ref for ref in SCRIPT_REF.findall(text) if ref not in seen)
    return seen


def specs(root: Path) -> dict[str, str]:
    base = root / VISUAL
    return {
        p.relative_to(base).as_posix()[: -len(".spec.ts")]: p.read_text(encoding="utf-8", errors="replace")
        for p in sorted(base.rglob("*.spec.ts"))
        if "node_modules" not in p.parts and "__baselines__" not in p.parts
    }


def coverage(root: Path, names: list[str]) -> Coverage:
    cov = Coverage()
    config_path = root / REGRESSION_CONFIG
    config = config_path.read_text(encoding="utf-8") if config_path.is_file() else ""
    cov.gallery = gallery_specs(config)
    files = ci_files(root)
    cov.config_runs = any(Path(REGRESSION_CONFIG).name in text for text in files.values())
    if cov.config_runs:
        cov.regression = literal_test_matches(config)
    for rel, text in files.items():
        hits = set(SPEC_PATH.findall(text))
        for flt in playwright_filters(text):
            try:
                pattern = re.compile(flt)
            except re.error:
                pattern = re.compile(re.escape(flt))
            hits |= {n for n in names if pattern.search(f"tests/visual/{n}.spec.ts")}
        for name in hits:
            cov.named.setdefault(name, set()).add(rel)
    return cov


def load_allow(root: Path) -> tuple[dict[str, tuple[str, str]], list[str]]:
    allow, problems = {}, []
    path = root / ALLOW
    if not path.is_file():
        return allow, problems
    for n, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        parts = line.split(None, 2)
        if len(parts) < 3 or parts[0] not in KINDS:
            problems.append(f"{ALLOW}:{n}: want `<{'|'.join(KINDS)}> <spec> <reason>`, got: {line}")
            continue
        allow[parts[1]] = (parts[0], parts[2])
    return allow, problems


def check(root: Path) -> tuple[list[str], dict[str, int]]:
    found = specs(root)
    names = sorted(found)
    cov = coverage(root, names)
    allow, problems = load_allow(root)
    tally = {"gallery": 0, "regression": 0, "named": 0, "local": 0, "gap": 0}

    for name in sorted(cov.gallery - set(found)):
        problems.append(f"{name}: in GALLERY_SPECS but there is no {VISUAL}/{name}.spec.ts")

    for name in names:
        gate = bool(GATE.search(found[name]))
        listed = name in cov.gallery
        regression = any(p.search(f"{name}.spec.ts") for p in cov.regression)
        named = sorted(cov.named.get(name, ()))
        kind, _reason = allow.get(name, (None, ""))
        if listed and not gate:
            problems.append(f"{name}: in GALLERY_SPECS but tags no gate() state, so the gallery gate runs nothing from it")
        if gate and not listed:
            if kind != "elsewhere":
                problems.append(f"{name}: tags a gate() state but isn't in GALLERY_SPECS ({REGRESSION_CONFIG}), so CI never compares it; list it there, or allowlist it as `elsewhere` naming the job that runs it")
            elif not (named or regression):
                problems.append(f"{name}: allowlisted as run elsewhere, but no CI step names it")
        elif kind == "elsewhere":
            problems.append(f"{name}: allowlisted as `elsewhere` but {'is in GALLERY_SPECS' if listed else 'tags no gate state'}; drop its line from {ALLOW}")
        compared = listed and gate and cov.config_runs
        if compared or regression or named:
            tally["gallery" if compared else "regression" if regression else "named"] += 1
            if kind in ("local", "gap"):
                problems.append(f"{name}: allowlisted as `{kind}` but CI runs it now ({', '.join(named) or 'the visual gate'}); drop its line from {ALLOW}")
        elif kind in ("local", "gap"):
            tally[kind] += 1
        elif not (gate and not listed):  # rule 1 already said it
            problems.append(f"{name}: run by no CI job (no gallery-gate compare, no regression testMatch, named by no workflow or script CI runs); add it to its job, or allowlist it in {ALLOW} with why")

    for name, (kind, reason) in sorted(allow.items()):
        if name not in found:
            problems.append(f"{name}: allowlisted ({kind}) but there is no {VISUAL}/{name}.spec.ts; drop its line")
        elif len(reason.strip()) < 12:
            problems.append(f"{name}: its allowlist line gives no real reason")
    return problems, tally


def selftest() -> int:
    failures = 0

    def expect(label: str, cond: bool, got: object = "") -> None:
        nonlocal failures
        if not cond:
            failures += 1
            print(f"selftest FAIL: {label} {got}")

    config = """
const GALLERY_SPECS = [
  "listed-gallery",
  // "commented-out",
];
export default defineConfig({ projects: [
  { name: "regression", testMatch: /regression\\.spec\\.ts/ },
  { name: "gallery", testMatch: new RegExp(`(${GALLERY_SPECS.join("|")})`), grep: /@gate/ },
]});
"""
    workflow = """
jobs:
  visual:
    steps:
      # tests/visual/commented.spec.ts is named only in prose here
      - run: pnpm exec playwright test -c playwright.regression.config.ts --shard="$SHARD/4"
  a11y:
    steps:
      - run: ./scripts/a11y_gate.sh --shard="$SHARD/6"
  happo:
    steps:
      - run: pnpm exec happo -- pnpm exec playwright test filtered --grep "x — 1280x800"
  static:
    steps:
      - run: python3 scripts/visual_specs_check.py
"""
    a11y = 'pnpm exec playwright test \\\n  tests/visual/assertion.spec.ts \\\n  "$@"\n'

    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        (root / VISUAL).mkdir(parents=True)
        (root / WORKFLOWS).mkdir(parents=True)
        (root / "scripts").mkdir()
        (root / REGRESSION_CONFIG).write_text(config, encoding="utf-8")
        (root / WORKFLOWS / "ci.yml").write_text(workflow, encoding="utf-8")
        (root / "scripts/a11y_gate.sh").write_text(a11y, encoding="utf-8")
        # The check itself, called by CI, names specs but runs none.
        (root / SELF).write_text('playwright test orphan\n"tests/visual/commented.spec.ts"\n', encoding="utf-8")

        def spec(name: str, body: str) -> None:
            (root / VISUAL / f"{name}.spec.ts").write_text(body, encoding="utf-8")

        def run() -> list[str]:
            return check(root)[0]

        spec("listed-gallery", 'test("a", gate("01-a"), async () => {});')
        spec("regression", 'test("rail", async () => {});')
        spec("assertion", 'test("budget", async () => { await page.goto("/"); navigate(); });')
        spec("filtered", 'test("x", async () => {});')
        expect("a listed gate spec, the regression spec, an a11y-gate spec and a filtered one pass", run() == [], run())

        spec("unlisted-gallery", 'test("b", { tag: "@gate" }, async () => {});')
        got = run()
        expect("an unlisted gate spec fails", any(p.startswith("unlisted-gallery: tags a gate() state") for p in got), got)
        expect("…once, not also as an orphan", len([p for p in got if p.startswith("unlisted-gallery")]) == 1, got)
        (root / ALLOW).write_text("elsewhere unlisted-gallery the a11y job (scripts/a11y_gate.sh) runs it\n", encoding="utf-8")
        got = run()
        expect("an `elsewhere` gate spec no CI step names fails", any("no CI step names it" in p for p in got), got)
        (root / "scripts/a11y_gate.sh").write_text(a11y + "tests/visual/unlisted-gallery.spec.ts\n", encoding="utf-8")
        expect("an `elsewhere` gate spec a CI script names passes", run() == [], run())

        spec("orphan", 'test("lonely", async () => {});')
        spec("commented", 'test("prose only", async () => {});')
        got = run()
        expect("an orphan spec fails", any(p.startswith("orphan: run by no CI job") for p in got), got)
        expect("a spec named only in a comment is an orphan", any(p.startswith("commented: run by no CI job") for p in got), got)

        allow = (root / ALLOW).read_text(encoding="utf-8")
        (root / ALLOW).write_text(allow + "local orphan a wall-clock budget, run by `just load-gate` only\ngap commented could run in CI; no job does yet\n", encoding="utf-8")
        expect("allowlisted specs pass", run() == [], run())
        expect("…and are tallied", check(root)[1]["local"] == 1 and check(root)[1]["gap"] == 1, check(root)[1])

        (root / ALLOW).write_text(allow + "local orphan a wall-clock budget, run by `just load-gate` only\ngap commented could run in CI; no job does yet\ngap assertion could run in CI\ngap gone-spec this spec was deleted long ago\nlocal filtered ok\n", encoding="utf-8")
        got = run()
        expect("an allowlisted spec CI now runs fails", any(p.startswith("assertion: allowlisted as `gap` but CI runs it now") for p in got), got)
        expect("an allowlisted spec that's gone fails", any(p.startswith("gone-spec: allowlisted") for p in got), got)
        expect("an allowlist line with no real reason fails", any(p.startswith("filtered: its allowlist line gives no real reason") for p in got), got)
        (root / ALLOW).write_text(allow + "local orphan a wall-clock budget, run by `just load-gate` only\ngap commented could run in CI; no job does yet\n", encoding="utf-8")

        spec("listed-gallery", 'test("a", async () => {});')
        got = run()
        expect("a GALLERY_SPECS spec with no gate state fails", any(p.startswith("listed-gallery: in GALLERY_SPECS but tags no gate()") for p in got), got)
        (root / VISUAL / "listed-gallery.spec.ts").unlink()
        got = run()
        expect("a GALLERY_SPECS name with no spec fails", any(p.startswith("listed-gallery: in GALLERY_SPECS but there is no") for p in got), got)
        spec("listed-gallery", 'test("a", gate("01-a"), async () => {});')

        (root / WORKFLOWS / "ci.yml").write_text("jobs: {}\n", encoding="utf-8")
        got = run()
        expect("with no CI step running the config, a listed gate spec is an orphan", any(p.startswith("listed-gallery: run by no CI job") for p in got), got)

    expect("GALLERY_SPECS parsing skips comments", gallery_specs(config) == {"listed-gallery"}, gallery_specs(config))
    expect("filters skip options and their values", playwright_filters('playwright test -c x.ts home --grep "a b" --shard=1/4 "$@"') == ["home"],
           playwright_filters('playwright test -c x.ts home --grep "a b" --shard=1/4 "$@"'))
    if failures:
        return 1
    print("visual-specs: selftest ok")
    return 0


def main() -> int:
    if "--selftest" in sys.argv:
        return selftest()
    problems, tally = check(ROOT)
    for problem in problems:
        print(f"visual-specs: {problem}")
    if problems:
        return 1
    total = sum(tally.values())
    print(f"visual-specs: {total} specs: {tally['gallery']} compared by the gallery gate, {tally['regression']} by the regression gate, "
          f"{tally['named']} named by a CI step; {tally['local']} local by design and {tally['gap']} known gaps ({ALLOW})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
