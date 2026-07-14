# forest_epoch 架构件设计（H6 O(n²) sound 修复落地）

**日期**：2026-07-04
**工位**：on2-sweep #183 后继——H6 `TreeKey::of_forest` O(n²) 的 sound 落地设计
**状态**：设计（不实装）→ codex read-only 审 → 按审修订
**前置裁决**：`.chanlun/review-results/on2-fix-strategy-interp-rs (TreeKey::of_forest - tree_segment_cached_gen)-20260704.md`（NO-SHIP，codex 裁定唯一 sound 路 = classifier 域新增独立 `forest_epoch`）；commit `150c67e76b`。

**codex 审记录（read-only sandbox, gpt-5.5 xhigh, 2026-07-04）**：初稿裁 **NO-GO**——§3.1 写入站点枚举漏 `upper_moves.truncate`（frontier pop，mod.rs:1253-1254），致 §4 假命中逆否链断（A12 同型复现）。本轮已修订：补 **E2a**（pop 写入站点）入 §3.1/§4、补双 key 失效不变量（§5.4）、订正「至多 ++1」措辞（E4 clear public 方法独立 bump，§5.1）、补 G7 pop/交替专项守卫（§6.2）。codex 逐项确认 E1 门控可接受、双路方向 sound、借用方案可行。修订后重审裁 **GO**。

---

## 1. 问题复述（O(n²) 的精确形态）

`interp.rs:659` `tree_segment_cached_gen` 在**命中检查之前无条件**调 `TreeKey::of_forest(tower)`：

```
let key = TreeKey::of_forest(tower);      // ← O(全塔节点)，每 bar 无条件
if c.valid && c.key == key { /* 命中 */ }
```

`of_forest`（interp.rs:534）遍历 tower **所有级别所有 LeveledMove** 前序深度发射（含 `sub_moves` 递归）。指纹长度 ≈ 全塔节点数，随 n 线性增长 ⟹ O(全塔)/bar × n bar = **O(n²)**。诊断实测 CL：n 4K→8K x_exp=2.02，8K→16K x_exp=2.24（坐实）。

forest 本身已 `Rc` 缓存（miss 才 `extract_carrier_forest` 重建），O(n²) **纯来自每 bar 的指纹计算**，与森林重建无关。

修复目标：把命中判据从「全塔指纹相等」（O(tree)）降为「epoch 计数器相等」（O(1) `u64` 读取比较）。

---

## 2. 为什么必须新增独立 epoch，而非复用 4g `generation`（#160 A12 删快路的教训）

### 2.1 A12 删 4g gen 快路的原文教训

interp.rs:606-607 现有注释与 641-644 函数文档记录了 #160 A12 删 4g `gen: Option<u64>` 快路的理由：

> `TowerCache::generation` 维护点只覆盖 upper 级 cascade/extend/clear——**L0 段尾部古怪线段重划不 bump gen**（CandidateCache bar 3020 坐实）。T_i 只读最高非空级 ⟹ gen 快路 sound；K_i 读全塔含 L0 ⟹ gen 命中 ≠ K_i 不变 ⟹ **假命中返陈旧森林**。故 K_i 缓存唯一命中判据 = `TreeKey::of_forest`。

即：`generation` 是为 **T_i**（`extract_elements`，只读最高非空级）设计的判据。它对 K_i（`extract_carrier_forest`，读全塔含 L0）**不 sound**，因为它的维护点不直接覆盖 L0 内容变更——它靠「L0 变 → cascade 传播至 L1 → did_extend/cascade_reset bump」间接覆盖（**要求 L1 已存在**），并用 `l0_is_root` 每-bar 无条件 bump 兜住 L0-root 阶段的漏洞（mod.rs:1467-1477）。

### 2.2 H6 实证对 A12 soundness 归因的订正（但结论不变）

H6 逐 bar 对拍（CL 16K）发现：`generation` 对 forest 实证 **0 violation**（间接覆盖 + l0_is_root 兜底恰好没漏），但**命中率仅 1.5%**（98.5% bar bump）。而 forest 真变仅 **0.8%/bar**。⟹ `generation` 过度 bump ~124 倍。

