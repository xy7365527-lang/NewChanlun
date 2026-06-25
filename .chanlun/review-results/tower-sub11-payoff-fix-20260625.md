# 塔-子11 payoff 纤维修复判决：codex #95 四 FAIL 修复 — payoff=Σ' 上 L3 纤维丛，失血⊋孤儿，c≠f(L_confirm)，分类层只给 L0（task#15）

**工位**：tower-sub11-payoff（topo_address: swarm/tower-formalize/sub11-payoff | parent_callback: tower-formalize）
**日期**：2026-06-25 ｜ 分支：orbit9-B-dispatch-20260624（B worktree）
**完整构造**：`tmp/tower_payoff_fiber_fixed.md`（本文件是落盘判决摘要，构造细节见 tmp 主文件）
**认识论等级**：payoff 纤维**结构层 L0**（结构孤儿/可操作二分，从 H¹ 闭合循环推导）+ **payoff 符号 L3 未决**（regime 函数 OKLO 正/GC 负）
**约束遵守**：no-patch-mentality（删 c=f(L_confirm) 单变量声明膨胀，据实重写）+ formalization-validity-domain（L0/L3 严格标注）+ result-package 六要素（tmp 主文件 §五）+ 生成性完备（G_payoff 吐两边界 case，tmp 主文件 §四）

> **【审计恢复注记 2026-06-25 codex-line-audit】** 本文件经 codex-line provenance 审计判 **MIXED**：派发任务 #97（transcript L8513）明令 `c=f(L_confirm)`（codex 第二选项,L_confirm 入轴后 c 被吸收）,但本文件 §二/§三**反向论证 c≠f(L_confirm)** 并把 payoff 符号还给 L3 regime 纤维。**codex 两轮异质审计（codex-verify-provenance + codex-verify-bc-concepts）独立确认 c≠f(L_confirm) 在概念上更严格、方向正确**（三重代码级否定：P&L 代数 `pnl=u*(c-basis)` L_confirm 不决定符号 / codex#95 §八#2 FAIL 蕴含 / ConfDepth 不含 d_k·ρ_k·regime 完整自变量集；反例 GC seg-short −130k δ充分仍失血）。**本文件构成对派发指令 task#97 的 c=f 单轴断言的形式化推翻**（基于反例 + 框架,非绕开未完成任务）。头部 task#15 编号系原工位笔误,实际承接 task#97。provenance: pre-interrupt 发现 #95 4FAIL(L8476),修复执行于中断后(recovery),据实重写非声明膨胀。

---

## 〇、修复判决（一句话）

**三 FAIL 共同根 = 子8 把 L3 的 regime payoff（纤维值）降级为 L0 的结构判据。** 修复 = 把 payoff 重构为建在形态层 case 空间 Σ' 之上的**纤维丛**（底空间=形态 case 向量，纤维=payoff 标量×regime），分类层只声明 L0 底空间结构（结构孤儿/可操作），payoff 符号还给 L3 纤维截面（regime 函数，不降级声明）。

---

## 一、FAIL① 修复：失血 ⟸ 孤儿是单向蕴含（分离两个失血概念）

**严格分离（构造，非补丁，tmp 主文件 §二）**：

| 失血概念 | 等级 | 判据 | 闭合性 |
|---------|------|------|-------|
| **结构孤儿失血** | **L0** | L_confirm ≻ L_move ∨ d_k=Up（永不产平腿买卖点） | 孤儿 ⟹ 必失血（H¹ 腿不闭合，∮ 浮亏到 eod，L0 充分） |
| **regime payoff 失血** | **L3** | 可操作（L_confirm⪯L_move ∧ d_k=Down）但 net<0（无足够下跌段 regime） | 可操作（非孤儿）但 net<0 ⟹ 失血 ∖ 孤儿 ≠ ∅ |

**失血 ⊋ 孤儿（FAIL① 核心，反向不成立）**：
- 孤儿 ⊊ 失血：孤儿是失血真子集（结构性必失血，L0）。
- **反例（失血 ∧ ¬孤儿）**：可操作机动腿（L_confirm⪯L_move ∧ d_k=Down，非孤儿）在强牛 regime 下捕获跌幅 0.68% < 574 滞后 2.3%（project_selloff_root_cause_granularity）⟹ 腿闭合但 net<0 ⟹ 失血 ∧ ¬孤儿。
- ∴ 失血 ⇍ 孤儿。子8 等价声明（孤儿⟺失血）= 把 L3 regime payoff 失血降级为 L0 结构孤儿（声称失血都可由 L_confirm≻L_move 刻画），修复 = 等价 ⟺ 改单向 ⟸ + 显式声明失血∖孤儿=regime payoff 失血（L3）。

