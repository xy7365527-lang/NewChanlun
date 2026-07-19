# wave-1 实装卡：方案 1 trend_confirm_time 单源化（2026-07-19）

**状态**：待②验收后执行（lib 改动须在验收基线代码版本稳定后）。设计依据：`optimize-wave1-research-20260719.md` 方案 1 + `wave1-trend-confirm-double-compute-evidence-20260719.md`。
**收益** [实测]：消除 provide 侧重算 ~23.5s@1M（prefix 10.3%）；难度小、风险低。
**bit-exact 论证**：同函数同入参同输出，查表替换重算；`None` 语义两侧逐字一致（assemble mappable 门未过存 None ≡ provide 预滤 Extreme 拒掉的对应对不查表）。

## 改动清单（lib `theta_v0/classifier/level_view.rs`，四处）

### A. 结构定义增字段（:936-941）

前：
```rust
pub struct LevelAsOfView {
    pub query: LevelViewQuery,
    pub cache_key: C2CacheKey,
    pub moves: Vec<AssembledMove>,
    pub pairs: Vec<DivergencePair>,
}
```

后：
```rust
pub struct LevelAsOfView {
    pub query: LevelViewQuery,
    pub cache_key: C2CacheKey,
    pub moves: Vec<AssembledMove>,
    pub pairs: Vec<DivergencePair>,
    /// ★wave-1 方案 1（trend_confirm_time 单源化）：与 `pairs` 逐位对齐的确认时点缓存。
    /// assemble 在 pairs 产出后对每 pair 以与 provide 逐字相同的入参计算一次
    ///（mappable 门未过存 `None`，与 trend_confirm_time 内部 map 失败返 None 语义等价）；
    /// provide 查表替代重算（纯函数结果复用，判据零改动）。
    pub confirm_times: Vec<Option<usize>>,
}
```

### B. pairs 产出后统一计算（:1003 之后、:1005 循环之前插入）

在 `let segments: Vec<_> = material.lower_legs.iter().map(leg_as_segment).collect();`（:1003）之后、`let mut moves = ...`（:1005）之前插入：

```rust
    // ★wave-1 方案 1：对每 pair 统一计算一次 trend_confirm_time（单源）。
    // mappable 门 = :1036-1047 同一判定（seg_a+seg_c 双侧 map 成功）；未过存 None。
    // direction/side/last 与 moves 循环内 :1054-1061 的逐字同源：
    //   pair.id.direction == block.dir（pair 由 provide_divergence_pairs 按 block 构造，
    //   id.block_end_center == block.end_center；assemble 由 move_start == start_index 反查同一 block）。
    let confirm_times: Vec<Option<usize>> = pairs
        .iter()
        .map(|pair| {
            let mappable = map_src_to_close_idx(material.close_src, pair.seg_a.0, pair.seg_a.1)
                .is_some()
                && map_src_to_close_idx(material.close_src, pair.seg_c.0, pair.seg_c.1)
                    .is_some();
            if !mappable {
                return None;
            }
            let direction = pair.id.direction;
            let side = match direction {
                Direction::Down => Side::Long,
                Direction::Up => Side::Short,
            };
            let last = &projection.seeds[pair.id.block_end_center].center;
            trend_confirm_time(
                &segments,
                last,
                direction,
                side,
                pair.seg_a,
                pair.seg_c.0,
                query.as_of,
                material.hist,
                material.dif,
                material.close_src,
            )
        })
        .collect();
```

### C. moves 循环查表替代（:1025/:1062）

前（:1025）：
```rust
                let pair = pairs.iter().find(|pair| pair.move_start == start_index);
```

后：
```rust
                let pair_pos = pairs.iter().position(|pair| pair.move_start == start_index);
                let pair = pair_pos.map(|pos| &pairs[pos]);
```

前（:1036-1084，mappable+match trend_confirm_time 整块）：
```rust
                        let mappable = map_src_to_close_idx(...).is_some() && ...;
                        if !mappable {
                            CompletionStatus::Pending { as_of: query.as_of, reason: PendingReason::MissingMacdCoordinates }
                        } else {
                            let direction = block.dir.expect(...);
                            let side = ...;
                            let last = &projection.seeds[block.end_center].center;
                            match trend_confirm_time(&segments, last, direction, side, pair.seg_a, pair.seg_c.0, query.as_of, material.hist, material.dif, material.close_src) {
                                Some(_) => CompletionStatus::Completed { as_of: query.as_of, evidence: CompletionEvidence::TerminalDivergence { pair_id: pair.id } },
                                None => CompletionStatus::Pending { as_of: query.as_of, reason: PendingReason::TerminalLegNotDivergent },
                            }
                        }
```

