"""#1374：只读组合独立 E/B 权威图像，不持写连接、不运行经济 reducer。"""

from collections import OrderedDict
from concurrent.futures import ThreadPoolExecutor
import copy
import json
from pathlib import Path
import re
import threading
import time
import uuid

from s_socket_client import canonical, decode_frame, digest, exchange, validate_reply


class EconomicQueryError(ValueError):
    pass


def integer(value, field, *, positive=False):
    if type(value) is not str or not re.fullmatch(r"0|[1-9][0-9]*", value):
        raise EconomicQueryError(field + " 须为规范非负整数文本")
    result = int(value)
    if result > 2**63 - 1 or (positive and result == 0):
        raise EconomicQueryError(field + " 越出范围")
    return result


def text(value, field):
    if type(value) is not str or not value:
        raise EconomicQueryError(field + " 缺失")
    return value


def verify_cut(cut, domain):
    if type(cut) is not dict or set(cut) != {"domain_id", "commit_seq", "root_hash"}:
        raise EconomicQueryError("经济 cut 字段不完整或多余")
    if cut["domain_id"] != domain:
        raise EconomicQueryError("经济 cut 属于其他域")
    integer(cut["commit_seq"], "commit_seq")
    if type(cut["root_hash"]) is not str or not re.fullmatch(r"[0-9a-f]{64}", cut["root_hash"]):
        raise EconomicQueryError("经济 cut 缺少完整根摘要")
    return cut