---

## 二、FAIL② 修复：删 c=f(L_confirm) 单变量声明膨胀，c 自变量=整个 case 向量+regime

**c 的真实自变量集逐一核验（no-patch §禁止模式5，tmp 主文件 §1.2）**：

```
c(case) = c( ⟨E_k, F_k, ρ_k, d_k, δ_k(=L_confirm), completion_k, α*_k⟩_{k=a0..r*},  regime )
        ≠ c(L_confirm) 单变量
```

- L_confirm（δ）：✅ 是自变量之一——决定**腿是否闭合**（循环内 vs 落循环外），但**不决定闭合后 net 符号**。
- d_k：✅——d_k=Up 开空 ⟹ L_confirm 发散 ⟹ 孤儿 net 浮亏（payoff-regime §3.2 因子①）。
- ρ_k：✅——核心 H⁰(long) 8/8 正 vs 机动 H¹(short) 高级别逆势失血，net 符号随角色翻转。
- L_pullback：✅——决定捕获跌幅大小（α*→0 vs >0），影响 net 量级。
- completion_source（type1/type3，#96）：✅——平腿时机不同 ⟹ 捕获跌幅不同。
- regime：✅（**L3 维度**）——同形态 case OKLO 正/GC 负，net 符号决定性自变量，外生于形态层。

**为何 c=f(L_confirm) 是膨胀**：单变量声明推出"调 L_confirm 就能改 payoff 符号"，但 payoff-regime §2.2 实测高级别逆势空腿即使 δ 充分（L_confirm 可达）在强牛 regime 仍失血（GC seg-short −130k，因 d_k=Up + 无下跌段）。**L_confirm 充分 ≠ payoff>0** ⟹ c≠f(L_confirm)。删单变量声明，据实重写为整个 case 向量 + regime 的函数（删除，非加分支）。

---

## 三、FAIL③ 修复：分类层只给 L0 结构判据，payoff 符号严守 L3 未决

**分类层有效域边界（formalization-validity-domain，tmp 主文件 §三）**：

```
分类层 L0 可声明：
  (a) 哪些 case 结构孤儿（L_confirm≻L_move ∨ d_k=Up）⟹ 必失血（L0 充分）
  (b) 哪些 case 结构可操作（L_confirm⪯L_move ∧ d_k=Down）⟹ 腿可闭合（L0）
  (c) 可操作是 α*_k>0 布尔前驱（子9 分离定理）

分类层 L0 不可声明（声明=降级膨胀）：
  (d) ✗ 可操作 ⟹ payoff>0      （L3 regime 函数）
  (e) ✗ 可操作 ⟹ 超 BH         （L3，自适应后超 BH 未决）
  (f) ✗ 消除孤儿 ⟹ 0/8 转正    （消结构孤儿≠消 regime payoff 失血）
```

**payoff 符号是 L3 regime 函数（不降级）**：sign(net_c) = sign(捕获跌幅(c,regime) − 摩擦 − 滞后) = regime 函数（OKLO 有下跌段 net>0 / GC 强牛 net<0，payoff-regime §2.2）。同形态 case 符号随 regime 翻转 ⟹ payoff 符号不可从形态层 L0 推出（L3 未决）。与 #94 §六 δ 连续轴「分类层 L0 ≠ payoff 层 L3」分层一致——本任务是该分层的 payoff 侧严格化。

---

## 四、生成性完备：G_payoff 吐两个边界 case（不断言完备+补丁）

G_payoff: case c ∈ Σ' ↦ (L0 结构类: 结构孤儿/可操作) × (L3 纤维: payoff×regime)。机械吐出两个未被指出的边界 case（生成证据，feedback_generative_completeness_not_asserted）：

| 边界 case | 结构类（L0） | payoff 符号（L3） | 意义 |
|----------|------------|------------------|------|
| **① 可操作但 regime 失血** | 结构可操作（⪯ ∧ d_k=Down） | net<0（强牛无下跌段 GC） | **证失血 ⊋ 孤儿**（可操作∧失血，FAIL① 反例的生成式来源） |
| **② 对角线 L_confirm=L_move** | 结构可操作（勉强闭合） | net→0（捕获∮→0） | no-op（开≈不开），承接子9 §4.3 对角线 |

