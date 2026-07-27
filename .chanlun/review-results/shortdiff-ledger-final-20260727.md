# #384 终验报告：短差账本双臂 wf8 对照 + post-mortem

- 票：#384（parent #379 图的**关图终验票**；SPEC 对照源 = #386 全篇）
- 日期：2026-07-27
- 前置：#380（口径四项）/ #381（空头键形状）/ #366（判据对齐 + 清算两终局）/ #383（阶段三）
  / #414（Superseded 非终结）/ #441（死亡吞挂起）/ #442（level 维探针）——全部已落地
- 本票改动：**只补一项报告行**（`profit_ready_but_suspended` 进 wf8 见证打印），无行为变化
- 不碰：`coverage.rs` / `persistent.rs` / `center.rs` / `recursive_tower.rs`（#59 线区域）

---

## 0. 实测留痕（三行硬证据，可复验）

```
worktree HEAD          : 02399d7e0b9bc3ff0d25de0682a21acffb0d67e4   （/tmp/wt-384，git rev-parse HEAD）
git status --short     :  M rust/src/theta_v0/backtest/wverify_run.rs
                         ?? analysis/data_cache            （← symlink 到主仓数据，非源码脏）
CARGO_TARGET_DIR       : /tmp/wt-384-target
```

隔离理由（#405 教训条）：主工作区 `git status --short -- rust/src/` 命中他人未提交改动
（` M rust/src/bin/p107_level_calib.rs`），故全部实测在独立 worktree + 独立 target 目录执行，
数据经 `ln -s /Users/silencehan/Projects/NewChanlun/analysis/data_cache` 接入。worktree 内
唯一改动 = 本票的 5 行（`git diff --stat`：1 file changed, 5 insertions(+)）。

### 完整命令行（#414 锚条：无锚的「亲测」按转述论）

```bash
# 建工位
git worktree add /tmp/wt-384 HEAD
ln -sfn /Users/silencehan/Projects/NewChanlun/analysis/data_cache /tmp/wt-384/analysis/data_cache

# 臂 A：默认关（center_oscillation.enabled=false）
cd /tmp/wt-384/rust && CARGO_TARGET_DIR=/tmp/wt-384-target \
  M8_WIN_FILTER=wf8 VOICE_EXEC=1 OPSEM_DUMP_DIR=/tmp/wt384_default_dump \
  cargo test --release --lib theta_v0::backtest::wverify_run::m8_e2e_all_systems_oos \
  -- --ignored --nocapture > /tmp/wt384_default.log 2>&1          # EXIT=0

# 臂 B：开启（THETA_CENTER_OSCILLATION=1）
cd /tmp/wt-384/rust && CARGO_TARGET_DIR=/tmp/wt-384-target \
  M8_WIN_FILTER=wf8 VOICE_EXEC=1 THETA_CENTER_OSCILLATION=1 OPSEM_DUMP_DIR=/tmp/wt384_on_dump \
  cargo test --release --lib theta_v0::backtest::wverify_run::m8_e2e_all_systems_oos \
  -- --ignored --nocapture > /tmp/wt384_on.log 2>&1               # EXIT=0

# 双侧单测
cd /tmp/wt-384/rust && CARGO_TARGET_DIR=/tmp/wt-384-target cargo test --lib           > /tmp/wt384_test_debug.log 2>&1
cd /tmp/wt-384/rust && CARGO_TARGET_DIR=/tmp/wt-384-target cargo test --release --lib > /tmp/wt384_test_release.log 2>&1
```

输出锚：`/tmp/wt384_default.log`、`/tmp/wt384_on.log`、`/tmp/wt384_test_debug.log`、
`/tmp/wt384_test_release.log`；四层报告快照 `/tmp/wt384_default_fourlayer.md`、
`/tmp/wt384_on_fourlayer.md`；dump 目录 `/tmp/wt384_default_dump/`、`/tmp/wt384_on_dump/`。
本报告全部读数逐字取自这些文件，无转述。

---

## 1. 双臂验收（票面要点①）

### 1.1 默认关臂 = 金标准逐字节一致

```
cmp /tmp/wt384_default_dump/trades.jsonl       /tmp/vocab_align_dump/trades.jsonl       → IDENTICAL (948340B)
cmp /tmp/wt384_default_dump/tower_events.jsonl /tmp/vocab_align_dump/tower_events.jsonl → IDENTICAL (354946B)
```