过度 bump 的两个来源：
- **`l0_is_root` 每-bar 无条件 +1**（mod.rs:1475）：L0-root 阶段每 bar bump，与 L0 是否真变无关（blunt 兜底）。
- **`did_extend` 耦合 T_i 语义**：`generation` 还被 CandidateCache（interp.rs:830）与 T_i 历史契约消费，其递增点服务 T_i 的可观察变更，非 K_i forest。

**为什么过度 bump 使 gen 无法修 O(n²)**：把 `of_forest`（O(tree)）换成 `generation` 读取（O(1)）后，命中省了指纹；但 miss（98.5% bar）要重建 `extract_carrier_forest` = O(tree)/bar = **O(n²) 原样回归**。只有 bump 率贴近 forest 真变率（0.8%）才能把重建摊还到 O(n)。故 gen 不仅 codex 裁 C（不改语义服务 K_i），实证上也**无收益**。

### 2.3 forest_epoch 与 4g gen 的三条本质区别

| 维度 | 4g `generation`（T_i 判据） | forest_epoch（K_i 判据） |
|------|---------------------------|--------------------------|
| 服务视图 | T_i（`extract_elements`，最高非空级） | K_i（`extract_carrier_forest`，全塔含 L0） |
| L0 覆盖方式 | **间接**（L0→cascade→L1→bump，需 L1 存在）+ `l0_is_root` 每-bar blunt 兜底 | **直接**在 L0 塔重建站点 bump（覆盖 L0-root ∧ L0-尾段重划-under-L1，无需 blunt 兜底） |
| 消费者 | CandidateCache + T_i 历史契约（不可动，codex 裁 C） | 仅 K_i 森林段（`tree_segment_cached_gen` tree 段） |
| bump 率 vs forest 真变 | 98.5% vs 0.8%（过度 124×，无收益） | 目标贴近 0.8%（bump 仅在 L0/upper 实际 mutate 时）|

**关键**：forest_epoch 在**塔实际发生字节变更的写入点**直接 ++，取代 gen 靠 cascade 间接传播 + l0_is_root 每-bar 兜底的两段式覆盖。这既修了 A12 所指的 L0-root soundness 漏洞（在 L0 重建站点直接 bump，比 l0_is_root 更紧——只在 L0 真重建尾段时 bump 而非每 bar），又避免了过度 bump。

---

## 3. 递增点全枚举（覆盖一切塔可观察变更）

`extract_carrier_forest(tower)` 的输入 = `tower: &[Rc<Vec<LeveledMove>>]` = `tower_snapshots`，其内容 = `[moves_tower_l0, upper_moves(L1), upper_moves(L2), ...]`（mod.rs:1123/1300）。森林输出是该 tower 的纯函数。故 forest_epoch **当且仅当 tower 任一级 `Vec<LeveledMove>` 内容发生字节变更时** ++。

枚举 `classify_with_tower_incremental` 中一切对 tower 级 `Vec<LeveledMove>` 内容的写入站点（`Rc::make_mut` 后的 truncate/push/extend/clear）：

