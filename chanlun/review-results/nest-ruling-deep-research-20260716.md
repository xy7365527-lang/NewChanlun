# 区间套证书机器五裁定点建议书（#92 重放前）

- 日期：2026-07-16
- 范围：只读调研；未运行 `cargo build/test`，未修改代码、任务状态或 git 状态。本文件是唯一产出。
- 规范锚：`docs/formal-chain/proofs-full-strategy-20260703.md:75-83,235-243`。
- 原文权威：`docs/chanlun/text/blog/`。任务给出的 `chanlun/definitions/` 与 `chanlun/text/` 在本树不存在；实际速查定义位于 `.chanlun/definitions/`，原文位于 `docs/chanlun/text/`。以下原文判断以博客正文为准，定义文件只作交叉核验。
- 预注册先验：BTC 全历史 95% 的区间套深度为 1，约 92% 属“小转大”，深度≥2 几乎为 0；这允许忠实机器产零，但不允许以“追求非零”反推定义（`docs/formal-chain/proofs-full-strategy-20260703.md:75-83,235-243`）。
- 排除项：确认窗口径，以及“子级 A 腿塞进父级 C 离开 episode”，已经被判为符号对象错配；本文不复活（`chanlun/review-results/dr-l1l2-zero-genealogy-20260711.md:9-13,118-139`）。

## 1. TL;DR 裁定建议表

| 裁定点 | 建议 | 置信度 | 一句话理由 |
|---|---|---|---|
| ① `Cand^δ_ℓ` 定义式 | **A\*（条件采纳）**：选择 A 而非 B/C，但“严格 C2 pair”必须承载 `dir ∧ Comparable ∧ Extreme`；P83 的 raw 相邻 Completed pair 不够 | 中 | 现行严格订正版把 Cand 定为宽结构候选、明确不含 Weak；B 把力度命中提前塞回 Cand，C 又绑定已局部 supersede 且零产的旧对象 |
| ② `J^δ_ℓ` 区间端点 | **B：完整背驰段 `c` 的结构区间**；A 仅作并行结构诊断，未证明无假阴性前不得作硬过滤 | 高 | 第27课逐字嵌套“不同级别背驰段”，不是 leave→retest 全跨度；157 是另一几何口径的计数，不是已证明的 B 上界 |
| ③ 基例 `Conf^δ_e` | **维持 `BspBits::confirm_side`**；D5 CompletedFreeze 只作完成/持久性见证 | 高 | `Conf^±` 是三类买/卖点析取，保持 `Λ≠∅` 且不要求唯一；D5 没有六 bit 或方向化买卖点语义 |
| ④ 时序递降时间戳 | **首次因果可证钟 + snapshot**；现阶段仅作正交时序 sidecar，不新增为 `N^δ` 硬门 | 中 | 原文与递归式均未给 `parent.confirm≤child.confirm`；若另裁必须加门，唯一诚实的钟是所选 Cand 全部证据首次可见的 `judge_at`，不是图形端点或终点冻结写入时刻 |
| ⑤ 背驰类型域 | **趋势背驰与盘整背驰都入链，但保持 typed 分流**；盘背不得冒充同级 B1/S1 | 高（教义）/中（C2 映射） | 第27课先把二者都定义为“背驰段”，并以盘背大级别案例讲区间套；代码已能区分证书，但当前 C2 provider 与旧 strict chain 仍排除盘整 |

## 2. 论证

### 2.1 ② `J^δ_ℓ` 定位区间端点（深挖）

#### 原文依据

第27课先定义：“在某级别的某类型走势，如果构成背驰或盘整背驰，就把这段走势类型称为某级别的背驰段”（`docs/chanlun/text/blog/027-第27课.md:22`）。随后以季度→月线为例，明确说月线背驰段“在季度线的背驰段里，而且区间比之小”（同文 `:32-40`），最后给出定理：“不同级别背驰段的逐级收缩范围”，并要求在当前背驰段内继续找次级别背驰段（同文 `:42-46`）。因此 `J^δ_ℓ` 的文本对象是该级构成背驰的走势类型区间，不是确认窗，也不是确认后的回试窗口。

