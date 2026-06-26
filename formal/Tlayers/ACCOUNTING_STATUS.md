# 会计层 T₃₃-T₄₆ Lean 形式化状态（task #49 / t-accounting）

编排者裁决（2026-06-25，覆盖 codex D1 + Lead 原"仅 T₄₂"消息）：**会计层 T₃₃-T₄₆ 全部进 Lean E-set**。
路由：报告用 TaskUpdate + 本文件，**不 SendMessage main**；仅 /escalate 才发 team-lead。
认识论等级：Lean = L0（定义内蕴，结构/代数定理）。Rust N8/A4/N1 守卫保留作 L2 补充（不被 Lean 取代）。

## 进度总览

| 定理 | 内容 | Lean 状态 | 模块 |
|------|------|----------|------|
| T₃₃ | 同一笔物理交易双层记账 | ❌ 需重写(vacuous已删) | 见下"守恒模块 codex FAIL 教训" |
| T₃₄ | 股数守恒 Σunits=N_base | ❌ 需重写(vacuous已删) | 必须建真 active voice units 分布+spawn/close 真转移 |
| T₃₅ | NAV 价值中性(同价操作前后不变) | ⚠️ 部分(long-only子命题) | nav 漏空头 capital 项,需补完整会计 NAV |
| T₃₆ | earning 多空构造不对称 | ❌ 需重写(vacuous已删) | short:=no-op 是把"不可构造"编码成不变,非证不存在构造 |
| T₃₇ | child.P&L≡parent.cost_reduction 有条件 | TODO | (L0:有条件恒等+无条件NAV形式) |
| T₃₈ | N不主动加仓;N_base双向重定基 | TODO | (L0:守恒守"两侧一致"非"N常数") |
| T₃₉ | 递归链守恒(任意深度) | ❌ 需重写(vacuous已删) | 平坦list丢了树深度,需真森林flatten保真定理 |
| T₄₀ | 子voice翻转=会计断面 M=N | ⚠️ 部分(flip对|units|忠实,缺事件分解) | flip_preserves_magnitude OK,但缺"关闭+反向开启"分解 |
| T₄₁ | 零强平(否定线先于保证金线) | TODO | (L0:A₀⟹买点必现⟹C先消费,条件式) |
| **T₄₂** | **孤儿不可能·森林良构结构侧** | **✅ DONE** | **Accounting.lean (8定理,codex 6/6 PASS)** |
| T₄₃ | N5⊥N7严格解(confirm两路消费) | TODO | Accounting (L0:同一fire分两路,非互斥) |
| T₄₄ | 递归深度有限 | TODO | Accounting (L0:级别良序严格递减⟹有限) |
| T₄₅ | 操盘结构周期性(角向φ-周期) | TODO | Accounting (L0:有限状态确定转移⟹周期) |
| T₄₆ | 零破产 NAV≥0 | TODO | Accounting (L0:T₄₁+T₃₄+T₃₅组合,条件式) |

## ★守恒模块 codex FAIL 教训（vacuous 失败模式，后续重写必读）

第一版 `AccountingConservation.lean`（T₃₃-T₃₆/T₃₉/T₄₀）被 codex high 审计 **6/6 FAIL**，
已删除（no-patch-mentality：vacuous 不保留）。codex thread 见 tasks/b4fi3cwdp.output。
失败根因（formalization-validity-domain 合成确认偏差 + 把结论塞进定义）：

1. **最关键 vacuous**：`Entry.net (_:Entry) := 0` 直接定义为 0，不用 delta ⟹ `ledger_net_zero`/
   `totalUnits_conserved`/`ledger_net_zero_any_depth` 全退化成「0 的归纳传播」，不是会计守恒。
   → 重写要求：Entry 显式建模父腿/子腿(debit/credit)，net = (+delta)+(−delta) **计算**出 0，非定义。
2. **不存 voice 分布**：Entry 不存 active voices / 父子视图 / spawn/close 后单位分布 ⟹ 没捕捉
   「Σ active voice.units = N_base 单位流转不增不减」。→ 重写要求：建真 units 分布 + spawn(父−m/子+m)/
   close(子归还父) 真转移操作,守恒 = 转移保持 Σ(真证,非 0 传播)。我最初的 `spawnAt`/`List.modify`
   方向是对的(真转移),但卡在 v4.31 `List.modify` API(参数序+lemma名 modify_cons_zero/succ 不存在)——
   后续用 `List.set` + getElem lemma 或查正确 modify API,或自定义递归 set 函数避开。
3. **平坦list丢深度**(T₃₉)：`l₁++l₂` 只表达「任意长列表」非「任意递归深度森林」。→ 用真树(复用 T₄₂ 的
   `Voice` nested inductive)+ flatten 保真定理。
