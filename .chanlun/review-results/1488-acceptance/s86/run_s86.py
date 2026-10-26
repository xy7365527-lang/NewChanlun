#!/usr/bin/env python3
"""#1488 验收 S1：用给定 s_structure_session 二进制完整路径跑 S86（新库、新进程）。

流程：init（s-session/3）→ serve（Unix socket，单进程常驻，跨请求热态保留）
→ 逐条 86 个信封各一连接一帧 → 每代记读数 → SIGTERM 停服。

读数（Linux /proc；均为进程级代理量，非产品内计数器）：
  elapsed_ns   客户端往返（发送到收到完整回复帧）
  rchar/syscr  服务进程该请求期间 read 系统调用字节/次数增量（SQLite 页读的上界代理）
  read_bytes   实际落到块设备的读字节（页缓存未命中）
  wchar/syscw/write_bytes 写侧对应量
  cpu_ticks    utime+stime 增量（USER_HZ）
  host_loadavg_1m  提交后主机 1 分钟负载（共享主机，计时须结合此列解读）
  stored_nodes/stored_body_bytes  提交后另开只读连接计 s_content_nodes 行数与 body 总长
main 二进制无「SQL 节点读量」产品计数；rchar/syscr 为可比代理，二臂同口径即可对照。

用法：run_s86.py --bin BIN --workdir DIR [--inputs DIR]
DIR 必须不存在；产物：s.sqlite*、init.json、responses.jsonl、readings.jsonl、
serve.stderr、run-manifest.json。
"""

import argparse
import hashlib
import json
import os
import signal
import sqlite3
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parents[3] / "s_session"))
from s_socket_client import exchange  # noqa: E402

SESSION = "s-session-testonly-001"


