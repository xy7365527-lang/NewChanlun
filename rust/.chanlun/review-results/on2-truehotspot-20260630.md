# O(n²) 真热点定位报告（Task #106，实测驱动）

**认识论等级**：L2（真实 BTC 数据三窗口阶段计时，可产否定性结果——两次预设根因已被实测证伪）。
**方法**：阶段计时插桩（`stage_profile` thread_local 累加器，env `THETA_PROFILE_STAGES` gated，
只测时间不改逻辑，bit-exact 安全），三窗口 100K/200K/400K BTC 真实 bar 跑 `classify_with_tower_incremental`。
**异质审计**：codex（read-only sandbox）审查增量化方案 bit-exact 语义。

---

## 1. 结论：真热点是 macd_incremental + l0_tower_rebuild（占 87%）

### 阶段计时三窗口实测（ms）

| 阶段（源码行 mod.rs） | 100K | 200K | 400K | 200→400 比 | 标度 |
|----------------------|------|------|------|-----------|------|
| **02_macd_incremental** (864→649) | 1181 | 4733 | **18286** | **3.86x** | **O(n²)** |
| **01_l0_tower_rebuild** (860) | 367 | 1408 | **5957** | **4.23x** | **O(n²)** |
| 03_frontier_compare (934) | 80.6 | 284 | 1161 | 4.08x | O(n²) 小 |
| 07b_extract_second (1015) | 42.8 | 174 | 711 | 4.08x | O(n²) 小 |
| 04_cached_units_copy (953) | 36 | 142 | 684 | 4.82x | O(n²) 小 |
| 08_levels_centers_clone (1039) | 30.4 | 92 | 369 | 4.01x | O(n²) 小 |
| 10_projected_units_clone (1048) | 26.8 | 83 | 358 | 4.31x | O(n²) 小 |
| 07a_extract_signals_l0 (1011) | 9.96 | 40 | 183 | 4.58x | 小 |
| 05_compose_resume (959) | 13.9 | 35.8 | 106 | 2.96x | 接近线性 |
| **09_project_to_units_resume (#106 改点)** (1047) | 7.36 | 17.7 | 44.2 | **2.50x** | **近线性** |
| 06_extend_centers_upper (974) | 7.41 | 16.1 | 37 | 2.30x | 线性 |

**macd + l0_tower = 400K 总计时的 87%（24.2s / 27.9s）。** 三窗口一致 ~4x（bar×2）= 确证 O(n²)。

### 两次误判的实测证伪
- **#106 改点（project_to_units_resume）= 09 阶段，2.50x 近线性**，44ms（占 400K 的 0.16%）。前缀缓存修对了一个真线性化，**但它本就不是热点**（与 profiler 1 样本一致）。误修非热点确证。
- decompose 路径走 `classify_at`→`classify_with_tower_incremental`，自耗 6651 样本中真正的 O(n²) 在 **02/01 两个 per-bar 全量操作**（不在级别循环内），profiler 因 inline 把它们的自耗归到了 `classify_with_tower_incremental` 函数体（未归因的 ~6300 样本）。

---

## 2. 源码级根因

### 根因 A（macd，~80%）：每 bar O(n) 前缀比较 + 全量克隆

`compute_macd_hist_incremental` (mod.rs:649) 的 O(1) 快路 (line 680)：
```rust
if cache.macd_state.is_some()
    && closes.len() == cached_len            // ← per-bar 每 bar closes.len()+1 ⟹ 永远 false
    && closes.last() == cache.macd_closes_prefix.last()
{ return; }
```
per-bar 回测每 bar 新增 1 bar ⟹ `closes.len() == cached_len` **永不命中** ⟹ 每 bar 走增量路 (line 694)：
- line 692 `closes[..cached_stable] == cache.macd_closes_prefix[..cached_stable]` — **O(n) 比较每 bar**
- line 708 `cache.macd_closes_prefix = closes.to_vec()` — **O(n) 全量克隆每 bar**

两者 ×n bar = **O(n²)**。`macd_closes_prefix` 唯一用途是检测前缀 inclusion 改写，每 bar 全量重存是冗余。

### 根因 B（l0_tower，~20%）：每 bar 全量重建 L0 走势塔

mod.rs:860（stage_profile 包裹后行号下移；原始 line 778）：
```rust
let l0_units: Vec<UnitRange> = l0.segments.iter().map(segment_to_unit).collect();  // O(segs) 每 bar
let moves_tower_l0: Vec<LeveledMove> = l0_units.iter().enumerate()
    .map(|(i, u)| LeveledMove::from_unit(u, ElementId { level: 0, ordinal: i as u64 }))
    .collect();   // O(segs) 每 bar，无缓存 ⟹ O(segs)×n = O(n²)
```
segments 前缀 confirmed 不可变（仅尾段古怪线段重划改写），但每 bar 从头全量 map。

---

## 3. 增量化方案（codex 异质审计裁决）

### 方案 A（macd）：codex 判「按朴素描述不 bit-exact」，给出修正

**漏洞**：`cache.closes[..len-1]` 严格不可变的契约**不成立**——`update_closes_cache` 自己承认
`only_open_tail` 方向翻转会重写 merged_prefix（前缀任意位置 `k < len-1` 可改写）。MACD 侧只比尾 close
会漏检：旧 macd_state 已含旧 `closes[k]`，从 cached_stable=m-1 续推必然污染后续 hist。

**bit-exact 修正（codex 推荐）**：让 `update_closes_cache` 返回 **verdict** `{Unchanged | PrefixStable | PrefixRewritten}`
（或维护 `close_prefix_epoch`）。MACD 存自己见过的 epoch：
- `PrefixRewritten` ⟹ 清 MACD state + 全量重建（bit-exact 退化）。
- `PrefixStable`/`Unchanged` ⟹ 才允许用 `(macd_cached_len, macd_cached_tail)` O(1) 续推。

收益：消除 macd 侧**重复**的 O(n) 前缀比较（复用上游 update_closes_cache 已做的 O(stable) 校验结果，
而非独立再比一次）+ 消除 `macd_closes_prefix.to_vec()` 全量克隆。上游 update_closes_cache 的 O(stable)
校验仅在 miss 时付（频次 ≈ segments 变化次数 ≪ n），不是 O(n²)。

**注意**：close_src 坐标映射要求 update_closes_cache 的 verdict 同时反映 `(close, source_index)` 两者一致
（codex 3a/3b 已锚）——verdict 的 PrefixStable 判定须比 close 与 source_index。

### 方案 B（l0_tower）：codex 判「用正确 dirty 边界可 bit-exact」

缓存 `cache.l0_units` + `cache.moves_tower_l0`（Rc），增量维护（与 update_closes_cache 同构）：
- **前缀稳定性比对字段**（codex 裁决）：`(direction, start_index, end_index, lo, hi)`（lo=min(start_price,end_price),
  hi=max(start_price,end_price)）。保守用完整 `Segment{direction,start_index,end_index,start_price,end_price}`
  也正确（仅可能多重建）。
- **只比 segments.len() 不 bit-exact**（bar 1464 seg[9] end 1384→1170，段数不变内点改写——len guard 会复用
  旧 end_index=1384，全量语义是 1170，后续 center/BSP/坐标全漂）。
- **suffix 重建 ordinal 必须用全局索引 `i`**（k, k+1, ...），不是 tail-local enumerate（否则 ElementId 漂，
  破跨 bar 身份稳定）。前缀稳定 ⟹ ordinal 不变（codex 确认）。

---

## 4. 边界条件（结论翻转条件）

- **结论翻转**：若数据集 segments 变化频次接近 O(n)（高波动标的每 bar 都重划线段），则 update_closes_cache
  / l0_tower 增量校验每 bar miss → 退化全量 → O(n²) 不消除。BTC 300K 实测 segments 变化 ≪ n（maxseg~134），
  增量有效；高频重划标的需 L3 交叉验证才能声明普适。当前结论有效域 = BTC 单标的（L2）。
- **方案 A 翻转**：若 update_closes_cache 的 verdict 实装无法区分 PrefixStable/PrefixRewritten（某些
  inclusion 边界移动既非纯尾改写也非整段重写），则 verdict 退化为永远 PrefixRewritten → 全量重建 → 不优化
  （但仍 bit-exact）。
- **未实装**：本报告只定位 + 设计，**未改 classifier 核心**。方案 A/B 的实装是下一步，须 Lead 决策是否
  改 update_closes_cache 签名（返回 verdict = 接口变更，影响所有调用方）。

---

## 5. 影响声明

- **已改**：mod.rs 加 `stage_profile` 模块（profile-only，env gated）+ classify_with_tower_incremental
  内 10 处计时包裹（闭包包裹表达式，零 bit-exact 风险）；econ_positive.rs 测试末尾加 dump 调用。
  全量 `cargo test --lib` = **1303 passed, 0 failed**（插桩未破 bit-exact）。
- **方案 A 若实装**：改 `update_closes_cache` 返回 verdict（**接口变更**）+ TowerCache 移除 `macd_closes_prefix`
  字段、加 `macd_cached_len`/`macd_cached_tail`/epoch。影响 MACD 背驰下游（hist 数组 bit-exact 不变）。
- **方案 B 若实装**：TowerCache 加 `l0_units`/`moves_tower_l0` 缓存字段 + dirty 边界检测。影响所有读
  tower_snapshots[0] 的下游（runner/interp/l3，内容 bit-exact 不变）。
- **谱系引用**：本定位推翻 mod.rs:794-807 注释「O(n²) 真因在 classify_at 每 bar O(tree) 续算」的部分归因
  ——真因不是 O(tree) 续算（compose_resume/extend 都近线性），而是 **per-bar 全量操作**（macd 前缀克隆 +
  l0 tower 重建）。formalization-validity-domain：旧注释的「O(tree)/bar」声明有效域 < 定义域。

---

## 6. 待 Lead 决策

方案 A 改 update_closes_cache 签名（返回 verdict）= 接口变更，影响调用方。这是「选择」类
（朴素 macd 侧独立全量校验 vs 上游 verdict 复用——两种合理实装，bit-exact 等价但接口耦合度不同）。
方案 B 是定理类（缓存增量维护，dirty 边界由 codex 锚定字段确定），可直接实装。
建议：B 先实装（独立、bit-exact 清晰），A 待 Lead 裁定接口变更方向。

---

## 7. 实装结果（方案A=confirmed_len 证书，2026-06-30 续）

### 图景修正：真 #1 是 update_closes_cache（67%），非 macd（21%）
初版 02 计时只包 `compute_macd_hist_incremental`，漏了上游同模式的 closes cache 层。补计时后 200K 实测：
update_closes_cache **15287ms（67%）** > macd 4756ms（21%）> l0_tower 1441ms（6%）。两层是同一 O(n²)
模式（每 bar O(n) 前缀全量比较检测 inclusion 改写）重复两次。

### 根除方案：parser confirmed_len 证书（codex 两轮审计通过）
codex 裁决 classifier 接口下 O(n) 比较不能 bit-exact 降 O(1)（hash 碰撞≠equality）。唯一根除 = parser
输出 `merged_confirmed_len`（跨 bar 物理稳定前缀长度，O(1)）：
- **parser/mod.rs `ParseLayerIncr::append`**：相 B 稳态（append 前后均非 only_open_tail）⟹ confirmed_len
  = `merged.len()-1`（append_folded 仅 Rc::make_mut pop/push 末根，前缀字节不动）；相 A / 相 A→B fold_all
  迁移 ⟹ 0（保守，退化全量重建）。全量 parse_layer ⟹ 0（无血缘）。
- **ParseLayer 加 `merged_confirmed_len: usize`**，手写 PartialEq 排除该证书字段（性能元数据非结构语义，
  使 incr==full 对拍不被误判）。
- **classifier update_closes_cache**：`reuse = min(confirmed_len, cached)` truncate + 续 push，删除每 bar
  O(n) 前缀逐值比较。
- **classifier compute_macd_hist_incremental**：`macd_state_len`（替代 macd_closes_prefix Vec，绑 state_len
  修 codex 血缘断裂漏洞）+ confirmed_len 定 resume_from，删除 `macd_closes_prefix.to_vec()` 全量克隆。

### 实测：O(n²) → O(n)（三窗口 100K/200K/400K）
| 阶段 | 100K | 200K | 400K | 比 | 标度 |
|------|------|------|------|-----|------|
| 00_update_closes | 1.74 | 3.54 | 8.15ms | **2.0x** | **O(n)** ✓ |
| 02_macd | 1.91 | 3.73 | 7.32ms | **1.96x** | **O(n)** ✓ |

update_closes 200K 从 15287ms → 3.54ms（降 4318x）；macd 400K 从 18286ms → 7.32ms（降 2498x）。
两者合计从占 88% 降到 0.04%。

### codex 异质审计裁决（read-only，第二轮）
4 问题全确认 bit-exact：(1) merged_confirmed_len=len-1 相 B 稳态成立，only_open_tail 翻转被 was_open_tail
捕获给 0；(2) update_closes reuse 在每 bar 同源调用下正确；(3) macd rebuild→续推与全量逐 bar append
bit-exact；(4) config 变更靠 cache.clear()（生产契约，IncrementalClassifier 持固定 config）。两个反例均
违反契约（漏调 classifier / 同 cache 换 config），生产路径不触发。硬契约：`classify_with_tower_incremental`
public API「换 config 必 clear」。

### bit-exact 护栏
- 全量 `cargo test --lib` = **1304 passed, 0 failed**（含新增 `bit_exact_confirmed_len_open_tail` 专项测试，
  覆盖 only_open_tail 前缀改写 + 相 A→B 迁移 + 相 B 稳态边界，逐 bar 断言增量==全量）。
- 既有 parser 对拍 `bit_exact_parse_layer_incr_per_bar_synthetic` + classifier `bit_exact_synthetic` 仍通过。

### 剩余 #1 热点（方案B 待做）
A 根除后新 #1 = **01_l0_tower_rebuild**（400K 5775ms, 4.2x 仍 O(n²)）+ 03_frontier_compare（1254ms, 4.0x）。
l0_tower 每 bar 从 segments 全量重建 moves_tower_l0。**复杂度高于 macd**：segments 的 confirmed 前缀**不像
merged_bars 只末元素可变**——古怪线段重划可改写更深的 confirmed 段（bar-1464 seg[9] end 1384→1170，codex
第一轮坐实）。需独立 `segments_confirmed_len` 证书 + codex 审计古怪线段重划深度，非 merged_bars 证书的直接
复制。frontier_compare 同属 level 循环内的 O(n) 比较，可能同一证书路径解。

---

## 8. 口径订正（231号有效域，避免声明膨胀）

**两种 exp 口径回答不同问题，不可混用：**

| 口径 | 测量对象 | A 修复后 | 回答的问题 |
|------|---------|---------|-----------|
| **单次 classify** | 一次 classify_with_tower_incremental 内部逐级投影（#106 project_to_units_resume） | exp≈1.0（线性） | classify 单调用是否线性 |
| **decompose 端到端** | 每 bar 调 classify_at × n 次累积（ECON_L2，编排者「全历史能跑吗」口径） | **exp≈3.5x（仍超线性）** | 全历史回测能否跑通 |

**订正先前「#106 前缀缓存把 classify exp 2.1→1.0 根治」声明**：该声明有效域 = **单次 classify**，对
**decompose 端到端无效**（声明有效域膨胀，231号）。#106（project_to_units_resume）在单次 classify 口径
线性化成立且 bit-exact，但占 decompose 端到端仅 0.16%——**不是 decompose 瓶颈，保留不撤**。

**decompose 端到端实测（A 修复后，/usr/bin/time real）**：
| 100K | 200K | 400K | 200→400 exp |
|------|------|------|-------------|
| 2.66s | 6.87s | 24.32s | **3.54x（仍 O(n^~1.8)）** |

A 根除了 macd+update_closes（88%）但端到端 exp 仍 3.54x——**新主导 = l0_tower(4.2x) + frontier_compare(4.0x)
+ 其余 level 循环阶段**。**编排者口径的「根治」（端到端 exp→~1）= A + B 全做完**，A 单独是必要不充分。
