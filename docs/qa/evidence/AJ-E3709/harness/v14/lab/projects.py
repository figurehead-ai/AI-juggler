#!/usr/bin/env python3
"""AJ-E3831 harness v14: the lab's throwaway projects (made up, no real
names), each a small stdlib-only Python package with unittest tests and a
few real gaps and bugs for a lab agent to work on. `record.py` copies one
into a fresh folder per scenario. Nothing here runs a model.

    python3 projects.py --list
    python3 projects.py --make <project> <dir>
"""
from __future__ import annotations

import sys
from pathlib import Path

P: dict[str, dict[str, str]] = {}

P["tidepool-ledger"] = {
    "README.md": "# tidepool-ledger\n\nA tiny bookkeeping library: entries, balances, CSV import and a monthly report.\n\nRun the tests: `python3 -m unittest discover -s tests`\n",
    "tidepool/__init__.py": "",
    "tidepool/ledger.py": '''from dataclasses import dataclass
from datetime import date


@dataclass
class Entry:
    day: date
    account: str
    amount_cents: int
    memo: str = ""


class Ledger:
    def __init__(self):
        self.entries = []

    def add(self, entry):
        self.entries.append(entry)

    def balance(self, account):
        return sum(e.amount_cents for e in self.entries if e.account == account)

    def accounts(self):
        return sorted({e.account for e in self.entries})

    def between(self, start, end):
        # BUG: end is exclusive here, the docs say inclusive
        return [e for e in self.entries if start <= e.day < end]
''',
    "tidepool/csv_import.py": '''import csv
from datetime import date

from .ledger import Entry


def parse_amount(text):
    text = text.strip().replace(",", "")
    if "." in text:
        whole, frac = text.split(".")
        return int(whole) * 100 + int(frac.ljust(2, "0")[:2]) * (1 if not whole.startswith("-") else -1)
    return int(text) * 100


def read_entries(path):
    out = []
    with open(path, newline="") as f:
        for row in csv.DictReader(f):
            y, m, d = row["date"].split("-")
            out.append(Entry(date(int(y), int(m), int(d)), row["account"], parse_amount(row["amount"]), row.get("memo", "")))
    return out
''',
    "tidepool/report.py": '''def monthly_totals(ledger, year):
    totals = {}
    for e in ledger.entries:
        if e.day.year != year:
            continue
        key = e.day.month
        totals[key] = totals.get(key, 0) + e.amount_cents
    return totals


def format_cents(cents):
    return "%d.%02d" % (cents // 100, cents % 100)


def render(ledger, year):
    lines = []
    for month, cents in sorted(monthly_totals(ledger, year).items()):
        lines.append("%02d  %s" % (month, format_cents(cents)))
    return "\\n".join(lines)
''',
    "tests/__init__.py": "",
    "tests/test_ledger.py": '''import unittest
from datetime import date

from tidepool.ledger import Entry, Ledger
from tidepool.csv_import import parse_amount


class LedgerTest(unittest.TestCase):
    def test_balance(self):
        l = Ledger()
        l.add(Entry(date(2026, 1, 3), "cash", 1200))
        l.add(Entry(date(2026, 1, 4), "cash", -200))
        self.assertEqual(l.balance("cash"), 1000)

    def test_parse_amount(self):
        self.assertEqual(parse_amount("12.5"), 1250)
        self.assertEqual(parse_amount("1,200.00"), 120000)


if __name__ == "__main__":
    unittest.main()
''',
    "data/sample.csv": "date,account,amount,memo\n2026-01-03,cash,12.00,coffee beans\n2026-01-04,cash,-2.00,refund\n,,,\n2026-02-01,bank,-1.5,fee\n",
}

