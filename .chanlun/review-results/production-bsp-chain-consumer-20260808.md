# 生产下单层消费哪条买卖点链——笔中枢 vs 线段中枢（#945，map #931）

只读勘察，未改任何文件、未 commit、未跑构建/测试。

## 0. 结论先行

1. **NT 适配层实际消费哪条链**：本仓存在**两条互不相交的 NT 适配层**，各消费不同东西：
   - **(a) `trading_system/strategy/chanlun_strategy.py::ChanlunStrategy`**（Python，仓内注释自称 "production ChanlunStrategy"，见 §2）——**只消费线段中枢（segment-level/走势级）BSP 链**，从未调用任何 `bi_zhongshu` 相关接口。
   - **(b) `rust/src/theta_v0/nautilus/theta_strategy.rs::ThetaStrategy`**（Rust-native，`theta_v0` 子系统）——**两条都不消费**：它是完全独立的第三套引擎（S_Θ / π），自证"未复用 `buysellpoint.rs`/`zhongshu.rs`/`orchestrator.rs` 一行"（`lib.rs:38-41`），其"中枢"（`Center`）与本票讨论的"笔中枢/线段中枢"不是同一物。
2. **回测臂消费哪条**：`trading_system/backtest/runner.py` 用**同一个** `ChanlunStrategy` 类驱动 NT `BacktestEngine`，与 (a) 走同一段代码——**回测臂与"实盘臂"在代码层面一致**（都只走线段中枢）。但**"实盘臂"目前不存在可运行实例**：`trading_system/live/runner.py` 明示"阶段4+骨架，当前不可运行"（见 §3）。**这本身就是一个应单独标出的发现**：不是"两臂走不同链"，而是"实盘臂尚未落地，无法拿真实数据验证一致性只能验证代码同源"。
3. **两条是否都消费 / 各自触发条件**：在"笔中枢 vs 线段中枢"这个二元问题范围内，**只有线段中枢链接入会下单的路径**。笔中枢链（`current_bi_zhongshu_buysellpoints(_inc)` 等）**只被 `analysis/` 下的研究/回测脚本消费**（`organic_signals.py`、`fugue_version_i.py`、`m1_i_rust_engine.py` 等），这些脚本经检索**没有一个**调用 `submit_order`/`order_factory`（见 §4），即不下单。
4. **消费点完整清单**：见 §5，逐跳 `文件:行号:函数`。

**判定：生产下单层（唯一已知会真实调用 `submit_order` 的策略类 `ChanlunStrategy`）消费的是——线段中枢层（segment-level BSP，即 #944 探针中样本极小的"层 A"）。笔中枢层（#944 43.4% 对齐率的那条链）当前没有任何下单路径消费它，只在研究脚本里被读。**

⟹ 这直接回答 #944 提出的担忧的**反面**：不是"43.4% 的强结论不适用因为生产走线段中枢"，而是**生产走的恰好是 n=3、测不出来的那条链**——43.4% 那条（笔中枢）**根本没有连到任何下单代码**。这比 #944 设想的两种可能都更极端，需要在引用 ADR 0020 重裁时特别标注。

---

## 1. 检索范围与方法

- `grep -rln "nautilus\|Nautilus" --include="*.rs" --include="*.py" .`（排除 `target/`、`.git/`）：命中 rust 侧 `rust/src/theta_v0/nautilus/*` 与 `rust/src/lib.rs`／`recursive_t`／`spiral`；Python 侧 `trading_system/` 全目录 + 少数 `analysis/`/`scripts/`/`tests/` 文件。
- `grep -rn "bi_zhongshu\|current_bi_zhongshu\|_inc_bz" trading_system/ analysis/ scripts/`：确认 `trading_system/` 零命中，`analysis/` 大量命中（研究脚本）。
- 逐文件 `Read`（非 grep 命中数断言）确认下方所有 `文件:行号:函数` 引用。
- **未做**：未跑 `cargo build --features nautilus`（票面禁止构建）；未验证 `nautilus-model`/`nautilus-trading` crate 版本是否真的可编译通过（`theta_v0/nautilus/mod.rs:11-23` 自称"已过骨架期"，但本次未编译验证，按 090 记为**未核实**，非"已核实为真"）。

