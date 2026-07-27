# treasury 重验 T1：v4 基线**重建**+ 回归臂（臂R）逐位对照

- **日期**：2026-07-27　**票据**：issue #387（母 SPEC #385，map #59 Destination 末项）
- **工位**：worktree `/tmp/kimi-nest-mainline`，分支 `kimi-nest-mainline-20260717`，HEAD = `c126d4cf69`
- **性质**：**重建基线**——2026-07-20 v4 三窗三臂验收的原始产物（`/tmp/m8_win*` 的 `trades.jsonl` /
  `tower_events.jsonl`、`/tmp/m8_win.out`、`/tmp/m8_win_gate.out`、`/tmp/armC_run.log`）已灭失（编排者
  2026-07-27 查证）。本批按 v4 报告 §1 所载命令**重新跑出**臂A/臂B 基线，本文件内一切「臂A/臂B/臂R」
  读数均为**重建基线**，不是 2026-07-20 的原件。
- **对照口径**（票体明文降级条款）：原始 trades.jsonl 不可得 ⟹ 对照声明限于
  **「读数逐位 + digest 冻结」**——读数与 v4 报告**发表表**（§2.1/§2.2/§2.3/§2.4）逐项逐位比对；
  臂R 的 trades 逐字节 digest 冻结为**后向**回归锁（锁的是本批产物，不是 v4 原件）。
- **只跑批不改语义**：`rust/src` 零改动，零 git mutation，主仓 `/Users/silencehan/Projects/NewChanlun` 全程只读。
  新增件只有一个校验脚本 + 一个 golden + 本报告。
- **口径标签**：执行层数值为 `[L1机制/费率未标定]`（成交费率科目与 cost 三常费率均未标定，
  跑批产物自带该标签）。**认识论 L2**（真实 BTC OOS 假设检验，可否证）。数值**不作 alpha 论据、
  不作策略择优输入**（v3 硬禁令 / A10 附则B）。

---

## 0. 结论摘要

1. **票面核心问题的答案是「成立」**：v4 之后**票面枚举的整条修复链**（#351/#356/#363/#365/#376/
   #360/#374/#246/#361）在 default 口径（`fee_schedule=None`、`enforce_level_cap=false`）下对
   treasury 三窗双臂读数**零影响**——本批 HEAD 读数与 `2fd5ee08ba` 处读数**逐字段完全相同**
   （§4.3 实证）。「default bit-exact 不破」的逐票声明链在 treasury 层**复合成立**。
2. **但本批读数确实大幅偏离 v4 发表表**，发散源**不在**该修复链，而是两处：
   - **臂A（门关）**：§2.2 全部读数由**单个提交 `2fd5ee08ba`（#309 LEE M3 事件门控）**翻转；
     其父提交 `4cde104869` 三窗读数与 v4 发表表**逐位全中**（§4.1）。#309 提交自述明文
     「订单流分叉……digest 不等，不作优劣判断，**未借 bit-exact**」——它从未声明 bit-exact，
     故**不是** bit-exact 声明链的反例，而是母 SPEC #385 前提枚举的**遗漏项**。
   - **臂R（门开）**：§2.1/§2.3/§2.4 的发散**早于** #309，落在 v4 之后的头 10 个提交
     （`ec728bf6cf`..`12aa929d44`）内，而其中 **9/10 不可独立构建** ⟹ 该区间**不可二分**，
     无法归因到单个 commit（§4.2）。
3. **v4 报告自身的溯源缺陷（登记）**：v4 报告标注 HEAD = `640609071`，但该提交上
   `THETA_NEST_CERT_GATE` / `NEST_GATE_STATS` **根本不存在**（`git grep` 零命中），臂A ≡ 臂B，
   且臂A p3fold execR = **-15383474** ≠ 发表表 +923705。⟹ v4 三臂读数**不产自其自述 HEAD**，
   而产自当日未提交的工作区状态（该状态于次日起以 `ec728bf6cf` 等提交入库）。

---

## 1. 跑批命令 / env / 落盘路径（供 T3 复现节引用）

