# 影子评审 #326 — #303 CostModel 口径切 spot（commit `68e65bd42a`）

- 评审位：独立新上下文（claude Opus lineage，非 #303 实装位），只读。
- 复跑环境：worktree `/tmp/kimi-nest-mainline`，2026-07-26。
- **总判：PASS（无 HIGH，不回票）**；MED×2、LOW×2 登记待裁。

## Spec 轴

| # | 核对点 | 判 | 证据 |
|---|---|---|---|
| S1 | 切 spot 语义落唯一权威处 | PASS | `risk.rs:812-830` 节头三通道现货语义 + venue 适用性 + 重叠上界，齐 |
| S2 | 「按现参数」不动数值 | PASS | `wverify_run.rs:1078` `CostModel::new(0.0001, 480, 0.000001, 0.005)` 一字未动 |
| S3 | 数值零变化机械核验 | PASS | `git diff -U0` rust 侧非注释改动**仅 3 处**，全为落盘 md 报告字符串字面量（`runner.rs:4596`、`wverify_run.rs:1122-1125`、`:1247-1248`）；`risk.rs` 非注释 diff = **0 行**；Python 侧非注释改动仅 `:57` 行尾注释，`MAKER_SIDE=0.0002` 未动 |
| S4 | C1 双计「上界」诚实性 | PASS | `fill.rs:714` funding 基数 = `net_notional_usd`（|N| 全额）vs `risk.rs:932` borrow 基数 = `max(0,\|N\|−E)` ⟹ 借入部分确被双计；方向为**高估**，"上界非精确分科"属实，无美化 |
| S5 | FundingScheduleBook 冻结签名 | PASS | `risk.rs:723/747` `new`/`as_of` 逐字未动；全仓无生产注入点（仅定义 + `:1586` 测试），"不注入"断言成立 |
| S6 | fugue_v3 / dual_ledger 分叉正确 | PASS | `fugue_v3/mod.rs:58`、`dual_ledger.rs:20` 各自已显式区分 perp/spot，无需改 |
| S7 | `trading/ledger.rs:647` 中文常义误命中 | PASS | "单相永续" = 持续义，非 venue 科目 |
| S8 | `complete/event.rs:56` 「唯二构造点」断言 | PASS | 全仓 grep 仅 `event.rs:136`（`price_only`, amount 0）+ `complete/mod.rs:179`（测试, −1） |
| S9 | 数据源出处 | PASS | `scripts/download_btc_binance.py:38-40` BASE_URL = `data.binance.vision/data/**spot**/monthly/klines/BTCUSDT/1m` |
| S10 | 年化换算 | PASS | 1bp×3/日=0.03%/日→10.95%/年；1e-6×1440=0.144%/日→52.56%/年；均标注"≈非 venue 原文" |
| S11 | Python maker 范围判定 | PASS | 有效域悬置替代改数值，与「本票只做声明面」自洽（但见 M2） |

## Standards 轴

| # | 核对点 | 判 | 证据 |
|---|---|---|---|
| T1 | 唯一权威 + 短指向单源性 | **PARTIAL** | `config.rs:313`/`fill.rs:633`/`overlay_state.rs:42,643`/`runner.rs:152` 均为纯短指向 ✅；`wverify_run.rs:1070-1085` 复制三通道现货语义与上界结论（见 M3） |
| T2 | venue 措辞与实装一致（090） | PASS | 含"现货借币利率一手数字未取到""不声称对齐任何真实档位"照实声明，对齐 #285 §2.1/§4 |
| T3 | L2 等级标注 | PASS | `risk.rs:832-838`、`config.rs:313-315`、`wverify_run.rs:1086-1089` 三处 L1 机制 / L2 费率缺口口径一致 |
| T4 | `cargo doc --no-deps` | PASS | exit 0，**0 处** unresolved intra-doc link；8 改动文件零 warning |
| T5 | `cargo test --release --lib` | PASS（附限制） | **1862 passed / 1 failed**，唯一失败 `classifier::signal::tests::extract_signals_bit_exact_digest_guard`（在案 #110）。**限制**：工作区已被并行 #309 session 推进（`fill.rs`+126/`runner.rs`+260 行，测试数 1843→1862），故非对 `68e65bd42a` 树的严格复跑；`signal.rs` 本身未被改动 |

## 问题分级

- **MED-1｜venue 裁定的对称推论遗漏**：切 spot 后，`ExecConfig` 默认 3bp/side（`config.rs:221-235`）与 #285 §2.1 一手数字（Binance 现货 VIP0 taker **10bp/side**）差 3.3 倍。Resolution「登记不改」清单**未列此项**，而同性质的 Python maker 落差却做了登记 + 有效域悬置——同一裁定下同类落差处理不对称。建议：至少补一条登记（不改值），或明示划归 #285 标定票。
- **MED-2｜悬置未触达落盘产物**：`btc_2week_1s_backtest.py:134/140/153` 仍计算并写出 `friction.maker_side` / `maker_face` / stdout 行，JSON 与终端**无「有效域悬置」运行时标注**——只读 docstring 的人知道，读产物的下游看不到。建议在 JSON 该字段旁落 `"validity": "suspended(#303)"`（纯标注，不动数值）。
- **LOW-1｜牵连面漏一条**：`dual_ledger.rs:20` 声明"现货标的 `q_short` 读法须部署层 gate=0"——#303 裁定 spot 后该 gate 的触发前提已成立。commit 判其"本已分叉正确"无误，但该下游推论未进牵连面登记。
- **LOW-2｜单源性轻度渗漏**：`wverify_run.rs:1070-1085` 在短指向之外复制了三通道现货语义与"读作持有成本上界"结论。其中年化换算/数据源出处属该处独有信息（合理），语义与上界结论宜收敛为纯指向。

## 结果包

- **结论**：#303 裁定忠实、数值零变化机械可证、C1 登记诚实、清单抽查全对。判 PASS，不 reopen。
- **定义依据**：#303 票体 + Resolution；#285 报告 §1.2/§1.6/§2.1/§4；090（照实措辞）；231 号（有效域等级）。
- **边界条件**：本判会翻转，若——(a) `CostModel::new` 四参数中任一被改；(b) `risk.rs` 出现非注释 diff；(c) T5 在干净 `68e65bd42a` 树上出现第二个失败。
- **下游推论**：MED-1/MED-2 是**声明面**缺口而非数值缺口，可由 #285 标定票一并收；LOW-1 应进 #62/部署层 gate 清单。
- **谱系引用**：090（严格性/声明膨胀禁止）、231（有效域 ≠ 定义域）、A10 C2/F2（承接位冻结签名）。
- **影响声明**：本次评审只读，唯一写入 = 本文件。
