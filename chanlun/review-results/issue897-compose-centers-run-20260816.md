# #897 留痕：compose 中枢收集修正（M-1 落地）——实测读数与改前改后对拍

- 日期：2026-08-16
- 票据：#897 主项（compose 只存一个中枢 → 按 M-1 收集全部相应级别中枢）
- commit：`92dfb32c1d`（main，镜像同步）

## 先查结论（票面「先查再改」义务的交代）

1. **拿得到**：`compose_level` 调用点本就有该级全部窗口中枢（`centers: Vec<Center>`）；
2. **教义分组在案**：`decompose.rs` MoveBlock 折叠（`classify_relation` 关系链）即「走势类型 = maximal 同标签中枢连续段」的现役实现，不需新造；
3. **语义裁量点**（本票判定，照实登记）：run **只沿同向延续链**（Up/DownContinuation，M-2 外缘分离判趋势）延伸，**LevelExpansion 一律断链**——依据 = M-2 转正「核心分离但外缘仍重叠＝中枢扩展、升一级，不是趋势」+ M-1「盘整＝只含一个中枢」。扩展链成员保持单中枢载荷 + 端点兼容缝（B2 夹具族行为逐位不变）；
4. **bit-exact 保全**：run 计算只用 ≤ 本窗的前缀关系（全量/增量两路径逐位一致）；`compose_level_resume` 新增 `prefix_centers` 入参（前缀+tail 拼接视图）。

## 实测读数（验收核心，BTC 尾 300K bar / 全量 461 万，显式有效域）

探针：`classifier/tests/pipeline_geometry.rs::issue897_centers_run_distribution_btc`（#[ignore]，release 1.44s）。

**① Compose.centers 长度分布**（修复前恒 1）：

| 级别 | len=1 | len=2 | len=3 |
|---|---|---|---|
| L1 | 549 | 24 | 1 |
| L2 | 119 | 13 | 0 |
| L3 | 24 | 2 | 0 |
| L4 | 4 | 1 | 0 |

**② rmove_dir 对拍**（新 M-2 序列判据 vs 旧端点兼容缝）：**一致 722 / 改判 15**；多中枢 Compose 共 **41 个，全部经 M-2 判出方向**（41/41）——S4-a 三臂自此有真实区分力，#870 重测前提达成。

**③ wf8 e2e**（全量 461 万 bar，金标准逐字节硬门）：**绿**（73.52s）。

## 验收逐条

- [x] compose 按 M-1 修正（拿得到，已改；非「拿不到」分支）
- [x] centers 长度分布实测（上表①）
- [x] rmove_dir 前后差异实测（上表②）
- [x] 两处押注转正——**查证已在此前落地**：Turn.lean:36 已追认 M-3（#877 关票后）；rmove_dir 名分已 `[旧缠论]`（#815 M-2 + 020:58）；「接受集互不包含」订正已在 cand_predicate.rs:110 在案（残余两处命中为 review-results 历史文档，按 ADR 0012 裁定三不追溯更新）
- [x] `cargo build --release` + `cargo test --release`：2682/0/152（与 debug 差 3 件 = debug_assert 固有闸，非本 diff）；debug 全量 2686/0/152 含新增锁 `compose_trend_run_payload_collects_continuation_centers`
- [x] fmt 0 Diff；白名单守卫绿

## 失败率拆因纪律的交代

票面纪律「失败率必须拆成因（A/B/C + C 桶条件号）」针对的是 #846 形态的重测读数——**那是 #870 的活**（三臂重测），本票交付的是它的前置（载荷与判据链）。本票读数（分布 + 对拍）按票面验收原文给全。