工作目录一律 `/tmp/kimi-nest-mainline/rust`。

| 用途 | 命令（env 与 v4 报告 §1 表逐字对齐） | 落盘 |
|---|---|---|
| 臂A 门关（全窗） | `VOICE_EXEC=1 OPSEM_DUMP_DIR=/tmp/m8_win cargo test --release --lib theta_v0::backtest::wverify_run::m8_e2e_all_systems_oos -- --ignored --nocapture 2>&1 \| tee /tmp/m8_win.out` | `/tmp/m8_win.out`、`/tmp/m8_win/{trades,tower_events}.jsonl`、报告副本 `/tmp/m8_armA_report.md` |
| **臂B = 臂R**（全窗） | `VOICE_EXEC=1 THETA_NEST_CERT_GATE=1 OPSEM_DUMP_DIR=/tmp/m8_win_gate cargo test --release --lib theta_v0::backtest::wverify_run::m8_e2e_all_systems_oos -- --ignored --nocapture 2>&1 \| tee /tmp/m8_win_gate.out` | `/tmp/m8_win_gate.out`、`/tmp/m8_win_gate/{trades,tower_events}.jsonl`、报告副本 `/tmp/m8_armR_report.md` |
| 分窗产物（两臂各三次） | `M8_WIN_FILTER=<tag> <上表同 env> OPSEM_DUMP_DIR=/tmp/m8_win[_gate]/<tag> cargo test …` | `/tmp/m8_win/<tag>/trades.jsonl`、`/tmp/m8_win_gate/<tag>/trades.jsonl`（tag ∈ {p3fold,wf7,wf8}） |
| 四层报告（每次跑批覆写） | 测试体内部 | `/tmp/m8_e2e_all_systems_oos.md` |

**臂B 与臂R 是同一次跑批**：臂R 的配置（`fee_schedule=None`、`enforce_level_cap=false`、κ=0 冻结）
即 default，与 v4 臂B 的 env 完全相同 ⟹ 重建臂B 基线与跑臂R 是**同一个产物**，未跑两遍，
下文「臂B/臂R」指同一批数据。默认值实证：`rust/src/theta_v0/config.rs:304`（`fee_schedule: None`）、
`:228`（`enforce_level_cap: false`）。

**dump 目录结构与 v4 报告的差异（照实）**：`opsem_dump.rs:207-209` 每次跑批在
`OPSEM_DUMP_DIR` **根目录**截断新建 `trades.jsonl`/`tower_events.jsonl`，测试体**不建**逐窗子目录
⟹ 一次全窗跑批的根目录产物只留**末窗（wf8）**。v4 报告 §1 所载的
`/tmp/m8_win/{p3fold,wf7,wf8}/trades.jsonl` 布局须靠 `M8_WIN_FILTER=<tag>` 逐窗跑批产生，本批照此办理。
`M8_WIN_FILTER` 的 bit-exact 中性**本批实证**：三窗分窗跑批的 `[m8]` 行与 `NEST_GATE_STATS` 行
与全窗跑批**逐位相同**。

**确定性实证**：臂R p3fold 独立复跑一次落 `/tmp/m8_determinism_check/`，与
`/tmp/m8_win_gate/p3fold/trades.jsonl` **`cmp` 逐字节相同**。

**数据面未漂移**：`analysis/data_cache/btc_1m_full.json`（符号链接至主仓）
mtime = 2026-06-25 10:27:51、size = 329485099、md5 = `fe2cfcbee0f6117794c0550c73cfb1c1`，
早于 v4 跑批日 ⟹ 排除数据漂移，发散是代码侧。

---

## 2. 对照表：本批（重建基线）vs v4 报告发表表

### 2.1 trades（按 `certificate.dir` 聚合；pnl = `pnl_raw_unlevered` 求和）

