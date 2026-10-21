#!/usr/bin/env python3
"""#1372：S 本地持续入口的一次输送，不生成或重试业务身份。

每连接一个 JSON 行；上限和总期限显式给出。开始发送后若没有完整合法响应，
结果仅为 DeliveryUnknown，调用方须按原身份查询持久收据，不能改身份重做。
Received/退出0只表示完整输送，S业务成功及响应身份仍由调用方按合同核实。
"""

import argparse
import base64
import binascii
import hashlib
import json
import math
import re
import socket
import sys
import threading
import time
from pathlib import Path


def canonical(value):
    """与 Rust session_protocol 相同的规范 JSON；不进行业务计算。"""
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"),
                      allow_nan=False).encode("utf-8")


def digest(value):
    return hashlib.sha256(canonical(value)).hexdigest()


def reply_envelope(request, payload, *, session_id, session_generation, namespace,
                   producer_id, producer_epoch):
    """公共响应头；身份参数必须来自服务配置，不能从请求复制为权威。"""
    payload_hash = digest(payload)
    cause = {key: request[key] for key in (
        "source_namespace", "source_epoch", "message_id", "payload_hash")}
    return {
        "schema_revision": request["schema_revision"], "session_id": session_id,
        "session_generation": session_generation, "source_namespace": namespace,
        "source_epoch": producer_epoch, "producer_id": producer_id, "producer_epoch": producer_epoch,
        "message_id": "reply-" + digest([
            request["source_namespace"], request["source_epoch"], request["message_id"], payload_hash]),
        "payload_hash": payload_hash, "causal_refs": [cause], "payload": payload,
    }


def _unique_object(pairs):
    out = {}
    for key, value in pairs:
        if key in out:
            raise ValueError("重复 JSON 键：" + key)
        out[key] = value
    return out


def _reject_number(value):
    raise ValueError("协议不接受非精确数值：" + value)


def decode_json_value(raw):
    """精确 JSON 值；调用方另核其文件或协议容器形状。"""
    return json.loads(raw.decode("utf-8"), object_pairs_hook=_unique_object,
                      parse_float=_reject_number, parse_constant=_reject_number)


def decode_frame(raw):
    result = decode_json_value(raw)
    if type(result) is not dict or not result:
        raise ValueError("协议帧必须为非空 JSON 对象")
    return result


def validate_reply(request, response, *, producer, producer_epoch, allow_control_instance=False):
    """业务调用方核实完整公共头、内容摘要与原请求因果身份。"""
    fields = {"schema_revision", "session_id", "session_generation", "source_namespace", "source_epoch",
              "message_id", "producer_id", "producer_epoch", "payload_hash", "causal_refs", "payload"}
    actual = set(response) if type(response) is dict else set()
    if allow_control_instance:
        actual.discard("control_instance_id")
    if actual != fields or type(response.get("payload")) is not dict:
        raise ValueError("响应公共头字段不完整或多余")
    for key in ("schema_revision", "session_id", "session_generation"):
        if response[key] != request[key]:
            raise ValueError("响应身份不属于原请求：" + key)
    revision = request.get("schema_revision")
    economic = revision in ("economic-session/1", "economic-session/2")
    if not economic and revision not in ("s-session/2", "s-session/3"):
        raise ValueError("未知响应协议版本")
    economic_book = (type(producer) is str and producer.startswith("B:")
                     and bool(producer[2:]) and not any(c.isspace() for c in producer))
    allowed = ((producer in ("E", "Q") or economic_book)
               if economic else producer in ("S", "Q"))
    if not allowed or type(producer_epoch) is not str or not producer_epoch:
        raise ValueError("须显式指定响应生产者及epoch")
    expected_namespace = ("economic-session/replies" if economic and producer != "Q"
                          else "economic-observe/replies" if economic
                          else "s-session/replies" if producer == "S" else "s-observe/replies")
    if (response["producer_id"] != producer + ":" + request["session_id"]
            or response["producer_epoch"] != producer_epoch
            or response["source_epoch"] != producer_epoch
            or response["source_namespace"] != expected_namespace):
        raise ValueError("响应生产者/epoch不符")
    payload_hash = digest(response["payload"])
    cause = {key: request[key] for key in ("source_namespace", "source_epoch", "message_id", "payload_hash")}
    reply_id = "reply-" + digest([request["source_namespace"], request["source_epoch"], request["message_id"], payload_hash])
    if response["payload_hash"] != payload_hash or response["causal_refs"] != [cause] or response["message_id"] != reply_id:
        raise ValueError("响应内容摘要或原请求因果绑定不符")
    return response["payload"]


