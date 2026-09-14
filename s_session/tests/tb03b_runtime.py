#!/usr/bin/env python3
"""#1394：正式 S/E/两 B/Q/浏览器验收。原始证据只写 --output。

--seed 必须提供真实已核 TB-03-A Voice 权威、原开局和正式 S 输入；本驱动不制造 γ 独评。
普通调用只发送 ChongAdvance，三阶段由 E 模块协调。故障只通过已配置 marker 停止实际 owner。
"""
import argparse
from concurrent.futures import ThreadPoolExecutor
import copy
import hashlib
import json
from pathlib import Path
import socket
import subprocess
import sys
import time
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
from s_socket_client import canonical, decode_frame, digest, exchange, validate_reply
from tb03b_oracle import check as independent_check, read_domain

FAULTS = [("PrepareDecision", "alpha", "B:alpha"), ("Reserve", "slot", "E"),
          ("CommitDecision", "alpha", "B:alpha"), ("ImportResponsibility", "existing20", "E"),
          ("AbortDecision", "alpha", "B:alpha"), ("CloseApplicants", "slot", "E")]


def save(path, value):
    with path.open("x") as stream:
        json.dump(value, stream, ensure_ascii=False, indent=2)
        stream.write("\n")


def pinned(seed, item):
    path = Path(item["path"])
    raw = path.read_bytes()
    if hashlib.sha256(raw).hexdigest() != item["sha256"]:
        raise ValueError("种子具名原件 hash 不符：" + str(path))
    return decode_frame(raw)


def envelope(manifest, source, message, payload):
    return {"schema_revision": "economic-session/1", "session_id": manifest["session_id"],
            "session_generation": manifest["session_generation"], **source, "message_id": message,
            "payload_hash": digest(payload), "causal_refs": [], "payload": payload}


def probe(output, chrome):
    results = []
    for kind, family, address in (("unix", socket.AF_UNIX, str(output / "probe.sock")),
                                  ("http", socket.AF_INET, ("127.0.0.1", 0))):
        with socket.socket(family) as connection:
            try:
                connection.bind(address)
                results.append({"kind": kind, "status": "bound", "address": str(connection.getsockname())})
            except OSError as error:
                results.append({"kind": kind, "status": "blocked", "error": str(error), "errno": error.errno})
    command = [chrome, "--headless", "--disable-background-networking", "--disable-component-update",
               "--disable-sync", "--no-first-run", "--no-default-browser-check",
               "--user-data-dir=" + str(output / "probe-profile"), "--dump-dom", "about:blank"]
    try:
        done = subprocess.run(command, capture_output=True, timeout=20,
                              env={"PATH": "/usr/bin:/bin", "TMPDIR": str(output)})
        (output / "browser.stdout").write_bytes(done.stdout)
        (output / "browser.stderr").write_bytes(done.stderr)
        results.append({"kind": "browser", "command": command, "exit_code": done.returncode,
                        "status": "started" if done.returncode == 0 else "blocked"})
    except (OSError, subprocess.SubprocessError) as error:
        results.append({"kind": "browser", "command": command, "status": "blocked", "error": str(error)})
    save(output / "preflight.json", results)
    return all(r["status"] != "blocked" for r in results)