第29课把趋势场景的对象称作最后中枢之后的“最后的背驰段”（`docs/chanlun/text/blog/029-第29课.md:30`）；第37课进一步要求在 `c` 内部递归分析。既有谱系据此冻结：父区间从产生该级转折的完整 `c_p` 走势类型结构起点开始，到其结构端点结束；排除父级 A 起点、整个趋势起点、确认点与局部 reentry episode（`chanlun/review-results/dr-l1l2-zero-genealogy-20260711.md:9-13,88-116`）。

结论是 B：

```text
J^δ_ℓ = D_ℓ = [start(c_ℓ), end(c_ℓ)]
```

这里的 `c_ℓ` 必须有“最后中枢—离开—产生该级转折”的结构身份。不能仅凭字段名字把 P83 pair 的第一项 `leave` 自动等同于完整 `c_ℓ`；该映射需由 C2 `DivergencePair.seg_c`、CompletedMove 身份及其归属证书逐项证明。找到了 `seg_c` 坐标，不自动等于找到了完整走势类型 `c` 的起止身份。

#### 工程实证

P83 的 A 口径定义非常清楚：任意两个相邻且均 Completed 的 Move 形成 strict pair，区间取 `[pair[0].start_index, pair[1].end_index]`（`rust/src/bin/p83_yield_remeasure.rs:393-417`）；跨级按闭包含做动态规划（同文件 `:461-521`）。全量终点快照得到 2,289 CompletedMove、1,092 strict pair 与 157 条 L1–L5 结构链（`chanlun/review-results/c2-yield-remeasure-20260715.md:10-20`）。该报告也明确：157 只回答结构闭包含，不是 `Chi`、终端 BSP 或交易信号（同文 `:49-62,182-188`）。

当前 C2 背驰 provider 的 `DivergencePair` 另存 `seg_a/seg_c`（`rust/src/theta_v0/classifier/level_view.rs:384-398`）；它只从有方向的 Trend block 生成，Consolidation `dir=None` 被跳过（同文件 `:400-415`）。所以“P83 strict pair”和“背驰段对象”是两个不同集合：前者是相邻完成结构，后者还需 A/C 归属与背驰段身份。157 证明新域有跨级结构几何，不证明 B 已落地。

#### A/B 双口径能否成立

可以并行保留，但职责必须收窄：

- B 是裁定判据：用完整 `c` 区间做 `J_child⊆J_parent`。
- A 是诊断口径：回答“相邻 Completed pair 全跨度是否存在闭包含结构”，用于对象覆盖、性能估算与差分解释。
- A 暂不能作生产硬过滤。要成为安全预筛，必须先证明对每条 B 边都有 `B_edge ⇒ A_edge`，即不会筛掉 B 真链。

不能从“同一节点 A 比 B 更宽”推出链数单调。父子两端都会同时变宽：即使 `B_child.end≤B_parent.end`，也可能出现 `A_child.end>A_parent.end`；反向也可能发生。因此 A 链与 B 链一般既非子集也非超集。报告中若把 157 称为 B 的“真上界”，属于未经证明的结论。

#### 反方论证

A 的最强理由是工程上稳定且已非零；把 retest 纳入还能表达“结构得到后续确认”。但这把“对象区间”和“对象何时可知”混成一个字段：retest 更适合作④的可用时点证据，不应右扩第27课所说的背驰段。

B 的最强反方则是：完整 `c` 的机器身份目前并未在所有 C2 对象上闭合，贸然替换会再次归零。对此应接受 fail-closed：缺身份就是 Pending/Unassigned，而不是把 A 重新命名成 B。定义忠实优先于产量。

#### 建议、零产量解释与可逆性

建议选 **B**，置信度高。只有在以下条件同时满足后，B 重放得到 0，才可说“0 是正确答案”：完整 `c` 身份已覆盖目标域；⑤的趋势/盘整 typed provider 均已纳入；① Cand 与③ Conf 按裁定执行；snapshot 因果域没有终态回填。若任一 provider 尚缺，0 只能说明能力缺口。

