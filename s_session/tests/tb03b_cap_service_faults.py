#!/usr/bin/env python3
"""#1716 F5：tb03b-finance/4 内容域上 cap 激活的经济服务进程故障与恢复。

E 以正式 `e_economic_session serve` 运行，经 economic_service_control.py 起停与恢复。故障只经
manifest 的 TestOnly marker 定位：E 进程写出 marker 后，由控制器对该 PID 发 SIGKILL。
每个阶段（before_commit / after_commit）用新库，结束后以独立 oracle 只读复算 E 原库。

冻结输入取自 Rust 内容域夹具（finance_tests.rs content_domain_cap_activation_on_fixed_carriers）
写在 TB03B_TEST_ROOT 下的 tb03b-finance/4 原件；本驱动不生成 Voice、不启动 B、S 或浏览器。
"""
import argparse
from concurrent.futures import ThreadPoolExecutor
import copy
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
sys.path.insert(0, str(ROOT / "tests"))
from s_socket_client import canonical, decode_frame, digest, exchange, validate_reply
from tb03b_oracle import cap_activation as independent_cap_oracle, read_domain

PHASES = ("before_commit", "after_commit")
ACTIVATION = "tb1716-activate"
CAP_KEY = "tb03b-finance/4:ActivateCapPolicy:"


def save(path, value):
    with path.open("x") as stream:
        json.dump(value, stream, ensure_ascii=False, indent=2)
        stream.write("\n")


def template(root):
    for manifest in sorted(Path(root).glob("unit-*/manifest.json")):
        value = json.loads(manifest.read_text())
        if (value.get("protocol_revision") == "economic-session/2"
                and value["finance"]["schema_revision"] == "tb03b-finance/4"):
            return value
    raise ValueError("TB03B_TEST_ROOT 下没有 tb03b-finance/4 内容域夹具原件")


