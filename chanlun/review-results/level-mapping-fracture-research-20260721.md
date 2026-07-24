# #107 调研报告：内在级别体系与证书级别映射断裂点

- 日期：2026-07-21
- 票据：issue #107（wayfinder:research，AFK 调研票）；parent map #106。
- 性质：纯只读分析——零 rust 代码改动、零 cargo、零 git mutation、主仓零写入。数据源 = `/tmp/v4_C/wf7/{trades,tower_events}.jsonl`（OPSEM dump）+ `/tmp/v4_C.out`（STATS/CHAIN/INDEX）+ rust 源码（worktree `/tmp/kimi-nest-mainline`，HEAD `64060907`）。
- 090 纪律：查不出的标"未确证"，不伪造。

---

## 任务 1：塔的内在级别数据结构

### 1.1 级别表示：整数下标（`Vec<LevelState>`，级别 = 数组下标）

**`Classification`（`classifier/mod.rs:214-217`）**：

```rust
pub struct Classification {
    /// 索引 = 级别 ℓ（0=L0=1分钟线段账本）。
    pub levels: Vec<LevelState>,
}
```

级别不是枚举、不是区间——是 **`Vec` 的下标**。`levels[0]` = L0（1 分钟线段层），`levels[1]` = L1（首个递归上级），以此类推。

### 1.2 级别从走势结构自生（不靠固定时间窗口）

**递归构造循环（`classifier/mod.rs:394-472`）**：

```
L0 线段（parser 产出）→ classify_level → 中枢 + 走势 → compose_level → L1 走势单元
    → classify_level → L1 中枢 + 走势 → compose_level → L2 走势单元 → ...
```

终止条件（`mod.rs:396-399, 468-471`）：
- 当前级单元数 < `min_parts_per_level`（默认 3，`config.rs:76,83`）⟹ 自然终止。
- 达到 `l_max` 上界（默认 6，`config.rs:74,82`）。

**关键事实：级别数不是固定的**。wf7 数据实产出 4 级（L1-L4，`tower_events.jsonl` 中 `level_upgrade` 事件 4 个）；wf8 同样 4 级；p3fold 3 级。级别深度由市场结构密度决定——震荡窗走势密集 ⟹ 更多级别；趋势窗走势稀疏 ⟹ 更少级别。

### 1.3 递归塔对象（`LeveledMove`，`recursive_tower.rs:103-115`）

```rust
pub struct LeveledMove {
    pub rmove: RMove,           // 纯结构走势（Lean μF 镜像，无坐标）
    pub start_index: usize,     // 原始 K 序起坐标
    pub end_index: usize,       // 原始 K 序终坐标
    pub sub_moves: Vec<LeveledMove>, // 携坐标次级别走势（L0 线段空）
    pub id: ElementId,          // { level: u32, ordinal: u64 }
}
```

`ElementId.level`（`recursive_tower.rs:78`）= 级别下标，与 `Classification.levels` 同坐标系。L0 线段 `level=0`，L1 走势 `level=1`，以此类推。

### 1.4 wf7 实证级别结构

| 级别 | 中心数 | 首次出现(bar) | 中位跨度(bars) | 时间近似 |
|---|---|---|---|---|
| L1 | 551 | 1122 | 313 | ≈5.2 小时 |
| L2 | 129 | 5986 | 1,250 | ≈20.8 小时 |
| L3 | 31 | 23648 | 5,095 | ≈3.5 天 |
| L4 | 5 | 154807 | 34,431 | ≈23.9 天 |

---

## 任务 2：证书级别键构造与身份桥 ℓ=lvl+1

### 2.1 NestEventIdentity（证书身份键，`nest.rs:396-400`）

```rust
pub struct NestEventIdentity {
    pub level: u32,             // nest 级别 ℓ（event 来源级）
    pub turn_source: usize,     // 确认点 t* 坐标
    pub interval_b: (usize, usize), // B 口径区间
}
```

