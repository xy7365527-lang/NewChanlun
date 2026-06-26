# 督导回路（Supervision Loop）

> 编排者裁定（2026-06-26）：「teach 里面的问题也要反过来回到你那里去，他相当于督导。」

## 这是什么

teach 每一课结尾的「**带走的锋利问题**」不只是给编排者学习用——它**反向施加于蜂群自身的产出**，成为一道**证伪闸**：任何工位声明「我把 X 忠实做完了」，必须先用对应的锋利问题盘问自己，过不了就是未完成（且若声明了已完成 = 声明膨胀，090号）。

教学方法 = 督导工具。lesson 把「识破声明膨胀」的方法结晶下来；本回路把这些方法**作用回**蜂群的代码与裁决。它与 no-patch / no-workaround / formalization-validity-domain / quality-guard 同向，是它们的**可操作锋刃**。

## 常设规则

1. **每出一课 → 本台账追加一行**：登记该课的锋利问题。
2. **每条「已忠实完成」声明 → 必须先过对应锋利问题**：工位在报「done/faithful」前，自答台账里所有相关锋利问题，把见证（witness）随报告附上。
3. **过不了 = 未完成**：不写 workaround，不声明膨胀。失败即开/挂一个被追踪的缺口任务。
4. **L 等级强制**：每个「过了」必须标 L0（类型/骨架）/ L1（缠论规则真编码）/ L2（真实数据撑）——只过 L0 不等于忠实。

## 三层识破法（lesson 0002 §3，督导通用模板）

对任何「我把 X 忠实实现了」：
1. **反退化见证**：给一个**具体例子**证明它没退化（给不出 = 退化/聚合）。
2. **哪一层真理**：L0 骨架（类型/函数自动成立）/ L1（缠论规则真编码）/ L2（真实数据）？
3. **边界条件**：什么情况下结论翻——翻了就是没覆盖那个情况。

## 活台账

