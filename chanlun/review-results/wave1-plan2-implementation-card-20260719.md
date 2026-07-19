# wave-1 实装卡：方案 2 判据窗化（a-e 五处，2026-07-19）

**状态**：待②验收后执行（与方案 1 同批）。设计依据：`optimize-wave1-research-20260719.md` 方案 2（§2.2 全切片扫描点）。
**收益** [实测靶叶 + 推演削减]：靶叶 ≈ 85.4s@1M（pan_ext 73.2 + tr_env 7.9 + tr_conf 内 a_env ~4.3 + 小额），窗化后削减 ≥90% ≈ **省 ~77-81s@1M（prefix 34-36%）**；主靶叶 ∝ S²，4.6M 收益更大。
**难度**：小-中（五处局部改写，每处 ≤10 行）。**风险**：低-中——全部依赖「Segment 序列 start_index/end_index 双升序」前提（塔不变量，`recursive_tower.rs:118-121` partition_point 先例；`recursive_tower.rs:124` debug 断言守护）。

## bit-exact 总论证（五处同构）

段序列 start/end 双升序 ⟹ 对谓词 `P(s)`（含 start/end 界条件）：
- 界外段（start 或 end 越界）⟹ 界条件恒假 ⟹ P 恒拒——排除正确；
- 界内段 ⟹ 保持原 filter/fold/find 与同一顺序 ⟹ 结果逐字一致。
逐处展开见各节；(a) 的完整展开：start > span.1 ⟹ end > start > span.1 ⟹ `end_index <= span.1` 恒失败（双升序使 `start ≤ span.1` 是单调后缀补、`end ≤ span.1` 是单调前缀，两界都可用 partition_point 封口）。

## 改动清单（五处）

### (a) `divergence.rs:864-876` `move_range_envelope`（单一来源，三调用点共用）

前：
```rust
pub(crate) fn move_range_envelope(segments: &[Segment], span: (usize, usize)) -> Option<(Tick, Tick)> {
    segments
        .iter()
        .filter(|segment| segment.start_index >= span.0 && segment.end_index <= span.1)
        .fold(None, |acc, segment| { ... })
}
```

后：
```rust
pub(crate) fn move_range_envelope(segments: &[Segment], span: (usize, usize)) -> Option<(Tick, Tick)> {
    // ★wave-1 方案 2(a) 窗化：start/end 双升序（塔不变量）⟹ 界化窗口恰好包含满足
    // start>=span.0 ∧ end<=span.1 的同一元素集同一顺序（界外恒拒：start>span.1 ⟹ end>span.1 恒失败）。
    // 窗内保持原 filter+fold 逐字——fold 结果不变（bit-exact 构造性论证见实装卡 §总论证）。
    let lo = segments.partition_point(|s| s.start_index < span.0);
    let hi = segments.partition_point(|s| s.start_index <= span.1);
    segments[lo..hi]
        .iter()
        .filter(|segment| segment.start_index >= span.0 && segment.end_index <= span.1)
        .fold(None, |acc, segment| { ... })
}
```

（`hi` 用 `start <= span.1` 而非 `end <= span.1`——前者是更紧的上界且单调性直接由 start 升序给；窗内 filter 的 `end <= span.1` 条件保留，两条件合取与原全切片逐项等价。filter 条件保留不改，只是把全切片换成 [lo..hi]。）

### (b) `signal.rs:750-762` `pan_div_structure_extreme` 局部 envelope 闭包

前：
```rust
    let envelope = |span: (usize, usize)| {
        segments
            .iter()
            .filter(|segment| segment.start_index >= span.0 && segment.end_index <= span.1)
            .fold(None, |acc: Option<(Tick, Tick)>, segment| { ... })
    };
```

后：
```rust
    // ★wave-1 方案 2(b) 窗化：同 (a) 形态（双升序界化，窗内保持原 filter+fold）。
    let envelope = |span: (usize, usize)| {
        let lo = segments.partition_point(|s| s.start_index < span.0);
        let hi = segments.partition_point(|s| s.start_index <= span.1);
        segments[lo..hi]
            .iter()
            .filter(|segment| segment.start_index >= span.0 && segment.end_index <= span.1)
            .fold(None, |acc: Option<(Tick, Tick)>, segment| { ... })
    };
```

**顺手项**：(a)(b) 是同一原语形态（报告 :122「(a)(b) 是同一原语形态」）——(b) 可改为调用 `move_range_envelope`（divergence.rs 单一来源，675 号纪律），但 (b) 的 fold 闭包类型签名与 (a) 相同（Option<(Tick,Tick)>），直接调用可消除重复。实装时优先合流：(b) 的 envelope 闭包体直接 `move_range_envelope(segments, span)`。若合流则 (b) 窗化随 (a) 自动获得，无需重复改。

### (c) `signal.rs:735-738` `locate_pan_div_structure_front_anchor` 的 A′ 反查

前：
```rust
    let a_prime = segments
        .iter()
        .rev()
        .find(|s| s.direction == dir && s.end_index <= c.start_index)?;
```