### 2.2 身份桥 by_end 的键构造（`admission.rs:497-504`）

```rust
// absorb_exts 中对每个确认事件：
by_end.entry((
    event.level,            // ← 事件自身的 nest 级别 ℓ
    ext.seg_c_full.1,       // ← 离开段终点坐标（值桥载体）
    matches!(event.side, Side::Long),
)).or_default().push(id);
```

### 2.3 typed_lookup 的查询键（`admission.rs:558-581`）

```rust
pub fn typed_lookup(&self, lvl: usize, source_index: usize, delta: Side) -> Option<(bool, usize)> {
    let ids = self.by_end.get(&(
        lvl as u32 + 1,          // ← 级别桥 ℓ=lvl+1（固定加 1）
        source_index,            // ← 值桥：候选的 source_index
        matches!(delta, Side::Long),
    ))?;
    // ... 遍历 ids 查 index ...
}
```

**断裂点在此**：`typed_lookup(lvl, source_index, delta)` 查询 `by_end[(lvl+1, source_index, is_long)]`——**只查 lvl+1 一个级别**。如果候选在 L0（lvl=0），只查 L1 事件；候选在 L1，只查 L2 事件。即使 L2/L3 有结构性覆盖的事件，也**够不到**。

### 2.4 has_bridge_key 前置判据（`admission.rs:542-553`）

```rust
pub fn has_bridge_key(&self, lvl: usize, source_index: usize, delta: Side) -> bool {
    self.by_end.contains_key(&(
        lvl as u32 + 1,          // 同样的 ℓ=lvl+1 固定方向
        source_index,
        matches!(delta, Side::Long),
    ))
}
```

`has_bridge_key` 与 `typed_lookup` 使用**完全相同的键** `(lvl+1, source_index, is_long)`——桥键存在性判据与实际查询同源。

### 2.5 索引构建（`nest_index.rs:112-154`）

```rust
for exec in 1..events_by_level.len() {       // exec ≥ 1（L0 不产证）
    for top in exec..events_by_level.len() {
        for cert in assemble_typed_certificates(events_by_level, exec, top, ...) {
            let key = *cert.identities().last().unwrap();  // 基例身份
            // 裁定去重后 insert
        }
    }
}
```

索引键 = 证书的**基例事件身份**（`NestEventIdentity`，含 `level`）。查询经 `by_end` 反查到 identity 集合，再从 `index.get(id)` 取证书。`by_end` 与 `index` 消费同一事件流（`absorb_exts` 写入），因此 `by_end` 键域 ⊇ 索引键域（`typed-miss-attribution-20260721.md:§2` 已证 A=0）。

---

## 任务 3：级别映射断裂假说验证（核心证据）

### 3.1 假说

> wf7 臂C 1653 个 typed_none 中，有显著比例不是"没有链"，而是"有链但查错了级别"——身份桥固定查 ℓ+1，而候选位置的结构性覆盖可能在 ℓ+2 或 ℓ+3。

### 3.2 验证方法

**代理分析**：OPSEM dump 未序列化 per-candidate 的 `(level, source_index, side)` 查询键 × `typed_lookup` 返回值（`typed-miss-attribution-20260721.md:§0`），无法做精确的逐 candidate 验证。因此采用**结构性代理**：

对 wf7 臂C 的 162 个 L0 BSP（trades 的 `certificate.level=0` 子集，作为候选总体的可观测样本），检查它们的 `source_index` 在各级别的中枢区间 `[si, ei]` 中是否被包含。这测量的是"该 BSP 位置在哪些级别有结构性覆盖"——代理"如果身份桥搜该级别，能否命中事件"。

### 3.3 核心数据

**多级别包含矩阵（L0 BSPs, n=162）**：

