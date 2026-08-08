# 三套 NT 实现的事实补齐——第一程（#949，map #931）

只读勘察，未改任何既有文件、未修 #947 编译失败、未 commit 代码改动。只裁事实，不裁「哪套是正本」。

## 0. 结论先行（五项各一句话）

1. **路径二能编译（release，已实测本仓 HEAD `5348c981e3`）**，且**已有真实回测记录**：`theta_backtest` CLI 在 2026-07-30（#792）用真实 BTC 一周数据跑通，装的正是 `theta_v0::nautilus::theta_strategy::ThetaStrategy`，经真实 `nautilus_backtest::engine::BacktestEngine` 产出 2754 笔真订单、177 个仓位，退出码 0；本仓自己的实盘化判据（认识论等级 L2）判它已达标。**唯一缺口是本次会话环境没有那份 314MB 行情文件，无法在本 worktree 内亲手复现**，但该证据是已 commit 的报告，非道听途说。
2. **Python 侧到 `theta_v0` 没有 FFI 通道**：`theta_v0` 模块内 `grep pyclass|pyfunction|pymodule` 零命中——它压根没向 PyO3 暴露任何符号，`trading_system/`/`analysis/`/`scripts/` 全目录检索 `theta_v0`/`ThetaStrategy`/`theta_backtest` 命中的全是注释/文档字符串，零实际调用（含 subprocess）。
3. **`recursive_t` 是第四条独立链，而且是四条里唯一走「目标敞口」而非「BSP 信号」范式的**：它有自己的 `center.rs`、自己的 PyO3 导出 `RecTStream`（`ffi.rs:294`），Python 侧 `RecTStrategy`（`trading_system/strategy/rec_t_strategy.py`）真的调用 `order_factory.market()` + `submit_order()`（`:101,106`），但只接在 `trading_system/backtest/rec_backtest.py`（回测），没有任何 live 入口引用它。
4. **路径一的「voice/fusion_tr/hold26 状态机接管信号」改造完全没做**：`signal_bridge.py`（全文 192 行已通读）里 `drain_signals()` 仍是「走势级 confirmed BSP 事件 → BUY/SELL」的骨架直读映射（`:118,129`），「BSP 直读映射退役」是 TODO 里写的**目标状态**不是现状；`notional_frac` 硬编码 `1.0`（`:140`）；`chanlun_strategy.py` 里 `trading_mode: str = "fusion_tr"`（`:39`）只进日志字符串（`:101`），从未被读出来分叉任何行为。而 `fusion_tr`/`hold26`/`voice` 这些概念在 `analysis/`（十余份回测脚本）和 `rust/src/trading/`、`rust/src/theta_v0/strategy/voice.rs` 里**已经有丰富实装**——只是没接到 `trading_system/` 生产壳上。
5. **`enable_orders` 在本仓能查到的所有部署面里，运行时真实取值是 False，且找不到任何生产/部署/CI 路径能把它推成 True**：读取点是 `trading_system/backtest/runner.py:84` 的 `argparse` `action="store_true"`（硬默认 False，非 env var），本仓不存在 `deploy/` 目录（本 HEAD 下无此路径），`.github/workflows/ci.yml` 未见任何 `--enable-orders`/`enable_orders` 调用，`trading_system/live/runner.py` 的 `main()` 只打印诊断信息后 `return 1`（`:66`），把该策略实例化的那段代码只存在于函数体内的**伪代码注释块**（`:34-56`），不会被执行——因此实盘路径连"是否 True"这个问题都无法成立，它从未真正跑起来过。

---

## 1. 环境自检（开工先做的三件事，附结果）

