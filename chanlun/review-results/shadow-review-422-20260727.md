# 影子评审报告：#422 措辞订正 + #418 LOW×5（commit `e02df3fbc1`）

- **日期**：2026-07-27
- **被审面**：commit `e02df3fbc1`（固定点 `639773fe01`），6 文件 +157/−34
- **评审工位**：`/tmp/nc-review-422`（detached @ `e02df3fbc1`）。**零 git mutation，仓内文件零改动**
  （评审期间只跑 `cargo check`/`cargo test`，产物落 `rust/target/`，未跟踪、未提交；`git status --porcelain` 收工时仅 `target/` 忽略项）。
- **两轴**：Standards（090 声明纪律 / 认识论分级 / fail-loud 先例 / 注释诚实）+ Spec（#422 票体 + #385 裁定其三 = 方案一）

---

## 结论

# **PASS**（MED×1 + LOW×5）

无 HIGH。**本 commit 未引入任何错误的数字、行号、commit 哈希或归因**——逐条独立复核见 §2，全部对上。
措辞订正的三个核心事实主张（「无良定义只成立于按股档」/「按金额档恒 12bp」/「实现按 `fee_schedule.is_some()` 一刀切」）
经独立读码 + 读 datum + 独立算哈希，**全部成立**。读数零改动经数值 token 全量前后对拍确认。

唯一实质缺口是 **MED-1：同型误用没扫干净**——`rust/src` 里还有三处「把按股档的理由挂到标定档全体」的旧文本
（其中 `runner.rs:139-140` 是最核心的一处，本次已订正的 `metrics.rs` 注释正指向它）。这与 #422 验收标准第一条
「措辞订正全文一致（无对按金额档的残留误用）」不符。因为它们是**未订正的旧文本**而非本 commit 新写的错误陈述，
按「归因错误 = HIGH」的字面判会失准，故判 MED；但**验收第一条不应勾选为已达成**（勾选即 090 声明膨胀）。

---

## 1. Spec 轴：与 #385 裁定其三 / #422 票体的对齐

`gh issue view 385 --comments` 裁定其三（2026-07-27）+ 其后「票号订正（编排者笔误）」评论确认：

| 裁定项 | 挂票 | 本 commit 行为 | 判定 |
|---|---|---|---|
| 措辞订正，**照实，不恢复读数** | #422 | 6 处 T2 + 2 处 T3 订正块，读数一字未动 | ✅ |
| 层4 恢复（按档位形态分叉 scalar 口径） | #423 | 全部订正块明写「恢复属 #423，本次不恢复」 | ✅ |
| #418 LOW×5 随本票处置 | #422 | LOW-1/2/3/4/5 逐条落盘 | ✅ |

> 注：#422 **票体正文**里「读数不恢复（恢复属 **#422**）」是自指笔误，已由 #385 的「票号订正」评论修正为 #423；
> commit message 写的是 #423，**正确**。这是票体的既存问题，不是本 commit 的缺陷。

### 票体验收标准逐条

| # | 验收标准 | 判定 | 证据 |
|---|---|---|---|
| ① | 措辞订正全文一致（无「无良定义」对按金额档的残留误用） | ⚠️ **部分达成** | 报告面 + `wverify_run.rs`/`metrics.rs` 已净；`runner.rs`/`treasury.rs`/`fill.rs` 三处残留 → **MED-1** |
| ② | OKLO 读数文件重生成，抬头标签为真哈希；生成测试同步修 | ✅ | `/tmp/388_oklo_per_share_readings.md:3` = `[L2费率标定: datum cc0d3fa3695d]`；我独立 `shasum -a 256 analysis/data_cache/venue_fee_ibkr_pro_20260726.json` = `cc0d3fa3695dba4e…`，前 12 位一致。文件 mtime `11:52` < commit `12:03`（先重生成后提交，顺序对） |
| ③ | LOW-1/3/4/5 逐条处置落盘；LOW-5 数字与修复前逐位相同 | ✅ | 我把 `639773fe01` 版脚本抽出到 `/tmp/prefix422` 独立复跑，与当前版逐行对拍：**9 列 27 行完全相同**，仅结构性新增 tax 列（恒 0.00）。见 §2.5 |
| ④ | `rust/src` 改动最小面（fill.rs 注释+标签一行）；基线不劣化 | ✅（带偏差登记） | 实际 fill.rs +31 行（含 27 行新断言），**超出票体字面描述的「注释+标签一行」**。但这正是 #418 LOW-2 要的机器防线，方向是加强而非扩权，且断言全在 `#[cfg(test)]` 内、不碰生产路径。登记为偏差说明，不判缺陷 |

