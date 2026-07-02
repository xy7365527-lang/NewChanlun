# codex 审计裁定：区间套问题② parser 线段增量 frontier 修复设计（#87）

**审计对象**：`.chanlun/review-results/frontier-bt-consumed-20260702.md` §四 修复设计（候选 A/B/C）
**codex 原始交互记录**：`.chanlun/review-results/codex-review-20260702-231316-9c66.md`
**审查上下文**：`tmp/codex-87-parser-frontier-fix-ctx.md`（PDF §六-§十一 canonical/Prefix-Mutable-Frontier/增量等价定理 + `segment.rs::IncrSegments::append` + `feature_seq.rs::second_seq_has_fractal` 原文引用）

---

## 一、结论

**codex 裁决（`verdict: conditional`）**：候选 A **原始形态不可直接实装**（`candidate_a_direct: reject`），
但存在可接受的**修补版 A**（`candidate_a_repaired_unsealed_frontier: accept`）；候选 B 排除成立
（`candidate_b_for_current_canonical: reject`）；候选 C 不可作主路径（`candidate_c_as_primary: reject`），
只能作 oracle/兜底（`candidate_c_as_oracle_fallback: accept`）；性能红线待补证据
（`performance_redline: needs_work`）。

**我的判定：codex 否定成立，接受裁决。** 理由（简化质询三问）：

1. **问题是否真实存在**——已在 `rust/src/theta_v0/parser/feature_seq.rs:327-372`
   (`FeatureSeqState::scan_trigger`) 逐行核对：当 `has_gap && !second_seq_has_fractal(...)` 时代码执行
   `continue`（feature_seq.rs:364-367），**不写任何字段记录这个被跳过的 SecondKind 候选**——`last_checked`
   只在**确认成功**分支更新（feature_seq.rs:368），跳过分支不留痕迹。`IncrSegments`（segment.rs:414-431）
   的字段集里也确实**没有**任何"曾经 pending 过的候选起点"的持久化槽位。故候选 A 原报告草案里的
   `earliest_pending_secondkind_stroke_idx`若实现为"记录**当前仍处于 pending**的锚点"，会在该候选
   被后续更多 bar 确认为 confirmed 段之后**立即失去追踪**——但确认这个候选所依赖的第二特征序列扫描
   仍是"当前可见 strokes 的一次快照判定"，并不能排除**更早的另一个仍处于 pending 状态**的
   SecondKind 候选，在未来 bar 到达后确认，从而级联改写这个"看似已 confirmed"的段（因为它的起点/终点
   本身落在被级联改写的更早段范围内）。这正是报告 §三 反例（`seg[404]` 被全量重划到更早的
   `48798/7648`，而非局部微调）的结构性成因——问题不真实存在的可能性被代码事实排除。
2. **是否已被其他机制覆盖**——没有。`skip_trigger`/`skip_until_stroke`（segment.rs 状态机侧）只是
   "跳过 stroke_idx ≤ 该值的分型候选"的**去重**机制（防止同一 trigger 被重复处理），不是"标记此区域
   theoretically unsafe for incremental resume"的语义,两者职责不同、不能互相顶替。
3. **严重性判定是否合理**——"致命"（问题1：pending 锚点不 sound）合理，因为它直接对应 PDF §九定理
   前提2（prefix stability）不成立的根因，不是风格问题。其余"重要"级问题（apex 锚点误用、候选C
   循环论证、候选B 改变 canonical、性能无界）判定层级恰当，均可在代码中逐一验证位置存在（见下方
   引用行号，除 `config.rs:52` 与实际字段声明行 `config.rs:57`/默认值行 `config.rs:65` 有轻微偏移
   外，其余引用位置经核对与代码一致）。

**codex 给出的具体修复设计（本裁定采纳，供实装工位直接消费）**：
- 新增持久化字段 `earliest_unsealed_from: Option<usize>`——在 `FeatureSeqState::scan_trigger` 遇到
  `has_gap && !second_seq_has_fractal(...)`（即将被跳过的 SecondKind 候选）时，记录该候选**所属段的
  `seg_start`**（不是 apex/b_stroke 偏移）到本轮 `earliest_unsealed_from`，取历史最小值（跨多次
  append 持久化，不因候选后续被 confirm 或被新的 confirmed 段"覆盖"而丢弃）。
- `IncrSegments::append` 的 `confirmed_bound` 由现在的"末段 end_array_idx"改为
  `min(last_end_idx, earliest_unsealed_from.unwrap_or(last_end_idx))`——即回退到"该 unsealed 起点"
  之前的最深稳定段端，而不是固定 1 段。
- 候选 C 降级为**测试期 oracle / 罕见兜底**，不进主路径（"检出确认前缀内改写后 cascade"本身需要先
  全量重算才能判断是否需要 cascade，等于先付出 O(n) 才决定要不要 O(n)，循环论证）。