P["kestrel-cli"] = {
    "README.md": "# kestrel-cli\n\nA to-do list for the terminal, stored as JSON.\n\n    python3 -m kestrel add \"water the ferns\" --due 2026-12-01\n    python3 -m kestrel list\n\nTests: `python3 -m unittest discover -s tests`\n",
    "kestrel/__init__.py": "",
    "kestrel/__main__.py": "from .cli import main\n\nraise SystemExit(main())\n",
    "kestrel/store.py": '''import json
import os


class Store:
    def __init__(self, path):
        self.path = path

    def load(self):
        if not os.path.exists(self.path):
            return []
        with open(self.path) as f:
            return json.load(f)

    def save(self, items):
        with open(self.path, "w") as f:
            json.dump(items, f)
''',
    "kestrel/cli.py": '''import argparse
import os
from datetime import datetime

from .store import Store

DEFAULT = os.path.expanduser("~/.kestrel.json")


def parse_due(text):
    # BUG: accepts 2026-13-40 silently by swapping fields
    try:
        return datetime.strptime(text, "%Y-%m-%d").date().isoformat()
    except ValueError:
        return datetime.strptime(text, "%Y-%d-%m").date().isoformat()


def main(argv=None, path=None):
    p = argparse.ArgumentParser(prog="kestrel")
    sub = p.add_subparsers(dest="cmd", required=True)
    a = sub.add_parser("add")
    a.add_argument("title")
    a.add_argument("--due")
    sub.add_parser("list")
    d = sub.add_parser("done")
    d.add_argument("index", type=int)
    args = p.parse_args(argv)
    store = Store(path or DEFAULT)
    items = store.load()
    if args.cmd == "add":
        items.append({"title": args.title, "due": parse_due(args.due) if args.due else None, "done": False})
        store.save(items)
    elif args.cmd == "list":
        for i, it in enumerate(items):
            mark = "x" if it["done"] else " "
            print("%d [%s] %s%s" % (i, mark, it["title"], (" (due %s)" % it["due"]) if it["due"] else ""))
    elif args.cmd == "done":
        items[args.index]["done"] = True
        store.save(items)
    return 0
''',
    "tests/__init__.py": "",
    "tests/test_cli.py": '''import io
import os
import tempfile
import unittest
from contextlib import redirect_stdout

from kestrel.cli import main


class CliTest(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.path = os.path.join(self.dir.name, "k.json")

    def tearDown(self):
        self.dir.cleanup()

    def test_add_and_list(self):
        main(["add", "water the ferns"], path=self.path)
        out = io.StringIO()
        with redirect_stdout(out):
            main(["list"], path=self.path)
        self.assertIn("water the ferns", out.getvalue())


if __name__ == "__main__":
    unittest.main()
''',
}

P["marlin-cache"] = {
    "README.md": "# marlin-cache\n\nAn in-memory LRU cache with per-key time to live.\n\nTests: `python3 -m unittest discover -s tests`\nBenchmark: `python3 bench.py`\n",
    "marlin/__init__.py": "from .cache import LRUCache\n",
    "marlin/cache.py": '''import time


class LRUCache:
    def __init__(self, capacity, ttl=None, clock=time.monotonic):
        self.capacity = capacity
        self.ttl = ttl
        self.clock = clock
        self.data = {}
        self.order = []

    def get(self, key, default=None):
        if key not in self.data:
            return default
        value, at = self.data[key]
        if self.ttl is not None and self.clock() - at > self.ttl:
            del self.data[key]
            return default
        self.order.remove(key)
        self.order.append(key)
        return value

    def put(self, key, value):
        if key in self.data:
            self.order.remove(key)
        elif len(self.data) >= self.capacity:
            # BUG: evicts the newest key instead of the least recently used
            oldest = self.order.pop()
            del self.data[oldest]
        self.data[key] = (value, self.clock())
        self.order.append(key)

    def __len__(self):
        return len(self.data)
''',
    "bench.py": '''import time
from marlin import LRUCache

c = LRUCache(1000)
t = time.perf_counter()
for i in range(200000):
    c.put(i % 1500, i)
    c.get((i * 7) % 1500)
print("ops/s: %d" % (400000 / (time.perf_counter() - t)))
''',
    "tests/__init__.py": "",
    "tests/test_cache.py": '''import unittest

from marlin import LRUCache


class CacheTest(unittest.TestCase):
    def test_put_get(self):
        c = LRUCache(2)
        c.put("a", 1)
        self.assertEqual(c.get("a"), 1)

    def test_ttl(self):
        now = [0.0]
        c = LRUCache(2, ttl=5, clock=lambda: now[0])
        c.put("a", 1)
        now[0] = 6
        self.assertIsNone(c.get("a"))


if __name__ == "__main__":
    unittest.main()
''',
}