class EconomicQuery:
    """一个 Q 配置固定可信 owner；固定向量缓存只留完整已核图像。"""

    def __init__(self, config_path):
        path = Path(config_path).resolve()
        config = decode_frame(path.read_bytes())
        if config.get("schema_revision") != "economic-query/1":
            raise EconomicQueryError("需要 economic-query/1")
        manifest_path = (path.parent / text(config.get("manifest"), "manifest")).resolve()
        raw_manifest = manifest_path.read_bytes()
        manifest = decode_frame(raw_manifest)
        if manifest.get("schema_revision") != "economic-manifest/1":
            raise EconomicQueryError("需要绑定 economic-manifest/1")
        if digest(manifest) != config.get("manifest_hash"):
            raise EconomicQueryError("manifest 与已配置摘要不同")
        self.manifest = manifest
        self.manifest_hash = config["manifest_hash"]
        self.session = text(manifest.get("session_id"), "session_id")
        self.generation = manifest.get("session_generation")
        integer(self.generation, "session_generation", positive=True)
        self.source = config.get("query_source")
        source_fields = {"source_namespace", "source_epoch", "producer_id", "producer_epoch"}
        if type(self.source) is not dict or set(self.source) != source_fields:
            raise EconomicQueryError("Q 连接身份不在获准源内")
        matching = [entry for entry in manifest.get("authorized_sources", [])
                    if type(entry) is dict and {k: entry.get(k) for k in source_fields} == self.source]
        read_ops = {"ReadView", "Watch", "QueryFact", "QueryAllocation"}
        if (len(matching) != 1 or type(matching[0].get("operations")) is not list
                or any(type(op) is not str for op in matching[0]["operations"])
                or not {"ReadView", "Watch"} <= set(matching[0]["operations"]) <= read_ops):
            raise EconomicQueryError("Q 身份须明确只有只读操作权限")
        for key, value in self.source.items():
            text(value, key)
        integer(self.source["producer_epoch"], "producer_epoch", positive=True)
        self.query_epoch = config.get("producer_epoch")
        integer(self.query_epoch, "Q producer_epoch", positive=True)
        resources = config.get("resources")
        fields = {"max_frame_bytes", "timeout_ms", "operation_timeout_ms", "max_reply_bytes",
                  "max_cached_bytes", "max_snapshots", "max_domains", "max_changes"}
        if type(resources) is not dict or set(resources) != fields:
            raise EconomicQueryError("Q 资源必须逐项声明")
        self.resources = {k: integer(resources[k], k, positive=True) for k in fields}
        nodes = [("E", manifest.get("e"))]
        chongs = manifest.get("chongs")
        if type(chongs) is not list:
            raise EconomicQueryError("manifest.chongs 必须是数组")
        for node in chongs:
            if type(node) is not dict:
                raise EconomicQueryError("重路由必须是对象")
            nodes.append(("B:" + text(node.get("chong_id"), "chong_id"), node))
        if len(nodes) > self.resources["max_domains"] or len({d for d, _ in nodes}) != len(nodes):
            raise EconomicQueryError("域数量越界或身份重复")
        self.nodes = {}
        for domain, node in nodes:
            if type(node) is not dict:
                raise EconomicQueryError("域路由缺失")
            self.nodes[domain] = str((manifest_path.parent / text(node.get("socket"), "socket")).resolve())
        epochs = config.get("expected_epochs")
        if type(epochs) is not dict or set(epochs) != set(self.nodes):
            raise EconomicQueryError("须分别声明每个 owner 的获准 epoch")
        for domain, epoch in epochs.items():
            integer(epoch, domain + " epoch", positive=True)
        self.epochs = copy.deepcopy(epochs)
        self.lock = threading.Lock()
        self.cache = OrderedDict()
        self.cached_bytes = 0
        self.budget = threading.local()

    def _start(self):
        self.budget.deadline = time.monotonic() + self.resources["operation_timeout_ms"] / 1000

    def _remaining_ms(self):
        deadline = getattr(self.budget, "deadline", None)
        if deadline is None:
            self._start()
            deadline = self.budget.deadline
        left = int((deadline - time.monotonic()) * 1000)
        if left <= 0:
            raise EconomicQueryError("QueryBudgetExceeded：完整经济查询达到总期限")
        return min(left, self.resources["timeout_ms"])

    def _parallel_domains(self, operation):
        deadline = self.budget.deadline

        def run(domain):
            self.budget.deadline = deadline
            try:
                return operation(domain)
            except (ValueError, KeyError, TypeError, OSError, RecursionError) as exc:
                return {"status": "Unavailable", "error": str(exc)}

        # 每域同时开始且各自有界；慢域不会消耗健康域尚未开始的读取机会。
        with ThreadPoolExecutor(max_workers=len(self.nodes)) as pool:
            return dict(zip(self.nodes, pool.map(run, self.nodes)))

    def request(self, payload):
        return {
            "schema_revision": "economic-session/1", "session_id": self.session,
            "session_generation": self.generation, **self.source,
            "message_id": "query-" + uuid.uuid4().hex, "payload_hash": digest(payload),
            "causal_refs": [], "payload": payload,
        }

    def owner(self, domain, payload):
        request = self.request(payload)
        result = exchange(self.nodes[domain], request, timeout_ms=self._remaining_ms(),
                          max_frame_bytes=self.resources["max_frame_bytes"])
        if result["transport"] != "Received":
            raise EconomicQueryError(result["transport"] + ": " + result.get("detail", ""))
        response = validate_reply(request, result["response"], producer=domain,
                                  producer_epoch=self.epochs[domain])
        if response.get("ok") is False or "error" in response:
            raise EconomicQueryError(str(response.get("error", "权威域不可用")))
        if response.get("domain_id") != domain:
            raise EconomicQueryError("权威域内容与获准连接不同")
        return response

    def view(self, domain, cut=None, as_known_ns=None):
        if cut is not None:
            verify_cut(cut, domain)
        if as_known_ns is not None:
            integer(as_known_ns, "as_known_ns")
        response = self.owner(domain, {"op": "ReadView", "cut": cut, "as_known_ns": as_known_ns})
        if response.get("kind") != "EconomicView":
            raise EconomicQueryError("owner 未返回完整经济图像")
        verify_cut(response.get("cut"), domain)
        commit_ns = response.get("commit_ns")
        integer(commit_ns, "commit_ns")
        if as_known_ns is not None and int(commit_ns) > int(as_known_ns):
            raise EconomicQueryError("owner 返回了获知截止时刻之后的图像")
        if cut is not None and response["cut"] != cut:
            raise EconomicQueryError("owner 回看切面与请求不同")
        image = response.get("image")
        if (type(image) is not dict
                or set(image) != {"book", "recorded", "pending", "applied", "unmatched", "history"}
                or any(type(image[k]) is not list for k in set(image) - {"book"})
                or (image["book"] is not None and type(image["book"]) is not dict)):
            raise EconomicQueryError("经济图像字段不完整或类型错误")
        if response.get("image_hash") != digest(image):
            raise EconomicQueryError("经济图像与完整摘要不同")
        # 当前读取 PID/运行环境/epoch 单列诊断，不改变历史图像或语义 token。
        return {"status": "Available", "cut": response["cut"], "commit_ns": commit_ns, "image": image,
                "image_hash": response["image_hash"]}

    def _store(self, body):
        token = "economic-snapshot-" + digest(body)
        encoded = canonical(body)
        if len(encoded) > self.resources["max_reply_bytes"] or len(encoded) > self.resources["max_cached_bytes"]:
            raise EconomicQueryError("QueryBudgetExceeded：完整经济切面超过事前容量")
        with self.lock:
            if token not in self.cache:
                while self.cache and (len(self.cache) >= self.resources["max_snapshots"]
                                      or self.cached_bytes + len(encoded) > self.resources["max_cached_bytes"]):
                    _, old = self.cache.popitem(last=False)
                    self.cached_bytes -= len(old)
                self.cache[token] = encoded
                self.cached_bytes += len(encoded)
        return {**body, "snapshot_token": token}

    def snapshot(self, payload):
        self._start()
        if type(payload) is not dict or set(payload) != {"op", "cuts", "as_known_ns", "snapshot_token"}:
            raise EconomicQueryError("snapshot 参数不完整或多余")
        if payload["op"] != "snapshot":
            raise EconomicQueryError("请求操作不符")
        token, cuts, as_known = payload["snapshot_token"], payload["cuts"], payload["as_known_ns"]
        if token is not None:
            if type(token) is not str or cuts is not None or as_known is not None:
                raise EconomicQueryError("固定 token 不能混入新 cut 或获知时刻")
            with self.lock:
                encoded = self.cache.get(token)
            if encoded is None:
                return {"kind": "Gap", "reason": "SnapshotExpired", "requested_token": token,
                        "session_id": self.session, "session_generation": self.generation}
            return {**json.loads(encoded), "snapshot_token": token}
        if cuts is not None and (type(cuts) is not dict or set(cuts) != set(self.nodes)):
            raise EconomicQueryError("回看须给完整域向量")
        if cuts is not None and as_known is not None:
            raise EconomicQueryError("固定向量与获知时刻不能同时指定")
        if as_known is not None:
            integer(as_known, "as_known_ns")
        if cuts is not None:
            for domain in self.nodes:
                verify_cut(cuts[domain], domain)
        views = self._parallel_domains(lambda domain: self.view(
            domain, None if cuts is None else cuts[domain], as_known))
        return self._store({
            "kind": "EconomicSnapshot", "session_id": self.session, "session_generation": self.generation,
            "manifest_hash": self.manifest_hash,
            "mode": "AsKnown" if cuts is not None or as_known is not None else "Current",
            "as_known_ns": as_known, "domains": views,
            "completeness": "CompleteVector" if all(v["status"] == "Available" for v in views.values()) else "PartialVector",
        })

    def watch(self, payload):
        self._start()
        if type(payload) is not dict or set(payload) != {"op", "cuts"} or payload["op"] != "watch":
            raise EconomicQueryError("watch 参数不完整或多余")
        cuts = payload["cuts"]
        if type(cuts) is not dict or set(cuts) != set(self.nodes):
            raise EconomicQueryError("watch 须给完整域向量")
        for domain, cut in cuts.items():
            verify_cut(cut, domain)
        def read_changes(domain):
            base = self.view(domain, cuts[domain])
            response = self.owner(domain, {"op": "Watch", "after_cut": cuts[domain]})
            if response.get("kind") != "EconomicChanges" or type(response.get("changes")) is not list:
                raise EconomicQueryError("变化响应不完整")
            if len(response["changes"]) > self.resources["max_changes"]:
                raise EconomicQueryError("QueryBudgetExceeded：变化数量超过事前界限")
            verify_cut(response.get("cut"), domain)
            previous, previous_ns = cuts[domain], integer(base["commit_ns"], "base commit_ns")
            for change in response["changes"]:
                if type(change) is not dict or set(change) != {"cut", "image", "image_hash", "commit_ns"}:
                    raise EconomicQueryError("变化项不是完整原子批次")
                verify_cut(change["cut"], domain)
                ns = integer(change["commit_ns"], "change commit_ns")
                if ns < previous_ns or change["image_hash"] != digest(change["image"]):
                    raise EconomicQueryError("变化时间倒退或完整图像摘要不同")
                if integer(change["cut"]["commit_seq"], "commit_seq") != integer(previous["commit_seq"], "commit_seq") + 1:
                    raise EconomicQueryError("变化序号存在缺口或重复")
                fixed = self.view(domain, change["cut"])
                if (fixed["image"] != change["image"] or fixed["image_hash"] != change["image_hash"]
                        or fixed["commit_ns"] != change["commit_ns"]):
                    raise EconomicQueryError("变化与同切面权威图像/时间不同")
                previous, previous_ns = change["cut"], ns
            if previous != response["cut"]:
                raise EconomicQueryError("变化缺少到实际前沿的完整批次")
            return {"status": "Available", "base_cut": cuts[domain],
                    "next_cut": response["cut"], "changes": response["changes"]}

        domains = self._parallel_domains(read_changes)
        result = {"kind": "EconomicWatch", "session_id": self.session,
                  "session_generation": self.generation, "manifest_hash": self.manifest_hash, "domains": domains}
        if len(canonical(result)) > self.resources["max_reply_bytes"]:
            raise EconomicQueryError("QueryBudgetExceeded：完整经济变化超过事前容量")
        return result
