# #384 终验报告：enabled=true 双臂 wf8——亏损入账 / 清算两终局 / 多空对称 / 阶段推进见证

- 票：#384（parent #379 图，图的最后一张票；blocked-by #380 / #381 / #366 / #383 全部已关）
- 日期：2026-07-27
- 执行位：验收工位（**不写码**）。本轮未改 `rust/src` 任何文件，未执行任何 git commit/push/branch。
- 口径来源：ADR 0001 补充五 / 补充七 / 补充八 / 补充九 / 补充十 / 补充十一 / 补充十二；
  #380 / #381 / #366 / #383 四张已关票的 resolution 评论；#384 票体 + 2026-07-27 追加评论
  （Forfeit 三源账目对齐）。
- 090 纪律：**零读数、负读数、对拍差异一律照实进表**；发现不一致不修代码，只定位。

---

## 0. 环境与纯态核验（ADR 补充九「流程教训」的前置动作）

| 项 | 读数 |
|---|---|
| HEAD | `02399d7e0b9bc3ff0d25de0682a21acffb0d67e4`（docs(campaign): #441 复审必办+建议3 doc 订正） |
| `git status --short rust/`（实测**后**复核，终态） | `M rust/src/bin/p107_level_calib.rs`<br>`M rust/src/theta_v0/backtest/wverify_run.rs` |

### 0.1 ⚠️ 工作区非纯态——两个未提交改动，逐项定性

**① `rust/src/bin/p107_level_calib.rs`**（+51 行，mtime 2026-07-25 03:51）

不污染。它是 `[[bin]]` target；本轮全部实测走 `cargo test --lib` / `--release --lib`，
只编 lib target，不链接任何 bin。

**② `rust/src/theta_v0/backtest/wverify_run.rs`**（+5 行，mtime **2026-07-27 12:34:54**）

⚠️ **该改动落在本轮全部实测之前 66 秒，本轮所有读数均包含它。** 时间线：

| 时刻 | 事件 |
|---|---|
| ~12:33 | 本工位首次 `git status --short rust/` ⟹ 当时**只有** p107（该改动尚未落盘） |
| **12:34:54** | 另一 session 写入 `wverify_run.rs`（+5 行） |
| 12:36:00 | 臂 A（默认关）dump 落盘 |
| 12:36:32 | 臂 B（enabled=true）dump 落盘 |
| 12:37:59 | `cargo test --lib`（debug）完成 |
| 12:39:05 | `cargo test --release --lib` 完成 |

**改动全文（有效行仅 2 行，其余为注释）**：

```rust
// 格式串：
+                 profit_ready_but_suspended={:?} \
// 参数：
+                w.profit_ready_but_suspended,
```

**定性：不影响本报告任何对拍结论，但影响一项读数的可得性。** 依据三条：

1. 改动位于 `#[test] fn m8_e2e_all_systems_oos` **函数体内**（`wverify_run.rs:1363` / `:1402`），
   是 `eprintln!` 的格式串与参数——不在任何生产路径上，连 lib 生产代码都不是；
2. 行为化验证：本轮双锚 `trades.jsonl` / `tower_events.jsonl` 与金标准**逐字节一致**、
   两臂四层报告文件**零字节差异**（§1.2 / §1.3）——若该改动动了任何决策量，这三个 `cmp`
   不可能全过；
3. 唯一后果是 §2.4 的 `profit_ready_but_suspended` 读数**因为这个改动才被打印出来**——
   它此前不在报告行里（#383 关票明文登记「`profit_ready_but_suspended` 未进报告行，
   #384 终验需补列」）。本报告该项读数**依赖未提交改动**，此点必须随读数一并转述。

**照实登记**：ADR 补充九的「实测前核 lib 树干净度」纪律，本轮**执行了但只执行了一次**
（开跑前），未在实测后复核——差 66 秒就会在报告里留下「lib 树干净」的错误声明。
该纪律的正确形式见 post-mortem §3.4。

---

## 1. 双臂对拍（票体验收 1）

### 1.1 跑法（两臂只差一个 env）

```bash
# 臂 A：默认关（config.rs:303-307 center_oscillation.enabled 默认 false）
M8_WIN_FILTER=wf8 VOICE_EXEC=1 OPSEM_DUMP_DIR=/tmp/fin384_off_dump \
  cargo test --release --lib theta_v0::backtest::wverify_run::m8_e2e_all_systems_oos -- --ignored --nocapture

# 臂 B：enabled=true
M8_WIN_FILTER=wf8 VOICE_EXEC=1 THETA_CENTER_OSCILLATION=1 OPSEM_DUMP_DIR=/tmp/fin384_on_dump \
  cargo test --release --lib theta_v0::backtest::wverify_run::m8_e2e_all_systems_oos -- --ignored --nocapture
```

