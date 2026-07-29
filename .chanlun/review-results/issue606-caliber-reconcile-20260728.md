# #606 返工：生产挂点 vs #585 普查 逐案对拍报告

- 日期：2026-07-28
- 票据：#606 S1 返工（观测挂点挪到生产 `judge_segment`，撤回诊断塔改动）
- 工位：`/tmp/wt-606`（分支 `ticket-606`）
- 对拍对象：`/tmp/research-585/results/wf8/firstclass.jsonl`（#585 普查，独立 worktree
  `/tmp/research-585/wt-run`，冻结快照 `c8e87c4a0d`）

## 1. 两口径定义

| | 本票（#606 返工） | #585 普查 |
|---|---|---|
| 挂点 | `signal.rs::judge_segment`，`judge_first_cached` 返回后、`points.push` 之前 | 同一行为锚（`signal::judge_segment`，`judge_first_cached` 已返回、`points.push` 之前只读外化） |
| 判据函数 | `t3_in_c_fixed_first_pair`（补充十五：锚 = `last_center.end_index`，固定首对，首对失败不后扫） | `trend_third_class_in_c`（全 c 窗 `[λ_C, seg.end]` 后扫，取首个成功对） |
| 锚坐标 | `last_center.end_index`（B 右边，含等值即紧邻） | `λ_C`（次级别离开起点，经 `departure_move_c_start` 定位） |
| 性质 | 生产口径提案（票面正呈候选） | 临时工程口径（用户指定，#586 未结案前的过渡探针） |

两口径均挂在同一生产调用点，共享同一批一类点（`buy1 ∨ sell1` 事件），差异只在“如何判 c 窗内是否含
T3”。

## 2. 身份键对拍

键 = `(level, source_index, side, center_start_index, center_end_index, center_zd, center_zg)`。

```text
本票记录数：59
#585 记录数：59
身份键交集：59
仅本票：0
仅 #585：0
```

**59/59 身份完全重合**——两探针捕获的是同一生产事件流（S1 版诊断塔挂点身份交集为 0，本次返工
后首次可做真实逐案对拍）。

## 3. 逐案判类对拍

| | 一致（trend_first/otherwise 同判） | 判异 |
|---|---:|---:|
| 计数 | 44 | 15 |

### 3.1 判异明细（15 点，全部逐点核对）

**类型 A：本票 `otherwise`（`SameDirection`）vs #585 `trend_first`（7 点，同一中枢）**

| level | source_index | side | center(start,end,zd,zg) | 本票判级 | #585 判类 |
|---:|---:|---|---|---|---|
| 1 | 257294 | Short | (253980,256098,4704517000000,4750400000000) | Missing(SameDirection) | trend_first |
| 1 | 257918 | Short | 同上 | Missing(SameDirection) | trend_first |
| 1 | 258016 | Short | 同上 | Missing(SameDirection) | trend_first |
| 1 | 258088 | Short | 同上 | Missing(SameDirection) | trend_first |
| 1 | 258114 | Short | 同上 | Missing(SameDirection) | trend_first |
| 1 | 258695 | Short | 同上 | Missing(SameDirection) | trend_first |
| 1 | 258787 | Short | 同上 | Missing(SameDirection) | trend_first |

归因：该中枢右边固定首对 leave/retest 同向续行（非离开/回试对），本票按补充十五「不得越过同向
续行后扫替代配对」判 `Missing(SameDirection)` 即止、不后扫；#585 口径继续在同一 c 窗内后扫，找到
更远的成功对，判 `trend_first`。此行为已由 D1 单测
`t3_in_c_fixed_first_pair_does_not_rescan_past_first_pair_failure`（`signal.rs`）机械锁定——**设计
使然，非 bug**。

**类型 B：本票 `trend_first`（`Present`）vs #585 `otherwise`（8 点）**

