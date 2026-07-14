# O(n²) 修复结果包：stage 09_project_to_units_resume（#H1）

**域**：classifier/mod.rs + recursive_tower.rs
**基线 HEAD**：2572c1f691（编排令基线 ebcd8f2a0f 已被在飞 merge 推进；descend.rs 归他域，未触）
**日期**：20260704

## 结论

stage `09_project_to_units_resume` 从 **1238ms → 35ms**（400K bar，35x），全阶段墙钟 **6.00s → 3.94s**（-34%）。stage 09 由全阶段 #1 热点降至倒数第三。

### 根因

`project_to_units_resume`（recursive_tower.rs:586）逐 frontier tail 元素调 `m.rmove.lo()/hi()`。对 `RMove::Compose` 变体，`RMove::lo()/hi()`（descend.rs:94/102）**递归遍历整棵子树**取 min/max。frontier 走势的子树随其窗口延伸增长 O(n)，且每 bar 重投影 ⟹ O(n²)。`start_index/end_index` 在 compose 时已缓存却单缓外缘 lo/hi 未缓 = 不对称漏洞。

### 修复

外缘 `[lo,hi]` 在 `compose` 时**已算入携带的 `Center`**（`RMove::Compose.centers[0].dd/gg`）。投影契约（compose_level_resume:463「units 是 subs_moves 的投影」）保证：
```
center.dd = min(窗口 units.lo) = min(subs.rmove.lo()) = rmove.lo()
center.gg = rmove.hi()   逐字段相等
```
新增 `LeveledMove::envelope() -> (Tick, Tick)`（recursive_tower.rs）读携带 center O(1)：
- `Segment`：直接读字段 `(lo,hi)`（对 Segment `rmove.lo()/hi()` 本已 O(1)）。
- `Compose`：读 `centers[0].dd/gg`；缺 center（不该发生）回退递归（值同，护 bit-exact）。

热路径 `project_to_units_resume` + `fold_direction` 改走 `envelope()`。**全量 `project_to_units`（line 555）保留递归 `rmove.lo()/hi()`**——作 mod.rs:1400 逐 bar `debug_assert` 神谕的对照面（envelope O(1) 增量 vs 递归深扫全量）。

## 计时（修前/修后，400K BTC，THETA_PROFILE_STAGES=1）

| | 修前 | 修后 |
|---|---|---|
| 09_project_to_units_resume | 1238.192 ms | 35.436 ms |
| 全阶段墙钟 | 6.00s | 3.94s |

## 守卫（全绿）

- `bit_exact_synthetic`（debug，2000 bar，mod.rs:1400 神谕逐 bar live）：PASS bit-identical
- `bit_exact_per_bar`（release，CL 8K 双跑对照）：PASS bit-identical
- `classifier::recursive_tower::tests`（17 项，含 project_to_units_resume_matches_full）：PASS
- `cargo test --release --lib` 全套：1484 passed / 0 failed / 111 ignored

## 边界条件（结论翻转条件）

- 若 `RMove::Compose` 停止在 compose 时携带单一 center（compose_level 改多 center / 空 center），`envelope()` Compose 支读值将 ≠ 递归深扫 → mod.rs:1400 debug_assert 即刻 panic（test 编译）。当前 `compose` 恒 `centers: vec![center]`（recursive_tower.rs:189），不变量成立。
- 若延伸吸收/升级重切改为不聚合窗口全 units 外缘（当前 detect_centers_windowed_resume `c.dd=c.dd.min(u.lo)` + 子窗 `map(|u|u.lo).min()` 两支均聚合），center.dd/gg ≠ rmove.lo()/hi()，同样被 mod.rs:1400 神谕捕获。

## 影响声明

- 改 `rust/src/theta_v0/classifier/recursive_tower.rs`：新增 `LeveledMove::envelope()`；`fold_direction` 与 `project_to_units_resume` 改走 O(1) envelope；import 加 `Tick`。
- mod.rs 未改（stage 09 调用点不变，护栏 debug_assert 不变）。
- descend.rs（`RMove::lo/hi`）未改（他域在飞）。
- 纯性能优化，bit-exact：信号集/GOLDEN/走势塔身份全部不变（浮点无涉，全整数 Tick）。
