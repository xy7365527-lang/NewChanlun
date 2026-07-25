# V4 三窗三臂 typed 真链门验收报告（issue #77，goal 判据④ 主体）

- **日期**：2026-07-21　**工位**：验收跑批工位（worktree `/tmp/kimi-nest-mainline`，分支 `kimi-nest-mainline-20260717`，HEAD = `640609071dd62c85d535fb8ed71ccd6807d07a79`）
- **票据**：issue #77（SPEC #73 A 线第四票）；goal 判据④；前置 #75（进场 typed 真链门）+ #76（出场门）+ #94（STATS/CHAIN 两行分工）已落地。
- **性质**：纯验收跑数票——`rust/src` 零改动、零 git mutation、主仓 `/Users/silencehan/Projects/NewChanlun` 全程零写入。runbook = `chanlun/review-results/issue77-v4-runbook-20260721.md`（对照表已随本批回填）。
- **口径标签**：执行层数值为 `[L1机制/费率未标定]` 口径；比较单位是**同口径三臂差分**，不作 alpha 论据（A10 附则B；v3 不回测验证策略）。
- **工作树状态注记**：跑批时 worktree 存在与本票无关的既有未提交改动（`.chanlun/scene-ledger.md`、`rust/src/bin/p101_cert_bsp_tag.rs` 等，bin 非 lib）；lib 源码树干净，两臂跑的都是 HEAD 实装态。

## 1. 三臂定义与实际执行命令

| 臂 | 含义 | env | 数据落盘 | 日志 |
|---|---|---|---|---|
| 臂A 门关 | 默认路径（直通锁） | 无 nest gate env | `/tmp/v4_A/{p3fold,wf7,wf8}/trades.jsonl` | `/tmp/v4_A.out` |
| 臂B v0 基例门 | L2 点包含/v0 基例判定（不可经 env 复现，#75 起门开=typed 真链） | —（旧跑 20260720 `THETA_NEST_CERT_GATE=1`，当时门=v0 基例） | `/tmp/m8_win_gate/{...}/trades.jsonl`（旧跑在案，本批复算核对） | `/tmp/m8_win_gate.out` |
| 臂C typed 真链门 | #75/#76 真链判定（N^δ 跨级链） | `THETA_NEST_CERT_GATE=1` | `/tmp/v4_C/{...}/trades.jsonl` | `/tmp/v4_C.out` |

实际执行命令（与 runbook §执行矩阵逐字一致，两臂串行，未并行）：

```bash
cd /tmp/kimi-nest-mainline/rust
# 臂A（33.50s，1 passed）
VOICE_EXEC=1 OPSEM_DUMP_DIR=/tmp/v4_A cargo test --release --lib \
  theta_v0::backtest::wverify_run::m8_e2e_all_systems_oos -- --ignored --nocapture > /tmp/v4_A.out 2>&1
# 臂C（831.57s，1 passed；typed 真链判定使单跑量级从 ~35s 升至 ~14min）
THETA_NEST_CERT_GATE=1 VOICE_EXEC=1 OPSEM_DUMP_DIR=/tmp/v4_C cargo test --release --lib \
  theta_v0::backtest::wverify_run::m8_e2e_all_systems_oos -- --ignored --nocapture > /tmp/v4_C.out 2>&1
```

一次跑产三窗（p3fold 2023-01-01..06-30 / wf7 2023-02-17..08-16 震荡窗 / wf8 2023-08-17..2024-02-16 趋势窗），同 config 同切片，三系统同开（M5 overlay + M6 margin/cost_model + M7 TW 账本，κ=0 冻结）。

**可复现性锚点**：臂A 本批全部读数（execR、trades 分侧、链深分布）与旧跑（20260720）臂A **逐位一致**（对照 `v4-three-arm-acceptance-20260720.md` §2.1/§2.2）；臂B 旧跑读数本批从 `/tmp/m8_win_gate` dump + `/tmp/m8_win_gate.out` 逐格复算，与旧报告全部对上（§4）。

## 2. 三窗 × 三臂读数全表

### 2.1 trades 与分侧 PnL（trades.jsonl，`pnl_raw_unlevered` 按 `certificate.dir` 聚合）

