# P0-1 下沉触发枚举 NestTrigger + 每桶 μ̂ 质量对照（econ 域）

**工位**：swarm/ws-bottomup（task #108） · **日期**：2026-07-03 · **规格**：codex-f2 修正案 #1/#2（`codex-f2-design-ruling-20260703.md`）+ 交接 `f2-impl-20260703.md §3`

---

## 0. TL;DR

把 econ_positive.rs **已存在**的三路准入 dispatch 显式命名为 `enum NestTrigger{Type1TrendDivergence, Type23SublevelType1, XiaoZhuanDa}`，作为 signal-provenance 穿 RawSignal→SignalDecomp。BTC 300K 实测每桶候选数+μ̂：**Type1TrendDivergence=0 / Type23SublevelType1=728 / XiaoZhuanDa=28**。零 Type1——若把 f1 提议的「须有高级别背驰段」单一 bool 门套到全通道，会一票否决 **100%**（全部 756 条）信号。这是 codex #1「Xzd 不受一票否决」裁决的最强 L2 验证。零生产行为改动（trigger 是纯派生字段）。

## 1. 结论

**codex #1 批评的「单一 bool 背驰前置门」在生产里从不存在**——`build_gate_certificate` 现状已是三路 dispatch：`Nest(cert)`（Type1 走 div_cand / Type2/3 走 descend anchor）+ `Xzd(evidence)`（小转大）。P0-1 = 把这个隐式三路显式化为枚举 + L2 每桶质量测量，**不新增任何门**。

`NestTrigger` 三变体 = 现有 dispatch 的命名：
- `Type1TrendDivergence`：Type1 → `Nest`，div_cand 准入（本级趋势背驰段，第29课 A3）。
- `Type23SublevelType1`：Type2/3 → `Nest`，`descend_type1_anchor_depth` base gate 准入。
- `XiaoZhuanDa`：Type2/3 → `Xzd`，C2/C3 小转大判据准入——**独立通道，不受背驰否决**。

分类器 `nest_trigger(&GateCertificate, BspCandType)` 纯派生（零行为改动）。

## 2. L2 每桶 μ̂ 质量对照（BTC 300K，2025-11-04→2026-05-31，756 配对信号）

| trigger | n | Σactual_pnl | μ̂ |
|---|---|---|---|
| Type1TrendDivergence | 0 | 0 | 0 |
| Type23SublevelType1 | 728 | −5.65e4 | −77.6 |
| XiaoZhuanDa | 28 | −5.30e3 | −189.2 |

**交叉校验**（防标注 bug）：直接数 `decomp.bsp_class` 含一类位(buy1/sell1)的条数 = **0**，与 Type1 桶 n=0 吻合 ⟹ 分类无洞（自检 `t1_n+t23_n+xzd_n==total` 通过）。

**过滤前后**（codex #1 框架）：若单一 bool「须有高级别背驰段」门套全通道，仅 Type1 存活 = **0 条**；被一票否决 = **756 条（100%）**，μ̂=−81.7。结论：该门外延不是「过窄」而是「全杀」——codex「Xzd 不受一票否决」裁决被最强验证。

**关键否定性发现**：BTC 该窗准入+配对信号集**零 Type1**，全为 Type2/3（Type23 96.3% + Xzd 3.7%）。与架构（mod.rs:247 上级层只产第二类，第一类仅 L0）+ [[project_oddeven_mu_identity]] 全战役无 confirmed 正结果一致。μ̂ 三桶全负（全窗含选择偏差，非 OOS alpha；负 μ̂ 不改 P0-1 架构结论——trigger 通道正确分离是 P0-1 的对象，盈利性是独立且已否证的问题）。

## 3. 边界条件（结论翻转条件）

1. Type1=0 是本 BTC 窗事实，非恒等式——其他品种/窗若第一类信号能通过 gate+pairing，Type1TrendDivergence 桶非空，此时「只作用于 Type1」的门才有非平凡过滤对象。有效域 = BTC 单标的 L2。
2. 若未来把「高级别背驰段」门真加到 Type1 通道（现无此门），须只门 `Type1TrendDivergence` 分支，`Type23SublevelType1`/`XiaoZhuanDa` 免门——否则重蹈 codex #1 全杀。
3. 若 `bsp_cand_type` 优先级改动（现 StructBreak>Type1>Type2>Type3），trigger 分类随之变——分类器单一来源，改一处即改全链。

## 4. 下游推论

- **不绑 depth≥2**（codex #2）：trigger 是准入通道归因，与 `effective_nest_depth`（区间套跨级层数）正交；加 trigger 不改任何门决策、不改深度分布（见 #101 bottomup-nest 深度分布不变）。
- trigger 是 SignalDecomp 的新归因维（同 bsp_class/z/sigma_higher），可供未来 per-trigger OOS μ̂ 分层（现只读全窗质量对照）。
- 「高级别无 alpha」图景不被本工位改变（零行为改动）。

## 5. 谱系引用

- codex-f2 #1/#2（`codex-f2-design-ruling-20260703.md`）：单一 bool 门 reject / depth 声明 reject → 验收降级为信号质量过滤。
- [[project_oddeven_mu_identity]] / fullz-policy：全战役无 confirmed 正 alpha——本工位三桶 μ̂ 全负与之一致。
- `no-patch-mentality`：未加 f1 提议的坏门（会全杀），未造死变体——把已存在的正确 dispatch 显式化，不是补丁。
- `ceremony-scan-completeness`：trigger 有真消费者（本探针 + SignalDecomp 归因维），非无消费者死字段。

## 6. 影响声明

**改动文件**：`rust/src/theta_v0/backtest/econ_positive.rs`（唯一）。

- 新增 `pub(super) enum NestTrigger`（3 变体）+ `pub(super) fn nest_trigger`（纯派生分类器）；`BspCandType` 提升 `pub(super)`（分类器签名可见性一致）。
- `RawSignal`/`SignalDecomp` 各加 `trigger: NestTrigger` 字段（signal-provenance），穿 `collect_signals`→`pair_signals`→`SignalDecomp` + dx harness 对拍路径（signals_dx 与生产同源分类，bit-exact 对拍不破）。
- **零生产行为改动**：trigger 从已决 `GateCertificate`+`bsp_cand_type` 派生，不参与任何门决策；门集/信号集/depth 分布全不变。
- 新增 `#[ignore]` 探针 `acc_nest_trigger_quality_probe`（L2 每桶 μ̂ + 交叉校验 + partition 自检）。
- 护航：`cargo test --release --lib` = 1407 passed / 0 failed / 106 ignored，无退化。

**复算**：`ECON_L2_MAX_BARS=300000 cargo test --release --lib acc_nest_trigger_quality_probe -- --ignored --nocapture`

## 认识论等级

**L2**（真实 BTC 单标的全历史窗，可产否定性结果——Type1=0 全杀是否定性发现）。per-trigger μ̂ 全窗含选择偏差，非 OOS alpha，不作功效声明。