| # | 站点 | 文件:行 | 写的是 | 触发条件 | epoch 动作 |
|---|------|---------|--------|----------|-----------|
| **E1** | L0 塔重建 | mod.rs:1072-1081（stage `01_l0_tower_rebuild`） | `moves_tower_l0`（L0 塔 = tower[0]） | `reuse < moves_tower_l0.len()`（尾段古怪线段重划截断）**或** `l0.segments[reuse..]` 非空（新确认段 push） | 满足条件 ⟹ `++` |
| **E2** | upper_moves 追加 | mod.rs:1284-1288（`did_extend`/stage `06_extend_centers_upper`） | 本级 `upper_moves`（tower[level+1]） | `!tail_upper.is_empty()`（本级向上产出新走势 = 现有 `did_extend` 触发条件） | 非空 tail ⟹ `++` |
| **E2a** | frontier pop（truncate） | mod.rs:1242-1255（`had_emitted_window` 块，`um.truncate`） | 本级 `upper_moves.truncate`（末窗口 pop_n 个产出回退，frontier 重切） | `had_emitted_window == true`（`resume_from < consumed`，末窗曾产出 ⟹ pop） | pop 发生 ⟹ `++`（**codex 审补漏**，见下） |
| **E3** | cascade 清塔 | mod.rs:1199-1211 | 本级 `upper_moves.clear()`（frontier 改写/回缩传播的重扫） | `cascade_reset == true`（本级或下级 frontier_mutated / 长度回缩） | cascade 触发 ⟹ `++` |
| **E4** | 全量 clear | mod.rs:602-618（`TowerCache::clear`） | `moves_tower_l0.clear()`（段账本前缩/config 变更/merged 前缀改写） | `clear()` 被调（`l0.segments.len() < last_l0_segments_len` 等，mod.rs:1060-1062） | `clear()` 内 `++`（与现 `generation += 1` 同批） |

**E2a 是 codex read-only 审（2026-07-04）补的漏点**：初稿 §3.1 只枚举 upper_moves 的 `clear`（E3）与 `extend`（E2），漏了 `had_emitted_window` 分支的 `upper_moves.truncate`（mod.rs:1253-1254，frontier pop）。**这是 did_extend 覆盖不到的独立写入站点**：pop 掉末窗口 pop_n 个产出后，若本 bar 重扫 `tail_upper` 为空（frontier 尚未重新凑齐窗口），则 upper_moves **净缩短**——forest 变了，但 `did_extend |= !tail_upper.is_empty()` 为 false（E2 漏），且此 bar 非 cascade（E3 漏）⟹ **epoch 漏 bump ⟹ 假命中返陈旧森林**。这正是 A12 教训的同型复现（一个未枚举的 mutate 站点），codex 裁 NO-GO 的主因。修复：E2a 在 `had_emitted_window` 为真时无条件 bump（over-invalidate：即便重扫复现完全相同字节也 bump，sound-safe；`pop_n>=1` 由 mod.rs:1247 debug_assert 保证 pop 非空）。

**E1 的门控是 sound 的紧化，不是宽化**：L0 塔每 bar 都进重建 stage，但 `truncate(reuse)` 当 `reuse == len` 时是 no-op，`push` 当 `segments[reuse..]` 空时不执行 ⟹ 无字节变更的 bar（inclusion-only，未产新段）**不满足** E1 条件 ⟹ 不 bump。这正是把 bump 率从 gen 的 98.5% 压回 0.8% 的机制。

**「宁可多失效不可假命中」的落点**：E1-E4 在**写入站点无条件**触发（不再二次判断「这次 mutate 是否影响 forest 输出」）。例如 E1 的 truncate 后即使 re-push 出字节相同的尾段（re-division 产出恰同旧值），仍 bump——over-invalidate（多失效一次，多重建一次 forest）是 sound-safe 方向；反方向（漏 bump）才产生假命中。这规避了 A12 之前「投影是否捕获深字段变化」那类易错判断（cascade 机制发明的同一动机，mod.rs:1135-1141），no-patch。

### 3.1 完备性论证：无遗漏写入站点

tower 级 `Vec<LeveledMove>` 的全部 `Rc::make_mut` 写入站点（grep `make_mut.*moves_tower_l0|make_mut.*upper_moves` + `moves_tower_l0.clear`；**codex read-only 审复核后逐站点核对**）：
- `moves_tower_l0`（= tower[0]）：**仅** E1（重建 truncate/push，mod.rs:1074-1080）与 E4（clear，mod.rs:609）。无其他站点原地改写其元素。
- `upper_moves`（= tower[level+1]）：**四个** make_mut 写入站点，全部枚举——
  - E2：`extend`（tail 追加，mod.rs:1288）
  - **E2a：`truncate`（frontier pop，mod.rs:1254）**——初稿遗漏，codex 补
  - E3：`clear`（cascade 重扫，mod.rs:1202）
  - （投影用的 `projected_units.clear/make_mut` 不是 tower 级 Vec，不进 forest）
  §16 confirmed 前缀不可变 ⟹ 无对已确认前缀元素的**原地值改写**（mod.rs:1283「前缀不可变，仅尾部追加」；E2a 的 truncate 只回退**未 sealed 的 frontier 末窗**，不动 confirmed 前缀）。