class Run:
    def __init__(self, seed_path, output, binaries, cap, fault=None, scenario="normal"):
        self.output, self.fault, self.scenario = output, fault, scenario
        output.mkdir()
        self.log = (output / "commands.jsonl").open("x")
        self.counter = 0
        self.started = []
        self.seed = decode_frame(seed_path.read_bytes())
        if self.seed.get("schema_revision") != "tb03b-runtime-seed/1":
            raise ValueError("缺 tb03b-runtime-seed/1 具名输入")
        if self.seed.get("contract_revision") != "efc1ddc4d6c015f4f8c6d44f904d704a88308b46":
            raise ValueError("种子未绑定本票已签合同")
        self.s = pinned(self.seed, self.seed["s_launcher"])
        self.manifest = pinned(self.seed, self.seed["economic_manifest"])
        self.data = pinned(self.seed, self.seed["finance_input"])
        if {b["chong_id"] for b in self.manifest["chongs"]} != {"alpha", "beta"}:
            raise ValueError("本具名验收域要求 alpha/beta 两个实际 owner")
        if not self.manifest.get("voice_binding"):
            raise ValueError("缺真实 TB-03-A Voice 权威，禁止生成假独评")
        if self.manifest["clock"]["kind"] != "TestOnly":
            raise ValueError("本脚本只运行已冻结 TestOnly 时钟")
        if self.manifest["session_id"] != self.s["session_id"] or self.manifest["session_generation"] != self.s["session_generation"]:
            raise ValueError("S/E/B 必须属于同一具名会话")
        # 冻结来源文件；不改写原签收证据、不把模板未知字段补默认值。
        self.data["policy"]["cap"] = cap
        self.data["policy"]["policy_id"] = self.seed["test_policy_ids"][cap]
        save(output / "finance-input.json", self.data)
        for domain, node in [("E", self.manifest["e"])] + [("B:" + b["chong_id"], b) for b in self.manifest["chongs"]]:
            short = "e" if domain == "E" else domain[2:]
            node["db"], node["socket"] = str(output / (short + ".db")), str(output / (short + ".sock"))
        self.s_config_path = output / "s-launcher.json"
        self.query_revision = 0
        self.epochs = {"E": "1", "B:alpha": "1", "B:beta": "1"}
        self.s.update(db=str(output / "s.db"), socket=str(output / "s.sock"),
                      binary=str(binaries / "s_structure_session"), browser=str(ROOT / "browser/index.html"), python=sys.executable)
        self.manifest["finance"] = {"schema_revision": "tb03b-finance/1", "input_path": str(output / "finance-input.json"),
            "input_sha256": hashlib.sha256((output / "finance-input.json").read_bytes()).hexdigest(),
            "coordinator_source": self.seed["coordinator_source"], "structure": {
                "binary": self.s["binary"], "binary_sha256": hashlib.sha256(Path(self.s["binary"]).read_bytes()).hexdigest(),
                "db": self.s["db"], "session_id": self.s["session_id"],
                "session_generation": self.s["session_generation"], "writer_epoch": self.s["writer_epoch"]}}
        self.manifest["clock"]["plan"]["messages"].update(self.seed["financial_clock_messages"])
        if fault:
            if scenario not in self.seed["scenario_clock_plans"]:
                raise ValueError("故障域缺显式时钟计划；不得把恢复后的获知回填原时刻：" + scenario)
            self.manifest["clock"]["plan"] = self.seed["scenario_clock_plans"][scenario]
        if fault:
            self.manifest["test_only_fault"] = {"message_id": "tb03b-" + fault[0] + "-" + fault[1],
                "phase": fault[3], "marker_path": str(output / "fault-marker.json")}
            if fault[0] in ("AbortDecision", "CloseApplicants"):
                self.manifest["test_only_fault"] = [self.manifest["test_only_fault"],
                    {"message_id": "tb03b-Reserve-slot", "phase": "after_commit",
                     "marker_path": str(output / "reserved-checkpoint.json")}]
        else:
            self.manifest.pop("test_only_fault", None)
        save(output / "manifest.json", self.manifest)
        self.launcher = {"schema_revision": "economic-launcher/1", "manifest": str(output / "manifest.json"),
            "manifest_hash": digest(self.manifest), "python": sys.executable,
            "e_binary": str(binaries / "e_economic_session"), "b_binary": str(binaries / "b_chong_session"),
            "state_root": str(output / "control-economic"), "startup_timeout_ms": self.seed["startup_timeout_ms"],
            "probe_source": self.seed["query_source"], "expected_epochs": self.epochs.copy()}
        save(output / "economic-launcher.json", self.launcher)
        self.q = {"schema_revision": "economic-query/1", "manifest": str(output / "manifest.json"),
            "manifest_hash": digest(self.manifest), "query_source": self.seed["query_source"],
            "producer_epoch": "1", "expected_epochs": self.epochs.copy(), "resources": self.seed["economic_query_resources"]}
        save(output / "economic-query.json", self.q)
        self.s["economic_query_config"] = str(output / "economic-query.json")
        save(output / "s-launcher.json", self.s)
        bindings = [{"path": str(seed_path), "sha256": hashlib.sha256(seed_path.read_bytes()).hexdigest()}]
        for path in [output / "manifest.json", output / "finance-input.json", output / "s-launcher.json"] + list(binaries / name for name in ("s_structure_session", "e_economic_session", "b_chong_session")):
            bindings.append({"path": str(path), "sha256": hashlib.sha256(path.read_bytes()).hexdigest()})
        save(output / "source-manifest.json", {"issue": 1394, "scope": "TestOnly formal run", "bindings": bindings,
            "policy": self.data["policy"], "fault": fault, "not_independent_review": True})

    def command(self, args, check=True):
        self.counter += 1
        label = str(self.counter).zfill(4)
        with (self.output / (label + ".stdout")).open("xb") as out, (self.output / (label + ".stderr")).open("xb") as error:
            done = subprocess.run(args, stdout=out, stderr=error, timeout=60)
        entry = {"command": args, "exit_code": done.returncode, "stdout": label + ".stdout", "stderr": label + ".stderr"}
        self.log.write(json.dumps(entry) + "\n"); self.log.flush()
        if check and done.returncode:
            raise RuntimeError("正式命令失败，原件 " + str(self.output / label))
        return decode_frame((self.output / (label + ".stdout")).read_bytes()) if check else entry

    def economic_control(self, action, domain, *extra, check=True):
        return self.command([sys.executable, str(ROOT / "economic_service_control.py"), action,
            "--config", str(self.output / "economic-launcher.json"), "--domain", domain,
            "--epoch", self.epochs[domain], "--e-epoch", self.epochs["E"], *extra], check=check)

    def s_control(self, action, check=True):
        return self.command([sys.executable, str(ROOT / "s_service_control.py"), action,
            "--config", str(self.s_config_path), "--state-dir", str(self.output / "control-s")], check=check)

    def send(self, domain, request, name):
        save(self.output / (name + ".request.json"), request)
        node = self.manifest["e"] if domain == "E" else next(b for b in self.manifest["chongs"] if "B:" + b["chong_id"] == domain)
        result = exchange(node["socket"], request, timeout_ms=int(self.seed["business_timeout_ms"]),
                          max_frame_bytes=int(self.manifest["resources"]["max_frame_bytes"]))
        save(self.output / (name + ".response.json"), result)
        if result["transport"] != "Received":
            return {"kind": "DeliveryUnknown", "transport": result}
        return validate_reply(request, result["response"], producer=domain, producer_epoch=self.epochs[domain])

    def request(self, op, fields, *, message=None):
        identity = fields.get("decision_id", fields.get("slot_id", fields.get("responsibility_id", "query")))
        return envelope(self.manifest, self.seed["coordinator_source"], message or "tb03b-" + op + "-" + identity, {"op": op, **fields})

    def advance(self, label="advance"):
        slot = self.data["slots"][0]
        return self.send("E", self.request("ChongAdvance", {"slot_id": slot["slot_id"],
            "request_set_hash": digest(slot["applicants"]), "writer_epoch": self.epochs["E"],
            "owner_epochs": self.epochs.copy()}, message=label), label)

    def http(self, payload, label):
        req = envelope(self.manifest, self.seed["query_source"], label, payload)
        raw = canonical(req)
        request = urllib.request.Request("http://127.0.0.1:" + self.s["port"] + "/api/economic/" + payload["op"], data=raw, headers={"Content-Type": "application/json"})
        with urllib.request.urlopen(request, timeout=30) as response:
            data = response.read(int(self.q["resources"]["max_reply_bytes"]) + 1)
        save(self.output / (label + ".request.json"), req)
        reply = decode_frame(data)
        save(self.output / (label + ".response.json"), reply)
        return validate_reply(req, reply, producer="Q", producer_epoch="1")

    def setup(self):
        self.started.append("S")
        self.s_control("start-s")
        for i, request in enumerate(pinned(self.seed, self.seed["s_requests"])):
            result = exchange(self.s["socket"], request, timeout_ms=int(self.seed["business_timeout_ms"]),
                              max_frame_bytes=int(self.s["s_resources"]["max_frame_bytes"]))
            save(self.output / ("s-input-" + str(i) + ".json"), {"request": request, "result": result})
            if result["transport"] != "Received":
                raise RuntimeError("S 正式输入 DeliveryUnknown")
            reply = validate_reply(request, result["response"], producer="S", producer_epoch=self.s["writer_epoch"])
            if reply.get("kind") != "Committed":
                raise RuntimeError("S 正式输入未提交")
        basis = self.command([self.s["binary"], "financial-basis", "--db", self.s["db"]])
        save(self.output / "actual-s-basis.json", basis)
        for plan in self.data["plans"]:
            obj = next(x for x in basis["snapshot"]["objects"] if x["object_id"] == plan["s_ref"]["object_id"])
            if digest(obj) != plan["s_ref"]["object_hash"]:
                raise ValueError("固定计划未引用本次真实 S 原结构")
        for domain in self.epochs:
            self.started.append(domain)
            self.economic_control("start", domain)
        for i, entry in enumerate(pinned(self.seed, self.seed["setup_requests"])):
            if entry["request"]["payload"]["op"] not in ("CaptureExternal", "ApplyAllocation", "BindVoice"):
                raise ValueError("前置输入不得预写本票财务成功")
            result = self.send(entry["domain"], entry["request"], "setup-" + str(i))
            if result.get("kind") in ("Error", "DeliveryUnknown"):
                raise RuntimeError("原开局/Voice 输入失败：" + str(result))
        if not self.fault or self.fault[0] != "ImportResponsibility":
            result = self.send("E", self.request("ImportResponsibility", {"responsibility_id": self.data["baseline"]["responsibility_id"], "writer_epoch": "1"}), "import-responsibility")
            if result.get("kind") != "ResponsibilityImportedReceipt":
                raise RuntimeError("既有责任尚未真实导入")
        self.started.append("Q")
        self.s_control("start-q")

    def fault_run(self):
        op, identity, domain, phase = self.fault
        if op in ("AbortDecision", "CloseApplicants"):
            # TestOnly 故障调度先中断 E 已增责/尚未投递 Commit 的真实窗口。
            with ThreadPoolExecutor(max_workers=1) as pool:
                initial = pool.submit(self.advance, "checkpoint-advance")
                self.wait_marker("reserved-checkpoint.json", initial)
                self.economic_control("stop", "E", "--signal", "KILL")
                save(self.output / "checkpoint-delivery.json", initial.result())
            self.economic_control("recover", "E")
            self.epochs["E"] = "2"
            self.economic_control("start", "E")
            if op == "AbortDecision":
                plan = next(p for p in self.data["plans"] if p["decision_id"] == identity)
                fields = {"decision_id": identity, "plan_hash": digest(plan), "writer_epoch": self.epochs[domain],
                          "reason": "#1394/TestOnly-explicit-abort-no-release"}
            else:
                plan = next(p for p in self.data["plans"] if p["owner"] == "B:alpha")
                fields = {"slot_id": identity, "writer_epoch": self.epochs[domain],
                          "closed": [{k: plan[k] for k in ("owner", "applicant_epoch", "applicant_generation")}],
                          "reason": "#1394/TestOnly-supervised-applicant-closure-no-release"}
            callback = lambda: self.send(domain, self.request(op, fields), "terminal-fault")
        elif op == "ImportResponsibility":
            callback = lambda: self.send("E", self.request(op, {"responsibility_id": identity, "writer_epoch": "1"}), "import-fault")
        else:
            callback = self.advance
        with ThreadPoolExecutor(max_workers=1) as pool:
            future = pool.submit(callback)
            self.wait_marker("fault-marker.json", future)
            stopped = self.economic_control("stop", domain, "--signal", "KILL")
            marker = decode_frame((self.output / "fault-marker.json").read_bytes())
            assert stopped["result"]["state"] == "stopped" and stopped["result"]["signal"] == "SIGKILL"
            assert str(stopped["result"]["pid"]) == str(marker["pid"])
            save(self.output / "kill-receipt.json", stopped)
            node = self.manifest["e"] if domain == "E" else next(b for b in self.manifest["chongs"] if "B:" + b["chong_id"] == domain)
            before = read_domain(node["db"])
            save(self.output / "after-kill-domain.json", before)
            save(self.output / "delivery-outcome.json", future.result())
        self.economic_control("recover", domain)
        self.epochs[domain] = str(int(self.epochs[domain]) + 1)
        self.economic_control("start", domain)
        after = read_domain(node["db"])
        assert before["commits"] == after["commits"] and before["receipts"] == after["receipts"]
        assert after["meta"]["writer_epoch"] == self.epochs[domain]
        existed = any(r.get("receipt_key", "").startswith("tb03b-finance/1:" + op + ":") for r in after["receipts"].values())
        assert existed == (phase == "after_commit")
        save(self.output / "fault-oracle.json", {"op": op, "phase": phase, "original_receipt_exists": existed,
            "persistent_history_unchanged_on_recovery": True, "expected_epoch": self.epochs[domain], "scope": "actual formal owner transaction seam; no completion claim for stale uncommitted applicants"})
        # E 已增责且回执丢失：业务原身份可找回，并可完成原 Prepared。
        if op == "Reserve" and phase == "after_commit":
            if self.scenario == "unknown-common":
                self.economic_control("stop", "E", "--signal", "KILL")
                prior = read_domain(self.manifest["e"]["db"])
                grant = next(g for g in prior["commits"][-1]["image"]["finance"]["slots"][0]["grants"] if g["owner"] == "B:beta")
                renamed = []
                try:
                    for suffix in ("", "-wal", "-shm"):
                        path = Path(self.manifest["e"]["db"] + suffix)
                        if path.exists():
                            unavailable = path.with_name(path.name + ".unavailable")
                            path.rename(unavailable)
                            renamed.append((path, unavailable))
                    payload = {"decision_id": grant["decision_id"], "plan_hash": grant["plan_hash"],
                               "grant_id": grant["grant_id"], "writer_epoch": self.epochs["B:beta"]}
                    result = self.send("B:beta", self.request("CommitDecision", payload), "unknown-common-commit")
                    assert result.get("kind") == "Error" and "StorageUnavailable" in result["error"]
                finally:
                    for path, unavailable in renamed:
                        unavailable.rename(path)
                after = read_domain(self.manifest["e"]["db"])
                assert after["commits"] == prior["commits"] and after["receipts"] == prior["receipts"]
                save(self.output / "oracle.json", independent_check(self.output / "manifest.json", "reserved"))
                save(self.output / "common-unknown.json", {"new_commit_refused": True, "e_history_and_responsibility_unchanged": True, "result": result})
            else:
                if self.scenario == "unavailable-alpha":
                    self.economic_control("stop", "B:alpha", "--signal", "KILL")
                result = self.advance("recover-advance")
                save(self.output / "resumed-business.json", result)
                save(self.output / "oracle.json", independent_check(self.output / "manifest.json", "unavailable_alpha" if self.scenario == "unavailable-alpha" else "committed"))
                self.restart_query()
                self.http({"op": "snapshot", "cuts": None, "as_known_ns": None, "snapshot_token": None}, "recovered-public")

    def wait_marker(self, filename, future):
        deadline = time.monotonic() + int(self.seed["business_timeout_ms"]) / 1000
        while not (self.output / filename).exists():
            if future.done():
                raise RuntimeError("请求未达故障缝：" + str(future.result()))
            if time.monotonic() >= deadline:
                raise TimeoutError("正式故障 marker 超时")
            time.sleep(0.02)

    def restart_query(self):
        self.s_control("stop-q")
        self.query_revision += 1
        self.q["expected_epochs"] = self.epochs.copy()
        qpath = self.output / ("economic-query-r" + str(self.query_revision) + ".json")
        save(qpath, self.q)
        self.s["economic_query_config"] = str(qpath)
        self.s_config_path = self.output / ("s-launcher-r" + str(self.query_revision) + ".json")
        save(self.s_config_path, self.s)
        self.s_control("start-q")

    def normal(self, chrome, node):
        old = self.http({"op": "snapshot", "cuts": None, "as_known_ns": None, "snapshot_token": None}, "before")
        result = self.advance()
        if result.get("kind") in ("Error", "DeliveryUnknown"):
            raise RuntimeError("正式业务入口未完成：" + str(result))
        expect = "committed" if self.data["policy"]["cap"] == "100" else "awaiting_policy"
        oracle = independent_check(self.output / "manifest.json", expect)
        save(self.output / "oracle.json", oracle)
        original_receipts = {d: read_domain(node["db"])["receipts"] for d, node in
                             [("E", self.manifest["e"])] + [("B:" + b["chong_id"], b) for b in self.manifest["chongs"]]}
        duplicate = self.advance("duplicate-advance")
        if expect == "committed":
            assert duplicate["reserve_receipt"] == result["reserve_receipt"]
            assert [r["receipt"] for r in duplicate["commits"]] == [r["receipt"] for r in result["commits"]]
            plan = self.data["plans"][0]
            abort = self.send(plan["owner"], self.request("AbortDecision", {"decision_id": plan["decision_id"],
                "plan_hash": digest(plan), "writer_epoch": "1", "reason": "#1394/TestOnly-Abort-Committed-must-fail"}), "abort-committed")
            assert abort.get("kind") == "Error" and "InvalidDomain" in abort["error"]
        for name, epoch, content in (("stale-e", "2", digest(self.data["slots"][0]["applicants"])),
                                     ("identity-conflict", "1", "0" * 64)):
            bad = self.send("E", self.request("ChongAdvance", {"slot_id": self.data["slots"][0]["slot_id"],
                "request_set_hash": content, "writer_epoch": epoch, "owner_epochs": {**self.epochs, "E": epoch}}, message=name), name)
            assert bad.get("kind") == "Error"
            assert ("StaleWriter" if name == "stale-e" else "IdentityConflict") in bad["error"]
        for domain, node in [("E", self.manifest["e"])] + [("B:" + b["chong_id"], b) for b in self.manifest["chongs"]]:
            assert read_domain(node["db"])["receipts"] == original_receipts[domain]
        cuts = {name: view["cut"] for name, view in old["domains"].items()}
        watch = self.http({"op": "watch", "cuts": cuts}, "changes")
        final_cuts = {name: view["next_cut"] for name, view in watch["domains"].items()}
        current = self.http({"op": "snapshot", "cuts": final_cuts, "as_known_ns": None, "snapshot_token": None}, "same-cut")
        for domain, change in watch["domains"].items():
            image = change["changes"][-1]["image"] if change["changes"] else old["domains"][domain]["image"]
            assert image == current["domains"][domain]["image"]
        if expect == "committed":
            for plan in self.data["plans"]:
                self.http({"op": "original", "kind": "decision", "domain_id": plan["owner"], "object_id": plan["decision_id"], "cuts": final_cuts}, "original-" + plan["decision_id"])
            self.command([node, str(ROOT / "tests/tb03b_browser.cjs"), "--chrome", chrome,
                "--url", "http://127.0.0.1:" + self.s["port"] + "/economic.html", "--output", str(self.output / "browser"),
                "--as-known", self.seed["both_prepared_as_known_ns"]])
        return oracle["domain_semantic_hashes"]

    def close(self):
        errors = []
        for domain in reversed(self.started):
            try:
                if domain in ("Q", "S"):
                    self.s_control("stop-q" if domain == "Q" else "stop-s")
                else:
                    self.economic_control("stop", domain)
            except (OSError, ValueError, RuntimeError, subprocess.SubprocessError) as error:
                errors.append({"domain": domain, "error": str(error)})
        save(self.output / "cleanup.json", {"errors": errors})
        self.log.close()
        return errors


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--output", type=Path, required=True)
    p.add_argument("--seed", type=Path)
    p.add_argument("--binaries", type=Path, default=ROOT.parent / "rust/target/debug")
    p.add_argument("--chrome", default="/Applications/Google Chrome.app/Contents/MacOS/Google Chrome")
    p.add_argument("--node", default="node")
    p.add_argument("--probe", action="store_true")
    p.add_argument("--faults", action="store_true")
    args = p.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    if args.probe:
        return 0 if probe(output, args.chrome) else 2
    if args.seed is None:
        save(output / "blocked.json", {"status": "blocked", "missing": "--seed 的真实 S/两 B 开局/已核 Voice 原件、冻结计划及资源时钟；禁止用单位 γ 信任夹具代填"})
        return 2
    outcomes, hashes = [], {}
    scenarios = [("normal", "100", None), ("cap80", "80", None)]
    if args.faults:
        scenarios += [(op + "-" + phase, "100", (op, name, domain, phase))
                      for op, name, domain in FAULTS for phase in ("before_commit", "after_commit")]
    if args.faults:
        scenarios += [(name, "100", ("Reserve", "slot", "E", "after_commit")) for name in ("unavailable-alpha", "unknown-common")]
    for name, cap, fault in scenarios:
        for repetition in range(2):
            run = None
            destination = output / ("r" + str(len(outcomes)).zfill(3))
            try:
                run = Run(args.seed.resolve(), destination, args.binaries.resolve(), cap, fault, name)
                run.setup()
                if fault:
                    run.fault_run()
                else:
                    current = run.normal(args.chrome, args.node)
                    if repetition:
                        assert current == hashes[name], "同输入新进程的原语义/回执不同"
                    else:
                        hashes[name] = current
                outcomes.append({"scenario": name, "repetition": repetition, "status": "passed", "raw_dir": str(destination)})
            except (ValueError, AssertionError, OSError, RuntimeError, TimeoutError, subprocess.SubprocessError) as error:
                outcomes.append({"scenario": name, "repetition": repetition, "status": "failed", "error": str(error), "raw_dir": str(destination)})
                save(destination / "failure.json", outcomes[-1])
            finally:
                if run is not None:
                    cleanup = run.close()
                    if cleanup:
                        outcomes[-1]["cleanup_errors"] = cleanup
                        outcomes[-1]["status"] = "failed"
    save(output / "RESULTS.json", {"issue": 1394, "outcomes": outcomes,
        "scope": "实际正式运行与本票事务缝；完整 TB04/外效和独评不由此脚本认定",
        "independent_review": "NotPerformed"})
    return 0 if all(r["status"] == "passed" for r in outcomes) else 1


if __name__ == "__main__":
    raise SystemExit(main())
