# 影子评审报告：#389 T3 帽臂（臂C）+ 臂间归因 + treasury 重验总报告

- **评审票**：#431　**被评票**：#389（已关，resolution 在案）　**母 SPEC**：#385
- **被评对象**：commit `af1d17af12`（`wverify_run.rs` +165 / `fee_account_decomposition.py` +28 / `chanlun/review-results/treasury-reverify-20260727.md` +662）
- **评审工位**：`/tmp/nc-review-389`（隔离 worktree，detached @ `af1d17af12`）。**零 git mutation**，主仓与 `/tmp/kimi-nest-mainline` 全程未碰。
- **评审方式**：独立复算（不转述报告自证）——trades.jsonl 逐笔重算、stdout 原文逐数核对、脚本重跑、回归门亲跑、行数/常量/env 名实核验。
- **认识论**：本评审的核验面 = 已落盘产物 + 仓内代码（L0/L1 核对），**未重跑三窗回测**（跑批成本，且回归门与产物 stdout 已提供逐数证据）。

---

## 结论

**PASS with MED×1 + LOW×3。**

七条重点核查中 ①②③④⑤⑦ 全部通过且经独立复算；**⑥（US10 缺口与 #419 承接票的引用）不通过**——报告全文零处引用 #419 / #422 / #423，三张票在 commit 落地前均已建立，报告仍写「待 Lead 裁定」「上浮 Lead」。此项为纯文本可修的登记滞后，不触动任何数值、不推翻任何结论，故判 MED 而非打回。

数值面**零错误**：报告中我逐项复算过的每一个数（三臂 × 三窗 × 四张表 + 费率分解九行 + §5.3 十三行百分比）与产物原文/独立重算**逐位相同**。

---

## 分级发现

### MED-1：承接/裁定票号 #419 / #422 / #423 在报告中零引用，处置状态登记滞后于 Lead 裁定

- **位置**：`chanlun/review-results/treasury-reverify-20260727.md:511`（§9 US10 缺口）、`:141`（§2.2 blockquote 末「上浮 Lead」）、`:578`（§12 条9）
- **主张**：报告把三处处置登记为「待 Lead 裁定 / 上浮 Lead 择一」，但对应的 Lead 裁定在 commit 之前已下、承接票已建：
  - #385 裁定其二 → US10 承接票 = **#419**（`2026-07-27T14:23:06Z` 建）
  - #385 裁定其三 + 票号订正 → 措辞订正+#418 LOW×5 = **#422**、层4 恢复 = **#423**（均 `14:51Z` 建）
  - 被评 commit 作者时间 = `2026-07-27 11:08:43 -0400` = **`15:08Z`**，晚于三张票 17～45 分钟。
- **证据**：
  ```
  $ grep -n "#419\|#422\|#423" chanlun/review-results/treasury-reverify-20260727.md
  （无输出）
  $ gh issue view 419 --json number,createdAt   # 2026-07-27T14:23:06Z
  $ gh issue view 422 --json number,createdAt   # 2026-07-27T14:51:57Z
  $ gh issue view 423 --json number,createdAt   # 2026-07-27T14:51:59Z
  $ git show -s --format=%aI af1d17af12          # 2026-07-27T11:08:43-04:00
  ```
- **为什么是 MED 而非 LOW**：#385 的红线登记义务写明「SPEC 完成判定须带此登记」，而这份报告就是 map #59 Destination 末项**被机械核对为「完成」的那份文件**。issue 线程里 #389 resolution 已正确引用 #419/#423，但落盘报告会比 issue 线程活得久；下游读者按报告去找「US10 承接到哪」会得到「待裁定」的错误状态。
- **为什么不是打回**：不影响任何读数、任何归因结论、任何红线判定；#389 resolution 已在 issue 层把 #419/#423 记全。
- **最小修复**（三处文本，无需重跑）：
  1. §9 末条「承接票待 Lead 裁定（#388 resolution 在案）」→「承接票 **#419**（Lead 裁定其二，2026-07-27）」；
  2. §2.2 blockquote 末「上浮 Lead：处置 (a)/(b) 择一」→「Lead 裁定其三已下：措辞订正随 **#422**、层4 按档位形态分叉恢复随 **#423**；本票只登记不改码」；
  3. §12 条9 同步。