- `centers` 不进 forest（`extract_carrier_forest` 只读 tower snapshots = moves_tower_l0 + upper_moves，不读 centers，coverage.rs:270-274）⟹ centers 的 make_mut（含 mod.rs:1252 pop、1287 extend、1203 clear）无关，正确地不触发 epoch。

⟹ E1-E4 **加 E2a** 覆盖 tower 内容变更的全部路径。（初稿缺 E2a，§4 逆否链断——codex 裁 NO-GO 的核心，本轮已补。）

---

## 4. 假命中不可能性论证

**命题**：若两个连续 bar 的 forest_epoch 相等，则 `extract_carrier_forest(tower)` 逐字节相等。

**证明（逆否 + 完备枚举）**：
1. `extract_carrier_forest` 是 tower 的纯函数（coverage.rs:267-306，无外部状态；dedup/重映射只依赖 tower 内容）。⟹ tower 内容不变 ⟹ 输出不变。
2. tower 内容变更 ⟺ 某级 `Vec<LeveledMove>` 字节变更 ⟺ 发生 E1∨E2∨E2a∨E3∨E4 之一（§3.1 完备性：这五类是全部 make_mut 写入站点）。
3. E1-E4 + E2a 每类在其写入条件成立时**无条件** `++`（§3 的 over-invalidate 落点）。⟹ tower 内容变更 ⟹ epoch 递增（严格单调，见 §4.1）。
4. 逆否：epoch 不变 ⟹ 无 E1/E2/E2a/E3/E4 触发 ⟹ tower 内容不变 ⟹（由 1）forest 输出不变。∎

**E2a 补齐前的洞（codex 定位）**：pop（E2a）+ 空重扫（tail_upper 空）的组合使 upper_moves **净缩短**却 `did_extend=false`（E2 不触发）、非 cascade（E3 不触发）——tower 变了但无 bump ⟹ 命中返陈旧森林。这是 A12 教训的同型复现（未枚举的 mutate 站点）。补 E2a 后逆否第 3 步的「无条件 ++」对 pop 成立，链条闭合。

**A12 L0-root 漏洞的封堵**：gen 的漏洞是 L0 内容变更在 L1 不存在时不经 cascade（E3 的 gen 类比）传播，靠 `l0_is_root` blunt 兜。forest_epoch 的 E1 **直接在 L0 塔重建站点**捕获 L0 内容变更（无论 L1 是否存在）⟹ L0-root 阶段与 L0-尾段-重划-under-L1 两种形态都在 E1 覆盖内，不依赖 cascade 传播、不需要 l0_is_root 每-bar 兜底。这是对 A12 教训的正面回应：把 L0 覆盖从「间接 + blunt 兜底」改为「站点直接捕获」。

### 4.1 单调性（不可 reset 为 0）

forest_epoch 是 `u64` 单调递增，**永不 reset**（含 E4 clear 时也是 `++` 而非置 0）——与现有 `generation` 同规格（mod.rs:615-617 原文：「不 reset 为 0——下游缓存旧 epoch 可能恰为 0 ⟹ 假命中复用陈旧树」）。u64 在 per-bar 递增下不可能溢出（16K bar × 1/bar 远小于 2^64）。

---

## 5. 消费点改动清单

### 5.1 classifier 域（TowerCache 内维护）

