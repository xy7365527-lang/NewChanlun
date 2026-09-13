"""#1374：现有正式 Q 服务的经济只读路由；没有经济写入口。"""

from pathlib import Path
import time

from economic_query import EconomicQueryError, integer, text
from s_socket_client import canonical, decode_frame, digest, reply_envelope


HEADER = {"schema_revision", "session_id", "session_generation", "source_namespace", "source_epoch",
          "message_id", "producer_id", "producer_epoch", "payload_hash", "causal_refs", "payload"}


def validate_request(request, query):
    if type(request) is not dict or set(request) != HEADER:
        raise EconomicQueryError("公共头字段不完整或多余")
    if request["schema_revision"] != "economic-session/1":
        raise EconomicQueryError("仅支持 economic-session/1")
    if (request["session_id"], request["session_generation"]) != (query.session, query.generation):
        raise EconomicQueryError("请求不是本会话/化身")
    for key in ("source_namespace", "source_epoch", "message_id", "producer_id"):
        text(request[key], key)
    integer(request["producer_epoch"], "producer_epoch", positive=True)
    if type(request["payload"]) is not dict or request["payload_hash"] != digest(request["payload"]):
        raise EconomicQueryError("请求内容摘要不同")
    if type(request["causal_refs"]) is not list:
        raise EconomicQueryError("causal_refs 必须为数组")
    for cause in request["causal_refs"]:
        if type(cause) is not dict or set(cause) != {"source_namespace", "source_epoch", "message_id", "payload_hash"}:
            raise EconomicQueryError("请求原因果字段不完整")
        for key, value in cause.items():
            text(value, key)
    return request["payload"]


def get(handler):
    static = {"/economic.html": ("economic.html", "text/html; charset=utf-8"),
              "/tb03a-client.js": ("tb03a-client.js", "application/javascript; charset=utf-8")}
    if handler.path in static:
        name, content_type = static[handler.path]
        try:
            raw = (Path(handler.browser_path).parent / name).read_bytes()
            handler._send_bytes(raw, content_type=content_type)
        except OSError:
            handler._send_json({"ok": False, "error": "not found"}, 404)
        return True
    if handler.path != "/api/economic/ready":
        return False
    query = getattr(handler.server, "economic_query", None)
    if query is None:
        handler._send_json({"ok": False, "error": "EconomicQueryNotConfigured"}, 503)
    else:
        handler._send_json({"schema_revision": "economic-discovery/1", "session_id": query.session,
                            "session_generation": query.generation, "producer_epoch": query.query_epoch,
                            "manifest_hash": query.manifest_hash, "domains": list(query.nodes),
                            "max_reply_bytes": str(query.resources["max_reply_bytes"]),
                            "operation_timeout_ms": str(query.resources["operation_timeout_ms"]),
                            "scope": "TestOnlyRecordedEconomics", "configured": True})
    return True


def post(handler):
    if handler.path not in ("/api/economic/snapshot", "/api/economic/watch"):
        return False
    query = getattr(handler.server, "economic_query", None)
    if query is None:
        handler._send_json({"ok": False, "error": "EconomicQueryNotConfigured"}, 503)
        return True
    request = None
    try:
        lengths = handler.headers.get_all("Content-Length", [])
        if len(lengths) != 1 or handler.headers.get("Transfer-Encoding") is not None:
            raise EconomicQueryError("需要唯一 Content-Length")
        size = integer(lengths[0], "Content-Length", positive=True)
        if size > min(query.resources["max_frame_bytes"], handler.server.resources["max_frame_bytes"]):
            raise EconomicQueryError("请求超过事前帧上限")
        deadline = time.monotonic() + handler.server.resources["socket_read_timeout_ms"] / 1000
        chunks, left = [], size
        while left:
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise EconomicQueryError("读取请求达到总期限")
            handler.connection.settimeout(remaining)
            part = handler.rfile.read1(min(left, 65536))
            if not part:
                raise EconomicQueryError("请求被截断")
            chunks.append(part)
            left -= len(part)
        candidate = decode_frame(b"".join(chunks))
        payload = validate_request(candidate, query)
        request = candidate
        op = handler.path.rsplit("/", 1)[1]
        if payload.get("op") != op:
            raise EconomicQueryError("路由与 op 不符")
        result = query.snapshot(payload) if op == "snapshot" else query.watch(payload)
        status = 200
    except (ValueError, KeyError, TypeError, UnicodeError, RecursionError, OSError) as exc:
        result, status = {"ok": False, "error": "EconomicQueryUnavailable", "detail": str(exc)}, 400
    if request is None:
        handler._send_json(result, status)
    else:
        response = reply_envelope(request, result, session_id=query.session,
                                  session_generation=query.generation, namespace="economic-observe/replies",
                                  producer_id="Q:" + query.session, producer_epoch=query.query_epoch)
        raw = canonical(response)
        if len(raw) > min(query.resources["max_reply_bytes"], handler.server.resources["max_reply_bytes"]):
            handler._send_json({"ok": False, "error": "QueryBudgetExceeded"}, 503)
        else:
            handler._send_bytes(raw, status)
    return True