| 窗 | 臂 | 总笔数 v4 → 本批 | Long n/pnl v4 → 本批 | Short n/pnl v4 → 本批 | 判定 |
|---|---|---|---|---|---|
| p3fold | A | 479 → **479** | 229/+13737.94 → **229/+13737.94** | 250/-11684.58 → **250/-11684.58** | **零差异** |
| wf7 | A | 504 → **504** | 229/+5597.59 → **229/+5597.59** | 275/-5598.74 → **275/-5598.74** | **零差异** |
| wf8 | A | 518 → **518** | 284/+23826.69 → **284/+23826.69** | 234/-16464.99 → **234/-16464.99** | **零差异** |
| p3fold | B/R | 431 → **149** | 211/+3626.09 → **72/+3645.77** | 220/-10178.65 → **77/-15493.57** | **发散** |
| wf7 | B/R | 467 → **196** | 212/-644.44 → **99/-2379.33** | 255/-13177.99 → **97/-15131.81** | **发散** |
| wf8 | B/R | 449 → **168** | 250/+20668.46 → **88/+24155.68** | 199/-8269.28 → **80/-7937.15** | **发散** |

### 2.2 execR / MaxDD / R / LCB_OOS / 三态 / 终Stage

| 窗 | 臂 | execR v4 → 本批 | MaxDD v4 → 本批 | R v4 → 本批 | LCB_OOS(R) v4 → 本批 | 三态 v4 → 本批 | 终Stage | 判定 |
|---|---|---|---|---|---|---|---|---|
| p3fold | A | +923705 → **-1051013** | 0.0409 → **0.2081** | +1005147 → **-842985** | -827536 → **-4584687** | INCONCLUSIVE → **无(R≤0)** | I → I | 发散 |
| wf7 | A | -1414690 → **-5489800** | 0.0813 → **0.2693** | -1317089 → **-5085991** | -3320147 → **-10125651** | 无(R≤0) → 无(R≤0) | I → I | 发散 |
| wf8 | A | +2508870 → **-2940746** | 0.0596 → **0.1985** | +2884075 → **-2597226** | -2804861 → **-8086406** | INCONCLUSIVE → **无(R≤0)** | I → I | 发散 |
| p3fold | B/R | -36842 → **-1268551** | 0.0313 → **0.1475** | +11960 → **-1191916** | -1153877 → **-3550861** | INCONCLUSIVE → **无(R≤0)** | I → I | 发散 |
| wf7 | B/R | -2563498 → **-5427281** | 0.1233 → **0.2601** | -2461868 → **-5147413** | -4956569 → **-10865720** | 无(R≤0) → 无(R≤0) | I → I | 发散 |
| wf8 | B/R | +3465366 → **+6346975** | 0.0558 → **0.0803** | +3813653 → **+6851062** | -2064630 → **-2105181** | INCONCLUSIVE → INCONCLUSIVE | I → I | 发散 |

**未发散项（照实登记）**：三窗双臂**终Stage 恒 I(降成本)**、`W_T` 恒 0、`Q_T`（notional_in）三窗
恒 16543670 / 23471260 / 28721770——与 v4 一致。

### 2.3 NEST_GATE_STATS 拒绝率（臂R；臂A 门关无统计，与 v4 同）

| 窗 | v4 臂B | 本批臂R | 判定 |
|---|---|---|---|
| p3fold | 309/1633 = 18.9%（nest_pass=1183 xzd_pass=141；rej: cert_none=45 nest_n_delta_false=30 xzd_gate_fail=234） | **1159/1633 = 71.0%**（admitted=474 nest_pass=24 xzd_pass=450；rej: flat_dir=0 no_level=0 cert_none=115 nest_n_delta_false=117 xzd_gate_fail=927） | 发散（**total 相同**） |
| wf7 | 291/1724 = 16.9%（nest_pass=1157 xzd_pass=276；rej: cert_none=23 nest_n_delta_false=14 xzd_gate_fail=254） | **1116/1724 = 64.7%**（admitted=608 nest_pass=32 xzd_pass=576；rej: cert_none=56 nest_n_delta_false=100 xzd_gate_fail=960） | 发散（**total 相同**） |
| wf8 | 293/1518 = 19.3%（nest_pass=1136 xzd_pass=89；rej: cert_none=30 nest_n_delta_false=23 xzd_gate_fail=240） | **1074/1518 = 70.7%**（admitted=444 nest_pass=33 xzd_pass=411；rej: cert_none=87 nest_n_delta_false=34 xzd_gate_fail=953） | 发散（**total 相同**） |