class EconomicWireEvidence:
    """显式启用的 TestOnly 原wire落盘；每公共请求有独立、不可覆盖的有限账。"""

    def __init__(self, directory):
        self.root = Path(directory)
        self.root.mkdir(mode=0o700, parents=False, exist_ok=False)
        self.lock, self.attempt = threading.Lock(), 0

    def scope(self, request, deadline):
        # 相同信封的每次物理尝试独立留证；不能用请求摘要拒绝合法精确重送。
        with self.lock:
            self.attempt += 1
            return _EconomicWireScope(self.root, request, deadline, self.attempt)


class _EconomicWireScope:
    def __init__(self, root, request, deadline, attempt):
        self.attempt = attempt
        self.root = root / f"attempt-{attempt:06d}"
        self.root.mkdir(mode=0o700, exist_ok=False)
        self.deadline, self.count, self.bytes = deadline, 0, 0
        self.lock = threading.Lock()
        self._write("public-request.json", canonical(request))
        self._json("begin.json", {"schema_revision": "c6a-q-owner-wire/1",
            "attempt": attempt, "public_request_sha256": digest(request), "started_at": time.monotonic(),
            "deadline": deadline})

    def _write(self, name, raw):
        with (self.root / name).open("xb") as stream:
            stream.write(raw)

    def _json(self, name, value):
        self._write(name, json.dumps(value, sort_keys=True, separators=(",", ":"),
                                    allow_nan=False).encode())

    def begin(self, address, request, timeout_ms, max_frame_bytes, deadline, logical_deadline):
        with self.lock:
            # 不新增每scope拒绝帽；直接落原证据目录，受原外层磁盘/RSS/期限约束。
            if max_frame_bytes > 1048576 or deadline > self.deadline:
                raise ValueError("原wire采集帧界或期限不属于公共请求")
            self.count += 1
            index = self.count
            self.bytes += len(request)
            prefix = f"rpc-{index:04d}"
            self._write(prefix + "-request.bin", request)
            self._json(prefix + "-begin.json", {"index": index, "address": str(address),
                "timeout_ms": timeout_ms, "max_frame_bytes": max_frame_bytes,
                "started_at": time.monotonic(), "deadline": deadline,
                "logical_deadline": logical_deadline,
                "request_sha256": hashlib.sha256(request).hexdigest(),
                "request_bytes": len(request)})
            return prefix

    def finish(self, prefix, response, result, error_class, started_send):
        # bytearray直接写入；不再复制/缓存一个完整文档。溢出哨兵最多多1字节。
        with self.lock:
            if len(response) > 1048577:
                raise ValueError("原wire证据超过帧上限及单字节溢出哨兵")
            self.bytes += len(response)
            self._write(prefix + "-response.bin", response)
            self._json(prefix + "-end.json", {"transport": result["transport"],
                "error_class": error_class, "started_send": started_send,
                "observed_at": time.monotonic(), "response_bytes": len(response),
                "response_sha256": hashlib.sha256(response).hexdigest()})

    def close(self, error_class):
        with self.lock:
            self._json("end.json", {"attempt": self.attempt, "rpc_count": self.count, "wire_bytes": self.bytes,
                "error_class": error_class, "observed_at": time.monotonic()})


