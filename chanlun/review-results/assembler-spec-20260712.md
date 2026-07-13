# 走势类型组装器：消费层 as-of 纯函数视图（task #58）

**日期**：2026-07-12

**状态**：Rust 原型 + 硬门测试

**实现**：`rust/src/theta_v0/classifier/move_view.rs`

**生产接入**：无。模块只注册公开 API，`classify`、`recursive_tower`、`judge_third_cert`、信号与交易路径均无调用。

## 1. 边界与动机

递归塔的 `LeveledMove::Compose` 是不可变规范窗口单元：一个窗口只封装连续三段及其一个中枢。它是可靠的中枢种子，不等于完整走势类型。完整走势的“连续同向窗口合并、中枢延伸、被后续走势终结、末段背驰”是消费时、给定观察前缀才有意义的判定，因此放在纯函数视图层按需计算，不回写塔。

本任务同时保持以下边界：

- 不改变塔的三段非重叠窗口扫描，也不让塔提前吸收延伸或裁决趋势；
- 不改变生产分类、买卖点、证书或信号路径；
- 不把未来尾部读进 `as_of=t` 的结果；
- 不把塔游标失败分支 `i += 1` 跳过的下层段静默丢掉；
- 不预设待裁决的 A/C 自动配对规则。配对是显式 hook，MACD 面积比较是真计算。

## 2. 函数签名

Rust 原型：

```rust
pub fn assemble_move_view(
    material: MoveViewMaterial<'_>,
    level: u32,
    coordinate_window: CoordinateWindow,
    as_of: usize,
) -> MoveView;
```

其中：

```text
MoveViewMaterial = {
  tower_windows: &[TowerWindowUnit],       // 塔的三段+一中枢规范窗口适配值
  lower_ledger: &[LowerComponent],         // 完整下层不可变账本；最坏为 L0 Segment
  divergence: Option<MacdDivergenceMaterial>
}

MacdDivergenceMaterial = {
  hist: &[f64],
  close_src: &[source_index],
  pairs: &[DivergencePairMaterial]         // 外部 A/C 配对 hook
}
```

`TowerWindowUnit::from_leveled` 消费真实 `LeveledMove::Compose`。塔对象没有已结算走势方向，因此适配时方向必须显式传入；禁止把 `fold_direction` 的首项占位值冒充定义裁决。`LowerComponent::from(&Segment)` 是最坏 L0 账本路径。

## 3. as-of 输入域与纯函数契约

令请求窗口为闭区间 `W=[w₀,w₁]`，观察时点为 `t`。可见材料定义为：

```text
Visible_t(X) = stable_sort { x ∈ X |
  w₀ ≤ start(x) ≤ end(x) ≤ min(w₁,t)
}
```

组装器只读取 `Visible_t(tower_windows)`、`Visible_t(lower_ledger)`，背驰配对还要求 `end(C)≤t`。函数无 I/O、全局状态、缓存或可变外部引用。

因此前缀稳定性完成条件为：

```text
∀D, F, t, k≥0.
assemble(D[≤t], F[≤t], t) = assemble(D[≤t+k], F[≤t+k], t)
```

等号是 `MoveView` 全字段相等：完成状态、构成链、中枢延伸快照、孤儿来源、证据坐标及 MACD 背驰布尔均相等。

## 4. 组装语义

### 4.1 连续同向窗口合并

将可见塔窗口按 `(start_index,end_index)` 稳定排序。对相邻窗口作 maximal run 分组：

```text
G = maximal consecutive [u_i ... u_j]
    such that direction(u_n) = direction(u_i), i≤n≤j
```

每个 `G` 形成一个 `AssembledMove` 候选。第一个后继异向组的起点既是当前组的坐标右边界，也是盘整“被后续走势终结”的证据。

### 4.2 孤儿段补全

对组 `G`，构成链不取 `concat(window.subs)`。定义首组左界为查询窗口左界（补回首个成功塔窗口之前的 `+1` 跳段），后续组左界为其首窗口起点：

```text
left(G) = w₀                         if G is first group
        = start(first_window(G))    otherwise

Chain(G,t) = { s ∈ Visible_t(lower_ledger) |
  left(G) ≤ start(s) ∧ end(s) < start(next_group)
}
```

若无后继组，右界为 `min(w₁,t)`。对每个链元素：

- 完全落在 `G` 任一塔窗口坐标内：`TowerWindow`；
- 否则：`OrphanFromLowerLedger`。

这解决塔滑窗失败分支 `i += 1` 的信息缺口：被跳过段仍存在于下层账本，按坐标进入完整构成链。仅拼上级窗口单元的实现不满足本规范。

### 4.3 中枢延伸吸收

每个塔窗口的中枢 `z=(ZD,ZG,DD,GG)` 是种子。自种子第三段之后扫描下层账本，直到下一中枢种子或下一走势组：

```text
Absorb(z,s)  iff  high(s) ≥ ZD(z) ∧ low(s) ≤ ZG(z)
GG' = max(GG, high(s))
DD' = min(DD, low(s))
end' = end(s)
```

