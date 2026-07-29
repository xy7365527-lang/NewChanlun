# Θ_parse 固定 617 pending ① —— 「617」所指核对与 HEAD 现状复测

- 票：#650（Part of map #530）
- 日期：2026-07-29
- 复测基线：`main @ 039cf86974`（2026-07-29 14:20 -0400）
- 产出分支：`research/theta-parse-617`（worktree `/private/tmp/research-650/wt`；主仓工作区零写入）

---

## 0. TL;DR

1. **「617」= 谱系条目 617**（`.chanlun/genealogy/settled/617-classification-itself-theta-parametric.md`，概念分离「连分类器 C_Θ 本身都是 Θ 参数化」，2026-06-27 结算）。不是计数、不是行号、不是 fixture 读数、不是 GitHub 票号。
2. **「pending ①」= 617 条目 `pending_verification` 五条待验义务（①–⑤）的第 ① 条**：fiber 原像划分只是「全函数 ⟹ partition」的弱抽象，不证标签语义/递归正确/因果，逐 claim 证明（605/615 各部分）仍负责给字段内容。①–⑤ 是条目内义务编号，不是 P0/P1/P2 价值序，也不是 harness 优先级档。
3. **HEAD 现状：该 pending 仍成立（未清零、亦无恶化）**。`formal/Strict/` 自 2026-06-25（`afc720f1ba`）至今零提交；HEAD `lake build` 145 jobs 绿、Strict 零 sorry/admit/axiom；但无任何新增的标签语义/递归正确/因果无前视逐 claim 定理，兜底责任方 615 仍为生成态。「617 pending ① 清零」作为 map #530 Destination 目标仍未达成。

---

## 1. 雾项原文与出处锚

- **#59 body**（`gh issue view 59`）：Not yet specified 区「~~Lean 残余（Θ_parse 固定 617 pending ①；E5 杠杆完全分类缺失）~~（雾挪走 2026-07-28（编排者裁定 A）：独立成图 = map #530…）」。经 `userContentEdits` 核对，**该行自 #59 创建（2026-07-21T02:56:27Z）起即在 body 中**（最早编辑版本已含「617」与该雾行）。
- **#530 body**（`gh issue view 530`）：Destination「Θ_parse 固定 617 pending ① 清零、E5 杠杆完全分类补齐」；Notes「自 map #59 雾挪出（2026-07-28，编排者裁定）：原雾项『Lean 残余（Θ_parse 固定 617 pending ①；E5 杠杆完全分类缺失）』」。#530 无评论；#59 全部 6 条评论均不含 Θ_parse/617，雾项无更上游票据锚（票据检索仅 #59/#530/#650 提及）。
- E5 半句 = #651 范围，本报告不涉。

## 2. 「617」的确切所指：谱系条目 617

证据链（四条互证）：

1. **直接引用绑定**：`docs/formal/full-definition-strategy-v1.md:80`（2026-06-26 蓝图）：「L0；**Θ_parse 是 R（617）**，#84 引擎补强此槽」——把 Θ_parse 与「617」作为谱系引用直接绑定（R = 231 号有效域规则中 extra-缠论设计选择的标级，见同文件 :101、:133-134「615/616/617 谱系」「231（有效域…）」）。
2. **谱系条目本体**：`.chanlun/genealogy/settled/617-classification-itself-theta-parametric.md`，title（:9）：「连分类器 C_Θ 本身都是 Θ 参数化：缠论结构公理单独连唯一分类都给不出——**需要 Θ_parse（边界/canonical 选择器）固定『在哪里切、相同极值取谁、开闭边界、未完成尾部』，才得到固定分类器 C_Θ**……」。雾文本「Θ_parse 固定 617」与该标题逐字对应。条目 status = 已结算（:4，2026-06-27 codex 异质委托结算，最终结算待 /ritual）；`dag.yaml:3150-3157` 同步登记 id '617' / status 已结算。
3. **谱系链位置**：617 depends_on 616 ← 615（`dag.yaml:6792-6797`；615→616→617 链见 617 文件 :120「谱系链：615 → 616 → 617」）。616（`settled/616-...md`）=「完全分类 ⊬ 唯一策略（π_Θ 族）」；617 把 616 的「π 需 Θ」升级为「C 与 π 都需 Θ」。
4. **排除项**：
   - formal/ 树内无「617」字面（开工侦察已核）；`formal/Strict/Parse.lean` 542 行，617 非行号。
   - `chanlun/review-results/` 与 `docs/` 下「617」命中均为 jsonl 数据、上述策略文档、或 **GitHub 票号碰撞**（如 `issue617-batch-confirmation-20260728.md` ↔ issue #617「批量确认与 pending 槽模型失配」——该票 2026-07-29 才创建，晚于雾文本 2026-07-21，且内容与 Θ_parse/Lean 无关，**碰撞排除**）。
   - 仓内（docs/.chanlun/chanlun/formal 全树 grep）不存在「Θ_parse 固定」原文短语——雾文本是地图作者对谱系 617 的压缩引用，非逐字转录。