字节数与 ADR 补充九末条所载金标准（948340B / 354946B）逐位相符。默认关臂见证行全零
（`center_oscillation.enabled=false trigger_attempts=0 … other_violation_count=0
campaign_active_end=0`）——**门关时确实零覆盖**，非未触达的假阴性。

### 1.2 开臂四层报告与关臂逐位相同

```
diff /tmp/wt384_default_fourlayer.md /tmp/wt384_on_fourlayer.md   → 无差异（全文逐位相同）
```

四层报告行（两臂同值）：

| 窗 | n_orders | execR | MaxDD | 终Stage | Q_T(notional_in) | W_T | R(含浮盈) | LCB_OOS(R) | 三态 |
|---|---|---|---|---|---|---|---|---|---|
| wf8 | 1217 | **+4040483** | **0.0917** | I(降成本) | 28721770 | 0 | **+4417092** | −1921382 | INCONCLUSIVE |

与 ADR 补充九「绝对值基线」（execR=+4040483 / R=+4417092 / MaxDD=0.0917）逐位相符——
**短差是自包含旁路账本**这条教义断言，在含 #414/#441/#442 全部新增改动后仍成立。
本次为该基线的第三次独立纯态亲验（前两次：29d0faa4b8、3ed7912ae7）。

> 附带作用：ADR 补充九勘误的「−20144870 是混合编译污染读数」判定再获一次独立佐证
> （见记忆条 `project_wf8_fourlayer_baseline_negative` 所载的悬空验收句问题——本次纯态
> 亲验只支持 +4040483 这一侧）。

---

## 2. 见证读数六项（票面要点②）

原始行取自 `/tmp/wt384_on.log` 的 `[m8][#357] wf8:` 单行（`enabled=true`）。

### 2.1 亏损入账

| 桶 | 读数 | 严格语义 |
|---|---|---|
| `loss_round_trip_accounted_count` | **{}（零读数）** | 「**落账后桶现金为负的动作次数**」——不恒等于亏损往返笔数 |
| `resource_exhausted_holding_negative_count` | {}（零读数） | 货透支（holding<0）=记账错误，照拒；本窗零次 |
| `other_violation_count` | **0** ✓ | 接线/记账逻辑错误警报，硬断言恒 0 |

零读数**不等于「亏损往返没发生」**：该桶只在某次动作落账后桶现金转负时 +1。#366 判据收严后
回补动作从 91 降到 23（见 §2.6），亏损往返的载体（买回价>卖出价的**已闭合**往返）随之稀薄，
桶现金全程未被打穿。退役的 `resource_exhausted_free_negative_count`（旧拒绝桶）不再出现。

### 2.2 清算两终局（分开呈报，不冲销）

| 终局 | 桶 | 读数 |
|---|---|---|
| 三买 → 回补闭合（`CoverAndClose`） | `settlement_by_side` | (1,long):**3**、(1,short):**1**、(2,long):**1** ⟹ 共 **5** 条 |
| 三卖 → 未闭合减出核销（`WriteOffUnclosed`） | `unclosed_write_off_count` / `_units_gap` / `_cash_booked` / `_nothing_to_settle` | **四桶全零** |
| 其余来源（`Forfeit`） | `settlement_by_side` | (1,long):47、(1,short):25、(2,long):1、(2,short):1、(3,short):1 ⟹ 共 **75** 条 |

**照实：三卖核销路径在 wf8 全窗零读数。** `suspension_by_source` 中
`(1,"long","broken_by_third_class_buy"):3`、`(2,"long","broken_by_third_class_buy"):1`、
`(1,"short","broken_by_third_class_sell"):1` 三键存在，而它们的**镜像方向**
（多头遇三类卖 / 空头遇三类买）**一个键都没有**——即 wf8 窗内从未发生「多头挂起遇三类卖点」
或「空头挂起遇三类买点」的终局形态。核销路径的端到端接线由单测覆盖
（`gate_on_third_class_sell_produces_write_off_request_not_a_cover_action`、
`drive_campaign_wiring_settles_write_off_and_reports_gap_and_cash_separately`）；
**声明=能力（090）**：该路径在真实窗口**未被数据触达**，不得声称「已实测验证」，
本窗零读数亦不可外推其他窗口。

`Forfeit` 75 条 vs 闭合 5 条 = **15:1**——本窗挂起的绝大多数走的既不是三买闭合也不是三卖核销，
而是结构异常来源（Reset/RebaseVanished）的作废路径。见 §4 post-mortem 与 §5 fog。