| 课 | 锋利问题（摘要） | 施加于（蜂群产出） | 裁决 | 产出缺口/任务 |
|----|------------------|---------------------|------|----------------|
| 0001 | classify 的纤维**恰好**是行为等价类吗？↔ 两个方向都兑现？是 L0 schema 还是 L2 撑？ | codex `Origin/CompleteClassification.CompleteClassifier` | **FAIL** — ↔ 是结构字段假设，全仓从未对具体缠论分类器构造/兑现；`BehEquiv` 在 TrendCompleteClassification 一次未现；真实分类器只证 L0 函数平凡；后向（行为极小性）很可能为假。纯 L0 标签，零 L2。 | **#91** |
| 0002 | 给我一个**输出上级序列长度 ≥ 2** 的具体例子 + 每个上级的 **≥3 个不同下级 witness**。 | cc-foundation-faithful 的 `composeStep`（#87 ChanlunInstantiation §1） | **FAIL** — `composeStep xs := if len≥3 then [compose xs]`，输出长度永远 ≤1（单窗口聚合摘要）；只证单 window soundness，未证 `MovesComposedFrom` 全约束（∀uppers WellFormed + upperMoves.length*3 ≤ lowerMoves.length）。曾声明「忠实完成」= 声明膨胀。 | **#89** |
| 0004a | 「这是自动的」——平台真发那个事件吗（查 `platform_support`：true 还是 false/partial）？触发 hook 是 GUARD（echo systemMessage 就 exit）还是 LAUNCHER（真 spawn 工位）？若 ≠true 且 GUARD，则不是自动的，靠某 Lead/人手动认领兜底——说出兜底人是谁、漏了会怎样。 | **通用督导工具**（施加于任何蜂群「X 是自动/事件驱动」声明，含 Lead 自身的「结构工位已自动起」声明） | **可施加（物证已立）** — `.chanlun/dispatch-dag.yaml` 31× `platform_support` 全 false/partial（true=0）；`.claude/hooks/` 30 脚本全 `-guard/-verify/-prompt/-dispatcher`，无 LAUNCHER（`post-write-edit-dispatcher.sh` 终点 `systemMessage`+`exit 0`，不 spawn）；DAG 自承 `:285` 手动认领（D策略）。真相：自动=Lead 手动解释 DAG（033号），Lead 欠执行=唯一承重点。施加判据：报「结构工位事件驱动已生效」须指出该路径的 platform_support=true 或一个真 LAUNCHER；否则诚实标「D策略手动认领」。 | spec-execution-gap 活教材（声明=自动/实际=手动）；谱系 082/073a/412 |
| 0004 | 「S_Θ 闭环已装配/已实装」——给我一个**已实例化的 `HybridComponents`**，其 `transition` 真改 ledger（Dynamics.δ + Accounting 接进去）。给不出 = 闭环是空环，L2 形式落点悬空。两条增量除 `rfl` 外是否只剩「类型签名强制 intent 读 Class + T 写回完整态」这一条？ | `Strict/HybridStep.lean`（#86 骨架 / #88 重基 pending / #94 引擎实装 in_progress） | **待施加**（骨架 PASS-L0：编译通过零错误，`policy_factors_through_classify:164`+`hybridStep_is_closed_transition:183` 把 Chain 并列合取 `∧` 升级为闭环复合 `∘`，约束前移到接口签名 `intent:HybridState→Class→Intent:84`；但 `transition` 仍是抽象字段，`:264` 自承 adapter 实例化「后续」——闭环实例化=未完成，#88/#94 在途）。施加判据：任何报 #88/#94 done 须附已实例化 transition 改 ledger 的 witness。 | **#88 / #94（追踪中）** |
| 0005 | 「账本闭环已重锚」——①那个单向棘轮维度 `stage` 还在吗？给两个 TW 三量相同、仅 stage 不同的态，证投影**没塌缩**（`not_isomorphic_stage_collapses` 仍可复现）。②OQ-9 端A 的 raw violation witness（带未闭 legacy 腿进 earning + 亏损闭合使 cumNetCash 下降）重锚后**还保留**吗，还是被顺手加入口守卫抹掉？③端B 给的是**单步**定理还是 `oq9inv_trace` 带初态的 **trace 级** invariant（单步挡不住绕路）？三缺一 = 把两会计范畴糊成一个。 | **#101**（双账本桥+HybridAssembly 重锚，单 owner 守 HybridAssembly+TotalWealth+新桥）；通用施加于任何「账本/闭环已重锚/已对齐」声明（含 #93/#94 引擎双账本写回） | **可施加（物证已立）** — `TotalWealth.lean` 三反例 + OQ-9 trace 级链全 machine-checked（`:592/:341/:377/:438/:453/:480`），L0 全程零 sorry。施加判据：报「账本闭环重锚 done」须同附 (1) stage 坍缩反例可复现 (2) raw witness 仍在 (3) trace 级 invariant（非单步）三件 witness。反 workaround 判准：判路径非法须该路径领域语义本就不该发生（earning 后未闭 legacy 腿违第87课），非为锁死而凑。 | spec-execution-gap 活教材（重锚=换 import vs 重锚=保住扩维结构）；谱系 #42/#86/#90/#93/OQ-9；memory `oq9-extend-dimension-resolution-pattern` |
| 0006 | 「已重锚/已验证/已对齐」——这是 **L 几**？证据若是 `lake build` 绿 / bit-exact / 合成数据 ⟹ L0/L1，**信息增量为零**（验管线非验假设），不准叫「已验证」。**有效域多大**？声称覆盖整个定义域（所有走势/标的/时段）须给**等大**验证；只测一标的一时段就标 L2-窄。这次重锚**有没有可能产出否定性结果**？结构上不可被真实数据否证 = L0 同义反复，谈不上验证。给不出 L2+ 就老实标 L0。 | **通用督导工具**（施加于任何蜂群「已验证/已对齐/已重锚」声明，特别 #89/#91→Origin 重锚的 `lake build` 绿、#102 theta_v0 bit-exact 重锚、theta_v0 回测 strat≈0% 分层诊断） | **可施加（物证已立）** — 规则 `formalization-validity-domain.md`（L0-L3 表+4禁止模式）+ 三判例谱系 222（范畴错误,gemini 异质抓）/223（不完备）/230（L3 否定性：独立性非无条件,控制$后恢复）。施加判据：报「已验证」须标 L 等级 + 声明有效域 + 说明是否可否证；只过 `lake build`=L0 不准称 L2。230号示范：否定性结果（缩小有效域）> 确认性结果。 | spec-execution-gap（声明=已验证 vs 实际=L0 管线对）；谱系 222/223/230/231 |
| 0007 | 「teammate 全面并行」——它真后台了吗？`Agent` 返回的是**立即 `agentId` handle**（`a…-…`）还是**阻塞到完整报告**？编排者能在任务板看到实时 `in_progress` 吗（`TaskList` 响应吗）？还是退化成**同步 subagent 批量**、Lead 被逐个阻塞、「并行」只是快速串行？给不出「立即 handle + 实时任务板」两证 = 声明膨胀。 | **通用督导工具**（施加于 Lead 自身任何「teammate/后台/全面并行 spawn」声明） | **可施加（物证已立，session 局部）** — 矛盾坐实：`Agent` 描述「`run_in_background`/`name`/`mode` unavailable — only synchronous subagents」**对撞** schema 内 `run_in_background` 属性定义。第三方物证打破对称：①SendMessage 描述预设 `agentId` 续聊 + `to:"main"`(background subagents only) ⟹ 平台**原语层支持后台**；②本 session deferred 回执「`TaskCreate/TaskList/TaskGet/TaskUpdate` MCP disconnected」(仅 `TaskStop`/`SendMessage` 加载) ⟹ **任务板此刻断连**。裁决：「能否用」非全局真假而是 session 级运行时事实——原语支持 / 当前 session 任务板不可观测。施加判据：Lead 报 teammate 并行须**实测**(立即 handle? + TaskList 响应?)，两皆真=成立；任一假=诚实退化标「同步 subagent 批量，编排者暂无实时板」，不嘴上 teammate 底下阻塞。 | spec-execution-gap 活教材（声明=teammate / 实际=阻塞 subagent）；规则 `lead-parallel-dispatch`(218号全并行,批量≠并行)；谱系同 0004a 082/073a |
| 0008 | 「严格 schema ≠ 完整忠实分类器」——A′ Origin 焊好了**元判准骨架**（纤维=行为商/四类穷尽/↔ 诚实降级），但缠论分类**血肉判据**（中枢/买卖点/线段/背驰「具体凭什么这样分」）是平凡桩或缺失。报「分类已 formalized」须先过四道**反退化见证**（见下「#113/#114 预置督导闸」），区分 L0 骨架 vs L1 真编码。 | **#113 分类血肉 / #114 策略组件**（正由 Origin 港入）；通用施加于任何「分类/判据/策略族 已 formalized/已实装」声明 | **可施加（预置闸，物证=审计台账）** — A′ Origin = 严格 schema 非完整忠实分类器：DONE（纤维=行为商↔双向/四类穷尽/#89 递归/#91 ↔ 诚实降级/闭环六段 π̄∘C+T/R=Π−A−W/TW-OQ9 bridged）已给反退化见证或为结构定理（有效域=定义域，免 L2）；gaps（#113 中枢三态[MISSING]·买卖点单射[degenerate]·线段特征序列法[`total_unique_of_fun` 桩]·背驰区间套[MISSING]；#114 π_Θ/C_Θ/RiskProj/VoiceTree[全 MISSING in Origin,仅 legacy Strict]；#88 partial[接口 done/实例化未 done]）。施加判据：报 #113/#114 任一 done 须附该 gap 对应反退化见证（见下四问），给不出=L0 骨架冒充忠实=222/223/230 第四次重演。 | spec-execution-gap 活教材（声明=分类忠实 / 实际=元判准 L0 骨架）；谱系 222/223/230/231（有效域≠定义域）+ #91（诚实降级范例）；**#113 / #114（追踪中）** |

