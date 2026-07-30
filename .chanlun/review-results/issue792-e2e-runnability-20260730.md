# issue #792 端到端跑法实测：从 K 线到下单有几条路真能跑

Part of #787。测量报告，不改代码不改配置。

## 环境说明（先于结果，避免误判）

本 worktree（`agent-a1b46ac04253134e4`）的 HEAD = `19b4015927`（2026-04-20 的老 commit），
当时仓库里**还没有** `rust/`、`trading_system/`、`analysis/`、`formal/`——cargo 项目在本 worktree
里物理不存在。这是环境滞后问题，不是代码问题（协调者已确认）。

绕法：`git worktree add /tmp/e2e-792-main-check main --detach`，在本仓 `.git` 对象库上新开一个
独立目录，checkout 本地 `main` 分支（commit `f6d000fed2`，含 #785 漂移普查订正节，是测试时刻能拿到
的最新 main）。**全部构建与实测在 `/tmp/e2e-792-main-check` 里进行，全新 `target/` 目录**（首次
`cargo build --release --features backtest_bin` 编译耗时 46.64s，`cargo check --lib` 9.52s，
`cargo check --features nautilus` 20.37s——均在超时预算内，无未测完项）。本 worktree 自身
`HEAD`/工作区未被 checkout、switch 或 pull 触碰（自检：`git status` 仍 clean，`HEAD` 仍
`19b4015927`）。

Python 侧测试复用 `/Users/silencehan/Projects/NewChanlun/.venv`（已预装 `newchan_rust` 扩展 +
`nautilus_trader`），因为 `/tmp` 快照没有自己的 venv，且现装 venv 超出"只测不改"的时间预算。用这个
共享 venv 跑 `/tmp` 快照里的脚本，import 链路真实跑通（无 ModuleNotFoundError），数据缺失点的报错
也是脚本对 `/tmp` 快照目录的真实路径解析结果，不是猜测。

---

## 五条候选路径逐张卡

### 路径①：Python 引擎链（`src/newchan/orchestrator/recursive.py::RecursiveOrchestrator`）

- **能否构建**：Python，无编译步骤。`import newchan.orchestrator.recursive` 成功（用主仓
  `.venv`，`sys.path` 指向 `/tmp` 快照的 `src/`）。
- **能否跑**：能跑到"结构"这一步（Bi→Seg→ZS→Move→BSP 五层管线，有 `BuySellPointEngine`/
  `BuySellPointSnapshot`），但**实测未发现任何一条路径把它的输出接到下单**：
  - `grep` 全 `src/newchan/` 引用 `RecursiveOrchestrator` 的文件（`nested_pipeline.py` /
    `ab_bridge_newchan.py` / `gateway.py` / `live_pool.py` / `live_engine.py` / `replay.py` /
    `parallel_edges.py` / 拓扑/交易相关文件共 14 处），**没有一处出现
    `place_order`/`submit_order`/`ib_insync`/`broker`**（实测 grep，非猜测）。
  - `live_engine.py`：docstring 自称"实时数据驱动 RecursiveOrchestrator 增量计算"，逐 bar 产出
    `RecursiveOrchestratorSnapshot`，**没有导出到执行层**。
  - `gateway.py`：FastAPI 网关，REST/WS 回放 + 实时推送，是给前端图表用的可视化后端。
  - `cli.py` 的子命令只有 `fetch` / `plot` / `chart` / `synthetic` / `fetch-db`——没有
    "跑引擎产订单"这一档。
  - `src/newchan/backtest/orchestrator.py` 虽然 import 了 `RecursiveOrchestrator`，但那条链是
    K4 六边选股（支柱 III），本图 Out of Scope 声明已排除，不算"缠论买卖点→下单"这条链。
- **断点**：不是"跑不动"，是**架构性断头**——结构层（Bi/Seg/ZS/Move/BSP）完整存在且可跑，但
  它和任何执行/broker 层之间**没有代码连接**。这是"测出来是断的"，不是"没测出来"。
- **输出形态**：只到 `RecursiveOrchestratorSnapshot`（中枢/走势/买卖点结构），到不了订单。
- **是否在用**：活跃——`recursive.py` 近期提交含 526 号谱系增量化重构（`d495d4c643`）、MACD 背驰穿透
  等，是研究/可视化侧的正本引擎。但"在用"是指结构分析在用，不是指它在驱动交易。

