---
trigger: team-lead-request
target: multi-instance-view
mode: decide
result: pass
model: gemini-3.1-pro-preview
timestamp: 2026-03-08
---

# 多实例同步穿越显示 — Gemini Decide 结果

## 质询者判定

**决策成立**。Gemini 的四个维度决策均通过三步验证：
- 定义回溯：正确引用约束（避免额外后端服务、不改 WS 格式）
- 反边界条件均已被 Gemini 自行识别（实例数量膨胀到 20+ 时推翻方案A）
- 下游推论与现有技术栈（React/Zustand/d3）一致

---

## 决策结论（四维度）

### 1. 数据聚合架构：方案A（前端直连多个 WS）

理由：
- 2-5 个实例规模，N 个 WS 连接可控
- 约束要求不引入额外后端服务
- "前端是唯一观察者"概念清晰，无聚合中间层

边界翻转条件：实例数量膨胀到 20+，或引入跨实例协调逻辑

### 2. 视觉设计

| 维度 | 决策 | 理由 |
|------|------|------|
| 实例区分 | 不同颜色脉冲（蓝/橙/绿…） | 颜色是最低渲染开销的区分方式 |
| 多实例同节点 | 同心圆叠加（半径递增） | 拓扑同构映射，不改变节点坐标 |
| 历史轨迹 | 不显示历史轨迹 | 拓扑图是结构级别，非时间序列 |
| 视角跟随 | 不自动跟随 | 多实例并发下视角归属逻辑冲突 |

### 3. 实例管理

- 实例列表：硬编码 `INSTANCE_LIST` env 变量（`ws://A:port,ws://B:port`）
- 离线处理：隐藏该实例穿越标记（对象消亡即消亡，拒绝幽灵节点）
- 主实例：列表第一个实例为主实例，承接 chat/query 交互

### 4. 渲染策略

- 穿越标记与底图分离：新建 `TraversalOverlay.tsx` SVG 层
- Zustand 层对 step 消息做 100ms 节流
- Overlay 层必须 `pointer-events: none`，避免遮挡拓扑节点点击区域

---

## 数据流图

```text
.env: INSTANCE_LIST="ws://local:8765,ws://vps:8765"
  => App.tsx 解析，映射为 [{id:"local", url:"ws://local:8765"}, ...]
  => 渲染 <DaemonConnection id="local" url="..." />
      渲染 <DaemonConnection id="vps"   url="..." />
  => 各 DaemonConnection 建立 WS，接收 step 消息
     -> store.updateInstance("local", { position: "nodeLabel", step: N })
     -> store.updateInstance("vps",   { position: "nodeLabel", step: N })
  => Zustand Store:
     instances: {
       "local": { posLabel: "概念A", color: "#4a9eff", online: true },
       "vps":   { posLabel: "概念B", color: "#ff8c42", online: true }
     }
  => TopologyView.tsx (底图不感知实例，traversalPosition prop 废弃或指向主实例)
  => TraversalOverlay.tsx:
     - 订阅 store.instances
     - 接收父层传入的 d3 zoom transform（nodePositionsRef）
     - 按 posLabel 查找节点坐标
     - 渲染对应颜色同心圆
```

---

## 组件结构建议（基于现有代码库）

### 新建文件

- `frontend/src/components/DaemonConnection.tsx`
  - Props: `{ id: string, url: string, isPrimary: boolean }`
  - 功能：单实例 WS 生命周期（复用现有 `useDaemonWS` 逻辑，扩展支持 `instanceId`）
  - 在组件内调用 `store.updateInstance(id, ...)` 而非全局 `handleWsBatch`

- `frontend/src/components/TraversalOverlay.tsx`
  - Props: `{ nodePositions: Map<string, {x,y}>, transform: d3.ZoomTransform }`
  - 功能：读取 `store.instances`，查找坐标，渲染同心圆和颜色编码光晕
  - 必须设置 `pointer-events: none`

### 修改文件

- `frontend/src/hooks/useStore.ts`
  - 新增 `instances: Record<string, InstanceState>` 替代单一 `currentPositionLabel`
  - 新增 `updateInstance(id, state)` action
  - 保留 `currentPositionLabel` 兼容（主实例的位置）

- `frontend/src/tokens.ts`（或新增 `instanceColors.ts`）
  - 添加 `INSTANCE_COLORS = ["#4a9eff", "#ff8c42", "#66ff66", "#ff66cc", "#ffff44"]`

- `frontend/src/App.tsx`
  - 解析 `INSTANCE_LIST` env 变量
  - 渲染多个 `<DaemonConnection />` 不可见组件
  - 将 `nodePositionsRef` 和 `zoomTransform` 传给 `TraversalOverlay`

- `frontend/src/components/TopologyView.tsx`
  - 导出 `nodePositionsRef`（当前已在内部维护，需要暴露给 Overlay）
  - 导出 `transformRef`（当前已在内部维护 zoom transform，需要暴露）

---

## 陷阱列表

1. **React Hook 规则**：不能在循环中调用 hook。
   - 缓解：`<DaemonConnection />` 不可见组件模式，每个实例一个组件实例。

2. **d3 zoom 坐标同步**：`TraversalOverlay` 必须实时获取 d3 zoom transform，否则光晕位置漂移。
   - 缓解：TopologyView 需将 `transformRef` 通过 `forwardRef` 或 callback 传出。

3. **同心圆遮挡**：5 个实例同占一节点时，最外层光晕（radius ≈ 30+）遮挡相邻节点。
   - 缓解：Overlay 全局设 `pointer-events: none`。

4. **实例颜色冲突**：颜色需与现有 `T.traversalPulse`（当前单实例颜色）区分。
   - 缓解：单实例模式下沿用现有颜色，多实例模式下切换为实例颜色数组第一个。

5. **VITE 环境变量**：`INSTANCE_LIST` 需要前缀 `VITE_`（Vite 规范），即 `VITE_INSTANCE_LIST`。

---

## 决策范围

此设计文档覆盖 multi-instance-view 工位的前端实现范围：
- 不修改后端 `daemon_api.py` 的 WS 消息格式
- 不引入额外后端服务
- 实例数量预期 2-5 个

边界外（本次不实现）：
- 动态实例发现机制
- 跨实例穿越策略协调
- 运行时添加/移除实例的 UI
