# treasury 重验 T1：v4 基线**重建**+ 回归臂（臂R）逐位对照

- **日期**：2026-07-27　**票据**：issue #387（母 SPEC #385，map #59 Destination 末项）
- **订正状态**：本报告经 **#401 影子评审打回（MED×4 + LOW×5）**，已由 **#408 订正**（2026-07-27）；
  #408 又经 **#417 影子复审（PASS with MED×1 + LOW×3）**，其中三条由 **#420 再订正**（2026-07-27）。
  正文中形如〔**#408 订正**〕/〔**#420 订正**〕的方括号段即订正处，逐条对照与扩散面见
  **§11 订正记录（#408）**、**§13 订正记录（#420）**，LOW 登记见 **§12**。
  订正后新增 §4.2b（臂R §2.2 归因，#408）、§4.2c（臂R §2.1/§2.4 区段未定，#420）。
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

1. **票面核心问题的答案是「成立（限被测八项）」**〔**#408 订正**，原文写「票面枚举的整条修复链
   （九项）」〕：v4 之后**被测区间实际覆盖的八项**（#351/#356/#360/#361/#363/#365/#374/#376）在
   default 口径（`fee_schedule=None`、`enforce_level_cap=false`）下对 treasury 三窗双臂读数**零影响**
   ——本批 HEAD 读数与 `2fd5ee08ba` 处读数**逐字段完全相同**（§4.3 实证）。**#246 不在被测区间内**
   （其全部四个提交都是 `2fd5ee08ba` 的祖先），臂A 侧由二分 GOOD 链间接覆盖、**臂R 侧零覆盖**（§4.3）。
   「default bit-exact 不破」的逐票声明链在 treasury 层于**这八项**上复合成立。
2. **但本批读数确实大幅偏离 v4 发表表**，发散源**不在**该修复链，而是两处：
   - **臂A（门关）**：§2.2 全部读数由**单个提交 `2fd5ee08ba`（#309 LEE M3 事件门控）**翻转；
     其**父提交 `57a50d939f`** 与更早的 `4cde104869` 三窗读数均与 v4 发表表**逐位全中**（§4.1）。
     〔**#408 订正**：原文写「其父提交 `4cde104869`」——`2fd5ee08ba^` 实为 `57a50d939f`，
     `4cde104869` 与之相隔 5 个提交（区间 `4cde104869..2fd5ee08ba` 共 6 条，含 #309 自身）。
     二者读数相同，结论不变，标签订正。〕#309 提交自述明文
     「订单流分叉……digest 不等，不作优劣判断，**未借 bit-exact**」——它从未声明 bit-exact，
     故**不是** bit-exact 声明链的反例，而是母 SPEC #385 前提枚举的**遗漏项**。
   - **臂R（门开）**：发散**按小节分列为三档**〔**#408 订正**，原文只登记了不可二分那段；
     **#420 订正**，#408 把 §2.1/§2.4 与 §2.3 并列归入不可二分段，而这两项在锚点上无产物留存，
     证据不足，降格为「区段未定」〕：
     - **§2.2（execR/MaxDD/R/LCB/三态）**：读数由 #309 翻转，**可归因**（§4.2b）。
     - **§2.3（门控拒绝率）**：发散**早于 #309**——两锚点 `4cde104869` / `12aa929d44` 与 HEAD
       逐子项相同（§4.2 补跑实证），落在 v4 之后的头 10 个提交（`ec728bf6cf`..`12aa929d44`）内，
       而其中 **9/10 不可独立构建** ⟹ 该段**不可二分**，无法归因到单个 commit（§4.2）。
     - **§2.1（trades 聚合）/§2.4（链深）**：**区段未定**——两锚点无臂R trades 侧产物留存，
       既不能断言落在该不可二分段，也不能断言归 #309（§4.2c）。
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

## 4. 发散归因（逐条落盘）

> 〔**#420 订正**〕本节标题原写「三条」——#408 新增 §4.2b、#420 新增 §4.2c 后已非三条，
> 计数删去。当前归因结论共**四档**：臂A §2.2 归 #309（§4.1）、臂R §2.2 归 #309（§4.2b）、
> 臂R §2.3 不可二分（§4.2）、臂R §2.1/§2.4 区段未定（§4.2c）。

### 4.1 臂A：单一提交 `2fd5ee08ba`（#309 LEE M3 事件门控）翻转 §2.2 全部读数

二分终判：`2fd5ee08ba9db404430a5ac9bc1ec4344f533a6a is the first bad commit`
（`feat(rust): #309 LEE M3 事件门控——clock_ℓ 事件钟落地，结构交易只在事件时点重估目标`，2026-07-26）。

**父提交 `57a50d939f`**（= `2fd5ee08ba^`）与二分 GOOD 链上更早的 `4cde104869` 处，臂A 三窗读数
与 v4 §2.2 发表表**逐位全中**（两点读数彼此亦逐位相同；证据
`/tmp/bisect_log/57a50d939f.out`、`/tmp/bisect_log/4cde104869.out`）：

