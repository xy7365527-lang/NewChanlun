# wave-1 实装卡：方案 3 盘背去重 O(n²)→sort+dedup（2026-07-19）

**状态**：待②验收后执行（与方案 1/2 同批）。设计依据：`optimize-wave1-research-20260719.md` 方案 3（§2.4 provide 残余纯税）。
**收益** [实测残余归因]：~17-18s@1M（prefix ~7.7%）。**难度**：小（两处共 ~6 行）。**风险**：低。

## 现状（`level_view.rs:798-808`）

- :798-800 循环内 `if !out.contains(&event) { out.push(event); }`——每盘背事件对 `out`（~154 趋势 + 已积盘背）线性扫，L1 @1M = 1,447 次/view × 平均扫 ~800 事件 ≈ 2.3ms/view ≈ 残余 19.4s 主体。
- :802-807 已有 `out.sort_by_key(|event| (event.turn_source, event.interval_b, event.kind, matches!(event.side, Side::Short)))`（全输出排序，趋势+盘整）。
- 实测旁证（报告 §2.4）：`pan_ev == n_div` 逐位相等（@250k 660,996==660,996；@1M 12,290,012==12,290,012）——contains 在生产数据**零拦截**，是纯税。

## bit-exact 论证

`NestCandidateEvent` 的 PartialEq = 派生全字段比较。全等事件 ⟹ 所有字段相等 ⟹ 排序键（4 字段）相等 ⟹ 稳定排序后相邻 ⟹ `dedup_by(PartialEq)` 保留首次出现 = contains 的「首插胜出」语义；非全等事件（排序键相等但其他字段不同）各自保留、相对序由稳定排序保持 = 现行为。输出逐字一致（构造性证明 + 实测零拦截旁证）。

## 改动清单（`level_view.rs` 两处）

### A. 循环内无条件 push（:798-800）

前：
```rust
        if !out.contains(&event) {
            out.push(event);
        }
```

后：
```rust
        // ★wave-1 方案 3：无条件 push，去重移至 sort 后 dedup_by（O(n²)→O(n log n)）。
        out.push(event);
```

### B. sort 后接 dedup_by（:807 后插入）

前：
```rust
    out.sort_by_key(|event| (
        event.turn_source,
        event.interval_b,
        event.kind,
        matches!(event.side, Side::Short),
    ));
    out
```

后：
```rust
    out.sort_by_key(|event| (
        event.turn_source,
        event.interval_b,
        event.kind,
        matches!(event.side, Side::Short),
    ));
    // ★wave-1 方案 3：dedup_by(PartialEq) 替代循环内 out.contains。
    // 全等事件 ⟹ 排序键相等 ⟹ 稳定排序后相邻 ⟹ 保留首次出现 = contains 首插胜出语义（构造性证明）；
    // 实测旁证：contains 在生产数据零拦截（pan_ev == n_div 逐位相等 @250k/@1M）⟹ 输出逐字一致。
    out.dedup_by(|a, b| a == b);
    out
```

## 验证协议（与方案 1/2 同批）

1. `cargo test --lib` 全绿零变红。
2. 250k/1M stdout+dump 对同码基线 diff=0（白名单仅 views=）。
3. `P123_SHADOW=1` 全程对拍 mismatches=0。
4. 计数对账：pan 事件数（P*_YIELD pan_candidates/pan_divergence）与基线逐位一致——dedup 零拦截预期与 contains 相同。
5. 全量双跑（②验收后）：与②同 dump 协议 diff=0。

## 边界声明

- `out.dedup_by(|a, b| a == b)` 的闭包 `(a, b)`：a = 已保留末元素，b = 当前考察元素；a==b ⟹ b 移除（保留首次出现）——与 contains 的「已存在则不插」逐项对应。
- 本卡只动 provide_nest_candidate_events 的盘背事件段；趋势事件段（:646 起）无 contains 模式，不受本卡影响。
- 与方案 1/2 同批：三卡改动点互不交叉（方案 1 在 LevelAsOfView/两处调用点，方案 2 在 divergence.rs/signal.rs/level_view.rs 他处，本卡在 :798-808），可同一 commit 或分 commit，验证协议同一套。
