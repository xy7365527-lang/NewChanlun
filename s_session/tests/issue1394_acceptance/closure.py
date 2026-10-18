"""独立 s-content/1 有类型遍历；不调用 Rust verifier/reducer，不借本地缓存补流。

依据 b46 的 s_content.rs 标签/帧/ContentRef 与 sequence.rs 的唯一分叉规则。
这里核内容闭包，不证明 S 的业务 authority、六族语义或产品资源硬隔离。
"""
from dataclasses import dataclass
import hashlib
import math
import re
import time
from driver import require, exact


DOMAIN = b"NewChanlun/SContent/1\0"
KINDS = {0: "null", 1: "bool", 2: "bool", 3: "string", 4: "number",
         5: "array", 6: "object", 7: "json_text", 8: "template_batch"}


def node_root(tag: int, body: bytes) -> str:
    require(type(tag) is int and 0 <= tag <= 255 and type(body) is bytes, "节点类型错误")
    return "sc1-" + hashlib.sha256(DOMAIN + bytes([tag]) + len(body).to_bytes(8, "big") + body).hexdigest()


def root_bytes(root: str) -> bytes:
    require(isinstance(root, str) and re.fullmatch(r"sc1-[0-9a-f]{64}", root), "内容根格式错误")
    return bytes.fromhex(root[4:])


@dataclass(frozen=True)
class Node:
    root: str
    tag: int
    body: bytes


@dataclass
class Budget:
    max_nodes: int
    max_bytes: int
    max_visits: int
    deadline: float
    nodes: int = 0
    bytes_read: int = 0
    visits: int = 0

    def check(self):
        require(all(type(n) is int and n > 0 for n in
                    (self.max_nodes, self.max_bytes, self.max_visits)), "闭包资源必须显式冻结")
        require(type(self.deadline) in (float, int) and math.isfinite(self.deadline)
                and time.monotonic() < self.deadline, "闭包同一绝对截止耗尽")
        require(self.nodes <= self.max_nodes and self.bytes_read <= self.max_bytes
                and self.visits <= self.max_visits, "闭包累计预算耗尽")


