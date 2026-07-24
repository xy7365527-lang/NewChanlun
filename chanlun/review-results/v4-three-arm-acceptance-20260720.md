# V4 三窗三臂实战验收跑批（门关 / v0 门 / v1 真链门）

- **日期**：2026-07-20　**工位**：验收跑批工位（worktree `/tmp/kimi-nest-mainline`，分支 `kimi-nest-mainline-20260717`，HEAD = `640609071`）
- **票据**：wayfinder #66（v1 真链 goal 最后一关）
- **性质**：只跑批不改码——`rust/src` 零改动、零 git mutation；主仓 `/Users/silencehan/Projects/NewChanlun` 全程只读。臂C = 当前 lib 实装态（V2 力度门已实装于 `rust/src/theta_v0/classifier/nest.rs:776`，`extend_typed_upward` 候选过滤合取 `!event.divergence_confirmed → continue`，设计卡 `v2-rung-force-gate-implementation-card-20260720.md`）。
- **口径标签**：执行层数值为 `[L1机制/费率未标定]` 口径（cost 三常费率保底未标定）；本验收的比较单位是**同口径三臂差分**，不作 alpha 论据、不作策略择优输入（A10 附则B）。历史数据只用于验证门机制行为与不变量，不回测验证策略（v3）。

---

## 1. 三臂定义与数据来源

| 臂 | env | 数据落盘 | 日志 |
|---|---|---|---|
| 臂A 门关基线 | `VOICE_EXEC=1` | `/tmp/m8_win/{p3fold,wf7,wf8}/trades.jsonl`（在案） | `/tmp/m8_win.out` |
| 臂B v0 门基线 | `VOICE_EXEC=1 THETA_NEST_CERT_GATE=1` | `/tmp/m8_win_gate/{...}/trades.jsonl`（在案） | `/tmp/m8_win_gate.out` |
| 臂C v1 真链门 | 同臂B env（当前 lib = V2 力度门实装态） | `/tmp/m8_win_v1/{...}/trades.jsonl`（本批新跑） | `/tmp/armC_run.log` |

跑批命令（臂C，本批执行，35.92s 通过）：
`VOICE_EXEC=1 THETA_NEST_CERT_GATE=1 OPSEM_DUMP_DIR=/tmp/m8_win_v1 cargo test --release --lib theta_v0::backtest::wverify_run::m8_e2e_all_systems_oos -- --ignored --nocapture`
（测试体 `rust/src/theta_v0/backtest/wverify_run.rs:1199`；三窗 = p3fold 2023-01-01..06-30 + anchored wf7 2023-02-17..08-16（震荡窗）+ wf8 2023-08-17..2024-02-16（趋势窗），同 config 同切片，三系统同开：M5 overlay + M6 margin/cost_model + M7 TW 账本，κ=0 冻结。）

## 2. 三臂逐窗对照

### 2.1 trades（按 `certificate.dir` 聚合；pnl = `pnl_raw_unlevered` 求和）

| 窗 | 臂 | 总笔数 | Long n / pnl | Short n / pnl |
|---|---|---|---|---|
| p3fold | A 门关 | 479 | 229 / **+13737.94** | 250 / -11684.58 |
| p3fold | B v0门 | 431 | 211 / +3626.09 | 220 / -10178.65 |
| p3fold | C v1门 | 431 | 211 / +3626.09 | 220 / -10178.65 |
| wf7（震荡） | A 门关 | 504 | 229 / **+5597.59** | 275 / **-5598.74** |
| wf7（震荡） | B v0门 | 467 | 212 / **-644.44** | 255 / **-13177.99** |
| wf7（震荡） | C v1门 | 467 | 212 / **-644.44** | 255 / **-13177.99** |
| wf8（趋势） | A 门关 | 518 | 284 / +23826.69 | 234 / -16464.99 |
| wf8（趋势） | B v0门 | 449 | 250 / +20668.46 | 199 / -8269.28 |
| wf8（趋势） | C v1门 | 449 | 250 / +20668.46 | 199 / -8269.28 |

### 2.2 execR / 三态 / MaxDD（m8 四层报告层2口径，`/tmp/m8_e2e_all_systems_oos.md` 各臂末次落盘 + 日志行）