**逐窗 total（候选事件流规模）三窗全部与 v4 逐位相同（1633 / 1724 / 1518）**——发散在门的判定，
不在候选事件流。本批另有 v4 未收录的诊断行（`NEST_GATE_CHAIN` / `_T3` / `_INDEX` / `_FAIL` / `_LEVEL`），
原文见 `/tmp/m8_win_gate.out`。

### 2.4 链深构成（`certificate.nest_depth`，trades 侧）

| 窗 | 臂 | v4（depth 0/1/2/3/4） | 本批 | 判定 |
|---|---|---|---|---|
| p3fold | A | 153/159/89/70/8 | **153/159/89/70/8** | **零差异** |
| wf7 | A | 162/160/101/53/28 | **162/160/101/53/28** | **零差异** |
| wf8 | A | 207/146/101/44/20 | **207/146/101/44/20** | **零差异** |
| p3fold | B/R | 148/131/74/70/8 | **1/56/48/39/5** | 发散 |
| wf7 | B/R | 158/137/94/50/28 | **13/73/63/33/14** | 发散 |
| wf8 | B/R | 187/114/90/39/19 | **15/54/58/28/13** | 发散 |

---

## 3. 归因规程执行记录

**第一级：开关二分**——`fee_schedule` 与 `enforce_level_cap` 在本批**均为 default Off**
（`config.rs:304` / `config.rs:228` 亲读确认，跑批未注入任何新开关），故按票体规程「均 Off 时应零差异」
的前提**成立**，差异不可归因到开关 ⟹ 进入 commit 二分。

**第二级：commit 二分**——在**独立 clone**（`git clone --shared /Users/silencehan/Projects/NewChanlun
/tmp/bisect387`，只读源仓，不动 worktree 与主仓的任何 git 状态）中执行，独立 target dir
`/tmp/bisect387_target`，判据脚本 `/tmp/bisect_armA.sh`（GOOD = v4 锚值 `execR=+923705`，
BAD = 其它值，构建失败 = 125 skip），逐步日志 `/tmp/bisect_log/<short-sha>.out`。

> **构建面注记**：v4 时代的 `rust/.cargo/config.toml` 设 `rustc-wrapper =
> /tmp/kimi-nest-mainline/.chanlun/locks/cargo_gate.sh`（该脚本现已随 `.chanlun/locks` 一并删除）。
> 二分期间以 `CARGO_BUILD_RUSTC_WRAPPER=""` 旁路——该 wrapper 的语义是「哨兵不在则逐字透传 rustc」，
> 旁路不改变编译产物，仅解开构建门。

---

## 4. 发散归因（三条，逐条落盘）

### 4.1 臂A：单一提交 `2fd5ee08ba`（#309 LEE M3 事件门控）翻转 §2.2 全部读数

二分终判：`2fd5ee08ba9db404430a5ac9bc1ec4344f533a6a is the first bad commit`
（`feat(rust): #309 LEE M3 事件门控——clock_ℓ 事件钟落地，结构交易只在事件时点重估目标`，2026-07-26）。

父提交 `4cde104869` 处臂A 三窗读数与 v4 §2.2 发表表**逐位全中**：

| 窗 | `4cde104869`（#309 之父） | v4 发表表臂A | 判定 |
|---|---|---|---|
| p3fold | execR=+923705 MaxDD=0.0409 R=+1005147 LCB=-827536 INCONCLUSIVE | 同左 | **逐位相同** |
| wf7 | execR=-1414690 MaxDD=0.0813 R=-1317089 LCB=-3320147 无(R≤0) | 同左 | **逐位相同** |
| wf8 | execR=+2508870 MaxDD=0.0596 R=+2884075 LCB=-2804861 INCONCLUSIVE | 同左 | **逐位相同** |