| 改动 | 位置 | 内容 |
|------|------|------|
| C1 新字段 | mod.rs:~564（`generation` 字段旁） | `TowerCache` 加 `forest_epoch: u64`（独立于 `generation`，不复用；文档标注 K_i 判据） |
| C2 accessor | mod.rs:~578（`generation()` 旁） | `pub fn forest_epoch(&self) -> u64 { self.forest_epoch }` |
| C3 E1 bump | mod.rs:1072-1081 内 | L0 重建 stage 内，`reuse < old_len || !segments[reuse..].is_empty()` 时 `self.forest_epoch += 1`（需在 make_mut 前捕获 old_len） |
| C4 E2 bump | mod.rs:1284 旁 | 复用 `did_extend` 同条件累积到局部 bool（见借用注记） |
| C4a E2a bump | mod.rs:1242 `had_emitted_window` 块内 | **codex 补**：`had_emitted_window == true`（pop 发生）⟹ 置局部 bool（不管 tail_upper 是否为空——pop 本身即 tower 变更） |
| C5 E3 bump | mod.rs:1199 `if cascade_reset` 块内 | cascade 触发 ⟹ 置局部 bool |
| C6 E4 bump | mod.rs:617 `clear()` 内 | `self.forest_epoch += 1`（与现 `generation += 1` 并列，方法内直接 bump） |

**借用注记（实装约束）**：E1 在 `for level_idx` **循环前**的 L0 rebuild stage 触发（mod.rs:1072-1081）；E2/E2a/E3 在循环内触发，循环持 `lc = &mut cache.levels[level_idx]`。对 `cache.forest_epoch` 的写入须避开与 `lc` 的可变别名——用**单个**局部 bool `forest_dirty`：E1 在循环前置位（L0 rebuild 满足门控时），E2/E2a/E3 在循环内 `|=` 累积，循环结束后（mod.rs:~1462 `generation` bump 旁）统一 `if forest_dirty { cache.forest_epoch += 1; }`。这与现有 `did_extend`/`cascade_reset` 局部累积-循环后消费的模式同构（mod.rs:1142-1145）。
- **循环内至多 ++1**（局部 bool 折叠）；即使改成每级 ++ 也 sound（只多失效）。
- **但 E4（`clear()`）是 public 方法，其 bump 在方法内直接发生**（不经循环 bool）——故不能声称「全局每 bar 至多 ++1」：一个 bar 若既 `clear()`（E4，某 caller 显式调）又走增量循环，可能 ++2。多 ++ 恒 sound（codex item 6 明确）。准确表述 = **循环内至多一次；clear() 独立一次**。

### 5.2 backtest 域（透传）

| 改动 | 位置 | 内容 |
|------|------|------|
| C7 accessor 透传 | incremental.rs:~96 | `IncrementalClassifier::forest_epoch(&self) -> u64 { self.tower_cache.forest_epoch() }`（与现 `tower_generation()` 并列） |
| C8 runner 读取 | runner.rs:774 | `classify_at` 返回或紧邻读 `classifier_incr.forest_epoch()`（当前返 `(classification, tower, tower_gen)`——需扩为携 `forest_epoch`，或 runner 单独调 accessor） |

### 5.3 strategy 域（消费判据切换）

| 改动 | 位置 | 内容 |
|------|------|------|
| C9 `tree_segment_cached_gen` 加参 | interp.rs:647-689 | 加 `forest_epoch: Option<u64>` 参数；命中判据从 `TreeKey::of_forest(tower)` 比较改为 epoch 比较（见 §5.4）；TreeCache 存 `key_epoch: Option<u64>` 替代/并存 `key: TreeKey` |
| C10 TreeCache 字段 | interp.rs:590-608 | 加 `key_epoch: Option<u64>`（命中判据）。`key: TreeKey` 字段：`Some(epoch)` 路径命中判据改用 epoch；`None`（合成/全量路径无 epoch）fallback 仍用 `of_forest`（见 §5.4 双路） |
| C11 两 wrapper 加参透传 | interp.rs:810/919 | `coverage_elements_with_tower_cached_gen` 与 `coverage_elements_and_gamma_with_tower_cached_gen` 加 `forest_epoch: Option<u64>`，透传给 tree_segment_cached_gen。**candidate 段仍消费 `tower_gen`（generation）不动**（interp.rs:830，codex 裁 C：generation 语义不改） |
| C12 两 runner 调用点 | runner.rs:808-810 / 1041-1043 | 传 `Some(forest_epoch)` 实参 |