---

## 2. 重点核查项：逐条独立复核

### 2.1 订正措辞的事实准确性（本次评审核心）— 三条主张全部成立

**主张 A：「无良定义只成立于按股档（per-share）」**

- `rust/src/theta_v0/venue_fee.rs:158-173` `effective_rate`：`FeeUnit::PerShare(_)` 分支 = `fee_usd(qty,px,side,role)/(qty*px)`，
  而 `fee_usd` 的 per-share 分支（:133-150）含 `.max(min_commission_usd)`（托底）、`.min(notional*max_commission_frac)`（1% 上限）、
  `sell_taf.min(cap)`（TAF 封顶）、`sell_sec` 仅卖出计 ⟹ 有效费率**确是 (qty, px, side) 的非线性函数**，无单标量。✅
- `treasury.rs:58-61` 的 `SCALAR_COST_RATE_UNDEFINED` 消息原文本身已写「在 **per-share 档**无良定义」——
  即**代码里的单一来源串一直是收窄口径的**，膨胀发生在环绕它的注释与报告文案上。订正方向与代码事实一致。✅

**主张 B：「按金额档恒 12bp」**

- datum：`analysis/data_cache/venue_fee_binance_spot_20260726.json` 的 `(BTC, VIP0)` 条目 `unit=notional, maker_bps=10.0, taker_bps=10.0`。✅
- 臂D 实跑档位：T2 报告 §2 跑批命令 `M8_FEE_DATUM=venue_fee_binance_spot_20260726.json:BTC:VIP0` ⟹ 取的就是 10bp 那条（非 VIP0_BNB25 的 7.5bp）。✅
- `FeeUnit::Notional` 分支（`venue_fee.rs:160-170`）返回 `bps/10_000.0`，**函数体内不使用 qty/px**（只做合法性 assert）⟹ 与 (qty,px) 无关。✅
- addon：`FeeQuoter::rate_for`（`venue_fee.rs:207-212`）`Some(s) => s.effective_rate(...) + self.uncalibrated_addon`；
  `treasury::fee_quoter`（`treasury.rs:97-103`）传入 `exec.slippage_bps/10_000.0` = 2bp（`config.rs:303` default）。10+2 = **12bp**。✅
- 撮合恒 Taker：`fill.rs:139` 与 `overlay_state.rs:56` 明写「流动性角色恒 `LiquidityRole::Taker`」；
  全仓 `grep LiquidityRole::Maker` 命中仅在 `venue_fee.rs` 的 match 臂与测试内，**生产路径零 Maker**。✅
- **边界（照实登记，非缺陷）**：`rate_or_fallback`（:216-222）在 `qty<=0 || px<=0` 时回落 `fallback_rate`（3bp），
  故「恒 12bp」严格说是「合法成交上恒 12bp」。该分支的 doc（:214-215、:224-225）已声明这些位点是 noop/拒单、费率不进算术。订正文案未逐字带这个限定，可接受。

**主张 C：「实现按 `fee_schedule.is_some()` 一刀切判 None」**

```rust
// rust/src/theta_v0/backtest/treasury.rs:76-81
pub fn scalar_cost_rate_opt(exec: &ExecConfig) -> Option<f64> {
    match exec.fee_schedule { None => Some(fee_rate(exec)), Some(_) => None }
}
```
只 match `None`/`Some(_)`，**不看 `FeeUnit` 形态** ⟹ 一刀切属实。✅

### 2.2 报告内事实类陈述 — 逐一独立核对，全对

