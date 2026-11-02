# #1654 非作者独审（claude-fable-5-1，只读）

- 审查对象：PR #1665，分支 `sandcastle/issue-1654`；第 1 轮覆盖 `109539125d`（实装）+ `0183e92004`（评审工蜂），第 2 轮快速轴覆盖 harvest owner 按第 1 轮意见追加的 `c86480f31f`（只改注释）。
- 路由：Claude Code CLI 经 CLIProxyLocal（本机代理端口 8318），模型 `claude-fable-5-1`；实装与评审工蜂均为 `claude-opus-5-5`，本审查非作者。
- 工具限制：Read/Grep/Glob 与只读 git；未跑 cargo。第 1 轮提示原文：`prompt.md`。日期：2026-10-21。

---

## 第 1 轮

**结论：PASS**

## 总评

PR #1665 的 62 行改动全部落在注释、doc 注释和断言消息字符串上，没有任何可执行代码、判据、常量或默认值变化；默认 `Segment` 路径逐位不变是由构造保证的（代码未动）。「Stroke 不进准入/生产判定路径」的证据链在仓内成立：唯一能传入 `"stroke"/"bi"` 的入口是两个 pyclass 的 `a0` 参数和 `#[ignore]` 测试的 `T_A0` 环境变量，现存 3 个 Python 调用者都不传 `a0`，且 ADR 0039 已把这两个 pyclass 归为「冻结影子」。新注释引用的出处均存在且转述基本如实；验收 rg 0 命中；C3 八处齐全。下面的 MEDIUM 是对「保留理由」的证据强度提醒，不阻塞合并。

## HIGH

无。

## MEDIUM

1. **`rust/src/recursive_t/types.rs:75-76`、`ffi.rs:42`、`stream.rs:233-234` — 把「#951 B3 冻结缓存」列为保留 Stroke 档的理由，证据偏弱。** 冻结链 `trading_system/backtest_t_fugue.py:167` 调用 `nr.TFugueStream(self.config.mode)` 从未传 `a0`（`git log -S'a0='` 该文件历史上也无），24 个 `t_fugue_*.json` 缓存里没有 `a0` 字段——即这些缓存在 Segment 默认下生成，复现它们不需要 Stroke 档。#951 B3 原文（`chanlun/review-results/issue951-impl-20260814.md:107`）确实写了「mode/a0 两实验自变量，24 缓存不可复现」，所以这不是编造，但把它当作 Stroke 的保留依据属于夸大。真正依赖 Stroke 的复现只有 526 号 E1 A/B（归档 `e1-a0stroke-result-20260623.md:115`：经 `T_A0=stroke` 环境变量跑 Rust 回测测试，现为 `rec_stream.rs:1577` 的 `rec_btc`）。建议把保留理由收敛为「仅供 `rec_btc` 复现 E1 A/B」，B3 只作冻结链不动的旁证。

## LOW

1. **`rust/src/recursive_t/ffi.rs:294-298`（PR 未动的既有文本）与新增的 `ffi.rs:43-45` 在同一文件内互相矛盾。** 旧 GUARD-ROLE 块说 `RecTStream` 有「4 处真实 python 调用者，含 NT 生产策略 `trading_system/strategy/rec_t_strategy.py:45`」；该文件已在 #951 被 `git rm`（继任 `theta_rec_strategy.py`，不用 RecTStream），现存只有 `capture_ratio_matrix.py:85`、`t3_exit_trigger_diag.py:36` 两处 RecTStream 调用 + `backtest_t_fugue.py:167` 一处 TFugueStream，与新注释登记的 3 处一致。既有块陈旧不是本 PR 引入的，但本 PR 既然在同文件落「准入路径证据」，顺手改一行或加指针会更干净。
2. **`rust/src/recursive_t/types.rs:77` 以 ADR 0005 为「不得进入准入／生产判定路径」的依据，口径略松。** ADR 0005 本体是 nest 管线的条款 9 批复，其 2026-08-18 修订节（`docs/adr/0005:36-39`）明写该 ADR 所指对照臂已退役、#799 裁定六例外面「作为历史记录保留」；`qujiantao.md:1564` 称 ADR 0005 是「该例外面的退役记录」。现行落点是 `AGENTS.md:56` / `docs/unified-architecture-canon.md:81`（「可换装、不参与生产判定路径（#799 裁定六『不适用』类）」）。PR 措辞与 `AGENTS.md:56` 一字不差，故不算编造，只是引 canon:81 更稳。
3. **`rust/src/recursive_t/stream.rs:568`、`:657`、`rec_stream.rs:1270`** 测试函数名仍保留「底座下移」「笔底座」旧名分（`a0_stroke多于segment_底座下移`、`流式笔底座零panic`、`递归流式笔底座零panic`）。改函数名属代码变更，票面只要求改注释，不改是合理的，但下次触碰这些测试时值得顺手更名。
4. 范围之外的残留：`rust/src/spiral/state.rs:55` 「0=bar(a0)/1=笔/2=线段」是 spiral 自己的径向口径（a0=bar），与 #1627 的 a₀ 值域不是同一概念，不在 C2/C3 清单内，不算漏改，仅记录。

