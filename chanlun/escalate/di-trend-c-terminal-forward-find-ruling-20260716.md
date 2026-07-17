# 裁定申请：Trend 背驰 C 终段定位改正向 find（level_view.rs:691）

日期：2026-07-16 ｜ 状态：**DRAFT，待人工裁定** ｜ 关联：task #104（归因已完成）、#100/#101/#102（受波及）
归因报告：`chanlun/review-results/p104-trend-div-attribution-20260716.md`（commit `717a93e6aa`）

## 待裁定问题

`provide_divergence_pairs`（`rust/src/theta_v0/classifier/level_view.rs:691-697`）中
Trend 背驰 C 终段的选取，是否由：

```rust
segments.iter().rev().find(|segment| { ... })   // 全域最后一个同向段
```

改为：

```rust
segments.iter().find(|segment| { ... })          // 离开最后中枢后第一个同向段
```

## 事实基础（p104 漏斗 + 反事实，btc_1m_full 全量）

1. 生产口径：Trend total=995，confirmed=1；994 个 unconfirmed 全部为面积比不达标
   （min=1.876，p50=5,217.8），C 段中位 **905,921** close bars（锚到数据末端），c_gt_a=994/994。
2. 映射失败=0、终段缺失=0 —— 唯一失败源就是 C 段跨度。
3. 反事实（正向 find，c_start 沿用生产值）：**849/994 翻为 ratio<1**，C 段中位 96，
   面积比 p50=0.274；修复后 Trend confirm ≈ 85%。
4. Pan 侧对照（结构定位块内局部）confirmed 806/3,162=25.6%，证明 MACD 面积比口径本身无恙。

## 语义论证

- 模块头注释既有意图：「A/C 是同趋势方向、分属相邻两个中心锚后的**离开** episode」——
  正向 find 是注释语义的直译；rev().find 与注释矛盾，属实现 bug 而非口径分歧。
- 缠论语义：C 段 = 破最后中枢的当下离开段，其确认时点 = 次级别离开段完成时点。
  这是区间套背驰降低「滞后一个次级别走势」的前提（主线诉求）。

## 波及面（裁定生效即触发）

- N 真值、nest 证书（A46/B45）、p92/p93 系列结论全部需要重跑；
- #100 对账基线必须基于修复后口径重建（修复前对账无效）；
- #102（B 口径 Long 侧 0 张）可能被同一病灶部分解释，修复后需复查。

## 建议决议

采纳正向 find；实装顺序：改码 → p104 复跑验证（预期 confirm ≈850/995）→
p92/p93/nest 证书全量重跑 → #100 对账重启。回归口径以修复后为唯一基线。

## 裁定记录

- [ ] 裁定人：＿＿ ｜ 结论：＿＿ ｜ 日期：＿＿