**commit 链 `0dc5bc785d → 9e3cd25fd6 → 78151e988e → d7d3451c64`**（T2 报告 §12）：

```
0dc5bc785d 2026-07-27 07:23:44 test(chanlun): #387 treasury 重验 T1……
9e3cd25fd6 2026-07-27 07:57:10 docs(chanlun): 影子评审报告入库——#387 T1（#401）
78151e988e 2026-07-27 10:11:21 docs(chanlun): #408 订正 #387 T1 报告四条 MED（#401 打回修复）
d7d3451c64 2026-07-27 10:13:21 feat(rust): #388 标定臂（臂D 真实费率）……
```
`git log --ancestry-path 0dc5bc785d..d7d3451c64` 输出恰为后三者 ⟹ **链线性、无遗漏**。✅
「中间两个 commit 只动 `chanlun/`、`rust/src` 零改动」：`9e3cd25fd6` = 1 文件（`shadow-review-387-20260727.md`，+334），
`78151e988e` = 1 文件（`treasury-reverify-t1-armR-20260727.md`，+202/−19）⟹ **属实**。✅

**行号引用**（逐一打开核对，非转述）：

| 引用出处 | 声称 | 实际 | 判定 |
|---|---|---|---|
| `fee_account_decomposition.py:59-60`、表尾 | `config.rs:271` = `fee_schedule=Some` 时 `tax_bps==0` 的约束 | `config.rs:271` 正文：「同时要求 `tax_bps == 0`（`treasury::fee_quoter` fail-loud，禁与 datum 内的监管税费重复计）」 | ✅（见 **LOW-5** 的隔层说明） |
| 同上 | `config.rs:302-304` = 臂R 三常数 | `302: commission_bps: 1.0` / `303: slippage_bps: 2.0` / `304: tax_bps: 0.0` | ✅ 逐行精确 |
| `wverify_run.rs:1871-1873`（新增） | `data.rs` 校验块 299 行、读盘 308 行 | `299: if let Some(sched) = &config.exec.fee_schedule` / `308: let path = data_dir().join(file);` | ✅ 逐行精确 |
| `wverify_run.rs:1876-1877`（新增） | `data::tests::fee_schedule_symbol_must_match_dataset` 在 `data.rs:321` | `321: fn fee_schedule_symbol_must_match_dataset() {` | ✅ 逐行精确 |
| `wverify_run.rs:310-311`（新增） | 链接 `treasury::scalar_cost_rate_opt` | `treasury.rs:76`，`backtest/mod.rs:97 pub mod treasury` ⟹ `super::treasury::…` 路径有效 | ✅（`cargo check` 无 intra-doc 报错） |

**§5 数字与 `/tmp/388_oklo_per_share_readings.md` 对照**：

| 报告陈述 | 产物实测 | 判定 |
|---|---|---|
| 50 股组最低佣金托底 **40/40 全触达** | 表行 `50 \| 40 \| … \| 40/40 \| 0/40 \| 0/40` | ✅ |
| 有效费率从 `7.65e-5` 变到 `1.38e-4`，**1.8 倍** | `1.375701e-4 / 7.654387e-5 = 1.7973` | ✅（四舍五入到 1.8 属实） |
| 100 股与 500 股有效费率**完全相同** `7.654387e-5` | 两行均 `7.654387e-5` | ✅ |
| 卖出 SEC `1.1803 / 2.3605 / 11.8027`、TAF `0.1950 / 0.3900 / 1.9500` | 表内六数逐位相同 | ✅ |

**datum 哈希标签**（独立 `shasum -a 256` 复算，非采信报告）：

- IBKR：`cc0d3fa3695dba4e873325cbaa09615c456de8e41ca6b2e77d77d194c9d8e4a8` ⟹ 前 12 = `cc0d3fa3695d` = 产物抬头标签。✅ sidecar `.sha256` 亦一致。
- Binance：`cdd23adef9b34b7a86496769b60c07c14055e5ff88061b7dcf02f5343c4c2c7c` ⟹ 前 12 = `cdd23adef9b3` = 两份报告全篇使用的臂D 标签。✅