- `git log -3 --oneline`：worktree 初始 HEAD 落在 `19b40159272c0ed9797769ca3e6b63c12d98ef01`（2026-04-20 前后老提交，`rust/`、`trading_system/`、`AGENTS.md` 均不存在于该 commit 树）——**命中票面警告的老坑**：worktree 分支 `worktree-agent-af96a70cefeff59ce` 没有指向本地 `main`。
- `git checkout main` 报 `fatal: 'main' is already used by worktree`（主仓已 checkout main），改用 `git switch -C worktree-agent-af96a70cefeff59ce main` 把本 worktree 分支重置指向本地 `main` 的提交 `5348c981e3`（`docs(#938): ADR 0019 补裁 A/B`）。重置后 `rust/`、`trading_system/`、`AGENTS.md` 均存在，`git status` clean。**本报告全部事实基于该 HEAD。**
- `gh issue view 949`：读完整票面，五项待补事实、纪律（逐条打开确认、检索式、090 照实）已内化到下方各节。
- 读 `.chanlun/review-results/production-bsp-chain-consumer-20260808.md`（#945 报告，本票背景表格的直接来源）：已复用其 §2.1-§2.3、§5 的既有结论（线段中枢链是唯一下单消费者），本票只做补齐，不重复其已完成的核验。

---

## 2. 路径二：能否编译、有没有跑过真实回测（票面第一项）

### 2.1 编译（本次会话亲手实测，非援引）

```
$ cd rust && cargo build --release --bin theta_backtest --features backtest_bin
   Finished `release` profile [optimized] target(s) in 59.77s
$ ls -la target/release/theta_backtest
-rwxr-xr-x  1 silencehan  staff  9634864  ... theta_backtest

$ cargo build --release --lib --features nautilus
   Finished `release` profile [optimized] target(s) in 8.10s
```

两条命令均 exit 0，产出真实可执行文件（9.6MB）。**未使用 `cargo check`**（本仓在案「重放缓存假绿」教训），也**未跑全 crate `cargo build --release`**（#947 已知会因 `p938_bound_center_openness` 报 `E0433` 失败，本次绕开，未触碰）。

- `rust/src/theta_v0/nautilus/backtest_engine.rs:169-170`（`Read` 逐行确认）：
  ```rust
  let strategy = ThetaStrategy::new(strat_config, theta.clone(), bar_type, 6);
  engine.add_strategy(strategy)?;
  ```
  确认 `theta_backtest` 二进制里真的装的是 `theta_v0::nautilus::theta_strategy::ThetaStrategy`（`backtest_engine.rs:36` `use super::theta_strategy::ThetaStrategy;`），不是别的占位。

### 2.2 有没有跑过真实回测——找到的是：**是**，已 commit 的运行记录

`.chanlun/review-results/issue792-theta-backtest-btc-week-readout.md`（`git log` 确认属提交 `b7eaff1097`，非未追踪草稿）：

- 命令：`./target/release/theta_backtest BTC 2024-01-01 2024-01-07`（release + `backtest_bin` feature）
- 数据：`analysis/data_cache/btc_1m_full.json`（314MB，gitignore 内，报告称磁盘实存；本次会话在本 worktree 内**未找到该文件**——见 §6 未查到项）
- 退出码：0，10080 bar
- 真实 Nautilus `BacktestEngine` 段（第⑥段）日志原文（报告内直接粘贴）：`Total orders: 2_754`、`Total positions: 177`、`Iterations: 10_080`
- 报告自评「认识论等级：L2（真实 Nautilus 引擎 + 非空订单流）——生产引擎贯通验证通过」

同一 map（#787）下另一份报告 `.chanlun/review-results/issue792-e2e-runnability-20260730.md`（同日，`/tmp/e2e-792-main-check` 独立 worktree 复测）**报告了「数据缺失，未跑到产订单流那一步」**——与上一份看似矛盾，实测两者是**同一天不同 worktree 的两次尝试**：`/tmp` 快照没有 314MB 数据文件（gitignore 文件不随 `git worktree add` 复制），主仓 worktree 磁盘上有；两份报告互不矛盾，只是数据文件的物理存在性因 worktree 而异。该报告里同时确认：
- `cargo test --features nautilus --lib theta_v0::nautilus` → 19/19 pass（adapter 层单测）
- `cargo test --release --features backtest_bin --lib theta_v0::nautilus::backtest_engine` → 4/4 pass（仅覆盖 `require_btc_symbol` 门控，不覆盖端到端订单流）
- 全仓 `grep -rn "LiveNode" rust/src` 零命中——**没有任何实盘（非回测）入口调用 `ThetaStrategy`**