窗口：`wf8 = BTC anchored i=8，test = 2023-08-17..2024-02-16，264960 bar`。两臂各 `1 passed`，
耗时 13.59s / 17.77s。

### 1.2 金标准对拍（默认关臂 vs 金标准）

金标准位置 = `/tmp/vocab_align_dump/`（ADR 补充九尾句 + `.chanlun/review-results/`
`oscillation-campaign-wiring-20260727.md:105-106` / `oscillation-criteria-alignment-20260727.md:98-99`
三处独立指认同一物；文件 mtime 2026-07-25 23:49，本轮**只读不改**）。

| 对拍 | 命令 | 结果 |
|---|---|---|
| trades | `cmp /tmp/fin384_off_dump/trades.jsonl /tmp/vocab_align_dump/trades.jsonl` | **IDENTICAL**（948340 B，md5 `7c304b17a3330f2739cf5ed22fc61ad4`） |
| tower_events | `cmp /tmp/fin384_off_dump/tower_events.jsonl /tmp/vocab_align_dump/tower_events.jsonl` | **IDENTICAL**（354946 B，md5 `7119ffdb177855c865e1dba5e365b81b`） |

字节数 948340 / 354946 与 ADR 补充九所载**逐位相同**。

### 1.3 开启臂 vs 关臂（旁路账本不改主决策）

| 对拍 | 结果 |
|---|---|
| `cmp /tmp/fin384_on_dump/trades.jsonl /tmp/vocab_align_dump/trades.jsonl` | **IDENTICAL**（开启臂也与金标准逐字节一致） |
| `cmp /tmp/fin384_on_dump/tower_events.jsonl /tmp/vocab_align_dump/tower_events.jsonl` | **IDENTICAL** |
| `cmp` 两臂 `center_lifecycle.jsonl`（460177 B） | **IDENTICAL** |
| `diff` 两臂四层报告 md（`/tmp/fin384_off_m8report.md` vs `/tmp/fin384_on_m8report.md`） | **无差异（整文件逐字节相同）** |

### 1.4 四层报告读数（两臂同值）

| 窗 | n_orders | ΣN_tΔP_t | Comm+Slip | Funding | Borrow | LiqLoss | **execR** | **MaxDD** | 声部(A/S/F) | 终Stage | Q_T | W_T | η_T/η_* | cum_holding_cost | η_corrected | **R** | **LCB_OOS(R)** | 三态 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| wf8 | 1217 | +6412487 | 1992111 | 379893 | 0 | 0 | **+4040483** | **0.0917** | 462/230/300 | I(降成本) | 28721770 | 0 | 8802360/28721770 | 379893 | 8422467 | **+4417092** | **−1921382** | INCONCLUSIVE |

ADR 补充九「四层报告轴」的唯一教义断言 —— **开启臂与关臂逐位相同** —— **成立**（四项
execR/R/MaxDD/LCB(R) 逐位相同，且整份报告文件无一字节差异）。

绝对值与补充九勘误后所载基线 `execR=+4040483 / R=+4417092 / MaxDD=0.0917` **逐位相同**。
（关于「−20144870 基线」争议，见 §5 缺口 G9，本轮未展开复检。）

**结论：验收 1 PASS。**

---

## 2. 见证读数全表（票体验收 2；产物级，零读数照列）

全部读数来自臂 B 单行产物级打印（`wverify_run.rs:1345-1400` 的 `[m8][#357]` 行），
原始行见 §6 附录，日志 `/tmp/fin384_on_run.log`。臂 A 对应行**全部为空/0**（`enabled=false`
时 witness 恒空），已核，不重复列表。

### 2.1 触发面

| 轴 | 读数 |
|---|---|
| `trigger_attempts` | 1437 |
| `dropped_center_not_alive` | 113（7.9%） |
| `dropped_other_trigger`（FlatSignal + PriceOutsideZone） | 961 |
| `dropped_center_moved_down`（#366 新判据专桶） | 37 |
| 落地触发（推算 1437−113−961−37） | **326** |

平账与 #366 关票所载 `113+961+37+326=1437` **逐条相同** ⟹ 判据层自 #366 关票以来无漂移。

### 2.2 动作面 `action_by_level`（键 = 级别 × 侧 × 动作）

| 级别 | long reduce | long replenish | short reduce | short replenish |
|---|---|---|---|---|
| L1 | 126 | 10 | 74 | 8 |
| L2 | 78 | 3 | 23 | 3 |
| L3 | 1 | 1 | 24 | **0（无键）** |
| 小计 | 205 | 14 | 121 | 11 |

