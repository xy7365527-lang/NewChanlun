#!/usr/bin/env python3
"""#1716：tb03b-finance/4 新 cap 版本经 E/B 正式服务进程与 Unix socket 的实际消费与新进程恢复。

输入由 Rust 正控 `content_domain_activated_cap_consumed_through_formal_sockets` 写出：manifest 已冻结真实 S
会话读法，三域库已以生产 handler 完成开局与既有责任导入。本驱动经 economic_service_control 起停 E、两 B，
经 socket 发 ActivateCapPolicy 与 ChongAdvance；故障只经 manifest 的 TestOnly marker 定位后 SIGKILL 实际 PID。
"""
import argparse
from concurrent.futures import ThreadPoolExecutor
import json
from pathlib import Path
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
sys.path.insert(0, str(ROOT / "tests"))
from s_socket_client import decode_frame, digest, exchange, validate_reply
from tb03b_oracle import cap_activation as independent_cap_oracle, read_domain

DOMAINS = ("E", "B:alpha", "B:beta")


def save(path, value):
    with path.open("x") as stream:
        json.dump(value, stream, ensure_ascii=False, indent=2)
        stream.write("\n")


def receipts_of(domain, op):
    return {k: v for k, v in domain["receipts"].items() if k.startswith("tb03b-finance/4:" + op + ":")}