首个不满足 `Absorb` 的段记为 `break_at` 并停止。这与 `rust/src/zhongshu.rs` 的 `extend_zhongshu` / `ScanResume::Unsettled` 在 **ZD/ZG 不变、gg/dd 累积、首个不重叠段终止** 上同口径。

诚实差异：本模块是批量 as-of 纯函数，每次从可见前缀重算，不复用 `ScanResume` 的增量缓存状态；且为避免一个种子越过下一个规范中枢，扫描上界额外截在下一种子之前。它声明语义等价，不声明状态机复用。

### 4.4 类型与完成判定

对组内延伸后中枢序列 `Z=[z₀...zₙ]`：

```text
Consolidation(G) := |Z| = 1

SameDir(Up,Z)   := ∀i, DD(zᵢ₊₁) > GG(zᵢ)
SameDir(Down,Z) := ∀i, GG(zᵢ₊₁) < DD(zᵢ)

Trend(G) := |Z| ≥ 2 ∧ SameDir(direction(G), Z)
```

完成条件：

```text
CompletedConsolidation(G,t)
  := Consolidation(G) ∧ ∃ next_move, start(next_move) ≤ t

CompletedTrend(G,t)
  := Trend(G) ∧ PairHook(G,t)=Some(A,C)
              ∧ MapToHist(A,C)=Some(A',C')
              ∧ MACDArea(C') < MACDArea(A')
```

完成输出：

- 盘整：`Completed { as_of:t, SubsequentMove { at } }`；
- 趋势：`Completed { as_of:t, TerminalDivergence { seg_a, seg_c } }`；
- 否则显式 `Pending { as_of:t, reason }`。

`PendingReason` 区分等待后续走势、多个中枢方向混合、缺 A/C 配对、source→hist 坐标不可映射、末段不背驰等，不把“不知道”折叠为伪 `false` 完成。

## 5. 背驰 hook 与 C 裁决兼容性

本阶段不在组装器内部发明 A/C 定位。`DivergencePairMaterial` 是显式 hook：调用方提供候选走势起点及 A/C source 坐标。本模块用 `recursive_tower::map_src_to_close_idx` 映射到 MACD 序列，再调用 `classifier/divergence.rs::segments_diverge`，按严格 `area(C)<area(A)` 真算。

因此：

- C 裁决选择任一合法配对方向，只需改变 hook 产物，组装/孤儿/延伸语义不变；
- 没有 hook 时趋势只能 `Pending(MissingDivergencePair)`；
- **still-MISSING**：从走势构成链和中枢自动生成唯一 A/C 配对的裁决与生产接线。本任务不伪造、不默认。

## 6. 输出与证据

`MoveView` 携带：

- `level / coordinate_window / as_of`；
- 每个候选走势的 `direction / [start,end] / kind / completion`；
- 完整 `chain`，逐段标注塔覆盖或下层孤儿补全；
- 每个中枢的 seed、延伸 `end/gg/dd`、吸收段坐标和 `break_at`；
- `evidence_coordinates`：判定实际读取的构成段、中枢延伸及 A/C 坐标。

所有结构字段使用整数 tick/source_index；只有现有 MACD 力度函数位于隔离的 `f64` 域。

## 7. 认识论标注（formalization-validity-domain 231号）

### L0：结构/定义性质

- `Visible_t` 前缀裁剪和纯函数确定性；
- 连续同向窗口 maximal-run 合并；
- 从下层账本按坐标补孤儿；
- 中枢重叠吸收条件及 `gg/dd/end` 累积；
- 盘整/趋势的中枢计数、外缘同向关系；
- 给定 A/C 和 MACD 序列后的严格面积比较算术；
- `Completed/Pending as-of t` 的证据不可越过 `t`。

这些是给定形式化口径后的结构性质，不声明市场有效性。

### L1：管线验证

- Rust 在线式 as-of 纯函数与独立批处理 oracle 逐字段 bit-exact；
- 未来超集输入在固定 `as_of` 下不改变输出；
- 合成样例命中延伸吸收点并恢复塔 `+1` 跳过的孤儿；
- `cargo test` 证明当前代码路径实现了上述契约。

L1 只说明实现/管线一致，不证明走势分类、MACD 背驰或交易信号在真实行情上的 L2/L3 有效性。

## 8. 硬门与完成条件

1. `prefix_stability_at_extension_boundaries`：固定 `t` 比较短前缀与未来超集，采样 `t=29,39,49,69,79,109`；`t=39` 覆盖延伸段被吸收后的边界。
2. `offline_batch_oracle_is_bit_exact`：独立批处理实现不调用生产组装 helper，多前缀对 `MoveView` 全字段 `assert_eq!`。
3. `orphan_is_recovered_and_center_extension_uses_scan_resume_overlap` + `leading_cursor_skip_is_recovered_from_coordinate_window`：断言窗口间及首窗口前的孤儿来源、吸收坐标、`gg/dd` 和真实 MACD 完成证据。
4. 工作树全量 `cargo test` 通过，`git diff --check` 通过。
5. `rg` 审计确认 `move_view` 无生产调用方；禁改函数和塔文件没有 diff。

任一门失败则 task #58 未完成。