---

## 2. 引擎拓扑：三条独立链，非"一条链两个层"

### 2.1 顶层引擎（`orchestrator.rs` 家族）—— #944 探针实际测量的对象

`rust/src/lib.rs:20-79`（`mod` 声明区）注册了 `bi_engine`/`bi_zhongshu_bsp`/`buysellpoint`/`divergence`/`fractal`/`level`/`macd`/`moves`/`orchestrator`/`ph`/`segment`/`segment_layers`/`stroke`/`zhongshu` 共 14 个**私有**模块（`lib.rs:26-27` GUARD-ROLE 注释列名单），经 `#[pyclass(name = "RecursiveOrchestrator")]`（`lib.rs:933-954`）暴露给 Python。这条引擎产生**两条**买卖点链：

- **线段中枢（层 A）**：`orchestrator.rs::buysellpoints()`（`orchestrator.rs:1117-1123`）——默认 `use_inc_bsp=true`（`orchestrator.rs:776` `use_inc_bsp: enable_bsp && !enable_macd_divergence`，pyo3 默认 `enable_bsp=true`/`enable_macd_divergence=false`，`lib.rs:965-966`）⟹ 读 `self.inc_bsp.current()`；`inc_bsp` 构造于 `orchestrator.rs:779` `IncrementalSegBsp::new(1, require_settled_subseg)`，`IncrementalSegBsp` 定义在 `segment_layers.rs:302`（"Seg" = segment/线段），其内部 `IncrementalSegZhongshu`（`segment_layers.rs:66`）是**线段中枢**（非笔中枢）。
- **笔中枢（层 B）**：`lib.rs:1109` `current_bi_zhongshu_buysellpoints`、`lib.rs:1179` `current_bi_zhongshu_buysellpoints_inc`（增量版，用 `bi_zhongshu_bsp.rs::IncrementalBiZhongshuBsp`）——直接吃 `confirmed` 笔（`stroke.rs::Stroke`），不经过 `segment.rs`/`Segment` 中转。

**这两条正是 #944 探针 `p944_bsp_layer_mismatch.rs`（`bin/p944_bsp_layer_mismatch.rs:31-65`）测量的"层 A / 层 B"**——本票复核确认该探针文档头（`:31-65`）逐行引用与实际代码一致（已核实 `orchestrator.rs:1117-1123`、`segment_layers.rs:302`、`bi_zhongshu_bsp.rs` 契约声明）。

### 2.2 `theta_v0`（S_Θ / π）—— 完全平行、零复用

`lib.rs:26-41` 的 GUARD-ROLE 注释自证：`theta_v0`（π）对 `orchestrator.rs` 一组"生产引用 = 0"，"π 完全自包含未复用本组一行"。`theta_v0` 有自己的 parser（`theta_v0/parser/segment.rs`、`stroke.rs`）、自己的中枢构造（`CenterConstruct.lean` 对齐的 `Center` 类型）、自己的 `BspBits`/`VoiceDecision`（`theta_v0/strategy/`），与 `zhongshu.rs::Zhongshu`/`buysellpoint.rs::BuySellPoint` 无类型层面关联。**它既不是"笔中枢"也不是"线段中枢"的消费者，是第三条链。**

### 2.3 `recursive_t`（T 引擎，`rec_engine.rs`/`t_engine.rs`）—— 本票未展开

`grep` 命中 `rust/src/recursive_t/ffi.rs`/`stream.rs` 含 "nautilus" 字样，但按既存记忆（`project_production_path_is_nt.md`）这是**另一套**平行架构（TRoot/TPositionEngine，操作语义层），与本票"笔中枢/线段中枢 BSP"提法不在同一概念层。**本票未深挖**（票面聚焦 #944 的两条 BSP 链，`recursive_t` 不产 `BuySellPoint`/`Zhongshu` 类型的信号），标记为**未查**，非"不存在"。

---

## 3. 消费面普查