## 逐项核对

**1. 只改注释/断言消息** — 对 `-U0` diff 的全部 `+/-` 行做过滤：除 `//` 注释行和纯字符串字面量行外，仅剩 4 行，分别是 `stream.rs:548/554` 两条 `assert_eq!` 的消息字符串、`nested_fugue.rs:149` 与 `positional_fusion.rs:1341` 两行的行尾注释，代码部分逐字相同。改动的格式字符串 `{}` 占位数与实参数一致（两处各 2 个）。

**2. 不进准入/生产路径** — `A0Source::Stroke` 在非测试代码只出现在 `stream.rs:157/351`、`rec_stream.rs:229` 三个 match 臂和 `ffi.rs:49` `parse_a0`；`new_with_a0`/`new_production` 的非测试调用者只有 `ffi.rs:238/316`；`T_A0` 只在 `#[ignore]` 的 `rec_btc` 里读。Python 侧全仓 `TFugueStream|RecTStream` 构造调用 3 处，全部只传 mode，`a0=`、`"stroke"`、`"bi"` 在 `trading_system/`、`analysis/`、`scripts/` 0 命中。ADR 0039:222 把 `PyTFugueStream`/`PyRecStream` 归「冻结影子」。评审工蜂的登记属实，三处行号核对无误。

**3. 引文三查** — `zhongshu.md:160-174` §2.1.1 存在，值域 `segment/kline3/trade3`、第 2 条「缠论笔不在值域内」及 `083:14/:16`、`064:34`、`roadmap.yaml:970-972` 均原文如此；`roadmap.yaml:962-975` 条目 `status: falsified`，970-972 行即 E1 A/B 三路证据；`#951 B3` 见 MEDIUM 1；`#808` 在 `issue815-m1-...md:247` 核为「退场条款票（已 CLOSED）」；`#799 裁定六` 见 LOW 2；`bi.md:151`、`065:182` 为既有引用未复核。

**4. 验收** — `rg -n "笔 ?= ?a0|a₀ = 笔序列|a₀（笔序列）" rust/src` 0 命中；扩展检索 `a0 ?= ?笔|递归底座下移|a₀ 笔` 亦 0 命中。C3 八处（`positional_fusion.rs:32/1341`、`unified_osc.rs:229`、`nested_fugue.rs:149/915`、`positional.rs:340`、`config.rs:636`、`level_sub_lou.rs:547`）与 `zhongshu.md:632` 清单逐一对应，`FIRST_BSP_LADDER` 与判据未动。

**5. 编号与范围** — 两个提交均带 #1654，分支 `harvest/issue-1654`；12 个文件全部在 `zhongshu.md:631-632` C2/C3 列出的文件集内，无越界。

## 看过什么 / 未核实什么

看过：完整 `git diff 63ecebf8f8...HEAD`；`ffi.rs:200-330`；`zhongshu.md:158-176、624-637`；`roadmap.yaml:960-980`；ADR 0005 全文；`issue951-impl-20260814.md:70-112`；`AGENTS.md:56`、`canon.md:81`、`qujiantao.md:1564`；归档 E1 结果（`git show 6aeac38754`）；全仓 Python/Rust 对 A0Source、parse_a0、T_A0、TFugueStream、RecTStream、new_production 的检索；24 个 `t_fugue_*.json` 的 `a0` 字段检索。

未核实：GitHub issue #1654 票内是否已按票面要求写下「保留 vs 删除」裁定及证据（无 gh 权限，仓内只有 `types.rs:77` 一句「随 recursive_t 退场一并删除（#808）」）；未跑 `cargo build/test`（按约束不跑；改动为纯注释与字符串，编译风险极低但非实测）；`bi.md:151`、第 65 课 `065:182`、第 83/64 课原文行号为既有引用，未逐字复核。


---

## 第 2 轮（快速轴，`0183e92004..c86480f31f`）

**结论：PASS**

## 处置核对（M1 / L1 / L2）

| 第 1 轮意见 | 处置 | 核对 |
|---|---|---|
| M1 把 #951 B3 冻结缓存当 Stroke 保留依据属夸大 | 已采纳 | `types.rs:74-78`、`ffi.rs:42`、`stream.rs:233-234`、`types.rs:86` 四处保留理由统一收敛为「仅供 `rec_btc`（`T_A0=stroke`）复现 526 号 E1 A/B」，B3 改为旁证（冻结链从未传 `a0`、缓存不依赖本档），与我建议的措辞一致 |
| L1 ffi.rs GUARD-ROLE 块陈旧 | 已采纳 | `ffi.rs:296` 「有 4 处」改「当时有 4 处」，并追加现状两行（`rec_t_strategy.py` 已在 #951 移除，现存 2 处且不传 `a0`）；旧块的历史记载保留未删，口径自洽 |
| L2 ADR 0005 口径宜改引 canon:81 | 已采纳 | `types.rs:78` 改为 `docs/unified-architecture-canon.md:81` |

