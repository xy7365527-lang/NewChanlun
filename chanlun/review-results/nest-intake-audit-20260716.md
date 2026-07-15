# 区间套 0 产量上游扔料点全链审计（goal 遗漏排查）

日期：2026-07-16 ｜ 关联：#91 裁定、#92 重放 ｜ 状态：审计进行中（codex 实施并行）

## 目标

Goal：修好进料口（盘整块允许进链）→ 放宽初筛（候选只看结构不看力度）→ 补齐身份标签 → 重放测产量；0 不可签为市场答案；找出**所有**遗漏的扔料点。

## 已确认的三个主扔料点（裁定已覆盖，codex #92 实施中）

1. **进料口**：`provide_divergence_pairs` 遇 `MoveBlock.dir=None` 跳过（`level_view.rs`，注释自认"盘整 None 严格产零 pair"）。1,615 个 Consolidation CompletedMove 整体出局。
   - 修复现状：codex 新增 `provide_nest_candidate_events`，Trend/Consolidation 双分支，盘整走 `locate_pan_div_structure` 纯结构定位，`kind: NestDivergenceKind::Consolidation` 分桶输出。✅方向正确
2. **初筛门**：旧 `nest.rs` 装配绑死 `cand_delta`，`pan_div_diag` 只做诊断位不入谓词（nest.rs:573-577,711,775-776）。
   - 修复现状：新路径以 `NestCandidateEvent` 为 Cand（纯结构 extreme 检查），`divergence_confirmed` 独立合取放在装配级（②口径），符合 D1"力度不进候选门"。旧 cand_delta 路径保留 = bit-exact 回归用，符合裁定。✅
3. **身份标签**：`CompletionStatus::Pending` 带 typed reason（AwaitingSubsequentMove / MissingDivergencePair / TerminalLegNotDivergent / MissingMacdCoordinates）。fail-closed 保留，但重放报告必须按 reason 分桶计数，否则归零不可归因。

## 新发现的遗漏候选（需在 #92 报告中逐条回应）

- **G1｜MACD 坐标缺失静默清零**：`divergence_confirmed` 计算中 `map_src_to_close_idx` 任一端失败即 `_ => false`。若 close_src 映射覆盖不全，所有候选静默变 unconfirmed 再次归零——这是 R7(a) provider 完整性的隐蔽变体。**要求：重放报告输出 map 失败率；若 >0 须列 MissingMacdCoordinates 桶计数。**
- **G2｜盘整分支的后继块要求**：`structural_pair_span` 要求 `leave_index+1` 的 retest 块也 Completed。盘整样本天然靠近数据尾部时会被批量过滤。**要求：报告"因无后继块被滤"的样本数，与 AwaitingSubsequentMove 桶对齐。**
- **G3｜盘整分支中心配对条件**：`nearest_confirmed_center_idx` + `kinds==Consolidation` + block 包含 center 三重查找，任一 miss 即 continue。**要求：分步计数，定位首个归零谓词（对应 deep-research 第188行"零产量归因门"）。**
- **G4｜recursive_tower `chain_is_trend` 硬门**（recursive_tower.rs:1647-1656）：属"完成态趋势分解资格"证据构造，趋势延续关系是定义本身，**不算扔料点**。已核验：新装配 `assemble_typed_certificate` 的终端确认走 `terminal_of` 闭包，按 `turn_source` + `bits.confirm_side(side)` 匹配 BspBits（p92 bin :738-761），与 kind 无关、不经 CompletedTrendDecomposition。**结构上闭合 ✅；经验命中率待重放分桶（Trend/Consolidation 各自的终端 BspBits 非零率，即 deep-research P-③ 探针）。**
- **G5｜重放 bin 路径确认**：已核验 `p92_nest_replay_postruling.rs:514,588` 走新 `provide_nest_candidate_events`，且 P92_RULE 头声明 `cand=dir_and_comparable_and_extreme ... d3=sidecar terminal=BspBits_confirm_side clock=first_prefix created_at=forbidden`，与 #91 裁定逐项对齐。**闭合 ✅**

## R7 签字门映射

| R7 条件 | 对应审计项 |
|---|---|
| (a) provider 完整 | 进料口①、G1、G2、G5 |
| (b) 定义忠实 | 初筛②、G3、G4 |
| (c) snapshot 无前视 | judge_at/provider_window 因果性（codex 实施④）。**预审已过**：p92 bin 三重 as_of 门（`interval_a.1 > as_of ∥ interval_b.1 > as_of ∥ turn_source > as_of` 即排除，:619），judge_at = first-prefix 首见钟（`book.candidates.or_insert(as_of)` 单调登记，:624-638），`created_at=forbidden` 入 P92_RULE 头。待测试绿 + 重放数字终验。 |

结论：三主点裁定已覆盖且实施方向正确；G1–G5 为本审计新增遗漏候选，#92 报告须逐条给出计数或豁免理由，缺一不得签"0 为市场几何"。

## 终判（#92 重放完成后，2026-07-16）

重放实测（`P92_*` 原始行已核对，与报告逐项一致）：

| 漏斗 | 数值 |
|---|---:|
| 结构 Cand | 3,807（Trend 894 / Pan 2,913） |
| ② Weak 确认 | 727（Trend 1 / Pan 726） |
| BspBits 终端 | 19（全 Pan） |
| 生产证书（B 口径） | **19** |

- **进料口①闭合**：Pan 候选 2,913 > 0，盘整块已实际进链（旧路径此处恒 0）。
- **初筛②闭合**：Cand 3,807 vs 旧 483（+688%），力度税确已移出候选门；Weak 独立合取后 727。
- **G1 部分闭合（残项）**：map 失败率计数器未实装；但 727 个 divergence_confirmed 非零，"MACD 坐标静默清零致全军覆没"情形被经验排除。残项：下轮加 `MissingMacdCoordinates` 桶计数。
- **G2/G3 经验豁免（残项）**：分步计数未实装；但 Pan 漏斗全程非零，零产量归因需求本轮不成立。留计数器需求给下一次归因场景。
- **G4/G5 闭合**：见上文。
- **R7 终判**：(a) **FAIL/能力边界**——`invalid_seed=215`（#90 exact-three 投影 fail-closed），`unresolved_targets=0` 但 `complete=false`；(b) PASS；(c) PASS（`future_violations=0, created_at_reads=0`）。
- **bit-exact**：旧路径 5 项 diff 全 0，迁移未污染既有终态。
- **签字口径**：B=19 非零，零产量签字分支未触发；且因 (a) 未过，任何未来 B=0 只能写"provider 能力边界/未决"，不得写"市场无证书"。**"0 不可能是市场答案"已由 19 张实证收回——旧路径的 0 是工程扔料，不是市场几何。**

遗漏总账：三主扔料点 + G1–G5 全部排查完毕；唯一存续缺口 = 215 个 `InvalidSeed`（能力边界，独立追踪）。