**可操作 ⇏ payoff>0 由生成函数三符号吐出**（net>0 regime 有利 / net<0 边界① / net→0 边界②），非断言完备。底空间 = 结构孤儿 ⊔ 结构可操作（L0 二分穷尽），可操作上 payoff 纤维三符号（L3），无遗漏（tmp 主文件 §4.4）。

---

## 五、result-package 六要素（摘要，完整见 tmp 主文件 §五）

1. **结论**：payoff = Σ' 上纤维丛（底=形态 case 向量，纤维=payoff×regime）；失血⟸孤儿单向（失血⊋孤儿，失血∖孤儿=regime payoff 失血 L3）；c 自变量=整个 case 向量+regime（删 c=f(L_confirm)）；分类层只给 L0 结构判据（不声明 payoff 符号 L3）。
2. **定义依据**：[[project_trend_dev_two_source_classification]]（纤维拍扁）+ 子9 #93（可操作⟺L_confirm⪯L_move）+ payoff-regime #75 §2.2（OKLO 正/GC 负 regime 函数）+ #91/#96（L_confirm/completion 独立轴）+ project_isolated_fugue_forest_verdict（孤儿不可能）。
3. **边界条件**：若 payoff 符号能从形态层 L0 唯一推出 ⟹ payoff 退化为维度（payoff-regime §2.2 符号翻转使不成立）；若失血都是结构孤儿 ⟹ 等价恢复（边界①使不成立）；若 c 真只依赖 L_confirm ⟹ 单变量成立（三因子合取使不成立）；若区间套链 a0=segment 坍缩 ⟹ 结构判据该尺度断裂（554号继承）。
4. **下游推论**：子4 bleed_k 只诊断结构孤儿（L0），regime payoff 失血不进 bleed_k；子5 实装只过结构孤儿门，不加 payoff 符号门（用 L3 做 L0 门=未来函数/降级）；#16 核验无降级声明；#20 codex 重测质询点见 tmp 主文件 §七。
5. **谱系引用**：574（L_confirm 是 c 自变量之一非唯一）/539（d_k 失配=孤儿根因）/payoff-regime #75（regime payoff 失血 L3 根）/561 b2（不擅自结算）/子9/子10。**不新增谱系节点**；潜在新分离「结构孤儿失血 L0 ⊊ 失血（含 regime payoff L3）」+「payoff 纤维丛底=形态 case 空间纤维=regime」待 #20 异质重测确认后由结晶节点评估。
6. **影响声明**：不改代码/定义；新增 tmp 主文件 + 本落盘文件；修复三 FAIL（失血单向/c 自变量据实/分类层守 L0）；守 561 b2 不结算 + payoff 符号 L3 未决；不影响底空间 Σ'（#90/#94/#96 生成）/O6/引擎。

---

## 六、认识论等级（formalization-validity-domain，摘要见 tmp 主文件 §六）

- payoff=纤维丛 / 结构孤儿⟹必失血 / 失血⊋孤儿 / c 自变量集 / 分类层只给 L0 / G_payoff 生成完备 = **L0**（结构层 + 据实，部分由 L3 反例支撑）。
- payoff 符号 sign(net_c) = regime 函数 = **L3 未决**（OKLO 正/GC 负，本任务不声明符号）。
- 结构判据在 a0=segment 尺度坍缩 = **L3 未决**（554号继承 #94）。

**核心诚实分层**：分类层 L0 完备（结构孤儿/可操作二分穷尽）≠ payoff 层有效域（哪些 case 净赚 L3）。三 FAIL 修复的本质 = 把被降级到 L0 的 regime payoff 纤维值还给 L3，分类层只留 L0 底空间结构。本任务**不**声明：哪个 case 实际 payoff 符号（L3 payoff-regime #75）、payoff 实装代码（子5 #84）、自适应后超 BH（L3 未决）。

---

## 七、异质重测衔接（#20 codex，约束4）

本判决核心（payoff 纤维丛 L0/L3 分层 + 失血⊋孤儿 + c 自变量集）= L0 构造 + L3 锚定（payoff-regime fugue_v3 per-leg dump 可复跑）。异质重测 #20 质询点（tmp 主文件 §七）：①纤维丛 vs 维度是否换名（regime 是否该进 Σ' 作维度）②对角线 net→0 算不算失血（失血边界清晰性）③c 含 regime 但分类层不读 regime 是否自相矛盾（c 值 vs case 归属两件事）④结构孤儿⟹必失血是否 a0=segment 坍缩 ⑤子8 丢失下修复是否修不存在的问题（no-patch 角度：L0/L3 分层构造本身有效不依赖子8 是否犯错）。#20 blocked by #16 审查。