| 0009 | 「已 formalized/已对齐/已重锚」——给我这份产出的**最弱那一环**：①还没做到的部分（L2 经验符合/真实数据）放在 `sorry`/裸 `axiom`/L0 平凡桩（=膨胀,藏缺口），还是显式 `structure` 字段/`Prop` 前件+标 L2+永不 discharge（=严格,缺口可见可否证）？②不变量是「流通」（入口携带+结构字段自动保持）还是「生成」（归纳真推导）——说错=夸大。③声明「绿」是**我此刻重跑**的全量 `lake build`/`cargo test`（owner 改过须重跑,#93），还是 commit message 陈旧快照？ | **通用督导工具**（施加于任何「已 formalized/已对齐/已重锚」声明）；本轮施加于四方向（判据血肉#113/116/117 · 深层判据+下钻#118/119/120/125 · rust 港入 G1/G2 · TW native 重锚#103/127） | **可施加（物证已立）+ 四方向本轮 PASS** — 范本 `ForceConformance.lean`(#131)：L2 缺口（rust MACD 是否合法 ForceMeasure 实例）显式留成 `structure` 字段+`Prop` 前件，**从不 discharge**，标 L2+留否证入口（违 mono/faithful 则不构成 witness）——「诚实标级 ＞ 假装满级」。施加判据：报「done」须给最弱环放法（显式前件 vs sorry/桩）+ 不变量流通/生成区分 + **督导亲自重跑**的全量绿，三缺一=膨胀。本轮四方向各给机器 witness 且无 L0 冒充 L2 = 真 done（详见下「本轮四方向裁决」）。 | spec-execution-gap 活教材（缺口诚实标 vs 桩堵）；谱系 222/223/230/231（有效域≠定义域）+ 0008（正面镜像：缺口正解=诚实标级非 L0 桩） |