| level | source_index | side | center(start,end,zd,zg) | 本票 leave/retest 区间 |
|---:|---:|---|---|---|
| 0 | 121837 | Short | (121297,121625,3664060000000,3664277000000) | (121633,121837)/(121852,121910) |
| 0 | 168779 | Long | (167870,168304,4180600000000,4185708000000) | (168316,168470)/(168491,168548) |
| 0 | 191511 | Short | (190694,191302,4290000000000,4318560000000) | (191337,191424)/(191444,191511) |
| 0 | 208980 | Short | (208095,208590,4492598000000,4498826000000) | (208597,208701)/(208713,208980) |
| 1 | 160085 | Short | (157103,159192,4157138000000,4202600000000) | (159297,160085)/(160091,160187) |
| 1 | 253533 | Short | (252025,253028,4450539000000,4463378000000) | (253028,253160)/(253186,253533) |
| 1 | 253615 | Short | 同上 | (253028,253160)/(253186,253615) |
| 3 | 131929 | Short | (92368,119258,3342124000000,3474191000000) | (121297,131929)/(131967,141982) |

归因：本票锚 = `last_center.end_index`（B 右边紧邻首段即为 leave），判定即在该首对成立；#585 口径
锚 = `λ_C`（次级别离开起点，晚于 B 右边），扫描起点更靠后，跳过了本票捕获的这一对，落入 c 窗
后段扫描 → 该窗口下未获成功对 → 判 `otherwise`。此为「锚坐标差异」现象，D1 单测
`t3_in_c_fixed_first_pair_anchor_is_b_right_edge_not_lambda_c`（`signal.rs`）同源机械见证。

### 3.2 判异归因小结

15/15 判异**全部**可归因至两口径的两处已知设计差异（锚坐标选择 + 首对失败是否后扫），无一处
判异缺乏结构性解释——两口径本身在各自定义下均自洽，判异是政策差异的必然产物，不是实现缺陷。

## 4. 统计口径行（分级分侧总数 vs native/otherwise 切分）

| (level, side) | 本票合计 | #585 合计 | 合计是否一致 | 本票(native,otherwise) | #585(native,otherwise) |
|---|---:|---:|---|---|---|
| (0, Long) | 5 | 5 | ✓ | (2,3) | (1,4) |
| (0, Short) | 6 | 6 | ✓ | (4,2) | (1,5) |
| (1, Short) | 27 | 27 | ✓ | (5,22) | (9,18) |
| (3, Short) | 21 | 21 | ✓ | (11,10) | (10,11) |
| **合计** | **59** | **59** | ✓ | **(22,37)** | **(21,38)** |

四桶**总数逐一相同**（同一 59 点总体，身份键 59/59 交集已证）；native/otherwise 内部切分因判级
政策不同而不同（本票 native 多 1，因锚坐标差异类型 B 的净贡献 8 减类型 A 的净贡献 7）。两侧数字
均如实登记，互不冒充彼此的终值。

## 5. 统计口径元数据

```text
schema=otherwise_domain_wf8_reconcile_v1
git_sha(worktree,本票)=<见 commit 记录>
git_sha(worktree,#585)=c8e87c4a0d056c039bb215347adf6c21e75c408c（快照冻结）
data_sha256=btc_1m_full.json（两侧同一份 analysis/data_cache 源文件）
window=wf8(2023-08-17..2024-02-16, 264960 bars)
本票 env=THETA_OTHERWISE_DOMAIN_SIDECAR=1 + THETA_CENTER_OSCILLATION=1（开臂）
本票 pair_policy=fixed_first（锚=last_center.end_index，首对失败不后扫，补充十五）
#585 pair_policy=scan_first_success（锚=λ_C/c_start，全 c 窗后扫取首个成功对，临时工程口径）
join_key=(level,source_index,side,center_start_index,center_end_index,center_zd,center_zg)
```

## 6. 090 终结论

- **已判定**：身份键 59/59 完全重合（本票挂点与 #585 探针捕获同一生产事件流）；44/59 逐案判类
  一致；15/59 判异且全部可机械归因至两口径已知设计差异（锚坐标选择、首对失败是否后扫），非
  随机噪声、非实现缺陷；(level,side) 四桶总数逐一相同（59），仅 native/otherwise 内部切分因
  判级政策不同而不同——两侧统计口径可比性已诚实登记。
- **未能判定**（需 #586 正式裁定，非本票范围）：固定首对（本票）与临时扫描口径（#585）孰为
  37:18 教义正呈；#586 结案前，本对拍表只作两口径差异的机械见证，不裁决终值。