**结论**：雾项语义 = 「谱系 617 号揭示：要得到固定分类器 C_Θ 必须先固定 Θ_parse；该条目登记的 pending 第 ① 条义务尚未清偿」。

## 3. 「pending ①」的分档语义

- 617 条目 frontmatter `pending_verification` 字段（`settled/617-...md:30`）登记**五条编号义务 ①–⑤**。同格式见 616（`settled/616-...md:30`，①–⑤）、615（`pending/615-...md:37`，①–③）——这是谱系条目「本概念分离留下哪些证明债」的清单序号，**不是** #59 的 P0a/P0b/P1/P2 形式化价值序，也不是 harness 模型/优先级档位。
- **第 ① 条原文**（617 文件 :30）：
  > ①原像划分是『全函数⟹fiber partition』的弱抽象——**不证**标签语义/递归正确/因果，逐 claim 证明（605/615 各部分）仍负责给字段内容。
- 语义展开：`fiber_total`/`fiber_disjoint`/`fiberSetoid`/`theta_fiber_partition` 证的是「任何全函数 C_Θ 的 fiber 自动构成互斥穷尽划分」——纯类型论结构事实；C_Θ 各标签（如 R6 态 {⊥,I,U⁰,U¹,D⁰,D¹}、BSPLabels）的**缠论语义**、**递归正确性**、**因果无前视**不在其内，须由 605/615 的逐 claim 证明兜底（617 文件 :129 认识论诚实节同述，并立 `StructurePartitionOnly` 标签防声明膨胀）。
- ①–⑤ 全目（备查）：② Θ_parse 全包当前只形式化 gaugeFix「level 最小、平级最左」，包含处理/相同极值取舍/开闭边界/未完成尾部仍待实例化；③ R6态/E bit-vector 依赖「最后中枢」「3买/3卖事件」定义，其 Θ 依赖待显式化；④ Θ_parse 最小充分集 = 开放问题（继承 616 ④）；⑤ 盈利/最优属 L3 不声称（继承 616 ⑤）。
- 与 #530 Notes「617 与 E5 是否属于 P1/P2 的组成部分」的关系：pending ① 指向的是**谱系侧 605/615 的逐 claim 证明债**（615 Layer2 六部分标准），与 #59 留雾的 P1 两族判同 / P2 测量守恒是不同清单；归并裁定属 #652（grilling），本报告不越界。

## 4. HEAD 现状复测（`039cf86974`）

复测口径与读数（全部在 worktree 内执行，主仓零写入）：