def exchange(socket_path, request, *, timeout_ms, max_frame_bytes, evidence=None, logical_deadline=None):
    """输送错误与 S 的业务响应分离；有限等待，不自动重发或解释成未发生。"""
    if type(timeout_ms) is not int or timeout_ms <= 0:
        raise ValueError("timeout_ms 必须是正整数")
    if type(max_frame_bytes) is not int or max_frame_bytes <= 0:
        raise ValueError("max_frame_bytes 必须是正整数")
    if type(request) is not dict or not request:
        raise ValueError("request 必须是非空对象")
    body = canonical(request) + b"\n"
    if len(body) > max_frame_bytes:
        raise ValueError("请求超过具名帧字节上限")
    decode_frame(body)
    deadline = time.monotonic() + timeout_ms / 1000
    if logical_deadline is not None:
        deadline = min(deadline, logical_deadline)
    if evidence is not None:
        deadline = min(deadline, evidence.deadline)
    started_send, error_class, response = False, None, bytearray()
    ticket = None if evidence is None else evidence.begin(
        socket_path, body, timeout_ms, max_frame_bytes, deadline, logical_deadline)

    def remaining():
        value = deadline - time.monotonic()
        if value <= 0:
            raise TimeoutError("一次输送的总期限已到")
        return value

    try:
        with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as connection:
            connection.settimeout(remaining())
            connection.connect(str(socket_path))
            connection.settimeout(remaining())
            started_send = True
            connection.sendall(body)
            connection.shutdown(socket.SHUT_WR)
            while True:
                connection.settimeout(remaining())
                chunk = connection.recv(min(65536, max_frame_bytes + 1 - len(response)))
                if not chunk:
                    if b"\n" not in response:
                        raise ValueError("连接结束但没有完整响应帧")
                    frame, tail = response.split(b"\n", 1)
                    if tail:
                        raise ValueError("单次输送收到多帧或额外内容")
                    decoded = decode_frame(frame)
                    remaining()
                    result = {"transport": "Received", "response": decoded}
                    break
                response.extend(chunk)
                if len(response) > max_frame_bytes:
                    raise ValueError("响应超过具名帧字节上限")
                if b"\n" in response:
                    _, tail = response.split(b"\n", 1)
                    if tail:
                        raise ValueError("单次输送收到多帧或额外内容")
    except (OSError, ValueError, RecursionError) as exc:
        error_class = type(exc).__name__
        result = {"transport": "DeliveryUnknown" if started_send else "TransportUnavailable",
                  "message_id": request.get("message_id"), "detail": str(exc)}
    if evidence is not None:
        evidence.finish(ticket, response, result, error_class, started_send)
        if result["transport"] == "Received":
            remaining()  # 原wire保存成本也不能另开期限。
    return result


ECONOMIC_PAGE_REVISION = "economic-read-pages/1"
ECONOMIC_PAGE_BYTES = 262144


def _page_fields(value, fields, name):
    if type(value) is not dict or set(value) != set(fields):
        raise ValueError(name + " 字段不完整或多余")


def _page_integer(value, name, *, positive=False):
    # Wire integers are canonical decimal strings, never JSON bool/int/float.
    if type(value) is not str or re.fullmatch(r"0|[1-9][0-9]*", value) is None:
        raise ValueError(name + " 须规范非负十进制文本")
    if len(value) > 20:
        raise ValueError(name + " 超过可处理整数范围")
    number = int(value)
    if positive and number == 0:
        raise ValueError(name + " 须为正数")
    return number


def _page_hash(value, name):
    if type(value) is not str or re.fullmatch(r"[0-9a-f]{64}", value) is None:
        raise ValueError(name + " 须为小写 SHA256")
    return value


def _page_cut(value, domain):
    _page_fields(value, ("domain_id", "commit_seq", "root_hash"), "cut")
    if value["domain_id"] != domain:
        raise ValueError("分页 cut 跨域")
    _page_integer(value["commit_seq"], "commit_seq")
    _page_hash(value["root_hash"], "root_hash")


def _page_image(value):
    if "image" not in value or value.get("image_hash") != digest(value["image"]):
        raise ValueError("分页正文图像摘要不同")


