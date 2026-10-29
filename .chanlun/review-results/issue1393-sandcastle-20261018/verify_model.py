"""#1393 独立纯模型/toy 账验证。没有导入或执行 Rust/Lean/产品；不替代正式验收。"""
from __future__ import annotations

import copy
import hashlib
import itertools
import json
from pathlib import Path
import sqlite3
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[3]
ORACLE = ROOT / "rust/tests/fixtures/tb02u/oracle.json"
POLICY = "raw-initialization-w/1"


def digest(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True, separators=(",", ":")).encode()).hexdigest()


def pair_direction(a, b):
    """严格同向的两个端点是独立数学判据；等号与交叉均无方向。"""
    changes = (b[0] - a[0], b[1] - a[1])
    if min(changes) > 0:
        return "UP"
    if max(changes) < 0:
        return "DOWN"
    return None


def reference(raw):
    """从完整原件求首对，再直接计算手算账的组；不调用流模型。"""
    evidence = next(([i, i + 1, direction] for i in range(len(raw) - 1)
                     if (direction := pair_direction(raw[i], raw[i + 1]))), None)
    if evidence is None:
        return {"pair": None, "groups": [], "pending": list(range(len(raw))), "direction": None}
    direction = evidence[2]
    groups = [[0, *raw[0], [0]]]
    for i, interval in enumerate(raw[1:], 1):
        acc = groups[-1]
        current = pair_direction(acc[1:3], interval)
        if current:
            direction = current
            groups.append([i, *interval, [i]])
        else:
            choose = max if direction == "UP" else min
            acc[1:3] = [choose(acc[1], interval[0]), choose(acc[2], interval[1])]
            acc[3].append(i)
    for group in groups:
        group.extend([[i for i in group[3] if raw[i][1] == group[2]],
                      [i for i in group[3] if raw[i][0] == group[1]]])
    return {"pair": evidence, "groups": groups, "pending": [], "direction": direction}


class Stream:
    """用于对拍/断点序列化的另一纯模型，显式保持 pending、acc 与方向。"""
    def __init__(self):
        self.raw = []
        self.pending = []
        self.groups = []
        self.direction = None
        self.pair = None

    def append(self, interval):
        i = len(self.raw)
        self.raw.append(list(interval))
        if self.direction is None:
            self.pending.append(i)
            if i == 0:
                return
            low, high = self.raw[i - 1]
            new_low, new_high = interval
            if low >= new_low and high <= new_high or new_low >= low and new_high <= high:
                return
            self.direction = "UP" if new_high > high else "DOWN"
            self.pair = [i - 1, i, self.direction]
            self.pending = []
            for index in range(i + 1):
                self._fold(index)
        else:
            self._fold(i)

    def _fold(self, i):
        low, high = self.raw[i]
        if not self.groups:
            self.groups.append([i, low, high, [i], [i], [i]])
            return
        group = self.groups[-1]
        old_low, old_high = group[1:3]
        included = low <= old_low and high >= old_high or old_low <= low and old_high >= high
        if not included:
            self.direction = "UP" if high > old_high else "DOWN"
            self.groups.append([i, low, high, [i], [i], [i]])
            return
        if self.direction == "UP":
            low, high = max(low, old_low), max(high, old_high)
        else:
            low, high = min(low, old_low), min(high, old_high)
        group[1:3] = low, high
        group[3].append(i)
        group[4] = [r for r in group[3] if self.raw[r][1] == high]
        group[5] = [r for r in group[3] if self.raw[r][0] == low]

    def view(self):
        return {"pair": self.pair, "groups": self.groups, "pending": self.pending, "direction": self.direction}

    def checkpoint(self):
        return json.dumps(self.__dict__)

    @classmethod
    def restore(cls, wire):
        state = cls()
        state.__dict__.update(json.loads(wire))
        return state


class Ledger:
    """规格级 toy SQLite 历史账；不是产品 schema，也不声称验证产品 SQL。"""
    def __init__(self, path):
        self.db = sqlite3.connect(path)
        self.db.executescript("""
            CREATE TABLE IF NOT EXISTS objects(
                id TEXT PRIMARY KEY, kind TEXT, coord INTEGER, payload TEXT,
                refs TEXT, born INTEGER, dead INTEGER, successor TEXT);
            CREATE TABLE IF NOT EXISTS raw_history(
                coord INTEGER, revision INTEGER, payload TEXT, PRIMARY KEY(coord,revision));
        """)

    def publish(self, cut, raw, revisions):
        result = reference(raw)
        active = {row[0]: row for row in self.db.execute("SELECT * FROM objects WHERE dead IS NULL")}
        wanted = []
        for coord, (interval, revision) in enumerate(zip(raw, revisions)):
            self.db.execute("INSERT OR IGNORE INTO raw_history VALUES(?,?,?)", (coord, revision, json.dumps(interval)))
        for coord in result["pending"]:
            payload = json.dumps({"raw": raw[coord], "direction_state": "Undetermined", "policy": POLICY})
            refs = json.dumps([coord, revisions[coord]])
            old = next((r for r in active.values() if r[1:5] == ("pending", coord, payload, refs)), None)
            identity = old[0] if old else digest(["pending", coord, payload, refs, cut])
            wanted.append((identity, "pending", coord, payload, refs, old[5] if old else cut))
        for group in result["groups"]:
            refs = json.dumps(list(enumerate(revisions)))
            payload = json.dumps({"group": group, "policy": POLICY, "pair": result["pair"]})
            wanted.append((digest(["group", payload, refs]), "group", group[0], payload, refs, cut))
        ids = {r[0] for r in wanted}
        for old in active.values():
            if old[0] in ids:
                continue
            successor = next((r[0] for r in wanted if r[1] == "group" and old[1] == "pending"
                              and old[2] in json.loads(r[3])["group"][3]), None)
            self.db.execute("UPDATE objects SET dead=?,successor=? WHERE id=?", (cut, successor, old[0]))
        for row in wanted:
            self.db.execute("INSERT OR IGNORE INTO objects VALUES(?,?,?,?,?,?,NULL,NULL)", row)
        self.db.commit()

    def as_known(self, cut):
        return list(self.db.execute("SELECT id,kind,coord,payload,refs,born FROM objects WHERE born<=? AND (dead IS NULL OR dead>?) ORDER BY kind,coord", (cut, cut)))


