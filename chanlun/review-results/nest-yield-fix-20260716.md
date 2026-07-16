# #97 nest 链产量修复：盘背进料口 + 结构性初筛 + 身份标签（2026-07-16）

对应 stop-hook 指令：修好进料口（盘整块允许进链）→ 放宽初筛（候选只看结构不看力度）→
补齐身份标签，然后重放测产量（0 不可能是市场答案），并枚举所有遗漏。

## 1. 改动清单（本次提交）

| 文件 | 内容 |
|---|---|
| `rust/src/theta_v0/classifier/level_view.rs` | 进料口修复：盘背候选在 leave→retest 对不可用（盘整块为末块、离开块未 Completed）时不再丢弃；`interval_a` 仅供 A 口径诊断，回填为盘整块自身结构跨度（再兜底 `seg_a.0..seg_c.1`）。新增 `intake_fallback: bool` 审计标记，不进入 N 真值与 B 生产口径 |
| `rust/src/theta_v0/classifier/nest.rs` | 身份标签：新增 `NestEventIdentity { level, turn_source, interval_b }`，`TypedNestCertificate` 增 `identities` sidecar（高→低、含基例、与 `kinds` 对齐），只作归因/对账，不参与证书真值 |
| `rust/src/bin/p92_nest_replay_postruling.rs` | 探针补齐：`P92_MISSED` 遗漏枚举（terminal_confirmed 全集 vs 证书覆盖对账）、`intake_fallback` 计数、identities 输出 |

## 2. 重放口径

```text
P92_INPUT bars=4613599 replay_bars=4613599 first_date=2017-08-17 04:00:00 last_date=2026-05-31 23:59:00
P92_RULE cand=dir_and_comparable_and_extreme weak=divergence_stage interval_production=B
         typed=trend_or_consolidation d3=sidecar terminal=BspBits_confirm_side
         clock=first_prefix created_at=forbidden
```

btc_1m 全量 4,613,599 根，逐 prefix 因果重放（首次可证钟 = 逐 prefix 首见，禁读 `created_at`）。
prefix 阶段耗时 ~10,774s（4157 快照全部收齐），terminal 阶段 ~574s。

## 3. 产量（0 已被否证）

```text
P92_YIELD candidates=4157 trend_candidates=995 pan_candidates=3162
          divergence_confirmed=807 trend_divergence=1 pan_divergence=806
          terminal_confirmed=21
P92_CERT  caliber_A=46 A_trend=0 A_pan=30 A_mixed=16
          caliber_B=45 B_trend=0 B_pan=22 B_mixed=23
P92_D3    edges=46 violations=23 rate=0.500000000
P92_BASELINE old=483/483/483 new_candidates=4157 new_terminal_confirmed=21 new_B_certificates=45
```

- 候选 4,157（趋势 995 / 盘整 3,162）：盘背进链后候选面扩大 ~8.6 倍（对旧口径 483）。
- 盘整背驰确认 806 例，趋势背驰 1 例——印证「盘背是市场主要供给」的预期；旧进料口把它们全部挡在链外。
- B 口径证书 45 张（上轮 #92 仅 19 张），A 口径 46 张。
- D3 跨级边 46 条中违约 23 条（rate=0.5），仍为 sidecar 诊断，不回写真值。

## 4. provider 完整性（上轮能力边界已消除）

```text
P92_PROVIDER snapshots=4158 views=55771 too_short=0 invalid_seed=0
             missing_carried_center=0 other=0 unresolved_targets=0 complete=true
```

上轮 #92 遗留的 215 个 fail-closed `InvalidSeed`（exact-three 投影）本轮为 **invalid_seed=0**：
进料口修复 + CarriedOnly 选项 1 结裁（#96, `c704d972f5`）联合把该能力边界收敛为空。
`complete=true` 首次在全量重放中成立。

## 5. 遗漏枚举（missed=0）

```text
P92_MISSED terminal_confirmed=21 covered_b=26 intake_fallback_events=2 missed=0
```

- terminal_confirmed 全集 21 例逐一与证书覆盖对账：**missed=0**，无一例被链装配丢弃。
- `intake_fallback_events=2`：全量中仅 2 例候选走了兜底跨度回填（即旧进料口会直接丢弃的实例），
  已入链参与生产，身份标签可回溯。

## 6. 无前视与 bit-exact

```text
P92_SNAPSHOT future_violations=0 created_at_reads=0 no_forward=true
P92_BIT_EXACT old_path_diff=0 tower_diff=0 moves_centers_bsp_pan_diff=0
              lifecycle_cp_ownership_diff=0 classification_total_diff=0
P92_R7 provider_complete=true definition_faithful=true snapshot_no_forward=true B_zero=false
```

旧路径（moves/centers/BSP/pan-div/tower/ownership/classification）全量终态 0 diff；
无 `created_at` 读取、无随机源、无挂钟。R7 三项全 PASS 且 `B_zero=false`，零产量签字分支未触发。

## 7. 测试

`cargo test --lib`：**1638 passed; 0 failed**（较 #96 基线 1637 +1，为身份标签新钉点）。

## 8. 结论

进料口 + 初筛 + 身份标签三项修复后，产量从「B=19、215 窗 fail-closed」修复为
**B=45、invalid_seed=0、missed=0、complete=true**。stop-hook 的三项要求全部落地，
「0 不可能是市场答案」已由 807 例背驰确认与 45 张 B 证书实证否定。