class Run:
    def __init__(self, case, binaries, output):
        self.case, self.output = case, output
        output.mkdir()
        self.log = (output / "commands.jsonl").open("x")
        self.counter = 0
        self.started = []
        self.elapsed, self.attempts = {}, []
        self.manifest = decode_frame(Path(case["manifest"]).read_bytes())
        self.data = decode_frame(Path(self.manifest["finance"]["input_path"]).read_bytes())
        auth = self.manifest["authorized_sources"][0]
        self.source = {k: auth[k] for k in ("source_namespace", "source_epoch", "producer_id", "producer_epoch")}
        self.epochs = {d: "1" for d in DOMAINS}
        save(output / "economic-launcher.json", {
            "schema_revision": "economic-launcher/1", "manifest": case["manifest"],
            "manifest_hash": digest(self.manifest), "python": sys.executable,
            "e_binary": str(binaries / "e_economic_session"), "b_binary": str(binaries / "b_chong_session"),
            "state_root": str(output / "control"), "startup_timeout_ms": "30000",
            "probe_source": self.source, "expected_epochs": dict(self.epochs)})

    def node(self, domain):
        if domain == "E":
            return self.manifest["e"]
        return next(b for b in self.manifest["chongs"] if "B:" + b["chong_id"] == domain)

    def domains(self):
        return {d: read_domain(self.node(d)["db"]) for d in DOMAINS}

    def control(self, action, domain, *extra):
        self.counter += 1
        label = str(self.counter).zfill(3)
        args = [sys.executable, str(ROOT / "economic_service_control.py"), action,
                "--config", str(self.output / "economic-launcher.json"), "--domain", domain,
                "--epoch", self.epochs[domain], "--e-epoch", self.epochs["E"], *extra]
        done = subprocess.run(args, capture_output=True, timeout=120)
        (self.output / (label + ".stdout")).write_bytes(done.stdout)
        (self.output / (label + ".stderr")).write_bytes(done.stderr)
        self.log.write(json.dumps({"command": args, "exit_code": done.returncode, "stdout": label + ".stdout"}) + "\n")
        self.log.flush()
        if done.returncode:
            raise RuntimeError("控制命令失败，原件 " + str(self.output / label) + "：" + done.stderr.decode()[-400:])
        return json.loads(done.stdout)

    def send(self, domain, message, payload, name):
        request = {"schema_revision": "economic-session/2", "session_id": self.manifest["session_id"],
                   "session_generation": self.manifest["session_generation"], **self.source,
                   "message_id": message, "payload_hash": digest(payload), "causal_refs": [], "payload": payload}
        save(self.output / (name + ".request.json"), request)
        started = time.monotonic()
        result = exchange(self.node(domain)["socket"], request, timeout_ms=60000,
                          max_frame_bytes=int(self.manifest["resources"]["max_frame_bytes"]))
        self.elapsed[name] = str(int((time.monotonic() - started) * 1000))
        save(self.output / (name + ".response.json"), result)
        if result["transport"] != "Received":
            return {"kind": "DeliveryUnknown", "transport": result["transport"]}
        return validate_reply(request, result["response"], producer=domain, producer_epoch=self.epochs[domain])

    def op(self, domain, payload, name, retry=True):
        """以 E 协调器同一内部消息身份（`tb03b-<op>-<id>`）直发 owner socket；回包超期（3000 ms 期限只结束连接）
        时按原身份重送一次取原结果，两次读数都留在 attempts。"""
        identity = payload.get("decision_id", payload.get("slot_id"))
        message = "tb03b-" + payload["op"] + "-" + identity
        reply = self.send(domain, message, payload, name)
        if reply.get("kind") == "DeliveryUnknown" and retry:
            self.attempts.append({"name": name, "first": reply, "elapsed_ms": self.elapsed[name],
                                  "receipts_after_first": self.counts()})
            reply = self.send(domain, message, payload, name + "-resend")
        return reply

    def prepare(self, name):
        plan = next(p for p in self.data["plans"] if p["decision_id"] == name)
        owner = plan["owner"]
        return self.op(owner, {"op": "PrepareDecision", "decision_id": name, "plan_hash": digest(plan),
                               "writer_epoch": self.epochs[owner]}, "prepare-" + name)

    def reserve_payload(self):
        slot = self.data["slots"][0]
        return {"op": "Reserve", "slot_id": slot["slot_id"], "request_set_hash": digest(slot["applicants"]),
                "writer_epoch": self.epochs["E"]}

    def commit_payload(self, grant):
        return {"op": "CommitDecision", "decision_id": grant["decision_id"], "plan_hash": grant["plan_hash"],
                "grant_id": grant["grant_id"], "writer_epoch": self.epochs[grant["owner"]]}

    def advance(self, name):
        slot = self.data["slots"][0]
        return self.send("E", name, {"op": "ChongAdvance", "slot_id": slot["slot_id"],
                                     "request_set_hash": digest(slot["applicants"]),
                                     "writer_epoch": self.epochs["E"], "owner_epochs": dict(self.epochs)}, name)

    def counts(self):
        domains = self.domains()
        return {"E:Reserve": len(receipts_of(domains["E"], "Reserve")),
                **{d + ":Prepare": len(receipts_of(domains[d], "PrepareDecision")) for d in DOMAINS[1:]},
                **{d + ":Commit": len(receipts_of(domains[d], "CommitDecision")) for d in DOMAINS[1:]}}

    def wait_marker(self, future):
        marker = Path(self.case["fault"]["marker_path"])
        deadline = time.monotonic() + 60
        while True:
            if marker.exists():
                try:
                    return decode_frame(marker.read_bytes())
                except ValueError:
                    pass
            if future.done():
                raise RuntimeError("请求未达故障缝：" + json.dumps(future.result(), ensure_ascii=False))
            if time.monotonic() >= deadline:
                raise TimeoutError("故障 marker 超时")
            time.sleep(0.02)

    def kill_and_restart(self, domain, submit):
        fault = self.case["fault"]
        with ThreadPoolExecutor(max_workers=1) as pool:
            future = pool.submit(submit)
            marker = self.wait_marker(future)
            stopped = self.control("stop", domain, "--signal", "KILL")
            delivery = future.result()
        assert stopped["result"]["signal"] == "SIGKILL" and str(stopped["result"]["pid"]) == str(marker["pid"]), (stopped, marker)
        assert (marker["message_id"], marker["phase"]) == (fault["message_id"], fault["phase"]), marker
        before = read_domain(self.node(domain)["db"])
        recovered = self.control("recover", domain)
        self.epochs[domain] = "2"
        self.control("start", domain)
        after = read_domain(self.node(domain)["db"])
        assert before["commits"] == after["commits"] and before["receipts"] == after["receipts"], "恢复改写了持久历史"
        assert after["meta"]["writer_epoch"] == "2", after["meta"]
        return {"marker": marker, "kill": stopped["result"], "delivery": delivery,
                "recover": recovered["result"]["reply"], "receipts_after_kill": self.counts()}

    def run(self):
        cap, fault = self.case["cap"], self.case["fault"]
        target = fault["domain"] if fault else None
        for domain in DOMAINS:
            self.started.append(domain)
            self.control("start", domain)
        evidence = {"prepared": {}}
        for name in ("alpha", "beta"):
            prepared = self.prepare(name)
            assert prepared.get("kind") == "PreparedReceipt", prepared
            evidence["prepared"][name] = prepared
        activated = self.send("E", "ActivateCapPolicy-pre", self.case["activation"], "activate")
        assert activated.get("kind") == "CapPolicyActivatedReceipt", activated
        to = activated["record"]["to"]
        assert (to["policy_version"], to["cap"], activated["record"]["liability_at_activation"]) == ("2", cap, "20"), activated
        evidence["activation"] = activated
        original = self.reserve_payload()
        if target == "E":
            evidence["fault"] = self.kill_and_restart("E", lambda: self.op("E", original, "reserve", retry=False))
            stored = receipts_of(read_domain(self.manifest["e"]["db"]), "Reserve")
            assert len(stored) == 1, stored
            reserved = self.op("E", original, "reserve-original-after-recover")
            assert reserved == next(iter(stored.values())), "新进程按原请求取回原 Reserve 回执"
        else:
            reserved = self.op("E", original, "reserve")
        assert reserved.get("kind") == "ReserveReceipt", reserved
        record = reserved["record"]
        assert record["cap_policy"] == to and record["required_total"] == "90", record
        if cap == "90":
            assert (record["state"], record["retained_total"]) == ("Reserved", "90"), record
            assert sorted(g["owner"] for g in record["grants"]) == ["B:alpha", "B:beta"], record
        else:
            assert (record["state"], record["retained_total"], record["grants"]) == ("AwaitingAllocationPolicy", "20", []), record
            assert record["waiting_for"] == self.data["policy"]["allocation_policy_id"], record
        evidence["reserve"] = reserved
        commits = {}
        for grant in record["grants"]:
            owner, payload = grant["owner"], self.commit_payload(grant)
            if owner == target:
                evidence["fault"] = self.kill_and_restart(owner, lambda: self.op(owner, payload, "commit-" + grant["decision_id"], retry=False))
                stored = receipts_of(read_domain(self.node(owner)["db"]), "CommitDecision")
                assert len(stored) == 1, stored
                reply = self.op(owner, payload, "commit-original-after-recover-" + grant["decision_id"])
                assert reply == next(iter(stored.values())), "新进程按原请求取回原 Commit 回执"
            else:
                reply = self.op(owner, payload, "commit-" + grant["decision_id"])
            assert reply.get("kind") == "LocalCommitReceipt" and reply["record"]["grant"]["original"] == grant, (reply, grant)
            commits[owner] = reply
        evidence["commits"] = commits
        before_dup = self.domains()
        coordinated = self.advance("chong-advance")
        if cap == "90":
            assert coordinated.get("kind") == "ChongAdvance" and coordinated["reserve_receipt"] == reserved, coordinated
            assert {c["owner"]: (c["status"], c["receipt"]) for c in coordinated["commits"]} == {
                o: ("Committed", r) for o, r in commits.items()}, coordinated["commits"]
        else:
            assert coordinated == reserved, coordinated
        after_dup = self.domains()
        for d in DOMAINS:
            assert after_dup[d]["commits"] == before_dup[d]["commits"] and after_dup[d]["receipts"] == before_dup[d]["receipts"], d
        evidence["chong_advance"] = coordinated
        evidence["ledger"] = self.ledger(cap)
        evidence["elapsed_ms"] = self.elapsed
        evidence["delivery_unknown_resends"] = self.attempts
        return evidence

    def ledger(self, cap):
        domains = self.domains()
        e_image = domains["E"]["commits"][-1]["image"]["finance"]
        counts = self.counts()
        liability = sum(int(r["amount"]) for r in e_image["responsibilities"])
        oracle = independent_cap_oracle(self.case["manifest"])
        expected = {"E:Reserve": 1, "B:alpha:Prepare": 1, "B:beta:Prepare": 1,
                    "B:alpha:Commit": 1 if cap == "90" else 0, "B:beta:Commit": 1 if cap == "90" else 0}
        assert counts == expected, counts
        assert liability == (90 if cap == "90" else 20), liability
        assert oracle["final_cap_version"] == "2" and oracle["final_cap"] == cap, oracle
        assert oracle["final_liability"] == str(liability) and len(oracle["reserves"]) == 1, oracle
        assert e_image["cap_policy"]["policy_version"] == "2" and e_image["cap_policy"]["cap"] == cap, e_image["cap_policy"]
        return {"receipt_counts": counts, "e_liability": str(liability), "oracle": oracle,
                "writer_epochs": {d: domains[d]["meta"]["writer_epoch"] for d in DOMAINS}}

    def close(self):
        for domain in reversed(self.started):
            try:
                self.control("stop", domain)
            except (RuntimeError, ValueError, OSError, subprocess.SubprocessError):
                pass
        self.log.close()


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--case", type=Path, required=True)
    p.add_argument("--binaries", type=Path, required=True)
    p.add_argument("--output", type=Path, required=True)
    args = p.parse_args()
    case = decode_frame(args.case.read_bytes())
    output = args.output.resolve()
    run = None
    try:
        run = Run(case, args.binaries.resolve(), output)
        outcome = {"status": "passed", **run.run()}
    except (AssertionError, ValueError, RuntimeError, TimeoutError, OSError, KeyError, subprocess.SubprocessError) as error:
        outcome = {"status": "failed", "error": repr(error)}
    finally:
        if run is not None:
            run.close()
    output.mkdir(exist_ok=True)
    save(output / "RESULT.json", {"issue": 1716, "cap": case["cap"], "fault": case["fault"], **outcome})
    print(json.dumps({"status": outcome["status"], "error": outcome.get("error")}, ensure_ascii=False))
    return 0 if outcome["status"] == "passed" else 1


if __name__ == "__main__":
    raise SystemExit(main())
