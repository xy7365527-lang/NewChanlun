# O(n²) 修复结果包：classifier/mod.rs — H4 [00b_l0_units_clone]

状态：**FIXED**（bit-exact，落地）
日期：2026-07-04
基线 HEAD：ebcd8f2a0f（实际提交时在飞 merge，见下）

## 结论

`00b_l0_units_clone` 阶段每 bar `cache.l0_units_cache.clone()` 全量克隆 L0 units
Vec（长度 = segments 数），O(segments)/bar × n bar = O(n²)。

修复：`l0_units_cache` 由 `Vec<UnitRange>` 改为 `Rc<Vec<UnitRange>>`，消费点
`cache.l0_units_cache.clone()` → `Rc::clone(&cache.l0_units_cache)`（引用计数 O(1)）。
`units` 循环变量统一为 `Rc<Vec<UnitRange>>`（与同一文件在飞的 H7 泳道
projected_units Rc 化共用同一类型统一）。写点（stage 00 build、clear()）经
`Rc::make_mut`——同 `moves_tower_l0` 先例（mod.rs:532/609）。

## 计时（本域代表窗 = BTC 前 400K bar，逐 bar classify_with_tower_incremental）

| 阶段 | 修前 | 修后 | 倍数 |
|------|------|------|------|
| 00b_l0_units_clone | 380.7 ms | 5.9 ms | ~64x |

标度：修前 400K→1M=6.55x/2.5x⟹exp≈2.05（O(n²)）；修后 Rc::clone O(1)/bar ⟹ 阶段
成本线性于 n（≈常数/bar），O(n²) 项消除。

## 守卫（全绿）

- `cargo test --release --lib`：1484 passed / 0 failed
- `bit_exact_per_bar`（CL 真实数据 8K bar，增量逐 bar == legacy
  `classify_with_tower(parse_layer(..=i))`）：PASS，bit-identical
- `bit_exact_synthetic`（2000 合成 bar 逐 bar 对照）：PASS
- `incremental_tower_scaling_dominates_full_synthetic`：PASS
- bsp/center/divergence/nest/signal `*_bit_exact*` 电池：全 PASS

## 边界条件（结论翻转条件）

- bit-exact 成立的充要：`units` 全程只读消费（compose/extract 借 `&[UnitRange]`，
  从不原地改），仅整级重赋值。若未来在 loop 内对 `units` 做原地 mutation（`&mut`
  切片写），`Rc::make_mut` 会在 strong_count>1 时静默写时复制 → 语义仍正确但退化不再
  O(1)。当前无此路径。
- `Rc::make_mut` 原地性依赖 strong_count==1：上 bar 的 l0_units Rc 在循环内被投影
  重赋值 drop（loop 尾 `moves_tower`/下一级 `units` 重赋）⟹ 下 bar build 时唯一持有。
  若 caller 跨 bar 持有该 Rc（理论 tower_snapshots 不含 l0_units_cache，无此别名），
  退化写时复制，仍 bit-exact。

## 影响声明

- 改动文件：`rust/src/theta_v0/classifier/mod.rs`（本域）
  - `TowerCache.l0_units_cache`：`Vec` → `Rc<Vec>`
  - stage 00 build：`truncate/extend` 经 `Rc::make_mut`
  - stage 00b：`.clone()` → `Rc::clone`
  - DIAG 对拍：`l0_units != full` → `*l0_units != full`（deref 比较）
  - `units` 初值：直接 move（无 `Rc::new` 双重包裹）
  - `clear()`：`Rc::make_mut(&mut l0_units_cache).clear()`
- 与在飞 H7 泳道（同文件 projected_units Rc 化 + stage-10 Rc::clone + 07b 探针）
  类型互锁于 `units: Rc<Vec<UnitRange>>` 循环变量——无法拆为独立可编译 commit，
  同批提交（均本域 mod.rs，bit-exact 共同验证通过）。
- 下游：无接口变更。`Classification`/tower_snapshots 输出逐字段 bit-identical
  （值不变仅所有权/引用计数变）。信号集/GOLDEN digest 未改（守卫证）。