### 2.3 读数零改动 — 全量数值 token 对拍，通过

对两份报告做 `639773fe01` vs `e02df3fbc1` 的数值 token 计数 diff（正则 `[0-9]+\.[0-9]+([eE][-+]?[0-9]+)?|[0-9]{3,}`）：

- T3 总报告：`2.2` +1、`422` +2、`423` +1 — 全部为新增的章节引用与票号。
- T2 报告：`090` +1、`1.8` +1、`231` +1、`2026` +1、`418`/`422`/`423` 若干、`3451`+`78151`+`988` 若干（= 新写入的 commit 短哈希）。

**没有任何一个数值 token 的计数下降**，即无读数被改写或删除。✅ 逐个 hunk 目视复核亦确认：所有改动都是
「原句保留 + 追加『订正（#422）』标注块」或「把原句改成带时点限定的历史陈述」，**没有一处删除原判断**。

### 2.4 全仓「无良定义」残留扫描 —— **发现三处，构成 MED-1**

`grep -rn "无良定义" --include={md,rs,py} .`（排除 `shadow-review-*.md` 证词），逐条判定：

已收窄 / 正确（不列）：`treasury.rs:59`（原文即「per-share 档」）、`treasury.rs:214`（`should_panic` 匹配串）、
`config.rs:286`（「per-share 档下」）、`wverify_run.rs:310/318/1413/1417/1524/1782-1785`（本次订正）、
`metrics.rs:318-320`（本次订正）、两份报告全部命中（订正块或已限定 per-share）。

**残留（未订正，均是「把按股档的理由挂到标定档全体」的同型误用）**：

1. `rust/src/theta_v0/backtest/runner.rs:139-140`
   > `**标定档 \`None\`**——per-share 档的逐笔费率随 (qty, px, side) 变，没有使「随机对照含同等成本」成真的标量`

   这是 `RunResult::fee_rate` 字段本身的 doc，**是全部同型误用里最核心的一处**——本次已订正的 `metrics.rs:321-322`
   注释恰恰用 intra-doc link 指向它（`[runner::RunResult::fee_rate]`）。读者顺着修好的链接过去，读到的还是旧归因。

2. `rust/src/theta_v0/backtest/treasury.rs:191`
   > `★#388 T2：标定档 ⟹ \`None\`（"无良定义"由类型承载，不 panic 也不产近似值）`

   把「无良定义」归给标定档全体（引号略微软化，但归因未变）。

3. `rust/src/theta_v0/backtest/fill.rs:3848`（**就在本 commit 改动点上方约 40 行**）
   > `这是单标量费率口径在**标定档**失效的**经验证据**（#374 有效域收窄的实证面）`

   OKLO 是 per-share 档，其证据只覆盖按股形态 —— 这正是 #418 LOW-1 指出的越界表述，报告面已订正，**代码注释面未订正**。

### 2.5 机器断言充分性（LOW-2）

新增三件套（`fill.rs:4033-4058`）：

| 断言 | 对象 | 防住什么 | 判定 |
|---|---|---|---|
| ① `!label_line.contains('<') && !contains('>')` | **回读的落盘文件** | 正是 LOW-2 的原缺陷形态（字面 `<前12位>`）。format 串若回退成字面占位符，此条必红 | ✅ **真防线** |
| ② `digest12` 全为 `0-9a-f` | `sched.datum_sha256[..12]` | 哈希形态（含拒大写） | ✅ 有效 |
| ③ `label_line.contains("[L2费率标定: datum {digest12}]")` | 回读文件 vs 同一变量 | 写-读一致 + 标签格式串未被改坏 | ⚠️ 与生成源同变量，**对哈希本身的正确性是重言的**（见 **LOW-3**） |

- **断言对象确为落盘交付物本身**：`std::fs::read_to_string(OKLO_READINGS_PATH)`，不是内存 `md`。✅ 这是关键的正确设计。
- 「与本测 footer 同源」的声明属实：footer（`fill.rs:4018`）与抬头（:3899）都用 `&sched.datum_sha256[..12]`。✅
- fail-loud 纪律未削弱：回读失败 `panic!`、落盘失败 `panic!`，无 `.ok()` 吞错。✅

