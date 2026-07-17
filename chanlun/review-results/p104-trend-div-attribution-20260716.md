# #104 归因：Trend 侧背驰确认仅 1/807（p104 漏斗 + 反事实）

日期：2026-07-16 ｜ 探针：`rust/src/bin/p104_trend_div_funnel.rs`（只读，含反事实段）
数据：`analysis/data_cache/btc_1m_full.json`（bars=4,613,599，as_of=4,613,598，events_total=4,157）

## 结论（一句话）

**Trend 侧背驰确认极低不是 MACD 力度口径问题，是 C 段定位 bug**：
`level_view.rs:691` 的 `segments.iter().rev().find(...)` 把 C 终段取成**全域最后一个**
同向下级段（仅受 `end_index <= as_of` 约束），最终快照下 C 段被锚到数据末端，
面积比必然天文数字，`curr < prev` 恒 false。

## 漏斗证据（生产路径复算）

- Trend：total=995，confirmed=1，map_a_none=0，map_c_none=0，diverge_false=994
  - 面积比：min=1.876，p25=778，p50=5,217.8，p75=27,312，max=2.41M；lt1=0
  - 段长（close idx 域）：a_len_p50=286，**c_len_p50=905,921**，c_len_max=2,654,343，c_gt_a=994/994
  - 分层：L1 734/0，L2 190/0，L3 56/0，L4 12/0，L5 3/1（唯一 confirmed 在最粗层，跨度巧合可比）
- Pan（对照，结构定位块内局部）：total=3,162，confirmed=806（25.6%），
  a_len_p50=56，c_len_p50=168 —— 正常量级，证明 MACD 面积比口径本身没坏。

## 病灶

`rust/src/theta_v0/classifier/level_view.rs:691-697`（provide_divergence_pairs）：

```rust
let Some(c_terminal) = segments.iter().rev().find(|segment| {
    segment.direction == direction
        && segment.start_index >= last.end_index
        && segment.end_index <= as_of
}) else { ... };
...
seg_c: (c_start, c_terminal.end_index),
```

`rev().find` = 满足条件的**最后**一段；模块头注释的意图是「相邻两个中心锚后的
**离开** episode」，应为离开最后中枢后的**第一个**同向段（正向 `find`）。
历史 block 在最终快照下 C 段一律延伸到数据末端附近，905k bar 的 C 段即由此而来。

## 反事实（只读，探针内复算）

C 终段改「`start_index >= c_start` 的第一个同向下级段」（c_start 沿用生产 interval_b.0）：

- P104_CF：total=994，no_terminal=0，map_none=0，**lt1=849（85.4%）**
- P104_CF_RATIO：min=0.001，p25=0.126，p50=0.274，p75=0.577，max=35.9；**c_len_p50=96**

即修复后 Trend confirm ≈ 850/995 ≈ 85%。

## 修复建议（待裁定，未实装）

1. `level_view.rs:691` `iter().rev().find` → `iter().find`（离开后第一个同向段）。
2. 语义含义：C 段确认时点 = 次级别离开段完成时点——这正是区间套降低确认滞后的前提，
   与「背驰滞后一个次级别走势」的主线问题直接相关。
3. 波及面：N 真值 / nest 证书（A46/B45）/ p92·p93 对账全部会变，修复后须重跑
   （#100 对账应以修复后口径为准，否则对账基线作废重来）。
4. 反事实近似口径备注：c_start 沿用生产值（departure_move_c_start 以旧 c_terminal 计算），
   正式修复后 c_start 可能微移；ratio<1 为确认的充分主判据（漏斗 ①② 复算路径）。

## 状态

归因完成；生产修复为语义变更，**待人工裁定**后另行任务实装 + 全量回归。
