---
trigger: team-lead-request-v2
target: multi-instance-view
mode: decide
result: pass
model: gemini-3.1-pro-preview
timestamp: 2026-03-08T02
supersedes: gemini-decide-multi-instance-view-20260308.md
---

# 多实例穿越显示数据通道 — Gemini Decide v2（方案A被推翻）

## 质询者判定

**决策成立，且推翻上轮方案A决策。**

三步验证：
- 定义回溯：SharedLayer 的实际设计是 Event Sourcing（内容寻址事件日志），Gemini 的"区块事件"论据与代码实现完全一致
- 边界条件：Gemini 自行给出翻转条件（IPFS 同步延迟到分钟级），未发现未识别的盲区
- 推论检验：CrossInstanceSync 已有轮询逻辑，扩展 traversal_position 块类型为局部修改，前端只需增加一种 WS 消息类型

---

## 决策：方案C — 通过区块拓扑（SharedLayer）

### 核心判断：穿越位置是"区块事件"，不是"观察信号"

- 观察信号：随风飘散，前端做聚合观察者（方案A）
- 区块事件：系统演化历史的一部分，必须持久化到共享层（方案C）

实例A在节点X停留是一个**客观发生的事实**，可被实例B观察、记录、甚至作为概念材料摄入。这与 fold/negate 事件的本体论地位相同。

### 拒绝方案B（IPFS pubsub）

- `chain/ipfs_client.py` 没有 pubsub 实现，需要从零开发
- IPFS pubsub 是独立于 SharedLayer 的第二套通信机制，引入概念冗余
- 两套机制并存违反单一数据源原则

### 拒绝方案A（前端直连）

- 前端需硬编码所有实例地址
- 穿越位置被降级为"观察信号"，丢失了事件历史和因果可追溯性
- 已废弃：移除 VITE_INSTANCE_LIST 方案

### 拒绝混合方案

- 实时状态走方案A + 穿越记忆走方案C = 两条数据通道并存
- 违反概念一致性

---

## 数据流图

```text
[实例A 后端，每次穿越节点]
  │ 节流（仅当停留>2s 或发生 fold/negate）
  ▼
写入块：{"type": "traversal_position", "instance": "A", "position_label": "概念X", "step": N, "timestamp": ...}
  │
  ▼
SharedLayer（本地文件 / IPFS 目录同步）
  │
  ▼
[实例B 后端，CrossInstanceSync 轮询]
  │ 1. read_new_blocks 发现新块
  │ 2. type=="traversal_position" → 提取 position_label
  │ 3. 更新本地维护的 peer_positions[instance_id]
  ▼
WS 推送给前端：{"type": "peer_position", "instance": "A", "position_label": "概念X"}
  │
  ▼
[前端 Zustand store]
  │ peersPositions: { "A": {posLabel: "概念X", color: "#ff8c42"} }
  ▼
TraversalOverlay 渲染实例A的同心圆在"概念X"节点上
```

---

## 需要修改的代码

### 后端修改

**1. swarm/shared_layer.py** — 无需修改
（SharedLayer 已经支持任意块类型，`write_block` 接受任意 dict）

**2. swarm/cross_instance.py** — 扩展 sync() 处理 traversal_position 块

```python
# 在 sync() 的 for block 循环中增加分支：
if block.get("type") == "traversal_position":
    self._update_peer_position(block)
    # 不走注入流程（不修改概念图）
    continue  # 跳过 _inject_external_operation
else:
    # 原有逻辑
    self._inject_external_operation(block)
    self._interpret_and_respond(block, block_hash)

def _update_peer_position(self, block: dict) -> None:
    instance_id = block.get("instance")
    if instance_id and instance_id != self.instance_id:
        self.daemon.peer_positions[instance_id] = {
            "position_label": block.get("position_label", ""),
            "step": block.get("step", 0),
            "timestamp": block.get("timestamp", 0),
        }
```

**3. daemon.py（逢亮主体）** — 两处修改

a. 增加 peer_positions 字典：
```python
self.peer_positions: dict[str, dict] = {}  # instance_id -> {position_label, step, timestamp}
```

b. 步进时写入穿越块（节流）：
```python
# 在 step() 方法中，满足节流条件时：
if self.cross_instance_sync and should_write_position:
    self.shared_layer.write_block({
        "type": "traversal_position",
        "instance": self.instance_id,
        "position_label": current_vertex.content or current_vertex.id,
        "step": self.total_steps,
        "timestamp": time.time(),
    })
```

**4. daemon_server.py（WS 服务器）** — 增加 peer_position 推送

在 CrossInstanceSync 发现新 traversal_position 块时（或定期扫描后），推送给所有 WS 客户端：
```python
peer_position_msg = {
    "type": "peer_position",
    "instance": instance_id,
    "position_label": position_label,
}
await ws.send(json.dumps(peer_position_msg))
```

### 前端修改

**5. frontend/src/hooks/useStore.ts** — 增加 peersPositions

```typescript
peersPositions: Record<string, { posLabel: string; color: string }>;
updatePeerPosition: (instanceId: string, posLabel: string) => void;
```

**6. frontend/src/hooks/useDaemonWS.ts** — 增加 peer_position 消息处理

在 handleBatch 中增加：
```typescript
} else if (msg.type === "peer_position") {
    store.updatePeerPosition(msg.instance, msg.position_label);
}
```

**7. frontend/src/components/TraversalOverlay.tsx** — 新建
（与上轮设计相同，只是数据来源改为 peersPositions）

**8. frontend/src/components/TopologyView.tsx** — 暴露 nodePositionsRef + transformRef
（与上轮设计相同）

**9. frontend/src/App.tsx** — 移除 VITE_INSTANCE_LIST，移除多路 WS 连接逻辑

---

## 陷阱列表

1. **存储膨胀**：高频穿越（1步/秒）会生成大量微块。
   - 缓解：后端节流，仅在停留>2s 或发生拓扑事件（fold/negate）时写入

2. **traversal_position 块不走注入流程**：必须在 CrossInstanceSync 中显式跳过，否则会把"位置"当成顶点/边注入概念图

3. **SharedLayer 跨主机同步**：本地多进程可共享文件目录；跨主机需要 IPFS 目录同步或 NFS 挂载——此部分是运维问题，不是代码问题

4. **peer_positions 过期**：实例A离线后，最后已知位置应如何处理？
   - 推荐：加 timestamp，前端根据时间差（>30s）判断为"离线"，隐藏标记

5. **instance_id 来源**：每个实例需要唯一稳定的 `instance_id`（如 hostname:port 或 UUID）

---

## 与上轮决策的差异

| 维度 | 上轮决策（方案A） | 本轮决策（方案C） |
|------|------------------|------------------|
| 数据通道 | 前端多路 WS | SharedLayer 区块事件 |
| 前端架构 | 多个 DaemonConnection 组件 | 单一 WS + peer_position 消息类型 |
| 实例发现 | 硬编码 VITE_INSTANCE_LIST | 无需前端发现 |
| 穿越位置本体 | 观察信号 | 区块事件 |
| 历史持久性 | 无（内存临时） | 有（SharedLayer 持久化） |
| 后端改动量 | 无 | 中（cross_instance + daemon + daemon_server） |
| 前端改动量 | 中 | 低（单一 WS，增加消息类型） |