P["quartz-templates"] = {
    "README.md": "# quartz-templates\n\nA small text template renderer: `{{ name }}` placeholders and `{% for x in xs %}…{% endfor %}` loops.\n\nTests: `python3 -m unittest discover -s tests`\n",
    "quartz/__init__.py": "from .render import render\n",
    "quartz/render.py": '''import re

VAR = re.compile(r"{{\\s*([a-zA-Z_][a-zA-Z0-9_.]*)\\s*}}")
LOOP = re.compile(r"{%\\s*for\\s+(\\w+)\\s+in\\s+(\\w+)\\s*%}(.*?){%\\s*endfor\\s*%}", re.S)


def lookup(ctx, dotted):
    cur = ctx
    for part in dotted.split("."):
        cur = cur[part] if isinstance(cur, dict) else getattr(cur, part)
    return cur


def render(template, ctx):
    def loop(m):
        name, seq, body = m.group(1), m.group(2), m.group(3)
        return "".join(render(body, {**ctx, name: item}) for item in ctx.get(seq, []))

    out = LOOP.sub(loop, template)
    # NOTE: values are inserted as-is (no HTML escaping yet)
    return VAR.sub(lambda m: str(lookup(ctx, m.group(1))), out)
''',
    "tests/__init__.py": "",
    "tests/test_render.py": '''import unittest

from quartz import render


class RenderTest(unittest.TestCase):
    def test_var(self):
        self.assertEqual(render("hi {{ name }}", {"name": "Ada"}), "hi Ada")

    def test_loop(self):
        self.assertEqual(render("{% for x in xs %}[{{ x }}]{% endfor %}", {"xs": [1, 2]}), "[1][2]")


if __name__ == "__main__":
    unittest.main()
''',
}

P["fennel-scheduler"] = {
    "README.md": "# fennel-scheduler\n\nParses five-field cron expressions and finds the next run time.\n\nTests: `python3 -m unittest discover -s tests`\n",
    "fennel/__init__.py": "from .cron import parse, next_run\n",
    "fennel/cron.py": '''from datetime import datetime, timedelta

RANGES = [(0, 59), (0, 23), (1, 31), (1, 12), (0, 6)]


def parse_field(text, lo, hi):
    out = set()
    for part in text.split(","):
        step = 1
        if "/" in part:
            part, step = part.split("/")
            step = int(step)
        if part == "*":
            a, b = lo, hi
        elif "-" in part:
            a, b = map(int, part.split("-"))
        else:
            a = b = int(part)
        # BUG: the range end is left out
        out.update(range(a, b, step))
    return out


def parse(expr):
    fields = expr.split()
    if len(fields) != 5:
        raise ValueError("need five fields")
    return [parse_field(f, lo, hi) for f, (lo, hi) in zip(fields, RANGES)]


def next_run(expr, after):
    minute, hour, dom, month, dow = parse(expr)
    t = after.replace(second=0, microsecond=0) + timedelta(minutes=1)
    for _ in range(366 * 24 * 60):
        if t.minute in minute and t.hour in hour and t.day in dom and t.month in month and t.weekday() in dow:
            return t
        t += timedelta(minutes=1)
    raise ValueError("no run within a year")
''',
    "tests/__init__.py": "",
    "tests/test_cron.py": '''import unittest
from datetime import datetime

from fennel import parse, next_run


class CronTest(unittest.TestCase):
    def test_parse_single(self):
        self.assertEqual(parse("5 * * * *")[0], {5})

    def test_next_hourly(self):
        self.assertEqual(next_run("0 * * * *", datetime(2026, 3, 1, 10, 15)), datetime(2026, 3, 1, 11, 0))


if __name__ == "__main__":
    unittest.main()
''',
}

P["heron-stats"] = {
    "README.md": "# heron-stats\n\nSmall statistics helpers: mean, median, percentile, and a text histogram.\n\nTests: `python3 -m unittest discover -s tests`\n",
    "heron/__init__.py": "from .stats import mean, median, percentile\n",
    "heron/stats.py": '''def mean(xs):
    return sum(xs) / len(xs)


def median(xs):
    s = sorted(xs)
    n = len(s)
    mid = n // 2
    return s[mid] if n % 2 else (s[mid - 1] + s[mid]) / 2


def percentile(xs, p):
    # BUG: nearest-rank without interpolation, and p=100 overflows
    s = sorted(xs)
    k = int(len(s) * p / 100)
    return s[k]
''',
    "heron/hist.py": '''def histogram(xs, bins=5, width=30):
    raise NotImplementedError("histogram is not written yet")
''',
    "tests/__init__.py": "",
    "tests/test_stats.py": '''import unittest

from heron import mean, median, percentile


class StatsTest(unittest.TestCase):
    def test_mean(self):
        self.assertEqual(mean([1, 2, 3]), 2)

    def test_median(self):
        self.assertEqual(median([3, 1, 2, 4]), 2.5)

    def test_percentile_50(self):
        self.assertEqual(percentile([1, 2, 3, 4, 5], 50), 3)


if __name__ == "__main__":
    unittest.main()
''',
}

