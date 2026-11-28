#!/usr/bin/env python3
"""AJ-E1558 (D-126 rich mode, PAT-COLOR §5.13, TGL-2, RC-2, DESIGN-SYSTEM
§2.2): Settings, the edit and create sheets and the pickers are making
screens, never "monochrome hell" (the maintainer, 0.5.1 Nightly).

The rich mode's dials say an icon or logo goes on every choice, row and
section that holds choices, and the accent marks selection and progress.
This lint reads, in each file of those surfaces, every component (a
top-level `function Name(` or `const Name = (`) that builds a choice (a
switch, checkbox, radio, choice card or line, picker, segmented control,
date or time field, or a Settings row) and counts it as `neutral` when it
is built only from neutral tokens:

- no icon: no `<Icon…>` drawn, no `icon=`/`icon:` given to a kit row or
  option (`icon={null}` keeps a column empty, so it doesn't count), no App
  or tool logo (`ToolLogo`, `AppIcon`, `AppMark`, `ToolMark`,
  `MemberLogo`), no Settings tile (`PaneTile`, `PaneIcon`);
- and no semantic or accent token: no `text-`/`bg-`/`fill-`/`stroke-`
  class in `accent`, `accent-hi`, `accent-fill`, `accent-soft`,
  `success`, `warning`, `live` (their `-soft`/`-strong` steps too), no
  `CHOSEN_CARD`, no `tone="accent|success|live|warning"`. An error line's
  `text-danger` doesn't count: it shows only when something failed.

Which files: every `.tsx` under `ui/src/screens/settings/`, every
component file a Settings pane imports (`ui/src/components/…`, read from
the panes' imports, so a new card is in scope the day it is added), the
edit and create sheets (`*Sheet*.tsx`, the new-session screens, the watch
form, the pool set-up steps), and the pickers (`*Picker*`, `*Pick.tsx`,
`*Chooser*`, `*Choice*`). Not scanned: the kit's own homes
(`components/controls/`, which take their icon from the caller), tests,
the preview's data, generated types.

Its allowlist `scripts/rich_mode.allow` (`path count`) **only shrinks**:
a file over its count, or not listed, fails; a file under its count fails
until its line is lowered or removed (`--print` gives today's counts).

Usage: rich_mode_lint.py [--print] [--selftest] [--root DIR]
"""
from __future__ import annotations

import re
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ALLOW_NAME = "scripts/rich_mode.allow"
EXEMPT = ("/tests/", ".test.", ".spec.", "/types/generated/", "/preview/", "/test/", "/components/controls/")
NAMED = re.compile(r"(?:Sheet|Picker|Chooser|Choice|Choices)[A-Za-z]*\.tsx$|Pick\.tsx$")
DIRS = ("ui/src/screens/settings/", "ui/src/screens/new-session/", "ui/src/pages/pool/setup/")
FILES = ("ui/src/components/watches/WatchForm.tsx",)
IMPORT = re.compile(r"""from\s+["']((?:\.\./)+components/[A-Za-z/]+)["']""")
COMMENT = re.compile(r"/\*[\s\S]*?\*/|(?<![:\\\"'])//[^\n]*")
COMPONENT = re.compile(r"^(?:export\s+)?(?:default\s+)?(?:function\s+([A-Z]\w*)\s*[<(]|const\s+([A-Z]\w*)\s*=\s*(?:\(|function|forwardRef|memo))", re.M)
CHOICE = re.compile(r"<(?:Checkbox|Radio|ChoiceOption|Switch|SwitchRow|SettingRow|ToggleRow|CheckRow|Picker|Segmented|DateField|TimeField|RadioCard|CheckCard)\b")
ICON = re.compile(
    r"<Icon[A-Z]\w*|<(?:ToolLogo|AppIcon|AppMark|ToolMark|MemberLogo|PaneTile|PaneIcon)\b"
    r"|\bicon=\{(?!\s*(?:null|undefined)\s*\})|\bicon:\s*(?!null\b|undefined\b)[A-Z{(]|\bIcon[A-Z]\w*\b"
)
COLOUR = re.compile(
    r"(?<![\w-])(?:[\w\[\]:/-]*:)?(?:text|bg|fill|stroke)-(?:accent(?:-hi|-fill|-soft)?|success|warning|live)(?:-soft|-strong)?(?:/\d+)?(?![\w-])"
    r"|\bCHOSEN_CARD\b|\btone=\"(?:accent|success|live|warning)\""
)