### 5.4 命中判据双路（保留 fallback，不制造假命中）

`tree_segment_cached_gen` 收 `forest_epoch: Option<u64>`：
- **`Some(e)`（生产增量路径，runner）**：命中 = `c.valid && c.key_epoch == Some(e)`。O(1)。这是 O(n²) 修复的主路径。
- **`None`（合成闭包 / 全量非增量路径，如 `coverage_elements_and_gamma_with_tower_cached` 无 gen 入口 interp.rs:900-906）**：无 epoch 可读 ⟹ **fallback 到现有 `TreeKey::of_forest` 比较**（O(tree)，但这些路径不在 per-bar 热循环，无 O(n²) 暴露）。

**为什么保留 `None` fallback 不是补丁思维**：`None` 路径的调用者（合成测试、全量 API）本就没有增量 epoch 语义（它们不跨 bar 复用 TowerCache），epoch 对它们无定义。对无定义输入用 epoch 会假命中。fallback 到指纹是这些路径的**正确判据**，非「先修一半」。生产热路径（O(n²) 暴露点）全部走 `Some` 路径。

**双 key 失效不变量（codex 审补，item 4）**：同一 `TreeCache` 实例若被 `Some`/`None` 两种模式交替调用，两个 key（`key: TreeKey`、`key_epoch: Option<u64>`）会**各自陈旧**——`Some` 命中/建缓存时不算 `of_forest`（`key` 陈旧于上次 `None` 建时的旧 tower），`None` 命中/建时不 bump epoch（`key_epoch` 陈旧）。若下一次切回另一模式仅比对本模式的 key，可能假命中另一模式遗留的陈旧 forest。**必须的失效规则**：TreeCache 建/刷新缓存（miss 重建分支）时**同时刷新两个 key**——`Some(e)` 建时也写 `c.key = TreeKey::of_forest(tower); c.key_epoch = Some(e)`（`of_forest` 只在 miss 重建时算一次，非每 bar，不回归 O(n²)）；`None` 建时写 `c.key = of_forest; c.key_epoch = None`。命中判据按当前模式选对应 key，但两 key 恒指向同一份已缓存 tree ⟹ 模式交替无陈旧。（生产 runner 恒 `Some`，交替只在混用测试出现——但不变量必须无条件成立，no-patch。）

---

## 6. bit-exact 风险评估

**核心声明**：本改动是**纯缓存命中键的语义替换**（`of_forest` 指纹相等 → epoch 相等），**不改任何输出计算**。命中 ⟹ 复用同一 `Rc<forest>`；miss ⟹ 走原 `extract_carrier_forest` 重建（一字不改）。forest 的**内容**由 `extract_carrier_forest` 决定，epoch 只决定「是否重算」，不参与 forest 构造。

⟹ 若 §4 假命中不可能性成立，则命中复用的 forest 与「当 bar 现算 `extract_carrier_forest(tower)`」逐字节相等 ⟹ 下游（candidate 附着、host^op、γ、merge、π）全链 bit-exact。

### 6.1 bit-exact 风险来源（唯一）= 假命中（epoch 漏 bump）

唯一能破 bit-exact 的形态：某个 tower 内容变更路径未被 E1-E4 覆盖 ⟹ epoch 漏 bump ⟹ 命中返陈旧 forest。§3.1（完备枚举）+ §4（假命中不可能性证明）论证此不可能。风险落在「§3.1 枚举是否真完备」这一命题上——由守卫清单（§6.2）在 debug/test 层每 bar 兜底验证。

### 6.2 守卫清单

