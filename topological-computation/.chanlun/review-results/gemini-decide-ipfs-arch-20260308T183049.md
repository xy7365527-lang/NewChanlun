---
trigger: team-lead-request-ipfs-arch
target: ipfs-architecture-four-decisions
mode: decide
result: pass
model: gemini-3.1-pro-preview
timestamp: 2026-03-08T18:30
---

# IPFS 架构四决策 — Gemini Decide 结果

## 质询者判定

**全部决策成立。**

三步验证：
- 定义回溯：Gemini 正确识别 MFS 本地性矛盾（MFS 不跨节点同步），将其重新定位为"本地物化视图"而非"全局共享目录"——概念定位准确
- 边界条件：Gemini 自行给出所有翻转条件（广播风暴/urllib 长连接断开/悬空边/离线推演模式），未发现未识别盲区
- 推论检验：Pubsub 方向与现有代码架构（IPFSClient + CrossInstanceSync）一致，扩展路径清晰

---

## 核心矛盾识别

**MFS 本地性问题（质询前已存在）**：
MFS 是每个 IPFS 节点的本地 mutable filesystem。节点A的 `/shared/blocks/CID1` 只存在于节点A的 MFS 中。节点B的 `files_ls` 只看到自己的 MFS。当前 SharedLayer 的跨节点场景下无法工作。

Gemini 决策将 MFS 重新定位：不是"全局共享目录"，而是"当前节点对全局拓扑的本地物化视图（Materialized View）"。

---

## 决策 1+4：IPFS 索引机制与跨网络同步（联合决策）

**决策：Pubsub 实时广播（发现）+ MFS 本地物化视图（存储）**

数据流：
```
节点A 产生新 Block → IPFS upload → 获取 CID → pin → 写入本地 MFS
  → pubsub publish(topic="fengliang-events", CID)
  → 节点B 监听 pubsub → 收到 CID → IPFS cat → pin → 写入节点B本地 MFS
  → CrossInstanceSync.sync() 发现新块 → 注入本地图
```

实现要求：
- `IPFSClient` 增加 `pubsub_pub(topic, msg)` 和 `pubsub_sub(topic)` 两个方法
- `/api/v0/pubsub/pub` 和 `/api/v0/pubsub/sub`（chunked 响应）
- `pubsub_sub` 需要健壮的断线重连机制（Daemon 线程）
- MFS 从"全局共享索引"降级为"本地物化视图"——语义变更

拒绝的选项：
- IPNS：更新延迟分钟级，对 agree/negate 实时响应不可接受
- 纯 MFS 跨节点轮询：MFS 本地性，跨节点不可见

翻转条件：节点数激增导致广播风暴；urllib 长连接在 VPS 环境频繁断开且无法低成本重连。

---

## 决策 2：Relation 存储

**决策：独立 IPFS Block（当前方案A）+ Pubsub 广播**

- Relation 作为独立 JSON 写入 IPFS，内容必须包含 `target_block_cid`
- 不使用专门的 DAG API
- Relation CID 同样通过 Pubsub 广播给对等节点

理由：JSON 中包含目标 CID → IPFS 内容寻址天然构成不可变 Merkle DAG（隐式）。引入 DAG API 增加实现复杂度，不带来额外拓扑语义价值。

**风险：悬空边**（收到 Relation CID 但尚未收到其指向 Block 的 CID）
- 需要在 `CrossInstanceSync` 中处理依赖等待逻辑
- 处理路径：收到 Relation 时检查 target_block_cid 是否已在 known_blocks；未在则推迟处理（deferred list），等目标 Block 到达后再处理 Relation

翻转条件：需要对 Relation 图做复杂图遍历且 cat 性能不满足。

---

## 决策 3：降级策略

**决策：严格拒绝启动（方案A——已实现）**

维持 `SharedLayer` 构造函数中的 `raise RuntimeError`。

理由：
- IPFS 不可用 = 实体失去在共享拓扑空间中的物理躯体
- 允许"降级等待"= 幽灵状态（能计算但无法交互）= 绕过矛盾
- 崩溃显式化矛盾，由外部进程管理器（systemd/Docker）处理重试退避