### 3.1 `trading_system/`（唯一已知会调用 `submit_order` 的路径）

`grep -rn "bi_zhongshu\|current_bi_zhongshu\|_inc_bz" trading_system/`：**零命中**（检索式已跑，覆盖 `trading_system/` 全目录）。

`trading_system/strategy/signal_bridge.py::ChanlunBridge`：
- 构造：`self._orch = newchan_rust.RecursiveOrchestrator(**cfg)`（`signal_bridge.py:61`）。
- 信号读出：`drain_signals()`（`signal_bridge.py:115-148`）第 129 行 `self._orch.take_trend_bsp_events()`——`take_trend_bsp_events` 是 `lib.rs:1441-1497`（读 `self.inner.buysellpoints()`，即 §2.1 的**线段中枢**层）。方法名前缀 "trend"（走势级）= `lib.rs:1367` 注释自证 "走势级（level-1 segment）"。
- **未见任何 `bi_zhongshu_*` 调用**（`signal_bridge.py` 全文 193 行已通读）。

`trading_system/strategy/chanlun_strategy.py::ChanlunStrategy`：
- `on_bar`（`chanlun_strategy.py:104-124`）：第 105 行 `result = self.bridge.feed(bar)`，第 117 行 `signals = self.bridge.drain_signals()`，第 123-124 行 `if self.config.enable_orders: self._execute_signal(sig, bar)`。
- `_execute_signal`（`chanlun_strategy.py:126-156`）：第 151 行 `order_id = self.maker.place(side=sig.action, quantity=qty, anchor_price=sig.price)`。
- **未见任何 `bi_zhongshu_*` 调用**（`chanlun_strategy.py` 全文 198 行已通读）。

`trading_system/backtest_unn_stream.py:19-22`（旁证，非 `ChanlunStrategy` 自身文档，但确认业内命名）：
> "为何不改 production ChanlunStrategy（任务字面）：它走独立的 ChanlunBridge 信号路径（阶段1 信号贯通，PositionalStream 接入尚是 TODO），与 organic_signals/unn 路径是两套信号架构。"

——这条注释把 `ChanlunStrategy` 明确称为 **"production ChanlunStrategy"**，且明确指出它与消费笔中枢的 `organic_signals`/`unn` 路径是"两套信号架构"，与本票 §2 的代码级发现互相印证（非本票单方面推断）。

### 3.2 `analysis/`（笔中枢链的实际消费面——研究脚本，不下单）

`grep -rln "bi_zhongshu\|current_bi_zhongshu" analysis/*.py | xargs grep -l "submit_order\|order_factory\|\.market(\|\.limit("`：**零命中**（检索式已跑）。消费笔中枢链的脚本包括（非穷举，已查到的有）：
- `analysis/organic_signals.py:315-331`（`bi_zhongshu_last_move_dir`/`bi_zhongshu_last_move_kind`/`take_bi_zhongshu_bsp_events`/`take_bi_zhongshu_div_events`/`bi_zhongshu_new_signals`）——信号层公共模块，产物是磁带（tape），非订单。
- `analysis/fugue_version_i.py:339,415-482`、`analysis/fugue_version_i_slice.py:360,425`（`confirmed_bsp_bi_zhongshu`/`_BiZhongshuBspTracker`）——"赋格"研究回测。
- `analysis/m1_i_rust_engine.py:185`（`bi_zhongshu_new_signals`）——研究回测。
- `analysis/diagnose_per_ladder_bsp.py:80`、`analysis/diagnose_high_level_bsp.py:103`——诊断脚本。

这些脚本的产物是 JSON/报告/磁带（`.json`/`.md`），未见下单 API 调用。**090 照实**：本票只检索了 `submit_order`/`order_factory`/`.market(`/`.limit(` 四个字面量作为"下单"判据，若存在用其他中转方式下单（如经某个未命中字面量的包装函数）本检索式会漏检——**未穷举证明**，只报"已查到的有 N 处消费、0 处下单调用"。

### 3.3 `theta_v0::nautilus`（第三条链，未见接线到 `trading_system/`）