- 总计 **351**；**多头 219 / 空头 132**（两侧不得相加，此处并列为多空对称读数，非求和口径）。
- reduce 326 / replenish 25 ⟹ **减/补尝试比 13.04 : 1**。
- ⚠️ 口径注：`record_action` 在 `apply_action` **之前**无条件调用（`fill.rs:1041`），
  故本桶是**尝试数**不是落账数。reduce 计数 326 与 §2.1 落地触发 326 数值相等，本报告
  **不判定**其是否为构造性恒等（未追接线），只照实并列。

### 2.3 拒绝面（`record_violation`）

| 轴 | long | short | 合计 |
|---|---|---|---|
| `no_active_campaign_count` | 207 | 130 | 337 |
| `defense_units_exceed_current_holding_count`（#380 项二） | 1 | **0（无键）** | 1 |
| `resource_exhausted_holding_negative_count` | **0** | **0** | **0（空 map）** |
| `replenish_triggered_but_full_count`（#380 项四「触发但货满」） | **0** | **0** | **0（空 map）** |
| `replenish_foreign_center_count`（#414 项三「异中枢回补被拒」） | **0** | **0** | **0（空 map）** |
| `earning_cash_unsound_count`（#383 硬门，恒 0 即正常） | **0** | **0** | **0（空 map）** |
| `earning_sizing_rounds_to_zero_count` | **0** | **0** | **0（空 map）** |
| `other_violation_count`（唯一警报轴，硬断言恒 0） | — | — | **0 ✅** |
| `other_violation_by_kind` | — | — | **空 map** |
| **拒绝合计** | 208 | 130 | **338** |

⟹ **成功落账 = 351 − 338 = 13 次**（推算值，非产物级；witness 无「Ok 分支计数」轴，
见缺口 G8）。即 **96.3% 的动作尝试被「该侧无仓」拒绝**——这是下面一组零读数的结构根因。

### 2.4 成功落账路径读数（基数 13）

| 轴 | 读数 | 票体对应 |
|---|---|---|
| `loss_round_trip_accounted_count`（亏损往返如实入账） | **{}（零读数，两侧皆无）** | 验收 2 第 1 条 |
| `cover_by_side`（挂起冲抵归属，same_center / other_center） | **{}（零读数）** | 验收 2 第 5 条 |
| `stage_recover_capital_count` / `stage_enter_earning_count` | **0 / 0** | 验收 2 第 4 条 |
| `stage_events` | **[]（空）** | 验收 2 第 4 条 |
| `profit_ready_but_suspended`（#383 登记 #384 需补列） | **{}（零读数）**<br>⚠️ 该项**依赖未提交改动**才被打印，见 §0.1 ② | ADR 补充十 |
| `earning_mode_switch_bar` / `earning_replenish_count` / `earning_units_gained` | **{} / {} / {}** | #383 报告层三项 |

⚠️ `loss_round_trip_accounted_count` 的严格语义（#380 关票登记、#384 按此措辞）：
「**落账后桶现金为负的动作次数**」，**不恒等于亏损往返笔数**。本窗读数为 0，含义是
「13 次成功落账中无一次落账后桶现金转负」，**不等价于**「本窗无亏损往返」。

⚠️ `realized_cash` 未在 witness 打印轴上单独外化（票体写「realized_cash 含负如实」）。
可外化的代理读数是 `unclosed_write_off_cash_booked` / `death_write_off_cash_booked`
（见 §2.6），二者本窗均为**正值**。见缺口 G7。

### 2.5 挂起终结：来源轴 vs 终局轴（票体验收 2 第 2/5 条）

**来源轴 `suspension_by_source`（键 = 级别 × 侧 × 来源，#442 加了 level 维）**

| 键 | 读数 |
|---|---|
| (1, long, `broken_by_third_class_buy`) | 3 |
| (1, long, `rebase_vanished`) | 3 |
| (1, long, `reset`) | 44 |
| (2, long, `broken_by_third_class_buy`) | 1 |
| (2, long, `rebase_vanished`) | 1 |
| (1, short, `broken_by_third_class_sell`) | 1 |
| (1, short, `rebase_vanished`) | 3 |
| (1, short, `reset`) | 22 |
| (2, short, `rebase_vanished`) | 1 |
| (3, short, `reset`) | 1 |
| **小计** | **long 52 / short 28（合计 80）** |
| `superseded` 子桶 | **不存在（#414 随变体删除退役）** |

**延续轴 `suspension_continued_count`（#414，事件次数非挂起条数）**

| 键 | (1,long) | (1,short) | (2,long) | (2,short) | (3,short) | 合计 |
|---|---|---|---|---|---|---|
| 读数 | 44 | 23 | 14 | 3 | 1 | **85** |

