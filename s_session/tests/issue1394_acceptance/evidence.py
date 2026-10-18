"""原件全集与有序出现序号核验；Received 仅表示收到 bytes。"""
from dataclasses import dataclass
from pathlib import Path
import driver as d
import scheduler as s


@dataclass(frozen=True)
class Original:
    path: Path
    sha256: str

    def read(self, budget):
        d.require(self.path.is_absolute() and self.path.is_file() and not self.path.is_symlink(),
                  "原件不是绝对普通文件")
        before = d.file_stat(self.path)
        raw = d.read_original(self.path, budget)
        d.require(d.file_stat(self.path) == before and d.sha(raw) == self.sha256, "原件变化或摘要不符")
        budget.check()
        return raw


@dataclass(frozen=True)
class Exchange:
    request: s.Request
    request_original: Original
    reply_original: Original


def verify_exchanges(expected, exchanges, budget, *, session_id, session_generation, domain_snapshots):
    """expected 由冻结完整请求序列提供；cut 是原完整 cut 的 canonical SHA。"""
    d.require(tuple(x.request for x in exchanges) == tuple(expected), "原请求回执全集缺漏/额外/重排")
    receipt_keys, occurrence, results = {}, {}, []
    for exchange in exchanges:
        request, binding = exchange.request, exchange.request.binding
        raw = exchange.request_original.read(budget)
        reply_raw = exchange.reply_original.read(budget)
        d.require(d.sha(raw) == binding.request_sha, "原请求摘要偏离冻结序列")
        req, reply = d.strict(raw), d.strict(reply_raw)
        d.require(req["schema_revision"] == "economic-session/2"
                  and req["session_id"] == session_id and req["session_generation"] == session_generation,
                  "原请求协议/会话错绑")
        d.require(req["payload_hash"] == d.sha(d.canonical(req["payload"]))
                  and req["payload"]["op"] == binding.operation, "原请求 payload/操作不匹配")
        d.require(reply.get("kind") not in {None, "Received"}
                  and reply["kind"] == request.expected_outcome, "传输状态不能替代冻结业务结果")
        d.require(reply["domain_id"] == binding.domain and reply["receipt_key"] == binding.receipt_key,
                  "回执 owner/key 不符")
        d.require(s.digest(reply["cut"]) == binding.cut, "回执不是冻结同 cut")
        d.require(binding.domain in domain_snapshots, "缺持久原库")
        snapshot = domain_snapshots[binding.domain]
        d.require(snapshot["db"] == binding.db and snapshot["owner"] == binding.owner
                  and snapshot["epoch"] == binding.epoch, "原库 owner/epoch/路径错绑")
        key = (binding.domain, binding.receipt_key)
        d.require(request.ordinal == occurrence.get(key, 0), "幂等出现序号不连续")
        occurrence[key] = request.ordinal + 1
        original = (raw, reply_raw)
        d.require(receipt_keys.setdefault(key, original) == original, "幂等原请求/回执字节变化")
        d.require(snapshot["requests"].get(binding.receipt_key) == req
                  and snapshot["receipts"].get(binding.receipt_key) == reply, "原件与持久库不相同")
        results.append(s.BusinessReceipt(request, d.sha(raw), d.sha(reply_raw),
                                         "ORIGINALS_VERIFIED", reply["kind"]))
    for domain, snapshot in domain_snapshots.items():
        keys = {key for dom, key in receipt_keys if dom == domain}
        d.require(set(snapshot["requests"]) == keys == set(snapshot["receipts"]),
                  "持久原请求/回执有额外或缺失身份")
    result = tuple(results)
    budget.check()
    return result


def same_cut(query_original, history_original, expected_cut, budget):
    query, history = d.strict(query_original.read(budget)), d.strict(history_original.read(budget))
    d.require(query["cut"] == history["cut"] == expected_cut, "query/history 非同 cut")
    d.require(query["image"] == history["image"], "同 cut 查询与持久历史图像不符")
    result = d.sha(d.canonical({"query": query, "history": history}))
    budget.check()
    return result


def recovery_exact(before, recovered, *, seam, pending_key, request, receipt):
    """持久字典按原 key 全集比对；before_commit 只允许那一个具名事务追加。"""
    d.require(seam in {"before_commit", "after_commit"}, "未知持久缝")
    for family, original in (("requests", request), ("receipts", receipt)):
        expected = dict(before[family])
        if seam == "before_commit":
            d.require(pending_key not in expected, "before_commit 已有业务提交")
            expected[pending_key] = original
        else:
            d.require(expected.get(pending_key) == original, "after_commit 缺原持久提交")
        d.require(recovered[family] == expected, "恢复丢失/重复/改写原请求或回执")
    old = before["commits"]
    new = recovered["commits"]
    d.require(bool(old), "恢复缺基线历史")
    d.require(new == old if seam == "after_commit" else
              len(new) == len(old) + 1 and new[:-1] == old
              and new[-1]["seq"] == old[-1]["seq"] + 1
              and new[-1]["receipt_key"] == pending_key, "恢复提交历史不重不漏失败")
    return {"scope": "PERSISTENT_RECOVERY_ONLY", "product_status": "NOT_RUN"}
