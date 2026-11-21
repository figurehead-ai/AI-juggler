#!/usr/bin/env python3
"""A 64-bit integer reaches the window only when it stays exact there.

The window reads every JSON number as JavaScript does: an f64, exact only
below 2^53. A u64/i64/u128/i128/usize/isize that can pass that (a hash, a
random id, nanoseconds, a wrapping sum) loses its last bits on the way,
silently. AJ-E1612 (045c97acb): the notices' version, a wrapping sum plus a
64-bit hash, read the same in the window after a need ended, and the rail
went stale.

So every such value that crosses typed as a number is listed in
`scripts/js_number.allow` with why it stays below 2^53 ("ms epoch, < 2^53
until year 287396"). What is scanned:

- the fields of every ts-rs type (`derive(TS)`, `ts_rs::TS`, also under
  `cfg_attr`): a 64-bit field is skipped only when it isn't sent
  (`ts(skip)`, `serde(skip…)`) or its TypeScript type has no number in it
  (`ts(type = "string")`, `ts(as = "String")`);
- the arguments and results of every `#[tauri::command]` (the command
  map, `commands.ts`, types them as numbers);
- the generated TypeScript: no `bigint` (JSON never yields one; a 64-bit
  field without `ts(type = …)` comes out so).

A value bounded by code rather than by what it is (a hash masked to 53
bits) says so in its entry, `[bound: <file>::<fn>]`, and that function
must keep using a mask defined as `(1 << 53) - 1`: dropping the mask
fails here. An entry left over once its field is gone fails too.

Event payloads are ts-rs types, so they are covered; a raw integer sent
by `emit` or a hand-built `json!` is not seen (none carry one past 2^53
today: AJ-E1612 inventory).

Usage: js_number_lint.py [--selftest]
"""
from __future__ import annotations

import re
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ALLOW = ROOT / "scripts" / "js_number.allow"
GENERATED = "ui/src/types/generated"

WIDE = ["u64", "i64", "u128", "i128", "usize", "isize",
        "NonZeroU64", "NonZeroI64", "NonZeroUsize", "NonZeroIsize"]
DERIVE_TS = re.compile(r"derive\([^)]*\bTS\b")
ITEM = re.compile(r"\b(struct|enum)\s+([A-Za-z_]\w*)")
TS_TYPE = re.compile(r'\bts\(\s*(?:type|as)\s*=\s*"([^"]*)"')
SKIPPED = re.compile(r"\bts\(\s*skip\s*\)|\bserde\([^)]*\bskip(?:_serializing)?\b(?!_)")
COMMAND = re.compile(r"#\[tauri::command[^\]]*\]")
INJECTED = ("State", "AppHandle", "Window", "WebviewWindow", "Webview",
            "Request", "CommandScope", "GlobalScope")
BOUND = re.compile(r"\[bound:\s*([^\]:]+)::(\w+)\s*\]")
MASK_CONST = re.compile(r"const\s+(\w+)\s*:\s*\w+\s*=\s*\(\s*1(?:u64|i64)?\s*<<\s*53\s*\)\s*-\s*1\s*;")
ALIAS = re.compile(r"\btype\s+(\w+)\s*=\s*(\w+)\s*;")


def strip_comments(text: str) -> str:
    """`text` with `//` and `/* */` comments blanked, strings kept."""
    out, i, n = [], 0, len(text)
    while i < n:
        c = text[i]
        if c == '"':
            j = i + 1
            while j < n and text[j] != '"':
                j += 2 if text[j] == "\\" else 1
            out.append(text[i:j + 1])
            i = j + 1
        elif text.startswith("//", i):
            j = text.find("\n", i)
            j = n if j < 0 else j
            out.append(" " * (j - i))
            i = j
        elif text.startswith("/*", i):
            j = text.find("*/", i + 2)
            j = n if j < 0 else j + 2
            out.append(re.sub(r"[^\n]", " ", text[i:j]))
            i = j
        else:
            out.append(c)
            i += 1
    return "".join(out)