## 新注释事实核查（均属实）

- **只改注释**：diff 为 3 文件 +12/−8，过滤 `///` 前缀后剩 0 行，无代码、字符串或属性变化。
- **`rec_btc` 是 `#[ignore]` 回测、经 `T_A0=stroke` 切笔**：`rec_stream.rs:1557-1559` 为 `#[ignore = "递归引擎全量回测…"] fn rec_btc()`，`:1577` 读 `T_A0` 环境变量。
- **「E1 A/B」命名**：`.chanlun/roadmap.yaml:972` 原文「E1 受控 A/B（唯一变量=a0 来源）」，该条 `status: falsified`；归档 `e1-a0stroke-result-20260623.md` 存在于 `6aeac38754`。
- **`backtest_t_fugue.py` 从未传 `a0`**：`:167` 为 `nr.TFugueStream(self.config.mode)`；`git log -S'a0='` 对该文件 0 命中。
- **24 个 `t_fugue_*.json` 不含 `a0` 字段**：全仓（排除 target）恰 24 个，`"a0"` 与 `stroke` 均 0 命中。
- **`rec_t_strategy.py` 已在 #951 移除**：文件不存在，删除提交为 `ea721c08a0 feat(#951)…`。
- **现存调用者行号**：`analysis/capture_ratio_matrix.py:85` `nr.RecTStream(mode)`、`analysis/t3_exit_trigger_diag.py:36` `nr.RecTStream(pm)`，全仓 Python 对 `TFugueStream(|RecTStream(` 的构造仅这 2 处 + `backtest_t_fugue.py:167`，均只传 mode。
- **canon:81**：原文「可换装、不参与生产判定路径（#799 裁定六『不适用』类）：…诊断对照臂…」，引用准确。
- **`lib.rs` pymodule 导出**：`lib.rs:2833/2836` 仍导出 `PyTFugueStream`、`PyRecStream`；旧块指向的 `rec_engine.rs` 头部 GUARD-ROLE 块仍存在。

## 新问题

无。两处可忽略的旁注：`rec_stream.rs:1556` 的 `rec_btc` 自身 doc 注释仍写「仅复现 526 号 A/B」（未带「E1」字样），与本次收敛口径不冲突，不必改；第 1 轮 LOW 3/4（测试函数旧名分、spiral 径向口径）本就不要求本 PR 处理，本轮未动，维持原判。

## 看过的内容

`git diff 0183e92004 c86480f31f` 全文；`rec_stream.rs:1503-1580`；`.chanlun/roadmap.yaml:960-982`；`docs/unified-architecture-canon.md:79-83`；三个 Python 调用者上下文；`lib.rs:2829-2836`；24 个 `t_fugue_*.json` 的字段检索；`rec_t_strategy.py` 删除历史。未跑 cargo，未调 gh（按约束）。

---

## 尾部纯格式提交登记（harvest owner）

独审第 2 轮覆盖到 `c86480f31f`。之后 CI `rust-check` 的 `cargo fmt --all -- --check` 在 `rust/src/recursive_t/stream.rs:545` 报格式漂移（来自实装提交改长的断言消息），追加提交逐字照 rustfmt 给出的 Diff 把该 `assert_eq!` 改为多行，实参与消息字面量不变。按 `docs/agents/delivery-discipline.md` 第 7 子句在此登记。

### 尾部快速轴复核（claude-fable-5-1，`b5403f01d3..2c60a45fb5`）

结论：PASS

我看了 `git diff b5403f01d3 2c60a45fb5` 全文，共两个文件、11 行新增、1 行删除。代码侧唯一改动是 `rust/src/recursive_t/stream.rs:548` 的 `assert_eq!` 从单行拆成多行，三个实参（`a0.len()`、`n_confirmed`、消息字面量 `"Stroke 对照臂单元数 = confirmed 笔数"`）逐字不变，是标准 rustfmt 的换行形态，没有任何语义变化。文档侧在 `review-claude-fable-5-1.md` 末尾追加的"尾部纯格式提交登记"段落所述（行号 545、起因为 CI `cargo fmt --check`、实参与消息不变）与实际 diff 一致。

补充说明：我尝试本地跑 `rustfmt --check` 做二次确认，该命令需要审批、未获执行，故"逐字照 rustfmt 输出"这一点是依据 diff 形态判断而非工具实跑，不影响结论。