| 窗 | 臂 | execR(net_r) | MaxDD | R(含浮盈) | LCB_OOS(R) | 三态 | 终Stage |
|---|---|---|---|---|---|---|---|
| p3fold | A | +923705 | 0.0409 | +1005147 | -827536 | INCONCLUSIVE | I(降成本) |
| p3fold | B / C | -36842 | 0.0313 | +11960 | -1153877 | INCONCLUSIVE | I(降成本) |
| wf7 | A | -1414690 | 0.0813 | -1317089 | -3320147 | 无(R≤0) | I(降成本) |
| wf7 | B / C | -2563498 | 0.1233 | -2461868 | -4956569 | 无(R≤0) | I(降成本) |
| wf8 | A | +2508870 | 0.0596 | +2884075 | -2804861 | INCONCLUSIVE | I(降成本) |
| wf8 | B / C | +3465366 | 0.0558 | +3813653 | -2064630 | INCONCLUSIVE | I(降成本) |

（B 与 C 三窗全部数值逐位相同，故合并一列；来源 `/tmp/m8_win_gate.out`、`/tmp/armC_run.log` 的 `[m8]` 行。）

### 2.3 NEST_GATE_STATS 拒绝率（`rust/src/theta_v0/backtest/runner.rs:2464` 逐窗一行）

| 窗 | 臂A（门关） | 臂B v0门 | 臂C v1门 |
|---|---|---|---|
| p3fold | 无统计（门关） | 309/1633 = 18.9%（nest_pass=1183 xzd_pass=141；rej: cert_none=45 nest_n_delta_false=30 xzd_gate_fail=234） | **逐字相同** |
| wf7 | 无统计（门关） | 291/1724 = 16.9%（nest_pass=1157 xzd_pass=276；rej: cert_none=23 nest_n_delta_false=14 xzd_gate_fail=254） | **逐字相同** |
| wf8 | 无统计（门关） | 293/1518 = 19.3%（nest_pass=1136 xzd_pass=89；rej: cert_none=30 nest_n_delta_false=23 xzd_gate_fail=240） | **逐字相同** |

### 2.4 链深构成（`certificate.nest_depth` 分布，trades 侧）

| 窗 | 臂 | depth 0 / 1 / 2 / 3 / 4 |
|---|---|---|
| p3fold | A | 153 / 159 / 89 / 70 / 8 |
| p3fold | B = C | 148 / 131 / 74 / 70 / 8 |
| wf7 | A | 162 / 160 / 101 / 53 / 28 |
| wf7 | B = C | 158 / 137 / 94 / 50 / 28 |
| wf8 | A | 207 / 146 / 101 / 44 / 20 |
| wf8 | B = C | 187 / 114 / 90 / 39 / 19 |

## 3. 核心发现：臂C ≡ 臂B 逐字节一致

- `diff` 实证：三窗 `trades.jsonl` 与 `tower_events.jsonl` 在臂B与臂C之间**全部逐字节相同**；NEST_GATE_STATS、execR、三态亦逐位相同。
- 含义照实判定：V2 力度门（`nest.rs:776` 合取 `divergence_confirmed`）在本三窗数据上**零裁剪效应**——所有实际被装配为 rung 的候选事件 `divergence_confirmed` 恒真，门未否决任何 rung，链深构成不变（若有 rung 被拒，链必收缩或重路由，trades 不可能逐字节一致）。
- 诚实边界：落盘数据无「候选被拒计数器」，故**无法区分**「存在 confirmed=false 的扩展候选但均未被选中」与「根本不存在此类候选」；可断言的只是门未改变任何实装决策。门代码本身在 lib 内（`nest.rs:776-778` 亲读确认），非「门未接线」——三窗同源同 env，唯一变量是 lib 中的 V2 合取项。
- 与在案结论一致：`nest-chain-existing-inventory-20260720.md:97-98` 记 sidecar 严格装配证书单级占 72.7%，多级链稀薄；econ 门 95.36% 信号 rungs 空（:55）。跨级候选池本就稀薄，V2 门在其上无新增裁剪与本批观测相容。

## 4. 验收线判定（issue #66 判据，照实判定不伪造）

### 4.1 wf7 震荡窗：「v1 门不再有害（Long 不翻负、Short 不亏翻倍）」——**未过 ✗**

| 判据 | 门关基线 A | v1 门 C | 判定 |
|---|---|---|---|
| Long 不翻负 | +5597.59 | **-644.44** | ✗ Long 由正翻负 |
| Short 不亏翻倍 | -5598.74 | **-13177.99**（×2.35，阈值 ×2 = -11197.48） | ✗ 亏超翻倍 |