### 2.3 多空对称（**分侧读数，两侧不得相加**）

| 轴 | long | short |
|---|---|---|
| `action_by_level` reduce | L1:126 / L2:78 / L3:1 = **205** | L1:74 / L2:23 / L3:24 = **121** |
| `action_by_level` replenish | L1:10 / L2:3 / L3:1 = **14** | L1:8 / L2:3 = **11** |
| `lifecycle_opened` / `lifecycle_died` | **224 / 223** | **85 / 85** |
| `no_active_campaign_count` | **207** | **130** |
| `defense_units_exceed_current_holding_count` | **1** | —（无键） |
| `suspension_continued_count`（事件次数，非条数） | L1:44 / L2:14 = **58** | L1:23 / L2:3 / L3:1 = **27** |
| `death_write_off_count` | **6** | **2** |

`campaign_active_end=1`（多头 224 开 − 223 死 = 1 条存活到窗末，与 `lifecycle` 对得上）。
多头侧 `lifecycle_opened/died = 224/223` 与 #357/#381 旧读数**逐条相同**——生死轴零漂移。

**两侧不得相加**的实证：空头侧的单腿桶现金不等于单笔成交现金（ADR 补充九
`realized_cash` 口径注），`loss_round_trip_accounted` 两侧同名不同义；本报告全部分侧列示，
下文出现的任何「共 N 条」仅用于同一语义轴内的条数汇总，不跨侧混计金额/份数。

### 2.4 阶段推进（零读数 + 本票补列桶）

| 桶 | 读数 |
|---|---|
| `stage_recover_capital_count` | **0** |
| `stage_enter_earning_count` | **0** |
| `stage_events` | **[]**（空） |
| `profit_ready_but_suspended`（**本票补列**） | **{}（零读数）** |
| `earning_mode_switch_bar` / `earning_replenish_count` / `earning_units_gained` | 全 {} |
| `earning_cash_unsound_count` | **{}** ✓（硬门，恒 0 非 0 即警报） |
| `earning_sizing_rounds_to_zero_count` | {} |

**本票唯一代码改动**即此行：`profit_ready_but_suspended`（ADR 补充十落的「利润够但挂起非空
被前置拦下」正面观测桶）此前**只有单测覆盖、从未进 wf8 报告行**——真实窗口读数不可见。
补列后本窗读数为 **{}**，配合 `stage_events=[]` 给出完整因果：本窗没有任何 campaign 到达
阶段 II，**且不是被「挂起非空」这条新前置门拦下的**——是钱那一半（free≥本金）就没到过，
与四层报告「终Stage=I(降成本)」一致。补列前这两种成因在产物上不可区分。

### 2.5 挂起归属与冲抵顺序

| 桶 | 读数 |
|---|---|
| `cover_by_side`（同中枢/异中枢冲抵明细） | **{}（零读数）** |
| `replenish_foreign_center_count`（异中枢回补被拒，#414 禁静默冲抵的正面读数） | **{}** |
| `replenish_triggered_but_full_count`（「触发但货满」） | **{}** |
| `suspension_by_source`（键 = (级别, 侧, 来源)） | (1,long,broken_by_third_class_buy):3 / (1,long,rebase_vanished):3 / (1,long,reset):**44** / (1,short,broken_by_third_class_sell):1 / (1,short,rebase_vanished):3 / (1,short,reset):**22** / (2,long,broken_by_third_class_buy):1 / (2,long,rebase_vanished):1 / (2,short,rebase_vanished):1 / (3,short,reset):1 |

`superseded` 键**整体消失**——这是 #414（ADR 补充十一）的直接产物级证据：Superseded 不再是
挂起终结来源，改走 `suspension_continued_count`（long 58 / short 27 事件次）延续到三类点清算。
`cover_by_side={}` 与 `replenish_foreign_center_count={}` 一同说明：本窗 14+11=25 次回补动作
全部落在「无挂起可冲抵」或被上游拒绝的形态上，**没有一次异中枢静默冲抵**——#414 要堵的那个
洞（旧读数 `("long","other_center"):3`）在本窗已归零。

### 2.6 判据对齐前后对照（旧判据基线 vs #366 后 vs 本次终验）

