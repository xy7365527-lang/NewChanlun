# payoff 层 reform → 真完全分类（详情）

工位：t-payoff（任务 #54）。日期：2026-06-25。
产物：`formal/Tlayers/Payoff.lean`（编译通过，无 sorry/admit/axiom）。

## 结论（codex 019efff7 复核降级后的诚实表述）

payoff 层 **不是**「纯连续 L3 净收益、无任何 datatype 结构」（codex D4「payoff 纯 L3」立场太强，
被编排者覆盖）；本文件也 **不** 声称「payoff 层完全分类」（那也太强）。诚实结论：
**活跃持仓成本管理的结构侧** 是 L0 完全分类（成本阶段三态穷尽 + 操作数量方向钉死 + c 自变量
结构含 L_confirm 且依赖可实现）；真 C 数值响应 + 纯连续净收益数值 + 持仓清仓终止态 = L2/L3
或本论域外，用 `opaque` / 有效域注 显式分开。

reform 成功（限定有效域）：成本阶段状态机 = 有限状态构造子穷尽 → 活跃持仓成本管理结构侧完全分类。

## ★真 codex 复核（异质物证）

- **session id：`019efff7-47df-71b3-8a3d-d6b561880b4f`**（codex-cli 0.125.0, gpt-5.5, reasoning high）。
- codex 独立编译 `cd formal && lake env lean Tlayers/Payoff.lean` → 通过。
- 复核发现 + 本文件修正（no-patch / 声明膨胀 090，已全部吸收并重新编译通过 EXIT=0）：
  - **Q1 CONCERN**：「历史大顶清仓」是持仓 **终止态** 非成本态 → CostStage 加有效域边界注
    （活跃持仓成本管理，非持仓生命周期完全分类）。
  - **Q2 CONCERN**：`zeroing` 缺 `sold>0` → `legalAt zeroing` 补 `bought=0 ∧ sold>0`，
    `zeroing_net_reduce` 收紧为 **严格 `< 0`**。
  - **Q3 FAIL（核心）**：`coeff_depends_on_lConfirm` 只证「CoeffArgs 有 lConfirm 字段」（平凡）→
    **重铸 §5**：(1) `coeffArgs_has_lConfirm_field`（字段存在，诚实命名）；(2) 新增 `coeffSchema` +
    `coeffSchema_depends_on_lConfirm`（一个依赖 lConfirm 的 C 模式证 `C(...,l₁)≠C(...,l₂)`，即
    「c 依赖 L_confirm 可实现」——否定「c 独立」需一个依赖反例）。不再声称形式化真 C 响应（L2/L3）。
  - **Q4 PASS**：`opaque payoffValue` 诚实分层（`payoffValue_is_opaque_witness` 装订说明，保留）。
  - **Q5 CONCERN**：§7「结构侧真完全分类」过称 → 降级为「活跃持仓成本管理结构侧 L0 完全分类」。
  - **总判定 CONCERN → 已按 codex 建议降级**：「活跃持仓成本管理结构侧 L0 完全分类；净收益
    数值与 C 的实际响应仍为 L2/L3。」

## reform 的三个结构侧 datatype（Payoff.lean）

| 结构侧对象 | datatype/定理 | 完全性根据（L0） |
|---|---|---|
| 成本阶段状态机（第31课） | `CostStage` {reducing, zeroing, harvesting} + `stage_total` | 第31课:24/28 三态穷尽；成本本体二值(>0/=0)+归零转移边界，无第四态 |
| 操作数量约束依阶段 | `SwingRound.legalAt` + `netDelta_sign_determined_by_stage` | 第31课:24(reducing 仓位不增)/:28(harvesting 股数增)/:28(zeroing 净减)，方向三分穷尽 |
| 仓位累积轨迹 | `harvesting_position_monotone` / `reducing_position_constant` | :28「股票越来越多」/:24「仓位不增加」by construction 累积 |
| c 自变量复合结构 | `CoeffArgs` + `coeff_depends_on_lConfirm` + `coeffStructure_retains_lConfirm` | codex 019eff17 §4A：c=C(走势配置,中枢列表,级别,L_confirm)，自变量字段穷尽 |
| payoff 操作态总判定 | `PayoffState` + `payoff_structural_complete` | 成本阶段三态 ∧ 走势三/四值(603 outcome_total) ∧ BSP 触发(603 升跌完备) |

## 诚实分层（编排者覆盖 D4 的边界）

- **结构侧 L0**（§1-§5, §7）：成本阶段 / 数量方向 / c 自变量字段——by construction 完全。
- **数值侧 L2/L3**（§6）：`opaque payoffValue : PayoffState → Int`。**不给任何 L0 def**
  （给 `fun _ => 0` 会是声明膨胀 090）。`payoffValue_is_opaque_witness` 只证值存在、不证值。
  「哪个 case 净收益正 / 配额具体数值 / regime 是否溶解」= L2/L3 数据阻塞，本文件不裁定
  （继承 597 pending_verification / 561 b2/563 / 597 L3 否证 BTC−132468%）。