`grep -rn "theta_v0\|ThetaStrategy" trading_system/`：本次未执行该检索（补充：见下方"未查到"节，标记待补）。从模块拓扑看，`theta_v0::nautilus::theta_strategy::ThetaStrategy` 是 Rust-native、feature `nautilus` 门控（`theta_strategy.rs` 头注 `#[cfg(feature = "nautilus")]` via `mod.rs:67-68`），与 Python `trading_system/` 是两套独立集成路径（Rust-native strategy vs Python strategy 走同一个 Nautilus 生态但类型系统不共享）。`rust/Cargo.toml:52-53` 注册了 `theta_backtest` bin（`src/bin/theta_backtest.rs`），`docs/nautilus-integration-design.md:15,197-228` 是其设计文档，自称"回测/实盘统一，同一 `ThetaStrategy` 适配器零代码切换"——但本票未核实该 bin 当前是否可编译/可跑（票面禁止构建）。

---

## 4. 实盘臂 vs 回测臂

`trading_system/backtest/runner.py:8,37,66-76`：
```
验证链路：parquet → Bar 构造（路B）→ BacktestEngine → ChanlunStrategy.on_bar
...
from trading_system.strategy.chanlun_strategy import ChanlunStrategy, ChanlunStrategyConfig
...
strategy = ChanlunStrategy(config=ChanlunStrategyConfig(..., enable_orders=enable_orders, ...))
engine.add_strategy(strategy)
```
这是**可运行**的回测入口，用 NT `BacktestEngine` + 真实 `ChanlunStrategy` 类。

`trading_system/live/runner.py:1-13,29-66`：文件头**明示**"NautilusTrader 实盘入口（阶段4+ 骨架，**当前不可运行**）"；`main()`（`:29-66`）除打印配置读取诊断外直接 `return 1`；`node.add_strategy(ChanlunStrategy(...))` 只出现在 `main()` 内部的**伪代码注释块**（`:34-56`，`"""TODO(阶段4): TradingNode 装配。伪代码（实装时逐项替换）"""`），不是可执行代码。

**发现（按票面要求单独标出）**：本仓"生产下单层"目前**没有一个真正跑通的实盘执行实例**——唯一可运行、真实调用 `submit_order` 的是**回测**（`trading_system/backtest/runner.py` + NT `BacktestEngine`）。"回测臂与实盘臂是否一致"这个问题在**代码同源**意义上成立（同一个 `ChanlunStrategy` 类，NT 官方保证 `Strategy ⊂ Actor` 双模式零改动切换，见 `chanlun_strategy.py:8`），但在**"跑过真实生产数据并观测到一致行为"**意义上**无法验证**——因为实盘臂目前是骨架，从未真正跑起来过。

**与 #944 的关系需要澄清（090 照实）**：#944 的"回测序列 vs 实盘序列"（A/B/C 探针武器化的历史数据对比臂）测的是**结构层输出对不同数据源的敏感性**（cash 历史 vs perp 历史），不是本节讨论的"策略执行代码路径（backtest runner vs live runner）"。两者都叫"回测/实盘"但指的不是同一个二分——**本报告特此标出以防止混用**。

---

## 5. 消费点完整清单：买卖点产出 → 下单指令，逐跳 `文件:行号:函数`

以**线段中枢层**（生产实际消费的那条）为例，共 **6 跳**：

