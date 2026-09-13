#!/usr/bin/env python3
"""#1374：正式入口的 E/各 B 独立生命期；复用既有 PID/锁/socket 核验。"""

import argparse
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import time
import uuid

from s_service_control import Controller, positive, remaining
from s_socket_client import canonical, decode_frame, digest, exchange, validate_reply


def read_config(path, domain):
    config = decode_frame(path.read_bytes())
    if config.get("schema_revision") != "economic-launcher/1":
        raise ValueError("需要 economic-launcher/1")
    for key in ("manifest", "python", "e_binary", "b_binary", "state_root"):
        if type(config.get(key)) is not str or not config[key]:
            raise ValueError("缺少启动路径 " + key)
        config[key] = str((path.parent / config[key]).resolve())
    positive(config.get("startup_timeout_ms"), "startup_timeout_ms")
    manifest = decode_frame(Path(config["manifest"]).read_bytes())
    if manifest.get("schema_revision") != "economic-manifest/1" or digest(manifest) != config.get("manifest_hash"):
        raise ValueError("经济 manifest 与已配置来源不同")
    nodes = {"E": manifest["e"]}
    for node in manifest["chongs"]:
        key = "B:" + node["chong_id"]
        if key in nodes:
            raise ValueError("重路由重复")
        nodes[key] = node
    if domain not in nodes:
        raise ValueError("指定域不在 manifest 中")
    epochs = config.get("expected_epochs")
    if type(epochs) is not dict or set(epochs) != set(nodes):
        raise ValueError("须声明各域初始 epoch")
    for name, epoch in epochs.items():
        positive(epoch, name + " epoch")
    source = config.get("probe_source")
    fields = {"source_namespace", "source_epoch", "producer_id", "producer_epoch"}
    if type(source) is not dict or set(source) != fields:
        raise ValueError("须明确只读探测源")
    matching = [a for a in manifest["authorized_sources"] if {k: a.get(k) for k in fields} == source]
    if len(matching) != 1 or "ReadView" not in matching[0].get("operations", []):
        raise ValueError("探测源没有明确 ReadView 权限")
    node = nodes[domain]
    for field in ("db", "socket"):
        if type(node.get(field)) is not str or not Path(node[field]).is_absolute():
            raise ValueError("持久域路径必须绝对")
    if len(os.fsencode(node["socket"])) > 100:
        raise ValueError("Unix socket 路径超过本入口 100 字节上限")
    return config, manifest, node