照实结论：v1 门在 wf7 **仍然有害**，且其有害程度与 v0 门**完全相等**（C≡B）。这与 `nest-exit-gate-impl-20260719.md:174` 在案判定同向：「门对 net_r 的净效应为负（当前口径下拒绝含盈利候选）——v0 基例门鉴别力边界」。V2 力度门因零裁剪（§3）未能改变这一格局。震荡窗门害的根治不在 rung 力度合取，而在 v0 基例门（`reverse_nest_cert_base`，`exit.rs:181`）的鉴别力边界——属 E2E-N3 真链当门的目标形态，本票不越权裁定。

### 4.2 wf8 趋势窗：「不劣于 v0 门」——**过 ✓**

臂C 与臂B 全部指标逐位相同（trades/execR/三态/拒率/链深），满足「不劣于 v0 门」。附注（非判据）：对门关基线 A，v0/v1 门在 wf8 趋势窗 Long 略降（+23826.69→+20668.46）、Short 明显改善（-16464.99→-8269.28），execR +2508870→+3465366——趋势窗门净效应为正，与 wf7 震荡窗反向，方向性结论与「门在趋势窗有利、震荡窗有害」的在案图景一致。

### 4.3 p3fold（非判据窗，对照落盘）

门（B/C）对门关：Long +13737.94→+3626.09、Short -11684.58→-10178.65；execR +923705→-36842。三态三臂皆 INCONCLUSIVE。

## 5. v1 端到端判定（N2 / N3 / 状态机，按 `chanlun/plans/mainline-merged-roadmap-20260717.md:60-79` 完成态口径逐段）

| 段 | 完成态定义（roadmap 迁移表） | 判定 | 依据 |
|---|---|---|---|
| E2E-N2 跨级包含谓词 | 塔原生 `LineageEdge` 保存 `child.divergence_interval ⊆ parent.divergence_interval` 逐边见证、同 side 及 skip 级别 | **◐** | V2 rung 力度门已实装且本批验证接线无误（`nest.rs:776`，候选过滤含 `side ∧ divergence_confirmed ∧ is_sub`），但这是 **typed 装配路径（塔外）** 的严格化；设计卡明示「不宣称结算 E2E-N2」（`v2-rung-force-gate-implementation-card-20260720.md:37`），塔原生 LineageEdge 逐边见证仍缺 |
| E2E-N3 跨级链/证书 | `E2E-L` 成为 `Classification` 原生、可复验、可持久化输出 | **✗** | `NestCertificate` 仍是塔外装配物（`nest-chain-existing-inventory-20260720.md:128`）；出场侧真链仅接口预留未接入（`exit.rs:154-157`、:45，同文档 :133 第 6 条） |
| 状态机（E2E-D5 状态与因果钟 / E2E-O 算子） | 事件五钟 + 谱系两钟 + Open/Closed/Invalidated/Unresolved 谱系状态，全由同一前缀产生 | **✗** | 五钟仅注释/接口预留（`event-lifecycle-existing-inventory-20260720.md:62`：exit.rs:154-156 注释「v1 E2E-N3 真链接入后改读最深处证书 first_provable」）；WireV1 状态算子未实装 |

按 roadmap 纪律（roadmap:79 末段）：八道 `E2E-S*` 未全部给出逐字段实证前只能称「塔内近似」。本批跑批的全部产出属塔外近似口径，不抬格。

## 6. 测试验证

- 全量 `cargo test --release --lib`（worktree `/tmp/kimi-nest-mainline/rust`，本批实跑）：**1770 passed / 0 failed / 129 ignored**，与基线 1770 完全一致，零变红（日志 `/tmp/full_lib_test.log` 末行）。
- 臂C 验收跑批本身：1 passed（m8_e2e_all_systems_oos，35.92s）。

## 7. 总结论

1. **wf8 趋势窗验收线过**：v1 门不劣于 v0 门（逐字节相等）。
2. **wf7 震荡窗验收线未过（照实）**：v1 门 Long 翻负（+5597.59→-644.44）、Short 亏 2.35×（-5598.74→-13177.99），有害性与 v0 门完全等同。不伪造通过。
3. **结构性发现**：V2 rung 力度门在本三窗零裁剪（臂C ≡ 臂B），门已接线但未被数据触发；v0 基例门鉴别力边界（震荡窗门害）仍待 E2E-N3 真链当门解决，N2 ◐ / N3 ✗ / 状态机 ✗。
4. 零代码改动、零测试变红、主仓零写入、零 git mutation。