⚠️ ADR 补充十一读数口径注：该轴是**事件次数**，重基可对同一条挂起重复计数，
精确条数需按挂起 ID 去重——本轮无去重手段，照实按事件次数呈报。

**终局轴 `settlement_by_side`（票体「两终局分开呈报」的直接读数）**

| 终局 | long | short | 合计 | 占比 |
|---|---|---|---|---|
| `cover_and_close`（三买回补闭合 / 空头三卖镜像） | 4（L1:3, L2:1） | 1（L1:1） | **5** | 6.25% |
| `write_off_unclosed`（三卖未闭合减出核销） | **0（无键）** | **0（无键）** | **0** | **0%** |
| `forfeit`（Reset / RebaseVanished 两源） | 48（L1:47, L2:1） | 27（L1:25, L2:1, L3:1） | **75** | 93.75% |
| 小计 | 52 | 28 | **80** | |

**两轴对账（ADR 补充九「两桶分侧总数恒相等」）：**

| 对账项 | 来源轴 | 终局轴 | 结果 |
|---|---|---|---|
| long 总数 | 52 | 52 | ✅ |
| short 总数 | 28 | 28 | ✅ |
| `cover_and_close` 5 ↔ 三类点两源 | `broken_by_third_class_buy` 4(long) + `broken_by_third_class_sell` 1(short) = 5 | 5 | ✅ |
| `forfeit` 75 ↔ 结构异常两源 | `reset` 67 + `rebase_vanished` 8 = 75 | 75 | ✅ |

**三个终局全部对上，无悬空。**

### 2.6 核销两路径（中枢死 vs campaign 死，分列不混计）

| 轴 | long | short | 来源 |
|---|---|---|---|
| `unclosed_write_off_count`（#366 三卖核销） | **{}（零）** | **{}（零）** | ADR 补充七 |
| `unclosed_write_off_units_gap` | **{}** | **{}** | 货缺口 |
| `unclosed_write_off_cash_booked` | **{}** | **{}** | 现金盈余（与货缺口不相减） |
| `unclosed_write_off_nothing_to_settle` | **{}** | **{}** | |
| `death_write_off_count`（#441 死亡吞挂起） | **6** | **2** | ADR 补充十二 |
| `death_write_off_units_gap` | **557** | **75** | 货缺口（units） |
| `death_write_off_cash_booked` | **17,516,044** | **3,028,302** | 现金（**整数美元**，非分），与货缺口分列不冲销 |

⚠️ 单位口径：`cash_booked` 的量纲是**整数美元**——生产调用点 `fill.rs:2655` 传 `px as i64`
（原始价格截断成整数）。旁证：17,516,044 ÷ 557 = **31,447**，正是本窗 BTC 价位；
若为「分」则单价 314.47 美元，与 BTC 不符。该量纲论证由并行工位完成，本报告转引
（见 §8 交叉验证；`shortdiff-ledger-final-20260727.md` §3.1）。

⚠️ 两侧**不得相加**：常见误读是把 `6+2=8` 与多头侧独有的 `557` / `17,516,044` 并列成
「8 次 / 557 units / 17,516,044」——这违反 ADR 补充九的分侧条款，8 与后两个数不同侧。

「未闭合减出」两条清算路径中，**中枢终结路径零读数、campaign 死亡路径 8 笔有读数**。

### 2.7 生死轴与多空对称（票体验收 2 第 3 条）

| 轴 | long | short | vs ADR 补充九（#381 时点） |
|---|---|---|---|
| `lifecycle_opened` | **224** | **85** | **逐条相同（零漂移）** |
| `lifecycle_died` | **223** | **85** | **逐条相同（零漂移）** |
| `campaign_active_end` | 合计 **1** | | = 224−223 ✅ 自洽 |

多空两侧 campaign 对称读数汇总（**两侧不得相加**，并列呈报）：

| 维度 | long | short |
|---|---|---|
| 开局 / 死亡 | 224 / 223 | 85 / 85 |
| 动作尝试（reduce / replenish） | 205 / 14 | 121 / 11 |
| 无仓拒绝 | 207 | 130 |
| 挂起终结（来源轴 = 终局轴） | 52 | 28 |
| 收手闭合 `cover_and_close` | 4 | 1 |
| 作废 `forfeit` | 48 | 27 |
| 死亡核销笔数 / 货缺口 / 现金 | 6 / 557 / 17516044 | 2 / 75 / 3028302 |
| 亏损入账 | 0 | 0 |
| 防线超卖拒绝 | 1 | 0 |

两侧**均有非零读数、均分侧成桶、无跨侧求和**——ADR 补充九「无例外字段：本补充下的全部
witness 计数轴均已分侧」在本窗产物级成立。

