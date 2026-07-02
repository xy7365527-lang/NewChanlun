# codex 裁定：parser frontier 修复（#88）性能红线三选一重评

- **工位**：ws-codexp2（沿用，goal 续接 parser frontier 修复链）
- **日期**：2026-07-02
- **审计对象**：`.chanlun/review-results/parser-frontier-fix-20260702.md` §四 性能红线（边界条件3已触发）
- **Codex 完整交互**：`.chanlun/review-results/codex-decide-20260702-234836-ae54.md`
- **裁定结论**：**第四选项——先建 H1/H2 判别探针，生产逻辑维持 #88 修补版 A 不变**（非三选一中任何一个，是三选一都不成熟前的必要前置步骤）

---

## 1. 结论

不接受 O(n²)、不上 advancing 变体、不将候选 C 升级为主路径。三者都需要先知道一个当前无法从现有诊断数据得出的事实：`earliest_unsealed_from` 锚定于 stroke 10 且 `euf_adv=1`（终值恒定），这个现象**同时兼容两个互斥假设**：

- **H1（真实持续复现）**：每一次 append 的重扫都独立重新发现 stroke 10 处的未决候选（该候选结构性地永不 resolve）——此时任何"advancing"式的不持久化方案都救不了，O(n²) 是 bit-exact 在 `second_seq_scan_window=0` 下的内在下界。
- **H2（persist-forever 掩盖）**：候选在某次重扫中已不再触发 skip，但 `earliest_unsealed_from` 的"历史最小值单调不增、永不清除"机制（`(Some(a), None) => Some(a)`，`segment.rs:666`）让旧值 10 永久沿用——此时"advancing 变体"（不跨 append 持久化，只用本轮重扫的新发现）可能可以 sound 前移，退回 O(n)。

现有诊断（`euf_fin`/`euf_min`/`euf_adv` 累积后的终值）在这两支下产出**完全相同的可观测结果**，无法区分。在区分之前做三选一中的任何一个都是在赌一个未经验证的假设。

**行动项**（低成本，Codex 判断"语义零改动，bit-exact 风险接近零"）：给 `segment.rs:621` 的局部变量 `euf_rescan` 加一个诊断透出——记录每次 `append` 本轮重扫的 `euf_rescan: Option<usize>`（`is_some` 计数、具体值、连续 `Some(10)` 次数）。这个探针只读已经算出来的值，不改变 `feature_seq.rs:375` 的生产判定逻辑。

## 2. 定义依据

- 代码事实（`segment.rs:664-668`）：
  ```rust
  let earliest_unsealed_from = match (euf_persisted, euf_rescan) {
      (Some(a), Some(b)) => Some(a.min(b)),
      (Some(a), None) => Some(a),   // H2 支：本轮未复现，仍沿用旧值——不可观测
      (None, b) => b,
  };
  ```
  `(Some(10), Some(10)) => Some(10)`（H1 支）与 `(Some(10), None) => Some(10)`（H2 支）产出的 `earliest_unsealed_from` 值完全相同，现有诊断字段（只记终值）无法回溯区分两支中哪一支被走过。
- `exp=2.04` 的墙钟实测判定为**真实持续 O(n²) 成本**，不是"一次性全量重扫被误报为持续"：只要 `earliest_unsealed_from` 持久化为 10，`confirmed_bound` 每次 append 都回退到 10，增量框架的机制决定了每个 bar 都要重放 `[10, n)` 区间——这个代价与"候选本身是否需要每次重新判定"无关，是框架设计的必然后果。

## 3. 边界条件（探针结果如何驱动后续决策）

