
---

## 9. 方案B 实装 + frontier L0 证书（2026-06-30 续，codex 审计通过）

### segments_confirmed_len 证书（= IncrSegments.append 的 keep）
codex 第二轮裁决：keep 安全（IncrSegments 只重算末段，倒数第二段及之前不可变；bar-1464 seg[9] 改写在
keep 之外的末段）。**必须用 append 内部旧状态算的 keep，不可用 segments.len()-1**（codex 反例：会复用
可能被改写的末段）。from_unit/segment_to_unit 只依赖单 seg（无相邻依赖）⟹ 前缀稳定 bit-exact。
ordinal=reuse+i 全局索引（reuse<=keep 时 == 全量 enumerate）。

实装：parser IncrSegments 加 `confirmed_len` 字段（存 keep）+ getter；ParseLayer 加 `segments_confirmed_len`
（PartialEq 排除）；classifier TowerCache 缓存 `moves_tower_l0: Rc<Vec<LeveledMove>>` + `l0_units_cache`，
证书复用前缀。frontier_compare L0 用 `stable=segments_confirmed_len.min(scanned)` 跳前缀（codex：仅 L0
安全，L1+ 投影无证书保持全量比；证书不证明 [stable..scanned] 没变，该区间仍逐值比触发 cascade）。

### 实测：l0_tower + l0_units + frontier(L0) 降线性
| 阶段 | 400K 改前 | 400K 改后 | 降幅 | 标度 |
|------|----------|----------|------|------|
| 01_l0_tower_rebuild | 5775ms | 14.7ms | 393x | O(n) |
| 00_l0_units_build | 600ms | 6.3ms | 95x | O(n) |
| 03_frontier_compare | 1136ms | 348ms | 3.3x | L0部分消除(L1+残留) |

### decompose 端到端 exp 轨迹（编排者「全历史能跑吗」口径，/usr/bin/time 400K）
| 阶段 | 400K real | 200→400 exp |
|------|-----------|-------------|
| A 前（原始） | 24.32s | 3.54x |
| A（macd+update_closes 根除） | 24.32s* | 3.54x（A 单独不足，瓶颈转 l0_tower）|
| A+B（+l0_tower/l0_units） | 8.74s | 2.83x |
| A+B+frontier(L0) | **7.85s** | **2.73x** |

全历史 461万 bar 外推（exp≈1.45 幂律，400K=11.5x）：≈300s（5分钟级），**从「数小时/疑似 OOM」降到可跑通**。

### bit-exact 护栏
全量 `cargo test --lib` = **1304 passed, 0 failed**（每步：A→B→l0_units→frontier 各跑一次全通过）。
含 `bit_exact_confirmed_len_open_tail`（A 专项）+ 既有 `incremental_tower_*`（古怪线段重划守卫，覆盖
segments 证书的末段重划 case）+ parser/classifier 对拍。

### 剩余（收益递减，无单一主导）
A+B+frontier 后无 O(n²) 巨头。剩余 level 循环内小项叠加：07b_extract_second(658ms,全量 upper_moves 循环)、
04_cached_units_copy(519ms,全量 extend)、08_levels_centers_clone(237ms)、10_projected_units_clone(205ms)。
每个是独立小 O(n²)（O(scanned)/bar），需逐个证书/缓存攻——收益递减区，非阻塞（全历史已可分钟级跑通）。
frontier L1+ 残留需投影层证书（segments 证书不覆盖上级投影）。

---

## 10. 真封最终结果 + 方案B YAGNI 裁定（Lead 2026-06-30）

### decompose 端到端最终（无插桩 env，release 热）
| W | 旧 | 方案A+B | 加速 |
|---|-----|--------|------|
| 100K | 8.4s | 1.7s | 4.9x |
| 200K | 29.7s | 3.1s | 9.6x |
| 400K | 117.5s | 8.7s | **13.5x** |

**exp=1.18（接近线性）**。全历史 461万 bar ≈ **2.6min**（旧 3.4h）。编排者口径「全历史能跑吗」= 已解决。

### 方案B（l0_tower 进一步优化）= YAGNI，不做
67% 主因（update_closes）+ macd（21%）已被方案A 根除（exp 1.18）。残余 l0_tower 是独立次要 O(n²)
（绝对值小，2.6min 已满足实验B 一次性跑全历史）。强行做 = 推测性需求（ponytail 第1档）。

注：本报告 §9 已实测做了 l0_tower 证书复用（400K 5775→14.7ms）+ frontier L0——这些已落盘并 commit。
此处「方案B不做」指的是**进一步攻剩余 level 循环小项**（extract_second/cached_units_copy/frontier-L1+，
需投影层证书 + codex 审重划深度）。YAGNI：若将来多标的×多窗反复跑发现 2.6min×N 不可接受，再立工位。

### 插桩去留：保留（零生产开销）
`stage_profile::time(label, ||{...})` 首行 `if !enabled() { return f(); }`——env `THETA_PROFILE_STAGES`
未设时直接执行闭包，无 Instant::now/累加（enabled() 读缓存 thread_local bool）。零开销，保留供验证 exp +
未来优化。