| 读数 | 旧判据（#366 前，ADR 补充九 / #381 亲跑） | #366 后（`oscillation-criteria-alignment-20260727.md`） | **本次终验（#414/#441/#442 后）** |
|---|---|---|---|
| `trigger_attempts` | 1437 | 1437 | **1437**（分母不动） |
| `dropped_center_not_alive` | 113 (7.9%) | 113 (7.9%) | **113 (7.9%)** |
| `dropped_other`（PriceOutsideZone 主体） | 768 | 961 | **961** |
| `dropped_center_moved_down` | —（无此桶） | 37 | **37** |
| 动作总数 | 647 | 349 | **351** |
| ├ reduce | 556 | 326 | **326**（long 205 + short 121） |
| └ replenish | 91 | 23 | **25**（long 14 + short 11） |
| `no_active_campaign` | long 323 / short 288 | —（未列） | **long 207 / short 130** |
| `loss_round_trip_accounted` | long 2 / short 1 | {} | **{}** |
| `defense_units_exceed_current_holding` | long 4 / short 7 | long 1 | **long 1** |
| `replenish_triggered_but_full` | short 1 | {} | **{}** |
| `cover_by_side` | (long,same):2 /(long,other):**3** /(short,same):3 | {} | **{}** |
| `suspension_by_source` superseded | long 67 / short 49 | （同旧口径） | **键消失**（#414 改延续） |
| `unclosed_write_off_*` | —（无此桶） | 全零 | **全零** |
| `death_write_off_*` | —（无此桶） | —（无此桶） | **见 §3** |
| `other_violation_count` | 0 | 0 | **0** ✓ |
| execR / R / MaxDD | +4040483 / +4417092 / 0.0917 | 逐位相同 | **逐位相同** |

**#366 后 → 本次终验的两处差异**（新增，需归因）：
- replenish **23 → 25**（+2）：#414 让 Superseded 挂起不再当场作废、延续到三类点，多出的
  存活挂起给了 2 次额外回补机会。方向与 #414 教义一致。
- `no_active_campaign` **long 323/short 288 → 207/130**：该对照跨越 #366（判据收严 ⟹ 落到
  `drive_campaign_wiring` 的动作大幅减少）与 #442（该桶未改）。降幅与 reduce 556→326 同向，
  未见异常。⚠️ 中间态（#366 后、#414 前）读数缺失，本行**只能给端点对照**，不构成逐票归因。

---

## 3. 死亡吞挂起（票面要点③，#441 / ADR 补充十二）

原始读数（`enabled=true` 臂）：

```
death_write_off_count       = {"long": 6, "short": 2}
death_write_off_units_gap   = {"long": 557, "short": 75}
death_write_off_cash_booked = {"long": 17516044, "short": 3028302}
```

| 侧 | 事件数 | 货缺口（units） | 现金（桶实收） | 占 notional_in(28,721,770) |
|---|---|---|---|---|
| long | **6** | **557** | **17,516,044** | 60.98% |
| short | **2** | **75** | **3,028,302** | 10.54% |

货缺口与现金**分列不冲销**（补充十二）：557 股的货没了、17,516,044 的钱在账上，两者不相减、
不造虚拟回补抵平。核销不产任何 `TwEvent`——四层报告两臂逐位相同即其行为化证据（§1.2）。

### 3.1 ⚠️ 票面口径订正（090 照实，三处）

票面要点③写「8 次 / 557 units / 17,516,044 分 = 17.5 万美元，本金 initial_nav=1e6=100 万美元
的 17.5%」。经查代码与本次产物，**三处均需订正**：

1. **「8 次 / 557 units / 17,516,044」是混口径**：8 = 两侧事件数之和（long 6 + short 2），
   而 557 与 17,516,044 是**多头侧独有**。按票面自己的「两侧不得相加」条款，8 这个数不该与
   多头侧的份数/金额并列。分侧读数见上表。
2. **现金单位不是「分」**：生产调用点 `fill.rs:2655` 传的是 `px as i64`——**原始价格截断成
   整数美元**（`trades.jsonl` 首条 `entry_px=28598.06`，量纲一致）。桶记账口径
   `Reduce` 记 `+units·price`（`short_diff_bucket.rs` 模块头 §P1 修复续），故
   `cash_booked` = **整数美元的毛减出所得**。旁证：17,516,044 ÷ 557 = **31,447**，
   正是本窗 BTC 价位；若单位是分，则单价 314.47 美元，与 BTC 不符。
   仓库内**无任何**「现金单位=分」的口径文件（`grep` 全 ADR + 相关模块 0 命中）。