“157 只是结构上界”应改写为：**157 是 A 口径的结构可行性基线/替代几何计数，不是 B 口径的数学上界**。该修辞变化不否定 157 的工程价值，但禁止用它反压 B 必须非零。

裁错可逆性：中。A/B 可以版本化并行重放，判据切换容易；但一旦 A 被写入生产证书身份或交易解释，回收历史语义成本较高，因此先 shadow、后裁切。

### 2.2 ⑤ 背驰类型域（深挖）

#### 原文依据

第27课在同一条“背驰段”定义中并列趋势背驰与盘整背驰（`docs/chanlun/text/blog/027-第27课.md:14-22`）。随后用大级别盘整背驰寻找历史底部，并明确说这是区间套方法（同文 `:24-40`）；其定理没有把“背驰段”再缩窄为 trend-only（同文 `:42-46`）。该课还给出两类用法差异：多数第一类买点由趋势背驰构成，多数第二、三类买点由盘整背驰构成（同文 `:18-20`）；大级别盘背可形成“类似第一类”的买点，但不是把它改名为标准同级第一类（同文 `:60-68`）。

第31课答疑更直接：只有一个中枢的转折既有 A/C 盘整背驰，又有 C 内部次级别背驰，“两者有着类似区间套的关系”（`docs/chanlun/text/blog/031-第31课.md:652-656`）。第54课说明，本级别结束常由第二个同向段相对第一个同向段的盘背当下判断，这正是区间套能当下的原因之一（`docs/chanlun/text/blog/054-第54课.md:222-224`）。速查定义也保持两类背驰段与逐级收缩（`.chanlun/definitions/beichi.md:107-135,364-382`）。

因此原文支持“盘整背驰入区间套定位域”，但不支持“盘背与趋势背驰产生相同买卖点类型/同样的转折保证”。链域与信号解释域必须分层。

#### 工程实证

代码已经有 typed 区分：`AbcDivergence.is_trend` 区分趋势/盘整，趋势 A/C 跨相邻中枢，盘整 A/C 属同一中枢（`rust/src/theta_v0/classifier/divergence.rs:20-27,631-650`）。`judge_pan_div` 生成 `PanDivCert`，显式保存方向化 `side`、A/C 区间，并在 C<A 后输出（`rust/src/theta_v0/classifier/signal.rs:567-646`）。这说明盘整块自身 `MoveBlock.dir=None` 并不等于区间套无法得到 δ；δ 可来自盘背证书的 Long/Short side。

但当前两条生产路径仍排除盘整：

1. C2 `provide_divergence_pairs` 遇 `MoveBlock.dir=None` 直接跳过（`rust/src/theta_v0/classifier/level_view.rs:400-415`）。P83 的 1,615 个 Consolidation CompletedMove 因而不在现有 pair provider 的背驰对域中（`chanlun/review-results/c2-yield-remeasure-20260715.md:14-18`）。
2. 旧 `nest.rs` 把 Cand 绑定 `cand_delta`，注释明确 `pan_div_diag` 不入谓词（`rust/src/theta_v0/classifier/nest.rs:380-390`）；现行 P0 裁定也要求盘背继续不入 strict chain，除非另案显式 supersede（`chanlun/escalate/p0-cand-delta-naming-dualtime-ruling-20260712.md:58-61`）。

所以“盘背已在 divergence.rs 区分实现”只证明上游判定能力存在，不证明它已进入 C2/N 链。

#### 反方论证

trend-only 的强论点是：标准第一类买卖点只由趋势背驰产生，且现行裁定明确排除盘背；贸然并入会把稳定语义重新混成一个 bool。该担忧成立，但它约束的是输出类型，不足以推导盘背不能参与转折定位。第27课本身就同时说“盘背属于背驰段”与“第一类肯定由趋势背驰构成”，说明原文要求的是 typed union，不是二选一。