> 注：#89 经 codex xhigh 78k 裁决为 A（可忠实焊接，非结构矛盾，pattern 已知），但仍**未实装**——督导裁决「FAIL（未完成）」成立，去风险≠已完成。

### 本轮四方向裁决（2026-06-26，并行全部最严格 4 方向）→ 全 PASS（机器 witness 核实）

> 督导亲自核实：全量 `lake build` = **90 jobs 绿**（非单文件 `lake env lean`，过 #93 真封闸）；`cargo test theta_v0` = **273 passed/0 failed**（当前工作树重跑，过 #93 owner-改动须重测闸）；Origin 全树零 `sorry`/`admit`/裸 `axiom`（grep 仅命中注释）；legacy `Tlayers.*` 零 live `import`（仅注释引用）。

| 方向 | 声明 | witness（file:line） | L 等级 | 裁决 |
|------|------|---------------------|--------|------|
| **判据血肉** #113/#116/#117 | Θ 缠论实例化首触 L1（真 case-split 缠论判据驱动 ledger，平凡占位做不到） | `ThetaInstantiation.lean:297-300`（`chanlunTransition` case-split on `recogChanlun` 计算分类非输入 enum）+ `:420-428`（`chanlun_transition_distinguishes_classes`：type1→A+1 vs type3→A+2，`omega` 非 `rfl`）；分类规则从 raw event 字段（brokeCenter/divPair/retracePrice）真算，非输入标签 | <span>L1</span>（缠论规则真编码） | **PASS** — 反退化见证给出（两事件产不同 delta），非裸 inductive/Bool 占位 |
| **深层判据+下钻** #118/#119/#120/#125 | 完整判据严格细化良构封装 + 真下钻 + 卖点对偶 + 端到端管线闭环 | `SegmentFeatureComplete.lean:206-285`（古怪线段 `peculiarStrokes` 两层分叉：committed `scanSegEnd` 真切 ∧ `SegEndComplete` 拒，`omega` 证 e3.high=15>e2.high=12）+ `Pipeline.lean:215-219`（管线各段输出真喂下段，`rfl` 类型对接）+ `:327-350`（R=Π−A−W **流通性**，`exact ledger.inv` 非归纳） | <span>L0/L1</span> | **PASS** — 注：闭环 inv 是「流通」非「生成」（结构字段自动携带），L0 结构定理（有效域=定义域），诚实标注无夸大 |
| **rust 港入** G1 卖侧/G2 下钻 | cargo 273 绿、买卖镜像 A+1↔A−1/A+2↔A−2、MACD 真算非占位 | `closed_loop/sell.rs:108-141`（`recog_chanlun_sell` 真三分支 + delta A−1/A−2）+ `:400-426`（`buy_sell_A_delta_mirror`/`buy_sell_closed_loop_A_mirror` 真 `assert_eq`）+ `classifier/descend.rs:102-107`（`descend` 真逆 lift 取 subs 非 passthrough）+ `:229-252`（级别严格递减 L2→L1→L0）+ `divergence.rs:43-100`（EMA/DIF/DEA/hist 真算）；28 `#[test]` 无 `#[ignore]`/无 `assert!(true)` | <span>L1</span>（MACD 真算，**实测 273**） | **PASS** — 督导亲自重跑 cargo 确认 273/0，非信 commit message 陈旧快照（#93） |
| **TW native 重锚** #103/#127 | TW 物理 port 进 Origin native，零 import legacy，#90 不同构 + OQ-9 全保 | `TotalWealth.lean:480-491`（`not_isomorphic_stage_collapses`：stage 字段真存活，两态 TW 三量同仅 stage 异）+ `:376-391`（`raw_earning_legacy_leg_closure_witness`：raw 违规腿存活在 twStep 层，未被入口守卫抹掉）+ `:343-363`（`oq9inv_trace`/`oq9_legacy_leg_unreachable_in_earning`：**trace 级** invariant 量化 `List TWEvent` 非单步）；零 `import`，`LedgerBridge.lean` 真降为 bridge 层 | <span>L0</span>（结构定理） | **PASS** — 0005 课三 witness（stage 坍缩/raw 腿/trace 级）重锚后**全部存活**，native 重写未塌缩扩维 |

