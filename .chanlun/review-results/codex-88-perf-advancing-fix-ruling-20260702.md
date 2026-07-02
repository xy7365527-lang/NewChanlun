# codex 全权裁定（补充轮次）：#88 parser frontier 性能三选一——advancing 变体直接可行性论证

**★重复派工声明**：本文件与 `.chanlun/review-results/codex-88-perf-ruling-20260702.md`（ws-codexp2
工位、`codex-decide-20260702-234836-ae54.md`）是**同一任务的两次独立 codex 会话**（team-lead 并行
派发导致的重复工位，非本工位主动重跑）。两份裁定**表面结论不同**（既有裁定："先建 H1/H2 判别探针，
生产逻辑维持不变"；本裁定：直接选 advancing 变体），**但经推理链核对后不矛盾，是可调和关系**——见
§三。**不覆盖既有文件**，供 team-lead 裁决如何处置正在进行的探针任务（#93）。

**本工位 codex 交互记录**：`.chanlun/review-results/codex-decide-20260702-235142-8fc1.md`
**审查上下文**：`tmp/codex-88-perf-ctx.md`（含本工位对 advancing 变体的 sound 草证 + 四个反例方向）

---

## 一、结论

**codex 裁决：选项2（advancing 变体），精确表述为 `earliest_unsealed_from = euf_rescan`**（本轮
`append` 完整重扫 `[resume_seg_start, n)` 后的新鲜结果直接持久化，不与更早历史取 min）。

拒绝选项1（accept O(n²)：现有证据不支持"这是 scan_window=0 语义下界"）与选项3（候选C 升主路径：
循环论证仍成立）。四个反例方向核对结果见 §二，**均可排除或已被现有代码正确处理**（详细代码级核实，
非空对空的 codex 口头确认）。

## 二、四个反例方向核对（代码级独立核实，非仅采信 codex 口头判断）

| # | 方向 | codex 判断 | 我的代码核实 | 结论 |
|---|------|-----------|------------|------|
| (a) | truncate 用 `end_idx`、flag 用 `seg_start`，off-by-one 导致漏扫？ | 非风险（前提"段连续"成立） | 逐行核对 `segment.rs`：`end_stroke=k-1` 推出后 `seg_start=k`，即 `seg_start_next=end_stroke_prev+1`；flag 在 `seg_start=S` 时，段 `j-1` 的 `end_idx=S-1<confirmed_bound=S` 必被保留，`resume_seg_start=(S-1)+1=S`，精确重合 | **非风险，代码证实** |
| (b) | `pending_start` 是否需要参与 `confirmed_bound` | 不是该字段职责 | 全仓 grep 确认 `pending_start` 从未参与 truncate/confirmed_bound 计算 | **非风险** |
| (c) | `skipped_secondkind` 是否需要段内 sticky（非逐笔瞬时） | 概念正确，前提 sticky 到 caller | 该字段是 `FeatureSeqState` 结构体字段，跨多次 `append`/`scan_trigger` 持续存在，仅 `reset()` 清零；caller 在 `reset` 前读取，符合设计 | **非风险，代码证实** |
| (d) | `feat.reset()` 是否漏清 flag，跨段污染 | **判定为真实风险，要求新增守卫** | **codex 此判断基于我提供的上下文不完整**（我未在 ctx 中贴 `reset()` 函数体）。实际 `feature_seq.rs:267-277`：`reset()` 内含 `self.skipped_secondkind = false;`（注释即写"#88：清 skipped_secondkind（每段独立）"）——**已经正确清零** | **codex 因上下文缺口误判，代码本身已正确，无需修复** |

**关键论证**（codex 给出 + 本工位复核）：`second_seq_has_fractal` 的"一旦真则永真"单调性（构建过程严格
由既定前缀决定，未来追加的笔不改变已算出的中间 elements）+ 段连续性保证的 `resume_seg_start` 精确重合
flag 位置 ⟹ 任何真正未解决的候选一定会在下一轮重扫中被重新发现；只有当某轮从该位置完整重扫到 `n`
全程零 flag 时才允许清零/前移，而这恰好等价于把 `divide_segments_with_tail` 直接跑在该后缀上的结果，
不存在级联改写残留风险。

## 三、与既有裁定（H1/H2 探针）的调和——为何不矛盾