> 〔**#408 订正**〕原文此处写「父提交 `4cde104869`（#309 之父）」——**事实不符**：
> `git rev-parse 2fd5ee08ba^` = `57a50d939f`，`4cde104869` 与 `2fd5ee08ba` 之间严格夹着
> **5 个提交，已全部列出**：`37a56deab1`(#308) / `38cbe04fa7` / `8dce346588` / `1cac0783f2` /
> `57a50d939f`（区间 `4cde104869..2fd5ee08ba` 共 **6 条**，多出的第 6 条即 `2fd5ee08ba` 自身）。
> 〔**#420 订正（LOW-B）**：原文写「相隔 **6 个提交**（…等）」——「6」是含 #309 自身的区间计数，
> 与紧随其后的 5 项枚举不自洽，且「等」暗示另有未列项（实际已列全）。数字与「等」均已订正。〕
> 真父 `57a50d939f` 独立跑批亦为 GOOD（三窗 execR=+923705 / -1414690 / +2508870），
> 故「单一提交 `2fd5ee08ba` 翻转 §2.2」的结论**不变且更稳**（GOOD/BAD 分界直接落在父子相邻处）；
> 本条订正的是提交谱系标签，不是结论。下表两列读数在 `57a50d939f` 与 `4cde104869` 上相同。

| 窗 | `57a50d939f`（#309 真父）＝`4cde104869` | v4 发表表臂A | 判定 |
|---|---|---|---|
| p3fold | execR=+923705 MaxDD=0.0409 R=+1005147 LCB=-827536 INCONCLUSIVE | 同左 | **逐位相同** |
| wf7 | execR=-1414690 MaxDD=0.0813 R=-1317089 LCB=-3320147 无(R≤0) | 同左 | **逐位相同** |
| wf8 | execR=+2508870 MaxDD=0.0596 R=+2884075 LCB=-2804861 INCONCLUSIVE | 同左 | **逐位相同** |

机制（四层报告费用科目分解，`/tmp/m8_report_4cde104869_A.md` vs `/tmp/m8_report_2fd5ee08ba_A.md`）：

| 窗 | n_orders | ΣN_tΔP_t | Comm+Slip | Funding |
|---|---|---|---|---|
| p3fold @`4cde104869` | 831 | +1475640 | 470383 | 81552 |
| p3fold @#309 | **831（不变）** | **+637723** | **1479273（×3.1）** | **209463（×2.6）** |

〔**#408 订正**〕上表首行原标「@父」——实为 `4cde104869`，非 `2fd5ee08ba` 的父提交（真父
`57a50d939f`，见上）。费用科目分解只有 `4cde104869` 一点留有四层报告文件，真父 `57a50d939f`
只留 `[m8]` 行（三窗与 `4cde104869` 逐位相同）⟹ 分解表的对照点照实标为 `4cde104869`。

**订单笔数不变、名义额与费用科目暴涨** ⟹ 变的是**每笔物理订单量 q_ℓ**，不是下单时点计数——
与 #309 自述「结构交易只在事件时点重估目标」「结构订单 495→6、n_rescaled 1190→2」同向。

**定性**：#309 提交自述明文写「**订单流分叉以 pre-M2 golden 反用为见证（digest 不等，不作优劣判断，
未借 bit-exact）**」——该提交**从未声明 default bit-exact**。因此臂A 发散**不是** bit-exact 声明链的
反例，而是**母 SPEC #385 前提枚举的遗漏**：#385 列举的修复链未含 #308/#309（LEE M2/M3），
而恰是 #309 改变了 treasury 层执行读数。

### 4.2 臂R **§2.3（门控拒绝率）**：发散早于 #309，落在**不可二分**的提交串内

> 〔**#420 收窄（MED-A）**〕本节的有效范围**只有 §2.3**——两锚点留存的产物是
> `NEST_GATE_STATS` 行与 `[m8]` 行，二者都不含 trades 聚合与 `nest_depth` 直方图。
> 臂R §2.1/§2.4 的发散区段**未定**，见 §4.2c；#408 版本把这两项一并写进本节结论，证据不足。

- 臂R 的 `NEST_GATE_STATS` 在 `4cde104869`（#309 之前，非其父——见 §4.1 订正）与 `12aa929d44`
  （v4 后第 10 个提交，2026-07-24）处**已经是今日形态**
  ⟹ 门控侧发散**不由 #309 引起**，早于它。

  〔**#408 补跑取证**〕原报告此断言无留存 stdout（#401 MED-4）。本票在独立 shared clone
  （`/tmp/fix408`，独立 target `/tmp/fix408_target`，`CARGO_BUILD_RUSTC_WRAPPER=""` 旁路）
  于两锚点各重跑一次臂R，stdout 落盘：

  | 锚点 | stdout | 三窗 `NEST_GATE_STATS`（rejected/total） |
  |---|---|---|
  | `4cde104869` | `/tmp/fix408_armR_4cde104869.out` | 1159/1633、1116/1724、1074/1518 |
  | `12aa929d44` | `/tmp/fix408_armR_12aa929d44.out` | 1159/1633、1116/1724、1074/1518 |
  | HEAD 本批（§2.3） | `/tmp/m8_win_gate.out` | 1159/1633、1116/1724、1074/1518 |

  三点**逐字段相同**，不止总数：`admitted=474/608/444`、`nest_pass=24/32/33`、`xzd_pass=450/576/411`、
  `cert_none=115/56/87`、`nest_n_delta_false=117/100/34`、`xzd_gate_fail=927/960/953`、
  `flat_dir=0`、`no_level=0` 全部一致。**原断言（含「逐字段相同」这一逐位强度）经补跑证实，未改写。**
  补跑命令：
  ```bash
  git clone --shared /tmp/kimi-nest-mainline /tmp/fix408 && cd /tmp/fix408 && git checkout <sha>
  mkdir -p analysis/data_cache && ln -sf /Users/silencehan/Projects/NewChanlun/analysis/data_cache/btc_1m_full.json analysis/data_cache/
  cd rust && CARGO_TARGET_DIR=/tmp/fix408_target CARGO_BUILD_RUSTC_WRAPPER="" \
    VOICE_EXEC=1 THETA_NEST_CERT_GATE=1 OPSEM_DUMP_DIR=/tmp/fix408_dump_<sha>_R \
    cargo test --release --lib theta_v0::backtest::wverify_run::m8_e2e_all_systems_oos \
    -- --ignored --nocapture > /tmp/fix408_armR_<sha>.out 2>&1
  ```
  两次跑批 `test result: ok. 1 passed; 0 failed`（109.78s / 102.57s）。
  **附带实证**：两锚点臂R §2.2 `[m8]` 行亦逐位相同（execR = -1031612 / -2036084 / +4580249）
  ⟹ 从 `12aa929d44` 到 `4cde104869` 这一段臂R 读数稳定，与 §4.2b 的「#309 处翻转」互为佐证。
- 可能区间 = v4 之后的头 10 个提交 `ec728bf6cf`..`12aa929d44`。实测：**其中 #1–#9
  （`ec728bf6cf` / `a080e97b1c` / `3a0dc77963` / `bb01eeb655` / `bbbd8f89fa` / `d2312352f9` /
  `997c920b7d` / `65292c7e44` / `9a48915d72`）全部不可独立构建**（`E0432 unresolved import`
  等，模块拆分/整文件搬迁的分批提交中间态；逐条构建日志在 `/tmp/bisect_log/`），
  第 10 个 `12aa929d44` 是 v4 之后**第一个可构建态**，而它已带今日读数。
- ⟹ **该区间不可二分，门控侧发散无法归因到单个 commit**。可断言的只有：发散产生于
  `ec728bf6cf`..`12aa929d44` 这一**整串**（对应 #112 进场门多级投影 / #85-92 seam 骨架 /
  #110-114 多级投影线 / #214-218 证书索引三部曲 / #76/#82/#93 出场门真值源 / C1 双开 W1 等多线交织），
  且**没有任何在册提交能复现 v4 §2.3 的臂B 门控读数**。
  〔**#420 订正（MED-A）**：原文此处写「§2.1/§2.3/§2.4」——§2.1/§2.4 在两锚点无 trades 侧产物留存，
  从未在 `4cde104869` / `12aa929d44` 上被测过，「没有任何在册提交能复现」对这两项**无证据支撑**，
  已从本条删除并移入 §4.2c 的「区段未定」。〕

### 4.2b 臂R §2.2 归因：由 #309 翻转（**#408 新增**）

> 本节是 #408 对原报告的**补全**：原报告 §4 未给臂R §2.2 的归因，而归因所需的两份文件
> （`/tmp/m8_report_4cde104869_R.md` / `/tmp/m8_report_2fd5ee08ba_R.md`）本就是本批同一次二分跑批的产物。

臂R（门开）§2.2 三窗读数与臂A 一样，在 `2fd5ee08ba`（#309）处翻转，且翻转后即为 HEAD 值：

| 窗 | 臂R @`4cde104869`（#309 前） | 臂R @`2fd5ee08ba`（#309） | HEAD（§2.2 本批列） |
|---|---|---|---|
| p3fold | execR=**-1031612** MaxDD=0.1351 R=-962558 LCB=-3471145 | **-1268551** 0.1475 -1191916 -3550861 | 同 #309（逐位） |
| wf7 | execR=**-2036084** MaxDD=0.1028 R=-1948592 LCB=-4577967 | **-5427281** 0.2601 -5147413 -10865720 | 同 #309（逐位） |
| wf8 | execR=**+4580249** MaxDD=0.0464 R=+4881460 LCB=-788538 | **+6346975** 0.0803 +6851062 -2105181 | 同 #309（逐位） |

证据：`/tmp/m8_report_4cde104869_R.md` vs `/tmp/m8_report_2fd5ee08ba_R.md`（三行数据行全变）；
`diff /tmp/m8_report_2fd5ee08ba_R.md /tmp/m8_armR_report.md` 仅口径标签行差、数据行全同。
本票补跑复核见 §4.2 表（`/tmp/fix408_armR_4cde104869.out`）。

**分解式**（臂R 相对 v4 发表表的总发散）：

```
v4 臂B p3fold execR = -36842
   ├─ 不可二分段（ec728bf6cf..12aa929d44，§4.2）：-36842 → -1031612
   └─ #309 段（2fd5ee08ba，可归因，本节）：      -1031612 → -1268551
HEAD = -1268551
```

⟹ **臂R 的发散不是整体不可归因**：§2.2（execR/MaxDD/R/LCB/三态）的末段可精确归因到 #309；
§2.3（门控拒绝率）的发散已证早于 #309，落在不可二分段（§4.2）；
§2.1（trades 聚合）/§2.4（链深）**区段未定**（§4.2c）。

> 〔**#420 订正（MED-A）**〕原文此处写「§2.1/§2.3/§2.4 的发散仍落在不可二分段」——三项并列
> **过强**：只有 §2.3 在两锚点有实测证据（§4.2 补跑），§2.1/§2.4 无任何锚点级产物，
> 归入不可二分段是无证据的定位。该两项已降格为「区段未定」。

### 4.2c 臂R §2.1/§2.4：归因状态 = **区段未定**（**#420 新增**）

**状态定义**：既**不**断言归 #309，也**不**断言落在 `ec728bf6cf`..`12aa929d44` 不可二分段——
证据不足以支持任何一侧。这不是「不可归因」（不可归因是一个有证据的判定：区间已确定、
区间内不可构建故无法二分），而是**发散所在的提交区段本身尚未被观测**。

**为什么无证据**（三条，照实）：

1. **锚点无 trades 侧产物**：`4cde104869` / `12aa929d44` 两次补跑（§4.2）的 dump 目录
   `/tmp/fix408_dump_*` 已随清理消失（本票复核：`ls -d /tmp/fix408_dump_*` → No such file or directory），
   trades 逐笔产物不可得。
2. **留存产物不含这两项读数**：两锚点留下的是 stdout（`[m8]` 行 + `NEST_GATE_STATS` 行）与四层报告，
   四层报告的 19 列**不含**按 `certificate.dir` 的 trades 聚合（§2.1）与 `certificate.nest_depth`
   直方图（§2.4）——这两项只能从 `trades.jsonl` 现算。
3. **二分残留目录顶不上**：`/tmp/bisect_dump/trades.jsonl` **仍在**（948340 字节，mtime 2026-07-27
   07:56，与同目录 `center_lifecycle.jsonl` / `tower_events.jsonl` 同批），但它是**臂A** 二分末次
   跑批在根目录的残留，**既非臂R，也无法定位到具体 sha**（`opsem_dump.rs:207-209` 每次跑批截断重建
   根目录同名文件，见 §1）⟹ 对臂R §2.1/§2.4 的区段定位无用。

**反向线索（暗示，不作结论）**：本报告 §4.2b 已引的两份臂R 四层报告显示，**wf7 的 `n_orders`**
在 #309 处 317 → 304**（p3fold 222 / wf8 260 两窗不变）：

| 窗 | n_orders @`4cde104869` | n_orders @`2fd5ee08ba`（#309） |
|---|---|---|
| p3fold | 222 | 222（不变） |
| wf7 | **317** | **304** |
| wf8 | 260 | 260（不变） |

订单集在 wf7 上确被 #309 改动过，而 trades 由订单派生 ⟹ **不排除 #309 对 §2.1/§2.4 亦有贡献**。
此处只登记为**暗示**：订单数变化不蕴含 trades 聚合/链深直方图必变，两者之间没有已验证的推导链；
且 p3fold/wf8 两窗订单数不变，线索本身也不齐。

**如何转为确定**：在 `4cde104869` / `12aa929d44` 两锚点各重跑一次臂R 并**保留 dump**
（§4.2 的补跑命令去掉清理即可），据实测的 trades 聚合与 `nest_depth` 直方图定位区段。
本票为纯文本订正票，不跑批。

### 4.3 修复链复合 bit-exact：**成立**（正面结论）

`2fd5ee08ba`（#309，2026-07-26）到本报告自述 HEAD `c126d4cf69` 之间共 **36 个提交**
（`git rev-list --count 2fd5ee08ba..c126d4cf69` = 36；其余口径：至交付 commit `0dc5bc785d` = 37、
至评审后 HEAD `9e3cd25fd6` = 38、含区间起点 `2fd5ee08ba~1..c126d4cf69` = 37）。
〔**#408 订正**：原文写「共 **48 个提交**」——**任何口径都到不了 48**，该数字虚高约 30%。〕

被测区间覆盖的是票面枚举九项中的**八项**：#351/#356/#360/#361/#363/#365/#374/#376。
〔**#408 订正**：原文写「含票面枚举的**全部** …/#246/#361」——**#246 不在区间内**。〕

> **#246 覆盖情况（照实）**：#246 的全部四个在册提交（`e79b12e10f` / `491f638a8c` / `77040317f2` /
> `4cde104869`，其中 `4cde104869` 是 #246 封口提交）**都是 `2fd5ee08ba` 的祖先**
> （`git merge-base --is-ancestor <each> 2fd5ee08ba` 四条全 YES；`git log --oneline
> 2fd5ee08ba..c126d4cf69 | grep '#246'` 为空）⟹ 本区间对 #246 **零覆盖**。
> - **臂A 侧**：#246 的 bit-exact **另有**间接证据——二分 GOOD 链
>   （`12aa929d44` / `e79b12e10f` / `8dce346588` / `381ae777ae` / `57a50d939f` / `bbb130c215` /
>   `4cde104869`）逐点 `execR=+923705`，跨越了 #246 的全部提交。**但那不是本节所引的证据**，
>   本节的「复合 bit-exact」声明不承载 #246。
> - **臂R 侧**：**零覆盖**——臂R 只在 `4cde104869` / `2fd5ee08ba` / HEAD / v4head 四点跑过，
>   最早一点 `4cde104869` 已在 #246 之后，区间内不含 #246 的任何提交。

实测两臂四层报告表：

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
- **触发条件（#408 补写，原文缺）**：该锁**不会自己去抓漂移**，只在有人**手动重跑三窗跑批 +
  手动执行校验器**时生效。三条限制照实登记：
  1. **无 CI 接线**——`grep -rn check_armR_trades_digest .github/` 零命中（姊妹件
     `check_fixture_drift.py` 有 `.github/workflows/ci.yml` 的 `fixture-drift` job，本校验器没有）；
  2. **输入在 `/tmp`**（默认 `/tmp/m8_win_gate`），系统重启即灭失；
  3. **缺产物返回 `EXIT_MISSING=4` 不判红**——产物不存在时校验器不报漂移。
  ⟹ 本锁的能力边界是「**能抓住漂移，但不会自己去抓**」。接 CI 与输入落到非易失路径列入
  §12 后续备忘（LOW-1）。

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
  **票据维度的有效域同样受限（#408 订正）**：只覆盖 #351/#356/#360/#361/#363/#365/#374/#376
  **八项**，**不含 #246**（其提交全在区间起点之前，臂R 侧零覆盖）。声明不抬格为「票面枚举九项全覆盖」。
- **§4.2 的诚实缺席（#408 收窄，#420 再收窄）**：**臂R §2.3** 的发散无 commit 级归因——不是「未做」，
  是**在册提交序列上不可做**（9/10 不可独立构建）。不以「大致归到某票」冒充归因。
  **臂R §2.2 不属此列**——它可归因到 #309（§4.2b）；原报告把不可归因的范围写成整个臂R，#408 已收窄。
  **臂R §2.1/§2.4 亦不属此列**——它们的状态是「**区段未定**」（§4.2c）：两锚点无 trades 侧产物留存，
  证据不足以断言归 #309，也不足以断言落在不可二分段。
  〔**#420 订正（MED-A）**：#408 版本此条写「臂R §2.1/§2.3/§2.4 …在册提交序列上不可做」——
  把只对 §2.3 成立的判定外推到了三项，是有效域膨胀（231号）。§2.1/§2.4 的有效域为**空**：
  这两项从未在任何锚点上被测过。〕
- 全部数值 `[L1机制/费率未标定]`，**不作 alpha 论据、不作策略择优输入**。

---

## 9. 与票体/母 SPEC 的冲突登记

1. **票体前提「均 Off 时应零差异」在本批不成立**——两开关确为 Off，读数仍发散。原因是该前提
   隐含「v4 与今日之间只有 bit-exact 声明的改动」，而 #309 明文未借 bit-exact（§4.1），
   且 v4 锚点本身不在册（§4.4）。前提本身需修正，不是跑批出错。
2. **母 SPEC #385 的修复链枚举缺 #308/#309（LEE M2/M3）**——正是 #309 改变了 treasury 层执行读数。
   建议 T3 报告采本报告 §4.1/§4.2b/§4.2c/§4.3 的分离口径：「**被测八项**枚举链复合 bit-exact 成立
   （#246 在区间外，另引二分 GOOD 链）；发散归 #309（臂A §2.2 + 臂R §2.2）+ 不可二分区间
   （臂R §2.3）+ **区段未定**（臂R §2.1/§2.4）」。〔**#408 订正**：原文写「枚举链」未限定项数，
   T3 引用时须改述为八项。〕〔**#420 订正**：#408 版本此处写「不可二分区间（臂R §2.1/§2.3/§2.4）」
   ——§2.1/§2.4 降格为「区段未定」，T3 引用时须采三档口径。〕
3. **票体要求「臂A/臂B 读数与 v4 发表表逐项比对」已完成**（§2）；「逐条归因到具体开关/commit」
   的完成度**按臂×小节分列**〔**#408 订正**，原文写「只对臂A 可完成……臂R 侧照实登记为不可二分」
   ——**过宽**，臂R §2.2 是可归因的〕〔**#420 订正**，#408 版本把臂R §2.1/§2.4 与 §2.3 并列写作
   「落在不可二分段」——**仍过强**，这两项无锚点级证据，降格为「区段未定」〕：
   - **臂A**：§2.2 全部读数归因到单个提交 `2fd5ee08ba`（#309），完成（§4.1）。
   - **臂R §2.2**：归因到 `2fd5ee08ba`（#309），完成（§4.2b）。
   - **臂R §2.3**：落在 `ec728bf6cf`..`12aa929d44` 不可二分段（9/10 不可独立构建），
     无 commit 级归因，照实登记（§4.2），不伪造归因。
   - **臂R §2.1/§2.4**：**区段未定**——两锚点无 trades 侧产物留存，既不归 #309 也不归不可二分段；
     已知 wf7 `n_orders` 在 #309 处 317→304（暗示，非结论），照实登记（§4.2c）。

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
  归因未抹平（§4.2 照实登记不可二分）〔**#420 订正**：此处「不可二分」在本次订正后仅指**臂R §2.3**；
  §2.1/§2.4 已降为「区段未定」（§4.2c），二者都是「不抹平」的表现，但证据强度不同〕；
  基线复跑 1928/1 与票面数字一致。
  〔**#408 订正**：本条自审当时未发现四处声明与证据不对齐（48↔36、九项↔八项、臂R §2.2 归因缺席、
  §4.2 两锚点无留存证据），故「Acceptance criteria 六条全部有对应落盘证据」这句在 #401 评审
  与本票订正之前是**过强**的。四条现已订正，逐条对照见 §11。〕
- **已修的一条 LOW**：票体「digest golden 文件落地（**含生成命令**）」字面要求命令随 golden 本体走。
  初版命令只在脚本 docstring 与本报告 §5 中，golden JSON 无。**已修**——golden 增
  `_regen_command` / `_check_command` 两个元字段（摘要值 `0x6bf47daf0aa737cd` /
  `0x282c28ca8ca65e16` / `0xcafa7c4846c762cd` 重落后逐位不变，校验器复验 exit=0）。
- **范围登记**：§4.4「v4 报告溯源缺陷」超出本票直接验收范围，已就地标注「非本票修复范围」，
  属诚实登记而非扩大改动面。
- 未发现「看起来实现了但实现得不对」的项；§2 对照表数字经独立核对与 v4 报告原表一致。

---

## 11. 订正记录（#408，据 #401 影子评审打回）

- **订正票**：#408　**评审票**：#401（打回，MED×4 + LOW×5）　**评审报告**：
  `chanlun/review-results/shadow-review-387-20260727.md`
- **性质**：纯文本订正 + 一次补跑取证。**`rust/src` 零改动、主批零重跑**、基线 1928/1 不因本票变化
  （本票不新增/不删除任何 Rust 测试）。
- **历史不改写**：已入库的 commit message、issue 评论一律不改，**以本节为覆盖性订正记录**。

### 11.1 四条 MED 逐条

| # | 原断言（原报告） | 订正后 | 证据 |
|---|---|---|---|
| **MED-1** | §4.3「`2fd5ee08ba` 到 HEAD `c126d4cf69` 之间共 **48 个提交**」 | **36 个提交**（报告自述 HEAD 口径）；并列全部口径：至 `0dc5bc785d` = 37、至 `9e3cd25fd6` = 38、含起点 = 37。任何口径都到不了 48 | `git rev-list --count 2fd5ee08ba..c126d4cf69` = 36（§4.3 内已列四口径表述） |
| **MED-2** | §4.3「含票面枚举的**全部** #351/#356/#363/#365/#376/#360/#374/#246/#361」（九项）；§0.1「整条修复链」 | 被测区间覆盖**八项**（#351/#356/#360/#361/#363/#365/#374/#376）；**#246 零覆盖**——其四个提交全为 `2fd5ee08ba` 的祖先。臂A 侧另有二分 GOOD 链间接证据，**臂R 侧零覆盖** | `git merge-base --is-ancestor {e79b12e10f,491f638a8c,77040317f2,4cde104869} 2fd5ee08ba` 四条全 YES；`git log --oneline 2fd5ee08ba..c126d4cf69 \| grep '#246'` 空 |
| **MED-3** | §9.3「『逐条归因到具体开关/commit』只对臂A 可完成；臂R 侧照实登记为不可二分」；§4 无臂R §2.2 归因 | 新增 **§4.2b**：臂R §2.2 三窗读数同样在 #309 处翻转且翻转后即为 HEAD 值 ⟹ **臂R 发散 = #309 段（可归因）+ pre-#309 段（不可二分）**；§0.2/§8/§9.2/§9.3 同步收窄。〔**#420 再订正**：本行末的分解式「#309 段 + pre-#309 不可二分段」**只对 §2.2/§2.3 成立**；§2.1/§2.4 已降格为「区段未定」（§4.2c、§13 MED-A），引用本行时须采三档口径〕 | `/tmp/m8_report_4cde104869_R.md` vs `/tmp/m8_report_2fd5ee08ba_R.md`（三行数据行全变：-1031612→-1268551、-2036084→-5427281、+4580249→+6346975）；本票补跑复核 `/tmp/fix408_armR_4cde104869.out` |
| **MED-4** | §4.2「臂R 的 `NEST_GATE_STATS` 在 `4cde104869` 与 `12aa929d44` 处已经是今日形态（…逐字段相同）」——**无留存 stdout** | **原断言经补跑证实，未改写**；两锚点 stdout 落盘并在 §4.2 逐项引用 | `/tmp/fix408_armR_4cde104869.out`、`/tmp/fix408_armR_12aa929d44.out`（三窗 `NEST_GATE_STATS` 与 HEAD 本批逐字段相同，含 admitted / nest_pass / xzd_pass / cert_none / nest_n_delta_false / xzd_gate_fail 全部子项） |

### 11.2 本票自行发现的第五处（评审未指出，照实登记）

| 原断言 | 订正后 | 证据 |
|---|---|---|
| §0.2 / §4.1 / §4.2「`4cde104869`（#309 之父）」 | `2fd5ee08ba` 的父提交实为 **`57a50d939f`**，`4cde104869` 与之严格夹着 **5 个提交**（含 `37a56deab1` = #308 LEE M2；区间 `4cde104869..2fd5ee08ba` 共 6 条，多出的第 6 条是 #309 自身）〔**#420 订正（LOW-B）**：本行原写「相隔 6 个提交」，与正文 5 项枚举口径不一〕。真父 `57a50d939f` 独立跑批亦为 GOOD（三窗 execR = +923705 / -1414690 / +2508870）⟹ **§4.1 结论不变且更稳**（GOOD/BAD 分界直接落在父子相邻处），订正的是提交谱系标签 | `git rev-parse 2fd5ee08ba^` = `57a50d939f`；`git log --oneline 4cde104869..2fd5ee08ba` = 6 条；`/tmp/bisect_log/57a50d939f.out` |

**承此误的其它载体（#420 补登，LOW-A 后半）**：

| 载体 | 承误处 | 处置 |
|---|---|---|
| **#401 评审报告** | 其 MED-2 写「`4cde104869` …恰是区间起点 `2fd5ee08ba` 的父」 | 不改写；该误述不影响其 MED-2 的结论（#246 四提交全在区间外，独立成立） |
| **#387 resolution 评论**（第 15 行） | 「臂A §2.2 全发散 ⟸ 单个提交 `2fd5ee08ba`……**其父 `4cde104869`** 逐位复现 v4 发表表」 | 不改写历史评论；以本节覆盖（真父 `57a50d939f`，两点读数相同，§4.1 结论不变） |

〔**#420 订正（LOW-A 后半）**：#408 版本此处只登记了 #401 评审报告一个承误载体，漏登 #387
resolution 评论——而后者正是 #385 红线裁定的输入材料之一，漏登会让该误述在下游无覆盖记录。〕

### 11.3 扩散面（历史不改写，以本节覆盖）

| 载体 | 受影响断言 | 覆盖方式 |
|---|---|---|
| commit `0dc5bc785d` message | 「票面枚举修复链（#351..#361 全链）对 treasury 读数零影响」（= **九项口径**，MED-2 的错误）；「臂R 早期发散区间 9/10 提交不可构建，照实登记不可二分」（= **臂R 不可二分过宽**，MED-3 的错误） | 不改写历史；以 §11.1 MED-2/MED-3 + §13 MED-A 覆盖（正确口径：被测**八项**；臂R 分三档——§2.2 归 #309 / §2.3 不可二分 / §2.1/§2.4 区段未定）。〔**#420 订正（LOW-A）**：本行原登记为「48 提交」——`git log -1 --format=%B 0dc5bc785d` 全文**不含**「48 提交」（唯一命中的 `48` 在 golden 十六进制串 `0xcafa7c4846c762cd` 内），既错登了它没携带的错误，又漏登了它实际携带的上述两条〕 |
| #387 resolution 评论 | 「48 提交」「整条修复链…零影响」（九项）「臂R 侧不可归因」；另承真父之误（「其父 `4cde104869`」，见 §11.2） | 不改写；以 §11.1 MED-1/2/3 + §11.2 + §13 MED-A 覆盖 |
| #401 票体验收标准 | 沿用「48 提交」 | 以本节覆盖；#401 已关，不追改 |
| **#385 Lead 红线裁定评论**（[issuecomment-5090766856](https://github.com/xy7365527-lang/NewChanlun/issues/385#issuecomment-5090766856)，2026-07-27） | 背景段写「48 提交 × 19 列 × 三窗 × 双臂逐字段相同」（→ 36，八项）；「臂R 早期发散区间 9/10 提交不可构建，无法归因到单 commit」（→ 仅 **§2.3**：§2.2 可归因 #309，§2.1/§2.4 **区段未定**）〔**#420 订正**：#408 版本此格写「→ 仅 §2.1/§2.3/§2.4」，仍把 §2.1/§2.4 算作不可二分〕 | 见 §11.4 |

### 11.4 对 #385 红线裁定的影响（显式声明）

原报告 §9.3 那句「臂R 侧照实登记为不可二分」是 **#385 红线裁定评论的输入材料**。本次订正的方向是：

> **裁定逻辑因本订正更稳，而非被推翻**——订正把臂R 发散中**可归因的部分变多了**
> （§2.2 归 #309），不可归因的部分相应缩小到 §2.3〔**#420 订正**：#408 版本此处写
> 「§2.1/§2.3/§2.4」；§2.1/§2.4 现为「区段未定」，既不算已归因也不算已定位到不可二分段〕。裁定的两条支撑
> （「v4 锚点来历不明」§4.4、「不可构建区间考古无增量信息」§4.2）**均未被触动**，
> MED-4 补跑还把「门控侧发散早于 #309」这条支撑从「无留存证据」升为「有 stdout 可复核」。
> ⟹ 裁定结论（接受解释、不判红、以 #387 臂R digest golden 为后向回归锚）**维持**。

但**裁定记录须订正**：T3 验收报告（#389）在引述红线触发事实时，应采本报告订正后的口径——
「臂R 发散 = **§2.2 归 #309（可归因）** + **§2.3 落在 `ec728bf6cf`..`12aa929d44` 段（不可二分）**
+ **§2.1/§2.4 区段未定（无锚点级 trades 产物）**」，
且「枚举链复合 bit-exact」限定为**八项**（#246 另引二分 GOOD 链，臂R 侧无覆盖）。

〔**#420 订正（MED-A）**：#408 版本此处写的两段式口径（「#309 段 + 不可二分段」）把 §2.1/§2.4
默认塞进了不可二分段。本票降格为三档后，**裁定结论仍维持**——#420 的方向是把「已定位」的部分
**减少**（§2.1/§2.4 由「已定位到不可二分段」退为「未定」），而裁定的两条支撑
（「v4 锚点来历不明」§4.4、「不可构建区间考古无增量信息」§4.2）**都不依赖** §2.1/§2.4 的定位：
前者与臂R 归因无关，后者只需 §2.3 这一项即可成立。⟹ 不判红、以 digest golden 为后向回归锚，
维持不变。〕

---

## 12. 后续备忘（#401 LOW×5 登记）

文本级的 LOW-1 上半已在本票就地修（§5 新增「触发条件」小节）。其余为 `scripts/` 与 `.github/`
的代码级改动，**超出本订正票的文件面**（#408 只碰 `chanlun/`），登记为后续票备忘。
**本票不代开 ticket**——LOW-2..5 至本节写就时**尚未挂成可追踪 issue**，挂到哪张票（新开 / 并入
T2 #388 / 并入 T3 #389）由编排者定；在此之前本节是它们唯一的登记处，读者不应据此认为已有票在跟。

| # | 位置 | 问题 | 建议处置 | 本票状态 |
|---|---|---|---|---|
| LOW-1 | `.github/workflows/ci.yml` / `scripts/check_armR_trades_digest.py` | 回归锁无 CI 接线（`grep -rn check_armR_trades_digest .github/` 空）、默认输入在易失的 `/tmp/m8_win_gate`、缺产物 `EXIT_MISSING=4` 不判红 ⟹ 「能抓漂移但不会自己去抓」 | 接 `fixture-drift` 同款 job，或把输入路径与触发条件写死在文档层 | **文本半已修**（§5 触发条件已写明）；CI 接线待后续票 |
| LOW-2 | `check_armR_trades_digest.py:128` / `:152` | `EXIT_MISSING=4` 双语义（跑批产物缺失 / golden 文件不存在），docstring `:27` 只写前者；姊妹件 `check_fixture_drift.py` 用 3/4/5 分层 | 拆码（如 4=产物缺失、5=golden 缺失）并补 docstring | 待后续票 |
| LOW-3 | `check_armR_trades_digest.py:119` | `--regen` 无任何门，误跑即静默重落基线，回归锁自毁 | 加 `--force` 二次确认，或落盘前打印 diff | 待后续票 |
| LOW-4 | `check_armR_trades_digest.py:71-77` | `readings_of` 边界校验弱：`certificate` 缺失取 `<none>`、`pnl_raw_unlevered` 缺失静默取 `0.0`，与 `coding-style.md`「Never trust external data / Fail fast」相左（digest 能兜住真漂移，但字段级 diff 会给出误导性的「pnl 相同」） | 缺字段 fail-loud | 待后续票 |
| LOW-5 | `check_armR_trades_digest.py:41` | `WINDOWS` 三窗硬编码，dump 目录出现第四窗时既不校验也不告警——「全绿」语义悄悄退化为「这三个窗无漂移」 | 对 golden 未覆盖的窗告警 | 待后续票 |

---

## 13. 订正记录（#420，据 #417 影子复审）

- **订正票**：#420　**复审票**：#417（**PASS with MED×1 + LOW×3**，不打回 #408）　**复审报告**：
  `chanlun/review-results/shadow-review-408-20260727.md`
- **被订正对象**：#408 的订正结果（commit `78151e988e`）——本票是**对订正的订正**，
  链条为 #401 打回 → #408 订正 → #417 复审 → #420 再订正。
- **性质**：**纯文本订正，零跑批、零取证**。`rust/src` / `scripts/` / `.github/` 零改动，
  改动面只有本文件一个。基线 1928/1 不因本票变化（不新增/不删除任何测试）。
- **历史不改写**：已入库的 commit message、issue 评论一律不改，**以本节 + §11 为覆盖性订正记录**。

### 13.1 三条逐条

| # | 原断言（#408 版本） | 订正后 | 证据（本票独立复核） |
|---|---|---|---|
| **MED-A** | §4.2b / §8 / §9.3 三处：「臂R **§2.1/§2.3/§2.4** 的发散落在 `ec728bf6cf`..`12aa929d44` 不可二分段」 | 三项**分档**：**§2.3** 维持「落在不可二分段」（两锚点逐子项实测已证，§4.2）；**§2.1/§2.4** 降格为「**区段未定**」——证据不足以断言归 #309，也不足以断言落在不可二分段（新增 **§4.2c**）。反向线索（wf7 `n_orders` 317→304）照实写入，标为**暗示**不作结论 | ① `ls -d /tmp/fix408_dump_*` → `No such file or directory`（两锚点臂R trades 产物未留存）；② 四层报告 19 列不含 trades 聚合与 `nest_depth` 直方图（`/tmp/m8_report_*_R.md:19` 表头直读）；③ `n_orders` @`4cde104869` = 222/**317**/260 vs @`2fd5ee08ba` = 222/**304**/260（同两文件 `:21-23`）；④ `/tmp/bisect_dump/trades.jsonl` 仍在但属臂A 且不可定位 sha（`ls -la /tmp/bisect_dump/`） |
| **LOW-A** | §11.3 扩散面首行：commit `0dc5bc785d` message 受影响断言 = 「48 提交」 | 该 message **不含**「48 提交」；实载错误是**两条**：「票面枚举修复链（#351..#361 全链）…零影响」（九项口径，MED-2）与「臂R 早期发散区间 9/10 提交不可构建，照实登记不可二分」（过宽，MED-3）。按实登记。另在 §11.2 补登第二个承误载体（**#387 resolution 评论**「其父 `4cde104869`」） | `git log -1 --format=%B 0dc5bc785d \| grep -n 48` → 唯一命中在 golden 十六进制串 `0xcafa7c4846c762cd`，无「48 提交」；`gh issue view 387 --comments` 第 15 行含「其父 `4cde104869` 逐位复现 v4 发表表」 |
| **LOW-B** | §4.1 订正段「`4cde104869` 与 `2fd5ee08ba` 之间隔 **6 个提交**（… **等**）」（§0.2、§11.2 同口径） | 严格夹在中间的是 **5 个提交且正文已全列**（`37a56deab1` / `38cbe04fa7` / `8dce346588` / `1cac0783f2` / `57a50d939f`）；「6」是含 #309 自身的区间计数，两处口径并存已注明；删「等」 | `git rev-list --count 4cde104869..2fd5ee08ba` = **6**（末条即 `2fd5ee08ba`）；`git rev-list --count 4cde104869..2fd5ee08ba^` = **5** |

### 13.2 「区段未定」的语义（防再次膨胀）

三种状态互斥，不可混用：

| 状态 | 含义 | 证据要求 | 本报告实例 |
|---|---|---|---|
| **可归因** | 发散翻转点定位到单个 commit | 该 commit 及其父点上的读数实测对照 | 臂A §2.2、臂R §2.2（归 #309） |
| **不可二分** | 区间已确定，但区间内提交不可独立构建 ⟹ **在册序列上无法**再细分 | 区间两端读数实测 + 区间内构建失败日志 | 臂R §2.3 |
| **区段未定** | 发散所在区段**尚未被观测** | ——（正因为无证据才是此状态） | 臂R §2.1/§2.4 |

**「不可二分」是有证据的判定，「区段未定」是没有证据的状态**——把后者写成前者，是把「没测过」
说成「测过且测不出来」，属声明膨胀（090号 / 231号有效域膨胀）。转为确定的路径见 §4.2c 末段。

### 13.3 #417 的第四条（LOW-C）——**本票不做，说明理由**

#417 另开 LOW-C：#385 第二条评论（裁定订正）把「裁定的两条支撑」复述为
「声明链复合 bit-exact 成立 + v4 锚点溯源缺陷」，与原裁定的两条（「v4 锚点来历不明」+
「不可构建区间考古无增量信息」）不符，且与该评论自己第 2 条（九项收窄为八项）自相矛盾。

**本票不处理**，两条理由：

1. **载体不在本票文件面**——LOW-C 的修复动作是**编辑 GitHub 评论**，而 #420 票体硬约束为
   「只碰 T1 报告一个文件」；本票的交付形态是该文件的未提交文本改动，不含对 issue 评论的写动作。
2. **本报告 §11.4 的引述本就是正确的那两条**（复审报告 L77 明确认可），故报告侧无可订正处。

⟹ LOW-C 悬置，处置权在编排者（编辑 #385 评论即可，一句话的事）。**在它被处理之前，
#385 那条评论对「两条支撑」的复述与本报告 §11.4 不一致，读者以本报告为准。**

## 14. #446 活动集唯一性修复后的红线订正（2026-07-28）

> **覆盖性订正**：本节替代本报告此前关于臂 R `trades.jsonl` golden 仍适用于当前代码态的
> 结论；旧数字和旧 digest 仅保留为修复前历史快照，不再作为当前回归基线。

#446 证实 `strategy_target_legs` 的输入活动集曾存在同一 `ElementId` 的树前缀 idx 与 registry
追加 idx，release 会静默双计。唯一注册修复后，臂 R 三窗与旧 `/tmp/m8_win_gate` 全部发生真实漂移，
故依 #385 红线先例重锚同一 golden 文件
`chanlun/review-results/treasury-reverify-t1-armR-trades-golden-20260727.json`。

| 窗 | 旧→新 trades | 旧→新 bytes | 旧→新 FNV-1a 64 | Long/Short（新） |
|---|---:|---:|---|---:|
| p3fold | 149→141 | 206057→195220 | `0x6bf47daf0aa737cd`→`0xac5952cfcd8b8746` | 72/69 |
| wf7 | 196→184 | 270684→253658 | `0x282c28ca8ca65e16`→`0x981a0560b8db0b40` | 99/85 |
| wf8 | 168→159 | 228108→216129 | `0xcafa7c4846c762cd`→`0x18291f8ba8f1d40f` | 87/72 |

逐笔对拍以六文件内唯一的 `position_node_id` 为连接键，忽略仅作输出序号的 `trade_id`：

| 窗 | unchanged | changed | removed | added |
|---|---:|---:|---:|---:|
| p3fold | 29 | 105 | 15 | 7 |
| wf7 | 2 | 169 | 25 | 13 |
| wf8 | 52 | 106 | 10 | 1 |

完整逐笔分类、变化字段及 old/new 值落在 `/tmp/446_armR_trades_diff.json`，人读索引在
`/tmp/446_armR_trades_diff.md`。因果边界严格收窄为：受控对拍中改变交易决策的生产代码只有
#446 活动腿唯一注册；probe/report 是 run 后观测，测试代码不进入生产路径。因此所有新增、删除和
字段变化归为“双计消除后的轨迹变化”，但不虚称每一笔都直接命中已知碰撞
`ElementId(0,162)`。

重锚后复核：

```
$ python3 scripts/check_armR_trades_digest.py --dump-dir /tmp/446_armR_dump
p3fold: n_trades=141 digest=0xac5952cfcd8b8746 bytes=195220 ✓
wf7: n_trades=184 digest=0x981a0560b8db0b40 bytes=253658 ✓
wf8: n_trades=159 digest=0x18291f8ba8f1d40f bytes=216129 ✓
臂R trades 逐位无漂移。
```

修复前门失败证据 `/tmp/446_armR_gate_before_reanchor.out`，修复后 exit 0 证据
`/tmp/446_armR_gate_after_reanchor.out`。本轮基于 base HEAD
`a12a1022d9ddd8d1cae867a107a3a33c359358cf` 的未提交 #446 工作区跑出，golden 元数据已明确
登记“未提交、待编排者验收提交”，未伪装成已有 commit。

最终核验期间并行 #421 将分支前推到 `6e15ceffeeb8259c065bf7c0ec9ec7c65935737c`，提交面仅
`classifier/level_view.rs`、`classifier/nest_lifecycle.rs` 与 `bin/p123_fast_replay.rs`，未触
coverage/runner/wverify/review-results 本票文件面。臂 R 三窗 dump 在前后 `cmp` 逐字相同；golden
元数据同时登记初始基点与最终核验 HEAD，未把并行推进藏掉。

> **canonical 目录订正（编排者验收轮，2026-07-28）**：修复批默认门曾指向冻结副本
> `/tmp/446_armR_dump`——与未来跑批惯例（`OPSEM_DUMP_DIR=/tmp/m8_win_gate/<tag>`，§14.2）
> 脱节，会静默失效。已恢复 `check_armR_trades_digest.py` 默认根为 `/tmp/m8_win_gate` 并将新产物
> 拷入该目录（无参门 exit 0 复核在案）；修复前产物存档于 `/tmp/423_backup_m8_win_gate`
> （#423 在案 6/6 SAME 即修复前态），逐笔 diff 物证 `/tmp/446_armR_trades_diff.{md,json}`。
