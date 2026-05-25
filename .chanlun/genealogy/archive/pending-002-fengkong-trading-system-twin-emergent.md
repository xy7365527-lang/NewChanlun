---
id: pending-002-fengkong-trading-system-twin-emergent
timestamp: 2026-04-27
status: 已结算
settlement: 吸收
settled_by: '220'
settled_date: 2026-05-23
settlement_scope: 风控独立性轴（回测引擎轴属 pending-004，不在本结算范围）
type: domain
negation_source: homogeneous
negation_form: separation
topo_effect: "absorb:fengkong→chanlun-trading-system:by-220"
---

# fengkong 与 chanlun-trading-system 双生成态——14 份定义中的概念分离候选

## 矛盾

W-2 自标：14 份缠论定义中独此 2 份处于 status: 生成态，且功能高度重叠（成本归零三阶段）。
- `chanlun-trading-system.md`：第182行肯定回测引擎（与蓝图 v3 第一原则"不回测策略"冲突——见 pending-004）
- `fengkong.md` v0.1：§"未结算问题3"自承"两文件高度重叠 + 待架构决策"

## 否定了什么

否定的是"风控是独立模块"的假设。220号已结晶："风控不是独立模块，是买卖点系统的内在属性"。但代码与定义层仍同时维护两份生成态文件——如果风控真的是 maimai 的内在属性，fengkong 应该被合并/吸收，而不是作为独立定义。

同时也否定了"通过持续生成态拖延 = 中性选择"——这是务实思维（161号否定）的表现：把矛盾留到后面。

## 推导链

- 220号：风控 = 买卖点系统内在属性（缠师原文五概念溯源）
- 267/338号：成本归零三阶段已在 cost_reduction_fsm.py 工程化（349号确认一致）
- 099号已识别的 Trend(Move)/Consolidation(Move) 子类型缺口同样跨这两份定义
- W-2 + W-7 双链交叉指认（W-7 详查 backtest 子包合法性，W-2 详查定义层重叠）
- 161号：务实 = 把矛盾留到后面 = 不允许

## 谱系链接

- 220号、267号、338号、349号（操作方法论形式化主链）
- 099号（Trend/Consolidation 子类型缺口）
- 161号（务实禁止）
- 005a/005b（概念分离前置语法）

## 影响声明

- 影响：`.chanlun/definitions/fengkong.md`、`.chanlun/definitions/chanlun-trading-system.md`、`src/newchan/backtest/`、`src/newchan/trading/cost_reduction_fsm.py`
- 改动方向（待编排者裁决）：
  - 路径 A：合并为单一 `chanlun-trading-system.md`（fengkong 降为参考）
  - 路径 B：概念分离仪式——fengkong = 风险拓扑层，chanlun-trading-system = 操作方法论层（明确边界）
  - 路径 C：保持双生成态（不允许——161号否定务实）
- 路径 A/B 都需走 `/escalate` 提请编排者价值判断

## 异质审计降级

本 session 全程 gemini-challenger 不可用。下游 Lead 可决定 spawn gemini-challenger 工位异质质询。

## 结算（2026-05-23）：吸收 — 由 220号 解决

### 四分法分类：吸收（absorb）

本矛盾的核心问题——"风控是否独立模块？fengkong 是否应作为独立定义？"——
已被**已结算的 220号**回答，不是需要价值判断的自由选择。

### 推导链（定理，非选择）

1. **220号已结算**（status: 已结算，settled/220-chanlun-trading-system.md）：
   - §5："风控不是独立模块，是买卖点系统的内在属性"（缠师第35/10课溯源）
   - 下游推论#1："`src/newchan/risk/` 不是独立风控层，而是买卖点系统的扩展"
2. **路径 B 被 220号排除**：本文件原列路径 B = "概念分离仪式——fengkong = 风险拓扑层"
   = 把风控立为独立层。这与 220号"风控不是独立模块"**直接矛盾**——
   路径 B 会否定一条已结算原则，故不合法。
3. **路径 A 是唯一与已结算原则一致的方向**：fengkong 被 chanlun-trading-system
   吸收（降为参考）。当多路径中唯一一条与已结算原则一致时，无自由选择 = 定理（018号四分法）。
4. **不上浮的依据**（no-unnecessary-escalation）：可从 220号推导的不上浮。
   本文件原判"路径 A/B 都需 /escalate"的前提（A、B 均为合法待选）已被 220号否证——
   B 非法，故无需编排者在 A/B 间做价值裁断。

### 范围限定（严格）

本结算**仅覆盖风控独立性轴**。本文件提及的另一矛盾——
chanlun-trading-system.md 第182行肯定回测引擎 vs 蓝图 v3"不回测策略"——
是**独立矛盾 pending-004**，不在本结算范围，状态不变。

### 下游实现（吸收的执行，未在本次完成）

吸收的概念结论已确立。其文件层实现——合并 `fengkong.md` 入
`chanlun-trading-system.md`（fengkong 降为参考）、调整 `src/newchan/risk/`——
是**高影响面的破坏性操作，且与 pending-004（回测引擎）纠缠**。
按存在论位置：概念层吸收已结算；实现层合并需在 pending-004 一并裁定后
作为一个原子重构执行，不在此单独切割。标记为 220号下游推论#1 的待执行项。