def closing(text: str, i: int) -> int:
    """The index just past the bracket closing the one at `text[i]`."""
    pairs = {"(": ")", "[": "]", "{": "}"}
    stack, j = [], i
    while j < len(text):
        c = text[j]
        if c == '"':
            j += 1
            while j < len(text) and text[j] != '"':
                j += 2 if text[j] == "\\" else 1
        elif c in pairs:
            stack.append(pairs[c])
        elif stack and c == stack[-1]:
            stack.pop()
            if not stack:
                return j + 1
        j += 1
    return len(text)


def split_top(body: str) -> list[str]:
    """`body` split at its top-level commas (brackets and `<…>` kept)."""
    parts, depth, start, i = [], 0, 0, 0
    while i < len(body):
        c = body[i]
        if c == '"':
            i += 1
            while i < len(body) and body[i] != '"':
                i += 2 if body[i] == "\\" else 1
        elif c in "([{<":
            depth += 1
        elif c in ")]}" or (c == ">" and body[i - 1] != "-"):
            depth -= 1
        elif c == "," and depth == 0:
            parts.append(body[start:i])
            start = i + 1
        i += 1
    parts.append(body[start:])
    return [p for p in parts if p.strip()]


def attrs_and_rest(part: str) -> tuple[str, str]:
    """A field's attributes (joined) and the declaration after them."""
    attrs, s = [], part.strip()
    while s.startswith("#"):
        end = closing(s, s.index("["))
        attrs.append(s[:end])
        s = s[end:].strip()
    return " ".join(attrs), s


def wide_in(rust_type: str, wide: list[str]) -> bool:
    return any(re.search(rf"\b{w}\b", rust_type) for w in wide)


def sends_number(attrs: str) -> bool:
    """Whether a field with these attributes is sent as a TS number."""
    if SKIPPED.search(attrs):
        return False
    override = TS_TYPE.search(attrs)
    if override:
        ts = override.group(1)
        return "number" in ts or "bigint" in ts
    return True


def fields(body: str, tuple_like: bool) -> list[tuple[str, str, str]]:
    """`(name, attrs, type)` per field of a struct body."""
    out = []
    for index, part in enumerate(split_top(body)):
        attrs, decl = attrs_and_rest(part)
        decl = re.sub(r"^pub(\s*\([^)]*\))?\s+", "", decl)
        if tuple_like:
            out.append((str(index), attrs, decl))
        elif ":" in decl:
            name, rust_type = decl.split(":", 1)
            out.append((name.strip(), attrs, rust_type.strip()))
    return out


def ts_findings(text: str, wide: list[str]) -> list[tuple[str, str]]:
    """`(Type.field, rust type)` of each wide field a ts-rs type sends."""
    text = strip_comments(text)
    found = []
    for derive in DERIVE_TS.finditer(text):
        item = ITEM.search(text, derive.end())
        if not item:
            continue
        kind, name = item.groups()
        rest = item.end()
        while rest < len(text) and text[rest] not in "{(;":
            rest += 1
        if rest >= len(text) or text[rest] == ";":
            continue
        end = closing(text, rest)
        body = text[rest + 1:end - 1]
        if kind == "struct":
            members = [(name, fields(body, text[rest] == "("))]
        else:
            members = []
            for variant in split_top(body):
                _, decl = attrs_and_rest(variant)
                head = re.match(r"(\w+)\s*([({])?", decl)
                if not head or not head.group(2):
                    continue
                at = decl.index(head.group(2))
                inner = decl[at + 1:closing(decl, at) - 1]
                members.append((f"{name}::{head.group(1)}", fields(inner, head.group(2) == "(")))
        for owner, items in members:
            for field, attrs, rust_type in items:
                if wide_in(rust_type, wide) and sends_number(attrs):
                    found.append((f"{owner}.{field}", rust_type))
    return found