机制（四层报告费用科目分解，`/tmp/m8_report_4cde104869_A.md` vs `/tmp/m8_report_2fd5ee08ba_A.md`）：

| 窗 | n_orders | ΣN_tΔP_t | Comm+Slip | Funding |
|---|---|---|---|---|
| p3fold @父 | 831 | +1475640 | 470383 | 81552 |
| p3fold @#309 | **831（不变）** | **+637723** | **1479273（×3.1）** | **209463（×2.6）** |

**订单笔数不变、名义额与费用科目暴涨** ⟹ 变的是**每笔物理订单量 q_ℓ**，不是下单时点计数——
与 #309 自述「结构交易只在事件时点重估目标」「结构订单 495→6、n_rescaled 1190→2」同向。

**定性**：#309 提交自述明文写「**订单流分叉以 pre-M2 golden 反用为见证（digest 不等，不作优劣判断，
未借 bit-exact）**」——该提交**从未声明 default bit-exact**。因此臂A 发散**不是** bit-exact 声明链的
反例，而是**母 SPEC #385 前提枚举的遗漏**：#385 列举的修复链未含 #308/#309（LEE M2/M3），
而恰是 #309 改变了 treasury 层执行读数。

### 4.2 臂R：发散早于 #309，落在**不可二分**的提交串内

- 臂R 的 `NEST_GATE_STATS` 在 `4cde104869`（#309 之父）与 `12aa929d44`（v4 后第 10 个提交，2026-07-24）
  处**已经是今日形态**（p3fold 1159/1633、wf7 1116/1724、wf8 1074/1518，逐字段相同）
  ⟹ 门控侧发散**不由 #309 引起**，早于它。
- 可能区间 = v4 之后的头 10 个提交 `ec728bf6cf`..`12aa929d44`。实测：**其中 #1–#9
  （`ec728bf6cf` / `a080e97b1c` / `3a0dc77963` / `bb01eeb655` / `bbbd8f89fa` / `d2312352f9` /
  `997c920b7d` / `65292c7e44` / `9a48915d72`）全部不可独立构建**（`E0432 unresolved import`
  等，模块拆分/整文件搬迁的分批提交中间态；逐条构建日志在 `/tmp/bisect_log/`），
  第 10 个 `12aa929d44` 是 v4 之后**第一个可构建态**，而它已带今日读数。
- ⟹ **该区间不可二分，门控侧发散无法归因到单个 commit**。可断言的只有：发散产生于
  `ec728bf6cf`..`12aa929d44` 这一**整串**（对应 #112 进场门多级投影 / #85-92 seam 骨架 /
  #110-114 多级投影线 / #214-218 证书索引三部曲 / #76/#82/#93 出场门真值源 / C1 双开 W1 等多线交织），
  且**没有任何在册提交能复现 v4 §2.1/§2.3/§2.4 的臂B 读数**。

### 4.3 修复链复合 bit-exact：**成立**（正面结论）

`2fd5ee08ba`（#309，2026-07-26）到 HEAD `c126d4cf69` 之间共 **48 个提交**，含票面枚举的
**全部** #351/#356/#363/#365/#376/#360/#374/#246/#361。实测两臂四层报告表：

- 臂A：`2fd5ee08ba` 处三窗行与 HEAD 处三窗行**逐字段完全相同**（n_orders / ΣN_tΔP_t / Comm+Slip /
  Funding / Borrow / LiqLoss / net_r / MaxDD / 声部数 / 终Stage / Q_T / W_T / η / R / LCB / 三态，
  全 19 列全窗全中）。
- 臂R：`2fd5ee08ba` 处 `[m8]` 行与 `NEST_GATE_STATS` 行与 HEAD 处**逐位相同**
  （p3fold -1268551 / wf7 -5427281 / wf8 +6346975）。

⟹ **「default 配置下 bit-exact 不破」的逐票声明链，在 treasury 层复合成立**（有效域：本三窗 ×
双臂 × 本报告所列读数集；trades 逐字节层面未对照，因 v4 原件灭失）。

