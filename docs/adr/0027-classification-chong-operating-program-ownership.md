# ADR 0027：Classification 与重操作程序分属两个互斥的唯一 owner

**日期**：2026-09-04
**裁定人**：编排者，走 [[grilling] 完整生产程序声明边界 #1321](https://github.com/xy7365527-lang/NewChanlun/issues/1321) 选择 B
**状态**：已采纳。本 ADR 只定 owner、依赖方向与运行边界，不改生产代码。
**后续**：[#1322](https://github.com/xy7365527-lang/NewChanlun/issues/1322) 先扫描真实调用边；[#1323](https://github.com/xy7365527-lang/NewChanlun/issues/1323) 再用 `/improve-codebase-architecture` 与 `/codebase-design` 裁接口和迁移，并产后续 SPEC。

## 背景

现有代码分出三条执行线：π 批量生产回测、organic/赋格交易线，以及 paper 的行情、订单与对账线。它们尚未形成一套有唯一 owner 的完整生产程序。若直接把任一现有入口扶正，结构事实、持久操作状态和交易决策仍可能各有第二套生产实现。

本裁定继承四条既有不变量：

- [ADR 0010](0010-chong-persistent-operating-unit.md)：重是 `⟨标的，操作级别，专属筹码⟩` 的持久操作单位。
- [ADR 0011](0011-operation-decomposition-layer.md)：一条纵向主干塔挂 N 个操作级别旁路；旁路与向下定位只消费主干，不回流主干。
- [ADR 0013](0013-partition-machine-bidirectional-form.md)：每重各有 TStage、专属筹码和毛账。
- [ADR 0014](0014-intra-chong-unidirectional-inter-chong-no-arbitration.md)：重内单向，重间不仲裁，物理订单只在最后按场所能力投影。

#1321 要裁的不是接口形状，而是谁逐 bar 对这些事实和状态负唯一责任。

## 裁定

### 一、两个域各有一个 owner，所有权互不重叠

| 领域 owner | 唯一拥有 | 不拥有 |
|---|---|---|
| **结构判定域** | 纵向主干塔与唯一 Classification | 各操作级别旁路、同级别分解调用、向下定位、Chong、TStage、Voice、专属筹码、毛账、持仓、生产交易意图和执行状态 |
| **重操作程序** | 各操作级别旁路、同级别分解调用、向下定位、每重 TStage、0..N Voice、专属筹码、逐重毛账，以及唯一生产交易意图 | 第二份 Classification、结构主干修改权，以及独立重做结构判定的权力 |

「重操作程序」是本裁定的领域角色名。#1321 评论中的 `ChongOperatingProgram` 只是工作名，不是最终模块名。

两个 owner 组成一套生产程序，不是保留两套交易程序。任何结构事实或操作状态都只能有一个生产 owner。失败必须作为失败处理，不能切到另一套判据、旧入口或宽松路径重新判定。

### 二、Classification seam 只向下游开放

结构判定域以只读、版本化的 Classification 契约向重操作程序交付结构事实。重操作程序只消费该事实，不复制结构判定，也不生成第二份 Classification。

资金、成交、风控、UI 状态及任何操作状态都不能回写结构主干。该边只有 `结构判定域 → 重操作程序` 一个方向。

「只读、版本化」在本 ADR 中是契约性质，不是接口设计。本 ADR 不预裁载荷字段、类型、序列化方式或版本机制。

### 三、历史、paper 与 live 共用两个核心 owner

历史回放、paper 与 live 必须共用同一个结构判定域实现和同一个重操作程序实现。不得为任一运行环境保留特有的结构判定、重状态推进或交易决策程序。

环境差异只允许落在后续裁定的行情、撮合和场所 adapter。adapter 的责任范围与具体接口不在本 ADR 内。

### 四、前端只消费 ReadModel

前端只消费 Classification 与重操作事件生成的只读 ReadModel。前端没有以下权力：

- 结构判定；
- 修改 TStage、Voice 或逐重账；
- 生成或改写交易决策。

控制动作必须经正式 command 与 RiskGate 边界进入生产程序。command、事件、RiskGate 和 ReadModel 的接口形状留给 #1322 扫描后的设计裁定。

### 五、旧入口只可收编或退役

现有 organic/赋格实现是可收编资产，不因本裁定自动成为正本。正确部分进入唯一重操作程序后，旧独立入口必须失去生产决策权或退役。

π 内重复拥有交易、成交或账本状态的入口同样必须迁移或退役。迁移期间不得用双 owner、回写结构主干或失败兜底维持兼容。

## 保留的严格多重赋格不变量

本裁定不改 [ADR 0010](0010-chong-persistent-operating-unit.md)、[ADR 0011](0011-operation-decomposition-layer.md)、[ADR 0013](0013-partition-machine-bidirectional-form.md) 和 [ADR 0014](0014-intra-chong-unidirectional-inter-chong-no-arbitration.md)。完整形态仍是：

1. 一条纵向主干塔产生一份 Classification；
2. N 个操作级别旁路各自做唯一同级别分解与向下定位；
3. 每重各自推进 TStage、Voice、专属筹码和毛账；
4. 旁路及下钻不回流主干，重内单向，重间不仲裁；
5. 物理订单最后才按场所能力投影。

变化只在所有权：第 1 项归结构判定域，第 2 至第 4 项及生产交易意图归唯一重操作程序。

## 明确延后

以下事项必须等 #1322 扫描真实调用边后再裁：

1. 结构判定域与重操作程序的最终模块名；
2. typed Classification 的载荷与版本形状；
3. command、event、RiskGate 和 ReadModel 接口；
4. 从 π、organic/赋格与 paper 现状迁入唯一 owner 的切片顺序；
5. 行情、撮合与场所 adapter 的具体边界；
6. 逐重毛账、场所持仓、成交 journal 与回放账之间的最终权威关系。

#1323 必须在 #1322 scan 之后使用 `/improve-codebase-architecture` 与 `/codebase-design` 完成这些设计，再由后续 SPEC 驱动实现。本 ADR 不发明类型、API、事件 schema、目录或迁移步骤。

## 受影响文件与后续票

**本票生产代码行为变更为零。** 本提交只改以下文档：

- `docs/adr/0027-classification-chong-operating-program-ownership.md`；
- `CONTEXT.md`；
- `docs/unified-architecture-canon.md`。

本票不猜测未来代码文件和行号。[#1322](https://github.com/xy7365527-lang/NewChanlun/issues/1322) 负责扫描真实全链、断边、旧入口和现行 owner，并给出精确受影响代码清单。[#1323](https://github.com/xy7365527-lang/NewChanlun/issues/1323) 的后续 SPEC 才能据该清单安排生产代码迁移、机械锁和验收。

## 取舍

选择 B 的理由是结构事实与跨交易持久状态的寿命不同。分开 owner 能保持 Classification 唯一，也能阻断资金、成交和 UI 状态反向污染结构。历史、paper 和 live 共用同一解释面，避免三套环境各自长出一套交易程序。

代价是现有 π 与 organic/赋格资产都不能按目录整体扶正。#1322 必须逐条辨认哪些逻辑收编、哪些入口退役；在该扫描完成前，本 ADR 不宣称接线已经实现。