def command_findings(text: str, wide: list[str]) -> list[tuple[str, str]]:
    """`(command name(arg) | name->, rust type)` for wide integers."""
    text = strip_comments(text)
    found = []
    for mark in COMMAND.finditer(text):
        fn = re.compile(r"\bfn\s+(\w+)\s*(?:<[^(]*>)?\s*\(").search(text, mark.end())
        if not fn:
            continue
        name = fn.group(1)
        args_end = closing(text, fn.end() - 1)
        for part in split_top(text[fn.end():args_end - 1]):
            _, decl = attrs_and_rest(part)
            if ":" not in decl:
                continue
            arg, rust_type = (s.strip() for s in decl.split(":", 1))
            if any(re.search(rf"\b{t}\b", rust_type) for t in INJECTED):
                continue
            if wide_in(rust_type, wide):
                found.append((f"command {name}({arg})", rust_type))
        body = text.find("{", args_end)
        result = text[args_end:body]
        result = result.split("where", 1)[0]
        if "->" in result and wide_in(result, wide):
            found.append((f"command {name}->", result.split("->", 1)[1].strip()))
    return found


def aliases(texts: list[str]) -> list[str]:
    """`WIDE` plus every plain `type X = <wide>;` alias."""
    names = list(WIDE)
    for text in texts:
        for alias, target in ALIAS.findall(text):
            if target in WIDE and alias not in names:
                names.append(alias)
    return names


def fn_body(text: str, name: str) -> str | None:
    match = re.search(rf"\bfn\s+{name}\b[^{{;]*\{{", text)
    if not match:
        return None
    return text[match.end() - 1:closing(text, match.end() - 1)]


def bound_problem(root: Path, rel: str, fn: str) -> str | None:
    """Why `rel::fn` no longer bounds its value below 2^53, if it doesn't."""
    path = root / rel
    if not path.is_file():
        return f"{rel} is gone"
    text = strip_comments(path.read_text(encoding="utf-8"))
    body = fn_body(text, fn)
    if body is None:
        return f"{rel} has no fn {fn}"
    masks = MASK_CONST.findall(text)
    if not any(re.search(rf"&\s*{m}\b", body) for m in masks):
        return f"{rel}::{fn} no longer masks with a `(1 << 53) - 1` const"
    return None


def load(path: Path) -> tuple[dict[str, str], list[str]]:
    entries, problems = {}, []
    if not path.exists():
        return entries, problems
    for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        key, _, reason = line.partition("#")
        key, reason = key.strip(), reason.strip()
        if not reason:
            problems.append(f"{path.name}:{number}: {key} gives no reason")
        entries[key] = reason
    return entries, problems


def sources(root: Path) -> list[Path]:
    return [p for p in sorted((root / "crates").rglob("*.rs"))
            if "/target/" not in p.as_posix()]


def check(root: Path, allow: Path) -> list[str]:
    files = [(p, p.read_text(encoding="utf-8", errors="replace")) for p in sources(root)]
    wide = aliases([t for _, t in files])
    seen: dict[str, str] = {}
    for path, text in files:
        rel = path.relative_to(root).as_posix()
        for key, rust_type in ts_findings(text, wide) + command_findings(text, wide):
            seen.setdefault(key, f"{rel} ({rust_type})")
    entries, problems = load(allow)
    for key, where in sorted(seen.items()):
        if key not in entries:
            problems.append(
                f"{key}: {where} reaches the window as a JS number (exact only below 2^53); "
                f"keep it below 2^53 and list it in scripts/js_number.allow with why, "
                f'or send it as a string (ts(type = "string"))')
    for key, reason in sorted(entries.items()):
        if key not in seen:
            problems.append(f"{key}: listed in scripts/js_number.allow but not found; drop its line")
        for rel, fn in BOUND.findall(reason):
            why = bound_problem(root, rel.strip(), fn)
            if why:
                problems.append(f"{key}: {why}")
    generated = root / GENERATED
    if generated.is_dir():
        for path in sorted(generated.glob("*.ts")):
            for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
                if re.search(r"\bbigint\b", line):
                    problems.append(
                        f"{path.relative_to(root)}:{number}: bigint (JSON gives a number): "
                        f'give the Rust field ts(type = "number") and list it, or ts(type = "string")')
    return problems