P["osprey-config"] = {
    "README.md": "# osprey-config\n\nLoads settings from an INI file, then environment variables (`OSPREY_<SECTION>_<KEY>`), then command-line overrides, later ones winning.\n\nTests: `python3 -m unittest discover -s tests`\n",
    "osprey/__init__.py": "from .load import load\n",
    "osprey/load.py": '''import configparser
import os


def load(path, env=None, overrides=None):
    env = os.environ if env is None else env
    cp = configparser.ConfigParser()
    cp.read(path)
    out = {s: dict(cp[s]) for s in cp.sections()}
    for k, v in (overrides or {}).items():
        section, key = k.split(".", 1)
        out.setdefault(section, {})[key] = v
    # BUG: environment applied after overrides, so it wins over them
    for k, v in env.items():
        if k.startswith("OSPREY_"):
            _, section, key = k.split("_", 2)
            out.setdefault(section.lower(), {})[key.lower()] = v
    return out
''',
    "tests/__init__.py": "",
    "tests/test_load.py": '''import os
import tempfile
import unittest

from osprey import load


class LoadTest(unittest.TestCase):
    def test_file(self):
        with tempfile.NamedTemporaryFile("w", suffix=".ini", delete=False) as f:
            f.write("[db]\\nhost = localhost\\n")
        try:
            self.assertEqual(load(f.name, env={})["db"]["host"], "localhost")
        finally:
            os.unlink(f.name)


if __name__ == "__main__":
    unittest.main()
''',
}

P["wren-markdown"] = {
    "README.md": "# wren-markdown\n\nA tiny Markdown-to-HTML converter: headings, paragraphs, emphasis, inline code.\n\nTests: `python3 -m unittest discover -s tests`\n",
    "wren/__init__.py": "from .convert import to_html\n",
    "wren/convert.py": '''import html
import re


def inline(text):
    text = html.escape(text)
    text = re.sub(r"`([^`]+)`", r"<code>\\1</code>", text)
    text = re.sub(r"\\*\\*([^*]+)\\*\\*", r"<strong>\\1</strong>", text)
    text = re.sub(r"\\*([^*]+)\\*", r"<em>\\1</em>", text)
    return text


def to_html(src):
    out = []
    para = []
    for line in src.splitlines():
        if line.startswith("#"):
            level = len(line) - len(line.lstrip("#"))
            out.append("<h%d>%s</h%d>" % (level, inline(line[level:].strip()), level))
        elif not line.strip():
            if para:
                out.append("<p>%s</p>" % inline(" ".join(para)))
                para = []
        else:
            para.append(line.strip())
    if para:
        out.append("<p>%s</p>" % inline(" ".join(para)))
    return "\\n".join(out)
''',
    "tests/__init__.py": "",
    "tests/test_convert.py": '''import unittest

from wren import to_html


class ConvertTest(unittest.TestCase):
    def test_heading(self):
        self.assertEqual(to_html("# Hi"), "<h1>Hi</h1>")

    def test_para_emphasis(self):
        self.assertEqual(to_html("a *b* c"), "<p>a <em>b</em> c</p>")


if __name__ == "__main__":
    unittest.main()
''',
}


def make(name: str, dest: Path, plant: dict[str, str] | None = None) -> None:
    for rel, text in {**P[name], **(plant or {})}.items():
        f = dest / rel
        f.parent.mkdir(parents=True, exist_ok=True)
        f.write_text(text)


if __name__ == "__main__":
    if sys.argv[1:2] == ["--list"]:
        print("\n".join(P))
    elif sys.argv[1:2] == ["--make"] and len(sys.argv) == 4:
        make(sys.argv[2], Path(sys.argv[3]))
    else:
        print(__doc__)
        sys.exit(2)