> **本轮督导元结论**：四方向「过」不是因为「全做到了」，而是因为**没做到的那一层（L2 经验符合）被诚实标成显式假设**（范本 #131 `ForceConformance`：L2 前件永不 discharge + 标否证入口）。四方向 L0/L1 结构声明属实，L2 经验有效性仍悬空待 rust 对齐 + 真实数据。**「诚实标级 ＞ 假装满级」——缺口的正解是显式标 L2，不是 L0 桩堵。**

### #113 / #114 预置督导闸（lesson 0008 派生，Lead 在港入回传时逐 gap 施加）

> 用法：任何工位（含子蜂群）报「中枢/买卖点/线段/背驰/策略族 已 formalized/已实装」，Lead **先把对应那一道问题贴回**，工位自答+附 witness 才接受。给不出对应反退化见证 = 未完成（FAIL），不入 done，开/挂追踪缺口。三层识破法（反退化见证 / L 等级 / 边界条件）逐道内置。

| # | gap | 锋利问题（Lead-actionable，原文施加） | 通过判据（L 等级） |
|---|-----|----------------------------------------|--------------------|
| 1 | **中枢三态**（#113，MISSING） | 「给一个**具体缠论中枢**在『之上 / 之下 / 之内』态的例子 + 中心定理一/二（GG/G/D/DD 区间）的**机器见证**，不是 Bool 占位字段。态判定翻在哪个边界（混合中枢态最易漏）？」 | 须 ≥ <span>L1</span>（三态判定 + GG/G/D/DD 真编码）；只给 Bool 字段=L0 骨架=FAIL |
| 2 | **买卖点三类**（#113，degenerate） | 「给单射裁定的**可判定反例**（两个语义不同的点被判同类即证非单射）或一个第三类子域 + 完备性『**只有一二三类**』定理，不是裸 `inductive` 标签。」 | 须 ≥ <span>L1</span>（单射裁定 + 完备性定理，可 `by decide` 反例）；裸 inductive=L0=FAIL。范例：#91 的 `BSPLabels` 可判定反例（语义不同/判同类/trace 区分） |
| 3 | **线段（古怪线段）**（#113，`total_unique_of_fun` 桩） | 「给**一个古怪线段**被特征序列法（第67/71/78 课**包含关系 + 顶高于底硬约束**）处理的具体例子，不是 `total_unique_of_fun`（『函数即全且唯一』对线段一无所知）。哪种古怪形态会让你的划分翻？」 | 须 ≥ <span>L1</span>（特征序列法 + 顶高于底真编码）；`total_unique_of_fun`=L0 平凡=FAIL（0001 课已点名） |
| 4 | **π_Θ / C_Θ / fiber**（#114，全 MISSING in Origin） | 「给『**分类 ⊬ 唯一策略**』在 **Origin 抽象分类器接口**上的 witness（同一分类纤维 → 多个合法 π_Θ），不是把 legacy Strict 直接 re-export 冒充 Origin 实例。RiskProj / VoiceTree 同样要 Origin 接口上的实例，不是 legacy 别名。」 | 须 ≥ <span>L1</span>（Origin 接口上真实例化的 witness）；legacy re-export=接口居留非实例化=FAIL（同 #88 partial 形状） |