`rust/src/theta_v0/nautilus/mod.rs:11-23`（`Read` 逐行确认）自称「已过骨架期」并给出三条依据（无条件编译进 crate / nautilus 依赖已入 Cargo.toml / `use nautilus_*` 已在产），并把「真实集成跑通」标为 L2、援引「task#8 真实 BacktestEngine 非空订单流」——与 #792 那份 2754 单/177 仓位的记录对得上，**本票核实该自称为真（此前 #945 未验证，本票补上）**。

### 2.3 【实测】vs【推断】标注

- 【实测】`theta_backtest` bin 本次会话能真编译（release，2026-08-07 环境）。
- 【实测，援引已 commit 报告】2026-07-30 该 bin 用真实数据跑过一次，产 2754 单/177 仓位，退出码 0。
- 【未复现】本次会话未能在本 worktree 亲手重跑该命令，因 314MB 数据文件缺失（§6）。

---

## 3. Python 侧到 `theta_v0` 的隐藏 FFI 调用（票面第二项）

### 3.1 结构性检查：`theta_v0` 是否向 PyO3 暴露任何符号

```
$ grep -n "pyclass\|pyfunction\|pymodule" rust/src/lib.rs | grep -i theta
（零命中）
$ grep -rn "pyclass\|#\[pyfunction\]\|pymodule" rust/src/theta_v0/ 2>/dev/null
（零命中）
```
覆盖目录：`rust/src/lib.rs` 全文 + `rust/src/theta_v0/` 全子树（含 `nautilus/`、`strategy/`、`backtest/`、`classifier/`、`parser/`、`complete/` 等全部子模块）。**结论：`theta_v0` 没有一个 `#[pyclass]`/`#[pyfunction]` 被导出，Python 侧结构性无法 import 到它的任何符号**——这是比"没检索到调用"更强的证据（不是漏检，是被暴露面为零）。

### 3.2 检索式：Python 侧文本引用

```
$ grep -rn "theta_v0\|ThetaStrategy" trading_system/     → 零命中（覆盖 trading_system/ 全目录）
$ grep -rln "theta_v0\|ThetaStrategy" --include="*.py" . （排除 .git/, tmp/）
  → analysis/_xcheck_secondkind_v1.py, analysis/_db_sec_pull.py, analysis/_fetch_es_1s_1y.py,
    analysis/_attrib_segment_divergence.py, analysis/_db_sec_pull_brn.py,
    analysis/segment_refsem_cert.py, tests/test_zhongshu_overlap_golden.py,
    scripts/check_fixture_drift.py, scripts/fee_account_decomposition.py,
    scripts/check_armR_trades_digest.py, src/newchan/a_segment_v1.py
```
逐个打开确认命中行内容：**全部是注释/docstring**（例如 `analysis/_attrib_segment_divergence.py` 是"在 Python 侧复刻 theta_v0 算法"做归因研究，注释里反复提 `theta_v0` 只是指代其对照的 Rust 算法标的，代码里没有 import/调用任何 `theta_v0` 符号；`scripts/check_fixture_drift.py` 提 `theta_v0_buy_parity` 是形式化 fixture 名字，不是运行时调用）。

```
$ grep -rn "theta_backtest\|subprocess.*theta\|Popen.*theta" trading_system/ analysis/ scripts/
（零命中）
```

### 3.3 结论