3. **本金不是 1e6**：wf8 的 `initial_nav = 首个可交易收盘价 × tick_size × 1000`
   （`wverify_run.rs:1214-1217`），本窗实测 `notional_in = Q_T = 28,721,770`。
   四层报告末行自己写着「I_0 报告门槛 = 1000000 …**各窗 nav 不同 ⟹ 门槛按 notional_in 列读**」
   ——1e6 只是报告层名义门槛，不是本窗本金。

**并且语义上**：`cash_booked` 是减出腿的**毛流水**（Σunits·price），**不是亏损**。
「占本金 17.5%」这句在原口径下会被读成「亏掉了本金的 17.5%」，而实际含义是
「累计有相当于 61% notional 的减出所得，其对应的货没有补回来」——是**未闭合规模**，
不是损益。报告措辞按订正后口径。

---

## 4. 增股无份数落点（票面要点④）

ADR 补充十末条登记的附注 4.2 口径：**增股按成交价进 holding、无份数登记、永不被卖出**——
这是旁路账本的直接后果（短差桶只记 units 与现金，增出来的股份并入本仓 `holding` 成本基，
桶不为其单独维护可售份数）。

本窗**该口径全程未被触达**：`earning_units_gained={}`、`earning_replenish_count={}`、
`earning_mode_switch_bar={}`——阶段三从未进入（`stage_enter_earning_count=0`），故不存在
任何增股。措辞按此口径记录，**不声称已实测验证**：附注 4.2 描述的是阶段三一旦启用后的
账目形态，本次终验只能确认「本窗无增股，因而无份数落点问题的实例」。

---

## 5. Post-mortem（票面要点⑤：异常桶 / 零读数 / 与旧基线差异）

### 5.1 硬警报桶：全部干净

- `other_violation_count = 0`（`other_violation_by_kind = {}`）——接线/记账逻辑错误恒 0，
  硬 `assert_eq!` 通过。
- `earning_cash_unsound_count = {}`——阶段三「桶现金永不为负」的唯一硬门，恒 0 语义
  （补充十恢复）成立（本窗因未进阶段三，属**空真**，不构成对该门的正面检验）。
- `resource_exhausted_holding_negative_count = {}`——货透支拒绝零次。

### 5.2 零读数清单（照实列全，不省略）

`loss_round_trip_accounted` / `unclosed_write_off_*`（四桶）/ `cover_by_side` /
`replenish_foreign_center_count` / `replenish_triggered_but_full` / `stage_events` /
`stage_recover_capital` / `stage_enter_earning` / `profit_ready_but_suspended` /
`earning_*`（五桶）/ `resource_exhausted_holding_negative` / `other_violation_*`。

其中**5 类零读数的成因各不相同**，不可一概而论：

| 零读数 | 成因 | 是否可外推 |
|---|---|---|
| `unclosed_write_off_*` | 窗内**没有**「多头遇三卖 / 空头遇三买」形态样本 | **否**——路径仅单测覆盖 |
| `stage_events` / `profit_ready_but_suspended` | 阶段推进的**钱那一半**从未达标（终Stage=I） | 否——阶段机未被检验 |
| `earning_*` | 阶段三从未进入（上一条的连带） | 否 |
| `cover_by_side` / `replenish_foreign_center` | #366 判据收严 ⟹ 回补稀薄 + #414 堵掉异中枢冲抵 | 部分——是**新判据的预期后果** |
| `other_violation_*` / `earning_cash_unsound` | 硬门断言，**期望恒 0** | 是——这才是正面通过 |

### 5.3 结构性异常：`Forfeit` 75 条 vs 闭合 5 条

本窗挂起终结的**93.8%**（75/80）走 `Forfeit`（Reset / RebaseVanished），只有 5 条走三买闭合、
0 条走三卖核销。#414 把 Superseded 从 Forfeit 里摘出来（superseded 键消失、改延续）之后，
剩下的 Forfeit **全部来自结构异常源**（`reset` 67 = L1:44/L1short:22/L3short:1，
`rebase_vanished` 8，合计 75）——而 `reset` 计数从 #381 旧读数的 long 2 / short 1
暴涨到 long 44 / short 23。

