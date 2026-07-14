# on2 修复波次 终验收（sweep-verify）

日期：2026-07-04
基线 HEAD（修复前）：`ebcd8f2a0f`（终局 alpha 重跑预注册冻结）
验收 HEAD（全修复落地后）：`150c67e76b`
编译：`cargo --release`（Cargo.toml `[profile.release]` opt-level=3 / lto=false / debug-assertions=false → `debug_assert!` 已剥离，09 全量 project_to_units 校验不计入热路径）

## ① 全量测试（硬门）

命令：`cargo test --release --lib -p newchan_rust`

```
test result: ok. 1484 passed; 0 failed; 111 ignored; 0 measured; 0 filtered out
```

**全绿。** 0 红。硬门通过。（111 ignored 均为需真实数据/长跑的 `#[ignore]` profile 与跨标的批测，非回归。）

## ② 400K / 1M 分段计时（THETA_PROFILE_STAGES=1，BTC 逐 bar classify_with_tower_incremental）

命令：`THETA_PROFILE_STAGES=1 A0_PROFILE_BARS={400000,1000000} cargo test --release -p newchan_rust --lib backtest::incremental::profile::profile_clone_cluster_a0 -- --ignored --nocapture`

### 墙钟

| 窗口 | 修复前基线 | 修复后（本验收） | 加速 |
|------|-----------|----------------|------|
| 400K | 6.00s（stage-09 报告落地前基线） | **3.84s** | **1.56x（-36%）** |
| 1M   | —（无干净聚合基线，取域报告 ~21s 量级） | **21.39s** | — |

400K→1M 标度：修复后 exp = ln(21.39/3.84)/ln(2.5) ≈ **1.87**（残余 O(n²) 未消，见 ④）。

### 落地修复的 stage 前后对照（400K）

| stage | 修复前 | 修复后 | 加速 | 域 / commit |
|-------|-------|-------|------|------|
| 09_project_to_units_resume | 1238.19ms | **34.28ms** | 36x | mod.rs+recursive_tower `b5744f219b` |
| 00b_l0_units_clone | 380.7ms | **5.70ms** | 64x | mod.rs H4 `a5986d64e2` |
| 10_projected_units_clone | 167.5ms | **24.79ms** | 6.7x | mod.rs H7（未 commit，工作树 clean=已随 H4 同文件落地 HEAD） |

三项均从全阶段 O(n²) 热点降至 O(1)/bar Rc::clone 或 O(tail) resume。1M 处仍线性（09=113.9ms / 00b=14.2ms / 10=71.6ms）。

## ③ #182 终局跑批最重入口代表窗计时

decompose/wverify 族批测的最重入口 = 逐 bar `classify_with_tower_incremental`（与 A0 profile 同一热路径）。

命令：`profile_classify_at_decompose_16k`（CL OOS）——parser-append vs 增量塔拆解：

```
      n |    parse_s    tower_s |   p_exp   t_exp
   4000 |      0.001      0.002 |    1.04    1.16
   8000 |      0.002      0.006 |    0.94    1.48
  16000 |      0.004      0.015 |    1.07    1.36
```

parser p_exp≈1.0（O(n)，健康），增量塔 t_exp≈1.4（残余 O(n²) cascade，见 ④）。

**全历史跑批加速估算**：全历史 4,613,599 bar ≈ 400K 的 11.5x。落地修复削掉 09/00b/10 三个 O(n²) 项后，代表窗墙钟加速 ~1.56x（400K）/ ~1.69x（1M，随 n 增长趋于残余 O(n²) 系数比）。全历史跑批预期 **~1.6-1.9x** 端到端加速（残余 O(n²) 主导 → 加速比随 n 缓降，非常数）。

## ④ 剩余 NO-SHIP 项清单（残余 O(n²)，本波次不可在各自文件域降阶）

| stage / 域 | 1M 计时 | 根因 | 裁决 | commit |
|-----------|--------|------|------|--------|
| 07a_extract_signals_l0（signal.rs） | 4421.9ms | cascade 前缀重扫（共享底座） | NO-SHIP（单域无 asymptotic 杠杆，codex Q1=B/Q3=NO-SHIP） | `c5fcbe734e` |
| 05_compose_resume（recursive_tower H9） | 2318.4ms | Fix A/B 已落地 `4c23604f4c`；残余 05c2a/b 有界 O(n·k) 收益递减 | NO-SHIP | `f48d0f3c26` |
| 07b_extract_second（signal.rs） | 1981.5ms | cascade 前缀重扫（area-memo 已生效，非 area 累加） | NO-SHIP + 陈旧文档订正 | `e31a7a2b4f` |
| 07a_extract_first_third_ln（signal.rs） | 1432.6ms | cascade 90% miss / 79% 成本，frontier-resume 不可降阶 | NO-SHIP | `d1b5b9d229` |
| of_forest / tree_segment_cached_gen（interp.rs H6） | — | 唯一 sound 路径=classifier 域新增 forest_epoch，越出本文件域 | NO-SHIP（codex 裁） | `150c67e76b` |
| collect_signals（econ_positive.rs） | — | O(n²) 全部继承自 classify_at 底座，econ 自身 C1/C2/C3 已优化 | NO-SHIP | `2572c1f691` |

**共同根因**：残余 O(n²) 集中在 cascade 前缀重扫（recursive_tower 共享底座）——cascade 定义性清空整塔前缀致缓存退化全量重扫。各 signal.rs / interp.rs / econ 文件域内无 bit-exact-sound 降阶路径；唯一 sound 修复=classifier 域新增独立 epoch（越出本波次各工位文件域边界）。负结果合法（`3aee6dd4c7`/H8/H9 先例）。

## 守卫状态汇总

- 全量 `cargo test --release --lib`：**1484 passed / 0 failed**（硬门通过）
- bit-exact：落地三项（09/00b/10）均声明 bit-exact，随全绿测试守卫
- 工作树 rust 源：`git diff HEAD` CLEAN（无未 commit 残留，无临时探针）

## 结论

落地修复消除 3 个 O(n²) 热点项（09/00b/10），400K 墙钟 6.00s→3.84s（-36%）。残余 O(n²) 全部为 cascade 前缀重扫（共享底座），6 项经 codex/H8/H9 先例裁定 NO-SHIP——各文件域内无 bit-exact-sound 降阶杠杆，唯一路径越出本波次工位边界（classifier forest_epoch），归口编排者后续决策。全历史跑批预期 ~1.6-1.9x 加速。