def _page_changes(changes, domain, base, end):
    if type(changes) is not list or len(changes) > 32:
        raise ValueError("分页完整变化批次超过32或类型不同")
    previous = base
    seq = -1 if base is None else _page_integer(base["commit_seq"], "base commit_seq")
    previous_ns = None
    for change in changes:
        if type(change) is not dict:
            raise ValueError("分页 change 不是对象")
        _page_cut(change.get("cut"), domain)
        if int(change["cut"]["commit_seq"]) != seq + 1:
            raise ValueError("分页变化存在缺口或重复")
        _page_image(change)
        ns = _page_integer(change.get("commit_ns"), "change commit_ns")
        if previous_ns is not None and ns < previous_ns:
            raise ValueError("分页变化时间倒退")
        previous, seq, previous_ns = change["cut"], seq + 1, ns
    if previous is None or previous != end:
        raise ValueError("分页变化未达固定前沿")


def _verify_page_document(value, descriptor, query, *, producer, session_id,
                          session_generation, manifest_hash, producer_epoch, owner_epochs):
    """只核页描述符与原逻辑对象的身份投影；金融 oracle 仍在调用方。"""
    kind = descriptor["document_kind"]
    if value.get("kind") != kind:
        raise ValueError("分页正文 kind 与描述符不同")
    if producer != "Q":
        if (value.get("domain_id") != producer or value.get("writer_epoch") != producer_epoch
                or value.get("cut") != descriptor["fixed_cuts"][producer]):
            raise ValueError("分页正文与描述符固定身份不同")
        if kind == "EconomicView":
            _page_image(value)
            _page_integer(value.get("commit_ns"), "commit_ns")
            if (query["as_known_ns"] is not None
                    and int(value["commit_ns"]) > int(query["as_known_ns"])):
                raise ValueError("分页正文晚于获知截止")
        else:
            _page_changes(value.get("changes"), producer, query["after_cut"], value["cut"])
        return
    if (value.get("session_id") != session_id or value.get("session_generation") != session_generation
            or value.get("manifest_hash") != manifest_hash):
        raise ValueError("Q 分页正文会话/manifest 不同")
    _page_fields(value.get("domains"), owner_epochs, "正文 domains")
    for domain, view in value["domains"].items():
        if type(view) is not dict or view.get("status") not in ("Available", "Unavailable"):
            raise ValueError("Q 分页正文域状态非法")
        fixed = descriptor["fixed_cuts"][domain]
        if view["status"] == "Unavailable":
            if fixed is not None:
                raise ValueError("Unavailable 不能冒充已固定 cut")
            continue
        actual = view.get("cut" if kind == "EconomicSnapshot" else "next_cut")
        _page_cut(actual, domain)
        if fixed != actual:
            raise ValueError("Q 正文 cut 与描述符不同")
        if kind == "EconomicSnapshot":
            _page_image(view)
            if query["cuts"] is not None and actual != query["cuts"][domain]:
                raise ValueError("Q 正文与请求固定向量不同")
            _page_integer(view.get("commit_ns"), "commit_ns")
            if (query["as_known_ns"] is not None
                    and int(view["commit_ns"]) > int(query["as_known_ns"])):
                raise ValueError("Q 正文晚于获知截止")
        elif view.get("base_cut") != query["cuts"][domain]:
            raise ValueError("Q watch 正文 base cut 与请求不同")
        else:
            _page_changes(view.get("changes"), domain, view["base_cut"], actual)
    if kind == "EconomicSnapshot":
        completeness = ("CompleteVector" if all(v["status"] == "Available" for v in value["domains"].values())
                        else "PartialVector")
        if value.get("completeness") != completeness:
            raise ValueError("Q snapshot 完整性标签与域状态不同")
        token = "economic-snapshot-" + digest({k: v for k, v in value.items() if k != "snapshot_token"})
        if value.get("snapshot_token") != token:
            raise ValueError("Q snapshot 原 token 摘要不同")
        if query["snapshot_token"] is not None:
            if token != query["snapshot_token"]:
                raise ValueError("Q snapshot 未返回请求 token")
        elif (value.get("as_known_ns") != query["as_known_ns"]
              or value.get("mode") != ("AsKnown" if query["cuts"] is not None
                                       or query["as_known_ns"] is not None else "Current")):
            raise ValueError("Q snapshot 模式或获知选择不同")