- **若探针显示几乎每次 append 都是 `Some(10)`**（H1 成立）：接受 O(n²)，或探索定义层面的其他优化路径；不做 advancing（做了也无效）。
- **若探针显示早期之后大量 `None` 或 `euf_rescan` 明显前移**（H2 成立）：重新审查 advancing 变体，目标改为 `euf_next = euf_rescan`（不跨 append 累积历史最小值），并用 CL/BTC 50K/150K/300K bit-exact 重验收（与 #88 验收清单同规格）。advancing 变体因涉及算法变更（偏离 #87 裁决"历史最小值持久化"的原授权设计），需要专门的 soundness 论证：从旧 euf 起完整重扫，若本轮没有再发现 false SecondKind，才允许下一轮前移/清空——不能退化回 #87 已明确否决的"只追踪当前 active pending"（候选A原始形态）。
- **若 1M/多标的规模下的实际 parse 成本在总流程占比很小**：即便 H1 成立，也可以接受 O(n²)，但需要把适用规模边界写清楚（当前只测到 300K，1M 外推 ≈20s 是否可接受需要结合下游流程总耗时判断，见下游推论）。

候选 C 升主路径的否决理由（#87 已给出的"循环论证"：判断是否需要 cascade 本身需要先全量重算）在本次重评中**维持成立**，未发现新证据推翻。固定周期 oracle 校验/兜底仍可作为候选 C 的降级用途，不改变其非主路径的定位。

## 4. 下游推论

- p2/parser 修复链暂不可继续往"最终性能方案"推进，需先插入一个诊断子任务（低成本，预计几十行代码：给 `IncrSegments::append` 内的 `euf_rescan` 加诊断透出 + 在 `perf_frontier_rescan_counters_88` 测试里增加统计输出）。
- 该诊断子任务本身有一个明确的 bit-exact 风险点（Codex 提示）：新增的诊断字段**不得**混入 `IncrSegments` 的 `PartialEq` 实现，否则会在增量/全量 bit-exact 对拍中制造假发散——必须作为纯诊断态字段排除在结构相等比较之外（参照现有 `strokes_len`/`confirmed_len` 等字段已有的 diag 排除模式）。
- 在探针结果出来前，parser 层的性能不阻塞正确性相关工作（#88 已 bit-exact），但阻塞任何依赖"parser 增量层达到 O(n) 或可接受性能"的规模化验证（1M+ 全市场回测、跨标的 L3 池化的运行时预算规划）。

## 5. 谱系引用

- `.chanlun/review-results/codex-87-parser-frontier-fix-ruling-20260702.md`（本次重评的直接上游裁决，边界条件3 的预警在此落笔）。
- `.chanlun/review-results/parser-frontier-fix-20260702.md`（#88 实装报告，性能红线数据来源）。
- `project_frontier_resume_bt_too_late`（memory，frontier 根因链）。
- 建议 genealogist 评估新增谱系条目：「持久化历史最小值机制在诊断层不可观测其内部两条路径（真实复现 vs 掩盖已解决），需要专门的逐轮探针才能做出后续算法选择——'终值相同不代表路径相同'是持久化单调机制的通用诊断陷阱，可能在其他类似"历史最小值/最大值持久化"设计中重现」。

## 6. 影响声明

- 本工位（decide 模式）纯只读审计 + Codex 调用，未修改任何生产代码。
- 产出：本裁定文件 + Codex 完整交互记录（`.chanlun/review-results/codex-decide-20260702-234836-ae54.md`）。
- 下游消费：parser 修复链下一个实装子任务（诊断探针，阻塞在此裁定之后）；#91 任务本身到此完成裁决交付。

---

```yaml
---stance-declaration---
verdict: needs_probe_before_decision
review_target: parser frontier fix (#88) 性能红线三选一
stances:
  accept_o_n_squared: defer
  advancing_variant: defer
  candidate_c_as_primary: reject
  build_h1_h2_probe: accept
  probe_cost: low
  candidate_c_circular_argument_still_valid: confirmed
diagnostic_gap_identified: "euf_adv 终值统计无法区分'每轮重新复现'(H1)与'persist-forever掩盖已解决'(H2)——(Some(a),Some(a))与(Some(a),None)产出相同終值"
next_action: "给 segment.rs::append 的局部变量 euf_rescan 加诊断透出（不入 PartialEq），重跑 perf_frontier_rescan_counters_88 判别 H1/H2"
concessions: []
---end-stance---
```
