# #719 交付报告：L19（#441 移交两条）落地 + 557 复算读数

日期：2026-08-16 ｜ 票：#719（map #695，L19）｜ 移交源：#441 关票评论「建议项移交」

## ① 桶拒绝落 other_violation_by_kind 警报桶 —— 已落地

**现状**：`OscillationCampaign::close()` 里短差桶拒绝取自归属账的在途量核销
（`write_off_unclosed` 返回 Err）此前只有 `debug_assert!`，release 侧静默（结构性不可达论证在案）。

**落地**（对齐 `cash_unsound_free_negative_unreachable` 既有先例）：
- `DeathWriteOff` 新增 `bucket_rejected: bool`；
- `record_lifecycle` 的 `Died` 臂：`bucket_rejected=true` ⟹ `other_violation_count += 1` 且落
  `other_violation_by_kind[（侧, "death_write_off_bucket_rejected_unreachable")]`；
- 新单测 `death_write_off_bucket_rejected_lands_in_other_violation_alert_bucket` 锁该通路；
- 生产判定零改动（纯 witness 警报面）。

## ② units_gap=557 按（级别, 中枢）分桶复算 —— 机制落地 + 复算读数 = 0

**机制**：`SuspensionAttribution::write_off_all` 返回逐（中枢）核销明细（私有签名内部调整）；
`CampaignBook::sync_position` 死亡分支新增观测门 `THETA_DEATH_WO_DUMP`（env_registry 登记第 51 键，
strategy 层首个登记键，观测门，默认关）：置位时逐中枢打一行
`[death_wo][#719] level=… side=… center=… units_gap=… cash_booked=…`（纯 stderr，不进产物、不改判定）。
dump 通路实证：单测 `death_write_off_applies_symmetrically_to_short_side` 置位运行，
stderr 见 `[death_wo][#719] level=0 side=Short center=CenterId{start_index:1,zd:100,zg:200} units_gap=100 cash_booked=1200`。

**复算读数（090 照实）**：

| 运行 | 树 | death_write_off_count | units_gap |
|---|---|---|---|
| #441 关票时（2026-07-27） | 当时 main | {long:6, short:2} | 557 |
| 本票复算（2026-08-16） | 当前 main HEAD（e7d0f80f31，干净 worktree 隔离并行线） | **{}（空）** | **0** |

复算命令：

```
cd rust && M8_WIN_FILTER=wf8 VOICE_EXEC=1 THETA_NEST_CERT_GATE=1 THETA_CENTER_OSCILLATION=1 \
  THETA_DEATH_WO_DUMP=1 M8_REPORT_PATH=/tmp/719_death_wo/m8_wf8.md OPSEM_DUMP_DIR=/tmp/719_death_wo/dump \
  cargo test --release --lib theta_v0::backtest::wverify_run::m8_e2e_all_systems_oos -- --ignored --nocapture
```

（首轮漏 `THETA_CENTER_OSCILLATION=1` 跑出全零属口径错误，纠正后上表读数成立；ADR 0001:295 的多窗对照行载明该键为 campaign 面必需。）

**557 → 0 的去向归因（同窗 witness 读数）**：campaign 生死照常（lifecycle_opened/died = long 42 / short 45），
但**全部干净死**——挂起在 campaign 死亡前已沿三类点清算通道收干：
`settlement_by_side` = cover_and_close 54 / write_off_unclosed 7（(1,long)/(1,short)/(2,long)/(2,short)/(3,long) 五键），
`suspension_continued_count` = 66（#414 延续机制让挂起活到三类点再清算）。
⟹ 「死亡吞挂起」这一盲区在当前 main 的 wf8 窗**不再发生**；557 是 2026-07-27 时点（#414 延续 +
后续清算链路定型前）的历史读数，当前无物可分桶。dump 机制留作任何未来窗口该读数复非零时的复算工具。

**披露**：漂移归因到「机制演进」层级（延续 + 三类点清算收干），未逐 commit 二分漂移点——
若编排者要精确漂移点（哪个 commit 使 wf8 死亡吞挂起归零），另开探针票。