def read_economic_pages(query, fetch, *, session_id, session_generation,
                        producer_id, producer_epoch, manifest_hash, owner_epochs,
                        deadline, clock=time.monotonic, max_frame_bytes=1048576,
                        max_document_bytes=4194304):
    """显式页协议：统一组装核；fetch 保全 wire 并核公共信封。

    全部片、完整文档、语义身份和 Close 均在同一根期限内核验后才交付。
    调用方继续核原业务图像、watch 连续性及领域资格。
    """
    if (type(max_frame_bytes) is not int or not 0 < max_frame_bytes <= 1048576
            or type(max_document_bytes) is not int or not 0 < max_document_bytes <= 4194304):
        raise ValueError("分页帧/完整文档帽不合法")
    if (type(deadline) not in (int, float) or not math.isfinite(deadline)
            or not callable(fetch) or not callable(clock)):
        raise ValueError("须给出有限绝对期限与输送函数")
    read_deadline = min(deadline, clock() + 30)
    if type(session_id) is not str or not session_id:
        raise ValueError("分页须给出会话身份")
    _page_integer(session_generation, "session_generation", positive=True)
    _page_integer(producer_epoch, "producer_epoch", positive=True)
    _page_hash(manifest_hash, "manifest_hash")
    if type(owner_epochs) is not dict or not owner_epochs:
        raise ValueError("分页须给出可信 owner epochs")
    for domain, epoch in owner_epochs.items():
        if not (domain == "E" or (type(domain) is str and domain.startswith("B:")
                                 and len(domain) > 2 and not any(c.isspace() for c in domain))):
            raise ValueError("分页 owner 域非法")
        _page_integer(epoch, "owner epoch", positive=True)
    owner_epochs = dict(owner_epochs)
    public = producer_id == "Q:" + session_id
    if public:
        producer = "Q"
    elif len(owner_epochs) == 1:
        producer = next(iter(owner_epochs))
        if producer_id != producer + ":" + session_id or owner_epochs[producer] != producer_epoch:
            raise ValueError("分页 owner 身份或 epoch 不符")
    else:
        raise ValueError("分页生产者身份不符")
    kinds = ({"snapshot": "EconomicSnapshot", "watch": "EconomicWatch"} if public
             else {"ReadView": "EconomicView", "Watch": "EconomicChanges"})
    if type(query) is not dict or query.get("op") not in kinds:
        raise ValueError("分页只接受已声明的原只读操作")
    op = query["op"]
    expected_kind = kinds[op]
    query_fields = {"ReadView": ("op", "cut", "as_known_ns"), "Watch": ("op", "after_cut"),
                    "snapshot": ("op", "cuts", "as_known_ns", "snapshot_token"), "watch": ("op", "cuts")}
    _page_fields(query, query_fields[op], op)
    if op in ("ReadView", "Watch"):
        selected = query["cut" if op == "ReadView" else "after_cut"]
        if selected is not None:
            _page_cut(selected, producer)
        if op == "ReadView":
            if query["as_known_ns"] is not None:
                _page_integer(query["as_known_ns"], "as_known_ns")
            if selected is not None and query["as_known_ns"] is not None:
                raise ValueError("固定 cut 与 as_known_ns 不可混用")
    else:
        if query["cuts"] is not None:
            _page_fields(query["cuts"], owner_epochs, "cuts")
            for domain, cut in query["cuts"].items():
                _page_cut(cut, domain)
        elif op == "watch":
            raise ValueError("watch 须给出完整 base cuts")
        if op == "snapshot":
            token = query["snapshot_token"]
            if token is not None and (type(token) is not str or not token
                                      or query["cuts"] is not None or query["as_known_ns"] is not None):
                raise ValueError("snapshot token 不能混入新选择条件")
            if query["as_known_ns"] is not None:
                _page_integer(query["as_known_ns"], "as_known_ns")
                if query["cuts"] is not None:
                    raise ValueError("固定向量与 as_known_ns 不可混用")
    query = decode_frame(canonical(query))
    query_hash = digest(query)

    def remaining_ms():
        value = int((read_deadline - clock()) * 1000)
        if value <= 0:
            raise TimeoutError("完整分页查询达到总期限")
        return min(value, 30000)

    def call(payload):
        value = fetch(payload, remaining_ms())
        remaining_ms()
        # The fetch contract separately counts the COMPLETE transport envelope.
        if len(canonical(value)) > max_frame_bytes:
            raise ValueError("分页 payload 已超过帧帽")
        return value

    opened = call({"schema_revision": ECONOMIC_PAGE_REVISION, "op": "EconomicReadOpen",
                   "query": query, "requested_remaining_ms": str(remaining_ms())})
    _page_fields(opened, ("schema_revision", "kind", "descriptor", "descriptor_sha256"), "Open")
    if opened["schema_revision"] != ECONOMIC_PAGE_REVISION or opened["kind"] != "EconomicReadOpened":
        raise ValueError("Open 页协议版本或 kind 不符")
    descriptor = opened["descriptor"]
    _page_fields(descriptor, (
        "schema_revision", "transfer_id", "session_id", "session_generation", "manifest_hash",
        "producer_id", "producer_epoch", "query_sha256", "document_kind", "fixed_cuts",
        "owner_epochs", "document_bytes", "document_sha256", "chunk_bytes", "page_count"), "descriptor")
    descriptor_hash = _page_hash(opened["descriptor_sha256"], "descriptor_sha256")
    if descriptor_hash != digest(descriptor):
        raise ValueError("descriptor 摘要不符")
    transfer_id = descriptor["transfer_id"]
    if type(transfer_id) is not str or re.fullmatch(r"[0-9a-f]{32}", transfer_id) is None:
        raise ValueError("transfer_id 须为小写32hex")

    def close():
        response = call({"schema_revision": ECONOMIC_PAGE_REVISION, "op": "EconomicReadClose",
                         "transfer_id": transfer_id, "descriptor_sha256": descriptor_hash})
        _page_fields(response, ("schema_revision", "kind", "transfer_id", "descriptor_sha256"), "Close")
        if response != {"schema_revision": ECONOMIC_PAGE_REVISION, "kind": "EconomicReadClosed",
                        "transfer_id": transfer_id, "descriptor_sha256": descriptor_hash}:
            raise ValueError("Close 未确认同一 transfer")

    try:
        for key, expected in {
            "schema_revision": ECONOMIC_PAGE_REVISION, "session_id": session_id,
            "session_generation": session_generation, "manifest_hash": manifest_hash,
            "producer_id": producer_id, "producer_epoch": producer_epoch,
            "query_sha256": query_hash, "document_kind": expected_kind,
            "owner_epochs": owner_epochs, "chunk_bytes": str(ECONOMIC_PAGE_BYTES),
        }.items():
            if descriptor[key] != expected:
                raise ValueError("descriptor 身份或查询不符：" + key)
        _page_fields(descriptor["fixed_cuts"], owner_epochs, "fixed_cuts")
        for domain, cut in descriptor["fixed_cuts"].items():
            if cut is None:
                if not public:
                    raise ValueError("owner 固定 cut 不可为空")
            else:
                _page_cut(cut, domain)
        if not public:
            fixed_cut = descriptor["fixed_cuts"][producer]
            if op == "ReadView" and query["cut"] is not None and fixed_cut != query["cut"]:
                raise ValueError("descriptor 未固定请求 cut")
            if (op == "Watch" and query["after_cut"] is not None
                    and int(fixed_cut["commit_seq"]) < int(query["after_cut"]["commit_seq"])):
                raise ValueError("descriptor Watch 前沿早于请求 cut")
        document_bytes = _page_integer(descriptor["document_bytes"], "document_bytes", positive=True)
        if document_bytes > max_document_bytes:
            raise ValueError("分页完整文档超过聚合帽")
        page_count = _page_integer(descriptor["page_count"], "page_count", positive=True)
        if page_count != (document_bytes + ECONOMIC_PAGE_BYTES - 1) // ECONOMIC_PAGE_BYTES:
            raise ValueError("分页 page_count 与完整长度不符")
        document_hash = _page_hash(descriptor["document_sha256"], "document_sha256")
        remaining_ms()

        document = bytearray()
        for _ in range(page_count):
            offset = len(document)
            response = call({"schema_revision": ECONOMIC_PAGE_REVISION, "op": "EconomicReadPage",
                             "transfer_id": transfer_id, "descriptor_sha256": descriptor_hash,
                             "offset": str(offset), "requested_remaining_ms": str(remaining_ms())})
            _page_fields(response, ("schema_revision", "kind", "transfer_id", "descriptor_sha256", "offset",
                                    "raw_byte_count", "chunk_sha256", "data_base64", "next_offset", "done"), "Page")
            if (response["schema_revision"] != ECONOMIC_PAGE_REVISION or response["kind"] != "EconomicReadPage"
                    or response["transfer_id"] != transfer_id or response["descriptor_sha256"] != descriptor_hash
                    or response["offset"] != str(offset)):
                raise ValueError("Page 身份或连续 offset 不符")
            expected_bytes = min(ECONOMIC_PAGE_BYTES, document_bytes - offset)
            count = _page_integer(response["raw_byte_count"], "raw_byte_count", positive=True)
            next_offset = offset + count
            last = next_offset == document_bytes
            if (count != expected_bytes or response["next_offset"] != (None if last else str(next_offset))):
                raise ValueError("Page 长度/后继越界、重叠或缺口")
            if type(response["done"]) is not bool or response["done"] != (next_offset == document_bytes):
                raise ValueError("Page done 与完整终点不符")
            data = response["data_base64"]
            if type(data) is not str or len(data) != 4 * ((count + 2) // 3):
                raise ValueError("Page base64 类型或长度不符")
            try:
                chunk = base64.b64decode(data, validate=True)
            except (ValueError, binascii.Error) as exc:
                raise ValueError("Page base64 非法") from exc
            if len(chunk) != count or base64.b64encode(chunk).decode("ascii") != data:
                raise ValueError("Page base64 非规范或解码长度不同")
            if _page_hash(response["chunk_sha256"], "chunk_sha256") != hashlib.sha256(chunk).hexdigest():
                raise ValueError("Page 片摘要不同")
            document.extend(chunk)
            remaining_ms()
        if len(document) != document_bytes or hashlib.sha256(document).hexdigest() != document_hash:
            raise ValueError("分页完整长度或摘要不符")
        value = decode_frame(document)
        if canonical(value) != document:
            raise ValueError("分页文档不是原 canonical JSON")
        _verify_page_document(value, descriptor, query, producer=producer,
                              session_id=session_id, session_generation=session_generation,
                              manifest_hash=manifest_hash, producer_epoch=producer_epoch,
                              owner_epochs=owner_epochs)
        remaining_ms()
    except Exception:
        # Best effort is bounded by the SAME deadline; no replacement success on failure.
        try:
            close()
        except (OSError, ValueError, KeyError, TypeError, RecursionError):
            pass
        raise
    close()  # A failed Close does not manufacture a successful completed transfer.
    remaining_ms()
    return value


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--socket", required=True, type=Path)
    parser.add_argument("--request", required=True, type=Path)
    parser.add_argument("--timeout-ms", required=True, type=int)
    parser.add_argument("--max-frame-bytes", required=True, type=int)
    args = parser.parse_args()
    try:
        if args.max_frame_bytes <= 0:
            raise ValueError("帧上限必须为正数")
        with args.request.open("rb") as stream:
            raw = stream.read(args.max_frame_bytes + 1)
        if len(raw) > args.max_frame_bytes:
            raise ValueError("请求文件超过具名帧字节上限")
        request = decode_frame(raw)
        result = exchange(args.socket, request, timeout_ms=args.timeout_ms,
                          max_frame_bytes=args.max_frame_bytes)
    except (OSError, ValueError, UnicodeError, RecursionError) as exc:
        result = {"transport": "InvalidRequest", "detail": str(exc)}
    print(json.dumps(result, ensure_ascii=True, indent=2))
    return {"Received": 0, "DeliveryUnknown": 2, "TransportUnavailable": 3,
            "InvalidRequest": 4}[result["transport"]]


if __name__ == "__main__":
    sys.exit(main())