def components(text: str) -> list[tuple[str, str]]:
    """Each top-level component's name and its source (to the next one)."""
    code = COMMENT.sub("", text)
    starts = [(m.start(), m.group(1) or m.group(2)) for m in COMPONENT.finditer(code)]
    out = []
    for i, (at, name) in enumerate(starts):
        end = starts[i + 1][0] if i + 1 < len(starts) else len(code)
        out.append((name, code[at:end]))
    return out


def iconned_consts(text: str) -> set[str]:
    """Top-level CONSTANT tables whose entries carry an icon (a picker's or
    a card set's options, defined once above the component)."""
    code = COMMENT.sub("", text)
    out = set()
    for m in re.finditer(r"^(?:export\s+)?const\s+([A-Z][A-Z0-9_]*)\b", code, re.M):
        start = code.find("\n", m.end()) + 1 or len(code)
        nxt = re.search(r"^\S", code[start:], re.M)
        block = code[m.end(): start + (nxt.start() + 1 if nxt else len(code))]
        if ICON.search(block):
            out.add(m.group(1))
    return out


def neutral_in(text: str) -> list[str]:
    """The components that build a choice from neutral tokens alone."""
    tables = iconned_consts(text)
    uses = lambda body: any(re.search(rf"\b{name}\b", body) for name in tables)
    return [name for name, body in components(text) if CHOICE.search(body) and not ICON.search(body) and not COLOUR.search(body) and not uses(body)]


def in_scope(root: Path) -> list[Path]:
    src = root / "ui" / "src"
    if not src.exists():
        return []
    picked: set[Path] = set()
    for path in src.rglob("*.tsx"):
        rel = path.relative_to(root).as_posix()
        if any(part in f"/{rel}" for part in EXEMPT):
            continue
        if NAMED.search(path.name) or rel.startswith(DIRS) or rel in FILES:
            picked.add(path)
    # The cards a Settings pane draws: whatever a pane imports from components.
    settings = src / "screens" / "settings"
    for pane in settings.rglob("*.tsx") if settings.exists() else []:
        if any(part in f"/{pane.as_posix()}" for part in EXEMPT):
            continue
        for spec in IMPORT.findall(pane.read_text(encoding="utf-8", errors="replace")):
            target = (pane.parent / f"{spec}.tsx").resolve()
            rel = target.relative_to(root.resolve()).as_posix() if target.is_relative_to(root.resolve()) else ""
            if target.is_file() and not any(part in f"/{rel}" for part in EXEMPT):
                picked.add(root / rel)
    return sorted(picked)


def scan(root: Path) -> dict[str, list[str]]:
    found: dict[str, list[str]] = {}
    for path in in_scope(root):
        names = neutral_in(path.read_text(encoding="utf-8", errors="replace"))
        if names:
            found[path.relative_to(root).as_posix()] = names
    return found


def allowed(path: Path) -> dict[str, int]:
    out: dict[str, int] = {}
    if not path.exists():
        return out
    for line in path.read_text(encoding="utf-8").splitlines():
        line = line.split("#", 1)[0].strip()
        if line:
            file, count = line.split()
            out[file] = int(count)
    return out


WHY = (
    "a choice built from neutral tokens alone (no icon, no semantic or accent token): give each row and option "
    "its icon (ToggleRow/CheckRow's chip, SettingRow's icon, RadioCard's tile, a Picker option's icon) and mark "
    "on and chosen in the accent's soft step (D-126 rich mode, PAT-COLOR, DESIGN-SYSTEM §2.2)"
)


def problems(found: dict[str, list[str]], allow: dict[str, int]) -> list[str]:
    out = []
    for file, names in sorted(found.items()):
        cap = allow.get(file, 0)
        if len(names) > cap:
            out.append(f"{file}: {len(names)} neutral ({', '.join(names)}){'' if cap == 0 else f' (allowed {cap})'}: {WHY}")
    for file, cap in sorted(allow.items()):
        n = len(found.get(file, []))
        if n < cap:
            out.append(f"{file}: {n} neutral, the allowlist says {cap}: lower its line in {ALLOW_NAME} (it only shrinks)")
    return out