既有裁定的核心担忧：现有诊断字段（`euf_fin`/`euf_min`/`euf_adv` 累积终值）无法区分
"H1：候选每轮真实持续复现" vs "H2：候选早已 resolve 但 persist-forever 掩盖"，在区分之前三选一都是
"赌一个未经验证的假设"。这个担忧**成立**——但它假设了"三选一"是互斥的、必须先诊断出 H1/H2 才能选对；
本裁定给出的论证表明这个前提不成立：

**advancing 变体（`earliest_unsealed_from = euf_rescan`）在 H1 和 H2 下都是安全的，且其运行结果本身
就是 H1/H2 的判别探针**——不需要一个独立的、只读的诊断先导步骤：

- 若 H1 成立（候选真实持续复现）：advancing 变体每轮重扫仍会在 stroke 10 重新发现 flag，
  `euf_rescan=Some(10)`，`earliest_unsealed_from` 依然锁定在 10——**性能与现状完全一致，无退化、
  无提升**，同时这个"仍然锁定"的观测结果本身就证实了 H1。
- 若 H2 成立（候选早已 resolve 但被掩盖）：advancing 变体在候选 resolve 后的那一轮，`euf_rescan`
  变为 `None`（或前移到更晚位置），`earliest_unsealed_from` 相应清零/前移，`confirmed_bound` 前移，
  **性能提升**，同时这个"前移"的观测结果本身就证实了 H2。

也就是说：**直接实装 advancing 变体 + 复用既有的 `perf_frontier_rescan_counters_88`/bit-exact oracle
回归测试，就是成本最低的 H1/H2 判别方式**——比"先建一个只读诊断探针（#93），再决定是否实装 fix"少一轮
工位往返，且不存在额外风险（advancing 变体本身已经过 §二 四项反例核实，不依赖 H1/H2 判别结果才能安全
实装——它在两个假设下都不会引入新的 bit-exact 发散，唯一的区别是"是否带来性能提升"）。

**结论**：既有裁定的"先建探针"建议在其自身的证据基础上是合理的保守选择（它没有做 §二 的代码级
soundness 核实，因此需要先经验性地排除风险）；但本裁定通过对 `second_seq_has_fractal` 单调性 +
段连续性的结构性论证，已经把"advancing 变体是否安全"这个问题从"需要先经验诊断"变成"可以先验证明"，
使独立诊断步骤（#93）成为**冗余**——直接实装 fix 本身就同时完成了"性能修复"和"H1/H2 判别"两件事。

## 四、建议处置

若任务 #93（`euf-probe: H1/H2 判别探针`）尚未产出实质代码改动：**建议 team-lead 将其范围直接扩展为
本裁定 §三/§四 的 advancing 变体实装**（把"只加诊断探针"升级为"实装 `earliest_unsealed_from =
euf_rescan` + 复用现有 bit-exact oracle 回归 + 观察 `euf_fin` 是否前移"），一次性完成修复与判别，
避免"先探针、再等结果、再派发实装工位"的三轮往返。若 #93 已经产出了独立诊断字段的代码且接近完成，
可以保留探针作为**额外的过程可观测性**（不与 fix 冲突，探针字段和 fix 可以在同一个改动里一起交付），
不需要为了避免"重复劳动"而回滚已完成的探针工作。

## 五、修复方案（与 §三 一致，供实装采用）

`rust/src/theta_v0/parser/segment.rs::IncrSegments::append` 末尾：

```rust
// 修改前：
let earliest_unsealed_from = match (euf_persisted, euf_rescan) {
    (Some(a), Some(b)) => Some(a.min(b)),
    (Some(a), None)    => Some(a),
    (None, b)          => b,
};