### LOW-1（承袭 #388，本票未触）：`config.rs` 行号引用偏移一行，而该引用正是脚本自述的漂移防线

- **位置**：`scripts/fee_account_decomposition.py:24`（docstring）与 `:52`（常量注释），报告 §3 口径条转载
- **主张**：引用写 `rust/src/theta_v0/config.rs:301-303`，实际三常数在 **302-304**（301 是 `entry_delay_bars: 1`）。
- **证据**：
  ```
  $ sed -n '301,304p' rust/src/theta_v0/config.rs
              entry_delay_bars: 1,
              commission_bps: 1.0,
              slippage_bps: 2.0,
              tax_bps: 0.0,
  ```
- **为什么值得登记**：脚本 docstring 自己声明「防线 = 表头随每次运行打印这三个数**与其来源行号**，复核时人工对一眼」。防线依赖行号准确；行号偏移使人工对照落到错误的行。数值本身（1.0/2.0/0.0bp）**核对无误**。
- **归属**：该注释在 `af1d17af12` 的 diff 之外（本票只改 `--armC-dir` 与缺失告警），属 #388 面既有欠账。**不构成本票缺陷**，登记供 #422 顺手带走。

### LOW-2：增量行数措辞与事实略有出入

- **位置**：报告 §13.1「本票再 +约 150 行」、commit message 同口径
- **事实**：`git show --stat` = `wverify_run.rs | 165 +++++`；改后 `wc -l` = **2077**。
- **判定**：「2000+ 行」相符；「约 150 行」低报 15 行（-9%）。措辞方向不是自我美化的反面（低报增量会使违规看起来轻一点），故登记。**最小修复**：改为「+165 行，改后 2077 行」。

### LOW-3：「母 SPEC Out of Scope 明列同类拆分」的措辞易被下游读成豁免

- **位置**：报告 §13.1 未修项第一条
- **事实**：#385 Out of Scope 逐字为「**runner.rs/fill.rs 拆分**、H-2 门面 glob 架构」——**不含 `wverify_run.rs`**。报告用的词是「明列**同类**拆分（runner/fill）」，属类比，措辞本身没说谎，且同段已写明「评审 Standards 轴判为**硬违规**，本报告接受该判定……登记为既有欠账 + 本票新增贡献，不粉饰、不当作合规」。
- **风险**：只读到第一句的下游会得到「已被 SPEC 豁免」的印象。**最小修复**：补一句「`wverify_run.rs` 本身**不在** #385 Out of Scope 列表内，此处是类比不是豁免」。

---

## 两轴分述

### Standards 轴

**合规面（复核通过）**

| 项 | 核验 |
|---|---|
| 090 声明膨胀 | `apply_m8_level_cap` 非法值 `panic!` 而非静默 false；函数文档写明与 `ENFORCE_GROSS_CAP` 先例的**有意偏离**及理由。核实 `apply_enforce_gross_cap_from_env` 确为 `== Ok("true")` 静默模式 ⟹ 偏离登记属实。 |
| 231号（有效域 / L0 同义反复） | 函数文档明写「对编译期常量做 `Σw_ℓ≤1` 运行时断言 = 零信息增量 L0 同义反复」并**删除**该断言，把承保点落在测试上。这是评审史上少见的**主动放弃一条看上去很硬的断言**，判定正确：`M8_LEVEL_CAP_WEIGHTS_310` 是 `const [f64;6]`，运行时断言恒真。 |
| 单一来源 | `lee_row_cells()` 抽出六字段，stdout 行与产物表共用 ⟹ 无两处手抄漂移。核实两处确实都走该数组解构。 |
| 参数归属 | `w_ℓ ∈ Θ_risk` 声明在常量文档、产物声明块、§6、§11 四处一致；`vec![0.05; 6]` 与 `runner.rs:2976`（#310 本体验收）/ `runner.rs:3020`（#351 补课）**逐字同值**，「仓内唯一既有配置」的考据成立（`grep` 无第二组数值）。 |
| v3 硬禁令 | 全文 `grep -E "alpha\|择优\|更优\|建议使用\|应采用\|推荐"` 仅命中自禁条款本身（8 处，全部是「不含 alpha 声明」「不作择优输入」形态）。§5.3 十三行读数全部以「Δ」呈现，**无一处**写「更优/应采用」。帽政策只量化不评判 ✓。 |
| bit-exact 中性 | 亲跑 `check_armR_trades_digest.py` **exit=0**，三窗 digest 全中；另独立 `cmp` 了 `/tmp/{m8_win_gate,m8_win_armD}/<tag>/{trades,tower_events}.jsonl` 与 `/tmp/389_backup/` 的 **12 个文件对，逐字节全同** ⟹ 新增 env 钩子 + 无条件 stderr 读数行在未设时确为 no-op，实证成立。 |