**结论：验收 2 —— 读数全部产物级、全部照实呈报（含 12 项零读数）。但票体点名要见证的
四项里有三项本窗为零（亏损入账 / 三卖核销 / 阶段推进），见 §5 缺口清单。**

---

## 3. 测试矩阵（票体 Acceptance）

| 臂 | 命令 | 结果 |
|---|---|---|
| debug | `cargo test --lib` | **2095 passed / 0 failed / 141 ignored** ✅ |
| release | `cargo test --release --lib` | **2093 passed / 0 failed / 141 ignored** ✅ |
| 重型窗口（关臂） | `M8_WIN_FILTER=wf8 ... m8_e2e_all_systems_oos -- --ignored` | **1 passed** ✅ |
| 重型窗口（开臂） | 同上 + `THETA_CENTER_OSCILLATION=1` | **1 passed** ✅ |

- debug 与 release 差 2 条 = `debug_assertions`-only 用例，非回归。
- `other_violation_count == 0` 的硬断言在**开启臂**内实跑通过（`wverify_run.rs:1428`）。
- ⚠️ 四行结果**均含** §0.1 ② 的未提交改动（+2 有效行，测试函数内的 `eprintln!` 增列）。
  该改动不新增/不删除任何测试用例，两侧 passed/ignored 计数与 #383 关票所载
  （2083/2081、141 ignored）的差额来自其后落地的 #414/#441/#442 三票，非本改动。

### 3.1 慢锁（`#[ignore]`）照实标注

`rust/src` 共 **195 处 `#[ignore]`**，本次两侧各 **141 ignored**（数量两侧一致）。
与本票直接相关的重型窗口锁：

| 测试 | ignore 理由（原文） | 本轮是否跑 |
|---|---|---|
| `wverify_run::m8_e2e_all_systems_oos` | 四层报告端到端；需 BTC 全历史 | **跑了（双臂）** |
| `wverify_run::flip_guard_wf8_onebar_prune_replay` | `#270 wf8 翻向守卫回归；需 BTC 数据（DATA BLOCKER 不伪造）` | 未跑（不在票面范围） |
| `wverify_run::center_lifecycle_wf8_events_replay` | `#291 wf8 中枢生命周期事件自证；需 BTC 数据` | 未跑（不在票面范围） |
| `runner::m8_treasury_reach_distribution_real_btc` | `M8 treasury 多窗 Reach；需 BTC 全历史（329MB）；重（12 窗 O(n²)）` | **未跑**——即票体「多窗推广读数」的现成入口，见缺口 G6 |

短差本体三个模块（`oscillation_campaign.rs` / `center_oscillation_trade.rs` /
`short_diff_bucket.rs`）**内部零 `#[ignore]`**——其全部单测都在上表两侧全绿数里，
无慢锁掩盖。

---

## 4. 遗留 fog 读数收集（票体验收 3；只收集，不处置）

### 4.1 多窗推广

**零覆盖。** 本轮按票面纪律只跑 wf8 单窗（`M8_WIN_FILTER=wf8`）。
`m8_e2e_all_systems_oos` 的完整窗清单为 `p3fold` + anchored 前两窗（wf7/wf8），
另有 `m8_treasury_reach_distribution_real_btc`（12 窗）为现成的多窗入口。
**处置（跑不跑、跑几窗）由编排 session 裁定。**

### 4.2 短差资金参数校准

**无独立读数轴。** 可作校准输入的间接读数（全部来自本窗）：

| 证据 | 读数 | 对校准的含义 |
|---|---|---|
| 减 / 补尝试比 | 326 : 25 = **13.04 : 1** | #366 判据变严后回补机会稀缺；与 #366 关票所载「减/补 14.2:1」同量级 |
| 成功落账率 | 13 / 351 = **3.7%** | 96.3% 被「该侧无仓」拒——短差账本在 wf8 上几乎不落账 |
| campaign 存活 | 开 224+85、死 223+85、终局在场 1 | 持仓极短（wf8 92.3% 交易持仓 1 bar，`wverify_run.rs:1475`），campaign 开了即死 |
| 阶段推进 | 全零，`profit_ready_but_suspended` 也为零 | 连「利润够但挂起非空被拦」都未发生 ⟹ 距阶段 II 门槛远，不是被挂起卡住 |
| I_0 / Q_T | notional_in = 28721770，W_T = 0 | 到 0 判据（挂起空 ∧ free≥本金）的分母量级 |
| 桶现金终态 | **无产物级读数**（witness 不打印桶现金） | 见缺口 G7 |

---

## 5. 缺口清单（照实，含零读数与不一致）