// 修改后（advancing 变体）：
let earliest_unsealed_from = euf_rescan;
```

`euf_persisted`（本轮开头取出的上一轮存值）仍用于计算本轮 `confirmed_bound =
last_end_idx.min(euf_persisted)`；改动的唯一点是本轮结束后持久化给下一轮的新值直接等于本轮重扫结果，
不再与更早历史取 min。

## 六、必须新增的守卫测试清单

1. **每 append 全量 oracle 对拍**：复用现有 `decisive_endpoint_tower_parity_longhistory`/
   `diag_parser_incr_vs_full_segments`，改动后重跑确认零发散（不新增测试文件）。
2. **flag resolve 后前移测试**：合成序列，构造"候选在 append T 被跳过、在 append T+k 因新增同向笔
   resolve、且该轮从 resume_seg_start 到 n 全程无其他 flag"——断言 `earliest_unsealed_from` 变为
   `None`/前移，`confirmed_len` 相应增长。
3. **清零后 later reflag 测试**：在上一条基础上继续追加数据构造新的更晚候选被跳过——断言
   `earliest_unsealed_from` 变为新位置（非退回旧位置）。
4. **continuity debug_assert**（(a) 项核实固化）：`IncrSegments::append` 内 debug-only 断言
   `resume_seg_start == confirmed_bound`（不多不少）。
5. **CL 300K 性能回归**：重跑 `perf_frontier_rescan_counters_88`，确认 `euf_fin` 是否前移（此即
   H1/H2 判别的直接产出，见 §三）。

**唯一未穷尽核实的分支**：段连续性论证仅覆盖主循环 `while cursor < n` 的正常路径，"无 confirmed 段"
边界分支（`segment.rs:580-606`，`find_overlap_start` 路径）未单独核实 `resume_seg_start` 是否精确
重合 flag 位置——建议第4项 debug_assert 一并覆盖该分支。

## 七、结果包六要素

1. **结论**：见 §一/§三。advancing 变体直接可行，且其自身就是 H1/H2 判别方式；既有"先建探针"裁定
   不矛盾（合理的保守默认，本裁定用结构性论证使其成为可选而非必须）。
2. **定义依据**：`second_seq_has_fractal` 一旦真则永真的单调性 + `seg_start_next=end_stroke_prev+1`
   段连续性，均已在本仓代码中逐行核实成立（§二）。
3. **边界条件**：(i) 若全量 oracle 对拍出现任一字段不一致 → 推翻，退回选项1或重新设计；(ii) 若
   `find_overlap_start` 边界分支的 `resume_seg_start` 不精确重合 flag 位置 → 需单独修正该分支；(iii)
   若未来发现 `second_seq_has_fractal` 在某边界下会因追加笔改写已算出的历史 elements → 单调性前提
   推翻，advancing 变体失效。
4. **下游推论**：`earliest_unsealed_from`/`confirmed_len` 诊断语义从"历史累积最小值"变为"本轮新鲜值"，
   两者均为纯诊断透出字段（已 grep 确认无其他模块依赖旧语义），下游安全。
5. **谱系引用**：本裁定是 `codex-87-parser-frontier-fix-ruling-20260702.md` 的直接后续，与
   `codex-88-perf-ruling-20260702.md`（ws-codexp2 裁定）构成同任务重复派工的两次独立 codex 会话——
   建议 genealogist 参照本仓已有的"编号碰撞判决"处置模式（近期 commit 9b1b7c2aee 三组编号碰撞判决）
   记录本次重复派工事件，作为"并行 spawn 可能对同一任务产生不同 codex 会话结论"的方法论案例。
6. **影响声明**：本工位零代码改动，只读 codex CLI 调用 + 结果落盘。改动交由后续实装工位（建议直接
   并入 #93 或新开工位），不涉及 `feature_seq.rs`（已核实正确）、不涉及 tower 层。

```yaml
---stance-declaration---
verdict: conditional
stances:
  option1_accept_on2: reject
  option2_advancing_variant: accept
  option3_candidate_c_primary: reject
  reset_clears_flag_already_correct: confirmed_no_fix_needed
  probe_task_93_necessity: redundant_but_not_harmful
  agent_second_order_judgment: codex_negation_upheld_with_correction
concessions:
  - codex 对 (d) feat.reset 清 flag 的风险判断经代码核实为误判（本工位提供上下文时遗漏 reset() 函数体
    导致），代码本身已正确，不构成对整体裁决的推翻。
  - 与既有裁定（codex-88-perf-ruling-20260702.md）的关系是调和而非否定——本裁定不主张既有裁定"错"，
    只主张其保守假设可以被结构性论证替代，从而节省一轮探针-实装往返。
  - 未穷尽核实"无 confirmed 段"边界分支（segment.rs:580-606）的 resume 点精确性。
---end-stance---
```