def verify(target: dict, received: list[Node], source, budget: Budget, *, allow_template=False) -> dict:
    """source(root)->Node 独立读取持久原件；received 是实际收到的严格升序流。"""
    require(allow_template is False, "模板位置/出生表未接线；不能以开关批准任意 0x08")
    exact(target, {"format", "root", "kind", "extent"}, "ContentRef")
    require(target["format"] == "s-content/1", "闭包格式错误")
    root_bytes(target["root"])
    budget.check()
    wire = {}
    prior = ""
    for node in received:
        require(type(node) is Node and node.root > prior, "闭包流重复/乱序")
        require(node_root(node.tag, node.body) == node.root, "闭包流摘要错误")
        budget.nodes += 1
        budget.bytes_read += len(node.body) + len(DOMAIN) + 9
        budget.check()
        wire[node.root] = node
        prior = node.root
    require(bool(wire), "缺闭包原件")
    seen, memo, active = set(), {}, set()
    # key=(root, context, count, json_only)。同摘要的不同有类型上下文不互相豁免。
    initial = (target["root"], "value", None, False)
    stack = [(initial, False, None)]

    def ref(raw):
        require(len(raw) == 32, "边摘要长度错误")
        return "sc1-" + raw.hex()

    def u64(raw):
        require(len(raw) == 8, "U64 长度错误")
        n = int.from_bytes(raw, "big")
        require(n < 2**63, "U64 超界")
        return n

    while stack:
        key, leaving, state = stack.pop()
        budget.visits += 1
        budget.check()
        if key in memo:
            continue
        root, context, count, json_only = key
        if not leaving:
            require(key not in active, "闭包环")
            active.add(key)
            require(root in wire, "本地缓存不豁免：流缺根可达节点")
            node = source(root)
            require(type(node) is Node and node.root == root
                    and node_root(node.tag, node.body) == root, "持久节点摘要错误")
            budget.bytes_read += len(node.body) + len(DOMAIN) + 9
            budget.check()
            require(node == wire[root], "收到节点与持久原字节不相同")
            seen.add(root)
            tag, body, children = node.tag, node.body, []
            if context == "value":
                require(tag in KINDS, "值边指向内部节点")
                require(not json_only or tag not in (7, 8), "json_text 内部不允许扩展值")
                require(tag != 8 or allow_template, "模板占位未授权")
                if tag in (0, 1, 2, 8):
                    require(not body, "标量 body 必须为空")
                elif tag == 7:
                    children = [(ref(body), "value", None, True)]
                else:
                    require(len(body) == 40, "有长度值 body 必须为 40 字节")
                    n = u64(body[:8])
                    flavor = "bytes" if tag in (3, 4) else "values" if tag == 5 else "fields"
                    children = [(ref(body[8:]), flavor, (n + 4095) // 4096 if flavor == "bytes" else n, json_only)]
            elif context in ("bytes", "values", "fields"):
                if count == 0:
                    require(tag == 0x11 and not body, "空序列形状错误")
                elif count == 1:
                    require(tag == 0x12, "单叶序列形状错误")
                    child_context = {"bytes": "chunk", "values": "value", "fields": "field"}[context]
                    children = [(ref(body), child_context, None, json_only)]
                else:
                    require(tag == 0x13 and len(body) == 72 and u64(body[:8]) == count,
                            "分支计数/形状错误")
                    split = 1 << ((count - 1).bit_length() - 1)
                    children = [(ref(body[8:40]), context, split, json_only),
                                (ref(body[40:]), context, count - split, json_only)]
            elif context == "chunk":
                require(tag == 0x10 and 0 < len(body) <= 4096, "字节块形状错误")
            else:
                require(context == "field" and tag == 0x14 and len(body) == 64, "字段边形状错误")
                children = [(ref(body[:32]), "value", None, json_only),
                            (ref(body[32:]), "value", None, json_only)]
            stack.append((key, True, (tag, body, children)))
            stack.extend((c, False, None) for c in reversed(children))
            continue
        tag, body, children = state
        values = [memo[c] for c in children]
        if context == "chunk":
            result = body
        elif context == "field":
            require(values[0][0] == "string", "对象键不是 string")
            result = values[0][2]
        elif context in ("bytes", "values", "fields"):
            if context == "values":
                result = count
            elif context == "fields":
                keys = [] if count == 0 else [values[0]] if count == 1 else values[0] + values[1]
                require(all(a < b for a, b in zip(keys, keys[1:])), "对象键重复/不按 UTF8 排序")
                result = keys
            else:
                chunks = [] if count == 0 else [values[0]] if count == 1 else values[0] + values[1]
                require(all(len(c) == 4096 for c in chunks[:-1]), "非末字节块不满 4096")
                result = chunks
        else:
            extent = int.from_bytes(body[:8], "big") if tag in (3, 4, 5, 6) else None
            scalar = None
            if tag in (3, 4):
                scalar = b"".join(values[0])
                require(len(scalar) == extent, "声明字节长度不相同")
                if tag == 3:
                    scalar.decode("utf-8", errors="strict")
                else:
                    require(re.fullmatch(rb"-?(0|[1-9][0-9]*)(\.[0-9]+)?([eE][+-]?[0-9]+)?", scalar),
                            "数字 token 非法")
            result = (KINDS[tag], extent, scalar)
        memo[key] = result
        active.remove(key)
    kind, extent, _ = memo[initial]
    require(kind == target["kind"] and (None if extent is None else str(extent)) == target["extent"],
            "ContentRef kind/extent 不相同")
    require(seen == set(wire), "闭包流含根不可达额外节点")
    budget.check()
    return {"scope": "CONTENT_CLOSURE_ONLY", "root": target, "nodes": sorted(seen),
            "execution_authorized": False, "product_status": "NOT_RUN"}