| 窗 | 臂 | trades | Long n / pnl | Short n / pnl |
|---|---|---|---|---|
| p3fold | A 门关 | 479 | 229 / +13737.94 | 250 / -11684.58 |
| p3fold | B v0门（旧跑） | 431 | 211 / +3626.09 | 220 / -10178.65 |
| p3fold | C typed门 | 164 | 80 / +5066.77 | 84 / -14410.94 |
| wf7（震荡） | A 门关 | 504 | 229 / **+5597.59** | 275 / **-5598.74** |
| wf7（震荡） | B v0门（旧跑） | 467 | 212 / **-644.44** | 255 / **-13177.99** |
| wf7（震荡） | C typed门 | 213 | 108 / **-4521.68** | 105 / **-13986.31** |
| wf8（趋势） | A 门关 | 518 | 284 / +23826.69 | 234 / -16464.99 |
| wf8（趋势） | B v0门（旧跑） | 449 | 250 / +20668.46 | 199 / -8269.28 |
| wf8（趋势） | C typed门 | 174 | 92 / +23415.97 | 82 / -8662.46 |

### 2.2 execR / MaxDD / 三态（`[m8]` 行，成本真实化 net_r 口径）

| 窗 | 臂 | execR(net_r) | MaxDD | R(含浮盈) | LCB_OOS(R) | 三态 |
|---|---|---|---|---|---|---|
| p3fold | A | +923705 | 0.0409 | +1005147 | -827536 | INCONCLUSIVE |
| p3fold | B（旧跑） | -36842 | 0.0313 | +11960 | -1153877 | INCONCLUSIVE |
| p3fold | C | +468096 | 0.0606 | +541012 | -1170806 | INCONCLUSIVE |
| wf7 | A | -1414690 | 0.0813 | -1317089 | -3320147 | 无(R≤0) |
| wf7 | B（旧跑） | -2563498 | 0.1233 | -2461868 | -4956569 | 无(R≤0) |
| wf7 | C | -1847334 | 0.0937 | -1787494 | -4117492 | 无(R≤0) |
| wf8 | A | +2508870 | 0.0596 | +2884075 | -2804861 | INCONCLUSIVE |
| wf8 | B（旧跑） | +3465366 | 0.0558 | +3813653 | -2064630 | INCONCLUSIVE |
| wf8 | C | +4724613 | 0.0443 | +5028367 | -571427 | INCONCLUSIVE |

### 2.3 准入分账 NEST_GATE_STATS（拒绝率）

| 窗 | 臂A（门关） | 臂B v0门（旧跑） | 臂C typed门 |
|---|---|---|---|
| p3fold | 无统计（门关） | 309/1633 = 18.9%（nest_pass=1183 xzd_pass=141；rej: cert_none=45 nest_n_delta_false=30 xzd_gate_fail=234） | 1079/1633 = **66.1%**（nest_pass=80 xzd_pass=474；rej: cert_none=119 nest_n_delta_false=0 xzd_gate_fail=960） |
| wf7 | 无统计（门关） | 291/1724 = 16.9%（nest_pass=1157 xzd_pass=276；rej: cert_none=23 nest_n_delta_false=14 xzd_gate_fail=254） | 1044/1724 = **60.6%**（nest_pass=71 xzd_pass=609；rej: cert_none=56 nest_n_delta_false=0 xzd_gate_fail=988） |
| wf8 | 无统计（门关） | 293/1518 = 19.3%（nest_pass=1136 xzd_pass=89；rej: cert_none=30 nest_n_delta_false=23 xzd_gate_fail=240） | 1050/1518 = **69.2%**（nest_pass=51 xzd_pass=417；rej: cert_none=89 nest_n_delta_false=0 xzd_gate_fail=961） |

### 2.4 真链命中 / cross 对照 NEST_GATE_CHAIN（#94 后口径，行尾 reuse= 复用剔除量）

| 窗 | 臂B v0门（旧跑） | 臂C typed门 |
|---|---|---|
| p3fold | **旧跑读数未落盘**（20260720 无 CHAIN 行；可代注的只有 STATS 行 nest_pass/xzd_pass） | typed_found=80 typed_none=1553 xzd_fallback=1153；admit_rungs r0=75 r1=5 r2+=0；rej_rungs 全 0；cross agree=454 old_pass_new_rej=797 old_rej_new_pass=27 reuse=355 |
| wf7 | 同上未落盘 | typed_found=71 typed_none=1653 xzd_fallback=1118；admit_rungs r0=69 r1=2 r2+=0；rej_rungs 全 0；cross agree=429 old_pass_new_rej=768 old_rej_new_pass=15 reuse=512 |
| wf8 | 同上未落盘 | typed_found=51 typed_none=1467 xzd_fallback=1122；admit_rungs r0=51 r1=0 r2+=0；rej_rungs 全 0；cross agree=422 old_pass_new_rej=769 old_rej_new_pass=12 reuse=315 |