另一个反方是产量先验：深度≥2 本来极少，增加盘背会人为扩域。回应是：⑤的目标不是增产，而是恢复原文定义域；新增样本必须分桶报告，不能把 trend-only 与 union 的结果混成同一统计。

#### 建议与可逆性

建议 **盘整背驰入链，但以独立 `PanDivContext/PanDivCert` 变体进入**：

- 区间 `J` 仍取该盘整背驰的 C/背驰段结构区间；
- δ 取 `PanDivCert.side`；
- 终端 `Conf` 仍按 B1/B2/B3 或 S1/S2/S3 实际 bit，不把盘背强置为同级 B1/S1；
- 报告至少分 `TrendNest`、`PanNest` 与混合链，禁止合并后宣称“趋势区间套产量”。

教义置信度高，C2 映射置信度中。裁错可逆性中高：typed additive provider 可 shadow 和关闭；但它显式 supersede `p0-cand-delta-naming-dualtime-ruling-20260712.md:R6` 的 strict-chain 边界，必须走人裁，不能以实现细节悄悄放开。

### 2.3 ① `Cand^δ_ℓ` 定义式（快评）

#### 原文依据

原文只给出“背驰段”与逐级收缩，没有独立命名 `Cand`，因此无法从第27课逐字推出 A/B/C。正式 PDF 提取也明确列为“Cand 未给独立定义式，仅作符号出现”（`.chanlun/specs/2026-06-28-recursive-complete-classification-bsp-pdf-extract.md:1254-1258`）。此处不能伪装成原文已有唯一答案。

项目后续严格订正版把 Cand 定为：

```text
StructEligible^δ = dir(s)=-δ ∧ Comparable_ℓ(s',s) ∧ Extreme^δ_ℓ(s',s)
```

并明确 Weak/力度弱化不进 Cand，而进入状态向量或 selector（`.chanlun/review-results/formal-criteria-20260705.md:60-70`）；另一份核验也强调 `Cand≠Conf`、Cand 是因果可见外层候选（`.chanlun/review-results/dlpdf-e-gap-verification-20260702.md:24-27`）。

#### 工程实证

A 的 P83 raw pair 只是 `view.moves.windows(2)` 中两个 Completed Move（`rust/src/bin/p83_yield_remeasure.rs:393-417`），没有验证方向、同一可比结构语境或创新高/低。因此 raw A 不能直接等同 Cand。

B 会在 raw pair 上再加背驰力度谓词，但仍可能缺 Comparable/Extreme，而且把严格订正版刻意移出 Cand 的 Weak 再塞回门内。代码历史也显示这种张力：`cand_predicate.rs` 当前仍含 Weak，而 `signal.rs` 已把 C<A 从结构候选 gate 降为 buy1 判据/feature（`rust/src/theta_v0/classifier/signal.rs:321-335`）。

C 更不宜继续：旧 `cand_delta` 已被裁为“算法背驰确认事件”，不等价完整趋势 `c` 资格（`chanlun/escalate/p0-cand-delta-naming-dualtime-ruling-20260712.md:11-21`），且与新的 C2 对象不同域。

#### 反方论证与建议

B 的反方论证很强：`.chanlun/definitions/beichi.md:373-377` 写每一级背驰应独立成立；若 N 被定义为“最终背驰证书”而非“宽定位候选”，B 更直观。与此同时，`docs/formal-chain/proofs-full-strategy-20260703.md:271-274` 又保留“Cand 力度门”措辞，与严格订正版存在文档内张力。故本点不能给虚假高置信度。

三项中建议 **A\***：选择 A 的“结构候选先行”分层，但必须把 strict pair 升级为 `StructEligible` carrier；raw adjacency 只是一项前置材料。若裁定只能接受三个选项的逐字含义，则答案是“**A/B/C 均不完整，A 最接近**”。建议人裁同时明确 `formal-criteria:60-70` 是否 supersede `proofs:D-1` 的力度门措辞。