后：
```rust
    // ★wave-1 方案 2(c) 窗化：end 升序 ⟹ end <= c.start_index 是单调前缀，partition_point 封口；
    // [..hi] 内所有段 end <= c.start_index 恒真 ⟹ find 条件只剩 direction；
    // [..hi] 末端 rev 扫与全切片 rev 扫命中同一「中枢前最近同向段」（界外段 end>c.start_index 恒拒）。
    let hi = segments.partition_point(|s| s.end_index <= c.start_index);
    let a_prime = segments[..hi].iter().rev().find(|s| s.direction == dir)?;
```

### (d) `level_view.rs:839-845`（c_terminal 前向 find）与 `:860-868`（c_end 反向 find）

c_terminal 前：
```rust
        let Some(c_terminal) = segments.iter().find(|segment| {
            segment.direction == direction
                && segment.start_index >= last.end_index
                && segment.end_index <= as_of
        }) else {
            continue;
        };
```

c_terminal 后：
```rust
        // ★wave-1 方案 2(d) 窗化：双升序 ⟹ [lo..hi] 内 start>=last.end_index ∧ end<=as_of 恒真，
        // find 条件只剩 direction；界外段（start<last.end_index 或 end>as_of）恒拒同原谓词。
        let lo = segments.partition_point(|s| s.start_index < last.end_index);
        let hi = segments.partition_point(|s| s.end_index <= as_of);
        let Some(c_terminal) = segments[lo..hi].iter().find(|segment| {
            segment.direction == direction
        }) else {
            continue;
        };
```

c_end 前：
```rust
        let Some(c_end) = segments
            .iter()
            .rev()
            .find(|segment| {
                segment.direction == direction
                    && segment.start_index >= c_start
                    && segment.end_index <= as_of
            })
            .map(|segment| segment.end_index)
        else {
            continue;
        };
```

c_end 后：
```rust
        // ★wave-1 方案 2(d) 窗化：同构（[lo..hi] 内两界恒真，rev 扫命中同一末个同向段）。
        let lo = segments.partition_point(|s| s.start_index < c_start);
        let hi = segments.partition_point(|s| s.end_index <= as_of);
        let Some(c_end) = segments[lo..hi]
            .iter()
            .rev()
            .find(|segment| segment.direction == direction)
            .map(|segment| segment.end_index)
        else {
            continue;
        };
```

（`lo <= hi` 守护：last.end_index ≤ as_of 由塔因果性保证（塔只含 ≤ 当前 bar 的走势），c_start ≥ last.end_index 由 :846 departure_move_c_start 语义保证；两处 lo ≤ hi 恒成立。若未来不变量松动，slice[lo..hi] lo>hi 会 panic——补 `if lo > hi { continue; }` 一行守护，实装时加。）

### (e) `level_view.rs:753-759` 盘整支 `blocks.iter().position`

前：
```rust
        let Some(leave_index) = blocks.iter().position(|block| {
            block.kind == MoveKind::Consolidation
                && block.start_center <= center_index
                && block.end_center >= center_index
        }) else {
            continue;
        };
```

后：
```rust
        // ★wave-1 方案 2(e) 窗化：blocks 按 start_center 升序 ⟹ start_center <= center_index
        // 是单调前缀，partition_point 封口；[..hi] 内找首个 Consolidation ∧ end_center >= center_index。
        // （小额叶 2.2s@1M；[..hi] 内 start_center<=center_index 恒真，原条件保留无害。）
        let hi = blocks.partition_point(|block| block.start_center <= center_index);
        let Some(leave_index) = blocks[..hi].iter().position(|block| {
            block.kind == MoveKind::Consolidation
                && block.start_center <= center_index
                && block.end_center >= center_index
        }) else {
            continue;
        };
```

（position 返回值是 [..hi] 内索引，与原全 blocks 索引一致（[..hi] 是前缀）⟹ 下游 leave_index 消费零改动。）

## 验证协议（与方案 1 同批）

1. `cargo test --lib` 全绿零变红；另对 (a) 跑 `strict_nest_check`（037:20 生产判据同原语消费）。
2. 250k/1M stdout+dump 对同码基线 diff=0（白名单仅 views=）。
3. `P123_SHADOW=1` 全程对拍 mismatches=0。
4. 双升序前提守护：`recursive_tower.rs:124` debug 断言已锁；本卡五处全部依赖该不变量，若未来塔结构变更须先复核本卡。
5. 全量双跑（②验收后）：与②同 dump 协议 diff=0。

## 边界声明

- (b) 的合流选项（改调 `move_range_envelope`）优先于独立窗化——消除重复代码且自动获得 (a) 的窗化；若合流则本卡 (b) 节作废，只记合流改动。
- 方案 2(c) 前锚窗化的实测份额未分叶（pan_loc 24.9s 内窄锚/前锚同计时器合并），净收益按保守侧计入；若实装后超保守估计，归入方案 2 盈余，无需复议（报告 §5 移交项）。
- 本卡与方案 1（单源化）同批但改动点独立（方案 1 在 LevelAsOfView/两处调用点，本卡在 divergence.rs/signal.rs/level_view.rs 他处），可同一 commit 或分 commit，验证协议同一套。
