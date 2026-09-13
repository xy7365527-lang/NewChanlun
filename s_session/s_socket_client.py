#!/usr/bin/env python3
"""#1372：S 本地持续入口的一次输送，不生成或重试业务身份。

每连接一个 JSON 行；上限和总期限显式给出。开始发送后若没有完整合法响应，
结果仅为 DeliveryUnknown，调用方须按原身份查询持久收据，不能改身份重做。
Received/退出0只表示完整输送，S业务成功及响应身份仍由调用方按合同核实。
"""

import argparse
import hashlib
import json
import socket
import sys
import time
from pathlib import Path


def _unique_object(pairs):
    out = {}
    for key, value in pairs:
        if key in out:
            raise ValueError("重复 JSON 键：" + key)
        out[key] = value
    return out


def _reject_number(value):
    raise ValueError("协议不接受非精确数值：" + value)


def decode_frame(raw):
    result = json.loads(raw.decode("utf-8"), object_pairs_hook=_unique_object,
                        parse_float=_reject_number, parse_constant=_reject_number)
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
    if producer not in ("S", "Q") or type(producer_epoch) is not str or not producer_epoch:
        raise ValueError("须显式指定响应生产者及epoch")
    expected_namespace = "s-session/replies" if producer == "S" else "s-observe/replies"
    if (response["producer_id"] != producer + ":" + request["session_id"]
            or response["producer_epoch"] != producer_epoch
            or response["source_epoch"] != producer_epoch
            or response["source_namespace"] != expected_namespace):
        raise ValueError("响应生产者/epoch不符")
    def digest(value):
        raw = json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"), allow_nan=False).encode("utf-8")
        return hashlib.sha256(raw).hexdigest()
    payload_hash = digest(response["payload"])
    cause = {key: request[key] for key in ("source_namespace", "source_epoch", "message_id", "payload_hash")}
    reply_id = "reply-" + digest([request["source_namespace"], request["source_epoch"], request["message_id"], payload_hash])
    if response["payload_hash"] != payload_hash or response["causal_refs"] != [cause] or response["message_id"] != reply_id:
        raise ValueError("响应内容摘要或原请求因果绑定不符")
    return response["payload"]


def exchange(socket_path, request, *, timeout_ms, max_frame_bytes):
    """输送错误与 S 的业务响应分离；有限等待，不自动重发或解释成未发生。"""
    if type(timeout_ms) is not int or timeout_ms <= 0:
        raise ValueError("timeout_ms 必须是正整数")
    if type(max_frame_bytes) is not int or max_frame_bytes <= 0:
        raise ValueError("max_frame_bytes 必须是正整数")
    if type(request) is not dict or not request:
        raise ValueError("request 必须是非空对象")
    body = json.dumps(request, ensure_ascii=False, sort_keys=True, separators=(",", ":"),
                      allow_nan=False).encode("utf-8") + b"\n"
    if len(body) > max_frame_bytes:
        raise ValueError("请求超过具名帧字节上限")
    # 同一解码约束核调用方直接传入的值，不能绕过文件入口数值校验。
    decode_frame(body)
    deadline = time.monotonic() + timeout_ms / 1000
    started_send = False

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
            response = bytearray()
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
                    remaining()  # 解析本身也在一次输送总期限内。
                    return {"transport": "Received", "response": decoded}
                response.extend(chunk)
                if len(response) > max_frame_bytes:
                    raise ValueError("响应超过具名帧字节上限")
                if b"\n" in response:
                    _, tail = response.split(b"\n", 1)
                    if tail:
                        raise ValueError("单次输送收到多帧或额外内容")
                    # 等待本连接写端关闭，避免分段到达的第二帧被误认合法。
    except (OSError, ValueError, RecursionError) as exc:
        return {"transport": "DeliveryUnknown" if started_send else "TransportUnavailable",
                "message_id": request.get("message_id"), "detail": str(exc)}


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