4. **NAV long-only**(T₃₅)：`nav=free+long×c` 漏空头 capital/liability/cover ⟹ 是子命题非无条件守恒。
   → 补空头项(子空 capital,见 §738 D3.2 nav=free+Σ多头units×c+Σ空头capital)。
5. **earning no-op**(T₃₆)：`applyEarning short:=units` 把「不可构造」编码成「定义成不变」,且 units:Int 允许
   负数削弱「现货载体不能表示负股数」。→ 用 ℕ(非负)载体 + 证「不存在合法 short earning 构造」(空类型/¬∃)。
6. **flip 缺事件分解**(T₄₀,最接近PASS)：flip 对 |units| 忠实,但没表达「同方向关闭+反向开启」会计事件。

**唯一 codex 认可方向**：T₄₂(Accounting.lean) 6/6 PASS——它建了真 datatype + 真递归操作 + 可证伪见证。
守恒律重写应仿 T₄₂：真结构对象 + 真操作 + 反例见证(证明定理有判别力,非 vacuous)。

## 账本轴范式 reform（#42）

★编排者口径补充（2026-06-25，决定性判据）：**不只是证明，要用递归方法推导真完全分类。**

reform **不是**把 Burnside 轨道枚举 Lean-证明一遍（那只是形式化轴范式，不够）。而是：
**把账本对象重铸为初代数（找它的构造子），让"完全分类"从 initial algebra 结构归纳原理
by construction 导出**——完全性是递归构造子穷尽的产物，不是"枚举46轨道+证明枚举正确"。

判据（后续工位 self-check）：
- ❌ 不够（轴范式证明）：定义 D∞ 群作用 + 证轨道计数=46 / 定义58槽 + 证 Burnside 506→46→2→1
- ✅ reform（603 真完全分类）：定义账本递归 datatype（inductive,找构造子）+ 构造子穷尽 by induction
  ⟹ 完全分类（仿 Formal/RecursiveConstruction.lean 的 outcome_total / Move 初代数 μF 模式）
- 账本若真只能轴枚举、无递归构造子结构 → **诚实 /escalate**（不伪造初代数,no-workaround）

守恒律 L0 同理：by construction 不变量（真递归 datatype + 构造子穷尽归纳），非逐 case，更非
"net:=0 把结论塞进定义"（第一版失败模式）。真 by construction 反例见证 = T₄₂（真 Voice datatype
+ 真递归操作 + orphan_violates_noOrphan 反例）。

状态：TODO（需充分上下文做忠实初代数重铸；对象=58槽/27配置/守恒012，先找其递归构造子）。

## 实装对接（Rust 账本/记账引擎）

bit-exact/env-gate 对接 isolated_fugue / unified_necessity 的 close_voice/try_spawn_cost_gated/
prove_n8_conservation。状态：TODO（Lean L0 结构形式定稿后）。

## T₄₂ 已完成详情（继承用）

- 模块：`formal/Tlayers/Accounting.lean`（自包含，未碰 lakefile.toml/Formal.lean）
- datatype：`inductive Voice (status:VoiceStatus) (children:List Voice)`（nested inductive）
- 关键定理：`closeVoice_no_orphan : NoOrphan (closeVoice v)`（well-founded 结构递归，
  nested inductive 不支持 induction tactic，用 termination_by v + decreasing_by sizeOf）
- 单根：`singleSubject_atMostOneRoot`(前提T₂₇) + `closeVoice_root_count_noninc`(真过程不变量)
  + `closedForest_zeroActiveRoot`(终态特例)。**codex Q5 修正**：单根来源是外部前提T₂₇非datatype内蕴
- 验证：lake build PASS(16 jobs)；#print axioms 仅 [propext,Quot.sound,Classical.choice]；无 sorry/admit/axiom
- codex 审计 thread：`019effeb-8e59-7d03-9a15-6e42e6ec93ce`（真 codex high，Q1-Q6 全 PASS）

## 复用资产

- `formal/Formal/RecursiveConstruction.lean`：Move 初代数 μF、WellFormed、CentersDerivedFrom、归纳守恒模式
- `formal/Formal/RStarNonSpecial.lean`：ValidTower 终余代数、StrictlyIncreasing、PairwiseRel、List 无 Mathlib 替代谓词
- nested inductive 归纳模板（见 Accounting.lean closeVoice_no_orphan）

## 边界条件（整层）

- 守恒律是运行时数值不变量(R类)，Lean 表达的是其 **L0 结构形式**（归纳/代数恒等），
  不是浮点数值守恒（那是 Rust N8 的 L2）。若某守恒律的结构侧在 datatype 上无法表达
  （需运行时浮点语义才成立）→ no-workaround → /escalate，不硬凑。
- T₄₁/T₄₆ 零强平/零破产含 A₀ 公理 + regime 论证：Lean 只能形式化"给定 A₀ 前提则结论"
  的条件式 L0，不能形式化 A₀ 本身（公理）。诚实标注为前提式定理（仿 T₄₂ Q5 修正）。