臂A 门关恒无 CHAIN 行（门关不输出门统计，同 STATS 惯例）。

### 2.5 出场侧 NEST_GATE_EXIT：本测试臂无此行（照实落账）

- `/tmp/v4_C.out` 全文 grep `NEST_GATE_EXIT` = **0 命中**（stdout/stderr 已合并重定向，非丢失）。
- 原因（亲读源码确认，非跑批缺失）：NEST_GATE_EXIT 落账点只在 `plan_and_fill_mtm`（v1 净额路径，`runner.rs:4307-4310`）与 `plan_and_fill_mtm_dual`（双仓路径，`runner.rs:4748-4751`）两 runner 内，由 `exit_gate: Option<ExitNestGateCtx>` 门开时构造（`runner.rs:4134`、`:4506`）。本测试 `m8_e2e_all_systems_oos` 走 `run_theta_v0_pi_overlay`（`runner.rs:786`，主 loop `pi_theta_fill_loop_overlay` :1933）——**overlay 主 loop 没有出场门 ctx 接线**（:2300-3160 全文无 exit_gate 引用），故出场侧统计结构性不输出。
- 结论：本票出场侧格子 = 「overlay 臂未接 #76 出场统计」。这不是验收跑批的漏取，是接线面事实；出场侧真链门的 e2e 读数需要走 v1/dual runner 的票（如 m8-dual 三臂测试 `wverify_run.rs:1436` 起的臂①②）才能落账。照实标注，不以「门关恒不输出」混充。

### 2.6 链深构成

trades 侧（`certificate.nest_depth` 归并 0/1/≥2，与旧报告 §2.4 同源）：

| 窗 | 臂 | depth 0 / 1 / ≥2 | 单级(depth 0)占比 |
|---|---|---|---|
| p3fold | A | 153 / 159 / 167 | 31.9% |
| p3fold | B（旧跑） | 148 / 131 / 152 | 34.3% |
| p3fold | C | 1 / 63 / 100 | **0.6%** |
| wf7 | A | 162 / 160 / 181 | 32.1% |
| wf7 | B（旧跑） | 158 / 137 / 172 | 33.8% |
| wf7 | C | 13 / 80 / 120 | **6.1%** |
| wf8 | A | 207 / 146 / 165 | 40.0% |
| wf8 | B（旧跑） | 187 / 114 / 148 | 41.6% |
| wf8 | C | 15 / 56 / 103 | **8.6%** |

装配/候选侧（NEST_GATE_INDEX + CHAIN admit_rungs，臂C）：

| 窗 | INDEX single_level_share | INDEX rungs_0/1/2p（indexed） | CHAIN admit_rungs r0/r1/r2+ |
|---|---|---|---|
| p3fold | **0.9579** | 91/2/2（95） | 75/5/0 |
| wf7 | **0.9605** | 73/1/2（76） | 69/2/0 |
| wf8 | **1.0000** | 55/0/0（55） | 51/0/0 |

**对照 72.7% 单级基线**（`nest-chain-existing-inventory-20260720.md:97-98`，goal 判据②「不再 95.36% 退化」同口径）：本批装配侧单级占比 **0.9579 / 0.9605 / 1.0000**——不降反升，**单级退化未消除**（goal 判据② 同口径读数照实落账；判据② 的正式对账不在本票范围，此处只供数）。trades 侧单级(depth 0)占比骤降（32-42% → 0.6-8.6%）是 survivorship 镜像：typed_found 身份桥命中率仅 ~4%（§5），准入主体是 Xzd 回退通道，其证书深度分布天然偏深——两个方向合起来说明**真链覆盖率极低，链深信号基本不由真链通道承载**。

## 3. 验收线判定（goal 判据④，照实判定不伪造）

### 3.1 wf7 硬线「臂C 不再有害」——**未过 ✗**