- **(a) 变更面**：`git log --follow -- formal/Strict/ClassificationFamily.lean` = 仅 `afc720f1ba`（2026-06-25 21:41 -0400，「L0 真完全分类形式化 Strict 内核库（41 jobs 绿，无 sorry/admit/axiom）」）；`git log --since=2026-06-27 -- formal/Strict/` = **空**。⟹ 617 结算（2026-06-27）后 Strict 内核零提交，pending ① 登记时的债面无后续清偿动作。
- **(b) 定理面**（HEAD `ClassificationFamily.lean`，8 定理全目）：`classifier_total_unique`(:100)、`fiber_total`(:135)、`fiber_disjoint`(:146)、`theta_fiber_partition`(:202)、`fibers_cover_image`(:220)、`realized_on_image`(:249)、`parse_unique_given_theta`(:298)、`family_not_true_classification`(:356)——全部属「给定 θ 的全函数 ⟹ 唯一 + partition」结构层，**无新增**标签语义/递归正确/因果无前视逐 claim 定理。旁证：`Chain.lean:351,367-369` 自述「复用 `theta_fiber_partition`，不重证 partition……partition 是纯类型论结构事实」并继受 `StructurePartitionOnly` 标签（:390）；`LevelState.lean` 虽有 R6 标签的谓词刻画与互斥穷尽定理（`rlevelOf_eq_*` :199-253、`rlevel_predicates_total` :294、`rlevel_pairwise_disjoint` :314、`rlevel_exhaustive_exclusive` :342、`position_partition` :512），但同属 `afc720f1ba` 原始提交，617 结算时已在案——pending ① 正是**在已知这些定理的状态下**登记的，债面表述至今不变。
- **(c) 编译面**：worktree（`research/theta-parse-617` @ `039cf86974`，`.lake` 构建缓存自主仓拷贝）`lake build` = **Build completed successfully (145 jobs)，EXIT=0**；`#print axioms` 输出仅 `propext`/`Quot.sound` 标准公理（如 `Origin/MainTheorem.lean:528` `main_sec16_L0_assumptions_discharged` depends on [propext, Quot.sound]）；唯一 warning = `Origin/SelfSimilarity.lean:414` unusedVariables linter（非错误）。`grep -rn 'sorry\|admit\|axiom' formal/Strict/` = 零真实命中（Tlayers 命中均为注释/文档行）。⟹ 「零 sorry 绿」在 HEAD 仍成立。
- **(d) 谱系面**：pending ① 点名的兜底责任方——605（`settled/605-...md`，已结算 2026-06-27，其定理在 Strict/Op.lean 等在案）与 615（`pending/615-mu-f-layer1-subset-chanlun-strict-classification-layer2.md`，status **生成态**，其 Layer2 六部分标准之 ②Eval健全+∼ₙ截面唯一 / ③δ无前视 / ⑤未完成→分支集 / ⑥π完全应对 未结算）——**615 未结，逐 claim 字段内容之债仍开**。

**复测结论**：「617 pending ①」读数在当前 HEAD **仍成立**——所述证明债未清零（Strict 零变更 + 615 仍生成态），且基线健康度无恶化（145 jobs 绿、零 sorry）。「清零」作为 map #530 Destination 目标仍未达成；清偿路径 = 615 Layer2 各逐 claim 证明（标签语义/递归正确/因果无前视）落地并回填 C_Θ 字段内容。

## 5. 边界与未能判定项

- 「617」所指之判定依靠仓内文档/谱系互证（§2 四条证据链），无编排者逐字确认件；若编排者另有口径（例如把「①」读作 P1），归并属 #652 裁定范畴。
- pending ① 的「清零」验收线（哪些逐 claim 定理算清偿充分）在任何票中均未具体化——本票只核所指与现状，不定义清零判据（属 map #530 开工范畴）。
- 未能判定项：无。三问均可答（所指 = 谱系 617 条目；① = 其 pending_verification 第 ① 条；现状 = 仍成立，复测口径与读数见 §4）。

## 6. 复测环境记录

- worktree：`/private/tmp/research-650/wt`，branch `research/theta-parse-617`，base `main @ 039cf86974`。
- 构建：`lake`/`elan` = /opt/homebrew/bin；`formal/.lake`（82M，gitignored）自主仓只读拷贝后在 worktree 内构建，主仓工作区与 git 状态零触碰。
- 票据证据：`gh issue view 59/530/650/617` + `gh api graphql userContentEdits`（#59 共 30+ 次编辑，最早 2026-07-21T02:56:27Z 版本已含雾行）。
