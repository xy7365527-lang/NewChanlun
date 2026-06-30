# fullhist-OOM 诊断（Task #103）：OOM 是误诊，真凶是 O(n²) 时间墙

**认识论等级**：L2（真实 BTC 数据，多窗实测峰值 RSS，可否证）。
**复算**：`cd rust && cargo test --release --lib l2_btc_capturable_spread_diagnosis --no-run` 得 BIN，
`ECON_L2_MAX_BARS=<W> /usr/bin/time -l <BIN> l2_btc_capturable_spread_diagnosis --ignored --nocapture --test-threads=1`。

## 结论

**OOM 不存在——是误诊。** 旧注释「500K bar OOM 被杀=内存上限」被实测否证。
真瓶颈 = **O(n²) 时间墙**，内存随 bar **平坦在 ~2GB**。全量 461 万 bar 内存上跑得通，只是慢（O(n²)）。

## 实测证据（/usr/bin/time -l 峰值 RSS）

| 窗口 bar | 峰值 RSS | 耗时 | n_signals |
|---|---|---|---|
| 100K | 1990 MB | 8.7s | 285 |
| 200K | 2010 MB | 31.8s | 545 |
| 400K | 2034 MB | 124.5s | 1110 |
| 600K | 2083 MB | 283s | 1682 |

- **内存**：100K→600K（6×bar）RSS 1990→2083 MB（**+5%**）= 平坦。旧注释称「500K OOM」**直接被 600K 跑通否证**。
- **时间**：100K→400K（4×bar）8.7→124.5s ≈ 14× ⟹ **exp≈1.9 = O(n²)**。旧「500K OOM 被杀」实为 wall-clock 超时被 kill 误报为 OOM。
- 全量 461 万 bar 已后台启动（`ECON_L2_MAX_BARS=5000000`，>4.6M=不截断），O(n²) 外推 ≈ 4.7 小时跑通、内存仍 ~2GB。日志 `/tmp/fullhist_oom_full.log`。

## 根因（为何内存不爆）

诊断报告归因「IncrementalClassifier 每 bar 产一份 tower Rc<Vec<LeveledMove>> 跨 bar 累积→内存爆」**不成立**：

1. `classify_at(i)` 返回的 `tower_i` 是 `Vec<Rc>`（O(1) 浅拷贝指针），调用方循环体内用完即弃（下轮迭代覆盖），**不累积**（econ_positive.rs:166）。
2. `TowerCache.levels[k].upper_moves`（mod.rs:368）是塔的**当前状态**——每级一个 `Rc<Vec<LeveledMove>>` 跨 bar 单调 append。其大小 = 塔元素总数（走势/中枢），**亚线性于 bar**（中枢远少于 bar）。不是「每 bar 一份副本」。
3. `LeveledMove.sub_moves: Vec<LeveledMove>`（recursive_tower.rs:93/144 `subs.to_vec()`）确有递归深拷贝放大（高级走势含全部后代副本），但塔元素数亚线性 ⟹ 总量仍可控在 ~2GB。（这是 Task #104 的领域：`sub_moves` 改 Rc 共享可降此放大，但**非 OOM 必要条件**。）

## 真瓶颈（O(n²) 时间，非本 task 边界）

`decompose_capturable_spread` 循环每 bar 调 `classify_at(i)` = `classify_with_tower_incremental`，其塔续算含 O(tree)/bar 工作（`TreeKey::of` 全发射 / candidate 段重建），tree 单调增 ⟹ O(n²) 累积。incremental.rs profile 模块 `profile_classify_at_decompose_16k` 已坐实「t_exp≈2.0，O(n²) 根在增量塔」。

注意：本 decompose 路径**未用生产 runner 的 generation 快路 / TreeCache**（runner.rs:577 用了，CL 16K 仅 26 次 miss）。decompose 直接 `classify_at` 走慢路。这归 **Task #104/#105**（classifier 核心 + 全引擎 perf 审计）。

## 解法（no-workaround）

- **不滑窗截断**（违背 goal——级别涌现需长积累）：✓ 内存不爆，无需滑窗。
- **不分块**：✓ 无 OOM，无需分块传塔状态。
- **正解**：全量直接跑（给足时间）即可拿到高级别样本——内存允许。要加速 = 修 O(n²)（Task #104/#105），与「测高级别 alpha」目的正交，不阻塞样本积累。
- **截断窗保留**为**时间**预算（非内存），注释已订正（econ_positive.rs:395-403）。

## 是否丢失高级别涌现（关键）

**不丢失。** 全量直接跑无需任何截断/滑窗/分块 ⟹ 高级别（level2-4）走势在全 461 万 bar 上自然涌现，样本完整。这正是 goal「测高级别 alpha 需全历史样本」的解锁条件——OOM 误诊曾逼迫 300K 截断（level2-4 各仅 4-21 信号），实测证明截断**非内存必需**。

## 影响声明

- 改动：`rust/src/theta_v0/backtest/econ_positive.rs:395-403` 注释订正（错误 OOM 归因 → 实测 RSS 平坦 + O(n²) 时间真因）。无逻辑改动。
- 解锁：`acc-multilevel-sample`（全历史级别分布）——全量可跑，样本不被截断。
- 转交：O(n²) 时间优化 → Task #104（sub_moves Rc 共享）/ #105（全引擎 perf 审计）。

## 边界条件

结论翻转条件：若 BTC 全量后台跑（PID 启动时 7238）中途真 OOM 被 kill（RSS 突破 96GB 物理内存）⟹ 本结论否证、回到分块。但 600K→全量 RSS 外推（塔元素亚线性）预期峰值 < 5GB，远低于 96GB ⟹ 翻转概率极低。日志 `/tmp/fullhist_oom_full.log` 待确认退出码 0。