- 【实测】`theta_v0` 无 PyO3 暴露面，结构性排除直接 import 调用。
- 【实测】`trading_system/` 全目录零文本命中。
- 【实测】全仓（`trading_system/`/`analysis/`/`scripts/`）零 subprocess/Popen 调用 `theta_backtest` 二进制。
- **未穷举证明**：本节检索式为 `grep -rn "theta_v0|ThetaStrategy|theta_backtest"` + subprocess 关键词组合，若存在完全不含这些字面量的间接调用（例如经环境变量拼路径再 `os.system` 一个变量名），本检索式会漏检——已查到的是 0 处，非"绝对不存在"的证明。

---

## 4. `recursive_t`（T 引擎）是不是第四条独立链（票面第三项，此前仅【推断】）

### 4.1 模块规模与注册

`rust/src/lib.rs:66`（`Read` 确认）：`pub mod recursive_t;`，与 `theta_v0`（`:77`）、`orchestrator`（`:62` 私有 `mod orchestrator;`）并列注册，非嵌套子模块。`find src/recursive_t -name "*.rs" | xargs wc -l` 合计 **17409 行**，含独立的 `center.rs`（笔中枢/线段中枢概念之外的"中枢"实现）、`operator.rs`、`trend.rs`、`divergence.rs`、`backtest.rs`、`ffi.rs` 等 13 个文件。

### 4.2 PyO3 导出：真实存在的 FFI（与 §3 的 `theta_v0` 零暴露形成对比）

`rust/src/recursive_t/ffi.rs:294`（`Read` 确认）：`#[pyclass(name = "RecTStream")]`，`:299` `#[pymethods]`，`:316` `fn push_bar(&mut self, o: f64, h: f64, l: f64, c: f64) -> f64`（返回目标净敞口，非 BSP 信号）。同文件 `:219` 还有 `#[pyclass(name = "TFugueStream")]`——**recursive_t 至少有两个 PyO3 导出面**。

### 4.3 Python 侧消费者：真实下单

`trading_system/strategy/rec_t_strategy.py`（全文已读）：
- `:17` `import newchan_rust`，`:33` `self.engine = newchan_rust.RecTStream(config.mode)`
- `docstring`（`:1-9`）自述定位：「与 ChanlunStrategy（BSP 信号→订单）的范畴差：递归引擎产出**仓位**不是信号，故走目标敞口跟踪（NETTING 镜像），不走 BSP→BUY/SELL 映射」——**这是本票需要的关键澄清**：`recursive_t` 不是"操作语义层但不下单"，而是**用完全不同的范式（目标敞口跟踪 vs BSP 信号触发）真实下单**。
- `_rebalance()`（`:90-106`）：`:101` `order = self.order_factory.market(...)`，`:106` `self.submit_order(order)`——真实 Nautilus 下单调用，逐行确认非注释。

### 4.4 是否接入任何 runner

```
$ grep -rln "RecTStrategy" trading_system/
  trading_system/backtest/rec_backtest.py
  trading_system/strategy/rec_t_strategy.py
```
只有 2 处，`rec_backtest.py`（回测入口）+ 自身定义文件。**未在 `trading_system/live/` 任何文件中出现**（`grep -rn "RecTStrategy" trading_system/live/` 零命中，本节已核实）。

### 4.5 结论（此前 #945 标注【推断】，本票升级为【实测】）

`recursive_t` 是**第四条独立链**，且和路径一（BSP 信号→订单）、路径二（`ThetaStrategy`，S_Θ 自有中枢概念）都不共享类型系统——它甚至不产生"买卖点"这个概念，产生的是"目标净敞口"。它**有真实下单代码**（`order_factory.market` + `submit_order`），但**只接在回测入口**，没有任何 live 驱动代码，与路径二（`ThetaStrategy`）的"回测能跑、无 live 入口"是同一种缺口形状。

---

## 5. 路径一的改造完成度：voice/fusion_tr/hold26 状态机 vs 骨架占位（票面第四项）

### 5.1 `signal_bridge.py`（全文 192 行，`Read` 通读）现状

