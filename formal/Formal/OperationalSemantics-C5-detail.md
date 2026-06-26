# C5 操作类型完全分类·两层定理 + π 完全应对 — 详情（task #67 T-op）

**文件**：`formal/Formal/OperationalSemantics.lean`（C5 Layer2 段，+14 定理）+ `formal/Phase2/Claim6_OperationalSemantics.lean`（C5 收口段）
**owner**：t-operational ｜ **标准**：chanlun-strict-classification-standard.md §A codex#1 两层规则 + §B 第6部分

## 结论

C5 操作类型 {买,卖,加,减,持,平} 升级到最严标准的 **两层定理** + 标准第6部分 **完全应对 π**，全部 L0 机器验证通过。

- **Layer1（标签商，保留+显式命名）**：`OpTag` 6 构造子穷尽（`optag_exhaustive`）= 按操作类型标签商分类。codex#1 规则：显式命名「按标签商分类」，不冒充语义双射。
- **Layer2（语义双射尝试，诚实失败）**：`Exec`（账户状态+动作+可行性）+ `OpEqSemantic`（账户转移效果等价：pre 相同 ∧ applyExec 相同）+ `I:Exec→OpTag`。
  - `exec_eq_gives_semantic`：标签+参数(qty/price) ⟹ 语义等价（弱不变量，忠实方向）。
  - `i_not_complete`：**complete 失败反例**——买1单位 vs 买10单位，同 OpTag `buy`（I 相同）但账户转移不同（units +1 vs +10）⟹ `∃ x y, I x=I y ∧ ¬OpEqSemantic x y`。证明商集非单射。
  - `optag_is_label_quotient_not_semantic`：诚实裁定 OpTag = 标签商 ⊋ 语义商（同标签含异语义 Exec），**不是** `Exec/∼ ≅ OpTag`。
- **标准第6部分 完全应对 π**：`pi:State→OpTag`（3×3 持仓×信号全覆盖全函数）。
  - `pi_total`：∀状态有应对（无未定义状态/死端点）。
  - `pi_in_optag`：应对穷尽于 6 操作类型。
  - `pi_no_lookahead`：同状态同应对（π 只依赖当前状态 Cₗ(hₜ)，不依赖未来=无前视因果，标准第3部分 π 形式）。
- **Claim6 收口**：`decideOp`（π_op 表全函数）+ `decideOp_no_lookahead` + `decideOp_in_optype`（decideOp 作为类型层完全应对 π）。

## 边界条件

- **语义 ∼ 维度**：OpEqSemantic = 账户转移效果（units delta + cash delta）。由 task #67 spec 钉定（非开放选择）。若改 ∼ 维度（如加入时机/级别）→ complete 失败结论可能变（但更细的 ∼ 只会让标签商更粗，complete 仍失败）。
- **complete 失败翻转条件**：若 OpTag 携带 qty/price（不再只是类型）→ 可能恢复单射。但那违背缠论"操作类型不决定数量 M"（M 属 payoff 层 L3，603 §诚实分层）——故 complete 失败是本质的。
- **π 全函数边界**：State = (持仓 3 态 × 信号 3 态)。若状态空间扩展（加级别/成本阶段）→ π 需重新全覆盖。当前 3×3 穷尽。

## 下游推论

- C5 在严格标准下定格为「标签商分类 + 完全应对 π」，**不是语义双射**——下游引用 OpType 时不可假设它决定完整操作语义（数量/价格在 payoff 层）。
- π 无前视因果 ⟹ 可对接 Rust 引擎实时决策（aₜ=π(Cₗ(hₜ)) 只读当前及之前数据，bit-exact/env-gate Phase3）。

## 谱系引用

- codex#1 两层规则（标准 §A）：μF 语法穷尽不自动给语义双射，必须显式命名标签商——本模块遵守。
- 603 §诚实分层（L0/L3）：OpType 给类型不给数量 M——complete 失败的概念根因。
- 605 OperationalSemantics（Σ 几何原子）vs 本 C5（操作类型 π_op 层）：不同层，不冲突。
- codex 自审 session：见 TaskUpdate metadata（gpt-5.5 high，read-only）。

## 影响声明

- 改动 `formal/Formal/OperationalSemantics.lean`（+C5 Layer2 段，14 定理）+ `Phase2/Claim6_OperationalSemantics.lean`（+C5 收口段，2 定理）。
- **未碰** lakefile.toml / Formal.lean（硬约束）。
- ⚠ **共享 `lake build` 红**：并发工位 T-trend(#58) 改 `TrendTrichotomy.lean:26` 加 `import Formal.RecursiveConstruction`，与 RecursiveConstruction 互 import = 构建循环。**非本 C5 改动**（我两文件单文件全绿，只 import BSPLabels）。已 SendMessage Lead 路由给 #58 owner 解循环。

## 验证（证据）

- `lake env lean Formal/OperationalSemantics.lean` → 零 error/warning/sorry/admit/axiom
- `lake env lean Phase2/Claim6_OperationalSemantics.lean` → 零 error/warning/sorry/admit/axiom
- `#print axioms`：`pi_total`/`pi_no_lookahead` 无 axiom 依赖；`i_not_complete`/`optag_is_label_quotient_not_semantic` 仅 propext/Quot.sound（无 user axiom，无 sorryAx）
- `lake build`（全项目）：红 —— **因 T-trend 并发构建循环，非 C5**（已上报 Lead）

## escalate

- **无 C5 概念冲突**（complete 失败是诚实刻画，非定义矛盾——operation type 不决定数量是已结算的 L0/L3 分层）。
- **已 /escalate 共享构建循环**（T-trend #58 引入 TrendTrichotomy↔RecursiveConstruction 互 import，非 C5 范围）。