置信度中；裁错可逆性高。A\*/B/C 三路可作为同一对象集上的 sidecar bit 并行统计，在进入证书身份前切换成本低。

### 2.4 ③ 基例 `Conf^δ_e`（快评）

#### 原文/规范依据

形式规范明确：`Conf^+_e=∨_{i=1}^3 B_{i,e}`、`Conf^-_e=∨_{i=1}^3 S_{i,e}`（`.chanlun/specs/2026-06-28-recursive-complete-classification-bsp-pdf-extract.md:277-285`）。这就是 `Λ≠∅`：至少一类同方向买卖点成立，不要求唯一，也不要求三类互斥。

#### 工程实证

`BspBits` 保存六个独立 bit；`conf_plus/conf_minus/confirm_side` 正是方向化析取（`rust/src/theta_v0/types.rs:167-220`）。`nest.rs` 的基例直接调用 `terminal.confirm_side(side)`（`rust/src/theta_v0/classifier/nest.rs:293-335`）。

D5 `FrozenCompletedMove` 只有走势身份、`judge_at`、端点、方向、类型和中心索引；`CompletedFreezeEvent` 只再加 sequence/created_at/cache key（`rust/src/theta_v0/classifier/level_view_store.rs:18-58`）。它没有 B1/B2/B3/S1/S2/S3，无法表达 `Λ`。P83 又是全量终点快照观察，明确不提供每个对象历史上的首次完成时点（`chanlun/review-results/c2-yield-remeasure-20260715.md:182-188`）。

#### 反方论证与建议

改挂 D5 的理由是 CompletedFreeze 更稳定、产量非零。但“走势已冻结”只回答可用对象是否完成，不回答执行级别是否出现任何买卖点；替换会把 `Conf` 偷换成 completion。

建议维持 `BspBits::confirm_side`。D5 可以与它正交合取，作对象完成性或版本稳定性见证，不能替换。`Λ≠∅` 语义完整保持。置信度高；裁错可逆性高（新增 D5 sidecar 易撤），但若历史上把 freeze 重写成 Conf，语义清洗成本高。

### 2.5 ④ 时序递降时间戳（快评）

#### 原文/规范依据

**未找到**第27课或相邻课文要求 `parent.confirm≤child.confirm`。原文说大级别背驰成立后可继续向下分析（`docs/chanlun/text/blog/027-第27课.md:30-40`），但这不足以推出跨级确认时点的硬偏序。正式递归式也只有 Cand、区间包含与子证书，没有时间合取项（`docs/formal-chain/proofs-full-strategy-20260703.md:75-83`）。

#### 工程实证

`NestRung.confirm_src` 与 `interval.end_time` 已被设计成独立字段（`rust/src/theta_v0/classifier/nest.rs:145-160`）；现行装配明确写“确认时点不参与排序或闸门”，实际过滤只有 Cand、方向与区间包含（同文件 `:551-596`）。模糊入口已 deprecated，要求显式 snapshot/terminal（同文件 `:462-506`）。现有 P0 裁定则禁止终态事实回填历史快照（`chanlun/escalate/p0-cand-delta-naming-dualtime-ruling-20260712.md:23-28,52-61`）。

三个钟的判断：

- `freeze/created_at`：不能直接用。D5 在 `observe(as_of)` 时把 `judge_at` 与 `created_at` 都写成查询 as_of（`rust/src/theta_v0/classifier/level_view_store.rs:35-48,307-327`）；在 P83 的 history-end 单次观察中，它是终点观察钟，不是对象首次可知钟。
- `retest.end`：是结构坐标；只有在因果 prefix 证明“所选 Cand 的最后必要证据恰在该 retest 完成时首次可见”时，才能与语义钟数值相等，不能先验同义。
- “背驰确认时点”：三选一中最接近，但应更严格命名为 `cand_judge_at`，即①所选 Cand 的全部必要证据首次共同可见的时点。若①选 A\*，它可能是结构候选完成时，而非 Weak 命中时；若人裁改选 B，才是背驰谓词首次可证时点。