def selftest() -> int:
    bare = 'export function Bare(): JSX.Element {\n  return <label className="flex text-ink"><Checkbox checked /> <span className="text-muted">Pause</span></label>;\n}\n'
    assert neutral_in(bare) == ["Bare"], neutral_in(bare)
    assert neutral_in(bare.replace("<span", "<IconPause /><span")) == []
    assert neutral_in('function Row() {\n  return <ToggleRow icon={IconBell} title="x" checked onChange={f} />;\n}\n') == []
    assert neutral_in('function Row() {\n  return <ChoiceOption type="radio" icon={null} label="x" />;\n}\n') == ["Row"]
    assert neutral_in('function Row() {\n  return <SettingRow title="x" control={<Switch checked />} />;\n}\n') == ["Row"]
    assert neutral_in('function Row() {\n  return <span className="bg-accent-soft text-accent-hi"><Picker choice /></span>;\n}\n') == []
    assert neutral_in('function Row() {\n  return <><Picker choice /><p className="text-danger" role="alert">{e}</p></>;\n}\n') == ["Row"]
    assert neutral_in('function Plain() {\n  return <p className="text-muted">No choice here</p>;\n}\n') == []
    assert neutral_in('// <Checkbox /> in a comment\nfunction Plain() {\n  return <p />;\n}\n') == []
    two = 'function A() {\n  return <Segmented options={o} />;\n}\nexport const B = () => <Radio checked />;\nfunction C() {\n  return <Picker choice options={[{ value: "a", label: "A", icon: IconSun }]} />;\n}\n'
    assert neutral_in(two) == ["A", "B"], neutral_in(two)
    table = 'const JOIN = [\n  { value: "a", label: "A", icon: IconSun },\n];\n\nfunction J() {\n  return <Picker choice options={JOIN} />;\n}\n'
    assert neutral_in(table) == [], neutral_in(table)
    assert neutral_in(table.replace("icon: IconSun", "hint: \"x\"")) == ["J"]
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        src = root / "ui" / "src"
        for d in ("screens/settings", "components/controls", "components", "screens/other"):
            (src / d).mkdir(parents=True, exist_ok=True)
        (src / "screens/settings/Pane.tsx").write_text('import { Card } from "../../components/CleanCard";\n' + bare)
        (src / "components/CleanCard.tsx").write_text(bare.replace("Bare", "Clean"))
        (src / "components/NotImported.tsx").write_text(bare)
        (src / "components/AddThingSheet.tsx").write_text(bare.replace("Bare", "Sheet"))
        (src / "components/controls/Kit.tsx").write_text(bare)
        (src / "screens/other/x.test.tsx").write_text(bare)
        found = scan(root)
        assert found == {
            "ui/src/components/AddThingSheet.tsx": ["Sheet"],
            "ui/src/components/CleanCard.tsx": ["Clean"],
            "ui/src/screens/settings/Pane.tsx": ["Bare"],
        }, found
        assert len(problems(found, {})) == 3
        allow = {"ui/src/components/AddThingSheet.tsx": 1, "ui/src/components/CleanCard.tsx": 1, "ui/src/screens/settings/Pane.tsx": 1}
        assert problems(found, allow) == []
        (src / "components/CleanCard.tsx").write_text(bare.replace("Bare", "Clean").replace("<span", "<IconBroom /><span"))
        assert any("lower its line" in p for p in problems(scan(root), allow))
    print("rich-mode selftest: OK")
    return 0


def main(argv: list[str]) -> int:
    if "--selftest" in argv:
        return selftest()
    root = Path(argv[argv.index("--root") + 1]).resolve() if "--root" in argv else ROOT
    found = scan(root)
    if "--print" in argv:
        for file, names in sorted(found.items()):
            print(f"{file} {len(names)}  # {', '.join(names)}")
        return 0
    issues = problems(found, allowed(ROOT / ALLOW_NAME))
    for issue in issues:
        print(f"rich-mode: {issue}")
    if issues:
        return 1
    print(f"rich-mode: OK ({sum(len(v) for v in found.values())} allowed, the list only shrinks)")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
