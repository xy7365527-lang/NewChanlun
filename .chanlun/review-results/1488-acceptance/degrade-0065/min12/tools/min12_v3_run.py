#!/usr/bin/env python3
"""
degrade-0065 改写：accept-s2 min12_oracle_run.py 的 S3 版（s-launcher/3 + Q3 query-serve）。
改动只有：launcher 版本、摄入/查询信封版本 s-session/3、记录 schema 名；查询与本地复算逻辑原样。
#1488 S2：MIN12 冻结流经正式会话入口逐事件摄入，再逐代公共查询生成 oracle。

正式入口 = `s_session/launch_s.sh service start`（s-launcher/2，S 常驻 + Q 只读）；
逐事件 = 冻结 messages.jsonl 原请求逐条经 `s_socket_client.exchange` 发往 S socket，等完整回执再发下一条；
公共查询 = Q 的 POST /api/v2/snapshot（AsKnown，as_of_generation=g，按 snapshot_token 翻完全部分页）。

一次调用 = 一个新进程、新库的完整运行；双跑由调用方用两个 --work 目录各跑一次。
时延帽不在本脚本里判定：S/客户端回执期限取 --reply-timeout-ms（oracle 生成不因帽中断），
每条实际回复耗时如实记录，是否 ≤10 s 由报告判读。

输出（--work 下）：
  config.json / control/          正式 launcher 配置与控制目录
  INGEST.jsonl                    每条输入：发送/回执 ns、耗时、回执 publication
  oracle/g-XXXX.json              第 g 代 AsKnown 完整公共切面（canonical JSON：fixed_cut/catalog/rows/counts/...）
  ORACLE-INDEX.jsonl              每代：服务端 cut_projection_digest、本地复算摘要、分页数、各族计数、查询耗时
  STRUCTURAL.jsonl                每代：CC-008 及以上结构对象（CC-055.relation 除外）完整记录，供读数抽取
  RUN.json                        输入哈希、二进制哈希、起止、结论
"""
import argparse
import hashlib
import http.client
import json
import os
import subprocess
import sys
import time
from pathlib import Path

BULK_KINDS = ("CC-004.", "CC-005.", "CC-006.", "CC-007.", "CC-055.relation")