### 2.6 LOW-5 语义与独立复跑

**语义**：`tax` 提为 `Decomposition` 的一等字段并进 `total`，`decompose()` 收 `tax_bps` 参数，
臂D/C 传 `ARM_CALIBRATED_TAX_BPS=0.0`、臂R 传 `ARM_R_TAX_BPS=0.0`。表尾明写「若上游改变该约束或该常量非零，
本表逐笔重算会自动纳入（一等科目，非特判）」——**与实现一致，无声明膨胀**。✅
约束依据属实：`treasury.rs:91-96` 的 `assert!(exec.fee_schedule.is_none() || exec.tax_bps == 0.0, …)`。✅

**独立复跑**（`python3 scripts/fee_account_decomposition.py --armD-dir /tmp/m8_win_armD --armC-dir /tmp/m8_win_armC --armR-dir /tmp/m8_win_gate`，**exit=0**）：

- tax 列 9 行**全 0.00** ✅
- 勾稽自算（抽 p3fold/D）：`615050128.62 × 10/1e4 = 615050.1286` + `615050128.62 × 2/1e4 = 123010.0257` = `738060.154` → 表载 `738060.15` ✅；
  p3fold/R：`68041.537 + 136083.074 = 204124.61` ✅
- 与 T1/T2/T3 报告引用数字对照：`treasury-reverify-t2-armD-20260727.md:155-159` 与 `treasury-reverify-20260727.md:212-218`
  的 9 列全部逐位相同（`615050128.62 / 615050.13 / 123010.03 / 738060.15`、`1399495.10 / 1679394.12`、`1883048.84 / 2259658.61`、`68041.54 / 204124.61` …）**无打架** ✅
- **修复前 vs 修复后**：我从 `639773fe01` 抽出旧脚本到隔离目录独立复跑，两版输出的 9 个共有列 **27 行逐位相同**，
  差异只有新增的 tax 列与表头 ⟹ 「数字与修复前逐位相同」**独立证实** ✅

### 2.7 回归门

```
$ python3 scripts/check_armR_trades_digest.py
p3fold: n_trades=149 digest=0x6bf47daf0aa737cd bytes=206057 ✓
wf7:    n_trades=196 digest=0x282c28ca8ca65e16 bytes=270684 ✓
wf8:    n_trades=168 digest=0xcafa7c4846c762cd bytes=228108 ✓
臂R trades 逐位无漂移。
EXIT=0
```
✅ **exit=0**，与 commit message 的验收声明一致。

### 2.8 Standards 轴顺带面

| 面 | 判定 | 证据 |
|---|---|---|
| **订正 vs 改写历史**（090） | ✅ **合格** | 全部 8 处订正都是**叠加标注**：原句保留、显式写「原句写作『…』」、订正块带票号。T2 §12 甚至把「其后无 commit」改成「**自审执行当时**其后无 commit」并附实际链——原判断可溯、时点明确。**无一处是删原句改写** |
| **时态订正（LOW-3）** | ✅ | §「交付为未提交改动」→「**截至本报告落盘时**…为尚未提交的工作区改动」+ 追修块；§9 标题、§12 固定点句同口径。历史陈述与当前状态被明确分离 |
| **认识论分级** | ✅ | 产物抬头 `**认识论 L1**` 保留；新增测试注释未擅自升级等级；`layer4_cells_calibrated_marks_unavailable` 的 `**认识论 L0**（契约）` 标注保留 |
| **fail-loud 先例** | ✅ | 无一处 `.ok()`/`unwrap_or_default` 吞错；新断言全走 `panic!`/`assert!`；`scalar_cost_rate` 的 panic 路径与 `SCALAR_COST_RATE_UNDEFINED` 单一来源未动 |
| **注释诚实** | ⚠️ | 主体诚实（LOW-4「磁盘无关」→「不依赖市场数据文件」的收窄是**照实修正失实声明**的范本）；一处小膨胀见 **LOW-2** |
| **注释密度/风格** | ✅ | 与本仓既有风格一致（票号锚点 + 「订正（#422）」标记 + 有效域块），无冗余 |
| **TDD 顺序（红先于绿）** | ⚠️ **无法证实** | 单 commit 交付，git 历史里不存在「断言先红」的中间态。逻辑上断言①对原缺陷形态必红（原 `String::from` 里写死 `<前12位>`），故防护有效性成立；但**先红后绿的过程未留证据**，照实登记为未验证 |