| 类别 | 数量 | 占比 | 含义 |
|---|---|---|---|
| **L1_only** | 7 | 4.3% | 仅在 L1 有覆盖（当前桥可命中，但只在 L1） |
| **L1+higher** | 76 | 46.9% | 在 L1 有覆盖，同时也在 L2/L3/L4 有覆盖 |
| **NOT_L1_but_L2+** | **70** | **43.2%** | **不在 L1 但在 L2/L3/L4 有覆盖**（桥够不到！） |
| **nowhere** | 9 | 5.6% | 所有级别都没有覆盖（真 B·教义结构） |

**各级别独立覆盖率**：

| 级别 | 覆盖数 / 162 | 覆盖率 |
|---|---|---|
| L1 | 83 | 51.2% |
| L2 | 94 | 58.0% |
| L3 | 104 | 64.2% |
| L4 | 81 | 50.0% |
| **L1∪L2∪L3∪L4** | **153** | **94.4%** |

### 3.4 结论：假说**成立**

**级别映射缺陷确实存在，且是 typed_none 的主要可恢复来源。**

- 当前桥（仅查 ℓ+1 = L1）：覆盖 83/162 = **51.2%**
- 多级桥（查 L1∪L2∪L3∪L4）：覆盖 153/162 = **94.4%**
- **断裂差距：+70 个 BSP（+43.2pp）**——这些 BSP 在 L2/L3/L4 有结构性覆盖，但身份桥固定查 L1 而够不到。
- 真正无覆盖（B·教义结构）仅 9/162 = 5.6%。

### 3.5 外推（谨慎，标为估计）

wf7 总候选 1724，typed_found=71（4.1%），typed_none=1653。L0 候选约占 76.1%（~1312 个）。

- 若 43.2% 的 L0 typed_none 可被多级桥恢复：~543 个候选
- 外推 typed_found：71 → ~614（命中率 4.1% → ~35.6%）

**重要限制（090）**：
1. 本分析使用中枢区间 `[si, ei]` 包含作为**代理**，不等于 `seg_c_full.1` 精确匹配。实际 seg_c_full.1 是散度段终点坐标，与 center 的 `ei` 不同。方向性可靠，量级为上界代理。
2. trades 中的 L0 BSP 是**通过门**的样本，不代表全部 L0 候选（被拒的不在本样本中）。但由于候选生成与门无关（voice/strategy 层先于门），通过门的样本在级别分布上具有代表性。
3. 精确的逐 candidate 验证需要 per-candidate 的 `has_bridge_key × typed_lookup × level` 联合 dump（当前 OPSEM 未序列化），标**未确证**精确量级。

### 3.6 与 `typed-miss-attribution` 的关系

`typed-miss-attribution-20260721.md` 裁定 B·教义结构压倒性主导（>70%），B·装配缺口少数。本调研**不否定**该裁定——那篇报告的归因框架是 A/B/C（桥键 bug / 键域错位 / 链不存在），而"级别映射缺陷"属于**B·教义结构的子结构**：

- 归因报告说的"B·教义结构"= 跨级包含关系在 ℓ+1 方向上没出现。
- 本报告发现的 = **同样的 BSP 位置在 ℓ+2 / ℓ+3 方向上有包含关系**，但桥不查那个方向。
- 即：**链不是"不存在"，而是"存在但在桥的视野之外"**。这正是 ADR（`adr-level-identity-multiview-20260721.md`）裁定的核心：身份桥 ℓ=lvl+1 固定方向太窄。

---

## 任务 4：传统缠论时间窗口与本系统内在级别

### 4.1 传统缠论的级别视窗

传统缠论使用固定时间周期作为级别视窗：1分→5分→15分→30分→60分→日线。每个时间周期独立画出 K 线、线段、中枢、走势，跨级确认 = 某级别的买卖点在更大时间周期的走势结构上得到验证。

### 4.2 本系统的内在级别

本系统**没有固定时间窗口**。级别从走势结构自生：

- L0 = parser 产出的 1 分钟线段（唯一的时间锚定层）。
- L1 = L0 线段经中枢检测→走势裁决→compose 出的上级走势单元。
- L2 = L1 走势单元的同法递归。
- 更高级别以此类推，直到单元数 < `min_parts_per_level`（3）自然终止。