**登记项核验（票体额外要求的两条）**

1. **TDD 红绿顺序违规**：登记措辞 = 「实现与三个测试在同一轮编辑内落地，实现略先于测试，未走『先看到红』；三测试的可证伪性事后核实」。
   - 「可证伪性」我复核**属实**：`m8_level_cap_unset_is_noop` 断言 `!enforce_level_cap ∧ level_weights.is_empty()`；`..._true_applies_310_weights` 逐值断言 `vec![0.05;6]` + Σ=0.3 + `≤1`；`..._invalid_value_fails_loud` 是 `#[should_panic(expected="M8_LEVEL_CAP")]`。三条都是真断言，无空测试。
   - 「先红后绿没做到」**无法外部证伪**（单 commit 交付，无红态留痕）。该登记是自我入罪方向，**措辞与可核事实不冲突**，接受。
2. **`wverify_run.rs` 越 800 行上限**：登记措辞 = 「单文件 2000+ 行，越 `coding-style.md` 的 800 行线……评审 Standards 轴判为**硬违规**，本报告接受该判定……不粉饰、不当作合规」。
   - 事实 `wc -l` = **2077**，`coding-style.md` 明文「200-400 lines typical, **800 max**」⟹ **违规成立，措辞相符**。
   - 两点瑕疵已降级登记为 LOW-2（+165 写成「约 150」）与 LOW-3（Out of Scope 类比措辞）。
   - **本评审同意不在本票拆分**：拆分 diff 会把验收改动面淹没，且 #385 已把同类拆分排除在外；但**不同意把它读作合规**——报告自己也没这么读。

**未修项复核**：报告列的 5 条未修项（gross_cap 未同步 fail-loud、第四个同形 env 包装、`M8_LEVEL_CAP` 不开放自定义权重、臂R 三常数手抄、800 行）理由逐条成立。其中「gross_cap 同一缺陷只修一半」的自我指认尤其诚实——论证确实对称，本评审附议「独立票统一三个 env 开关非法值策略」。

### Spec 轴（#389 票体六条 + #385 全文与四条裁定评论）

| #389 验收标准 | 独立复核结论 |
|---|---|
| 臂C 三窗跑批完成，`level_weights` 取 #310 既有配置；LEE 断言逐窗全绿 | **通过**。三窗 stdout `LEE_M4_ARM ... cap=true` 齐全；权重来源考据核实（见上）。 |
| R-vs-D、D-vs-C 对照表落盘（费率科目级 + 形态读数） | **通过**。§3 九行经脚本重跑逐位复现；§5.2/§5.3 经 trades.jsonl 独立重算逐项复现。 |
| 报告落盘且五节齐全；臂R 标「重建基线」 | **通过**。v4 对齐 §2 / 费率分解 §3 / 不变量 §4 / 归因 §5 / 认识论 §6 + 复现 §1 齐全；「重建基线」见 §2 引言、§5.4、§6、§11。 |
| 每臂命令 / env / 数据路径齐全可机械复现 | **通过**（见重点核查 ⑦）。 |
| 无 alpha 声明、无策略择优 | **通过**（见 Standards 轴 v3 行）。 |
| `cargo test --release --lib` 不劣化 | **通过**。`/tmp/389_full_lib_test_final.log` = `1940 passed; 1 failed; 135 ignored`；T2 基线 `/tmp/388_full_lib_test.log` = `1937 passed; 1 failed; 135 ignored`；新测 3 条在日志中出现 3 次 ⟹ 1937+3=1940 逐数对齐。唯一失败仍是 `extract_signals_bit_exact_digest_guard`（#115）。偶发第二失败 `incremental_tower_scaling_dominates_full_synthetic` 在首跑日志中确实存在、后两跑不复现——**披露属实，未隐藏**。 |