### 2.9 我独立跑的构建/测试

| 命令 | 结果 |
|---|---|
| `cargo check --lib --tests --message-format short` | **exit=0**，仅既有 warning（53 条，全部为 dead_code/naming，与本 commit 无关）⟹ 新增断言与 doc 链接**编译通过** |
| `cargo test --lib -- fee_datum_cross_symbol_borrow_is_blocked layer4_cells_calibrated_marks_unavailable scalar_cost_rate` | **6 passed / 0 failed** |
| `cargo test --lib venue_fee` | **28 passed / 0 failed / 2 ignored** |
| `cargo test --lib oklo_real_window_per_share_readings -- --ignored` | ❌ **未跑通** —— 评审工位缺 `analysis/data_cache/oklo_1m_databento.json`（21 MB，未入仓；主仓 `/Users/silencehan/Projects/NewChanlun` 有该文件）。**不copy进工位**（会改仓内文件面，违反本工位约束）。改以「读落盘产物 + 独立 `shasum` 复算哈希」替代验证，见 §2.2 |

---

## 3. 分级清单

### MED

**MED-1｜同型误用未扫尽：`rust/src` 三处残留，验收①不应勾为已达成**

- **位置**：`rust/src/theta_v0/backtest/runner.rs:139-140`、`treasury.rs:191`、`fill.rs:3848`（原文见 §2.4）
- **为什么是 MED 不是 HIGH**：本 commit 没有**新写**任何错误归因，三处均是 #388 遗留的旧文本；
  commit message 只声称订正了 `wverify_run.rs` 的三处（1413/1524/1782 — 属实）。
  但 #422 票体验收①明写「措辞订正**全文一致**（无残留误用）」，该条**未达成**。
- **为什么值得修**：`runner.rs:139` 是 `RunResult::fee_rate` 的字段 doc，本次**已订正**的 `metrics.rs:321-322`
  正用 intra-doc link 指向它 ⟹ 修好的注释把读者送到没修的注释上，归因矛盾在同一条阅读路径内可见。
  `fill.rs:3848` 更是在本 commit 改动点上方 40 行，属"改到跟前没改"。
- **建议**：三处一并按同口径追修（可并入 #423 的 scalar 口径分叉票，或另起一条 LOW 尾扫）。
  在此之前，**#422 的验收①不要勾选**——勾了就是 090 意义上的声明膨胀。

### LOW

**LOW-1｜T2 §3 订正块把三个机制并列后紧跟「§5 实测 1.8 倍」，但其中两个在 §5 里触达为 0**

- **位置**：`chanlun/review-results/treasury-reverify-t2-armD-20260727.md:114-115`
  > 「（IBKR 那类：**最低佣金托底 / 1% 上限 / TAF 封顶**使有效费率随单量变动，**§5 实测 1.8 倍**）」
- **事实**：§5 表三行的 `1%上限触达` 与 `TAF上限触达` **全部是 0/40**；1.8 倍**全部由最低佣金托底造成**。
- **影响**：读者可能把 1.8 倍读作三个机制的联合实测结果。定性结论（per-share 无良定义）不受影响——托底一个拐点就足以否掉常数性。
- **建议措辞**：「…三个拐点使有效费率可随单量变动（§5 真实窗上**仅托底一项触达**，即已实测 1.8 倍）」。

**LOW-2｜`fill.rs:4034-4036` 注释声称的正则消费方在仓内不存在**