| # | 缺口 | 证据 | 性质 |
|---|---|---|---|
| **G1** | **亏损入账见证零读数**——票体验收 2 第 1 条要求见证的「亏损入账计数（realized_cash 含负如实）」本窗为 0 | `loss_round_trip_accounted_count={}` | 结构根因已定位：成功落账仅 13 次（96.3% 被 `NoActiveCampaign` 拒），无一次落账后桶现金转负。**非 bug，但票面要的实测见证未取得**——现有覆盖仅 #380 的单测 |
| **G2** | **三卖未闭合减出核销零读数**——两终局中只有 `cover_and_close` 有实测（5 次），核销终局 0 次 | `settlement_by_side` 无 `write_off_unclosed` 键；`unclosed_write_off_*` 四轴全空 | #366 关票已声明「核销路径 wf8 零读数，单测覆盖，不得声称实测验证」——本轮**复现该零读数**。根因：本窗无「多头遇三卖 / 空头遇三买」的反向类点命中挂起 |
| **G3** | **阶段推进全零**——票体「stage_event（若有到 0）」的「若有」分支为否 | `stage_events=[]`、`stage_*_count=0`、`profit_ready_but_suspended={}`、`earning_*` 全空 | 与 #380/#383 关票登记一致（生产样本依赖重型窗口）。阶段三整条通道在 wf8 上零覆盖 |
| **G4** | **「触发但货满」零读数** | `replenish_triggered_but_full_count={}` | ADR 补充九载 #381 时点为 `{"short":1}`，本轮归零。#366 判据改后跨版本不可比，非回归判定 |
| **G5** | **#414 修复的正面读数与被禁路径同为零**——`cover_by_side` 与 `replenish_foreign_center_count` 双双为空 | 两轴皆 `{}` | #384 评论所引「Forfeit 被异中枢回补静默冲抵」的产物级证据 `cover_by_side ("long","other_center"):3` 在当前 HEAD **不再复现**。但**禁止路径**（异中枢回补被拒）也为 0 ⟹ **#414 的修复在 wf8 上无正面读数可验证**，只有「两边都没发生」。已由 #414 关票，此处仅登记可观测性事实 |
| **G6** | **多窗推广零覆盖** | 只跑 wf8 | 票面纪律所限。现成入口 `m8_treasury_reach_distribution_real_btc`（12 窗）未跑 |
| **G7** | **`realized_cash` 与桶现金终态无产物级外化** | witness 打印轴中无该两项 | 票体写「realized_cash 含负如实」；现只能经 `*_write_off_cash_booked`（本窗均正）间接读。短差资金校准（fog）缺关键读数轴 |
| **G8** | **「成功落账次数」非产物级** | witness 无 Ok 分支计数轴 | 本报告的 13 由 `Σaction_by_level(351) − Σviolations(338)` 推算。该量是 G1/G3/G5 一组零读数的分母，值得独立成轴 |
| **G9** | **wf8 四层基线的历史争议未复检** | 本轮当前 HEAD 干净态实测 **+4040483 / +4417092 / 0.0917**，与 ADR 补充九勘误后所载**逐位相同** | 另一线记录曾载「补充九勘误在 29d0faa4b8 / 3ed7912ae7 两纯态点复现不出 +4040483」。本轮**未在那两个历史提交点复检**（需 git checkout，超出本票纪律范围）。本轮结论只覆盖 `02399d7e0b` 一个点，**不构成对那条记录的证否** |
| **G10** | **ADR 补充九的 wf8 实测对照表大面积过期** | 见下 §5.1 | #384 票体的 blocked-by 只列 #380/#381/#366/#383，但其后又落了 **#414 / #441 / #442** 三票，改了键形状与语义 |
| **G11** | **Forfeit 剩余两源清算口径未裁** | `forfeit` 仍占清算 **93.75%**（75/80），来源 `reset` 67 + `rebase_vanished` 8 | ADR 补充十一明文「Reset/RebaseVanished 两源保留现口径+如实呈报，其清算是否同族**另议**」——该 fog 未闭，且在本窗是**清算的绝对主桶** |
| **G12** | **本轮实测在非纯态工作区完成**——`wverify_run.rs` 有 +5 行未提交改动（12:34:54 落盘，早于全部实测 66 秒） | §0.1 ②；改动为测试函数内 `eprintln!` 增列 `profit_ready_but_suspended` | 已三条依据定性为「不影响对拍结论」（其中一条是行为化的：三个 `cmp` 全过）。但**流程上是非纯态实测**，且 §2.4 该项读数依赖它。若编排 session 要求纯态复跑，本报告除 `profit_ready_but_suspended` 一项外的全部读数应可原样复现 |

### 5.1 ADR 补充九 wf8 实测对照表的逐项时效（G10 明细）