def sha256_file(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--repo", required=True, type=Path)
    ap.add_argument("--binary", required=True, type=Path)
    ap.add_argument("--python", required=True)
    ap.add_argument("--messages", required=True, type=Path)
    ap.add_argument("--clock", required=True, type=Path)
    ap.add_argument("--expect-messages-sha", required=True)
    ap.add_argument("--expect-clock-sha", required=True)
    ap.add_argument("--work", required=True, type=Path)
    ap.add_argument("--port", required=True)
    ap.add_argument("--socket", required=True)
    ap.add_argument("--reply-timeout-ms", type=int, default=120000)
    ap.add_argument("--query-resources", required=True, type=Path)
    args = ap.parse_args()

    repo = args.repo.resolve()
    sys.path.insert(0, str(repo / "s_session"))
    import s_socket_client as sc  # noqa: E402
    import s_query_integrity as qi  # noqa: E402
    sys.path.insert(0, str(Path(__file__).resolve().parent))
    import db_logical_digest as dbd  # noqa: E402

    for path, expect in ((args.messages, args.expect_messages_sha), (args.clock, args.expect_clock_sha)):
        got = sha256_file(path)
        if got != expect:
            raise SystemExit(f"输入哈希不符 {path}: {got} != {expect}")
    work = args.work
    work.mkdir(parents=True, exist_ok=False)
    (work / "oracle").mkdir()
    lines = args.messages.read_bytes().splitlines()
    first = json.loads(lines[0])
    config = {
        "schema_revision": "s-launcher/3",
        **{k: first[k] for k in ("session_id", "session_generation", "source_namespace", "source_epoch")},
        "producer_id": "tb1419-s0b-control", "producer_epoch": "1",
        "writer_epoch": "1", "query_epoch": "1", "delivery_retain_generations": "4",
        "port": args.port, "startup_timeout_ms": "10000",
        "db": str(work / "session.sqlite"), "socket": args.socket,
        "binary": str(args.binary.resolve()), "python": args.python,
        "catalog": str(repo / "s_session/catalog/signed-catalog.json"),
        "profile": str(repo / "s_session/profiles/ohlc_integer_tb02a_v1.json"),
        "clock_plan": str(args.clock.resolve()),
        "browser": str(repo / "s_session/browser/index.html"),
        "query_resource_config": str(args.query_resources.resolve()),
        "s_resources": {"max_frame_bytes": "1048576", "read_timeout_ms": "2000", "write_timeout_ms": "1000",
                        "queue_capacity": "32", "max_connections": "48",
                        "response_timeout_ms": str(args.reply_timeout_ms),
                        "audit_cache_source_bytes": "268435456"},
    }
    (work / "config.json").write_text(json.dumps(config, ensure_ascii=False, indent=2) + "\n")
    env = dict(os.environ, PYO3_PYTHON=args.python)
    launcher = [str(repo / "s_session/launch_s.sh"), "service"]
    control = ["--config", str(work / "config.json"), "--state-dir", str(work / "control")]
    run = {"schema": "1488-d65-min12-v3-run/1", "binary": str(args.binary), "binary_sha256": sha256_file(args.binary),
           "messages_sha256": args.expect_messages_sha, "clock_sha256": args.expect_clock_sha,
           "repo_head": subprocess.run(["git", "-C", str(repo), "rev-parse", "HEAD"], capture_output=True, text=True).stdout.strip(),
           # 0055/F10：留存工作树状态快照（空 = clean）
           "repo_status_porcelain": subprocess.run(["git", "-C", str(repo), "status", "--porcelain"],
                                                   capture_output=True, text=True).stdout[-4000:],
           "reply_timeout_ms": args.reply_timeout_ms, "query_resources_sha256": sha256_file(args.query_resources),
           "started_utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())}
    started = subprocess.run(launcher + ["start"] + control, capture_output=True, text=True, env=env)
    run["start_exit"] = started.returncode
    if started.returncode != 0:
        run["start_stderr"] = started.stderr[-4000:]
        run["start_stdout"] = started.stdout[-4000:]
        (work / "RUN.json").write_text(json.dumps(run, ensure_ascii=False, indent=2))
        raise SystemExit("launcher start 失败")

    ingest_ok = True
    try:
        with (work / "INGEST.jsonl").open("w") as out:
            for index, raw in enumerate(lines):
                request = sc.decode_frame(raw)
                # degrade-0065：S3 入口；信封除版本外逐字段不变（payload_hash 只覆盖 payload）。
                assert request["schema_revision"] == "s-session/2"
                request["schema_revision"] = "s-session/3"
                sent = time.monotonic_ns()
                result = sc.exchange(config["socket"], request, timeout_ms=args.reply_timeout_ms,
                                     max_frame_bytes=1048576)
                received = time.monotonic_ns()
                record = {"index": index, "message_id": request["message_id"], "sent_ns": sent,
                          "received_ns": received, "reply_ms": (received - sent) / 1e6,
                          "transport": result["transport"]}
                if result["transport"] == "Received":
                    payload = sc.validate_reply(request, result["response"], producer="S", producer_epoch="1")
                    record.update(kind=payload.get("kind"), ok=payload.get("ok"),
                                  accepted_seq=payload.get("accepted_seq"),
                                  publication=payload.get("publication"), receipt_id=payload.get("receipt_id"))
                    if payload.get("kind") != "Committed":
                        record["reply_payload"] = json.dumps(payload, ensure_ascii=False)[:4000]  # degrade-0065：留失败回执
                else:
                    record["detail"] = result.get("detail")
                out.write(json.dumps(record, ensure_ascii=False) + "\n")
                out.flush()
                print(f"op {index} {record['reply_ms']:.1f}ms {record['transport']}", flush=True)
                if result["transport"] != "Received" or payload.get("kind") != "Committed":
                    ingest_ok = False
                    break
        run["ingest_complete"] = ingest_ok
        head = int(record["publication"]["generation"]) if ingest_ok else None
        run["head_generation"] = head
        if ingest_ok:
            run["queries"] = query_all(config, head, work, qi, sc)
            run["queries_complete"] = True
    except Exception as exc:
        run["error"] = f"{type(exc).__name__}: {exc}"[:4000]
        raise
    finally:
        stopped = subprocess.run(launcher + ["stop"] + control, capture_output=True, text=True, env=env)
        run["stop_exit"] = stopped.returncode
        # 0055/F8：stop 的输出留存（run-b stop_exit=1 时没有可查的 stderr）
        run["stop_stdout"] = stopped.stdout[-4000:]
        run["stop_stderr"] = stopped.stderr[-4000:]
        run["finished_utc"] = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
        db = work / "session.sqlite"
        run["db_sha256"] = sha256_file(db) if db.exists() else None
        # 0055/F9：服务停止后把 WAL 并入主库，再记“主库文件 + 逻辑内容”摘要，跨跑才可比
        if db.exists():
            try:
                run["db_digest"] = dbd.digest(str(db), in_place=True)
            except Exception as exc:
                run["db_digest"] = {"error": f"{type(exc).__name__}: {exc}"}
        run["ok"] = bool(ingest_ok and run.get("queries_complete") and run["stop_exit"] == 0
                         and run.get("db_digest", {}).get("checkpoint_ok"))
        (work / "RUN.json").write_text(json.dumps(run, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps({k: run.get(k) for k in ("ingest_complete", "head_generation", "queries_complete", "stop_exit", "ok")}))
    if not run["ok"]:
        raise SystemExit(1)


def post(port, envelope, canonical):
    body = canonical(envelope)
    conn = http.client.HTTPConnection("127.0.0.1", int(port), timeout=900)
    conn.request("POST", "/api/v3/snapshot", body=body, headers={"Content-Type": "application/json",
                                                                 "Content-Length": str(len(body))})
    resp = conn.getresponse()
    data = resp.read()
    conn.close()
    return resp.status, json.loads(data)


def envelope_for(config, payload, counter, canonical):
    return {"schema_revision": "s-session/3", "session_id": config["session_id"],
            "session_generation": config["session_generation"], "source_namespace": "s-observe/oracle-1488-s2",
            "source_epoch": "1", "message_id": f"oracle-query-{counter}", "producer_id": "oracle:1488-s2",
            "producer_epoch": "1", "causal_refs": [], "payload": payload,
            "payload_hash": hashlib.sha256(canonical(payload)).hexdigest()}


def query_all(config, head, work, qi, sc):
    canonical = qi.canonical
    counter = 0
    summary = []
    with (work / "ORACLE-INDEX.jsonl").open("w") as index_out, (work / "STRUCTURAL.jsonl").open("w") as struct_out:
        for g in range(1, head + 1):
            t0 = time.monotonic_ns()
            rows, pages, base = [], 0, None
            payload = {"op": "snapshot", "session_id": config["session_id"],
                       "session_generation": config["session_generation"],
                       "scope": {"economic": "not_started", "structure": "CompleteCut"},
                       "history_mode": "AsKnown", "as_of_generation": str(g), "page_size": "31",
                       "order_version": "s-record-order/2"}
            while True:
                counter += 1
                status, response = post(config["port"], envelope_for(config, payload, counter, canonical), canonical)
                body = response.get("payload", response)
                if status != 200 or not body.get("ok"):
                    raise RuntimeError(f"g={g} 查询失败 {status} {json.dumps(body, ensure_ascii=False)[:600]}")
                pages += 1
                rows.extend(body["rows"])
                if base is None:
                    base = body
                elif body["cut_projection_digest"] != base["cut_projection_digest"]:
                    raise RuntimeError(f"g={g} 分页间切面摘要变化")
                if body["done"]:
                    break
                payload = {"op": "snapshot", "snapshot_token": body["next_token"]}
            elapsed_ms = (time.monotonic_ns() - t0) / 1e6
            projection = {k: base[k] for k in ("fixed_cut", "catalog", "counts", "scope", "omissions", "order_version")}
            projection["rows"] = rows
            blob = canonical(projection)
            local = hashlib.sha256(blob).hexdigest()
            (work / "oracle" / f"g-{g:04d}.json").write_bytes(blob)
            kinds = {}
            for row in rows:
                if row["family"] in ("objects", "withdrawn_objects"):
                    key = row["family"] + ":" + str(row["record"].get("kind"))
                    kinds[key] = kinds.get(key, 0) + 1
            entry = {"generation": g, "server_cut_projection_digest": base["cut_projection_digest"],
                     "local_projection_sha256": local, "digest_match": local == base["cut_projection_digest"],
                     "bytes": len(blob), "pages": pages, "query_ms": elapsed_ms, "counts": base["counts"],
                     "rule_revision": base["fixed_cut"].get("rule_revision"),
                     "structure_cut": base["fixed_cut"].get("structure_cut"), "kind_counts": dict(sorted(kinds.items()))}
            index_out.write(json.dumps(entry, ensure_ascii=False) + "\n")
            index_out.flush()
            if not entry["digest_match"]:
                # 0055/F8：本地复算与服务端切面摘要不符即失败，不只记录
                raise RuntimeError(f"g={g} 本地复算 {local} != 服务端 cut_projection_digest {base['cut_projection_digest']}")
            structural = [row["record"] for row in rows if row["family"] == "objects"
                          and "fact_key" in row["record"] and not str(row["record"].get("kind")).startswith(BULK_KINDS)]
            struct_out.write(json.dumps({"generation": g, "objects": structural}, ensure_ascii=False) + "\n")
            summary.append({"generation": g, "digest": base["cut_projection_digest"], "match": entry["digest_match"],
                            "query_ms": round(elapsed_ms, 1)})
            print(f"g {g} rows {len(rows)} pages {pages} {elapsed_ms:.0f}ms match={entry['digest_match']}", flush=True)
    return summary


if __name__ == "__main__":
    main()