- 候选 B 维持排除（`second_seq_scan_window` 是同时影响增量与全量的 canonical config，非纯性能参数；
  若要用有界 W，须显式标注为新的 `F_W` 实验对象，不能冒充当前 `F_∞`（default W=0）的修复）。

---

## 二、边界条件（本裁决在什么条件下翻转）

1. **若能证明"曾经 pending、已被 confirm"的段永不可能被更早 pending 的级联复活改写**（即
   second_seq_has_fractal 一旦对某候选返回 true 即为该候选的**终局真值**，不受该候选之前尚未
   resolve 的更早候选影响）——则候选 A 原始形态（只追踪当前 active pending）可能已经 sound，不需要
   `earliest_unsealed_from` 的历史持久化版本。但报告 §三 实测反例（`seg[404]` < `confirmed_len`）
   已经**经验证伪**这个可能性——发散段不是末段，说明改写确实跨越了"已被移出 active 追踪范围"的段。
   故本条边界在当前证据下不成立，codex 裁决站得住。
2. **若 `second_seq_scan_window` 被证明是纯粹的性能参数、且 canonical 判据本身对 W 不敏感**（即
   `divide_segments_with_tail` 用任意 W≥某阈值都产出相同 segments，W 只影响运行时间不影响结果）——
   则候选 B 的排除理由会弱化为"未证明而非不成立"，需要补一个"scan_window 敏感性"L2 实测（不同 W
   下 canonical 输出是否变化）才能真正关闭候选 B。这是本裁决**唯一留有余地**的一点，建议实装工位
   顺手补测（成本低：跑几个 W 值对比 canonical 输出）。
3. **若修补版 A 的最坏情况 rescan 深度在真实 CL/BTC 数据上经验证是 O(1)~O(小常数)（不是理论无界）**，
   则"性能红线 needs_work"的顾虑可解除，不需要额外候选 D；若经验证是持续接近 O(n)（例如
   `earliest_unsealed_from` 长期锚定在很早的位置、几乎不前移），则修补版 A 退化为候选 C 的性能特征，
   届时需重新评估是否值得维护"修补版 A"这层复杂度而不直接用候选 C。

---

## 三、影响声明

- **涉及模块**：`rust/src/theta_v0/parser/segment.rs`（`IncrSegments`/`IncrSegments::append`/
  `divide_segments_with_tail`）、`rust/src/theta_v0/parser/feature_seq.rs`
  （`FeatureSeqState::scan_trigger`/`second_seq_has_fractal`）。**不涉及** `second_kind.rs`（静态
  确认逻辑本身不改，只改增量层"何时可复用前缀"的判据）、**不涉及** tower 层（`econ_positive.rs`/
  `classifier/mod.rs`/`classifier/bsp.rs`，塔层 resume 已 bit-exact，本次修复域维持 #84/frontier
  报告 §二 的重定位结论：真 bug 在 parser，非 tower）。
- **本次审计零代码改动**（只读 codex CLI 调用 + 结果落盘）。生产改动交由后续实装子任务（阻塞至今解除）。
- **下游解锁**：#87 完成后，parser 修复实装子任务可按 codex 裁决的具体设计（`earliest_unsealed_from`
  + `seg_start` 锚 + 候选C降级oracle + 候选B维持排除 + 性能计数器）直接开工，验收标准见下。

---

## 四、验收约束清单（codex 给出，供实装子任务直接采用）

1. `diag_parser_incr_vs_full_segments` 在 CL/BTC 50K/150K/300K 无发散（div_count=0）。
2. `decisive_endpoint_tower_parity_longhistory` L2+ centers/bsp 全 bit-exact。
3. 新增合成测试覆盖"早期 SecondKindPending 后续被确认，改写其后已 confirm 的 FirstKind 段"这一
   级联复活模式（当前 `bit_exact_incr_segments_per_bar` 测试用的合成序列未必天然触发此路径，需专门
   构造）。
4. 不变式：任意时刻 `segments_confirmed_len` 对应的段端下标不得越过 `earliest_unsealed_from`。
5. 性能报告：必须给出 `frontier_rescan_strokes`/`earliest_unsealed_from` 前移轨迹/`cascade_count`
   （如启用候选C兜底）/p95-max rescan 长度的实测数据（CL/BTC 三档 n），不能只报最终 bit-exact
   结论——用以判定本裁决边界条件3是否触发。
6.（本裁决边界条件2留下的补测项，非强制阻塞但建议一并做）：跑 2-3 个 `second_seq_scan_window` 有限
  值 vs 0，对比 canonical (`divide_segments_with_tail`) 输出是否变化，坐实/证伪候选B排除的充分性。

```yaml
---stance-declaration---
verdict: conditional
stances:
  candidate_a_direct: reject
  candidate_a_repaired_unsealed_frontier: accept
  candidate_b_for_current_canonical: reject
  candidate_c_as_primary: reject
  candidate_c_as_oracle_fallback: accept
  performance_redline: needs_work
  agent_second_order_judgment: codex_negation_upheld
concessions: []
---end-stance---
```