#### 反方论证与建议

把 `parent.confirm≤child.confirm` 作为硬门可防未来证据倒灌，也曾出现在早期漏斗中；但 causality 应由“固定 as_of 只见 judge_at≤as_of”和下一 bar 消费保证，不应在无原文/规范依据时额外改变 N 的数学定义。现行代码把该序仅作诊断，是有意裁定，不是漏实现。

建议：#92 的 canonical 选择 **snapshot**；terminal 只作另列的 ex-post 形态审计，禁止回填。当前不把确认偏序加入 `N^δ`。若人裁另案要求外部时序门，使用 `cand_judge_at`，并由 causal prefix replay 取首次出现时点；不使用 history-end freeze，也不把 `retest.end` 无条件当钟。

置信度中；裁错可逆性高，因为钟可保留为 sidecar 并 shadow 对账，尚不必写进证书真值。

## 3. #92 重放前的只读证伪探针清单

所有探针均为只读/COUNTERFACTUAL_ONLY；不改生产路径，不运行 `cargo build/test`，不把探针输出回写历史事件。

1. **B 对象身份覆盖探针**：逐级列出可证明完整 `c` 身份的对象数，至少带 `{level, side, parent-center-id, c-move-id, c.start, c.end, available_at}`；任何只靠 `leave` 名字、`seg_c` 局部 episode 或 confirm window 得到的对象单列失败桶。证伪条件：无法把大多数拟议 B 候选映射到完整 `c`。
2. **A/B 四象限差分**：同一候选节点分别算 A=`[leave.start,retest.end]` 与 B=`[c.start,c.end]` 的每条跨级边，输出 `A∧B / A∧¬B / ¬A∧B / ¬A∧¬B`。若出现 `¬A∧B`，A 立即被证伪为安全硬过滤；即使样本中未出现，也仍需性质证明 `B_edge⇒A_edge` 才能进生产预筛。
3. **Cand 三路 shadow**：在同一 C2 身份集上分别统计 raw A、A\*=`dir∧Comparable∧Extreme`、B=`A\*∧Weak_Θ`，按 level/side/trend-pan 分桶；禁止把 raw 1,092 当 A\*。重点观察 A\*→B 的损失，而不是用非零率选定义。
4. **⑤ typed provider 覆盖**：分别输出 TrendDiv、PanDiv、mixed-chain；盘背路线必须携 `PanDivCert.side` 与自己的 `seg_c/c-move`，并验证没有强置 B1/S1。若 1,615 Consolidation 对象仍全部无可判 PanDiv，应标 provider 缺口，不能报“市场盘背为 0”。
5. **③终端析取探针**：对每个 B 链终端输出六 bit、`confirm_side` 与 D5 状态；验证 `confirm_side=true ⇔ 同方向三 bit 至少一个为真`。同时报告“D5 completed 但 Λ 为空”的数量，直接展示二者不可替换。
6. **④三钟因果差分**：在 raw-prefix 上记录候选首次出现的 `cand_judge_at`、结构 `retest.end`、D5 首次 freeze `judge_at`，再对比 history-end 单次观察的 created_at。输出逆序样本，但不让任何钟参与 N 真值。若固定前缀追加未来事实会改变旧 `cand_judge_at`，snapshot 实现被证伪。
7. **零产量归因门**：若 B 最终为 0，必须同时报告对象覆盖率、A\*/B 谓词损失、Trend/Pan provider 覆盖、Conf 命中与时序可见性。只有所有前置能力齐全且首个归零发生在忠实的 `J_child⊆J_parent` 几何门，才可签“0 为市场几何”。
8. **统计先验核验**：分开报告深度=1 与深度≥2。不得用“深度≥2≈0”的先验替“深度=1 小转大”缺失开脱，也不得因 157 非零而放松 B。

## 4. 必须人裁的残余点