⚠️ **归因未定**：该暴涨与 #414（Superseded 挂起不再当场终结，得以存活到后续 reset 事件才终结）
方向一致，是**最可能**的成因；但本次终验**没有做**「#414 前/后同工位纯态对照」，
不足以定论。ADR 补充十一自己写着「Reset/RebaseVanished 两源……占少数（wf8：20/64）」
（`docs/adr/0001-graded-exit-and-shortdiff-doctrine.md:210`，31%）——
本次实测 **75/80=94%**，与该句所载比例**相反**。这是本报告发现的**与既有教义文档最显著的读数背离**，
不在 #384 裁定范围，移交（见 §6）。

### 5.4 与旧基线差异汇总

见 §2.6 表。除 §5.3 的 reset 暴涨外，全部差异都能归到已裁定的口径变更上（#366 判据收严、
#414 挂起延续、#442 加 level 维），无未解释漂移。四层报告轴零漂移（三次独立亲验）。

### 5.5 本票自身的局限

- 本票**未新增测试**：改动是报告行补打一个既有 witness 字段（5 行，无分支、无行为），
  其正确性由该字段既有单测（`oscillation_campaign.rs` 的
  `profit_ready_but_suspended` 分侧用例、`fill.rs:1324` 端到端用例）覆盖；打印本身不可
  单测化。如实标注：**净新增 0 条**。
- 单窗结论不外推：全部读数只对 BTC wf8（2023-08-17..2024-02-16，264960 bar）成立。
- 慢锁（`--ignored` 重型族）除本报告所载三次 wf8 跑批外未跑，留重型窗口。

---

## 6. Fog 移交（票面要点⑥）

关图时不做、明确移交或另切票的项：

1. **多窗推广读数**——本报告全部结论限于 wf8 单窗。三卖核销 / 阶段推进 / 增股份数三条路径
   在本窗零读数，**必须**换窗才能拿到正面读数。建议：p3fold + 其他 wf 窗的同口径跑批。
2. **短差资金参数校准**——sizing（1/3）、到 0 判据的本金门、等金额回补的零头留桶，全部未做
   参数敏感性；本窗「阶段推进零读数」到底是参数太紧还是经济现实，无法区分。
3. **#441 建议两条**（本票不实施，原样移交）：
   - 桶拒绝落 `other_violation_by_kind`——让被拒动作的 kind 分布可见（现在 kind 桶只在
     `other_violation_count>0` 时才有内容，而该计数恒 0，等于该分桶永远空）。
   - `death_write_off_units_gap` 的 557 构成按 **（级别，中枢）** 分桶——现在只有侧维汇总，
     6 次事件各自贡献多少、集中在哪一级/哪个中枢不可读。
4. **Reset / RebaseVanished 清算同族另议**——ADR 补充十一明写「其清算是否同族另议」，
   本次实测把它从「占少数（20/64=31%）」推到 **75/80（94%）**（§5.3），优先级应上调：
   现在绝大多数挂起走的是这条**口径未裁定**的路。建议单独切票，含 #414 前后对照实验以定归因。
5. **`profit_ready_but_suspended` 的正面读数**——本票只是让它可见，本窗读数为空；
   要检验补充十的「挂起非空拦推进」是否真的生效，需要一个能到达「free≥本金」的窗口。

---

## 7. 测试读数（双侧全绿）

| 侧 | 命令 | 结果 |
|---|---|---|
| debug | `cargo test --lib` | **2095 passed / 0 failed / 141 ignored** |
| release | `cargo test --release --lib` | **2093 passed / 0 failed / 141 ignored** |

（release 少 2 条 = 既有 `debug_assert` 相关用例的既有差异，非本票引入——与 #366 报告所载
同一差额。）本票净新增测试 **0** 条，理由见 §5.5。

---

## 8. 未能判定 / 如实标注

1. **三卖未闭合减出核销**在 wf8 零读数——单测覆盖，真实窗口未触达，**不得声称已实测验证**。
2. **阶段推进（II / III）全链路**在 wf8 零读数——含增股份数落点（票面要点④），同上。
3. **`reset` 计数暴涨（2/1 → 44/22）的归因**未定——#414 是最可能成因但未做前后对照实验。
4. **Forfeit 占比 86% 与 ADR 补充十一所载「占少数 20/64」背离**——本报告只呈报，不裁定。
5. **票面要点③的单位/本金/语义三处口径**经查为误，本报告按订正后口径书写（§3.1）；
   订正依据全部给了代码行锚与算术旁证，可复检。
6. **多窗不外推**、**慢锁未跑**（§5.5）。