def replay():
    history = json.load(sys.stdin)
    with tempfile.TemporaryDirectory() as directory:
        ledger = Ledger(str(Path(directory) / "history.sqlite"))
        snapshots = []
        for i, entry in enumerate(history, 1):
            ledger.publish(i, entry["raw"], entry["revisions"])
            snapshots.append(ledger.as_known(i))
        ledger.db.close()
    print(json.dumps(snapshots))


class ModelTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.oracle = json.loads(ORACLE.read_text())

    def test_handwritten_oracle(self):
        for case in self.oracle["cases"]:
            with self.subTest(case=case["id"]):
                actual = reference(case["raw"])
                self.assertEqual(actual["pair"], case["pair"])
                self.assertEqual(actual["groups"], case["groups"])
        for case in self.oracle["history"]:
            actual = reference(case["raw"])
            self.assertEqual(actual["groups"], case["groups"])
            self.assertEqual(actual["pending"], case["pending"])

    def test_exhaustive_prefix_checkpoint_and_dual(self):
        intervals = [[low, high] for low in range(3) for high in range(low, 3)]
        count = 0
        for size in range(5):
            for raw in itertools.product(intervals, repeat=size):
                count += 1
                stream = Stream()
                for index, interval in enumerate(raw):
                    stream = Stream.restore(stream.checkpoint())
                    stream.append(interval)
                    self.assertEqual(stream.view(), reference(raw[:index + 1]))
                self.assertEqual(stream.view(), reference(raw))
                dual = [[-high, -low] for low, high in raw]
                mirrored = reference(dual)
                expected = copy.deepcopy(reference(raw))
                if expected["pair"]:
                    expected["pair"][2] = "DOWN" if expected["pair"][2] == "UP" else "UP"
                    expected["direction"] = "DOWN" if expected["direction"] == "UP" else "UP"
                for group in expected["groups"]:
                    group[1:3] = [-group[2], -group[1]]
                    group[4:6] = [group[5], group[4]]
                self.assertEqual(mirrored, expected)
        self.assertEqual(count, 1555)

    def test_history_replaces_old_cut_and_reopen(self):
        with tempfile.TemporaryDirectory() as directory:
            path = str(Path(directory) / "cuts.sqlite")
            ledger = Ledger(path)
            snapshots = []
            for cut, entry in enumerate(self.oracle["history"], 1):
                ledger.publish(cut, entry["raw"], entry["revisions"])
                current = ledger.as_known(cut)
                for old_cut, snapshot in enumerate(snapshots, 1):
                    self.assertEqual(ledger.as_known(old_cut), snapshot)
                if cut in (2, 4):
                    replacements = list(ledger.db.execute("SELECT coord,successor FROM objects WHERE kind='pending' AND dead=?", (cut,)))
                    projected = [[coord, next(r[2] for r in current if r[0] == successor)] for coord, successor in replacements]
                    self.assertEqual(sorted(projected), entry["replaces_pending"])
                snapshots.append(current)
                ledger.db.close()
                ledger = Ledger(path)
                self.assertEqual(ledger.as_known(cut), current)
            self.assertEqual(ledger.db.execute("SELECT COUNT(*) FROM raw_history").fetchone()[0], 5)
            self.assertTrue({r[0] for r in snapshots[0]}.isdisjoint(r[0] for r in snapshots[2]))
            self.assertTrue({r[0] for r in snapshots[1]}.isdisjoint(r[0] for r in snapshots[3]))
            ledger.db.close()
        child = subprocess.run([sys.executable, str(Path(__file__)), "--replay"],
                               input=json.dumps(self.oracle["history"]), capture_output=True, text=True, check=True)
        self.assertEqual(json.loads(child.stdout), json.loads(json.dumps(snapshots)))

    def test_pending_identity_retains_first_known(self):
        with tempfile.TemporaryDirectory() as directory:
            ledger = Ledger(str(Path(directory) / "pending.sqlite"))
            first = None
            for cut in range(1, 4):
                ledger.publish(cut, [[0, 10], [2, 8], [1, 11]][:cut], [1] * cut)
                raw0 = next(r for r in ledger.as_known(cut) if r[2] == 0)
                first = first or raw0
                self.assertEqual(raw0, first)
            ledger.db.close()


if __name__ == "__main__":
    if "--replay" in sys.argv:
        replay()
    else:
        unittest.main(verbosity=2)