### 路径②③：rust 回测链 + rust+nautilus 回测（合并成一张卡——实测发现是同一个 CLI 二进制）

`rust/src/bin/theta_backtest.rs` 的 `main()` 顺序做两件事：先跑 in-crate 模拟撮合
（`run_theta_v0_pi`，路径②），再调用 `theta_v0::nautilus::backtest_engine::run_theta_backtest`
（真实 `nautilus_backtest::engine::BacktestEngine`，路径③）。两条候选路径在生产入口层面是同一个
二进制的两段，不是两条独立可跑的链，故合并报告。

- **能否构建**：
  - `cargo check --lib`（默认，无 feature）：**通过**，66 warnings 0 error。
  - `cargo check --lib --features nautilus`：**通过**，无 error（含真实
    `use nautilus_common::actor::DataActor` / `nautilus_trading::{Strategy, StrategyCore}` 等，
    非占位）。
  - `cargo build --release --features backtest_bin --bin theta_backtest`：**通过**，46.64s，
    产出可执行文件 `target/release/theta_backtest`。
  - `cargo test --features nautilus --lib theta_v0::nautilus`：**19/19 pass**（adapter 层单元
    自测：bar_adapter/order_adapter/account_adapter/strategy 的 lockstep guard 等，纯合成数据，
    不依赖外部行情）。
  - `cargo test --release --features backtest_bin --lib theta_v0::nautilus::backtest_engine`：
    **4/4 pass**，但这 4 个测试只覆盖 `require_btc_symbol` 门控逻辑（BTC 通过/非 BTC 拒绝），
    **没有**一个测试跑通"真实 BacktestEngine 产非空订单流"的完整链路（那需要真实 dataset，属于
    一次性人工验收 task#8，不在可重复 CI 测试里）。
- **能否跑**：二进制能起、能解析参数、能走到数据加载这一步；**卡在数据缺失**：
  ```
  $ ./target/release/theta_backtest BTC
  数据加载失败: 读取 ".../analysis/data_cache/btc_1m_full.json" 失败: No such file or directory (os error 2)
  ```
  这是实测的真实报错原文，`analysis/data_cache/` 目录下实测只有 5 个文件（venue fee 相关 json
  + provenance md），`SYMBOLS` 表要求的 8 个品种数据文件（`btc_1m_full.json` /
  `es_1m_databento_10y.json` 等）一个都不在。全仓 `find` 未命中任何 `*.parquet`/带 BTC 的
  `*.csv`。**缺数据，未测到"能否产非空订单流"这一步**。
  - 非 BTC 品种（如 `ES`）走的是另一条真实分支：**在数据加载之前**就被
    `require_btc_symbol` fail-fast 拦下（"NT 段当前仅支持 BTC…"），这是设计内行为，**不是
    bug**，实测确认门控生效。
- **断点**：数据缺失，卡在 `load_by_symbol` 这一步，两段（in-crate + 真实 nautilus 引擎）都还
  没机会跑，**未测完**，原因=缺数据。
- **输出形态**（据代码/文档，未经真实数据验证）：`RunResult`（`n_orders`/`strat_return`/
  `sharpe`/`is_l2` 等指标）+ 真实 nautilus `BacktestResult`（`total_orders`）。
- **是否在用**：CI（`.github/workflows/ci.yml` 的 `rust-check` job）实测跑
  `cargo check --all-targets` 和 `cargo check --all-targets --features backtest_bin`，与本次
  手测的 feature 组合一致；近期有 #345/#346/#347/#524 等专门针对这条链的提交。

### 路径④：nautilus 实盘（`theta_v0/nautilus/theta_strategy.rs`）

- **能否构建**：随 `--features nautilus` 一并编译通过（见上）。`theta_strategy.rs` 是真实
  `StrategyCore + DataActor + Strategy` 实现（非占位骨架，模块头注释 #524 订正过一次）。