OLD_045C97ACB = '''
const WINDOW_EXACT: u64 = (1 << 53) - 1;
fn version_of(storage: &Storage, now: i64) -> i64 {
    latest.wrapping_mul(31).wrapping_add(earlier).wrapping_add(held)
}
'''
NEW_045C97ACB = '''
const WINDOW_EXACT: u64 = (1 << 53) - 1;
fn version_of(storage: &Storage, now: i64) -> i64 {
    i64::try_from(hasher.finish() & WINDOW_EXACT).unwrap_or(0)
}
'''
SUMMARY = '''
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct NoticeSummaryInfo {
    /// Unread, <b>all</b> of them, {in} (a, b).
    #[ts(type = "number")]
    pub version: i64,
    pub count: u32,
}
'''


def selftest() -> None:
    wide = list(WIDE)
    assert ts_findings(SUMMARY, wide) == [("NoticeSummaryInfo.version", "i64")]
    assert ts_findings(SUMMARY.replace('"number"', '"string"'), wide) == []
    assert ts_findings(SUMMARY.replace('#[ts(type = "number")]', "#[serde(skip)]"), wide) == []
    assert ts_findings(SUMMARY.replace("TS)]", "Serialize)]"), wide) == []
    core = '''#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Row { #[cfg_attr(feature = "ts", ts(type = "number | null"))] pub at: Option<i64>,
  pub pids: Vec<usize>, pub map: HashMap<String, (u8, u64)>, }'''
    assert [k for k, _ in ts_findings(core, wide)] == ["Row.at", "Row.pids", "Row.map"]
    enum = '''#[derive(TS)] pub enum Ev { A, B { at: u64, n: u32 }, C(i64, String) }'''
    assert [k for k, _ in ts_findings(enum, wide)] == ["Ev::B.at", "Ev::C.0"]
    assert [k for k, _ in ts_findings("#[derive(TS)] pub struct Id(pub u64);", wide)] == ["Id.0"]
    command = '''#[tauri::command]
pub async fn notifications_read(state: tauri::State<'_, Arc<AppState>>, id: Option<i64>, n: u32)
    -> AppResult<u64> where X: Y { body() }'''
    assert command_findings(command, wide) == [
        ("command notifications_read(id)", "Option<i64>"),
        ("command notifications_read->", "AppResult<u64>")]
    assert aliases(["pub type Millis = i64;"])[-1] == "Millis"
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        src = root / "crates/app/src"
        src.mkdir(parents=True)
        (src / "dto.rs").write_text(SUMMARY)
        (src / "centre.rs").write_text(NEW_045C97ACB)
        allow = root / "js_number.allow"
        # A new 64-bit field typed as a number, not listed: fails.
        allow.write_text("")
        assert any("NoticeSummaryInfo.version" in p for p in check(root, allow))
        # Listed without a reason: fails.
        allow.write_text("NoticeSummaryInfo.version\n")
        assert any("gives no reason" in p for p in check(root, allow))
        bounded = ("NoticeSummaryInfo.version  # a hash masked to 53 bits "
                   "[bound: crates/app/src/centre.rs::version_of]\n")
        allow.write_text(bounded)
        assert check(root, allow) == [], check(root, allow)
        # 045c97acb's old code: the version a wrapping sum, no mask: fails.
        (src / "centre.rs").write_text(OLD_045C97ACB)
        assert any("no longer masks" in p for p in check(root, allow))
        (src / "centre.rs").write_text(NEW_045C97ACB)
        # A stale entry fails; so does a bigint in the generated types.
        allow.write_text(bounded + "Gone.field  # ms epoch\n")
        assert any("Gone.field" in p for p in check(root, allow))
        allow.write_text(bounded)
        gen = root / GENERATED
        gen.mkdir(parents=True)
        (gen / "A.ts").write_text("export type A = { at: bigint | null, };\n")
        assert any("bigint" in p for p in check(root, allow))
    print("js_number_lint selftest: ok")


def main() -> int:
    if "--selftest" in sys.argv:
        selftest()
        return 0
    problems = check(ROOT, ALLOW)
    for problem in problems:
        print(problem)
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