def sha_file(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def proc_io(pid):
    out = {}
    for line in Path(f"/proc/{pid}/io").read_text().splitlines():
        key, value = line.split(":")
        out[key.strip()] = int(value)
    stat = Path(f"/proc/{pid}/stat").read_text()
    fields = stat[stat.rindex(")") + 2:].split()
    out["cpu_ticks"] = int(fields[11]) + int(fields[12])
    return out


def stored(db):
    conn = sqlite3.connect(f"file:{db}?mode=ro", uri=True)
    try:
        return conn.execute(
            "SELECT count(*),COALESCE(sum(length(body)),0) FROM s_content_nodes").fetchone()
    finally:
        conn.close()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--bin", required=True)
    ap.add_argument("--workdir", required=True)
    ap.add_argument("--inputs", default=str(HERE))
    ap.add_argument("--timeout-ms", type=int, default=3_600_000)
    a = ap.parse_args()
    inputs = Path(a.inputs)
    work = Path(a.workdir)
    work.mkdir(parents=True, exist_ok=False)
    binary = str(Path(a.bin).resolve())
    files = {"catalog": inputs / "signed-catalog.json", "profile": inputs / "ohlc_integer_tb02a_v1.json",
             "clock": inputs / "clock-plan.s86.json", "resources": inputs / "s-resources.json",
             "messages": inputs / "messages.jsonl"}
    db = work / "s.sqlite"
    common = ["--protocol-revision", "s-session/3"]
    env = {k: v for k, v in os.environ.items() if not k.startswith(("S_", "NC_TESTONLY"))}
    init = subprocess.run(
        [binary, "init", *common, "--db", str(db), "--session", SESSION,
         "--catalog", str(files["catalog"]), "--profile", str(files["profile"]),
         "--session-generation", "1", "--clock-plan", str(files["clock"]),
         "--delivery-retain-generations", "4"],
        capture_output=True, env=env)
    (work / "init.json").write_bytes(init.stdout)
    (work / "init.stderr").write_bytes(init.stderr)
    if init.returncode != 0:
        sys.exit(f"init 失败 rc={init.returncode}: {init.stderr.decode()[-2000:]}")
    sock = work / "s.sock"
    if len(str(sock).encode()) > 100:
        sys.exit("socket 路径超过 100 字节")
    resources = json.loads(files["resources"].read_text())["s_resources"]
    max_frame = int(resources["max_frame_bytes"])
    stderr = open(work / "serve.stderr", "wb")
    started_wall = time.time()
    server = subprocess.Popen(
        [binary, "serve", *common, "--db", str(db), "--profile", str(files["profile"]),
         "--writer-epoch", "1", "--clock-plan", str(files["clock"]), "--socket", str(sock),
         "--resource-config", str(files["resources"])],
        stdout=subprocess.PIPE, stderr=stderr, env=env)
    t0 = time.monotonic()
    while not sock.exists():
        if server.poll() is not None:
            sys.exit(f"serve 提前退出 rc={server.returncode}")
        if time.monotonic() - t0 > 600:
            server.kill()
            sys.exit("serve 600s 内未监听")
        time.sleep(0.01)
    startup_ns = int((time.monotonic() - t0) * 1e9)
    lines = files["messages"].read_bytes().splitlines()
    responses = open(work / "responses.jsonl", "w", encoding="utf-8")
    readings = open(work / "readings.jsonl", "w", encoding="utf-8")
    failure = None
    try:
        for n, line in enumerate(lines, 1):
            request = json.loads(line)
            before = proc_io(server.pid)
            begun = time.monotonic_ns()
            result = exchange(sock, request, timeout_ms=a.timeout_ms, max_frame_bytes=max_frame)
            elapsed = time.monotonic_ns() - begun
            after = proc_io(server.pid)
            responses.write(json.dumps(result, ensure_ascii=False, sort_keys=True,
                                       separators=(",", ":")) + "\n")
            responses.flush()
            kind = (result.get("response") or {}).get("payload", {}).get("kind")
            nodes, body = stored(db)
            row = {"generation": n, "kind": kind, "elapsed_ns": elapsed,
                   "host_loadavg_1m": float(Path("/proc/loadavg").read_text().split()[0]),
                   "stored_nodes": nodes, "stored_body_bytes": body,
                   **{k: after[k] - before[k] for k in after}}
            readings.write(json.dumps(row, sort_keys=True) + "\n")
            readings.flush()
            print(f"gen {n} {kind} {elapsed / 1e6:.1f}ms nodes={nodes} syscr={row['syscr']}",
                  flush=True)
            if result.get("transport") != "Received" or kind != "Committed":
                failure = f"第 {n} 条未 Committed：{json.dumps(result, ensure_ascii=False)[:2000]}"
                break
    finally:
        server.send_signal(signal.SIGTERM)
        try:
            server.wait(timeout=60)
        except subprocess.TimeoutExpired:
            server.kill()
            server.wait()
        stderr.close()
    # 停服后用产品冷路径取当前（末代）FinancialBasisRef，供对拍脚本核其独立重算的 basis 根。
    basis = subprocess.run(
        [binary, "financial-basis-ref", *common, "--db", str(db)],
        input=json.dumps({"schema_revision": "s-financial-query/1", "session_id": SESSION,
                          "session_generation": "1", "writer_epoch": "1", "object_ids": []}).encode(),
        capture_output=True, env=env)
    (work / "basis-ref.json").write_bytes(basis.stdout)
    (work / "basis-ref.stderr").write_bytes(basis.stderr)
    manifest = {"binary": binary, "binary_sha256": sha_file(binary),
                "inputs": {k: {"path": str(v), "sha256": sha_file(v)} for k, v in files.items()},
                "workdir": str(work), "started_unix": started_wall, "serve_startup_ns": startup_ns,
                "serve_returncode": server.returncode, "generations": n, "failure": failure,
                "basis_ref_returncode": basis.returncode,
                "host": os.uname().nodename, "kernel": os.uname().release}
    (work / "run-manifest.json").write_text(json.dumps(manifest, indent=1, ensure_ascii=False))
    if failure:
        sys.exit(failure)
    print("RUN_OK", work)


if __name__ == "__main__":
    main()