- `drain_signals()`（`:115-148`）docstring（`:118-122`）：
  ```
  当前骨架的信号映射：走势级 confirmed BSP 事件 → BUY/SELL。
  TODO(阶段2): 接入 PositionalStream（设计 §4.2）后，本方法改为
      row = 组装磁带行(bsp/div/move_settle 事件)
      intents = stream.on_bar_row(row)
  由 fusion_tr/hold26 voice 状态机产出意图，BSP 直读映射退役。
  ```
  实现体（`:124-148`）：`epoch = self._orch.bsp_epoch()` → `self._orch.take_trend_bsp_events()`（`:129`）→ 逐事件映射 `action="BUY"/"SELL"`（`:134`）——**就是 docstring 里说要退役的那条直读映射本身**，TODO 描述的是**尚未发生的未来状态**，不是已完成后留下的注释残迹。
- `:140` `notional_frac=1.0,  # TODO(阶段2): notional_frac 由 voice 配额产出；骨架占位 1.0`——硬编码常数，逐行确认非动态计算。
- `:182-186`（`current_zhongshu_band`）另一处 TODO：「阶段3」P_neg 严格定义未定，当前"骨架先暴露最近中枢带"。

### 5.2 `chanlun_strategy.py`（全文 198 行，`Read` 通读）现状

- `:39` `trading_mode: str = "fusion_tr"  # TODO(阶段2): PositionalStream mode 变体名`——只是个字符串标签。
- 全文档 `grep -n "trading_mode"`：仅两处，`:39`（声明）+ `:101`（`self.log.info(f"...mode={self.config.trading_mode}...")`，纯日志）。**没有任何 `if self.config.trading_mode == ...` 分支**——该字段不影响任何运行时行为，纯占位/元数据。
- `:96` 另一处 TODO：「阶段4 预热协议 §2.3：冷启动 catalog 重放 → request_bars 补缺口 → 订阅」未实装，当前回测模式绕过（"回测模式下数据全量来自 engine.add_data，直接订阅即可"）。
- `:168-169`、`:176`：`on_order_filled`/`on_order_canceled` 里各有一条「TODO(阶段2): stream.confirm_fill/reject_intent —— 影子账本只在成交确认时更新」，同样未实装。

### 5.3 `fusion_tr`/`hold26`/`voice` 在别处的实装状态（对比：不是"从未写过"，是"没接生产壳"）

检索式：`grep -rln "fusion_tr|hold26|voice" --include="*.py" --include="*.rs" .`（覆盖全仓，排除 `.git/`）：
- `fusion_tr`：命中 `analysis/` 下 9 份回测脚本（`unified_voice_v2_backtest.py`/`bidirectional_s1s4_backtest.py`/`p7_conjunction_backtest.py`/`btc_2week_1s_backtest.py`/`interval_nesting_backtest.py`/`axiom_voice_backtest.py`/`unified_voice_backtest.py`/`universal_combination_backtest.py`/`universal_combination_v2_backtest.py`）+ `rust/src/lib.rs`、`rust/src/trading/positional_fusion.rs`、`rust/src/trading/positional.rs`，以及 `trading_system/config/instruments.py`（`:111` `trading_mode="fusion_tr",  # 在册全史最优 +4298%`——**这是研究结论的备注，非生产代码分叉依据**，同文件 `:44` 确认 `trading_mode` 字段本身也只是"在册白名单归属"的元数据）。
- `hold26`：命中 `analysis/` 下 15+ 份回测脚本 + `rust/src/recursive_t/backtest.rs`、`rust/src/trading/{isolated_fugue,nested_fugue,positional_fusion,positional}.rs`。
- `voice`：命中 `rust/src/spiral/`（`voice.rs`/`engine.rs`/`result.rs`/`closure.rs`/`prove.rs`/`ffi.rs`/`mod.rs`/`operation.rs`/`signal.rs`/`accounting.rs`）、`rust/src/theta_v0/strategy/voice.rs`（及 `theta_v0/classifier/voice_eat.rs`）、`rust/src/trading/{unified_voice,axiom_voice,dual_voice}.rs` 等大量文件，**都在 `rust/src/trading/` 与 `rust/src/theta_v0/`，没有一处被 `trading_system/` 引用**（§3.2 已确认 `trading_system/` 全目录不 import `theta_v0` 任何符号；`trading_system/` 是否引用 `rust/src/trading/` 里的 voice 系列本节未逐一核实，标记未查——见 §6）。