- **原文**：「消费方以 `\[L2费率标定: datum ([0-9a-f]{12})\]` 提取档位与 datum 身份」
- **复核**：`grep -rn "L2费率标定"` 全仓 19 处命中，全部是**生产端**（`fill.rs`/`wverify_run.rs`/`venue_fee.rs` 写标签）
  或报告文本，**没有任何一处解析该标签**。#385 US4 的原文是「so that 读数的认识论等级**机器可读**」——
  即"可被机器读"是**契约目标**，消费端尚未落地。
- **性质**：注释用陈述句描述了一个尚不存在的消费方，属轻度声明膨胀（090）。断言本身仍然正确且有价值。
- **建议**：改为「本标签的契约目的是让消费方能以 `…` 提取（#385 US4；消费端尚未实装，本断言先把生产端锁死）」。

**LOW-3｜三件套第三条断言相对哈希正确性是重言的**

- `expected_label` 由 `&sched.datum_sha256[..12]` 构造，而落盘文件的抬头也由**同一个表达式**插值 ⟹
  该断言只能证明「写进去的和读出来的一样」+「格式串没被改坏」，**不能**证明标签里的哈希确是 datum 文件的真哈希。
- 真正防住 LOW-2 回归的是断言①（`<`/`>` 占位符检查）。真正证实哈希正确性的是 `load_datum` 的 sidecar 校验
  （`venue_fee.rs` 的 `tampered_sha256_sidecar_is_rejected` 等测试，我已复跑通过）——**防线整体是完整的**，
  只是注释里「三件套」的表述让第三条看起来比实际承担得多。
- **建议**：注释里点明第三条的作用域 = 「写-读一致 + 格式串完整性；哈希真实性由 `load_datum` sidecar 校验承保」。

**LOW-4｜报告内分解表（9 列）与脚本现输出（10 列）形状不再一致**

- 两份报告的 §「费率科目分解」表仍是修复前的 9 列，脚本现输出 10 列（含 tax）。
- **不判缺陷**：裁定方案一明令「读数一字未动」，不重生成报告是**遵守裁定**，且旧表数字仍逐位正确。
- 仅作登记：将来若有人拿脚本输出与报告对拍，会看到列数不同。可在 #423 落地重跑报告时自然消解。

**LOW-5｜`config.rs:271` 被引作 fail-loud 约束点，实际 `assert!` 在 `treasury.rs:91`**

- `config.rs:271` 是**记载**该约束的 doc 行，其自身点名了 `treasury::fee_quoter` 作为 fail-loud 位点，
  故引用可溯、不算错误，只是隔了一层。
- **建议**：脚本注释与表尾把两处一起给：「`config.rs:271` 声明 / `treasury.rs:91` 施加」。

---

## 4. 未覆盖声明（照实）

1. **`oklo_real_window_per_share_readings` 未在本工位复跑**——评审工位缺 `analysis/data_cache/oklo_1m_databento.json`
   （21 MB，未入仓；主仓有）。把它复制进工位会改动仓内文件面，违反本工位「禁改仓内任何文件」约束，故不做。
   代偿：读落盘产物 `/tmp/388_oklo_per_share_readings.md` + 独立 `shasum` 复算 IBKR datum 哈希，
   两者一致（`cc0d3fa3695d`）⟹ **LOW-2 的验收结果已独立证实**；断言逻辑经读码 + `cargo check --tests` 编译验证。
   **未验证的是「该测试在真实数据上今天仍能跑绿」**。
2. **`cargo test --lib` 全量未跑**——commit message 声称 `1941/4/135` 且失败集 = 动手前基线（`lee_m3`/`lee_m4×2` = #446；
   `extract_signals_bit_exact_digest_guard` = #115 线）。我只跑了与本 commit 直接相关的 6 + 28 个测试（全绿）
   和 `cargo check --lib --tests`（exit=0）。**「1941/4/135」这组计数与失败集我未独立复现**。
   （#446 已在案，按任务说明不重复挂票。）
