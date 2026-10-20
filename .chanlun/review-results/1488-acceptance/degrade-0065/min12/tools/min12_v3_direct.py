#!/usr/bin/env python3
"""degrade-0065：MIN12 直驱 S3 版（与 S86 run_s86 同形：init + 常驻 serve，不经 launcher）。

为什么不走 launcher：s-launcher/3 启动时先发一条 `ready`。在 testonly native 模式下，新库上
ready 先行之后 op-0000 回 `StaleWriter：construction资格失效：Carry租约已撤销`；
改前二进制 727b3bfb45 同样复现（min12-base-nat1），属于既有缺陷，不在 0065 P1 范围，见 REPORT。
b-0059 量化 MIN12 用的也是不带 ready 的回放。

流程：
1. init（s-session/3，身份取冻结 messages 首条，参数同 accept-s2 launcher 配置）；
2. 常驻 serve，52 条信封逐条要求 Committed（版本改写 s-session/3，payload 原样）；
3. SIGTERM 停服；
4. 起 Q3 `query-serve`，复用 min12_v3_run.query_all 逐代取 /api/v2/snapshot 并本地复算；
5. 与 accept-s2 ORACLE.json 逐代比 cut_projection_digest。
"""
import argparse
import json
import os
import signal
import socket
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[5]
sys.path.insert(0, str(REPO / "s_session"))
sys.path.insert(0, str(HERE))
import s_socket_client as sc  # noqa: E402
import s_query_integrity as qi  # noqa: E402
import min12_v3_run as m  # noqa: E402