| 守卫 | 层级 | 作用 | 状态 |
|------|------|------|------|
| G1 命中 debug_assert | debug/test（interp.rs:661 现有，**保留**） | 每次命中重算 `extract_carrier_forest` 逐字节对拍缓存 forest——**任何假命中（epoch 漏 bump）在 debug/test 立即 panic** | 已在位，改动后须**确认 assert 在 epoch 命中分支仍执行**（不能因换判据而移除） |
| G2 A12 双视图一致性 debug_assert | debug（interp.rs:668-672 现有，保留） | miss 重建分支 T_i↪K_i 嵌入一致性 | 已在位，miss 路不变 |
| G3 GOLDEN digest 电池 | test | 全窗输出指纹回归——epoch 若漏 bump 致输出偏移则 digest 变 | 须跑（改动后强制） |
| G4 `bit_exact_per_bar` / `bit_exact_synthetic` | test | 增量 vs 全量逐 bar 对拍 | 须跑 |
| G5 `incremental_tower_*` 电池 | test | 增量塔 bit-exact | 须跑 |
| G6 | CL 16K OOS 逐 bar forest 对拍（H6 诊断探针复用） | 一次性验证 | epoch 命中率实测（目标≈99% / bump≈0.8%）+ 0 violation | H6 诊断脚手架可复用（当时 revert，须重建为 ignored 探针或临时验证） |
| G7 | frontier pop（E2a）专项 + Some/None 双 key 交替测试 | test（codex 审补，item 4/5） | 构造 `had_emitted_window` pop + 空重扫 bar，断言 epoch bump（防 E2a 回归漏 bump）；构造同一 TreeCache 交替 Some/None 调用，断言无陈旧命中（防 §5.4 双 key 陈旧） | 新增（覆盖 codex 定位的两个具体漏点） |

**G1 是 sound 的最后防线**：即使 §3.1 枚举遗漏了某个写入站点（人为疏忽），G1 在 debug/test 每 bar 全量对拍会立即暴露。生产 release 不带 G1（性能），故 **release 上线前必须先在 debug + 全电池（G3-G6）绿灯**，坐实枚举完备性后方可信任 release 命中路径。这是 no-patch 的严格落点：不靠「当前没撞」，靠「debug 全量对拍 + 完备性证明」双证。

---

## 7. 边界条件（结论翻转）

- 若 G1/G4/G6 在某 bar 触发假命中 panic ⟹ §3.1 枚举不完备 ⟹ 存在未识别的 tower 写入站点 ⟹ 停下补枚举（不加 workaround），重走 §4 证明。
- 若 G6 实测 bump 率 ≫ 0.8%（如仍 ~98%）⟹ E1 门控失效（每 bar 满足条件）⟹ O(n²) 未消除 ⟹ 复查 E1 的 `reuse < len || push 非空` 是否被误写成无门控每-bar bump。
- 若编排者/codex 裁定 forest_epoch 应与 generation 合并（反 codex 裁 C）⟹ 本设计的双 epoch 结构翻转为单 epoch + T_i/K_i 分离 accessor（需重证 generation 对 K_i sound，即补 l0_is_root 类兜底——但那正是过度 bump 根源，不推荐）。

---

## 8. 影响声明

- **改动模块**：classifier/mod.rs（TowerCache 字段 + 4 处 bump + accessor）、backtest/incremental.rs（accessor 透传）、backtest/runner.rs（2 处调用透传）、strategy/interp.rs（tree_segment_cached_gen 判据 + 2 wrapper 加参 + TreeCache 字段）。
- **不改**：`extract_carrier_forest`（forest 构造）、`generation` 语义（candidate cache + T_i 契约保持）、`of_forest`（`None` fallback 路径保留）、任何输出计算路径。
- **谱系引用**：#160 A12（648 裁决 D，双视图 T_i/K_i 分离——本设计是其 K_i 侧 O(1) 命中判据的落地）；H6 NO-SHIP 结果包（codex 裁 A/B/C）。interp.rs:641-644 注释的「generation 对 K_i 不 sound」结论应订正为「generation 间接覆盖 + l0_is_root 兜底恰 sound 但过度 bump 无收益；forest_epoch 站点直接覆盖，紧且 sound」——留待实装工位一并修文档。
- **认识论等级**：本设计的 soundness 证明为 **L0（代数/定义，§4 纯函数 + 完备枚举）**；命中率/bump 率的 0.8% 目标为 **L2-实证（CL 16K，H6 已测 forest 真变基准）**，须 G6 复验。bit-exact 由 L0 证明 + G1/G3-G6 守卫双证。
