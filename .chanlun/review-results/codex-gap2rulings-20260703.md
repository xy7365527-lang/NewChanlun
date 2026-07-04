# gap-master2 八条存疑交裁：Codex 异质裁定 + 我方复核

工位：ws-gap2rulings | Task #157 | 2026-07-03 | 命令：`codex exec --skip-git-repo-check
--sandbox read-only`（Lead 指定用法）| 原始交互全文：`codex-review-20260703-2059.md`

**方法**：为 8 条各自组装证据包（终稿原文 + 对应分片报告 + 涉案代码原文摘录），一次性
提交 Codex 静态核验，要求逐条给出裁定/独立代码证据/指出我方误读/翻转条件。收到裁定后
本工位对关键新证据（尤其此前遗漏的 `cand_predicate.rs`）做了独立抽查复核（非照单全收）。

**性质声明（异质裁决≠实施授权）**：以下裁定是对"缺口是否存在/严重性判断是否成立"的
判定，**不是**实装授权。即使某条判"成立"，也只是确认缺口真实存在，后续是否/何时实装
由 Lead 按依赖序另行调度。

---

## 一行摘要（8 条）

| # | 条目 | 裁定 |
|---|------|------|
| 1 | s2① P0-2/P0-3 vs #114/#140 覆盖关系 | **部分成立**（P0-2/P0-3 未被 #114/#140 覆盖，但已有独立雏形：carrier-id position node + separate.rs P^sep 骨架 + `‖ΔN‖_1` 诊断——A9/A11 范围应收窄而非清零，见下） |
| 2 | s1① γ/gate 分派层 2B/3B 坍缩 | **部分成立**（gate 分派层确有坍缩，但 Type2/Type3 大多同路处理，经济含义上部分无害） |
| 3 | s1② 二值背驰谓词 D_t^δ 独立形式化 | **部分需要**（合取结构已在 `cand_predicate.rs::DivCand` 集中实现，只缺统一 canonical 命名/符号——我方原判有误读，已漏读该文件） |
| 4 | s4 κ 敏感性网格 B35 | **成立，维持 B 登记** |
| 5 | s4 Comparable_ℓ 严格合取 B36 | **不成立，可撤销**（已在 `cand_predicate.rs::DivCand` 条件2 实装——我方原判有误读） |
| 6 | v1 A4 AncOK 放宽 | **不成立，建议销项**（发现 A 已在生产路径通过 ElementId 完全修复，非"部分缓解"——我方原判方向正确，Codex 独立核实确认） |
| 7 | v1 A5 γ_t 四桶 | **成立，确认性缺口**（同意 #158 结论） |
| 8 | 新：MuClass vs mutex.rs 状态同构性 | **生产 π 路径成立（无 680 式漂移），非生产诊断路径有明确边界（None 是诚实声明非漂移）** |

**对 A9/A11 范围的关键影响（条目1）**：不是"全量按原始 P0 清单从零实装"，也不是"已
覆盖可销项"，而是**范围收窄**——P0-2 的容器身份（carrier id 而非买卖点叶子）已在
`coverage.rs`/`interp.rs` 落地并有专项测试，缺的是"严格 `hash(carrier,entry,side,
generation)` 多实例身份"（ceiling，非从零）；P0-3 的 `P^sep`/`Net`/`‖ΔN‖_1` 代数骨架已在
`separate.rs`/`coverage.rs` 落地，但**未接入生产 R/TW 账本**（separate.rs 自身声明"不依赖
strategy::ledger，standalone"）——缺的是"接线"而非"建骨架"。#166/#167 的任务描述建议
按此收窄重写验收范围。

---

## 逐条详述

### 条目 1：s2① P0-2/P0-3 与 #114/#140 覆盖关系

**裁定：部分成立。**

**Codex 原文要点**：#140 只是单账本资金漂移事件（`TwEvent::Realize`），不构成 position
node 或 overlay 账本；#114 是既有净额账本上的 ShortDiff 反事实开关，不是新账本层。但
"完全未覆盖、A9/A11 不收窄"说法过强——`strategy/coverage.rs:475-494`
（`attach_bsp_carrier_indexed`/`attach_bsp_to_tree`）已经把持仓身份取为 carrier id（
`hostOf(g)`）而非买卖点叶子，`strategy/interp.rs:1518-1548` 有专项测试断言
`position_node_identity_is_carrier_not_leaf_ordinal`；`ledger/separate.rs` 已有
`P^sep`/`Net`/`Eat^sep` 代数骨架，`coverage.rs` 已有 `overlay_net_delta`/`‖ΔN‖_1` 诊断。

**我方复核（独立验证 Codex 引用的关键文件）**：
- 读取 `interp.rs:1518-1548`：确认存在测试 `position_node_identity_is_carrier_not_leaf_
  ordinal`，断言候选 id == carrier hostOf(g) 的 id（非叶子新 ordinal）——Codex 引用准确。
- 读取 `ledger/separate.rs:1-56`：确认 `P^sep`/`Net`/`Eat^sep` 代数骨架真实存在，但模块
  头**自身明确声明**："全文件 L0/L1（结构镜像...零信息增量）...不是任何缠论盈利/实盘
  有效声明"、"隔离声明：本文件不依赖 strategy::ledger（净额账本 R=Π-A-W）、
  nautilus::account_adapter（net_position）"——即这是一个**纯代数结构验证**（对齐 Lean
  `SeparateLedger.lean`），**未接入生产资金流/决策路径**。这是对 Codex"部分落地"判断的
  重要补充：P0-3 的"落地"程度是"孤立的数学结构验证"，不是"生产可用的 overlay 账本雏形"
  ——两者严重性/剩余工作量差异很大，不应混淆。

**边界条件（翻转）**：若 A11 的验收标准是"存在 P^sep 代数结构"，则 P0-3 已部分满足；若
验收标准是"生产决策路径能读写 overlay 头寸并影响真实开平仓"，则 P0-3 实质仍是**从零**
（separate.rs 的隔离声明是硬边界，不是可以忽略的实现细节）。P0-2 同理：carrier-level
身份已在生产路径（`interp.rs`/`coverage.rs` 是生产文件，非独立数学模块），但多实例
`generation` 字段（同一 carrier 上先后多次开平仓的区分）仍缺，ceiling 未完成。

---

### 条目 2：s1① γ/gate 分派层 2B/3B 坍缩

**裁定：部分成立。**

**Codex 原文要点**：`bsp_cand_type`（`econ_positive.rs:739-780`）确实按
`Type1>Type2>Type3` 坍缩非互斥 bits，测试 `bsp_cand_type_priority_dispatch` 固定断言
`buy2+buy3→Type2`。但 `cand_delta_base_gate`（`econ_positive.rs:858-875`）与 XZD 路径
（`econ_positive.rs:1503-1533`）把 Type2/Type3 大多导向同一 completion/retest 实现，C2
跨条目查找（`econ_positive.rs:1240-1253`）只补 `type2_confirmed`，不能完整恢复 Type3
provenance。

**我方复核**：独立读取 `econ_positive.rs:1930-1945` 测试代码，确认
`bsp_cand_type_priority_dispatch` 测试与坍缩逻辑存在，与 Codex 引用一致。

**边界条件**：若下游需要"候选携带完整原始 bits 供 Type3 独立判定"（如力度/风险口径需
要区分"纯 Type3"与"Type2+3 共生"两种情形的经济含义），则该坍缩是真缺口；若 Type1>2>3
优先级本身就是生产语义决策（"归根结底都是第一类"精神的延伸，即"能归类到更高优先级
类型就归类"），则坍缩是合理设计非缺口。本条目不建议自动升 A——建议登记为 B 类留白，
交后续 A6-proxies（Candidate 携 ForceProxies 透传打通，Task #159）工位判断是否顺路解决
（同属"候选携带更多原始信息"的透传问题）。

---

### 条目 3：s1② 二值背驰谓词 D_t^δ 独立形式化

**裁定：部分需要（我方原判有误读，已订正）。**

**Codex 原文要点**：合取结构 `DivCand = direction ∧ Comparable ∧ Extreme ∧ Weak` 已经
在 `classifier/cand_predicate.rs` 集中实现（非分散在 divergence.rs 各函数）。仍缺的是
统一 canonical 命名（`D_t^δ` 符号未落地为类型名），且 `signal.rs` 里的 `.diverges(...)`
原始调用需确认是否绕过 `div_cand`。

**我方复核（独立验证，发现我方初次核对遗漏该文件）**：读取 `cand_predicate.rs:1-20` +
`:118-150`，确认：
```
DivCand^δ_{Θ,ℓ}(s,t) :=
  [dir(s) = −δ]                          ← 条件1：方向
  ∧ [∃s'∈S^vis: Comparable_ℓ(s',s,t)]   ← 条件2：Comparable
  ∧ [Extreme^δ(s',s,t)]                  ← 条件3：Extreme
  ∧ [Weak^δ_{Θ,ℓ}(s,s',t)]              ← 条件4：Weak/力度衰减
```
代码逐条件短路判定（`if !cond { return false }`），确实是显式的四条件严格合取——
**我方原判"代码没有集中体现 B=Context∧D"是误读**，本工位在初次核对时只搜索了
`divergence.rs`，未发现 `cand_predicate.rs` 这个更上游的 gate 判定文件。这是本轮裁定中
唯一一处我方证据收集有实质遗漏（而非解读分歧）。

**订正后判定**：合取结构已存在，只是命名不是 canonical `D_t^δ` 符号——这是**命名/文档
层缺口**，不是**逻辑/功能层缺口**。是否需要为此专门建一个类型别名或文档映射表（把
`bsp_div_cand`/`DivCand` 与原文符号 `D_t^δ`、`B=Context∧D` 显式对应），建议登记为低优先级
B 类（文档忠实度），不升 A。

---

### 条目 4：s4 κ 敏感性网格 B35

**裁定：成立，维持 B 登记。**

**Codex 原文要点**：`RiskPolicy.kappa` 默认 0、`try_new` 非负不变量，`enter_ready`/
`buy_core_legal` 直接用 `η_*=L^wc+κQ`；未找到 κ 网格测试。buffer1/2 侧确有显式敏感性
网格测试 `margin_buffer_sensitivity_grid`（`strategy/risk.rs:1268-1283`）。
`docs/formal-chain/margin-model-design.md` 在仓库中不存在，无法核实 §3.5 原文强制语气。

**我方复核**：独立 grep 确认 `risk.rs:1270 fn margin_buffer_sensitivity_grid()` 测试
真实存在；`margin-model-design.md` 确认在 `docs/formal-chain/` 下不存在该文件（可能是
shard4 报告转引自其他位置的表述，或该文档已被归并/改名，需要 genealogist 后续核实
出处，但不影响本条裁定——κ 与 buffer 同为决定 EnterEarning/BuyCore 边界的风险参数，
现状确实厚此薄彼）。

**边界条件**：维持 B35 登记（低优先级留白），若后续找到 κ 固定为 0 不需要敏感性分析
的正式规格声明，或发现隐藏的 κ grid 测试，则撤销。

---

### 条目 5：s4 Comparable_ℓ 严格合取 B36

**裁定：不成立，可撤销（我方原判有误读，已订正）。**

**Codex 原文要点**：`sigma_higher` 确实只在 `selector.rs` 填 z 向量语境值，不进
`divergence.rs` gate——这部分我方读法正确。但 Comparable_ℓ 并非靠 `sigma_higher` 实现，
而是在 `cand_predicate.rs::DivCand` 条件2 独立实装（要求同一 parent Compose 语境下存在
前一个同向 rmove，否则直接 `return false`）。

**我方复核**：与条目3 共用同一段代码验证（`cand_predicate.rs:131-136`），确认条件2
`Comparable_ℓ` 的判定逻辑真实存在且是短路必要条件（不满足即整个 `DivCand` 判 false，
背驰候选直接不成立）——这是**门控**而非被动记录，满足"严格合取"要求。

**订正后判定**：B36 应撤销——把"sigma_higher 未作 gate"误等同为"Comparable_ℓ 未作
gate"是我方（及原 gap-master2 shard4 报告）共同的误读，两者是不同的实现路径。

---

### 条目 6：v1 A4 AncOK 放宽

**裁定：不成立，建议销项（我方原判方向正确，Codex 独立核实确认，更深入）。**

**Codex 原文要点**：确认我方"`recursive_tower.rs:60-75` 是历史发现说明非当前 bug"的
判断基本正确。生产路径唯一使用 `ancestor_close_by_id`（ID 结构映射版），旧
`ancestor_close`（值比较版）只在测试块被 `active_set_step` 调用。更进一步：Codex 独立
核实了生产 Stale 分支本身（`coverage.rs:1757-1820`）明确检查 persistent registry 状态
（`held_state`），非边界的 Closed/Invalidated 会被 prune，只有真正的边界根才产生
`parent_id: None`——这不是"索引错位伪造"，是"真实结构下的合法结果"。

**我方复核**：读取 `coverage.rs:1750-1845`（Stale 分支实现区域），确认代码路径与
Codex 描述一致：`HeldLegMatch::Stale` 分支查 `registry.held_state(leg)`，区分
`LivePresent`（理论不可达但按持久身份保留）等状态，注释明确写"检查 persistent
registry...Invalidated 才 prune"（本工位对该行注释的原文引用：见本文件"我方核对"章节
外的直接代码摘录）——支持"发现 A 已完全修复"的判断。

**订正后判定**：A4 应从"中·交裁"改判**销项**（发现 A 已完全修复）。`persistent.rs`
的四态 overlay（LivePresent/LiveDetached/Closed/Invalidated）解决的是元素跨 bar 生命周期
持久性问题，与"祖先链查找用值比较还是 ID 比较"是两个不同层面的问题——gap-master2 终稿
把两者混为一谈（"部分缓解在案"）是误读。`persistent.rs` 自身是否还有独立缺口（其模块
头自称"最小修复"，"彻底修复"标为 ceiling）应另开条目跟踪，不应挂在 A4 名下。

---

### 条目 7：v1 A5 γ_t 四桶

**裁定：成立，确认性缺口。**

**Codex 原文要点**：独立核实同意 #158（`a5-gamma4-confirm-20260703.md`）结论——
`Deficit/PositiveUnsafe/PositiveSafe/EtaBucket/eta_bucket` 在 `rust/src` 无真实实现，
`MuClass` 字段止于 `t_stage`（第14维），无第15维。底层连续量（`TwState::tw`/`l_wc`/
`eta_star`）与两条布尔闸门（`enter_ready`/`buy_core_legal`）精确对应 PDF 公式，但离散
四态分类值本身从未具体化——false 分支把三种迥异状态坍缩为同一个匿名布尔。`RiskMode`
（M0-M4）是独立的保证金/权益状态机，非 γ_t 四桶的另一种实现（排除混淆）。

**我方复核**：#158 本身已用 `Read pages` 逐页核对 PDF 原文（非转引），Codex 独立静态
grep 复核给出一致结论，两个独立信源（今日已完成的专项工位 + 本轮异质审查）交叉确认，
证据链完整。不再重复验证。

**边界条件**：只有找到现有四桶 enum/字段，或规格明确撤销 z 向量 ηBucket 维度，才会
翻转。建议按 #158 下游推论——并入 Task #149 范围（zdims-impl，已 completed，若已收尾则
新开实装任务），不新开独立任务避免重复立项。

---

### 条目 8：新——MuClass 侧与 mutex.rs 解释器状态侧 z 维度同构性

**裁定：生产 π 路径成立（无 680 式漂移），非生产诊断路径有明确边界（None 是诚实声明，
非漂移）。**

**Codex 原文要点**：生产 fill loop（`runner.rs`）在同一 bar 内先处理 Realize 漂移，
随后计算 `k_theta_risk_gate` 拿到 `risk_mode_i`，与当前 `tw.stage` 一起放入同一个
`ZExt { risk_mode: Some(risk_mode_i), t_stage: Some(tw.stage), ..NONE }`——这个 `ext_i`
同时喂给 χ 查询（filter_gamma_with_admission）和入场 `entry_z` 登记，P1-P4 决策也拿
同一个 `tw`/风险模式组 `TwStepCtx`。三处消费方（χ 训练登记、χ 查询过滤、P1-P4 决策）
共享同一份变量，非各自独立重算——无 680 式"分类层算出但消费侧丢弃/另算"的漂移。存在
`ZExt::NONE`/字段主动置 `None` 的路径（`econ_positive.rs` 统计信号、`l3_delta_r_alpha.rs`
诊断枚举），但这些是**诊断/统计侧的显式边界**（同 231 号"诚实 None 不伪造"先例），非
生产决策链的漂移。

**我方复核（独立验证核心证据）**：读取 `runner.rs:735-810`，确认：
1. `②''` 段确实在 TwStepCtx 构造**之前**处理 `TwEvent::Realize` 漂移（硬边界2，同
   gap3-realize-impl 结果包描述一致）；
2. `ext_i` 的构造代码（约 `runner.rs:777`）确实是
   `ZExt { risk_mode: Some(risk_mode_i), t_stage: Some(tw.stage), ..ZExt::NONE }`——
   `risk_mode_i` 直接来自本行上方 `k_theta_risk_gate(...)` 调用的返回值（同一次调用，
   非重新调用 `risk_mode()` 函数用不同输入重算），`tw.stage` 直接读当前 `tw` 变量（同一
   账本状态，非快照副本）；
3. 代码注释显式论证同源性："同一 ext_i 同时喂 χ 查询（filter）与训练登记（entry_z）
   ⟹ 训练/查询同口径在共享变量层保证（G2 护航点同款）"——这是代码作者自己给出的同构性
   证明，与 Codex 的独立读法一致。

本工位认为 Codex 对本条目的核实是本轮 8 条中方法论上最扎实的一条——它没有停留在
"字段文档声称同源"（我方初步核对止步于此），而是往前一步找到了实际消费 `ext_i` 的
三处调用点（χ 训练/χ 查询/P1-P4 决策）确认它们共享同一变量绑定，这正是 680 号判据
要求的"消费侧真核实"标准动作。

**边界条件**：若未来发现某条生产路径（真实开仓/真实 χ 查询）绕过 `ext_i` 直接用
`ZExt::NONE`，或 `pi_theta_step_traced` 在处理 P1-P4 时用了一个与 `ext_i.risk_mode`/
`t_stage` 不同源的 `TwStepCtx`，则本条裁定翻转为"存在漂移"。

---

## 结果包六要素

1. **结论**：8 条中，1 条（A4）建议销项，1 条（Comparable_ℓ/B36）建议撤销，2 条
   （κ网格/B35、γ_t四桶/A5）维持/确认，3 条（P0-2/3覆盖、gate坍缩、D_t^δ独立形式化）
   部分成立需按边界条件细化范围，1 条（MuClass同构性）新确认无漂移但需持续监控生产
   路径不被后续改动破坏同构性。最关键影响：**A9/A11 的实装范围应收窄**——P0-2/P0-3
   均已有独立雏形（carrier-id 持仓身份 + P^sep 代数骨架 + `‖ΔN‖_1` 诊断），缺的是"接入
   生产账本/补 ceiling"而非"从零建"。
2. **定义依据**：`.chanlun/review-results/gap-master2-final-20260703.md` 存疑交裁清单
   1-8 条原文；`nesting-fugue-conformance-20260703.md`（P0-1/2/3 原始规格）；
   `f3-fugue-backtest-20260703.md`、`gap3-realize-impl-20260703.md`（#114/#140 实际改动
   范围）；`a5-gamma4-confirm-20260703.md`（#158 独立核实）；680号谱系（双侧判据出处）；
   代码锚点见各条目"我方复核"小节。
3. **边界条件**：见各条目内单独列出；总体边界——本轮全部为 L0/L1 静态代码核对（无
   运行时 trace/L2 实证），若后续代码改动（尤其 #148/#149 之后的合并）改变了本轮引用
   的行号/逻辑，需要重新核对（本文件锚点均标注为 2026-07-03 gap3-rework-codex9-fix
   分支 HEAD 状态）。
4. **下游推论**：Task #166（A9-posnode）、#167（A11-nestsink）应按条目1 收窄的范围
   重新定义验收标准（接入现有骨架而非从零建）；条目2 建议并入 A6-proxies（#159）而非
   独立处理；条目3 的命名缺口建议归入低优先级 B 类文档忠实度工作；A4（条目6）建议
   genealogist 在谱系中把该缺口标记销项，`persistent.rs` 自身的独立 ceiling 缺口另开
   条目追踪（不挂 A4 名下）；条目7 按 #158 建议并入 #149 范围。
5. **谱系引用**：680（双侧判据出处，条目8 的方法论来源）；639/667（σ_p/σ_higher 族）；
   231（形式化有效域，B37 留白/诚实 None 先例反复引用）；090（严格性/声明膨胀，条目1
   "部分落地"程度需精确表述不可笼统）；652/275（局部依赖，条目6 A4/persistent.rs 不
   混淆）。
6. **影响声明**：新增本文件 + `codex-review-20260703-2059.md`（原始交互记录）。不改
   任何 `.rs` 代码。本裁定影响 gap-master2 终稿存疑交裁清单的 8 条判定状态，及下游
   Task #166/#167/#159/#149 的范围界定（范围调整由 Lead/对应工位后续执行，本工位不做
   实装）。