- **能否跑**：**全仓实测未找到任何调用它的 `fn main()` 或 `LiveNode` 组装点**。
  `grep -rn "LiveNode" rust/src` 全仓 0 命中；`grep` 引用 `theta_strategy::ThetaStrategy` 的
  文件只有 `mod.rs`（模块声明）和 `backtest_engine.rs`（回测用），**没有第三处**。
- **断点**：不是数据缺失，是**压根没有实盘入口**——`ThetaStrategy` 目前只被"装进真实
  `BacktestEngine`"消费（=路径③的一部分），没有装进任何 `TradingNode`/`LiveNode`。这是"测出来
  是没有"，不是"没测出来"。
- **输出形态**：无（无可运行入口）。
- **是否在用**：作为回测引擎的策略壳在用（路径③），作为"实盘"路径**不存在**任何驱动代码。

### 路径⑤：`trading_system/`（`rec_t_strategy.py` + Python 侧 NautilusTrader 集成）

`trading_system/strategy/rec_t_strategy.py` 是 `RecTStrategy`（NautilusTrader `Strategy` 子类，
包 `newchan_rust.RecTStream` 递归 T 引擎，逐 bar 输出目标净敞口，NETTING 提单）。这是 issue 票面
点名的入口。

- **能否构建**：Python，无编译。import 链依赖 `newchan_rust`（PyO3 扩展）+ `nautilus_trader`
  （Python 包）。两者在共享 `.venv` 里都能 import 成功（`newchan_rust.__file__` /
  `nautilus_trader.__file__` 均解析到真实 site-packages）。**注意**：这是主仓 `.venv` 预装的
  扩展，不是从 `/tmp` 快照现编的——`/tmp` 快照本身没有自带 venv，本次没有为它单独 `pip install
  ./rust`（会显著超预算），故"能构建"这一档严格说是"（用既有构建产物）能 import"，不是本次从
  `/tmp` 快照源码现编出来的验证。
- **能否跑**：
  ```
  $ .venv/bin/python3 trading_system/backtest/rec_backtest.py --bars 1000
  数据文件不存在: .../analysis/data_cache/btc_1m_full.json
  ```
  ```
  $ .venv/bin/python3 trading_system/backtest/runner.py --bars 1000
  数据文件不存在: .../.cache/BZ_1min_2024_raw.parquet
  ```
  两条 Python 侧 NT 回测入口（`rec_backtest.py` 用 `RecTStrategy`，`runner.py` 用另一个
  `ChanlunStrategy`）都能把 import 链跑穿、能进到 argparse + 数据加载这一步，**同样卡在数据
  缺失**（`.cache/` 目录在 `/tmp` 快照里不存在）。
- **实盘侧**（`trading_system/live/`）：
  - `live/runner.py`：docstring 自称"阶段4+ 骨架，当前不可运行"，`main()` 里是 TODO 伪代码
    （`TradingNode` 组装从未写出），**代码自己承认没做**。
  - `live/databento_live_runner.py`：docstring 明写"数据-only TradingNode（无 exec client，
    不下单）"——只做行情订阅冒烟，设计上就不下单。
  - `live/hl_verify_nautilus.py`：**这条会真的下单**（Hyperliquid mainnet，post-only 0.001
    BTC 远离 mid 价，验证完自动撤单），但它是纯连接性验证脚本，**没有 import 任何
    `RecTStrategy`/`ChanlunStrategy`**——下的单和缠论引擎的信号无关，不构成"K 线到下单"链条。
- **断点**：回测两条子路径卡数据缺失（未测完）；实盘子路径中两条是设计内"不下单"（骨架/数据-only，
  非缺陷），一条能下单但与引擎信号无关（不算数）。
- **输出形态**：回测子路径——NT `BacktestEngine` 驱动的真实订单流（未验证，因数据缺失未跑到）；
  实盘子路径——无（骨架未写）或与引擎无关（连接验证单）。
- **是否在用**：`trading_system/` 近期提交活跃（recursive_t 系列、#320 数据子集合入），CI 的
  `pytest -m "not slow"` 覆盖 `test_trading_system_skeleton.py`（`pytest.importorskip
  ("nautilus_trader.model")` 门控，测的是杠杆计算等纯函数，不测端到端下单）。

---

## 一句话总判

