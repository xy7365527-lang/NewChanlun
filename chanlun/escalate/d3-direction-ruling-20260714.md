# D3 方向真值 结裁记录

- 日期：2026-07-14　裁决人：用户（口头裁定"需要方向"= 选项 A）　记录：Claude
- 上游材料：`chanlun/escalate/c2-pending-rulings-material-20260714.md` D3 栏
- 原文依据锚：`chan99/0011:5,9`（方向 = 走势类型级属性）、`chan99/0011:7,23`（盘整无方向 ⇒ 输出域须含 None）、`chan99/0010:17`（中枢中心定理二 GG/DD 判据）

## 结裁内容

1. **需要方向语义**（选项 A）。选项 0（方向语义豁免，系统收缩为无方向子集：中枢 + 三买卖 + 盘整背驰）经论证后被否——产品定位维持"走势类型分类"（C2 CompletedMove），盘整/趋势之分即方向之分，不可豁免。
2. **唯一方向定义 = `central-ggdd-v1`**：相邻两个同级别**已完成**中枢的外包络比较——
   - `next.dd > prev.gg` ⇒ 上（上涨延续）
   - `next.gg < prev.dd` ⇒ 下（下跌延续）
   - 其余（外缘重叠）⇒ 级别扩张，本级方向不存在
   - 单中枢（盘整）⇒ 无方向（None）
3. **实现定版 = 绑定主线既有实现，无新增代码**：
   - `rust/src/theta_v0/classifier/center.rs:207-215 classify_relation`——逐字实现上述判据，契约锚 `Origin.CenterStates.{IsUpTrend,IsDownTrend}`（Lean）。
   - `rust/src/theta_v0/classifier/decompose.rs MoveBlock.dir: Option<Direction>`——`Trend ⇒ Some(dir)`，`Consolidation ⇒ None`（第31课盘整无方向）。
4. **类型决议**：不给段 `Direction{Up,Down}` 加 None 变体（段方向天然二值），也不新增 `TrendDirection` 类型——主线既有 `Option<Direction>` 即三值语义。材料早先"加 None 变体"与"类型分裂"两案均作废。
5. **四个 seam provider 降级为工程回退**（`OwnershipFallbackV1`、`FirstLeafV1`、`FirstLastEnvelopeV1`、`SequenceEnvelopeV1`，存档 `chanlun/archive/c71-seam-20260714/level_view.rs:118-151`）：不得作语义真值。seam 复活（D1）时 `direction_provider_version` 必须指向 `central-ggdd-v1`。

## 勘误（对上游材料）

- 材料 D3 映射核对栏"无现成合规实现"的结论**范围仅限 seam 四 provider**；主线 `classifier/center + decompose` 当时未纳入对照。本记录修正：主线存在逐字合规实现，结裁对象从"新增实现"改为"定版绑定既有实现"。

## 下游解锁

| 项 | 状态 |
|---|---|
| D2 A/C 自动配对 | 解锁——基于 `MoveBlock.dir` 实现自动配对 provider（选项 A），列任务 |
| D1 投影 | 解锁——显式投影 + version tuple（含 `direction_provider_version=central-ggdd-v1`）同批实现（选项 A），列任务 |
| D4 PartitionPolicy | 确认型：维持 `GREEDY_V1`（#64 候选不达 P3 门，事实已判） |
| D5 事件持久化 | 工程排期，列任务 |
| D7 #56 例2 重开 | 先决三件套中 D3 已闭；余 D1 投影 + D2 hook，随任务完成后重开 |
