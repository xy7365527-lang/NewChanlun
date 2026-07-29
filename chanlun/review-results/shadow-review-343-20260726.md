# 影子评审：#343 NT 段非 BTC 品种门控（issue #344）

- 评审对象：commit `40d3082310`（`rust/src/theta_v0/nautilus/backtest_engine.rs` +56）
- 评审者：独立新上下文（实装为另一 claude lineage，资格成立）；只读，未改任何生产文件
- 工作区：`/tmp/kimi-nest-mainline`；目标文件 `git diff HEAD` 为空（未受并行 session 干扰）
- 结论：**无 HIGH，不回票**；MED×1、LOW×2

## Spec 轴（对照 #343 票体 + resolution）

| 检查点 | 判定 | 证据 |
|---|---|---|
| 选 (b) 论证成立（CurrencyPair vs FuturesContract/Equity） | PASS | `btc_instrument()` 构造 `InstrumentAny::CurrencyPair`（backtest_engine.rs:42-70，BTC/USDT + price_precision=2/size_precision=6）；全仓 grep `FuturesContract`/`Equity::new`/`InstrumentAny::Equity` 仅命中本文件注释 :118/:133，**无构造实装**——(a) 换 ID 字符串而底层结构仍假的判断成立（090） |
| 三打印面源头拦截完备（有无绕过） | PASS | `BacktestEngine::new` 全仓唯一出现 = backtest_engine.rs:147（在 `run_theta_backtest` 内、门控之后）；`run_theta_backtest` 全仓唯一调用点 = `rust/src/bin/theta_backtest.rs:147`；`rust/src/bin/` 下引用 nautilus 的 bin 仅 theta_backtest.rs；strategy 侧 `instrument_id` 取自 `bar_type.instrument_id()`（theta_strategy.rs:81），bar_type 由本函数构造 ⟹ PositionChanged/PositionId 均在门控下游，无旁路 |
| fail-fast 点名品种 + 能力边界 | PASS（生产）／守护不足（见 MED-1） | 格式串 :113-120 含 `` `{symbol}` `` 插值与「当前仅支持 BTC」声明；bin 侧 `Err(e) => eprintln + ExitCode::FAILURE`（theta_backtest.rs:164-167） |

## Standards 轴

| 检查点 | 判定 | 证据 |
|---|---|---|
| 纯函数形状 | PASS | `fn require_btc_symbol(symbol: &str) -> anyhow::Result<()>`（:111），不摸 `BacktestEngine`，无 I/O、无状态 |
| 测试只测外部行为（直测纯函数） | PASS（形状）／断言强度见 MED-1 | :180-206，三测试直调纯函数，不起引擎 |
| 注释陈述不变量非过程叙事 | PASS | :108-110 陈述门控约束与可测性；:132-137 陈述「instrument 身份失真（资产类/币种/精度全错）」这一不变量与后果，非步骤流水账 |
| 双 feature 门控下默认 lib 不编译的声明照实 | PASS | `backtest_bin = ["nautilus"]`（Cargo.toml:42）⟹ `--features backtest_bin` 单写即可拉起 nautilus，票体命令准确；默认 lib 共 2000 测试**不含**这 3 个，带 feature 为 2003 含。commit 的「1867/1」是默认口径（不覆盖新测），该事实已在 #343 resolution 中披露 |

## 复跑（本机实测）

```
cargo test --release --lib
  → 1867 passed; 1 failed; 132 ignored
  唯一失败 = theta_v0::classifier::signal::tests::extract_signals_bit_exact_digest_guard
    （#110 在案；未修、未归因）
cargo test --release --lib --features backtest_bin,nautilus backtest_engine
  → 3 passed; 0 failed
    btc_symbol_case_insensitive_passes_gate / non_btc_symbol_gate_names_symbol_and_declares_btc_only
    / every_other_catalogued_symbol_is_gated
```

## 问题

### MED-1：「点名品种」断言空转，插值删掉测试仍全绿

错误文本尾部静态列举了 `ES/CL/GC/BRN/DX 需 FuturesContract instrument，QQQ/OKLO 需 Equity instrument`（:118-119），覆盖 SYMBOLS 全部 7 个非 BTC 品种。于是：

- `every_other_catalogued_symbol_is_gated`（:200-203）的 `err.to_string().contains(sym)`，7 个品种全部被静态文本满足；
- `non_btc_symbol_gate_names_symbol_and_declares_btc_only`（:184）的 `msg.contains("OKLO")` 同理。

**经验证实**：在 /tmp 复刻该消息、把 `{symbol}` 换成 `<REDACTED>` 后编译运行，7/7 品种 `contains` 仍为 true。即测试**无法**检出「格式串丢失 `{symbol}` 插值」这一回归——被声明为守护「点名品种」的断言实际只守护了「返回 Err」。

生产行为本身正确（格式串确有插值），故不构成回票；但守护是假的，属声明>实际（090 的测试侧镜像）。

修法（任一）：断言 `` msg.contains(&format!("`{sym}`")) ``（反引号包裹只在插值处出现），或改用不在静态文本中的品种（如 `"FOO"`）验证点名。

### LOW-2：测试硬编码 7 品种数组，未迭代 `SYMBOLS`

:198 写死 `["ES","CL","GC","BRN","DX","QQQ","OKLO"]`，而 `SYMBOLS`（backtest/data.rs:54，同 crate `pub const`，本文件已从该模块 import `Dataset`）是权威表且注释预告会追加条目。表增项时测试覆盖静默漂移。门控是白名单（非 BTC 一律拒），**行为仍正确**，仅覆盖度漂移。

### LOW-3：门控在 ⑥ 段入口而非 CLI 入口，「fail-fast」在 CLI 层面不成立

非 BTC 品种仍会先跑完 in-crate ⑤ 全量回测、打印全部指标与 L1/L2 等级，最后在 ⑥ 段报错并 `ExitCode::FAILURE`。非本次引入（原状同样以「data was empty」失败退出），且 in-crate 段对非 BTC 合法（无 instrument 硬编码），故不判问题回退；仅记：对 8 品种中的 7 个，CLI 现在恒定以 FAILURE 收尾，「fail-fast」实为「fail-late-but-loud」。若后续要求 CLI 对非 BTC 可成功收尾，需拆分退出码语义（另票）。

## 影响声明

本评审只读；唯一落盘 = 本文件。未触碰 `coverage.rs`、未触碰 git status 中 ~370 个 `D` 条目、未修 #110。