| 补充九所载（#381 时点） | 本轮实测 | 判定 |
|---|---|---|
| `lifecycle_opened={"long":224,"short":85}` | **完全相同** | ✅ 零漂移 |
| `lifecycle_died={"long":223,"short":85}` | **完全相同** | ✅ 零漂移 |
| `other_violation_count=0` | **0** | ✅ |
| `resource_exhausted_holding_negative={}` | **{}** | ✅ |
| `action_by_level` 多头 344 / 空头 303 | 多头 219 / 空头 132 | ⚠️ #366 判据改 ⟹ **跨版本不可比**（#366 关票已声明） |
| `no_active_campaign={"long":323,"short":288}` | long 207 / short 130 | ⚠️ 同上 |
| `loss_round_trip_accounted={"long":2,"short":1}` | **{}** | ⚠️ 同上（G1） |
| `defense_units_exceed_current_holding={"long":4,"short":7}` | `{"long":1}` | ⚠️ 同上 |
| `replenish_triggered_but_full={"short":1}` | **{}** | ⚠️ 同上（G4） |
| `cover_by_side={("long","same"):2,("long","other"):3,("short","same"):3}` | **{}** | ⚠️ 同上（G5） |
| `suspension_by_source` 含 `superseded` long 67 / short 49 | **superseded 桶已随 #414 删除** | ⚠️ **桶退役**，键形状也随 #442 加了 level 维 |
| `stage_events=[]` | **[]** | ✅ 仍为空 |
| 四层报告 `+4040483 / +4417092 / 0.0917` | **完全相同** | ✅（见 G9） |

**建议**：ADR 补充九的 wf8 实测对照段应加一条时效注（本报告即其新读数源），
否则后续读者会拿 #381 时点的读数当现行基线比对。**处置由编排 session 裁定，本轮不改文档。**

---

## 6. 附录：原始产物级读数行（臂 B）

来源 `/tmp/fin384_on_run.log`，`[m8][#357]` 单行（下为换行整理，内容逐字未改）：

```
center_oscillation.enabled=true trigger_attempts=1437 dropped_center_not_alive=113 (7.9%)
dropped_other=961 dropped_center_moved_down=37
unclosed_write_off_count={} unclosed_write_off_units_gap={} unclosed_write_off_cash_booked={}
unclosed_write_off_nothing_to_settle={}
death_write_off_count={"long": 6, "short": 2} death_write_off_units_gap={"long": 557, "short": 75}
death_write_off_cash_booked={"long": 17516044, "short": 3028302}
action_by_level={(1,"long","reduce"):126, (1,"long","replenish"):10, (1,"short","reduce"):74,
  (1,"short","replenish"):8, (2,"long","reduce"):78, (2,"long","replenish"):3,
  (2,"short","reduce"):23, (2,"short","replenish"):3, (3,"long","reduce"):1,
  (3,"long","replenish"):1, (3,"short","reduce"):24}
suspension_by_source={(1,"long","broken_by_third_class_buy"):3, (1,"long","rebase_vanished"):3,
  (1,"long","reset"):44, (1,"short","broken_by_third_class_sell"):1,
  (1,"short","rebase_vanished"):3, (1,"short","reset"):22,
  (2,"long","broken_by_third_class_buy"):1, (2,"long","rebase_vanished"):1,
  (2,"short","rebase_vanished"):1, (3,"short","reset"):1}
suspension_continued_count={(1,"long"):44, (1,"short"):23, (2,"long"):14, (2,"short"):3, (3,"short"):1}
settlement_by_side={(1,"long","cover_and_close"):3, (1,"long","forfeit"):47,
  (1,"short","cover_and_close"):1, (1,"short","forfeit"):25, (2,"long","cover_and_close"):1,
  (2,"long","forfeit"):1, (2,"short","forfeit"):1, (3,"short","forfeit"):1}
replenish_foreign_center_count={}
lifecycle_opened={"long": 224, "short": 85} lifecycle_died={"long": 223, "short": 85}
no_active_campaign_count={"long": 207, "short": 130}
resource_exhausted_holding_negative_count={} loss_round_trip_accounted_count={}
defense_units_exceed_current_holding_count={"long": 1} replenish_triggered_but_full_count={}
stage_recover_capital_count=0 stage_enter_earning_count=0 stage_events=[]
profit_ready_but_suspended={}
earning_mode_switch_bar={} earning_replenish_count={} earning_units_gained={}
earning_cash_unsound_count={} earning_sizing_rounds_to_zero_count={}
cover_by_side={} other_violation_count=0 other_violation_by_kind={} campaign_active_end=1
```

产物落盘：