class EconomicController(Controller):
    def __init__(self, config, manifest, node, domain):
        self.launcher, self.manifest, self.domain = config, manifest, domain
        state = Path(config["state_root"]) / digest(domain)
        super().__init__({"db": node["db"], "socket": node["socket"], "python": config["python"],
                          "startup_timeout_ms": config["startup_timeout_ms"], "socket_service": "owner"}, state)

    def command(self, action):
        binary = self.launcher["e_binary" if self.domain == "E" else "b_binary"]
        result = [binary, action, "--manifest", self.launcher["manifest"], "--db", self.config["db"]]
        if self.domain != "E":
            result += ["--chong", self.domain.removeprefix("B:")]
        return result

    def probe(self):
        payload = {"op": "ReadView", "cut": None, "as_known_ns": None}
        return {"schema_revision": "economic-session/1", "session_id": self.manifest["session_id"],
                "session_generation": self.manifest["session_generation"], **self.launcher["probe_source"],
                "message_id": "control-" + uuid.uuid4().hex, "payload_hash": digest(payload),
                "payload": payload, "causal_refs": []}

    def start_owner(self, epoch, e_epoch):
        deadline = time.monotonic() + int(self.config["startup_timeout_ms"]) / 1000
        environment = self.runtime_environment(deadline)
        cleanup = self.prepare_socket(deadline)
        initialization = None
        if not Path(self.config["db"]).exists():
            done = subprocess.run(self.command("init"), capture_output=True, check=True,
                                  env=environment, timeout=remaining(deadline))
            initialization = decode_frame(done.stdout)
        command = self.command("serve") + ["--epoch", epoch]
        if self.domain != "E":
            command += ["--e-epoch", e_epoch]
        self.start_process("owner", command, environment, deadline=deadline)
        expected = self.status("owner")
        detail = "owner 尚未就绪"
        while time.monotonic() < deadline:
            if self.status("owner")["state"] != "running":
                raise ValueError("owner 已退出，请查本域日志")
            request = self.probe()
            result = exchange(self.config["socket"], request, timeout_ms=max(1, int(remaining(deadline) * 1000)),
                              max_frame_bytes=int(self.manifest["resources"]["max_frame_bytes"]))
            if result["transport"] == "Received":
                try:
                    reply = validate_reply(request, result["response"], producer=self.domain, producer_epoch=epoch)
                    runtime = reply.get("runtime", {})
                    if (reply.get("kind") == "EconomicView" and reply.get("domain_id") == self.domain
                            and reply.get("writer_epoch") == epoch
                            and runtime.get("control_instance_id") == expected["control_instance_id"]
                            and runtime.get("pid") == str(expected["pid"])):
                        self.confirm_ready_process("owner", expected, deadline)
                        record = {k: v for k, v in self.status("owner").items() if k != "state"}
                        record["socket_identity"] = self.socket_identity()
                        self.save_record("owner", record)
                        return {"domain_id": self.domain, "state": "ready", "reply": result["response"],
                                "initialization": initialization, "socket_cleanup": cleanup}
                    detail = reply
                except ValueError as exc:
                    detail = str(exc)
            else:
                detail = result
            time.sleep(min(0.05, remaining(deadline)))
        raise TimeoutError("经济 owner 未在明确期限内就绪：" + str(detail))

    def recover_owner(self):
        if self.status("owner")["state"] != "stopped":
            raise ValueError("须有本控制器记录且已确认原 owner 停止")
        deadline = time.monotonic() + int(self.config["startup_timeout_ms"]) / 1000
        done = subprocess.run(self.command("recover"), capture_output=True, check=True,
                              env=self.runtime_environment(deadline), timeout=remaining(deadline))
        return {"domain_id": self.domain, "state": "recovered", "reply": decode_frame(done.stdout)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("start", "stop", "status", "recover", "send"))
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--domain", required=True)
    parser.add_argument("--epoch")
    parser.add_argument("--e-epoch")
    parser.add_argument("--signal", choices=("TERM", "KILL"), default="TERM")
    parser.add_argument("--request", type=Path)
    args = parser.parse_args()
    control = None
    try:
        config, manifest, node = read_config(args.config.resolve(), args.domain)
        epoch = args.epoch or config["expected_epochs"][args.domain]
        e_epoch = args.e_epoch or config["expected_epochs"]["E"]
        positive(epoch, "owner epoch")
        positive(e_epoch, "E epoch")
        # 业务输送可能等到回执失联；不能持生命周期锁阻断同时进行的受控停止/恢复。
        # 业务事务和写者代际仍由实际owner核验；这里只核原请求与绑定域的公共回复。
        if args.action == "send":
            if args.request is None:
                raise ValueError("send 须提供完整原身份请求文件")
            limit = int(manifest["resources"]["max_frame_bytes"])
            with args.request.open("rb") as stream:
                raw = stream.read(limit + 1)
            if len(raw) > limit:
                raise ValueError("原请求文件超过事前帧上限")
            request = decode_frame(raw)
            if len(canonical(request)) + 1 > limit:
                raise ValueError("请求超过事前帧上限")
            result = exchange(node["socket"], request, timeout_ms=int(config["startup_timeout_ms"]), max_frame_bytes=limit)
            if result["transport"] == "Received":
                validate_reply(request, result["response"], producer=args.domain, producer_epoch=epoch)
            print(json.dumps(result, ensure_ascii=True, indent=2))
            return 0 if result["transport"] == "Received" else 2
        control = EconomicController(config, manifest, node, args.domain)
        if args.action == "start":
            result = control.start_owner(epoch, e_epoch)
        elif args.action == "stop":
            result = control.stop("owner", getattr(signal, "SIG" + args.signal))
        elif args.action == "status":
            result = control.status("owner")
        else:
            result = control.recover_owner()
        print(json.dumps({"ok": True, "result": result}, ensure_ascii=True, indent=2))
        return 0
    except (ValueError, OSError, KeyError, TypeError, subprocess.SubprocessError) as exc:
        print(json.dumps({"ok": False, "error": str(exc)}, ensure_ascii=True), file=sys.stderr)
        return 1
    finally:
        if control is not None:
            control.close()


if __name__ == "__main__":
    raise SystemExit(main())