class Phase:
    def __init__(self, base, carriers, binaries, output, phase):
        self.phase, self.output = phase, output
        output.mkdir()
        self.log = (output / "commands.jsonl").open("x")
        self.counter = 0
        raw = Path(base["finance"]["input_path"]).read_bytes()
        assert hashlib.sha256(raw).hexdigest() == base["finance"]["input_sha256"], "夹具原件摘要不符"
        (output / "finance-input.json").write_bytes(raw)
        self.data = json.loads(raw)
        manifest = copy.deepcopy(base)
        manifest.pop("voice_binding", None)
        sockets = Path(tempfile.mkdtemp(prefix="tb1716-", dir="/tmp"))
        tag = "svc-" + phase
        manifest["e"] = {"db": str(carriers[0] / (tag + "-e.db")), "socket": str(sockets / "e.sock")}
        for route, carrier in zip(manifest["chongs"], carriers[1:]):
            route["db"] = str(carrier / (tag + "-" + route["chong_id"] + ".db"))
            route["socket"] = str(sockets / (route["chong_id"] + ".sock"))
        manifest["finance"]["input_path"] = str(output / "finance-input.json")
        manifest["finance"]["input_sha256"] = hashlib.sha256(raw).hexdigest()
        manifest["clock"]["plan"]["messages"] = {
            "tb1716-import": {"received_ns": "10", "first_known_ns": "11", "commit_ns": "12"},
            ACTIVATION: {"received_ns": "20", "first_known_ns": "21", "commit_ns": "22"},
            ACTIVATION + "-retry": {"received_ns": "1100", "first_known_ns": "1101", "commit_ns": "1102"},
        }
        manifest["test_only_fault"] = {"message_id": ACTIVATION, "phase": phase,
                                       "marker_path": str(output / "fault-marker.json")}
        self.manifest = manifest
        save(output / "manifest.json", manifest)
        auth = manifest["authorized_sources"][0]
        self.source = {k: auth[k] for k in ("source_namespace", "source_epoch", "producer_id", "producer_epoch")}
        epochs = {"E": "1", **{"B:" + r["chong_id"]: "1" for r in manifest["chongs"]}}
        save(output / "economic-launcher.json", {
            "schema_revision": "economic-launcher/1", "manifest": str(output / "manifest.json"),
            "manifest_hash": digest(manifest), "python": sys.executable,
            "e_binary": str(binaries / "e_economic_session"), "b_binary": str(binaries / "b_chong_session"),
            "state_root": str(output / "control"), "startup_timeout_ms": "30000",
            "probe_source": self.source, "expected_epochs": epochs})
        policy = self.data["policy"]
        record = {"schema_revision": "funding-cap-policy-record/1", "kind": "TestOnly",
                  "profile_id": policy["profile_id"], "policy_id": policy["policy_id"], "version": "2",
                  "supersedes": "1", "cap": self.data["baseline"]["amount"], "currency": policy["currency"],
                  "money_unit": policy["money_unit"], "valid_through_ns": "100000",
                  "authorizer": "TestOnly 合成签署人（#1716 服务故障夹具，非真实资金签署）",
                  "authorized_at_ns": "1", "signature_ref": "TestOnly-synthetic-signature/#1716"}
        record_raw = canonical(record)
        (output / "cap-record.json").write_bytes(record_raw)
        self.activation = {"op": "ActivateCapPolicy", "activation_id": "tb1716-cap-1-2",
                           "policy_id": policy["policy_id"], "from_version": "1", "to_version": "2",
                           "cap": record["cap"], "valid_through_ns": "100000",
                           "policy_record": {"path": str(output / "cap-record.json"),
                                             "sha256": hashlib.sha256(record_raw).hexdigest()}}

    def command(self, args):
        self.counter += 1
        label = str(self.counter).zfill(3)
        done = subprocess.run(args, capture_output=True, timeout=120)
        (self.output / (label + ".stdout")).write_bytes(done.stdout)
        (self.output / (label + ".stderr")).write_bytes(done.stderr)
        self.log.write(json.dumps({"command": args, "exit_code": done.returncode, "stdout": label + ".stdout"}) + "\n")
        self.log.flush()
        if done.returncode:
            raise RuntimeError("命令失败，原件 " + str(self.output / label) + "：" + done.stderr.decode()[-400:])
        return json.loads(done.stdout)

    def control(self, action, epoch, *extra):
        return self.command([sys.executable, str(ROOT / "economic_service_control.py"), action,
                             "--config", str(self.output / "economic-launcher.json"), "--domain", "E",
                             "--epoch", epoch, "--e-epoch", epoch, *extra])

    def send(self, message, payload, epoch, name):
        request = {"schema_revision": "economic-session/2", "session_id": self.manifest["session_id"],
                   "session_generation": self.manifest["session_generation"], **self.source,
                   "message_id": message, "payload_hash": digest(payload), "causal_refs": [], "payload": payload}
        save(self.output / (name + ".request.json"), request)
        result = exchange(self.manifest["e"]["socket"], request, timeout_ms=60000,
                          max_frame_bytes=int(self.manifest["resources"]["max_frame_bytes"]))
        save(self.output / (name + ".response.json"), result)
        if result["transport"] != "Received":
            return {"kind": "DeliveryUnknown", "transport": result["transport"]}
        return validate_reply(request, result["response"], producer="E", producer_epoch=epoch)

    def wait_marker(self, future):
        deadline = time.monotonic() + 60
        marker = self.output / "fault-marker.json"
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

    def run(self):
        e_db = self.manifest["e"]["db"]
        self.control("start", "1")
        imported = self.send("tb1716-import", {"op": "ImportResponsibility", "writer_epoch": "1",
                                               "responsibility_id": self.data["baseline"]["responsibility_id"]}, "1", "import")
        assert imported.get("kind") == "ResponsibilityImportedReceipt", imported
        original = {**self.activation, "writer_epoch": "1"}
        with ThreadPoolExecutor(max_workers=1) as pool:
            future = pool.submit(self.send, ACTIVATION, original, "1", "activate")
            marker = self.wait_marker(future)
            stopped = self.control("stop", "1", "--signal", "KILL")
            delivery = future.result()
        assert stopped["result"]["state"] == "stopped" and stopped["result"]["signal"] == "SIGKILL", stopped
        assert str(stopped["result"]["pid"]) == str(marker["pid"]), (stopped, marker)
        assert marker["phase"] == self.phase and marker["message_id"] == ACTIVATION
        save(self.output / "kill-receipt.json", {"stop": stopped, "marker": marker, "delivery": delivery})
        before = read_domain(e_db)
        save(self.output / "after-kill-domain.json", before)
        recovered = self.control("recover", "1")
        self.control("start", "2")
        after = read_domain(e_db)
        save(self.output / "after-recover-domain.json", after)
        assert before["commits"] == after["commits"] and before["receipts"] == after["receipts"], "恢复改写了持久历史"
        assert after["meta"]["writer_epoch"] == "2", after["meta"]
        committed = [k for k in after["receipts"] if k.startswith(CAP_KEY)]
        assert bool(committed) == (self.phase == "after_commit"), committed
        image = after["commits"][-1]["image"]["finance"]
        assert ("cap_policy" in image) == (self.phase == "after_commit"), image.get("cap_policy")
        resent = self.send(ACTIVATION, original, "2", "resend-original")
        if self.phase == "after_commit":
            assert resent == after["receipts"][committed[0]], (resent, after["receipts"][committed[0]])
        else:
            assert resent.get("kind") == "Error" and "StaleWriter" in resent["error"], resent
            retry = self.send(ACTIVATION + "-retry", {**self.activation, "writer_epoch": "2"}, "2", "activate-retry")
            assert retry.get("kind") == "CapPolicyActivatedReceipt", retry
        final = read_domain(e_db)
        live = final["commits"][-1]["image"]["finance"]["cap_policy"]
        assert live["policy_version"] == "2" and live["profile_id"] == self.data["policy"]["profile_id"], live
        oracle = independent_cap_oracle(self.output / "manifest.json")
        assert oracle["final_cap_version"] == "2" and oracle["final_liability"] == self.data["baseline"]["amount"], oracle
        save(self.output / "oracle.json", oracle)
        self.control("stop", "2")
        return {"phase": self.phase, "status": "passed", "marker_pid": marker["pid"],
                "receipt_after_kill": bool(committed), "recover_reply": recovered["result"]["reply"],
                "resend": resent.get("kind"), "oracle": {k: oracle[k] for k in ("activations", "final_cap_version", "final_cap", "final_liability")}}

    def close(self):
        try:
            self.control("stop", "2")
        except (RuntimeError, ValueError, OSError, subprocess.SubprocessError):
            pass
        self.log.close()


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--fixtures", type=Path, required=True, help="Rust 内容域夹具 TB03B_TEST_ROOT")
    p.add_argument("--carriers", required=True, help="E:B:alpha:B:beta 三个固定容量 HFS+ 卷挂载点，冒号分隔")
    p.add_argument("--binaries", type=Path, required=True)
    p.add_argument("--output", type=Path, required=True)
    args = p.parse_args()
    carriers = [Path(x) for x in args.carriers.split(":")]
    assert len(carriers) == 3, "需要三个卷"
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    base = template(args.fixtures)
    outcomes = []
    for phase in PHASES:
        run = None
        try:
            run = Phase(base, carriers, args.binaries.resolve(), output / phase, phase)
            outcomes.append(run.run())
        except (AssertionError, ValueError, RuntimeError, TimeoutError, OSError, KeyError, subprocess.SubprocessError) as error:
            outcomes.append({"phase": phase, "status": "failed", "error": repr(error)})
        finally:
            if run is not None:
                run.close()
    save(output / "RESULT.json", {"issue": 1716, "scope": "tb03b-finance/4 E 正式服务进程 SIGKILL 与恢复；不含 S、B、Voice、浏览器",
                                  "outcomes": outcomes})
    print(json.dumps(outcomes, ensure_ascii=False))
    return 0 if all(o["status"] == "passed" for o in outcomes) else 1


if __name__ == "__main__":
    raise SystemExit(main())
