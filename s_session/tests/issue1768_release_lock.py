"""#1768 发布旧写入口负控，合法输入和原八帽锚必须到达退休边界。"""
import json
import os
import sqlite3
import subprocess
import sys
import tempfile
from contextlib import closing
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "s_session"))
from r4_anchor_fixture import TestAnchor
from tb01c_query_tests import QueryTests

REJECTION = "LegacyAdmissionBudgetAdapterMissing：完整旧域审计已调用，但旧证据图节点与Snapshot/Delta尚无Job同源费用适配，禁止签发准入"


SIDECARS = ("-wal", "-shm", "-journal")


def open_frozen(path):
    """冻结原件持久 WAL 头：普通只读打开会后生 -shm/-wal；无侧件即无未检查点帧，才可 immutable。"""
    path = Path(path).resolve()
    present = [s for s in SIDECARS if Path(f"{path}{s}").exists()]
    if present:
        raise RuntimeError(f"frozen 原件带侧件 {present}，非已证完整检查点副本，拒绝 immutable 读取：{path}")
    conn = sqlite3.connect(path.as_uri() + "?immutable=1", uri=True)
    conn.execute("PRAGMA query_only=ON")
    return conn


def logical_state(db):
    with closing(sqlite3.connect(db)) as conn:
        return list(conn.iterdump())


def main(binary, mode):
    binary, mode = Path(binary).resolve(), Path(mode).resolve()
    results = []
    with tempfile.TemporaryDirectory() as tmp:
        base = Path(tmp)
        initialized = base / "initialized.sqlite"
        init = subprocess.run([str(binary), "init", "--db", str(initialized), "--session", "tb01c-query-unit", "--catalog", str(ROOT / "s_session/catalog/signed-catalog.json")], capture_output=True, text=True, timeout=60)
        print(json.dumps(dict(init_exit=init.returncode, stdout=init.stdout, stderr=init.stderr)), flush=True)
        assert init.returncode == 0 and initialized.is_file(), init
        QueryTests.base = base
        event = QueryTests.input(4, 11500)
        for operation, seed in (("accept", "g4"), ("advance", "g4-pending"), ("recover", "g4-pending")):
            db = base / f"{operation}.sqlite"
            with closing(open_frozen(mode / 'frozen' / (seed + '.sqlite'))) as src, closing(sqlite3.connect(db)) as dst:
                src.backup(dst)
            state = base / f"{operation}-state"
            state.mkdir()
            anchor = TestAnchor(db, state)
            before = logical_state(db)
            args = [str(binary), operation, "--db", str(db), "--admission-job-limits", str(anchor.limits_path)]
            if operation == "accept":
                args += ["--input", str(event), "--profile", str(ROOT / "s_session/profiles/testonly_tick_1_1_ohlc.json")]
            result = subprocess.run(args, capture_output=True, text=True, timeout=60, env=dict(os.environ, **anchor.env()))
            entry = dict(operation=operation, argv=args, exit=result.returncode, stdout=result.stdout, stderr=result.stderr)
            results.append(entry)
            print(json.dumps(entry, ensure_ascii=False), flush=True)
            assert result.returncode != 0, operation
            terminal = json.loads(result.stderr)
            assert terminal["error"].startswith(REJECTION), terminal
            assert terminal["admission_accounting"]["job_present"] is True, terminal
            assert logical_state(db) == before, f"{operation} modified business state"
        seed_result = subprocess.run([str(binary), "seed", "--db", str(base / "forbidden.sqlite")], capture_output=True, text=True, timeout=60)
        assert seed_result.returncode != 0 and not (base / "forbidden.sqlite").exists(), seed_result
        print(json.dumps(dict(seed_exit=seed_result.returncode, seed_stderr=seed_result.stderr, ok=True), ensure_ascii=False))
    return 0


if __name__ == "__main__":
    sys.exit(main(*sys.argv[1:]))