**#385 层面的三项未闭合，报告均照实登记不冒充**：US6 部分降级（OQ-9 / dual_ledger 只在 default 档，非逐窗断言——§4.4 主动收窄了「不变量清单逐窗全绿」的读法，这是正确的自我限制）、US10 未达成（§9）、红线触发（§8）。

---

## 七条重点核查逐条结论

### ① 臂C 读数与 `/tmp/389_armC_*.out` 原文逐数核对 — **通过（三窗全表，非抽查一窗）**

拿产物报告副本 `/tmp/389_armC_report_<tag>.md` 的四层表行与报告 §2.2/§2.3 逐字段对：

```
p3fold(产物) | 222 | -1141049 | 950614 | 84218 | 0 | 0 | -2175881 | 0.1948 | ... | 16543670 | 0 | 16438616/16543670 | 84217 | 16354399 | -2097623 | 不可用(...) | 不可用(...)
wf7   (产物) | 287 | -4519343 | 1914037 | 267531 | ... | -6700911 | 0.3058 | ... | 22996038/23471260 | 267530 | 22728508 | -6436069
wf8   (产物) | 260 | +6977425 | 2176287 | 470148 | ... | +4330989 | 0.0876 | ... | 29137716/28721770 | 470148 | 28667568 | +4785250
```
与报告 §2.2 / §2.3 的臂C 三行**逐位相同**；臂D / 臂R 九行同法核对亦逐位相同（含「不可用(标定档有效域收窄 #374)」原文串）。§5.3 的 13 行 Δ 百分比我用产物数独立重算，**全部对齐**（例：`max_abs_net_units` −86.7/−88.8/−86.4%、`n_orders_generated` −31.8/−27.0/−14.1%、`η_corrected` +36.7/+33.8/−13.9%）。

### ② 「择时逐笔全等」抽查 — **通过（不是抽查，是三窗全量逐笔）**

从 `trades.jsonl` 独立重算（不看报告数）：

```
p3fold R/D/C: n=149  dir Long72/Short77  pnl_raw Long+3645.77 Short-15493.57  nest_depth[1,56,48,39,5]  Σpnl -11847.80
wf7   R/D/C: n=196  Long99/Short97  -2379.33 / -15131.81  [13,73,63,33,14]  Σ -17511.14
wf8   R/D/C: n=168  Long88/Short80  +24155.68 / -7937.15  [15,54,58,28,13]  Σ +16218.53
timing R==D: True   D==C: True   （(entry_bar,exit_bar) 序列全等，三窗）
units 相同笔数 C vs D: 45/149、50/196、42/168
exit_type C==D 全同；平均持仓 bar 952.60/1281.38/1644.42 两臂全同
```
与报告 §2.1 / §5.2 **逐位相同**，包括「约 2/3 的笔仓位规模改变」的对应数（149−45=104、196−50=146、168−42=126）。

复现命令（只读）：
```python
rows=[json.loads(l) for l in open(f'/tmp/m8_win_arm{X}/{tag}/trades.jsonl')]
[(r['entry_bar'],r['exit_bar']) for r in rows]  # 三臂比较
collections.Counter(r['certificate']['nest_depth'] for r in rows)
```

### ③ LEE 稀疏性全绿的非平凡性（`n_cap_narrowed>0` 且 `unexplained=0` 的读数出处）— **通过**

出处 = 本票新增的 `LEE_M4_ARM` stderr 行（`wverify_run.rs` 循环内，无条件打印），原文：

```
LEE_M4_ARM p3fold cap=true  n_cap_narrowed=885 n_rescaled=210 max_abs_net_units=117 off_clock=129 off_clock_explained=129 off_clock_delta=272 off_clock_delta_unexplained=0
LEE_M4_ARM wf7    cap=true  n_cap_narrowed=927 ... off_clock_delta=311 ... unexplained=0
LEE_M4_ARM wf8    cap=true  n_cap_narrowed=819 ... off_clock_delta=281 ... unexplained=0
```
与报告 §4.1 九行（含帽关 D/R 六行）**逐位相同**。非平凡性双前置成立：`n_cap_narrowed` 885/927/819 > 0（帽真 binding）且 `off_clock_delta` 272/311/281 > 0（逐级谓词有被检对象）。

**断言真实性核实**：`assert!(s.per_level_sparsity_has_no_unexplained_violation(), ...)` 确在 `if cfg.risk.enforce_level_cap` 分支内逐窗执行，非事后打印。**帽关臂的 0 是观测不是断言**——报告 §4.1 第二个 bullet 明写「帽关两臂的绿是弱证据」，判据平凡退化的机制也写了，**未把观测冒充断言**。「有效域跃升」（L1 合成 fixture → L2 真实跑批）核实属实：`runner.rs:3016` 的 `lee_m4_cap_on_sparsity_has_no_unexplained_violation` 用的是 `random_walk_dataset("RW9000M2D", 9000, ...)`，确为 9000-bar 合成。

### ④ 归因三档口径与 #420（`8cb64c2983`）一致性 — **通过，无旧两段式主行残留**

`grep -n "不可二分"` 命中 15 处，逐处判定：

- **主口径处**（§0 条6、§5.4 表、§6 标签表、§8.1、§12 条8）= **三档**（§2.2 归 #309 / §2.3 不可二分 / §2.1、§2.4 区段未定），与 #420 入库文本一致。
- **旧 #408 两段式**只出现在 §8.3 的订正链叙述（`:470`）与 §5.4 口径沿革（`:366`），两处**都带显式沿革标记**（`〔已被 #420 再订正……本行是 #408 当时的口径原文，保留作沿革，不是本报告的当前口径〕`）。
- 与 T1 报告（`treasury-reverify-t1-armR-20260727.md`，#420 订正后版）交叉核对：其 §4.2c「§2.1/§2.4 区段未定」、§8 行「#408 版本把这两项一并写进本节结论，证据不足」——**口径一致**。
- §8.1 红线触发面已同步收窄为「§2.3 一项成立」，并附「两种口径下红线均已触发，变的只是触发面宽窄」——逻辑正确，触发结论不依赖订正。

### ⑤ 红线登记与 #385 四条裁定评论逐点一致 — **通过**

| #385 评论 | 报告落点 | 一致性 |
|---|---|---|
| 裁定（红线触发案） | §8.2 三条逐字要点 + §8.1 触发事实 | ✓ 三条（接受解释不判红打回 / golden 重锚 / 照实写进 T3 报告）齐备 |
| 裁定订正（#408） | §8.3 MED-3/4/2 + 谱系订正 `57a50d939f` + 数字订正 36/37/38（非 48） | ✓ 逐条转载，`2fd5ee08ba` 真父与「八项非九项」均在案 |
| LOW-C 措辞订正（#417） | §8.2 末「裁定的两条支撑：①声明链复合 bit-exact 成立（限被测八项）②v4 锚点溯源缺陷实锤」 | ✓ **按订正后表述写**，未沿用被订正的旧措辞；「限被测八项」的限定词也带上了 |
| 裁定其二（US10 部分达成） | §9 全节 + §6 标签表 OKLO 行标 L1 费用面 | ✓ 口径正确（不冒充达成），**但承接票号缺失 → MED-1** |
| 裁定其三 + 票号订正（层4） | §2.2 blockquote：per-notional 档 `FeeUnit::Notional` 不依赖 (qty,px)、恒 12bp、标量其实可定义、缺席是一刀切保守收窄 | ✓ **措辞与裁定实质完全一致**；但处置写「上浮 Lead 择一」而非「已裁定 → #422/#423」→ **MED-1** |

红线三条中另两条「未触发」的核实：不变量无一条破（§4.2 三条逐窗 + §4.4 default 档，且 §4.4 主动收窄措辞）；标定臂标签在案（我在 `/tmp/389_armC_report_p3fold.md` 首部读到原文 `**口径标签：[L2费率标定: datum cdd23adef9b3]**`，且 `shasum -a 256 analysis/data_cache/venue_fee_binance_spot_20260726.json` = `cdd23adef9b3...`，前 12 位吻合、sidecar 一致）。

### ⑥ US10 缺口与 #419 承接票的引用正确 — **不通过（MED-1）**

见 MED-1。US10 的**事实口径**登记正确（费用面 L1 已达成 / treasury 层未达成 / 技术原因 = `load_by_symbol("BTC")` 硬编码，我核实该调用确在 `wverify_run.rs` 的 m8 测试体内），**不冒充完成**这一条守住了；缺的只是承接票号 #419（以及 #422/#423）的引用。

### ⑦ 复现节命令的机械可复现性（臂C 命令的 env 与钩子名实一致）— **通过**

逐个 env 与代码读取点对照：

| 报告 §1 的 env | 代码读取点 | 一致 |
|---|---|---|
| `M8_LEVEL_CAP=true` | `wverify_run.rs::apply_m8_level_cap_from_env` → `std::env::var("M8_LEVEL_CAP")`，`Some("true")` 分支 | ✓ 且**只接受 `"true"`**，报告 §12 条2 已登记 |
| `M8_FEE_DATUM=venue_fee_binance_spot_20260726.json:BTC:VIP0` | `apply_m8_fee_datum_from_env`（三段式 fail-loud，#388 面） | ✓ |
| `M8_WIN_FILTER=<tag>` | `wverify_run.rs:1377` | ✓ |
| `VOICE_EXEC=1` | `admission.rs:32`（`== Some("1")`） | ✓ |
| `THETA_NEST_CERT_GATE=1` | `admission.rs:59`（`== Some("1")`） | ✓ |
| `OPSEM_DUMP_DIR=/tmp/m8_win_armC/<tag>` | `opsem_dump.rs:199` | ✓ |
| 缝名 `theta_v0::backtest::wverify_run::m8_e2e_all_systems_oos -- --ignored` | 函数存在且 `#[ignore]` | ✓ |

产物路径全部实存并与报告登记一致（`/tmp/389_armC_{p3fold,wf7,wf8}.out`、`/tmp/389_armC_report_*.md`、`/tmp/m8_win_armC/<tag>/`、`/tmp/389_fee_decomp.md`、`/tmp/389_full_lib_test_final.log`、`/tmp/389_backup/`）。离线两件我**亲跑复现**：
```
$ python3 scripts/check_armR_trades_digest.py     → 三窗 ✓，exit=0
$ python3 scripts/fee_account_decomposition.py    → 与 /tmp/389_fee_decomp.md 及报告 §3 逐位相同
```
报告 §1 关于跑批基线 HEAD 的声明也核实成立：`git diff --stat d7d3451c64..c78e98ca66 -- rust/ scripts/` **零输出** ⟹「三个后续提交纯文档、跑批读数在新 HEAD 上依然成立、未重跑」的论证正确。

---

## 打回条件（未触发，供参考）

若 MED-1 的三处文本在 map #59 核销前不补，则报告作为「Destination 末项的机械核对件」在承接关系上不自足。修复成本 = 三处文本替换，**零重跑、零代码改动**。

## 一句话小结

数值与证据链是我评审过的这条线里最扎实的一份——七条重点里六条经独立复算逐位吻合，Standards 轴的两条硬违规自曝不粉饰，甚至主动删掉了一条「看上去很硬其实是 L0 同义反复」的运行时断言；唯一的 MED 是报告写成时三张裁定票刚建好还没来得及引进去，属登记滞后而非事实错误。

## #446 上游唯一性修复后的覆盖性订正（2026-07-28）

本复审的第③项“LEE 稀疏性全绿”所核旧数，以及依赖 `strategy_target_legs` 的相关经济读数，
后来证实受同一 `ElementId` 以树前缀 idx 与 registry 追加 idx 双出现的 release 静默双计污染。
因此第③项只能作为“当时报告与当时产物一致”的历史审计，**不再证明当前数值正确**；旧结论不得
增量沿用。

#446 唯一注册修复后的 R/D/C × p3fold/wf7/wf8 九窗已 9/9 exit 0，release 逐窗
`duplicate_id_violations=0`；新 LEE 表、经济表和 D-vs-C 差统一见
`treasury-reverify-20260727.md` §15。帽臂新 `n_cap_narrowed=883/911/809` 且
`off_clock_delta_unexplained=0/0/0`，仍是非平凡绿，但数值必须以该订正节为准。臂 R trades
漂移已依红线完成逐笔取证、golden 重锚和 T1 §14 订正，回归门恢复 exit 0。
