# ∼ₙ 组装零件交接（cc-spiral → t-decomp-uniq）

**背景**：cc-spiral（C14 螺旋覆盖）与 t-decomp-uniq 工位重叠，Lead 去重让位。
cc-spiral 在 `Spiral.lean` 临时做过的 ∼ₙ 组装已由 t-spiral 重写覆盖移除。
本文件交接 cc-spiral 验证过、而 `Strict/Decomp.lean` **尚未实现** 的零件 + 两轮
codex 异质审计的关键裁定，供 t-decomp-uniq 选择性吸收。**非强制采纳**。

认识论等级：全部 L0（Move/List/Quotient 结构推导，零数据依赖）。

---

## 一、Decomp.lean 已覆盖/超过 cc-spiral 的部分（无需吸收，仅备查）

cc-spiral 的方案 `∼ₙ := (canon = canon)`，`canon = (flattenBase=base取(lo,hi), gauge级别)`。
`Decomp.lean` 的方案 `∼ₙ := (I = I)`，`I = leaves`（递归拍平到 segment 叶序列）——
**更彻底且更强**：

| 点 | cc-spiral（已弃） | Decomp.lean（采纳） |
|----|------------------|---------------------|
| ∼ₙ 核 | `canon`=(base 区间序列, gauge级别) | `I = leaves`（底层 segment 序列）|
| `|𝒟ₙ(h)/∼ₙ|=1` | **有条件**（固定 base **且** gauge 级别 lvl） | **无条件**（仅固定 base，`decomp_quotient_card_one`）|
| 结合律 | `flattenBase_assoc` | `simN_connect_assoc` + `I_connect` |
| 非平凡 | `simN_separates` | `decomp_quotient_nontrivial` |
| Quot.lift | `IgaugeLevel`(gauge级别) | `liftedClass`(surfaceClass) |

**裁定**：`Decomp.lean` 的 `I=leaves` 无条件版本贴合 standard line 34 字面（"对每个 base
恰一等价类"无 gauge-floor 前提），优于 cc-spiral 的有条件 canon 版本。cc-spiral 的 canon
方案**不需要移植**。

---

## 二、cc-spiral 有、Decomp.lean 尚缺的唯一实质零件：gaugeFix→level 计算桥

`Decomp.lean` 注释（line 6/22/165/198/206）提到 `Spiral.gaugeFix` 但**未把它 Lean-焊接**
到 ∼ₙ——它走 `I=leaves` 路线，把 T₅₂ gauge 规范固定留作概念描述。

若 t-decomp-uniq 想补足"T₅₂ gaugeFix 在 Lean 层与 ∼ₙ 焊接"（而非只在注释声明），
以下两条 cc-spiral 已验证可证（依赖 `Spiral.gaugeFix`/`Spiral.liftLevel`/`Spiral.Fiber`）：

```lean
-- gauge 规范固定后的级别读出（Option：多义纤维归约为唯一截面级别，或 none）
def gaugeFixLevel (f : Spiral.Fiber) (floor : Nat) : Option Nat :=
  (Spiral.gaugeFix f floor).map (fun l => l.decomposition.level)

-- ★T₅₂ 桥（L0）：gaugeFix 选出截面 ⟹ 级别读出 = 该截面级别。
-- 把 §3 gaugeFix 规范固定算子与 ∼ₙ 的 gauge 级别分量焊接（计算桥层面）。
theorem gaugeFixLevel_some (f : Spiral.Fiber) (floor : Nat) (sec : Spiral.Lift)
    (h : Spiral.gaugeFix f floor = some sec) :
    gaugeFixLevel f floor = some (sec.decomposition.level) := by
  unfold gaugeFixLevel; rw [h]; rfl
```

**注意**：这只是**计算桥**（gauge 选出截面的级别 = 读出值），**不是**"完整商集唯一性
由 gaugeFix 端到端推出"（codex 复审明确：后者过强）。`I=leaves` 路线下 ∼ₙ 唯一性不依赖
gauge，此桥是可选补充（若想显式连 T₅₂ gaugeFix 到 Lean），非必需。

**判断**：因 `Decomp.lean` 走 `I=leaves` 无条件路线，gauge 桥**不影响** card_one 证明。
是否补此桥 = 价值判断（是否要在 Lean 显式 trace T₅₂ gaugeFix）→ 留 t-decomp-uniq/编排者裁。

---

## 三、两轮 codex(gpt-5.5 high) 异质审计裁定（cc-spiral 实测，供 t-decomp-uniq 防坑）

cc-spiral 对 canon 版本跑了两轮 codex 审计，以下裁定对 `I=leaves` 版本同样适用：

1. **`Subsingleton(Quotient)` + 非空见证 = `|·|=1` 严格替代**（core Lean 无 Mathlib
   `Unique`/`Fintype.card` 时）。codex 复审明确 **YES**：基数≤1(Subsingleton)∧≥1(非空)。
   → `Decomp.lean` 的 `decomp_quotient_subsingleton` + `decomp_nonempty` 组合**严格合法**。

2. **声明膨胀防坑**（codex 两轮共同批评，`Decomp.lean` 需自查）：
   - 不要在注释说 ∼ₙ 是"**完整语义规范代表**"——`I=leaves` 丢弃 `Direction`/`centers`/
     `level`，是 **leaves 摘要核**（比 MoveOutcome 标签细，比完整 Move 语义粗）。诚实标
     有效域 = "语义只由 leaves 序列决定"的范畴。
   - 不要声称 `X/∼≅P` **双射**——`liftedClass` 是单向良定义不变量（无逆/inj/surj）。
     完整双射(不变性+完备性+可实现性)是 T-trend/T-bsp 工位。诚实标"不变量良定义"。
   - 若用 `Decomp h := {t // I t = h}` 子类型，诚实标它以"I=h"为**前提**，非由 gaugeFix
     规范固定**推导落入**（端到端流水线 gaugeFix→Decomp 若未建，注释勿暗示已建）。

3. **`I=leaves` 非标签核重言确认**（codex#1 规则）：∼ₙ 按 `I=leaves`（底层 segment 序列）
   的核，**不是** 按分类标签 `MoveOutcome` 的核——leaves 区分由底层结构决定，非走势类型
   标签。`decomp_quotient_nontrivial`（不同 leaves 不等价）是合法非平凡见证。codex 确认
   leaves 核**不是** codex#1 禁止的"按分类标签 I 商分类"。

---

## 四、cc-spiral stand down

cc-spiral 任务 #63 标 completed（superseded by t-decomp-uniq，零件已交接）。
`Spiral.lean` 由 t-spiral owns（cc-spiral 不再编辑）。∼ₙ 组装权威 = `Strict/Decomp.lean`。