### 4.4 v4 报告溯源缺陷（登记，非本票修复范围）

| 事实 | 证据 |
|---|---|
| v4 报告标注 HEAD = `640609071` | v4 报告 §1 抬头 |
| 该提交上门控代码**不存在** | `git grep -c THETA_NEST_CERT_GATE 640609071 -- rust/src` 零命中；`NEST_GATE_STATS` 同 |
| 该提交上臂A ≡ 臂B（门 env 无效），且读数与发表表全不同 | 实跑：两臂三窗完全相同，p3fold execR=**-15383474** MaxDD=0.9406 |
| 门控代码入库时间 = v4 报告**次日** | `ec728bf6cf` = 2026-07-21 18:13:38（v4 报告日 2026-07-20） |
| v4 臂A 发表读数的真实代码态 | `4cde104869`（2026-07-25 前后）逐位复现（§4.1） |

⟹ v4 三臂读数产自**当日未提交的工作区状态**，报告的 HEAD 标注对读数**无效**。这解释了为何
「v4 之后的修复链声明 bit-exact」与「本批读数大变」并存却不矛盾。

---

## 5. trades digest golden（后向回归锁）

- **golden**：`chanlun/review-results/treasury-reverify-t1-armR-trades-golden-20260727.json`
- **校验器**：`scripts/check_armR_trades_digest.py`（FNV-1a 64 逐字节，与
  `classifier::signal::extract_signals_bit_exact_digest_guard` /
  `backtest::runner::order_stream_digest` 同款冻结协议；退出码 0=无漂移 / 1=真漂移（打字段级差异）/
  4=产物缺失（非漂移））
- **冻结值**（臂R，HEAD `c126d4cf69`，2026-07-27）：

| 窗 | n_trades | bytes | FNV-1a 64 |
|---|---|---|---|
| p3fold | 149 | 206057 | `0x6bf47daf0aa737cd` |
| wf7 | 196 | 270684 | `0x282c28ca8ca65e16` |
| wf8 | 168 | 228108 | `0xcafa7c4846c762cd` |

- **产物再生成命令**：
  ```bash
  cd rust
  for tag in p3fold wf7 wf8; do
    M8_WIN_FILTER=$tag VOICE_EXEC=1 THETA_NEST_CERT_GATE=1 OPSEM_DUMP_DIR=/tmp/m8_win_gate/$tag \
      cargo test --release --lib theta_v0::backtest::wverify_run::m8_e2e_all_systems_oos \
      -- --ignored --nocapture
  done
  ```
- **校验命令**：`python3 scripts/check_armR_trades_digest.py`（重落 golden：`--regen`，需经审）
- **诚实边界**：这是**后向**锁——冻结的是**本批重建产物**，不是 2026-07-20 的原件（原件灭失）。
  它防的是「本批之后的静默漂移」，不能追认 v4。

---

## 6. 不变量逐窗读数

### 6.1 跑批内断言（`wverify_run.rs` m8 测试体，本批八次跑批全部通过）

| 不变量 | 断言位置 | 结果 |
|---|---|---|
| R 分解守恒（无资金泄漏）`\|conservation_residual\| ≤ tol` | m8 测试体逐窗 | 三窗双臂**全绿** |
| 退本金不超投入 `W_T ≤ notional_in` | m8 测试体逐窗 | 三窗双臂**全绿**（W_T 恒 0） |

### 6.2 treasury / TW 逐窗读数（四层报告表，`/tmp/m8_armA_report.md`、`/tmp/m8_armR_report.md`）