**结论**：voice/fusion_tr/hold26 状态机作为**概念和 Rust/研究实现是存在的且相当丰富**（十余份研究脚本 + 若干独立 Rust 模块），但**生产壳（`trading_system/strategy/`）里一行都没接**——signal_bridge.py 仍是 BSP 直读，chanlun_strategy.py 的 `trading_mode` 字段是死标签。这是「概念已实装但没人知道/没接线」这类本仓在案事故的又一个标本（与统一架构正本图 #787 同款）。

---

## 6. `enable_orders` 在部署/CI 中的实际运行时取值（票面第五项）

### 6.1 真正的读取点（按纪律，不只看构造函数默认值）

- `ChanlunStrategyConfig.enable_orders: bool = False`（`chanlun_strategy.py:40`，`Read` 确认）——这是**类字段默认值**，会被任何显式传参覆盖。
- 实际赋值来源：`chanlun_strategy.py:123` `if self.config.enable_orders: self._execute_signal(sig, bar)`——运行时判据直接读 `self.config.enable_orders`，没有 env var 中转（`grep -n "os.environ|getenv" trading_system/strategy/chanlun_strategy.py` 零命中）。
- 唯一发现的赋值路径：`trading_system/backtest/runner.py:71` `enable_orders=enable_orders,`（传给 `ChanlunStrategyConfig`），该函数参数来自 `:94` `args.enable_orders`，`args` 来自 argparse。
- **argparse 定义**（`runner.py:84`，`Read` 确认）：
  ```python
  parser.add_argument("--enable-orders", action="store_true", help="开启下单（阶段3形态）")
  ```
  `action="store_true"` 的语义：不传该 flag 时值为 `False`，传了才为 `True`——**没有 env var 读取、没有配置文件覆盖点**，是本仓在案教训提到的"真正读取点"意义上的**硬默认 False**，不是"看起来关但其实被某个 `.is_ok()` 之类的判据悄悄打开"的那种坑。本节已按纪律去查了这一层，没查到类似隐藏开关。

### 6.2 部署/CI 面检索

```
$ ls deploy/                                        → 本 HEAD 下不存在 deploy/ 目录（No such file or directory）
$ grep -rln "enable_orders" deploy/ .github/ scripts/  → 零命中（deploy/ 目录本身不存在，其余两处零命中）
$ grep -rn "enable_orders" --include="*.py" --include="*.yml" --include="*.yaml" --include="*.sh" --include="*.toml" .
  → 仅命中 chanlun_strategy.py（声明/日志/判据三处，已列于 §6.1）与 backtest/runner.py（四处，已列于 §6.1），
    覆盖范围：全仓（排除 .git/）按上述扩展名检索。
```
`.github/workflows/ci.yml`（`Read` 全文）未见任何对 `trading_system/backtest/runner.py` 的调用（该文件里唯一相关文字是 `:68-69` 的注释，指向 `tests/test_trading_system_skeleton.py` 做纯 import 级 smoke test，不涉及 `--enable-orders`）。

### 6.3 live 路径的特殊情况

`trading_system/live/runner.py:29-66`（`Read` 确认）：`main()` 函数体里，`ChanlunStrategy(...)` 实例化只出现在 `:34-56` 的**三重引号伪代码块**（docstring 内文本，非可执行语句）。函数实际执行体（`:57-66`）只打印诊断信息、构造 `hyperliquid_config` 做连接性检查、最终 `return 1`。**因此 live 路径下 `enable_orders` 这个问题不成立——没有任何代码路径能把 `ChanlunStrategy` 在实盘场景实例化，遑论查它的 `enable_orders` 取值。**