| 判据 | 门关基线 A | 臂B（旧跑 v0 门） | 臂C typed门 | 判定 |
|---|---|---|---|---|
| Long 不翻负 | +5597.59 | -644.44 | **-4521.68** | ✗ 翻负，且亏幅为臂B 的 7.0× |
| Short 不亏翻倍 | -5598.74 | -13177.99（2.35×） | **-13986.31（2.50×，阈值 ×2 = -11197.48）** | ✗ 超翻倍阈值，且劣于臂B |
| （参考）execR | -1414690 | -2563498 | -1847334 | 优于臂B、劣于臂A |

照实结论：typed 真链门在 wf7 震荡窗**仍然有害**——Long 由 +5597.59 翻至 -4521.68，Short 亏至基线的 2.50 倍；且 Long/Short 双侧**比 v0 基例门更有害**（execR 口径因成本项与裁单结构差异略优于臂B，不改变分侧硬线判定）。不伪造通过。

### 3.2 wf8 线「臂C 不劣于臂B」——**过 ✓**

| 指标 | 臂B（旧跑 v0 门） | 臂C typed门 | 比较 |
|---|---|---|---|
| execR(net_r) | +3465366 | +4724613 | C 优（+1259247） |
| Long pnl | +20668.46 | +23415.97 | C 优（+2747.51） |
| Short pnl | -8269.28 | -8662.46 | C 略劣（-393.18） |
| 总 trades pnl | +12399.18 | +14753.51 | C 优 |
| MaxDD | 0.0558 | 0.0443 | C 优 |
| 三态 | INCONCLUSIVE | INCONCLUSIVE | 同 |

主指标 execR 与总盘 C 全面不劣于 B；Short 分侧略劣 393.18（相对总盘 +14753.51 为零头量级），不翻转判定。**wf8 线过**。

### 3.3 p3fold（非判据窗，对照落盘）

臂C execR +468096，介于臂A（+923705）与臂B（-36842）之间；Long +5066.77 优于臂B（+3626.09），Short -14410.94 劣于臂B（-10178.65）。三态三臂皆 INCONCLUSIVE。

## 4. 臂B 旧跑读数复算核对（可复算性）

臂B = V4 旧跑（20260720）读数，本批从在案原料逐格复算：

- trades 分侧/链深：由 `/tmp/m8_win_gate/{p3fold,wf7,wf8}/trades.jsonl` 按 §2.1 同法聚合，三窗 9 格与 `v4-three-arm-acceptance-20260720.md` §2.1/§2.4 **逐位一致**。
- execR/STATS：取自 `/tmp/m8_win_gate.out:279-286`（STATS 行 :279/:282/:285；`[m8]` 行 :280/:283/:286），与旧报告 §2.2/§2.3 逐位一致。
- CHAIN/EXIT 行：旧跑日志全文无此二行（#94/#76 口径在旧跑之后才落地）——对照表中臂B 的「nest 三态分布」「CHAIN 列」照实标**「旧跑读数未落盘」**，不以 STATS 代充。

## 5. 归因：臂C 在 wf7 的有害机制（照实，不伪造通过）

1. **门从「宽筛」变「严筛」且方向单调**：拒绝率 wf7 16.9%（B）→ 60.6%（C）；cross 对照 old_pass_new_rej=768 vs old_rej_new_pass=15（`/tmp/v4_C.out:286`）——typed 门几乎严格严于 v0 基例门，裁单量级 3 倍于旧门。
2. **身份桥命中率极低，真链通道近乎空转**：wf7 typed_found=71/1724 = 4.1%（typed_none=1653）；准入主体是 Xzd 回退（xzd_fallback=1118，xzd_pass=609 vs nest_pass=71）。admit 的真链几乎全单级（admit_rungs r0=69 r1=2 r2+=0），INDEX single_level_share=0.9605——跨级链稀薄格局未变（对照 72.7% 基线，§2.6）。
3. **裁掉的候选在震荡窗携带净正 PnL（Long 侧）**：按 (entry_bar, dir) 多重集差分（近似口径，路径依赖级联未分离）——wf7 Long 侧门关 229 笔/+5597.59 → 臂C 108 笔/-4521.68，被裁约 153 笔携带约 +7468；Short 侧被裁约 190 笔携带约 -3160（裁亏单，方向有利），但留存 105 笔仍 -13986.31。净效应：Long 侧门把震荡窗的盈利候选池抽干，与 `nest-exit-gate-impl-20260719.md:174` 在案判定「门对 net_r 净效应为负（当前口径下拒绝含盈利候选）」同向，且 typed 门比 v0 门抽得更狠。
4. **出场侧对照缺位**：NEST_GATE_EXIT 在 overlay 臂无输出（§2.5），出场侧 v1/dual 对照差无法在本测试臂落账——出场机制对 wf7 有害的分担份额本票无法量化，照实标注为归因缺口。
5. **exit_type 构成**（臂C wf7 trades）：CloseShortDiff=141 / CloseRoot=68 / ReduceCore=3 / Hold=1——ShortDiff 腿反向确认主导出场，与「震荡窗反向信号密集→真链反查 miss 率高→诚实不准出/不准入」的机制图景相容。