| 臂 | 窗 | 终Stage | Q_T(notional_in) | W_T | η_T / η_* | cum_holding_cost | η_corrected（判读口径） |
|---|---|---|---|---|---|---|---|
| A | p3fold | I(降成本) | 16543670 | 0 | 15800874 / 16543670 | 209463 | 15591411 |
| A | wf7 | I(降成本) | 23471260 | 0 | 22381443 / 23471260 | 403808 | 21977635 |
| A | wf8 | I(降成本) | 28721770 | 0 | 30355961 / 28721770 | 348945 | 30007016 |
| R | p3fold | I(降成本) | 16543670 | 0 | 14564077 / 16543670 | 77923 | 14486154 |
| R | wf7 | I(降成本) | 23471260 | 0 | 20774370 / 23471260 | 280463 | 20493907 |
| R | wf8 | I(降成本) | 28721770 | 0 | 36982765 / 28721770 | 509090 | 36473675 |

**stage 单向性读数**：三窗双臂终 Stage 恒 `I(降成本)`，`W_T` 恒 0 ⟹ 无 Stage 回退可能（与 v4 一致；
signal 层无 alpha ⟹ 已实现 PnL 无正累积 ⟹ 停在 CostReduction，与 M7 c3 witness 同向）。

**NEST_GATE_STATS 逐窗拒绝率（臂R）**：p3fold 1159/1633 = 71.0%、wf7 1116/1724 = 64.7%、
wf8 1074/1518 = 70.7%（分解见 §2.3）。

### 6.3 不变量的机器件（全量 `cargo test --release --lib` 内，本批复跑全绿）

| 不变量 | 机器件 |
|---|---|
| TW 守恒 | `backtest::runner::dual_tw_wiring_conservation`、`strategy::overlay_state::voice_exec_open_close_conservation`、`::delta_n_conservation`、`strategy::ledger::tw_step_preserves_tw`、`::tw_step_realize_drift_equals_dpi` |
| stage rank 单向 | `strategy::ledger::stage_rank_monotone`、`::earning_no_regress`、`closed_loop::transition::hybrid_step_stage_monotone` |
| OQ-9 gate | `strategy::ledger::oq9_enter_earning_gate` / `oq9_no_open_leg_in_earning` / `oq9_no_ghost_close` / `oq9_earning_no_legacy_leg_closure` / `oq9_clear_campaign_gate`、`closed_loop::transition::hybrid_step_oq9_gate_preserved` / `transition_oq9_illegal_returns_err` |
| dual_ledger R=Π-A-W | `strategy::ledger::ledger_initial_inv_holds` / `ledger_step_preserves_inv` / `ledger_noop_identity` |
| treasury floor 会计保守方向 | `backtest::treasury::floor_rounding_is_conservative` |
| R 分解守恒 | `strategy::risk::r_decomposition_conservation` |

---

## 7. 基线复跑

`cargo test --release --lib`（worktree `/tmp/kimi-nest-mainline/rust`，本批实跑，日志
`/tmp/387_full_lib_test.log`）：

```
test result: FAILED. 1928 passed; 1 failed; 134 ignored; 0 measured; 0 filtered out; finished in 1.02s
failures:
    theta_v0::classifier::signal::tests::extract_signals_bit_exact_digest_guard
```

**1928 passed / 1 failed，唯一失败即 `extract_signals_bit_exact_digest_guard`（#115 线在案）**——
与票面基线一致，**未劣化**。本票新增件是 Python 脚本 + JSON golden + 本报告，**不引入 Rust 测试**，
故基线计数不变。

---

## 8. 有效域与边界（`formalization-validity-domain` 231号）

- **认识论等级 L2**：真实 BTC OOS 三窗假设检验，可否证。
- **有效域**：BTC、三窗（p3fold 2023-01-01..06-30 / wf7 2023-02-17..08-16 / wf8 2023-08-17..2024-02-16）、
  双臂（default 门关 / `THETA_NEST_CERT_GATE=1`）、default 开关（`fee_schedule=None`、
  `enforce_level_cap=false`）、κ=0 冻结。**不外推**其它品种/窗口/开关组合。
- **§4.3「复合 bit-exact 成立」的有效域**：限于本报告所列**读数集**（四层报告 19 列 + `[m8]` 行 +
  `NEST_GATE_STATS` 行）在**本三窗双臂**上的相等；**未**在 trades 逐字节层面对照 `2fd5ee08ba`↔HEAD
  （本票只对 HEAD 产物做了 digest 冻结）。**声明不抬格为「全定义域 bit-exact」**。