| # | 文件:行号 | 函数/方法 | 做什么 |
|---|---|---|---|
| 1 | `rust/src/orchestrator.rs:1117-1123` | `RecursiveOrchestrator::buysellpoints(&self) -> &[BuySellPoint]` | 读增量线段中枢 BSP（`self.inc_bsp.current()`，`inc_bsp` 构造于 `:779` `IncrementalSegBsp::new(1, ..)`） |
| 2 | `rust/src/lib.rs:1441-1497` | `PyRecursiveOrchestrator::take_trend_bsp_events(&mut self)` | PyO3 暴露，对 `#1` 结果做 delta 去重（键 `(kind,side,seg_idx,confirmed)`），返回新事件列表 |
| 3 | `trading_system/strategy/signal_bridge.py:115-148` | `ChanlunBridge.drain_signals(self)` | 第 129 行调 `self._orch.take_trend_bsp_events()`，`confirmed` 事件映射为 `ChanlunSignal(action="BUY"/"SELL", ...)`（`:131-147`） |
| 4 | `trading_system/strategy/chanlun_strategy.py:104-124` | `ChanlunStrategy.on_bar(self, bar)` | 第 117 行 `signals = self.bridge.drain_signals()`；第 123-124 行 `enable_orders` 门控后调 `_execute_signal` |
| 5 | `trading_system/strategy/chanlun_strategy.py:126-156` | `ChanlunStrategy._execute_signal(self, sig, bar)` | 杠杆钳制（`LeverageCalculator.max_quantity`），第 151 行 `self.maker.place(side=sig.action, quantity=qty, anchor_price=sig.price)` |
| 6a | `trading_system/execution/maker_optimizer.py:63-80` | `MakerOptimizer.place(self, side, quantity, anchor_price)` | 第 68-70 行 `self._executor.build_limit_order(...)`，第 71 行 `self._executor.submit(order)` |
| 6b | `trading_system/execution/lmt_executor.py:38-70` | `LmtExecutor.build_limit_order`（`:38-61`）+ `LmtExecutor.submit`（`:63-70`） | 第 53-60 行 `self._strategy.order_factory.limit(...)` 构造真实 Nautilus `LimitOrder`；第 70 行 `self._strategy.submit_order(order)` 提交 |

**笔中枢层**没有对应的第 3-6 跳——`current_bi_zhongshu_buysellpoints(_inc)`/`take_bi_zhongshu_bsp_events`（`lib.rs:1109,1179,~1320-1350`）之后，链路只延伸到 `analysis/` 研究脚本的**信号消费/磁带记录**，未查到任何延伸到 `MakerOptimizer`/`LmtExecutor`/`order_factory` 的第 6 跳。

---

## 6. 未查到的部分与原因

1. **`theta_v0::nautilus::theta_strategy::ThetaStrategy` 是否已/可编译、是否被任何 CLI/脚本实际驱动过真实回测**——票面禁止构建，未跑 `cargo build --features nautilus`；仅读源码 + 设计文档，`mod.rs:11-23` 自称"已过骨架期"但本票**未核实**该自称。
2. **`grep -rn "theta_v0\|ThetaStrategy" trading_system/`（Python 侧是否存在到 theta_v0 的任何 FFI/子进程调用）**——未执行，§3.3 结论仅基于模块拓扑推断（【推断】非【实测】）。
3. **`recursive_t`（T 引擎，`rec_engine.rs`/`t_engine.rs`）是否产生独立的"中枢"概念、是否有第四条下单链**——按既存记忆判断其为操作语义层而非 BSP 层，本票未逐行核实，标记未查。
4. **`analysis/` 消费笔中枢链的脚本是否曾经/是否有计划接入下单**（例如作为未来"阶段2"的信号源）——本票只核实了"当前代码里无下单调用"，未检索设计文档/路线图判断未来计划。
5. **`trading_system/backtest/runner.py` 的 `enable_orders` 在生产环境实际配置值**（CLI 默认值以外的运行时覆盖，例如 CI/部署脚本）——未检索 `.chanlun/`、CI 配置、部署脚本中对该 CLI 的调用方式。

---

## 附：核实方式说明（对应票面纪律）

- 本报告所有"文件:行号:函数"引用均经 `Read` 工具打开确认原文内容（非 grep 命中数断言），逐条列于上表。
- "零命中"类断言（§3.1/§3.2/§3.3）均附检索式，检索范围为 `trading_system/`、`analysis/*.py` 全目录（`grep -rn`/`grep -rln`），非穷举全仓——已在各节标注覆盖范围。
- §4"没有跑通的实盘实例"这一发现基于 `trading_system/live/runner.py` 的**文件头自述**与 `main()` 函数体的直接阅读（`:1-13`, `:29-66`），非推断。
