#!/usr/bin/env python3
"""#1371：由正式 writer 造库，核 Python/Rust 对未来索引的同义拒绝及失败零写入。"""

import argparse
from contextlib import closing
import importlib.util
import json
import os
from pathlib import Path
import sqlite3
import subprocess


ROOT = Path(__file__).resolve().parents[2]
PROFILE = ROOT / "s_session/profiles/testonly_tick_1_1_ohlc.json"
CATALOG = ROOT / "s_session/catalog/signed-catalog.json"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--reader", type=Path, default=ROOT / "s_session/s_readonly_server.py")
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    calls = []
    cases = []
    reads = []
    spec = importlib.util.spec_from_file_location("r13_readonly", args.reader)
    reader = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(reader)

    def cli(db, command, *options):
        argv = [str(args.binary.resolve()), command, "--db", str(db), *map(str, options)]
        env = dict(os.environ, S_SESSION_PRAGMA_SIDECAR_PATH=str(args.output / "pragma.txt"))
        result = subprocess.run(argv, capture_output=True, text=True, timeout=60, env=env)
        calls.append(dict(argv=argv, exit=result.returncode, stdout=result.stdout, stderr=result.stderr))
        return result

    def ok(result):
        assert result.returncode == 0, result.stderr

    def input_file(name, event_id, revision, seq, price):
        payload = dict(schema_revision="1", session_id="r13-integrity", source_namespace="testonly.tick.ohlc",
                       source_epoch="1", instrument="TEST.TICK", profile="testonly_tick_1_1_ohlc",
                       events=[dict(event_id=event_id, revision=str(revision), seq=str(seq),
                                    price=str(price), timestamp=str(seq), volume="1", raw_text=str(price),
                                    received_at="2026-09-13T00:00:00Z")])
        path = args.output / (name + ".json")
        path.write_text(json.dumps(payload), encoding="utf-8")
        return path

    def dump(db):
        with closing(sqlite3.connect(db)) as conn:
            return "\n".join(conn.iterdump())

    def read(db, as_of):
        try:
            with closing(reader.open_readonly(db)) as conn:
                conn.execute("BEGIN")
                state = reader.read_state(conn, as_of)
        except Exception as exc:
            reads.append(dict(db=str(db), cut=as_of, ok=False,
                              exception=type(exc).__name__, message=str(exc)))
            raise
        reads.append(dict(db=str(db), cut=as_of, ok=True))
        return state

    def assert_unavailable(db, label, generation, extra):
        before = dump(db)
        for cut in [None, 0, generation]:
            try:
                read(db, cut)
            except ValueError:
                pass
            else:
                raise AssertionError(f"{label}: Python 未拒绝损坏存储，cut={cut}")
            options = [] if cut is None else ["--as-of", cut]
            result = cli(db, "snapshot", *options)
            assert result.returncode != 0 and "StorageUnavailable" in result.stderr
        for command, options in [
            ("accept", ["--input", extra, "--profile", PROFILE]),
            ("advance", []), ("recover", ["--new-epoch", "2"]),
        ]:
            result = cli(db, command, *options)
            assert result.returncode != 0 and "StorageUnavailable" in result.stderr
            assert dump(db) == before, f"{label}: {command} 失败后持久事实改变"

    failure = None
    try:
        seed = args.output / "seed.db"
        ok(cli(seed, "init", "--session", "r13-integrity", "--catalog", CATALOG))
        for i, price in enumerate([10000, 11000, 10500]):
            inp = input_file("e" + str(i), "e" + str(i), 1, i, price)
            ok(cli(seed, "accept", "--input", inp, "--profile", PROFILE))
            # 接纳未发布的合法原始事实不得被发布代检查误拒。
            read(seed, None)
            ok(cli(seed, "advance"))
        correction = input_file("correction", "e2", 2, 2, 11500)
        ok(cli(seed, "accept", "--input", correction, "--profile", PROFILE))
        ok(cli(seed, "advance"))
        for cut in [None, 0, 4]:
            read(seed, cut)
        extra = input_file("extra", "e3", 1, 3, 10800)
        probes = [("objects", "object_id", False), ("objects", "object_id", True),
                  ("witnesses", "witness_id", False), ("relations", "relation_id", False),
                  ("observations", "observation_id", False)]
        for index, (table, id_column, withdrawn) in enumerate(probes):
            db = args.output / ("future-" + str(index) + ".db")
            with closing(sqlite3.connect(seed)) as source, closing(sqlite3.connect(db)) as conn:
                source.backup(conn)
                with conn:
                    columns = [row[1] for row in conn.execute(f"PRAGMA table_info({table})")]
                    expressions = []
                    for column in columns:
                        if column == id_column:
                            expressions.append("'future-' || " + column)
                        elif column in ("first_known_generation", "published_generation"):
                            expressions.append("5")
                        elif column == "withdrawn_generation":
                            expressions.append("6" if withdrawn else "NULL")
                        else:
                            expressions.append(column)
                    # 标识符只来自固定测试表及正式 schema，值取真实已发布行。
                    inserted = conn.execute(f"INSERT INTO {table} SELECT {','.join(expressions)} FROM {table} LIMIT 1")
                    assert inserted.rowcount == 1
            assert_unavailable(db, table, 4, extra)
            cases.append(dict(table=table, withdrawn=withdrawn, passed=True))
        later = input_file("later", "e4", 1, 4, 10700)
        for published in [False, True]:
            for kind, coordinate in [("invalid", "bad"), ("collision", "0")]:
                db = args.output / f"pending-{published}-{kind}.db"
                if published:
                    with closing(sqlite3.connect(seed)) as source, closing(sqlite3.connect(db)) as conn:
                        source.backup(conn)
                else:
                    ok(cli(db, "init", "--session", "r13-integrity", "--catalog", CATALOG))
                    ok(cli(db, "accept", "--input", args.output / "e0.json", "--profile", PROFILE))
                ok(cli(db, "accept", "--input", extra, "--profile", PROFILE))
                read(db, None)  # 合法待发布原始事件仍可读。
                with closing(sqlite3.connect(db)) as conn, conn:
                    conn.execute("UPDATE raw_events SET source_coord=? WHERE event_id='e3'", (coordinate,))
                assert_unavailable(db, f"pending-{published}-{kind}", 4 if published else 0, later)
                cases.append(dict(kind="pending_raw", published=published, corruption=kind, passed=True))
    except Exception as exc:
        failure = dict(exception=type(exc).__name__, message=str(exc))
        raise
    finally:
        report = dict(cases=cases, calls=calls, reads=reads, failure=failure,
                      reader=str(args.reader.resolve()), binary=str(args.binary.resolve()))
        (args.output / "RESULT.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"PASS: {len(cases)} 个未来索引/未发布原始记录分支；双端当前/历史读取；三类写入拒绝且全库事实不变")


if __name__ == "__main__":
    main()