| 产物 | 路径 |
|---|---|
| 关臂日志 | `/tmp/fin384_off_run.log` |
| 开臂日志 | `/tmp/fin384_on_run.log` |
| 关臂 dump | `/tmp/fin384_off_dump/{trades,tower_events,center_lifecycle}.jsonl` |
| 开臂 dump | `/tmp/fin384_on_dump/{trades,tower_events,center_lifecycle}.jsonl` |
| 关臂四层报告 | `/tmp/fin384_off_m8report.md` |
| 开臂四层报告 | `/tmp/fin384_on_m8report.md` |
| debug 测试日志 | `/tmp/fin384_test_debug.log` |
| release 测试日志 | `/tmp/fin384_test_release.log` |
| 金标准（只读） | `/tmp/vocab_align_dump/{trades,tower_events}.jsonl` |

---

## 7. 验收裁定

| 票体验收项 | 判定 |
|---|---|
| 1. 默认关臂与金标准逐字节一致 | **PASS**（双锚 `cmp` IDENTICAL） |
| 1. enabled=true 臂四层报告同值 | **PASS**（四项逐位相同；整份报告文件零字节差异；两臂 dump 亦逐字节相同） |
| 2. 见证读数全部产物级 | **PASS**（全部经 `[m8][#357]` 单行外化；无一项靠推断——唯一推算值「成功落账 13」已标注为非产物级，见 G8） |
| 2. 报告照实（含零读数） | **PASS**（12 项零读数逐条列出，未省略） |
| 2. 亏损入账 / 三卖核销 / 阶段推进的**实测见证** | **未取得（零读数）**——G1/G2/G3，根因已定位，覆盖仅到单测层 |
| 3. 遗留 fog 只收集读数 | **PASS**（§4，不作处置） |
| cargo test 双侧全绿 | **PASS**（2095 / 2093，0 failed） |
| 慢锁照实标注 | **PASS**（§3.1，141 ignored 双侧一致，短差本体零慢锁） |

**总判定**：对拍与测试面**全部 PASS**；见证面**部分为零读数**（G1/G2/G3），零读数本身已按
090 照实呈报且根因定位到「wf8 窗内成功落账仅 13 次」这一结构事实——**是否以此关图、
是否补跑多窗取样，属编排 session 裁定，本工位不越权。**

---

## 8. ⚠️ 交叉验证：本票被两个工位并行执行

收尾时发现 **#384 由两个工位并行做了同一遍**：

| | 本报告 | 并行工位 |
|---|---|---|
| 产物 | 本文件 + post-mortem（**未提交**，`??` 状态） | `.chanlun/review-results/shortdiff-ledger-final-20260727.md`（368 行），已随 commit **`26dd7a81d2`**（12:47:22）落库 |
| 环境 | 主工作区，`cargo test --lib`（非纯态，见 §0.1 ②） | 独立 worktree `/tmp/wt-384` + 独立 `CARGO_TARGET_DIR`（纯态） |
| 代码改动 | 零 | `wverify_run.rs` +5 行（`profit_ready_but_suspended` 补列）——**即 §0.1 ② 那个改动**，本报告是提前吃到了它 |

**两条独立路径的读数完全一致**，逐项核对：

| 读数 | 本报告 | 并行工位 | |
|---|---|---|---|
| execR / R / MaxDD | +4040483 / +4417092 / 0.0917 | 同 | ✅ |
| 双锚 `cmp` | IDENTICAL（948340 / 354946 B） | 同 | ✅ |
| `trigger_attempts` | 1437 | 1437 | ✅ |
| `death_write_off`（long / short） | 6·557·17,516,044 / 2·75·3,028,302 | 同 | ✅ |
| 亏损入账 / `cover_by_side` / 阶段推进 | 全零 | 全零 | ✅ |
| `superseded` 键消失（#414 产物级证据） | 是 | 是 | ✅ |

**这是本轮唯一一次"两条路径互证"的机会，结果是完全吻合**——含 §0.1 那点非纯态疑虑在内，
本报告的读数由对方的纯态跑独立复现。

**并行工位比本报告多做的一项**：`cash_booked` 的量纲订正（整数美元非分）+ 票面
「8 次 / 557 units」混口径的纠正（§2.6 已转引）。

**本报告比并行工位多做的两项**：① §0.1 的非纯态时间线披露（对方在纯态 worktree 里跑，
不涉及此问题，但主工作区那 66 秒窗口是真实存在的流程风险，已入 post-mortem §3.4）；
② §5.1 的 ADR 补充九对照表逐项时效判定（G10）。

**处置建议（不越权，交编排裁定）**：并行工位的报告已入库、且环境更纯，应作主产物；
本报告可作交叉验证附件保留，或只把 §0.1 / §5.1 / post-mortem §3.4 三节的内容并入。
**重复劳动本身应上报——两个工位同时认领一张已 blocked-by 全清的 frontier 票，
是调度面的问题，不是执行面的。**