翻转条件：系统演进出"离线推演/梦境模式"（明确允许不落链的内部演化）。

---

## 需要修改的代码

### chain/ipfs_client.py

增加 pubsub 方法：

```python
def pubsub_pub(self, topic: str, data: str | bytes) -> None:
    """向 pubsub topic 发布消息。"""
    if isinstance(data, str):
        data = data.encode("utf-8")
    encoded_topic = urllib.parse.quote(topic, safe="")
    boundary = b"----PubSubBoundary"
    body = (
        b"--" + boundary + b"\r\n"
        b'Content-Disposition: form-data; name="file"\r\n\r\n'
        + data + b"\r\n"
        b"--" + boundary + b"--\r\n"
    )
    req = urllib.request.Request(
        f"{self.api_url}/api/v0/pubsub/pub?arg={encoded_topic}",
        data=body,
        headers={"Content-Type": f"multipart/form-data; boundary={boundary.decode()}"},
        method="POST",
    )
    with urllib.request.urlopen(req, timeout=10) as resp:
        resp.read()

def pubsub_sub(self, topic: str):
    """订阅 pubsub topic，返回生成器（每次 yield 一条消息的 data 字节）。

    调用者负责在独立线程中运行此方法，并处理断线重连。
    """
    encoded_topic = urllib.parse.quote(topic, safe="")
    req = urllib.request.Request(
        f"{self.api_url}/api/v0/pubsub/sub?arg={encoded_topic}",
        method="POST",
    )
    with urllib.request.urlopen(req, timeout=None) as resp:
        for line in resp:
            if line.strip():
                msg = json.loads(line.decode("utf-8"))
                # data 字段是 base64 编码
                import base64
                yield base64.b64decode(msg.get("data", ""))
```

### swarm/shared_layer.py

`write_block` 和 `write_relation` 写入 MFS 后，调用 `pubsub_pub` 广播 CID：

```python
# write_block 末尾
self._ipfs.pubsub_pub("fengliang-events", json.dumps({"type": "block", "cid": cid}))
return cid

# write_relation 末尾
self._ipfs.pubsub_pub("fengliang-events", json.dumps({"type": "relation", "cid": cid}))
```

### swarm/cross_instance.py

增加 pubsub 监听线程 + 悬空边处理：

```python
# __init__ 中增加
self._deferred_relations: list[dict] = []  # 悬空边等待队列

# sync() 中增加悬空边处理
def _try_process_deferred(self) -> None:
    """处理之前因目标 block 未到达而推迟的 relations。"""
    still_deferred = []
    for rel in self._deferred_relations:
        if rel.get("target_block_cid") in self.known_blocks:
            self._process_relation(rel)
        else:
            still_deferred.append(rel)
    self._deferred_relations = still_deferred
```

---

## 下游推论

1. Pubsub 要求 IPFS daemon 启用 pubsub（`--enable-pubsub-experiment` 或新版本默认开启）
2. Private network + pubsub：两个节点需要 bootstrap peer 连接才能交换 pubsub 消息
3. setup_private.sh 需要增加 `ipfs bootstrap add /ip4/VPS_IP/...` 步骤
4. MFS 语义变更不影响现有代码（write_block 仍写 MFS，只是不再期望对端能 ls 到）

---

## 谱系引用

- 20260308b 决策：穿越位置通过 SharedLayer 区块事件（方案C）
- 090号：严格性语法规则（幽灵状态 = 绕过矛盾）
- no-workaround 规则：网络断开是矛盾，崩溃显式化矛盾

## 影响声明

涉及模块：
- `chain/ipfs_client.py`：增加 pubsub_pub / pubsub_sub
- `swarm/shared_layer.py`：write_block / write_relation 增加 pubsub 广播
- `swarm/cross_instance.py`：增加悬空边处理逻辑 + pubsub 监听线程
- `chain/setup_private.sh`：增加 bootstrap peer 配置步骤
