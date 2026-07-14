# codex-f2 三 P0 实装设计审计裁定

> 工位 codex-challenger | goal g-nesting-fugue | 对象 = f1 报告
> `.chanlun/review-results/nesting-fugue-conformance-20260703.md` 提出的三个 P0 实装设计草案
> （下沉触发状态机 / 持仓节点=容器实例 / 多空对冲 overlay 诊断先行）。
> Codex 完整交互持久化：`.chanlun/review-results/codex-review-20260703-014607-1139.md`

## 0. 总裁定

**verdict: fail（三 P0 均需返工，且 P0-2 的问题前提本身有事实性错误，需重新定界）**

Codex 审计给出 6 条问题（编号沿用 Codex 原文），本工位对其中 3/4 号问题做了独立代码/谱系核实，
发现比 Codex 指出的更严重的事实——**f1 报告用来论证 P0-2 必要性的核心证据（"无 position_node
概念"+"B9 telemetry 矛盾待裁"）本身已经过时/不成立**。详见 §2。

## 1. Codex 逐条发现（review 模式，完整记录见持久化文件）

| # | 位置 | 严重性 | 问题 | 修复建议 |
|---|------|--------|------|----------|
| 1 | econ_positive.rs:573/856/1180 | 重要 | P0-1 单一 bool 门会误杀 Type2/3 小转大（Xzd）通道——现状已是 Nest/Xzd 二通道，"高级别背驰段前置门"外延过窄 | 改触发枚举（Type1TrendDivergence / Type23SublevelType1 / XiaoZhuanDa），Xzd 通道不受"无高级别背驰段"一票否决 |
| 2 | econ_positive.rs:327/843 | 重要 | 源码已明示 95.36% 信号 rungs 空——加触发门只会**减少候选数**，不会制造更深 rungs，不能作为修 depth=1 主导的机制 | P0-1 验收目标降级为"信号质量过滤"，不绑定 depth≥2 改善；depth 验收仍看 effective_nest_depth |
| 3 | coverage.rs:472/1662, interp.rs:562 | 重要 | **"无 position_node 概念"对 strategy/coverage.rs 不成立**——`attach_bsp_carrier_indexed` 已用 carrier_id 作 position instance | P0-2 应审计/升级既有 `CoverageElement.id→ActiveLeg.id→PersistentRegistry` 链路，不新起平行身份字段 |
| 4 | coverage.rs:481/1717 | 致命（若声称=完整§13）/重要（若仅作简化） | `posId^simp=carrier_id` 已知同 carrier 多买卖点共享 id，丢 entry_signal_id/side/generation——会误合并不同批次仓位 | 若保留简化必须显式声明"单 carrier 单 active instance"边界；若需多实例应直接上四元组 hash + release 级去重 |
| 5 | coverage.rs:1210/1260/1712 | 重要 | `‖ΔN‖_1` 可从净目标差算，但 `Π^overlay=H_tΔP−ΔC` 需要 ShortDiff/hedge 腿的独立 H_t 与成本，不能从最终净仓反推 | 诊断在 `strategy_target_legs` 后、`net_target_units` 前汇总，允许只读 accumulator；一旦触发真实对冲交易即须升级为账本 |
| 6 | 674号谱系 + NetValueImpossibility.lean:24 | 重要 | overlay 一旦执行即第三会计范畴，不能挂靠 R/TW 任一账本名下（674 已裁不同构） | 诊断阶段只读；执行阶段命名为独立 `OverlayState`，标注对 R/TW 的投影损失 |

**stance（Codex 原始声明）**：
```yaml
verdict: fail
stances:
  p0_1_divergence_bool_gate: reject
  p0_1_depth_distribution_claim: reject
  p0_2_carrier_id_simplification: needs_work
  p0_2_existing_identity_chain: accept
  p0_3_diagnostic_before_ledger: conditional
  implementation_order: needs_work
```