## 与 codex 两 session 的关系（真异质物证）

- **codex 019eff17**（session 019eff17-74b7-7502-b17d-2fe792c20b00, gpt-5.5 xhigh）§4A：
  c=C(走势配置,中枢列表,级别,L_confirm)，否定 bc-concepts 伪造的「c 独立于 L_confirm」。
  → §5 形式化（`coeff_depends_on_lConfirm`：lConfirm 是 C 的真自变量，非冗余字段）。
- **codex 019eff21** §诚实分层 Q2：「仓位数量依成本阶段状态（降成本→归零→挣股票，第31课），
  payoff 层 L3」。编排者覆盖：L3 的 **结构侧** 仍要 reform。→ §1-§3 兑现（成本阶段=datatype）。

## 实装（Rust 引擎，bit-exact）

成本阶段状态机 **已实装** 在 `rust/src/recursive_t/rec_engine.rs`：
- `enum RecStage { CostReduction, CapitalRecovered, EarningShares }`（:553-557）
  = Lean `CostStage { reducing, zeroing, harvesting }`（bit-exact 对应）。
- `as_u8` 0/1/2（:562-566）= Lean `nextStage` 单调链顺序（`costZero_monotone` 镜像）。
- 转移：CostReduction→CapitalRecovered（:1341）→EarningShares（:1347/1382，吸收态）
  = Lean `nextStage` reducing→zeroing→harvesting→harvesting。
- 数量规则：CostReduction 短差仓位不增 / EarningShares 卖后回补股数增（:1329-1356）
  = Lean `legalAt` 方向约束。env-gate：`enable_three_stage`（T_NO_THREESTAGE）
  / `enable_earning`（T_NO_EARNING），OFF 退化 bit-exact（:271-272）。

**no-patch 声明**：引擎已正确实装该状态机，不新增重复状态机。Payoff.lean 是其
**已验证的结构侧真完全分类镜像**（L0 证明引擎实装的三态穷尽 + 数量方向钉死）。

## 验证

- `lake env lean Tlayers/Payoff.lean` → EXIT=0，空日志（无 error/warning/sorry）。
- `#print axioms`：`payoff_structural_complete`/`coeff_depends_on_lConfirm` 不依赖任何公理；
  `netDelta_sign_determined_by_stage`/`harvesting_position_monotone` 仅 [propext, Quot.sound]
  （Lean 标准基础，omega/Int 所用）。无 sorryAx，无自定义 axiom。
- 不碰 `lakefile.toml` / `Formal.lean`（Tlayers 单文件验证层，约束遵守）。

## 结果包六要素

1. **结论**：payoff 层结构侧（成本阶段/数量方向/c 自变量）= datatype 真完全分类（L0）；
   净收益数值 = opaque L2/L3 分开。reform 成功，覆盖 codex D4。
2. **定义依据**：第31课:24/28/30+注36/169（成本三态+数量约束）；603 递归范式（构造子穷尽）；
   codex 019eff17 §4A（c=C(...,L_confirm)）；601（L_confirm 第四轴）。
3. **边界条件（翻转）**：①若成本存在「>0/=0」之外的第三本体态 ⟹ CostStage 三态不穷尽（第31课
   原文无第三本体）；②若 reducing 阶段允许加仓 ⟹ `reducing_no_position_increase` 翻转（第31课:24
   硬禁）；③若 c 可不依赖 L_confirm（已结算同级别中枢序列，codex 019eff17 例外域）⟹ §5 退化为
   三自变量（结构侧仍完全，少一字段）；④若净收益数值可由结构 L0 钉死 ⟹ §6 opaque 应改 def
   （但 codex 判 L3，未达 L2+ 验证，保持 opaque）。
4. **下游推论**：597 payoff 纤维 sub11 的 c 自变量结构获 L0 形式化支撑；603 §诚实分层的「payoff
   层 L3」精化为「结构侧 L0 + 数值侧 L3」；Rust RecStage 获结构侧完全分类背书。
5. **谱系引用**：597（仓位塔 payoff 纤维）/601（L_confirm 第四轴 + sub11 c=C 裁定）/603（递归范式
   + 诚实分层 Q2）/542（σ-不变，本文件不碰配额数值，无冲突）。这是 payoff 层从「轴范式 L3 不可
   datatype」到「递归范式结构侧 datatype 完全分类」的范式重铸。
6. **影响声明**：新增 `formal/Tlayers/Payoff.lean` + 本详情。不改 Rust（引擎已实装，bit-exact 对应）。
   不碰 lakefile.toml/Formal.lean。无 sorry/admit/axiom。最终结算待编排者 /ritual（597-603 整族）。
