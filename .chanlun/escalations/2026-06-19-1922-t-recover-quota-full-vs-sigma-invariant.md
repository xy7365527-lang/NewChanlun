---
date: 2026-06-19-1922
status: 生成态
type: 概念层矛盾上浮（recover 配额语义裁决）
triggers: 543号开放轴#1（recover 配额语义待编排者裁决）
relates: ['542', '543', '527']
---

## 矛盾报告

### 矛盾描述

用户假说：「sink 拿父级 1/3 给子级，recover 应把子级**全部**归还父级（关掉整个短差）。若 recover 也只归还子级 1/3，就产生几何衰减——核心仓永远回不到 100%。1/3 就是 bug。」

**事实部分核实成立**：
- `sink`（t_engine.rs:240）：`m = mobile_quota(u_p) = u_p × 1/3`，父级真减仓 1/3 ✓
- `recover`（t_engine.rs:264）：`m = mobile_quota(u_s) = u_s × 1/3`，子级**只平 1/3 短差**升回 ✓
- 几何衰减真实：子级短差 = m₀·(2/3)ⁿ → 0，核心仓 = u₀ − m₀·(2/3)ⁿ → u₀，**渐近但有限次内永不回 100%**，次级别恒留残余短差 ✓

**但「1/3 是 bug」这一判断不成立**——这不是孤立实现错误，而是触及已结算定义边界的**概念层裁决**：

1. recover=1/3 与 `operate.rs`/`cycle.rs::recover_chunk` 完全一致（同用 `mobile_quota(u_sub)`），且后者带 `prove_sigma_quota(m, u_sub, …)` 守卫**强制** m = f×u_sub = 1/3。改全量 → operate.rs 路径直接 panic。
2. recover=1/3 是 543号谱系**显式标注的开放轴#1**：「本实装 recover 拉 f·u_{k−1}（1/3 of 次级别），是否是用户要的"回收 1/3"——**待编排者裁决**」。它是已知的待裁决选择，不是隐藏的 bug。
3. 注释把 recover 归为 `σ∘τ`（ε 对称 τ 转移），与 sink 的 `σ⁻¹∘τ` 群论对称。改全量 = 把 recover 从「τ 转移」重新归类为「了结/清空」（liquidate 范畴），破坏对称。

### 双方论证

**立场A（保 1/3 — σ-不变 τ 转移）：**
- 定义依据：542号 σ-不变配额——单次 τ 转移配额 `m = f×units，f = 1/λ 级别无关`，由 T48（units=σ-不变 Casimir）+ T59（自相似 σWσ⁻¹=W）+ T23（递归 step-replication）L0 强制。
- recover 是 σ∘τ（τ 转移族），配额对称延拓 542 ⟹ 必为 f·u_sub。
- 几何衰减不是缺陷而是 **543号「核心仓涌现」机制本身**：「每次 τ 只下沉 f=1/λ，顶层保留多数 ⟹ σ-塔自然分布 f∝λ⁻ᴷ」。残余短差 = 几何塔的低级别小仓，是 feature。

**立场B（改全量 — 次级别走势了结）：**
- 定义依据：缠论次级别走势完整性——次级别买点（底背驰）= 次级别走势**完成**，短差应**完整了结**，资金全部回归核心。「完成 1/3」无缠论依据；走势完成是 0/1 事件，非配额事件。
- 类比：recover 本体论应同 `liquidate`（边界算子，全量 units，不受 mobile_quota 约束），而非 τ 转移。
- 经验动机（记忆 project_t_cross_level_coupling_falsified）：BTC−89.7 / BRN−178 真穿仓，根因「中间 L2/L3 灾难失血 + 每微小次级别卖点都开短差过度做空」。短差清不掉 → 累积穿仓。全量 recover 可及时清空过度做空残余。

### 涉及的定义

- **542号**（settled）`σ-不变配额`：`.chanlun/genealogy/settled/542-spawn-allocation-sigma-invariant.md` —— 但 542 **只裁决了 spawn/下放方向**（释放给子 voice 的配额 f=1/λ），**未显式裁决 recover/升回方向**是否也必须 σ-不变。这是定义的**有效域边界缝隙**。
- **543号**（settled）`操作=word`：`.chanlun/genealogy/settled/543-operation-as-word-not-hardcoded-cycle.md` 开放轴#1 —— recover 配额语义**待编排者裁决**。
- 守卫：`fugue_v3/prove.rs:38 prove_sigma_quota`（强制 m=f×units，作用于 operate.rs，**不作用于 t_engine** —— t_engine 仅 `prove_nav_neutral`）。

### 谱系比对结果

- **542号先例**：spawn 配额从全局 θ 归一化 → σ-不变 1/λ。编排者读法A「势∝r 是径向坐标 r 的定义」⟹ 下放配额零自由度 = 1/λ。但该裁决语境是**下放（父→子）**。recover（子→父）方向的对称性**从未被显式裁决**——543 把它单列为开放轴正是承认这个缝隙。
- 本次与 542 的不同：542 解决「下放配额随级别变 vs σ-不变」（同属 τ 转移内部）；本次是「recover 属 τ 转移（σ-不变 1/3）vs 属了结（全量）」的**本体论范畴归属**问题——更上一层。

### Lead 的建议方案

**不直接改代码（no-workaround：定义冲突不写 workaround 跑通回测）。** 建议编排者按以下三选一裁决，并明确**范围**：

- **方案①（保 1/3，关闭假说）**：recover 维持 σ-不变 τ 转移。几何塔残余是 feature。穿仓根因另循「加成本门拒微小 sink + σ-ascend」（记忆已记的开放轴）解决，不动 recover。
- **方案②（改全量，仅 t_engine）**：t_engine.recover 改 `m = u_s`（全量了结）。**风险**：t_engine 与 operate.rs 对 recover 本体论分裂（一了结/一 σ-不变），违 no-patch-mentality 的「两引擎概念一致」。需同时声明 t_engine 的 recover 脱离 542 有效域。
- **方案③（改全量，统一含 operate.rs）**：recover 全体重归类为「了结」。**触及 542 已结算定义** —— 须放宽 `prove_sigma_quota` 对 recover 的适用范围（recover 不再受 σ-不变约束），= 修改 542 定义边界，需走定义层修订仪式。

Lead 倾向：若编排者目标是「验证全量 recover 能否救穿仓」，**方案②先做 L3 回测探针**（t_engine 隔离，不污染 operate.rs/542），用否定性结果（穿仓是否缓解）反哺裁决；确认有效再走方案③统一。但这是**选择**，需编排者拍板。

### 需要决断的问题（缠论语言）

**次级别走势完成时（次级别买点/底背驰 fire），父级核心仓应回收次级别短差的「1/3」还是「全部」？**

即：recover 是「σ-不变 τ 转移」（每次只回收一个 1/λ chunk，几何塔保留低级别残余短差）——还是「次级别走势了结」（次级别走势既已完成，整条短差一次性平清，资金全回核心）？

若裁「全部」，**裁决范围**：仅 t_engine 探针（②），还是统一改 operate.rs 并放宽 542号 σ-不变配额对 recover 方向的有效域（③）？