KEEP = ("S_CAPTURE_NATIVE_TESTONLY", "S_MANIFEST_WORKSPACE_DIAGNOSTICS")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--bin", required=True)
    ap.add_argument("--work", required=True, type=Path)
    ap.add_argument("--port", required=True)
    ap.add_argument("--timeout-ms", type=int, default=1_800_000)
    a = ap.parse_args()
    root = HERE.parent
    messages, clock = root / "input/messages.jsonl", root / "input/clock.json"
    for path, expect in ((messages, "d61371c95036eaaaacc94c9780955555271be41a49ad4d5f242424a1c447c6cd"),
                         (clock, "779dcb8539bc9278933bde14d852e9d34493d956cb2c2647cb1db3ead9dfb36f")):
        if m.sha256_file(path) != expect:
            sys.exit(f"输入哈希不符 {path}")
    work = a.work
    work.mkdir(parents=True, exist_ok=False)
    (work / "oracle").mkdir()
    binary = str(Path(a.bin).resolve())
    lines = messages.read_bytes().splitlines()
    first = json.loads(lines[0])
    config = {k: first[k] for k in ("session_id", "session_generation", "source_namespace", "source_epoch")}
    catalog = REPO / "s_session/catalog/signed-catalog.json"
    profile = REPO / "s_session/profiles/ohlc_integer_tb02a_v1.json"
    resources = work / "s-resources.json"
    resources.write_text(json.dumps({"schema_revision": "s-structure-resources/1", "s_resources": {
        "max_frame_bytes": "1048576", "read_timeout_ms": "2000", "write_timeout_ms": "1000",
        "queue_capacity": "1", "max_connections": "1", "response_timeout_ms": str(a.timeout_ms),
        "audit_cache_source_bytes": "268435456"}}, sort_keys=True, separators=(",", ":")))
    db = work / "session.sqlite"
    common = ["--protocol-revision", "s-session/3"]
    env = {k: v for k, v in os.environ.items() if k in KEEP or not k.startswith(("S_", "NC_TESTONLY"))}
    run = {"schema": "1488-d65-min12-v3-direct/1", "binary": binary, "binary_sha256": m.sha256_file(binary),
           "env_testonly": {k: env.get(k) for k in KEEP}, "started_unix": time.time()}
    init = subprocess.run([binary, "init", *common, "--db", str(db), "--session", config["session_id"],
                           "--catalog", str(catalog), "--profile", str(profile),
                           "--session-generation", config["session_generation"], "--clock-plan", str(clock),
                           "--delivery-retain-generations", "4"], capture_output=True, env=env)
    (work / "init.json").write_bytes(init.stdout)
    if init.returncode != 0:
        sys.exit(f"init 失败 rc={init.returncode}: {init.stderr.decode()[-2000:]}")
    sock = work / "s.sock"
    stderr = open(work / "serve.stderr", "wb")
    server = subprocess.Popen([binary, "serve", *common, "--db", str(db), "--profile", str(profile),
                               "--writer-epoch", "1", "--clock-plan", str(clock), "--socket", str(sock),
                               "--resource-config", str(resources)], stdout=subprocess.PIPE, stderr=stderr, env=env)
    t0 = time.monotonic()
    while not sock.exists():
        if server.poll() is not None or time.monotonic() - t0 > 600:
            sys.exit(f"serve 未监听 rc={server.poll()}")
        time.sleep(0.01)
    ok, head = True, None
    with (work / "INGEST.jsonl").open("w") as out:
        for index, raw in enumerate(lines):
            request = sc.decode_frame(raw)
            assert request["schema_revision"] == "s-session/2"
            request["schema_revision"] = "s-session/3"
            begun = time.monotonic_ns()
            result = sc.exchange(str(sock), request, timeout_ms=a.timeout_ms, max_frame_bytes=1048576)
            elapsed = time.monotonic_ns() - begun
            payload = (result.get("response") or {}).get("payload", {})
            record = {"index": index, "message_id": request["message_id"], "elapsed_ns": elapsed,
                      "reply_ms": elapsed / 1e6, "transport": result["transport"], "kind": payload.get("kind"),
                      "publication": payload.get("publication"),
                      "host_loadavg_1m": float(Path("/proc/loadavg").read_text().split()[0])}
            if payload.get("kind") != "Committed":
                record["reply_payload"] = json.dumps(result, ensure_ascii=False)[:4000]
            out.write(json.dumps(record, ensure_ascii=False) + "\n")
            out.flush()
            print(f"op {index} {elapsed / 1e6:.1f}ms {record['kind']}", flush=True)
            if result["transport"] != "Received" or payload.get("kind") != "Committed":
                ok = False
                break
            head = int(payload["publication"]["generation"])
    server.send_signal(signal.SIGTERM)
    run["serve_returncode"] = server.wait(timeout=600)
    run.update(ingest_complete=ok, head_generation=head)
    if ok:
        qlog = open(work / "query-serve.log", "wb")
        q = subprocess.Popen([binary, "query-serve", *common, "--db", str(db),
                              "--resource-config", str(HERE / "query-resources-q3.json"),
                              "--producer-epoch", "1", "--host", "127.0.0.1", "--port", a.port],
                             stdout=qlog, stderr=qlog, env=env)
        try:
            t0 = time.monotonic()
            while True:
                try:
                    socket.create_connection(("127.0.0.1", int(a.port)), timeout=1).close()
                    break
                except OSError:
                    if q.poll() is not None or time.monotonic() - t0 > 600:
                        raise SystemExit(f"query-serve 未就绪 rc={q.poll()}")
                    time.sleep(0.1)
            run["queries"] = m.query_all(config, head, work, qi, sc)
            run["queries_complete"] = True
        finally:
            q.send_signal(signal.SIGTERM)
            run["query_returncode"] = q.wait(timeout=60)
        oracle = {g["generation"]: g["cut_projection_digest"]
                  for g in json.loads((root / "oracle/ORACLE.json").read_text())["generations"]}
        diffs = [s["generation"] for s in run["queries"] if oracle.get(s["generation"]) != s["digest"]]
        run["oracle_compare"] = {"generations": len(run["queries"]), "oracle_generations": len(oracle),
                                 "equal": len(run["queries"]) - len(diffs), "diff_generations": diffs}
    compare = run.get("oracle_compare", {})
    run["ok"] = bool(ok and run.get("queries_complete") and head == 52
                     and compare.get("equal") == 52 and not compare.get("diff_generations"))
    (work / "RUN.json").write_text(json.dumps(run, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps({k: run.get(k) for k in ("ingest_complete", "head_generation", "oracle_compare", "ok")},
                     ensure_ascii=False))
    sys.exit(0 if run["ok"] else 1)


if __name__ == "__main__":
    main()