后（查表，语义逐项对应）：
```rust
                        // ★wave-1 方案 1：查 confirm_times 替代重算（单源，入参逐字同 B 段）。
                        // mappable 未过存 None ≡ 原 MissingMacdCoordinates；
                        // 表值 None ≡ 原 trend_confirm_time 返 None（TerminalLegNotDivergent）；
                        // 表值 Some ≡ 原返 Some（Completed/TerminalDivergence）。
                        // 注：原实现 mappable 门与 trend_confirm 返 None 分别映射两种 Pending；
                        // 查表后两路径同归 None，但 CompletionStatus 的 reason 区分保留——
                        // 需要区分时按 B 段 mappable 判定重演一次（廉价 map 查询，非重算）。
                        let mappable = map_src_to_close_idx(material.close_src, pair.seg_a.0, pair.seg_a.1)
                            .is_some()
                            && map_src_to_close_idx(material.close_src, pair.seg_c.0, pair.seg_c.1)
                                .is_some();
                        if !mappable {
                            CompletionStatus::Pending {
                                as_of: query.as_of,
                                reason: PendingReason::MissingMacdCoordinates,
                            }
                        } else {
                            let confirm_t = pair_pos.and_then(|pos| confirm_times[pos]);
                            match confirm_t {
                                Some(_) => CompletionStatus::Completed {
                                    as_of: query.as_of,
                                    evidence: CompletionEvidence::TerminalDivergence { pair_id: pair.id },
                                },
                                None => CompletionStatus::Pending {
                                    as_of: query.as_of,
                                    reason: PendingReason::TerminalLegNotDivergent,
                                },
                            }
                        }
```

（mappable 判定保留在原地——它是 O(log n) map 查询，不是重算；保留它使两种 Pending 的 reason 区分不丢。）

### D. 构造点填字段（:1099-1104）

前：
```rust
    Ok(LevelAsOfView {
        query,
        cache_key,
        moves,
        pairs,
    })
```

后：
```rust
    Ok(LevelAsOfView {
        query,
        cache_key,
        moves,
        pairs,
        confirm_times,
    })
```

### E. provide 侧查表替代（:696 附近）

前（:672-696 区域，extreme 预滤后）：
```rust
        let last_center = &projection.seeds[pair.id.block_end_center].center;
        let __t = std::time::Instant::now();
        let confirm_t = trend_confirm_time(
            &segments,
            last_center,
            pair.id.direction,
            side,
            pair.seg_a,
            pair.seg_c.0,
            view.query.as_of,
            hist,
            dif,
            close_src,
        );
        p126_split::tick(&p126.t_tr_confirm, __t);
        p126_split::bump(&p126.n_tr_confirm, 1);
```

后：
```rust
        // ★wave-1 方案 1：查 view.confirm_times 替代重算（单源，assemble 已按同一入参算过）。
        // pairs 迭代需带索引（pairs.iter().enumerate()），表值 None ≡ trend_confirm 返 None。
        let confirm_t = view.confirm_times[pair_idx];
        p126_split::bump(&p126.n_tr_confirm, 1); // 计数口径保留（查表也算一次「确认查询」）
```

**注意**：provide 的 pairs 迭代处（:823 `for block in blocks` 构造 pairs，:646 起的 provide_nest_candidate_events 消费 view.pairs）需要确认迭代形态——若当前是 `for pair in &view.pairs` 需改为 `for (pair_idx, pair) in view.pairs.iter().enumerate()`。pre-滤（extreme）在查表前不变。

## 验证协议（与 p123/p124 同）

1. `cargo test --lib` 全绿零变红（含 level_view 测试网内 view 相等断言复核——PartialEq 派生含新字段，相等性语义增强更严格，已核无跨构造路径断言受影响）。
2. 250k/1M stdout+dump 对同码基线 diff=0（白名单仅 views=）。
3. `P123_SHADOW=1` 全程对拍 mismatches=0。
4. 计数对账：`n_asm_confirm`（assemble 统一计算次数）应 == pairs 总数（@250k 69,584；@1M 1,309,501）；`n_tr_confirm`（provide 查表次数）应 == 96.2% pairs——两侧计数不变（单源化不改调用域）。
5. 全量双跑（②验收后）：与②同 dump 协议 diff=0。

## 边界声明

- 本卡只做方案 1；方案 2（窗化）/方案 3（去重）另行同批但改动点独立（divergence.rs/signal.rs/level_view.rs 他处），不交叉。
- `segments`/`anchors` 三建合并（报告 :59）随本卡顺手做与否由实装时定——微税 ~1-2%，不做不欠账。
- p126_split 计时器：B 段统一计算的计时挂在 assemble 侧（`t_asm_confirm` 口径不变）；provide 侧 `t_tr_confirm` 计时行删除（查表 O(1) 无计必要），`n_tr_confirm` 计数保留（对账用）。