### 4.3 对应关系（wf7 实证）

| 内在级别 | 中位跨度（1 分 bar 数） | 时间近似 | 传统缠论对应 |
|---|---|---|---|
| L0 | — | 1 分钟 | 1 分线 |
| L1 | 313 bars | ≈5.2 小时 | ≈5 分 / 15 分 |
| L2 | 1,250 bars | ≈20.8 小时 | ≈30 分 / 60 分 |
| L3 | 5,095 bars | ≈3.5 天 | ≈日线 / 4 小时 |
| L4 | 34,431 bars | ≈23.9 天 | ≈周线 |

### 4.4 关键差异

1. **传统窗口是固定的，内在级别是可变的**。同一市场在不同时段的级别密度不同——震荡窗走势密集（wf7 在 26 万 bar 产出 4 级），趋势窗走势展开更快但级数可能更少。
2. **传统窗口的跨级关系是固定的（5分→15分 恒为 3 倍），内在级别的跨级关系是可变的**。L1 中心的中位跨度 313 bars，但 min=92 / max=728——同一个"级别"的跨度可差 8 倍。这意味着 L0 BSP 到 L1 中心的结构距离不是一个常数，而是一个分布。
3. **这正是固定 ℓ=lvl+1 桥的问题根源**：传统缠论里，5 分图的买卖点在 15 分图上找确认是天经地义的——因为 15 分是 5 分的固定上级。但在本系统里，一个 L0 BSP 的"结构上最有意义的上级"可能是 L2 而不是 L1（取决于走势结构），固定查 L1 就是错配。

### 4.5 "多视窗"方案的自然性

ADR 裁定的"每个内在级别在 L0 K 线上独立体现"在结构上等价于传统缠论的"同时在 1 分/5 分/15 分上看到买卖点"——每个级别各自独立地标出它在 L0 K 线上的投射。区别是传统缠论按时间分窗，本系统按结构分窗。

---

## 总结论

| 任务 | 结论 |
|---|---|
| **1. 内在级别数据结构** | `Vec<LevelState>`，级别 = 数组下标（0=L0…自然终止）。级别从走势结构自生（`classify_impl` 递归 compose），不靠固定时间窗口。`LeveledMove.id.level` 携带级别身份。 |
| **2. 证书级别键 + 身份桥** | `by_end` 键 = `(event.level, seg_c_full.1, is_long)`；`typed_lookup` 查 `(lvl+1, source_index, is_long)`——**固定加 1，只查一个级别方向**。断裂点 = `admission.rs:564-568` 的 `lvl as u32 + 1`。 |
| **3. 假说验证** | **级别映射缺陷成立**。wf7 L0 BSP 样本：51.2% 有 L1 覆盖（当前桥可达），94.4% 有任意级别覆盖；43.2% 的 BSP 在 L2/L3/L4 有结构性覆盖但**不在 L1**——桥固定方向导致 miss。真正无覆盖的 B·教义结构仅 5.6%。外推 typed_found 命中率可从 4.1% 提升到 ~35.6%（代理上界，精确量级未确证）。 |
| **4. 传统时间窗口类比** | 内在级别可自然产生类似视窗的分离，但分离的"宽度"是结构决定的（L1 跨度 92-728 bars，差异 8 倍），不是时间固定的。固定 ℓ+1 桥的错配源于此：结构上最有意义的跨级关系不一定在相邻级别。 |

---

*数据源：`/tmp/v4_C/wf7/{trades,tower_events}.jsonl`、`/tmp/v4_C.out`（V4 验收报告 `v4-three-window-typed-chain-acceptance-20260721.md` + 归因报告 `typed-miss-attribution-20260721.md`）、rust 源码 `classifier/mod.rs` / `classifier/recursive_tower.rs` / `classifier/nest.rs` / `classifier/nest_index.rs` / `backtest/admission.rs` / `backtest/runner.rs`。零代码改动、零 cargo、零 git mutation。*