**现在有 0 条路径能被实测证实"从 K 线走到下单"跑通到底**——路径①在架构上从未接执行层（结构性
断头，非缺数据）；路径②③④共享同一套 rust 引擎，能编译、能起、单元自测 19+4 全绿，但生产入口
（`theta_backtest` CLI）卡在市场数据文件全缺，且路径④（rust 原生实盘）连入口代码都不存在；路径⑤
两条 Python 回测入口同样卡数据缺失，三条实盘子路径中两条是设计内"不下单"骨架、一条能下单但与引擎
信号无关。**最接近能跑通的是路径②③（rust 回测 CLI）**——编译/单测/CI 三项全部实测通过，唯一缺口是
市场数据文件，理论上补上 `analysis/data_cache/btc_1m_full.json` 后大概率能推进到"是否产非空订单流"
这一步（但这本身也是未验证的推测，不算已跑通）。

## 未测完清单（逐条列原因）

1. **路径②③"真实 BacktestEngine 产非空订单流"（L2 验收）**——缺 `analysis/data_cache/
   btc_1m_full.json`（及其余 7 品种数据文件），全仓 `find` 未命中任何可替代的市场数据文件，未
   下载/生成（票面禁止）。
2. **路径⑤两条 Python 回测入口的实际运行结果**——同样缺 `analysis/data_cache/btc_1m_full.json`
   （`rec_backtest.py`）和 `.cache/BZ_1min_2024_raw.parquet`（`runner.py`）。
3. **路径⑤"能否构建"的现编验证**——本次用的是主仓既有 `.venv`（预装 `newchan_rust`/
   `nautilus_trader`），没有对着 `/tmp` 快照源码重新 `pip install ./rust` 现编 PyO3 扩展（预计
   耗时超出本次预算，且会在临时目录产生构建产物，与"只测不改"的最小足迹原则冲突）；因此"构建"
   这一档对路径⑤是间接验证（import 成功⇒历史某版本能建），不是本次从当前 main 源码严格重编。
4. **hl_verify_nautilus.py / hl_verify_sdk.py 的真实连接测试**——需要
   `HYPERLIQUID_PRIVATE_KEY` 环境变量（实盘私钥），未配置、也不应为测试去配置，未跑。
5. **`databento_live_runner.py` 的真实行情流验证**——需要 Databento 实时网关凭证与市场开市窗口，
   未配置未跑（且即便跑通也只是行情订阅计数，不涉及下单，价值有限，故未优先测）。

## 附：实测命令与关键原文（供复核）

- 环境重建：`git worktree add /tmp/e2e-792-main-check main --detach`（main = `f6d000fed2`）
- `cd /tmp/e2e-792-main-check/rust && cargo check --lib` → `Finished dev profile ... in 9.52s`
- `cargo check --lib --features nautilus` → `Finished dev profile ... in 20.37s`
- `cargo build --release --features backtest_bin --bin theta_backtest` → `Finished release
  profile [optimized] target(s) in 46.64s`
- `./target/release/theta_backtest BTC` →
  `数据加载失败: 读取 ".../analysis/data_cache/btc_1m_full.json" 失败: No such file or
  directory (os error 2)`
- `./target/release/theta_backtest ES` → 门控 fail-fast 提示原文（NT 段仅支持 BTC，非 BTC
  未实装 instrument 映射）
- `cargo test --features nautilus --lib theta_v0::nautilus` → `test result: ok. 19 passed; 0
  failed`
- `cargo test --release --features backtest_bin --lib theta_v0::nautilus::backtest_engine` →
  `test result: ok. 4 passed; 0 failed`
- `.venv/bin/python3 trading_system/backtest/rec_backtest.py --bars 1000` →
  `数据文件不存在: .../analysis/data_cache/btc_1m_full.json`
- `.venv/bin/python3 trading_system/backtest/runner.py --bars 1000` →
  `数据文件不存在: .../.cache/BZ_1min_2024_raw.parquet`
- `grep -rln "RecursiveOrchestrator" src/newchan --include="*.py" | xargs grep -ln
  "place_order\|submit_order\|ib_insync\|IB()\|broker"` → 空命中
- `grep -rn "LiveNode" rust/src` → 全仓 0 命中