**机制一句话**：typed 真链门把 v0 基例门的「宽进」改成「身份桥命中才准 nest 通道」，而桥命中率仅 ~4%，于是准入坍缩到 Xzd 回退；在 wf7 震荡窗，被真链否掉的候选恰好携带 Long 侧盈利，门害未被治愈反而加深（Long 侧 7.0× 于 v0 门）。这与跨级链稀薄（single_level_share ≥0.958）互为表里：真链覆盖不足时，「真链当门」等价于「近全拒+回退」。

## 6. 数字可复算索引（每个数字的出处）

| 数字 | 出处 |
|---|---|
| 臂A 三窗 execR/MaxDD/三态 | `/tmp/v4_A.out:279`（p3fold）、`:281`（wf7）、`:283`（wf8） |
| 臂A 三窗 trades/分侧/链深 | `/tmp/v4_A/{p3fold,wf7,wf8}/trades.jsonl`（479/504/518 行），聚合脚本口径见 §2.1（python json 逐行 sum `pnl_raw_unlevered` by `certificate.dir`，count `certificate.nest_depth`） |
| 臂B 三窗 execR/MaxDD/三态 | `/tmp/m8_win_gate.out:280`/`:283`/`:286`；旧报告 `v4-three-arm-acceptance-20260720.md` §2.2 同值 |
| 臂B 三窗 STATS | `/tmp/m8_win_gate.out:279`/`:282`/`:285` |
| 臂B 三窗 trades/分侧/链深 | `/tmp/m8_win_gate/{p3fold,wf7,wf8}/trades.jsonl`（431/467/449 行），本批 §4 复算逐位一致 |
| 臂C 三窗 execR/MaxDD/三态 | `/tmp/v4_C.out:283`（p3fold）、`:288`（wf7）、`:293`（wf8） |
| 臂C 三窗 STATS | `/tmp/v4_C.out:280`/`:285`/`:290` |
| 臂C 三窗 CHAIN（typed 三态/admit_rungs/cross/reuse） | `/tmp/v4_C.out:281`/`:286`/`:291` |
| 臂C 三窗 INDEX（single_level_share 等） | `/tmp/v4_C.out:282`/`:287`/`:292` |
| 臂C 三窗 trades/分侧/链深 | `/tmp/v4_C/{p3fold,wf7,wf8}/trades.jsonl`（164/213/174 行），同 §2.1 口径 |
| NEST_GATE_EXIT 零命中 | `grep -c NEST_GATE_EXIT /tmp/v4_C.out` = 0；接线面 `runner.rs:4134`/`:4307`/`:4506`/`:4748` 与 `runner.rs:786`/`:1933`（overlay 主 loop 无 exit_gate） |
| wf7 Short 翻倍阈值 | 2 × (-5598.74) = -11197.48；臂C -13986.31 → 2.498× |
| 72.7% 单级基线 | `chanlun/review-results/nest-chain-existing-inventory-20260720.md:97-98` |
| 逐笔差分归因（§5.3） | 近似口径：(entry_bar, dir) 多重集差分，组内均值摊派；路径依赖级联未分离，只作方向性证据 |

## 7. 纪律声明

- `rust/src` 零改动、`rust/Cargo.toml` 未触碰、零 git mutation、主仓零写入；仅写 `/tmp/v4_*`、`/tmp/kimi-nest-mainline/chanlun/review-results/` 内两文件（runbook 回填 + 本报告）。
- 判定照实：wf7 硬线未过即未过，归因以账面数字为限；臂B 缺格标「旧跑读数未落盘」，出场侧缺格标「overlay 臂未接线」，均不以近似值冒充。