**实装顺序（Codex 建议）**：P0-2 先行（确认 carrier_id 简化 vs 升四元组）→ P0-3（依赖 P0-2 的
active leg/parent identity 归属 hedge 腿）→ P0-1 可并行设计，但落码需等触发枚举定案。

## 2. 本工位独立核实（超出 Codex 审计范围，代码+谱系交叉验证）

对 Codex 问题 3/4，本工位额外做了两处核实，发现**f1 报告的 P0-2 论证前提本身有两处事实性错误**：

### 2.1 `attach_bsp_carrier_indexed`（coverage.rs:472-497）诚实标注已明确承认这是 §14 简化版

```rust
/// ★工位 H（级别容器.pdf §13/§14）：hostOf(g) = **carrier 容器**本身（产出 g 的走势元素）
/// ★简化标注（PDF §14）：本实装用 carrier id 作位置节点身份（= PDF §14 简化版 `host.active=true`），
/// **同一 carrier 同 bar 多买卖点会共享 id**（损失 entry-level 区分）。PDF 更严格版 = `posId =
/// hash(carrier_id, entry_signal, side, generation)` position instance——见 §H ceiling。
```

这段注释逐字对应 f1 报告 §3 P0-2 设计草案提出的方案（"先做 §14 简化 position identity
(posId^simp=c)"）——**这不是待建的新设计，是已经写在代码里、已经标注了同一限制的既有实现**。
f2 若把 P0-2 当作"从零新建 position_node"来规划，会重复造轮子。

### 2.2 B9 深度祖先闭合矛盾（f1 报告列为"待 codex 裁 telemetry 口径"）已在 642 号谱系结算，非开放项

`.chanlun/genealogy/settled/642-acceptance2-h2-bootstrap-gap-not-definition-conflict-...md`
（status: 已结算，2026-07-02）机器坐实：

- 真因 = `coverage.rs:1253` host 注入用错 key（注入候选**同级** host `(c.level, c.source_index)`，
  而 AncOK 需候选**高一级** `parent_id` 容器在 raw）——这是可在 coverage.rs 内修的**工程 bug**
  （定理类），**不是**定义冲突，也不是 f1 报告猜测的"AncOK 语义漂移"。
- 修复已落地：host 注入改为注入 parent_id 容器。L2 重测（commit `792305023b`）坐实**机制层**
  depth>0 腿真准入，`ΔN≠0`（裁定 b2→b1）——即 B9 描述的 `active_depth1=0∧active_depth2=0` 现象
  **已被修复且已 L2 验证**，不再是当前代码的真实状态。
- 限定语（642 谱系强制随行）：机制修复 ≠ 盈利——NAV 层 `ΔSharpe=0.000`（8/8 照实），零贡献结论
  收窄至 NAV 层，净头寸层已证伪。

### 2.3 对 P0-2 判定的下游影响

f1 报告 §3 P0-2 的必要性论证链是：「B1（无 position_node）→ B6/B7/B9 一族缺口的根因 → depth1/2≈0
的剪枝证据」。这条链的两个证据锚点（"无 position_node"、"B9 矛盾待裁"）经核实**均不成立**：
`strategy/coverage.rs` 子系统已有 carrier_id 简化 position instance，且其 host 注入 bug 已修复并
L2 坐实机制层生效。

**但**——f1 报告扫描的是 `backtest/econ_positive.rs`（信号分解/可捕获价差诊断子系统），该子系统的
`RawSignal`/`SignalDecomp` 确实**不消费** `strategy/coverage.rs` 的 position_node 机制（两个子系统
相互独立：coverage.rs 是实盘/多重赋格解释器层，econ_positive.rs 是离线信号分解诊断层）。所以
"econ_positive.rs 无 position_node" 这句话本身没错——错的是把它当作**整个系统**缺 position_node
的证据，从而设计一个"从零新建"的 P0-2。

## 3. 重新定界后的 P0-2

真实缺口不是"建 position_node"，而是**两个子系统间的架构分歧未被识别**：

1. `strategy/coverage.rs`（实盘解释器层）：已有 carrier_id 简化 position instance
   （`attach_bsp_carrier_indexed`），host 注入 bug 已修（642号），机制层验证生效。
2. `backtest/econ_positive.rs`（信号分解诊断层）：无 position_node，用买卖点信号级
   `RawSignal` 直接配对——这是该模块的诚实设计（"经济正条件"诊断本就是逐信号做的，非逐持仓）。

P0-2 若要推进，正确问题是：**多重赋格（B1/B6/B7/B9 一族）的目标载体是 coverage.rs 的实盘解释器，
还是 econ_positive.rs 的离线诊断？** 如果目标是前者，P0-2 的工作是"评估是否需要把 §14 简化升级为
§13 完整四元组"（Codex 问题4 已指出的碰撞风险），而非新建；如果目标是后者，需要先论证
econ_positive.rs 是否真的需要 position 身份（该模块当前用途是价差捕获诊断，多头/空头信号已经
逐笔配对，引入 position_node 前需要说明诊断目的会如何变化）。

## 4. 边界条件（结论翻转条件）

- 若 Lead 确认 f2 的多重赋格目标载体明确是 `econ_positive.rs`（而非 coverage.rs），则 §3 的
  "重复造轮子"担忧不成立，Codex 问题 3 降级为不适用——但仍需回答"为什么诊断层需要持仓身份"。
- 若 642 号谱系记录的 L2 重测（commit 792305023b）之后 coverage.rs 又发生结构性改动使
  depth>0 腿重新归零，则 §2.2 的"已解决"判定需重新核实（本工位未重跑测试，仅读谱系记录 + 源码
  当前状态，源码读取确认 host 注入确已改为 parent_id 容器）。
- Codex 问题 1/2/5/6 的判定不依赖本工位的 §2 补充核实，独立成立。

## 5. 下游推论

- P0-1：不能声称"解决 depth=1 主导"，只能声称"信号质量过滤"；落码前须先定案触发枚举
  （非单 bool），且不能让 Xzd 通道被背驰门一票否决。
- P0-2：**不建议按 f1 报告"从零新建"的方案推进**；应先向 Lead/编排者确认多重赋格目标载体，
  再决定是升级 coverage.rs 既有机制还是评估 econ_positive.rs 是否真需要持仓身份。
- P0-3：可保留"诊断先行"方向，但 `Π^overlay` 的计算需要 hedge 腿独立记账（不能纯反推），且
  一旦诊断结果触发实际交易即升级为独立第三账本（不得挂靠 R/TW）。
- 三者实装顺序：不建议"P0-2 先行"（因为 P0-2 本身需要重新定界，先行会锁死错误前提）；
  建议顺序改为——先由 Lead/编排者裁定多重赋格目标载体（定界问题，逻辑上先于三个 P0 的任何代码）
  →再按 Codex 建议顺序（P0-2→P0-3→P0-1）推进。

## 6. 谱系引用

- 674号（settled）：R/TW 账本不同构，双层并置，P0-3 相容性依据。
- 642号（settled）：coverage.rs host 注入 bug 定理类降级 + L2 坐实，P0-2 事实前提修正依据。
- 638/639/640/641号：642 的同轮关联谱系（正交机制分离、单值性分层、性能声明膨胀），供背景参照。
- f1 报告（`.chanlun/review-results/nesting-fugue-conformance-20260703.md`）：被审计对象，B1/B9
  两处证据锚点经本工位核实需修正。

## 7. 影响声明

本工位纯只读审计（Codex CLI 调用 + 源码/谱系交叉核实），未修改任何代码/定义。产出 = 三 P0
设计的裁定 + P0-2 论证前提的事实性修正意见。建议 Lead 在推进 P0-2 前先向编排者/自行确认多重
赋格的目标载体（coverage.rs vs econ_positive.rs），避免在错误前提上锁定实装方向。