### 6.4 结论

- 【实测】`enable_orders` 的运行时真实读取点是 `chanlun_strategy.py:123` 直读 `self.config.enable_orders`。
- 【实测】唯一赋值来源是 `backtest/runner.py` 的 CLI flag `--enable-orders`（`action="store_true"`，硬默认 False，非 env var）。
- 【实测】本仓当前 HEAD 下没有 `deploy/` 目录、CI 不调用该 runner——**找不到任何已知路径能让它在非人工手敲 CLI 的场景下变成 True**。
- 【未能证伪】不排除仓外（VPS/远程部署脚本不在本 git 仓库里）存在传 `--enable-orders` 的启动命令，本票检索范围仅覆盖本 git 仓库（`deploy/`、`.github/`、`scripts/`、全仓文本检索），仓外部署面未查——见 §6.5。

### 6.5 附：`setup_vps.sh`/`README.md` 快速扫描（补充,非本票主检索式）

顺手 `grep -n "enable_orders\|enable-orders" setup_vps.sh README.md` 零命中，未发现部署脚本层面的覆盖，但这两个文件不是本票纪律要求的强制检索范围，仅作旁证，不计入 §6.2 的正式检索式清单。

---

## 7. 未查到的部分与原因

1. **本次会话未能亲手复现 #792 的 `theta_backtest BTC 2024-01-01 2024-01-07` 真实回测**——原因：`analysis/data_cache/btc_1m_full.json`（314MB，gitignore）在本 worktree 磁盘上不存在（`ls` 确认 `No such file or directory`）。本票依据的是已 commit 报告 `.chanlun/review-results/issue792-theta-backtest-btc-week-readout.md`，非本次独立复现。
2. **`grep -rln "voice" rust/src/trading/*.rs | xargs grep -l` 是否被 `trading_system/` 间接引用**（例如经某个尚未识别的 Python↔Rust 边界）——§5.3 只确认了 `trading_system/` 不 import `theta_v0`，未逐一排查 `trading_system/` 是否 import `rust/src/trading/` 下 voice 系列模块对应的 PyO3 导出（若有）。标记未查，非"不存在"。
3. **`trading_system/` 之外、仓外的部署脚本/启动命令**（例如 VPS 上实际跑的 systemd/cron/CI-外部触发脚本）是否传了 `--enable-orders`——不在本 git 仓库检索范围内，未查、无法查。
4. **`theta_v0` 是否存在完全不含 `theta_v0`/`ThetaStrategy`/`theta_backtest` 字面量的间接 Python 调用**（例如经环境变量拼接的动态路径）——§3.3 已注明检索式局限，未做穷举式证明，只报告"已查到的有 0 处"。
5. **`recursive_t` 的 `TFugueStream`（`ffi.rs:219`）用途**——本票只核实了 `RecTStream`（`ffi.rs:294`）被 `rec_t_strategy.py` 消费，`TFugueStream` 是否也有 Python 消费者未检索，标记未查。

---

## 附：核实方式说明（对应票面纪律）

- 所有「文件:行号:函数」引用均经 `Read`/`awk 'NR==n'` 打开确认原文（`backtest_engine.rs:169-170`、`signal_bridge.py` 全文、`chanlun_strategy.py` 全文、`ffi.rs:219,294,299,316`、`runner.py:84`、`live/runner.py:29-66` 等），非 grep 命中数断言。
- 「零命中」类断言均附检索式与覆盖目录，见各节代码块。
- 「唯一/全部」类断言已按纪律弱化为「已查到的有 N 处」，检索式局限已在对应节标注。
- 编译验证严格用 `cargo build --release`（非 `cargo check`），避开本仓在案的重放缓存假绿教训；未触碰 #947 已知的全 crate 构建失败点。