3. **`m8_e2e_all_systems_oos` 未复跑**——`wverify_run.rs:1410-1420` 改的是 m8 报告的产物文案，
   该文案只在 `#[ignore]` 的全量 m8 跑批中落盘。我**未跑 m8**，故「新文案实际渲染出来长什么样」未验证。
   影响面仅限于将来重跑 m8 时的报告文本，不影响任何在案读数。
4. **rustdoc intra-doc link 未做 `cargo doc` 专项检查**——`cargo check --lib --tests` 通过且无 doc 相关 warning，
   但破损的 intra-doc link 只在 `cargo doc` 下报 warning。新增的 `[treasury::scalar_cost_rate_opt](super::treasury::scalar_cost_rate_opt)`
   我按模块路径手工核实有效（`backtest/mod.rs:97 pub mod treasury`），未跑 `cargo doc` 机器确认。
5. **TDD 先红后绿未证实**——见 §2.8，单 commit 交付，无中间态可查。
6. **#446 / #115 两条在案红测未查证**——按任务说明属本评审面之外。
7. **T3 总报告除 2 处订正外的其余内容未重审**——本次评审面是 `e02df3fbc1` 的 diff；
   T3 报告整体已由 #431/#432 评审链覆盖。

---

## 5. 结果包六要素

1. **结论**：**PASS**（MED×1 + LOW×5）。#422 的核心任务（措辞订正、不恢复读数、LOW×5 处置）在**报告面完成、代码面差三处尾扫**。
2. **定义依据**：#385 裁定其三（+ 票号订正评论）定义方案一 = 只订正措辞不恢复读数；#422 票体四条验收标准；
   090（声明与实际一致，禁声明膨胀）；231号 / `formalization-validity-domain.md`（有效域 ≠ 定义域，
   OKLO 的 per-share 证据不得声明为覆盖标定档全体）；#374（fail-loud 防线不得削弱）。
3. **边界条件**（结论在什么条件下翻转）：
   - 若 §2.4 的三处残留被认定为「本 commit 应当覆盖而未覆盖」而非「遗留旧文本」⟹ MED-1 升 HIGH ⟹ **打回**。
     这取决于验收①「全文一致」的解释宽窄，是价值判断，我按「未引入新错误」判 MED 并把判据摆明。
   - 若 `cargo test --lib` 全量复跑的失败集**大于**动手前基线（即出现 `lee_m3`/`lee_m4×2`/`extract_signals…` 之外的红）
     ⟹ 验收④不成立 ⟹ 打回。我未复跑（未覆盖声明 2）。
   - 若 `oklo_real_window_per_share_readings` 在真实数据上跑不绿 ⟹ LOW-2 的处置不成立 ⟹ 打回。我未复跑（未覆盖声明 1）。
4. **下游推论**：
   - **#423（层4 恢复）的前提已被本次订正锁死**：「按金额档标量可定义 = 10bp datum + 2bp addon = 12bp」
     这条我已独立证实，#423 可直接以此为设计输入，不必重推。
   - 引用「不可用(标定档有效域收窄 #374)」的下游一律须按新口径读：**按股档 = 无良定义；按金额档 = 保守收窄**。
     在 §2.4 三处残留修掉之前，读 `rust/src` 注释的下游仍会拿到旧归因。
   - 分解脚本的 tax 现为一等科目：将来任何 datum 带非零税科目、或 `treasury.rs:91` 的约束松动，分解表会自动纳入而非静默少算。
5. **谱系引用**：090（严格性/声明膨胀）、231号 + `formalization-validity-domain.md`（有效域 ≠ 定义域——
   本票订正的正是「per-share 证据被声明为覆盖标定档全体」这一有效域膨胀，与 222/223/230 三例同型）、
   #374（fail-loud 防线）、161号（不务实、不把矛盾留到后面 —— MED-1 的三处残留正属"留到后面"）。
6. **影响声明**：本报告只读评审，**未改动任何仓内文件、未做任何 git 操作**；产出仅
   `/tmp/shadow-review-422-20260727.md` 一份，以及评审期间 `cargo` 在 `rust/target/`（gitignore 内）生成的构建产物
   与隔离目录 `/tmp/prefix422`（复跑修复前脚本用）。
