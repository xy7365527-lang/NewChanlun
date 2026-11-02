#!/usr/bin/env python3
"""#1488 a5s6-0076：S6（D11）第 46 条 trace + A⑤（D3）测试客户端核对，一次运行一个二进制。

正式入口 = `s_session/launch_s.sh service start`（s-launcher/3：S 常驻写者 + Rust Q3 query-serve），
MIN12 冻结 52 条（S2 入仓 input/，sha 校验）逐条经 `s_socket_client.exchange` 发送，等完整回执再发下一条
（信封只把 schema_revision 改为 s-session/3，payload 与 payload_hash 原样，同 degrade-0065 min12_v3_run.py）。

S6 trace（生产构建没有阶段计时/SQL 计数埋点，以下全部来自服务端进程的内核计数，不改产品代码）：
  INGEST.jsonl  每条：回复墙钟、S 进程 CPU（utime/stime）、/proc/<pid>/io 全字段差（rchar/syscr/read_bytes/
                wchar/syscw/write_bytes）、RSS/HWM、线程数、库与 WAL 字节、宿主 /proc/stat 忙核数与 loadavg
  TRACE.jsonl   每 --sample-ms 采样一次 S 的累计计数（op 内时间线：读/算/写段）
  HOST.jsonl    每 5 s 宿主 loadavg 与 CPU 占用前 6 的进程（记录窗口是否安静/谁在占用）
A⑤：摄入完成后，经同一正式入口的 Q（冻结资源档 --q-resources）用 a5_client.cjs 逐代核对（A5-formal.jsonl）；
停服后另起 query-serve（--q-functional 功能期限档，诊断用）对 g=1..52 全量核对（A5-functional.jsonl）。
回执期限 --reply-timeout-ms 默认 1,800,000：为取完整 52 条曲线，不因帽中断；10 s 帽按 reply_ms 判读。
"""
import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
import threading
import time
from pathlib import Path

TICK = os.sysconf("SC_CLK_TCK")
MSG_SHA = "d61371c95036eaaaacc94c9780955555271be41a49ad4d5f242424a1c447c6cd"
CLOCK_SHA = "779dcb8539bc9278933bde14d852e9d34493d956cb2c2647cb1db3ead9dfb36f"