1. **R1—Cand 权威冲突**：确认 `.chanlun/review-results/formal-criteria-20260705.md:60-70` 的 `Cand=StructEligible、Weak 后置` 是否正式 supersede `docs/formal-chain/proofs-full-strategy-20260703.md:271-274` 的“Cand 力度门”措辞。未裁前建议 A\*，不能宣称唯一终局。
2. **R2—盘整背驰入链的 supersede**：⑤虽有强原文依据，但会越过 `p0-cand-delta-naming-dualtime-ruling-20260712.md:R6` 的现行 strict-chain 排除。必须明确裁为“链域扩展、信号类型不变”，不能静默改 bit。
3. **R3—B 的 C2 身份映射**：完整 `c` 应绑定哪个稳定 CompletedMove/版本、`seg_c` 是完整走势还是 episode、Consolidation 的盘背段如何映射，均需机器契约；不能以字段相似代替裁定。
4. **R4—A 的允许地位**：A 只做离线诊断，还是在证明 `B_edge⇒A_edge` 后允许作性能预筛。当前证据只支持前者；“157 是上界”不应获批。
5. **R5—确认偏序是否属于 N**：原文、PDF 递归与现行实现均没有该合取；若业务仍要求 `parent.confirm≤child.confirm`，须明示这是新增的因果/工程约束，并规定它在 N 外还是 N 内。
6. **R6—snapshot 的查询契约**：#92 是逐 prefix 首次可见重放，还是 history-end 终态形态普查。本文建议前者为 canonical、后者单列；两者不得共用一个“产量”数字。
7. **R7—零的签字条件**：人裁确认“provider 完整 + 定义忠实 + snapshot 无前视”三项均通过后，才允许把 B=0 写成正确市场答案；否则只能写能力边界或未决。

## 证据索引

| 主题 | 关键锚点 |
|---|---|
| N 递归、闭包含、95%/92%先验 | `docs/formal-chain/proofs-full-strategy-20260703.md:75-83,235-243` |
| 背驰段定义与区间套定理 | `docs/chanlun/text/blog/027-第27课.md:14-46` |
| 趋势/盘整的用途差异 | `docs/chanlun/text/blog/027-第27课.md:18-24,60-68` |
| 盘背与 C 内次级背驰类似区间套 | `docs/chanlun/text/blog/031-第31课.md:652-656` |
| 盘背使区间套可当下 | `docs/chanlun/text/blog/054-第54课.md:222-224` |
| 父 `c` 完整区间与错配谱系 | `chanlun/review-results/dr-l1l2-zero-genealogy-20260711.md:9-23,88-139` |
| P83 2,289/1,092/157 及测量边界 | `chanlun/review-results/c2-yield-remeasure-20260715.md:10-20,49-62,182-194` |
| raw strict pair 与链计数实现 | `rust/src/bin/p83_yield_remeasure.rs:393-417,461-521` |
| C2 `DivergencePair` 与 trend-only provider | `rust/src/theta_v0/classifier/level_view.rs:384-415` |
| Cand 严格订正版 | `.chanlun/review-results/formal-criteria-20260705.md:60-70` |
| PDF 中 Cand 定义缺失 | `.chanlun/specs/2026-06-28-recursive-complete-classification-bsp-pdf-extract.md:1254-1258` |
| 趋势/盘整证书区分 | `rust/src/theta_v0/classifier/divergence.rs:20-27,631-650`；`rust/src/theta_v0/classifier/signal.rs:567-646` |
| 旧 cand_delta 与盘背排除边界 | `rust/src/theta_v0/classifier/nest.rs:380-390`；`chanlun/escalate/p0-cand-delta-naming-dualtime-ruling-20260712.md:11-28,58-61` |
| Conf 与 Λ 非空 | `rust/src/theta_v0/types.rs:167-220`；`rust/src/theta_v0/classifier/nest.rs:100-109,293-335` |
| D5 字段与观察时点 | `rust/src/theta_v0/classifier/level_view_store.rs:18-58,272-329` |
| snapshot/terminal 与确认时点不作闸门 | `rust/src/theta_v0/classifier/nest.rs:145-160,462-506,551-596` |