> 背驰 / 区间套（#113，MISSING 整块）：报 done 须附背驰判定 + 区间套收敛的机器见证，整块缺失时连 L0 桩都没有，默认 FAIL 直到见证出现。
> #88（partial）施加：接口居留（六段闭环签名）done 不等于组件实例化 done——报 #88 done 须附已实例化 `transition` 真改 ledger 的 witness（沿用 0004 闸）。

### 裁决记录（PASS）

- **#91（2026-06-26，cc-gap91-classlimits，方向B）→ RESOLVED-by-honest-downgrade**：过督导闸——给出可判定反例（`BSPLabels.x_2bOnly` vs `x_2b3b` 语义不同，canonical 分类器 `Strict.BSP.IGlobal` 判同类 type2，`chanlunTrace x := x.leftCenter` 区分二者行为，by decide）。**没硬凑「↔ 成立」，而是机器证明 ↔ 对缠论非平凡 trace 不可满足（9 定理，`Foundation/CompleteClassificationLimits.lean`）+ 把 `CompleteMinimalClassification` 降级出 canonical**。L0 + 谱系物证（615/231），零 L2。`lake build` 53 jobs 绿（15 def/theorem，#print axioms 仅 propext）。这是「诚实降级 > 虚假兑现」的范例：缺口的正确解可以是「证明它不可满足并诚实标注」，不是「假装满足」。补强：`both_directions_fail` 证 ↔ 双向皆假（缠论分类与行为**正交**），完整否定 lesson 0001「两方向都兑现?」=否。

### 督导对 Lead 生效（双向回路物证）

回路不只盘问工位，也盘问 Lead。已记录两次 Lead 声明被工位督导纠正：
- **「方向A 丢弃 4 个机器可检验实例化」**（cc-direction-decide 驳）：ChanlunInstantiation.lean 两版相同，A 不丢实例化；A 真实代价=重映射 + 丢 theta_v0 Lean-Rust 对齐。
- **「codex 版 BehEquiv 双重悬空（两 namespace）→ 当前仓库」**（cc-gap91 驳）：双重定义仅是 codex from-origin worktree 属性；当前主仓库只有一个 `def BehEquiv`（HybridStateMachine:32）。

两次都是「声明与实际不一致」被即时纠正（090号）。督导回路的价值在于它对**所有**蜂群产出生效，含 Lead 自身——这正是编排者「他相当于督导」的本意。

- **#89（2026-06-26，cc-gap89-recursion，方向B）→ PASS（窗口化递归忠实实装）**：lesson 0002 的锋利问题（「给我长度≥2 + 每上级≥3 不同 witness」）被**机器见证**回答——`xs9`（9 个不同 segment）→ `composeStep` 产 **3 个**上级（`xs9_multi_upper`，rfl）+ 每上级 3 个**不同**下级（`xs9_each_upper_three_distinct_subs`，injection+omega）+ `MovesComposedFrom` 全约束（3*3≤9）+ 真派生中枢（非 `canG=[]`）。从「单窗口 length≤1 聚合退化」升级为窗口化多上级序列，54 jobs 绿零 sorry。对比上轮 cc-foundation-faithful 的 FAIL（`[compose xs]` 单窗口聚合）——同一锋利问题，这次给出了见证。**督导闸把「去风险（裁决A）」与「已实装（机器见证）」分开，逼出了真实装。**

## 后续

每新增一课，在此追加一行；缺口入 TaskList 追踪。本回路的元模式（教学方法结晶 → 反向督导）可由 meta-observer 适时上升为蜂群常设结构。