def sha256_file(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def proc(pid):
    """S 进程累计计数；进程不在则 None。"""
    try:
        stat = Path(f"/proc/{pid}/stat").read_text()
        fields = stat[stat.rindex(")") + 2:].split()
        io = dict(line.split(": ") for line in Path(f"/proc/{pid}/io").read_text().split("\n") if line)
        status = dict(line.split(":\t", 1) for line in Path(f"/proc/{pid}/status").read_text().split("\n") if ":\t" in line)
    except (FileNotFoundError, ProcessLookupError, PermissionError):
        return None
    return {"state": fields[0], "utime_ms": int(fields[11]) * 1000 // TICK, "stime_ms": int(fields[12]) * 1000 // TICK,
            "threads": int(fields[17]), **{k: int(v) for k, v in io.items()},
            "rss_kb": int(status.get("VmRSS", "0 kB").split()[0]), "hwm_kb": int(status.get("VmHWM", "0 kB").split()[0])}


def host_cpu():
    parts = Path("/proc/stat").read_text().split("\n", 1)[0].split()[1:]
    vals = list(map(int, parts))
    idle = vals[3] + vals[4]
    return sum(vals), idle


def loadavg():
    return [float(x) for x in Path("/proc/loadavg").read_text().split()[:3]]


def top_procs(n=6):
    out = subprocess.run(["ps", "-eo", "pid,pcpu,etimes,args", "--sort=-pcpu"], capture_output=True, text=True).stdout
    rows = []
    for line in out.splitlines()[1:n + 1]:
        pid, pcpu, et, cmd = line.split(None, 3)
        rows.append({"pid": int(pid), "pcpu": float(pcpu), "etimes": int(et), "cmd": cmd[:160]})
    return rows


def delta(a, b):
    if a is None or b is None:
        return None
    return {k: b[k] - a[k] for k in b if isinstance(b[k], int) and k in a and k not in ("threads", "rss_kb", "hwm_kb")}


class Sampler(threading.Thread):
    def __init__(self, path, pid, every_ms, state):
        super().__init__(daemon=True)
        self.out, self.pid, self.every, self.state = path.open("w"), pid, every_ms / 1000, state
        self.stop = threading.Event()

    def run(self):
        while not self.stop.is_set():
            p = proc(self.pid)
            if p is not None:
                p.update(t_ns=time.monotonic_ns(), op=self.state.get("op"), load1=loadavg()[0])
                self.out.write(json.dumps(p) + "\n")
            self.stop.wait(self.every)
        self.out.close()


class HostLog(threading.Thread):
    def __init__(self, path):
        super().__init__(daemon=True)
        self.out, self.stop = path.open("w"), threading.Event()

    def run(self):
        while not self.stop.is_set():
            self.out.write(json.dumps({"utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()), "loadavg": loadavg(),
                                       "top": top_procs()}) + "\n")
            self.out.flush()
            self.stop.wait(5)
        self.out.close()


def node_client(repo, tools, port, gens, out, oracle, limits, current, log):
    cmd = ["node", str(tools / "a5_client.cjs"), "--repo", str(repo), "--base-url", f"http://127.0.0.1:{port}",
           "--gens", gens, "--out", str(out), "--limits", limits]
    if oracle:
        cmd += ["--oracle", str(oracle)]
    if current:
        cmd += ["--current"]
    t0 = time.monotonic()
    with log.open("a") as f:
        f.write("$ " + " ".join(cmd) + "\n")
        f.flush()
        rc = subprocess.run(cmd, stdout=f, stderr=subprocess.STDOUT).returncode
    return {"cmd": cmd, "rc": rc, "seconds": round(time.monotonic() - t0, 1)}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--repo", required=True, type=Path)
    ap.add_argument("--binary", required=True, type=Path)
    ap.add_argument("--python", default="/usr/bin/python3.12")
    ap.add_argument("--work", required=True, type=Path)
    ap.add_argument("--port", required=True)
    ap.add_argument("--socket", required=True)
    ap.add_argument("--reply-timeout-ms", type=int, default=1800000)
    ap.add_argument("--q-resources", required=True, type=Path)
    ap.add_argument("--q-functional", required=True, type=Path)
    ap.add_argument("--oracle", type=Path)
    ap.add_argument("--formal-gens", default="1,16,32,46,47,52")
    ap.add_argument("--functional-gens", default="1-52")
    # 冒烟实测：fixture 客户端档 max_elapsed_ms=60000 在 g≥2 即 job_deadline（g1 已 2054 请求/51 s），
    # 功能档客户端放长期限（relaxed，诊断档），正确性逐代核对不被客户端期限截断。
    ap.add_argument("--functional-limits", default="relaxed", choices=("fixture", "relaxed"))
    # final1 实测：功能档客户端 g16 已 31 min/7143 请求（逐请求成本随代增长），大代与当前代不再逐代全取。
    ap.add_argument("--functional-no-current", action="store_true")
    ap.add_argument("--ready-probe", action="store_true", help="每条提交后探正式 Q /api/ready（READY-PROBE.jsonl）")
    ap.add_argument("--formal-prefix", default="", type=lambda s: {x for x in s.split(",") if x},
                    help="需 --ready-probe：head 到这些代时在冻结 Q3 上跑一次正式客户端核对（逗号分隔）")
    ap.add_argument("--sample-ms", type=int, default=250)
    ap.add_argument("--max-ops", type=int, default=0, help="冒烟用：只发前 N 条（0 = 全部 52 条）")
    a = ap.parse_args()

    repo = a.repo.resolve()
    tools = Path(__file__).resolve().parent
    sys.path.insert(0, str(repo / "s_session"))
    import s_socket_client as sc  # noqa: E402
    inp = repo / ".chanlun/review-results/1488-acceptance/min12/input"
    messages, clock = inp / "messages.jsonl", inp / "clock.json"
    if sha256_file(messages) != MSG_SHA or sha256_file(clock) != CLOCK_SHA:
        raise SystemExit("MIN12 冻结输入哈希不符")
    work = a.work
    work.mkdir(parents=True, exist_ok=False)
    lines = messages.read_bytes().splitlines()
    if a.max_ops:
        lines = lines[:a.max_ops]
    first = json.loads(lines[0])
    config = {
        "schema_revision": "s-launcher/3",
        **{k: first[k] for k in ("session_id", "session_generation", "source_namespace", "source_epoch")},
        "producer_id": "tb1419-s0b-control", "producer_epoch": "1",
        "writer_epoch": "1", "query_epoch": "1", "delivery_retain_generations": "4",
        "port": a.port, "startup_timeout_ms": "10000",
        "db": str(work / "session.sqlite"), "socket": a.socket,
        "binary": str(a.binary.resolve()), "python": a.python,
        "catalog": str(repo / "s_session/catalog/signed-catalog.json"),
        "profile": str(repo / "s_session/profiles/ohlc_integer_tb02a_v1.json"),
        "clock_plan": str(clock), "browser": str(repo / "s_session/browser/index.html"),
        "query_resource_config": str(a.q_resources.resolve()),
        "s_resources": {"max_frame_bytes": "1048576", "read_timeout_ms": "2000", "write_timeout_ms": "1000",
                        "queue_capacity": "32", "max_connections": "48",
                        "response_timeout_ms": str(a.reply_timeout_ms), "audit_cache_source_bytes": "268435456"},
    }
    (work / "config.json").write_text(json.dumps(config, ensure_ascii=False, indent=2) + "\n")
    env = dict(os.environ, PYO3_PYTHON=a.python)
    for k in ("S_CAPTURE_NATIVE_TESTONLY", "S_MANIFEST_WORKSPACE_DIAGNOSTICS"):
        env.pop(k, None)  # 生产构建 + 无 testonly 开关
    launcher = [str(repo / "s_session/launch_s.sh"), "service"]
    control = ["--config", str(work / "config.json"), "--state-dir", str(work / "control")]
    git = lambda *x: subprocess.run(["git", "-C", str(repo), *x], capture_output=True, text=True).stdout.strip()
    run = {"schema": "1488-a5s6-run/1", "binary": str(a.binary), "binary_sha256": sha256_file(a.binary),
           "messages_sha256": MSG_SHA, "clock_sha256": CLOCK_SHA, "repo_head": git("rev-parse", "HEAD"),
           "repo_status_porcelain": git("status", "--porcelain")[-4000:], "reply_timeout_ms": a.reply_timeout_ms,
           "q_resources_sha256": sha256_file(a.q_resources), "q_functional_sha256": sha256_file(a.q_functional),
           "nproc": os.cpu_count(), "host_before": {"loadavg": loadavg(), "top": top_procs()},
           "started_utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())}
    hostlog = HostLog(work / "HOST.jsonl")
    hostlog.start()
    started = subprocess.run(launcher + ["start"] + control, capture_output=True, text=True, env=env)
    run["start_exit"] = started.returncode
    run["start_stdout"], run["start_stderr"] = started.stdout[-4000:], started.stderr[-4000:]
    sampler = None
    try:
        if started.returncode != 0:
            raise RuntimeError("launcher start 失败")
        s_pid = json.loads((work / "control/s.process.json").read_text())["pid"]
        q_pid = json.loads((work / "control/q.process.json").read_text())["pid"]
        run["s_pid"], run["q_pid"] = s_pid, q_pid
        state = {"op": None}
        sampler = Sampler(work / "TRACE.jsonl", s_pid, a.sample_ms, state)
        sampler.start()
        ingest_ok, record = True, None
        with (work / "INGEST.jsonl").open("w") as out:
            for index, raw in enumerate(lines):
                request = sc.decode_frame(raw)
                assert request["schema_revision"] == "s-session/2"
                request["schema_revision"] = "s-session/3"
                s0, q0, h0, l0 = proc(s_pid), proc(q_pid), host_cpu(), loadavg()
                state["op"] = index
                sent = time.monotonic_ns()
                result = sc.exchange(config["socket"], request, timeout_ms=a.reply_timeout_ms, max_frame_bytes=1048576)
                received = time.monotonic_ns()
                state["op"] = None
                s1, q1, h1, l1 = proc(s_pid), proc(q_pid), host_cpu(), loadavg()
                wall = (received - sent) / 1e6
                ds = delta(s0, s1)
                busy = (h1[0] - h0[0]) - (h1[1] - h0[1])
                record = {"index": index, "message_id": request["message_id"], "reply_ms": round(wall, 1),
                          "transport": result["transport"], "s": ds, "q": delta(q0, q1),
                          "s_cpu_ms": (ds["utime_ms"] + ds["stime_ms"]) if ds else None,
                          "s_rss_kb": s1 and s1["rss_kb"], "s_hwm_kb": s1 and s1["hwm_kb"], "s_threads": s1 and s1["threads"],
                          "host_busy_cores": round(busy / max(1, h1[0] - h0[0]) * os.cpu_count(), 2),
                          "load1_before": l0[0], "load1_after": l1[0],
                          "db_bytes": os.path.getsize(config["db"]),
                          "wal_bytes": os.path.getsize(config["db"] + "-wal") if os.path.exists(config["db"] + "-wal") else 0}
                if record["s_cpu_ms"] is not None and wall > 0:
                    record["s_cpu_over_wall"] = round(record["s_cpu_ms"] / wall, 3)
                if result["transport"] == "Received":
                    payload = sc.validate_reply(request, result["response"], producer="S", producer_epoch="1")
                    record.update(kind=payload.get("kind"), publication=payload.get("publication"),
                                  accepted_seq=payload.get("accepted_seq"), receipt_id=payload.get("receipt_id"))
                    if payload.get("kind") != "Committed":
                        record["reply_payload"] = json.dumps(payload, ensure_ascii=False)[:4000]
                else:
                    record["detail"] = result.get("detail")
                out.write(json.dumps(record, ensure_ascii=False) + "\n")
                out.flush()
                print(f"op {index} {wall:.0f}ms cpu {record['s_cpu_ms']}ms syscr {ds and ds.get('syscr')} load {l1[0]}", flush=True)
                if a.ready_probe and result["transport"] == "Received":
                    # final1 实测：冻结 Q3 在 52 代库上 /api/ready 503 QueryBudgetExceeded（verify 5 s 门）；
                    # 每条提交后（下一条发出前，不与摄入重叠）探正式 Q 一次 ready，得 verify 成本随代数的曲线。
                    p0 = proc(q_pid)
                    probe = subprocess.run(["curl", "-s", "-m", "60", "-w", "\n%{http_code} %{time_total}",
                                            f"http://127.0.0.1:{a.port}/api/ready"], capture_output=True, text=True)
                    body, _, tail = probe.stdout.rpartition("\n")
                    code, _, secs = tail.partition(" ")
                    kind = None
                    try:
                        parsed = json.loads(body)
                        kind = (parsed.get("payload") or parsed).get("error") or (parsed.get("payload") or parsed).get("kind")
                    except ValueError:
                        pass
                    rp = {"index": index, "generation": (record.get("publication") or {}).get("generation"), "http": code,
                          "seconds": float(secs) if secs else None, "kind": json.dumps(kind)[:200] if kind else None,
                          "q": delta(p0, proc(q_pid))}
                    with (work / "READY-PROBE.jsonl").open("a") as rpf:
                        rpf.write(json.dumps(rp, ensure_ascii=False) + "\n")
                    print(f"  ready g={rp['generation']} {code} {secs}s {rp['kind']}", flush=True)
                    if rp["generation"] in a.formal_prefix:
                        # 前缀库正式核对：冻结 Q3 + fixture 客户端档，在 head=G 时核 g1 与 g=G（AsKnown）+ 当前
                        run.setdefault("a5_formal_prefix", {})[rp["generation"]] = node_client(
                            repo, tools, a.port, f"1,{rp['generation']}", work / f"A5-formal-prefix-g{rp['generation']}.jsonl",
                            a.oracle, "fixture", True, work / "a5-client.log")
                if result["transport"] != "Received" or payload.get("kind") != "Committed":
                    ingest_ok = False
                    break
        sampler.stop.set()
        run["ingest_complete"] = ingest_ok
        run["head_generation"] = int(record["publication"]["generation"]) if ingest_ok else None
        run["host_after_ingest"] = {"loadavg": loadavg(), "top": top_procs()}
        if ingest_ok:
            run["a5_formal"] = node_client(repo, tools, a.port, a.formal_gens, work / "A5-formal.jsonl", a.oracle,
                                           "fixture", True, work / "a5-client.log")
            # tb03a（经济域消费者）入口探测：Q3 结构入口不承载 economic（s_service_control.py:632-633）
            probe = subprocess.run(["curl", "-s", "-o", "/dev/null", "-w", "%{http_code}",
                                    f"http://127.0.0.1:{a.port}/api/economic/ready"], capture_output=True, text=True)
            run["tb03a_economic_ready_http"] = probe.stdout
    except Exception as exc:
        run["error"] = f"{type(exc).__name__}: {exc}"[:4000]
    finally:
        if sampler:
            sampler.stop.set()
        # 冒烟实测：Q 收 SIGTERM 后退出可超过 startup_timeout_ms（10 s），launcher 不升级信号即报错且不再停 S；
        # 故按正式入口重发 stop（已停的服务幂等跳过），逐次留痕。
        run["stop_attempts"] = []
        for _ in range(12):
            t0 = time.monotonic()
            stopped = subprocess.run(launcher + ["stop"] + control, capture_output=True, text=True, env=env)
            run["stop_attempts"].append({"exit": stopped.returncode, "seconds": round(time.monotonic() - t0, 1),
                                         "stdout": stopped.stdout[-2000:], "stderr": stopped.stderr[-2000:]})
            if stopped.returncode == 0:
                break
        run["stop_exit"] = stopped.returncode
        run["stop_stdout"], run["stop_stderr"] = stopped.stdout[-4000:], stopped.stderr[-4000:]
    # 诊断：停服后功能期限档 query-serve，对全部代逐代核对（不是验收档）
    if run.get("ingest_complete"):
        qlog = (work / "query-serve-functional.log").open("ab")
        qp = subprocess.Popen([config["binary"], "query-serve", "--protocol-revision", "s-session/3", "--db", config["db"],
                               "--resource-config", str(a.q_functional.resolve()), "--producer-epoch", "1",
                               "--host", "127.0.0.1", "--port", a.port], stdout=qlog, stderr=subprocess.STDOUT, env=env)
        try:
            for _ in range(200):
                if subprocess.run(["curl", "-sf", "-o", "/dev/null", f"http://127.0.0.1:{a.port}/api/ready"]).returncode == 0:
                    break
                time.sleep(0.25)
            run["a5_functional"] = node_client(repo, tools, a.port, a.functional_gens, work / "A5-functional.jsonl",
                                               a.oracle, a.functional_limits, not a.functional_no_current, work / "a5-client.log")
        finally:
            qp.terminate()
            qp.wait(timeout=30)
            run["functional_q_returncode"] = qp.returncode
    hostlog.stop.set()
    run["finished_utc"] = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
    db = Path(config["db"])
    run["db_sha256"] = sha256_file(db) if db.exists() else None
    run["ok"] = bool(run.get("ingest_complete") and run.get("stop_exit") == 0 and "error" not in run)
    (work / "RUN.json").write_text(json.dumps(run, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps({k: run.get(k) for k in ("ingest_complete", "head_generation", "a5_formal", "a5_functional", "stop_exit", "ok", "error")}, ensure_ascii=False))
    if not run["ok"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