- **§4.2 的诚实缺席**：门控侧发散无 commit 级归因——不是「未做」，是**在册提交序列上不可做**
  （9/10 不可独立构建）。不以「大致归到某票」冒充归因。
- 全部数值 `[L1机制/费率未标定]`，**不作 alpha 论据、不作策略择优输入**。

---

## 9. 与票体/母 SPEC 的冲突登记

1. **票体前提「均 Off 时应零差异」在本批不成立**——两开关确为 Off，读数仍发散。原因是该前提
   隐含「v4 与今日之间只有 bit-exact 声明的改动」，而 #309 明文未借 bit-exact（§4.1），
   且 v4 锚点本身不在册（§4.4）。前提本身需修正，不是跑批出错。
2. **母 SPEC #385 的修复链枚举缺 #308/#309（LEE M2/M3）**——正是 #309 改变了 treasury 层执行读数。
   建议 T3 报告采本报告 §4.1/§4.3 的分离口径：「枚举链复合 bit-exact 成立；发散归 #309 + 不可二分区间」。
3. **票体要求「臂A/臂B 读数与 v4 发表表逐项比对」已完成**（§2），但「逐条归因到具体开关/commit」
   只对臂A 可完成（§4.1）；臂R 侧照实登记为不可二分（§4.2），不伪造归因。

---

## 10. 交付件自审（`/code-review` 两轴，并行子代理）

被审 diff = 本票新增的三个未跟踪文件（`scripts/check_armR_trades_digest.py`、
本报告、golden JSON）。`rust/src` 零改动。

### 10.1 Standards 轴

- **无硬违规**。类型注解齐全；退出码分层（0 漂移无 / 1 真漂移 / 4 产物缺失）延续
  `scripts/check_fixture_drift.py` 的仓内先例（漂移与环境问题分离）。
- 本报告经核符合 `result-package.md` 六要素、`formalization-validity-domain.md`
  （L2 标注 + §8 有效域显式收窄，无声明膨胀）、`no-patch-mentality.md`（§4.2/§4.4 照实登记
  不可归因项与 v4 溯源缺陷，不伪造、不打补丁掩盖）。
- **判断题一条（未改，登记）**：`readings_of()` 返回 6 字段裸 `dict`，在 `collect()` /
  `diff_report()` / `--regen` 三处同形状消费——Primitive Obsession / Data Clump 味道，
  可改 `@dataclass(frozen=True)`。**不改的理由**：脚本 162 行、字段集由 golden 文件格式锁死，
  引入类型层属过度工程（票体「不过度工程，票面范围内交付」）。字段增多时再改。
- `print` 而非 `logging`：与姊妹件 `check_fixture_drift.py` 同惯例，仓内既有标准覆盖。

### 10.2 Spec 轴

- 票体 **Acceptance criteria 六条全部有对应落盘证据**；「重建基线」字样在案；
  臂B/臂R 同一次跑批的等价关系已按票体硬要求明文声明并给出 default 值代码行号实证；
  归因未抹平（§4.2 照实登记不可二分）；基线复跑 1928/1 与票面数字一致。
- **已修的一条 LOW**：票体「digest golden 文件落地（**含生成命令**）」字面要求命令随 golden 本体走。
  初版命令只在脚本 docstring 与本报告 §5 中，golden JSON 无。**已修**——golden 增
  `_regen_command` / `_check_command` 两个元字段（摘要值 `0x6bf47daf0aa737cd` /
  `0x282c28ca8ca65e16` / `0xcafa7c4846c762cd` 重落后逐位不变，校验器复验 exit=0）。
- **范围登记**：§4.4「v4 报告溯源缺陷」超出本票直接验收范围，已就地标注「非本票修复范围」，
  属诚实登记而非扩大改动面。
- 未发现「看起来实现了但实现得不对」的项；§2 对照表数字经独立核对与 v4 报告原表一致。
