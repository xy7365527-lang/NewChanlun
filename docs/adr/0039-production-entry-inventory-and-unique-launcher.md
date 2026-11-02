> 正本：本文件是生产入口清单与唯一正式 launcher 的正本。
> 最后裁定：#1530（2026-10-20）；修订：#1523（2026-10-28）、#1539（2026-11-02），见文末「修订」节
> 受影响代码清单：见「二、生产入口完整清单」节

## 问题

ADR 0027 定了「结构判定域 / 重操作程序」两个互斥的唯一 owner，但只管 owner、依赖方向和运行边界。它明写「本 ADR 不发明类型、API、事件 schema、目录或迁移步骤」（ADR 0027:89）。#1322 全链 scan 的结论是仓内还没有 ADR 0027 的端到端入口（#1323 票面正文第 88 行）。本票（FU-08）要在 #1340 已采纳的 R2 架构下裁三件事。R2 的四个域是：结构域 S、各重账 Bᵢ、经济域 E、唯一执行入口 X。

1. **全仓生产入口完整清单**：分命令、preset、feature、service、直发五类，每条写出终态和失去写权的机制。
2. **唯一正式 launcher 的 seam**：它是哪个进程，装配谁，不拥有什么，怎么接 #1523/#1527 线。
3. **第二写权的锁定与影子标记**：持久域写者、场所凭据、构建面三层各自的锁。

上一稿把 `theta_backtest`（π 回测 CLI）和 `run_theta_v0_pi` 扶正为 launcher。这与 ADR 0027 背景段和裁定五冲突，已被退回（#1530 评论 6276895863）。本稿不把任何现有回测入口、organic 入口或 Python 入口当作 launcher。

## 权威依据

1. **ADR 0027:10**【背景段】：「现有代码分出三条执行线：π 批量生产回测、organic 赋格交易线，以及 paper 的行情、订单和对账线。它们尚未形成一套有唯一 owner 的完整生产程序。若直接把任一现有入口扶正，结构事实、持久操作状态和交易决策仍可能各有第二套生产实现。」
2. **ADR 0027:28**【裁定一表格】：重操作程序唯一拥有「各操作级别旁路、同级别分解调用、向下定位、每重 TStage、0..N Voice、专属筹码、逐重毛账，以及唯一生产交易意图」。**ADR 0027:32**：「两个 owner 组成一套生产程序，不是保留两套交易程序。任何结构事实或操作状态都只能有一个生产 owner。失败必须作为失败处理，不能切到另一套判据、旧入口或宽松路径重新判定。」
3. **ADR 0027:46**【裁定三】：「历史回放、paper 与 live 必须共用同一个结构判定域实现和同一个重操作程序实现。不得为任一运行环境保留特有的结构判定、重状态推进或交易决策程序。」
4. **ADR 0027:62**【裁定五】：「现有 organic 赋格实现是可收编资产，不因本裁定自动成为正本。正确部分进入唯一重操作程序后，旧独立入口必须失去生产决策权或退役。」**ADR 0027:64**：「π 内重复拥有交易、成交或账本状态的入口同样必须迁移或退役。迁移期间不得用双 owner、回写结构主干或失败兜底维持兼容。」**ADR 0027:107**：拒绝「π 和 organic 赋格并行双 owner」，理由是「两条生产路径会重复拥有交易决策或状态，并允许失败时切换判据」。
5. **CONTEXT.md:14-15**【重操作程序词条】：「Classification 下游唯一拥有重操作状态和生产交易意图的领域 owner。它不拥有 Classification，也不重新判定结构事实。」「_Avoid_: 设置第二个重操作 owner；以旧入口或另一套判据接管失败」。
6. **ADR 0028:36**【裁定二】：「作用域取『可下单、不可划转』。载体为专供 testnet 的 API wallet……主钱包私钥不进任何运行时……凭据只记作用域与来源，不入仓、不贴票。」
   **ADR 0028:46**【裁定四机器层】：「正式 launcher 在每次以 `production` 证据类 profile 启动、以及任一失效触发后的首次发送前，核验 TB-09 证据指纹（旧 writer 禁入证据、X 排他锁、profile 身份绑定、凭据作用域版本），与 profile 版本一起持久化；失败即拒绝发送，不回退旧 writer。」
   **ADR 0028:63**：「旧 Python `trading_system/config/broker_config.py` 与 `trading_system/execution/risk_controls.py` 为可收编资产（ADR 0027 裁定五），不自动成为正本。」
   **ADR 0028:22/:25**：profile 字段 `credential_scope`、`evidence_class`。
7. **ADR 0030:44**：「无法连接场所是失败态（保留最大责任、重试、不切判据、不回退旧 writer），不是另一种政策。」
8. **ADR 0038:60**【裁定二】：「**恢复**：launcher 启动时检测上次 checkpoint，从该点加载六面快照并重放至当前 bar」。**ADR 0038:74**【裁定三】：「`evidence_class ∈ {simulation, experiment, production}` 作为 profile 版本字段（ADR 0028 裁定一），实验证据不得读为生产有效，simulation 产出不进 TStage 触闸计数」。
9. **#1340 票面（SPEC，R2 已采纳；行号为 `gh issue view 1340 --json body --jq .body` 输出行号）**
   - :10「R2 架构及其精确交付窗口已采纳」。
   - :22「采用已采纳的 R2：结构域 S 独立产生正式结构，各重账 Bᵢ 独立经营，经济安全域 E 保存共同责任及财务放行，唯一执行入口 X 消费精确命令的一次调用权。」
   - :326 C09.1「首个实现采用同一宿主上的独立进程：S、E、X 和各 Bᵢ 分别拥有独立持久域；前端查询外壳为只读进程。核心判断和经营 reducer 采用 Rust，现有 Python/NT 外壳仅承担模式输入、协议与展示适配。」
   - :332 C09.7「S/E/B 的写事务核数据库持久 writer_epoch……X 是唯一持有场所写凭据并执行网络请求的进程；其他域和旧脚本在获准部署中无该凭据。X 的适配器在 X 内部受控调用，不能委托另一个仍有直发权的异步后台」。
   - :333 C09.8「重启/换代先由部署监督器确认旧进程及发送子进程已退出，并取得持久发送身份和排他进程锁」。
   - :340 C10.1「提供唯一完整会话入口，载入已批准规则/政策、能力 manifest、原始输入、存储/时钟和执行 adapter……单独 π batch、ThetaPiStream 或旧 Python 递归结果仅可标注对应组件证据。」
   - :350 C11.1「实施前形成当前提交及未提交面绑定的逐入口清单：命令、preset、feature、默认启动、导入调用、服务、测试/研究入口、真实发送及其凭据/身份路径」。
   - :351 C11.2「唯一结构核及可复用的严格构造收编；旧 Python 回放/session/API 外壳迁为同核 adapter，内部第二套判定冻结为无生产权影子或删除；旧 π fill/organic/简化策略按对象功能逐项收编或退役，不能整体更名后继续产许可」。
   - :352 C11.3「HL SDK、HL NT、IB 探针和其他直发脚本仅可在独立、明确获准的非生产实验环境使用……研究/测试目录、开关默认 false、手工 y 或 ALO 都不构成唯一发送锁。」
   - :353 C11.4「每个条目仅有收编、冻结影子、删除之一的终态，附替代入口、失去的生产写权、保留用途和退场条件；用途未知先调查，不直接删除。冻结影子只输出明确带 provenance 的比较读数，不能影响准入、资金或订单。」
   - :355 C11.6「……默认 launcher 仍触发旧核时，本组不通过。」
   - :359 C12.1「以最高完整会话入口为主测试缝」。
   - :372 TB-09「受测部署旧写入口禁入、正式 launcher 的 X 独占发送闭环成立」；:373 TB-10「所有旧入口取得终态……默认启动只到新核」。
   - :396 FU-06「R2 组织已采纳；具体技术承载由 C09 提出随本稿审阅」；:397 FU-07「系统风险与结构判据名分已定；完整退出授权及责任退休策略仍需明确」。
   - :409-411 AT-01/02/03；:424 AT-16「每条入口/preset/feature/服务/直发路径有实际终态；默认启动、影子和带仓迁移不能绕过 owner/X」。
10. **签署 SPEC 验收组**：`.chanlun/review-results/issue1339-r2-spec-20260909/view/spec/SPEC.md:422`「| AT-16 | 每条入口/preset/feature/服务/直发路径有实际终态；默认启动、影子和带仓迁移不能绕过 owner/X | C11；Destination 2/3 |」。
11. **冻结旧入口登记**：`.chanlun/review-results/issue1339-r2-spec-20260909/view/sources/newchanlun-1339-architecture-20260908/author/OLD-ENTRY-DISPOSITIONS.md`。
    - :14「45 组含入口、公共 callee、支持、验证和范围边界，不能报道为 45 个可执行入口」。
    - :20-25 共同退出条件 X1–X6，其中 X6「文件删除是后续逐依赖验收事项，不由本候选自动授权」。
    - 本稿引用的组标题：OE01 :31、OE03 :61、OE04 :76、OE05 :91、OE06 :106、OE07 :121、OE08 :136、OE09 :151、OE10 :166、OE11 :181、OE12 :196、OE13 :210、OE14 :227、OE15 :243、OE16 :258、OE17 :272、OE32 :496、OE33 :511、OE35 :541、OE41 :631、OE42 :646、OE45 :689。
12. **#1323 票面**：
    - :32 Destination ②「生产 owner 唯一：……同一判断不存在第二套生产实现或失败兜底」。
    - :33 Destination ③「旧实现全部有终态：每条旧引擎／简化臂逐一落为收编、冻结影子臂或删除；冻结臂不得参与生产准入和交易决策」。
    - :88「结论是仓内尚无 ADR 0027 端到端入口」。
13. **#1413 票面**：
    - :7「默认launcher/preset/feature/service只装配新完整程序」。
    - :12「OE41/OE42/OE45遵OUT_OF_SCOPE_PRESERVE，只保留本图排除边界及原授权/名分，不纳交易退役分母」。
14. **#1383 票面**：
    - :7「正式launcher在具名部署启动时核验旧入口及凭据/身份作用域」。
    - :12「唯一X持写凭据……单靠目录/开关false/手工y/进程epoch不算阻断」。
15. **#1523 票面**：
    - :3「当前 Step 2 产物属于离线 adapter/生命周期候选，允许不消费未完成前置的准备和契约验证；不得绕过 S/E/B/X 或使旧直发路径取得生产权。」
    - :24「正式 launcher 代码落盘（`trading_system/execution/hl_paper_launcher.py` 或类似路径）」。
16. **#1527 票面**：
    - :3「已有 theta_replay 二进制与单日双跑仅为组件证据」。
    - :23「确认正式生产入口」。
17. **分级依据**：`.sandcastle/docs/graph-engineering.md:23`「T 类（技术取舍）：数据结构、接口与 seam、owner 落点、状态机与事件 schema、测试锁、迁移步骤、恢复语义。有权威源或本仓判例可援引。→ 自动裁决。」
18. **`docs/agents/generation-constitution.md:35`**（属「## 2. 现役线名单」:32）：「**π 是唯一现役引擎**。锚定方式 = 家族根函数名（`pi_theta_fill_loop_overlay`），不锚行号（行号随版本漂移）。」
19. **退回重写约束**：#1530 评论 [6276895863](https://github.com/xy7365527-lang/NewChanlun/issues/1530#issuecomment-6276895863)（xy7365527-lang，2026-10-19）的四条约束。分级评论 [6276984750](https://github.com/xy7365527-lang/NewChanlun/issues/1530#issuecomment-6276984750)：「分级：T」。

**仓内现状（main `b63a3fd586`，逐行核实，作为清单的事实依据）**

- **`rust/Cargo.toml` 与 bin 计数**
  - `cargo metadata --no-deps` 共 67 个 bin：
    - `backtest_bin` 门 21 个：`theta_backtest` 加 20 个 `p*`/`theta_*`/`*_bsp_timing` 探针；
    - `s_session` 门 3 个：`s_structure_session`/`e_economic_session`/`b_chong_session`，见 :81/:248/:253；
    - `lav_seal` 门 1 个：`lav_seal_samples`；
    - 无 required-features 42 个。
  - :50 `default = []`。
  - :54 注释「CLI 生产入口：解 theta_v0::backtest 的 #[cfg(test)] 门控为公开 API，链接 run_theta_v0_pi/load_by_symbol」。
  - :56 `backtest_bin = ["nautilus", "dep:sha2"]`。
  - :70-73 `theta_backtest` 门 `required-features = ["backtest_bin"]`。
- **π 线**
  - `rust/src/bin/theta_backtest.rs`：:17「本 CLI 是**最小生产路径**：用 in-crate 引擎（`run_theta_v0_pi` = π_Θ 七链 + 内部 fill 模拟）」；:30 `use newchan_rust::theta_v0::backtest::runner::run_theta_v0_pi;`；:106 `let r = run_theta_v0_pi(&dataset, &config, years, initial_nav);`。
  - `theta_v0/backtest/fill.rs`：:746/:791/:4767 `pub(super) fn pi_theta_fill_loop` / `_voice` / `_overlay`。
  - `theta_v0/backtest/runner.rs:250` `pub fn run_theta_v0_pi(`。
  - `theta_v0/stream.rs:117` `pub fn push_bar(&mut self, bar: Bar, p_t: f64, nav: f64) -> f64 {`。
  - `theta_v0/ffi.rs:59-61` `#[pyclass(name = "ThetaStream", unsendable)] pub struct PyThetaStream { core: ThetaPiStream,`。
  - `theta_v0/nautilus/mod.rs`：:69-70 `#[cfg(feature = "nautilus")] pub mod theta_strategy;`；:76-77 同门 `theta_pi_strategy`；:82-83 `#[cfg(all(feature = "nautilus", feature = "backtest_bin"))] pub mod backtest_engine;`。
  - `theta_strategy.rs:136` 与 `theta_pi_strategy.rs:161` 均为 `self.submit_order(order, None, None, None)`。
  - `backtest_engine.rs:141` `pub fn run_theta_backtest(`。
  - `order_adapter.rs:23` 文档行 `//! self.submit_order(order, None, None)?;`。
- **organic 线**
  - `rust/src/trading/runner.rs:313` `pub fn run_organic(`；`positional.rs:1066` `pub fn run_positional(`；`recursive_position.rs:140` `pub fn run_recursive(`。
  - `rust/src/lib.rs:2810-2837` `fn newchan_rust`（PyO3 模块）导出：
    - :2811-2815 `PyOrganicTape`/`run_organic_rust`/`run_recursive_rust`/`run_positional_rust`/`PyUnnStream`；
    - :2817-2818 spiral；:2820-2821 fugue_v3；
    - :2822-2832 recursive_t 三项，各带 `#[cfg(not(test))]`；
    - :2834 `PyThetaStream`；
    - :2835-2837 `PyBiEngine`/`PyOnlineMacdState`/`PyRecursiveOrchestrator`。
  - `lib.rs:948` `#[pyclass(name = "RecursiveOrchestrator")]`。
- **R2 正式线**
  - `rust/src/bin/s_structure_session.rs:25-28`「## 生产零调用纪律（不变）……不写第二份生产判定器；不调用 `ThetaPiStream` 资金推进冒充 S；不启动 E/B/X。」
  - `e_economic_session.rs` 与 `b_chong_session.rs` 各 7 行，`main` 分别调 `newchan_rust::economic_session::cli(false)` / `cli(true)`。
  - `economic_session/service.rs`：:565 `pub fn cli(is_b: bool) -> Result<(), String> {`；:595 `let _lock = process_lock(&config.db, &config.domain)?;`。
  - `session_transport.rs`：:40 `pub fn process_lock(`；:53 锁文件 `{}.writer.lock`；:62 取锁失败 `format!("StaleWriter：{domain_name}独占进程锁不可得：{e}")`。
  - `economic_session/store.rs:466-467` epoch 不等时返回 `"StaleWriter：持久epoch不等于获准写者"`。
  - 仓内**没有** X 进程，也没有 `production_session`：`rg -l "x_execution_session|production_session|唯一执行入口 X|X 独占" rust/src trading_system scripts` 零命中。
- **service / preset**
  - `pyproject.toml:34-35` `newchan = "newchan.cli:main"`。
  - `src/newchan/cli.py`：:42 `chart` 子命令；:325/:327 `run_server(port=args.port)`。
  - `src/newchan/server.py`：:420 `def api_newchan_overlay():`；:617 `def run_server(port: int = 8765, ...)`。
  - `src/newchan/gateway.py`：:855 `def _ensure_live_engine(symbol: str) -> RecursiveOrchestrator:`；:946 `async def ws_live(`。
  - `scripts/dev.ps1`：:30 `python -m newchan.cli serve --port 8765`；:38 `uvicorn newchan.gateway:app ... --port 8766`。
  - `scripts/realtime_server.py`：:25 import `RecursiveOrchestrator`；:67 构造它。
- **Python `trading_system/`**
  - `README.md:12/:15/:18` 三条运行命令：`backtest/runner.py`、`--enable-orders --db /tmp/chanlun_bt.db`、`live/runner.py`。
  - `backtest/runner.py:8-9`：BacktestEngine → `ChanlunStrategy.on_bar` → Rust 引擎。
  - `live/runner.py`：:61 `print("实盘入口为阶段4骨架，尚未实装。…")`；:69 `return 1`。
  - `live/databento_live_runner.py:3`「数据-only TradingNode（无 exec client，不下单）」。
  - 下单与凭据相关行：
    - `strategy/chanlun_strategy.py:20-21` import `LmtExecutor`/`MakerOptimizer`；
    - `execution/lmt_executor.py:70` `self._strategy.submit_order(order)`；
    - `execution/maker_optimizer.py:89` `self._strategy.cancel_order(order)`；
    - `strategy/theta_rec_strategy.py:123` `self.submit_order(order)`；
    - `config/broker_config.py:79` `private_key=os.environ.get("HYPERLIQUID_PRIVATE_KEY")`；
    - `execution/risk_controls.py:1`「PreTradeRiskGate —— paper 试运行风控四条」。
  - 研究回测脚本：
    - `backtest_t_fugue.py:1` `# FROZEN(#951)：绑 recursive_t.TFugueStream…`；
    - `backtest_fugue_v3.py:1`、`backtest_unn.py:1`、`backtest_unn_stream.py:1`、`backtest_spiral_stream.py:1`、`compare_spiral_unn.py:1`、`analyze_fugue_v3_losses.py:2`，均为 NT 回测或对账研究 docstring；
    - `scripts/batch_backtest.py:2`「批量回测 — 多标的并行运行 BacktestEngine」；
    - `scripts/av_recursive_run.py:2`「AV 长历史 5min 自下而上递归」。
- **直发**
  - `trading_system/live/hl_verify_sdk.py`：:6「order —— LMT 买单 0.001 BTC @ 远低于市价（tif=Alo 双保险）→ 立即撤单」；:87 `pk = os.environ.get("HYPERLIQUID_PRIVATE_KEY")`。
  - `hl_verify_nautilus.py`：:6「post-only LMT 买单 @ mid×0.5 → ACCEPTED → 撤单」；:128 读同一变量。
  - `analysis/buy_28316_warrant.py`：:21 `PORT = 7496          # TWS live`；:170 `trade = ib.placeOrder(contract, order)`；:197 `ib.cancelOrder(order)`。
- **写权闭包**：`rg -l 'HYPERLIQUID_PRIVATE_KEY|placeOrder\(|submit_order\(|cancel_order\(' trading_system src scripts analysis rust/src tests --glob '*.py' --glob '*.rs'` 恰好命中 10 个文件，下文直发、π、Python 组已逐一列出。其他接 TWS 7496 的脚本只读行情，不在命中集内：`scripts/tws/fetch_1m_tws.py:38`、`analysis/_tencent_warrants_quotes.py:61` 等。
- **交易外常驻服务**：
  - `.sandcastle/launchd/*.plist` 五个；
  - `scripts/fengliang.service:9`、`scripts/openclaw.service:10`；
  - `topological-computation/topo-daemon.service:9`；
  - `external/setup-c/com.junyu.k4-weekly.plist`。
  - 上述服务都不在写权闭包内。
- **前人勘查**：`.chanlun/review-results/issue1340-publication-20260909/payload/inputs/TB01-ENTRY-RECON.md:31`「前者TradingNode只在docstring，实际:61报尚未实装且:69返回1；后者没有S会话命令。需要真实外部S会话launcher。」

## 候选

**候选 A（采纳）：新立 R2 正式会话 launcher 作为唯一入口，旧线逐入口定终态，写权分三层锁。**
新建一个 Rust 二进制 `production_session`，挂在新 feature `production_session` 下，作为仓内唯一正式 launcher。它载入一份获准 manifest 和一份 profile，以独立进程启动 S、E、各 Bᵢ、X，只把场所写凭据交给 X。启动和换代时核 TB-09 指纹（ADR 0028:46）和 checkpoint（ADR 0038:60）。现有三个会话 bin 保留为域进程，不是 launcher。X 新立为 `x_execution_session`。π、organic、Python `trading_system/` 三条线的每个入口按 C11.4 三态落终态。写权锁分三层：持久域的 `.writer.lock` 加 `writer_epoch`；场所凭据只进 X；生产 feature 不含 `backtest_bin` 和 `nautilus`。

**候选 B（拒绝）：把 `theta_backtest` + `run_theta_v0_pi` 扶正为 launcher。**
这是上一稿的方案。`theta_backtest.rs:17` 自述「π_Θ 七链 + 内部 fill 模拟」；`pi_theta_fill_loop*` 同时拥有交易、成交和账本状态，正是 ADR 0027:64 点名的入口。ADR 0027:10 明禁直接扶正；C10.1 只允许「单独 π batch、ThetaPiStream」标注组件证据。`Cargo.toml:54` 把 `backtest_bin` 自称为「CLI 生产入口」，这只是旧注释，不构成 owner 裁定。

**候选 C（拒绝）：把 Python `trading_system/live/runner.py`（NT TradingNode）扶正为 launcher，Rust 经 FFI 挂入。**
`live/runner.py:61/:69` 是打印诊断后 `return 1` 的骨架。C09.1 规定核心判断和经营 reducer 用 Rust，Python/NT 外壳只做适配；C11.2 要求旧 Python 外壳「迁为同核 adapter」。扶正它等于让 adapter 层持有装配权和凭据，违反 C09.7。

**候选 D（拒绝）：允许多个 launcher（回测、paper、live 各一个）加协调锁。**
违反 ADR 0027:46「不得为任一运行环境保留特有的……交易决策程序」和 ADR 0027:32「不是保留两套交易程序」。协调锁本身就是 ADR 0027:64 所禁的「用双 owner……维持兼容」，也是 ADR 0027:107 已拒绝的并行双 owner。

## 裁定

采纳候选 A。本裁定只做接口 seam、owner 落点和迁移步骤的技术取舍，依据全部来自 ADR 0027/0028/0030/0038、CONTEXT.md 和 #1340 已采纳合同。不改 Destination，不裁任何资金、场所、账户或权限数值。凭据作用域与来源沿用 ADR 0028 裁定二。

正本落点：docs/adr/0039-production-entry-inventory-and-unique-launcher.md

ADR 0039 的编号按 main 顺序分配：落地时取 `docs/adr/` 现有最大编号加 1，当前最大为 0038。文件首三行按 ADR 0038 先例写正本头（见本文件首三行）。

### 一、「生产入口」的定义与五类口径

**生产入口**：任何能让进程取得下列三种写权之一的可执行路径。

- (a) S/E/B/X 任一持久域的写者身份，即 `writer_epoch` 加 `.writer.lock`；
- (b) 场所写凭据，即能发出网络写请求；
- (c) 生产交易意图的产生权（ADR 0027:28）。

只读观察、研究读数、测试驱动本身不是生产入口。但凡能触到 (a)(b)(c) 之一的路径，清单里都要有终态。

五类按 C11.1 口径划分：

- **命令**：可执行文件或脚本的 `main`；
- **preset**：默认参数、`project.scripts`、`dev.ps1` 和 README 运行命令；
- **feature**：Cargo feature 与 `required-features`、PyO3 导出；
- **service**：常驻 HTTP、WS 或 daemon；
- **直发**：任何持凭据并能对场所发写请求的路径。

**终态口径**：旧入口只取 C11.4 三态之一。

- **收编**：进入唯一 owner、X 或只读 adapter，失去独立写权。
- **冻结影子**：保留文件，只产带 provenance 的比较读数，不得触及 (a)(b)(c)。
- **删除**：执行由 FU-12 逐依赖验收，见 X6；本 ADR 不自动授权删文件。

本 ADR 定的是**入口**的终态。入口内部「正确部分」逐函数收编进哪个 owner（ADR 0027:62），属于模块级判定，归 FU-12（见裁定五）。模块级收编不改变入口终态，也不给入口带回写权。

### 二、生产入口完整清单（main `b63a3fd586`）与逐入口终态

**R2 正式线（唯一生产程序的组件，不适用旧入口三态）**

| 入口 | 类 | 地位 | 写权机制 |
|---|---|---|---|
| `production_session`（新立，`rust/src/bin/production_session.rs`） | 命令+feature | **唯一正式 launcher**（见裁定三） | 本身不持任何域写者身份，不持凭据，不产交易意图 |
| `s_structure_session`（`Cargo.toml:78-81`） | 命令+feature `s_session` | 收编：S 域进程（结构判定域） | 自持 `.writer.lock`（`session_transport.rs:53`）+ 持久 `writer_epoch`；生产部署中只由 launcher 拉起 |
| `e_economic_session`（`:245-248`） | 命令+feature `s_session` | 收编：E 域进程 | 同上（`service.rs:595`、`store.rs:466-467`） |
| `b_chong_session`（`:250-253`，`--chong ID`） | 命令+feature `s_session` | 收编：每重一个 Bᵢ 进程（重操作程序） | 同上 |
| `x_execution_session`（新立，`rust/src/bin/x_execution_session.rs`，`required-features = ["production_session"]`） | 命令+feature | 唯一执行入口 X（C09.7） | 唯一接收凭据的进程；自持 `.writer.lock` + `writer_epoch` |

**#1657 S 准入操作员入口**（资源准入，不授交易发送权；合同与签撤程序见 [ADR 0050](0050-s-resource-admission-contract-v1.md)）

| 入口 | 类 | 地位 | 写权机制 |
|---|---|---|---|
| `s_structure_session admission-prepare` | 命令+feature `s_session` | 正式签发资源锚；当前未测合同拒签 | 同服务 uid、内嵌已审版本、四锁、无既存锚才签发；不写数据库 |
| `s_structure_session admission-revoke` | 命令+feature `s_session` | 正式显式撤销并留档 | 同服务 uid、四锁、原对象身份匹配；不接管活写者 |
| `s_structure_session admission-contract` | 命令+feature `s_session` | launcher 写动作前的只读合同门 | 共用签发注册表，核版本/摘要/启动窗口；无域写权 |

**π 线（`theta_v0`）**

| 入口 | 类 | 终态 | 失去写权的机制 | OE |
|---|---|---|---|---|
| `theta_v0::parser::ParseLayerIncr`、`theta_v0::classifier::*` | feature（库） | 收编 → 结构判定域 S（`s_structure_session.rs:27` 已接通） | 只在 S 进程内调用；S 不调 `ThetaPiStream`（同文件 :28） | OE33 |
| `theta_backtest`（`theta_backtest.rs:30/:106` → `runner.rs:250` `run_theta_v0_pi`） | 命令+feature `backtest_bin` | 冻结影子：π 批量对照臂，只出组件证据（C10.1） | `backtest_bin` 不进 `production_session` feature；无 manifest、无凭据、不开任何域库 | OE32/OE35 |
| `fill.rs:746/:791/:4767` `pi_theta_fill_loop{,_voice,_overlay}` | feature（库内部，`pub(super)`） | 冻结影子：π 内交易、成交、账本状态（ADR 0027:64） | 仅被 `backtest_bin` 门内 runner 调用；可见性保持 `pub(super)` 不扩 | OE32 |
| 其余 20 个 `backtest_bin` 门内探针 bin | 命令 | 冻结影子（研究/证据 bin） | 同 `theta_backtest` | OE35 |
| 42 个无 required-features 的 bin、`lav_seal_samples` | 命令 | 冻结影子（研究/证据 bin） | 不持 manifest、凭据、`.writer.lock`；不在生产二进制集合内（裁定四·3） | OE35 |
| `stream.rs:117` `ThetaPiStream::push_bar` + `ffi.rs:59-61` `PyThetaStream`（`lib.rs:2834` 导出） | feature（PyO3） | 冻结影子 | 生产进程不加载 `newchan_rust` cdylib（裁定四·3） | OE08 |
| `theta_strategy.rs:136`、`theta_pi_strategy.rs:161` `submit_order`；`backtest_engine.rs:141` `run_theta_backtest`；`order_adapter.rs` | feature `nautilus` | 冻结影子：Nautilus ThetaStrategy 旧决策链 | `nautilus` 不进 `production_session` feature ⇒ 由 `nautilus/mod.rs:69-83` 的 cfg 保证不编入生产二进制 | OE09 |

**organic 线（`rust/src/trading` 与同族 FFI）**

| 入口 | 类 | 终态 | 失去写权的机制 | OE |
|---|---|---|---|---|
| `runner.rs:313` `run_organic`、`positional.rs:1066` `run_positional`、`recursive_position.rs:140` `run_recursive`，导出 `lib.rs:2811-2815`（`PyOrganicTape`、`run_organic_rust`、`run_recursive_rust`、`run_positional_rust`、`PyUnnStream`） | feature（PyO3） | 冻结影子：organic 家族对照臂 | 生产进程不加载 cdylib；批量函数只读 tape、只回结果，不开域库，不持凭据 | OE15/16/17 |
| `lib.rs:2817-2821` spiral / fugue_v3 导出：`PySpiralStream`、`run_spiral`、`PyFugueV3Stream`、`run_fugue_v3` | feature（PyO3） | 冻结影子 | 同上 | OE10/12 |
| `lib.rs:2822-2832` recursive_t 三项（`#[cfg(not(test))]`）：`run_recursive_t`、`PyTFugueStream`、`PyRecStream` | feature（PyO3） | 冻结影子 | 同上 | OE11 |
| `lib.rs:948`/`:2837` `PyRecursiveOrchestrator`、`:2835-2836` `PyBiEngine`/`PyOnlineMacdState` | feature（PyO3） | 冻结影子：旧递归判定核，不得成为 S 之外的第二份 Classification | 同上；Python 调用者见 service 组 | OE06 |

**Python `trading_system/` 线**

| 入口 | 类 | 终态 | 失去写权的机制 | OE |
|---|---|---|---|---|
| `trading_system/backtest/runner.py`（README:12/:15，含 `--enable-orders --db`） | 命令+preset | 冻结影子：NT 回测研究壳 | 不读凭据；`--db` 是其私有 SQLite，不是任何 R2 域库（`s_structure_session.rs:29`） | OE13 |
| `trading_system/live/runner.py`（README:18；:61/:69 骨架） | 命令+preset | **删除** | 删除前不得实装为任何 launcher；删除由 FU-12 逐依赖验收后执行（X6） | OE13 |
| `live/databento_live_runner.py`（:3 数据-only，无 exec client） | 命令 | 冻结影子：行情冒烟 | 无 exec client，不读场所写凭据 | OE13 |
| `strategy/chanlun_strategy.py`（:20-21）+ `execution/lmt_executor.py:70` + `execution/maker_optimizer.py:89` | feature（NT 策略） | 冻结影子：策略本体的经济判断退出生产；LMT-only 和 maker 撤单逻辑能否逐函数收编进 X adapter，归 FU-12 | 生产部署宿主无场所凭据（裁定四·2）；不得挂到任何生产 node | OE07 |
| `strategy/theta_rec_strategy.py:123` | feature（NT 策略） | 冻结影子 | 同上 | OE08 |
| `config/broker_config.py:79`、`execution/risk_controls.py:1` | preset/feature | 冻结影子：可收编资产，不自动成为正本（ADR 0028:63）；逐函数收编去向归 FU-12 | 生产进程不 import；凭据只经 launcher 注入 X（裁定三·4） | OE07/OE13 |
| `backtest_fugue_v3.py`、`backtest_t_fugue.py`（:1 FROZEN(#951)）、`backtest_unn.py`、`backtest_unn_stream.py`、`backtest_spiral_stream.py`、`compare_spiral_unn.py`、`analyze_fugue_v3_losses.py`、`scripts/batch_backtest.py`、`scripts/av_recursive_run.py` | 命令 | 冻结影子（研究回测） | 不读凭据，不开域库 | OE10/11/12/35 |

**service / preset 类**

| 入口 | 类 | 终态 | 失去写权的机制 | OE |
|---|---|---|---|---|
| `newchan` CLI（`pyproject.toml:35`）→ `chart`（`cli.py:42/:327`）→ `server.py:617` Bottle 8765；`dev.ps1:30` | preset+service | 收编：只读查询壳 adapter（C09.1、C11.2），改读 S 发布 | 永不持 manifest 和凭据，不开任何域库写者 | OE01/OE04 |
| `server.py:420` overlay 内重算、`gateway.py:855` 进程内 `RecursiveOrchestrator`、`src/newchan/orchestrator/recursive.py` | feature（库） | 冻结影子：外壳内部第二套判定（C11.2） | 只能输出带 provenance 的比较读数；查询壳收编后不得以它作为发布源 | OE03/OE05 |
| `gateway.py:946` `ws_live`（ASGI 8766）；`dev.ps1:38` | service+preset | 收编：只读查询壳 adapter | 同 `newchan` CLI | OE03 |
| `scripts/realtime_server.py:67`（进程内 `RecursiveOrchestrator`） | service | 冻结影子 | 不持凭据，不开域库 | OE04 |
| `.sandcastle/launchd/*.plist`、`scripts/fengliang.service`、`scripts/openclaw.service`、`topological-computation/topo-daemon.service`、`external/setup-c/com.junyu.k4-weekly.plist` | service | 不是生产入口：交易外工具和 topo/K4，不在写权闭包内，按 OUT_OF_SCOPE_PRESERVE 不纳入分母（#1413:12） | — | OE41/42/45 |

**直发类**

| 入口 | 类 | 终态 | 失去写权的机制 | OE |
|---|---|---|---|---|
| `trading_system/live/hl_verify_sdk.py`（:6 order 阶段，:87 读私钥） | 直发 | 冻结影子：只能在独立、明确获准的非生产实验环境使用（C11.3） | 生产部署的宿主环境、S/E/B 子进程和查询壳都没有 `HYPERLIQUID_PRIVATE_KEY`；TB-09-A #1383 用它作被拒负对照 | OE14 |
| `trading_system/live/hl_verify_nautilus.py`（:6，:128） | 直发 | 冻结影子 | 同上 | OE14 |
| `analysis/buy_28316_warrant.py`（:21 TWS 7496，:170 `placeOrder`，:197 `cancelOrder`） | 直发 | 冻结影子：人工维护工具 | 生产部署不启 TWS 7496；任何实际外部动作另经授权 | OE14 |

**完整性判据**：清单的完整性按写权定义机械闭合，不按目录列举。

- 能读场所凭据或调用下单、撤单 API 的文件集合，必须恰好等于上表点名的 10 个文件（验收 A4）；
- 能持域写者身份的只有 R2 正式线五个 bin；
- 能产交易意图的旧路径，已按 π/organic/Python 三线全部列出。

用这个判据，是为了不把「目录外形」当分类维度（AGENTS.md #804 ①）。

### 三、唯一正式 launcher 的 seam

1. **身份**
   - bin `production_session`，源文件 `rust/src/bin/production_session.rs`；
   - Cargo feature `production_session`，含 `s_session` 与 X adapter 所需依赖，**不含** `backtest_bin` 和 `nautilus`；
   - 全仓只有这一个 launcher。`theta_backtest`、`trading_system/live/runner.py`、`newchan` CLI、任何 `*_launcher.py` 都不是 launcher。
2. **输入**
   - 一份 manifest：已批准规则和政策版本、能力 manifest、原始输入来源、各域 DB 路径、时钟、adapter 选择（C10.1）；
   - 一份 profile：ADR 0028 裁定一字段。
   - `evidence_class` 只决定 adapter 装配：`simulation` 装受控撮合，`experiment` 和 `production` 装场所 adapter。三种运行环境只在 adapter 处分叉（ADR 0027:46）。
3. **装配**
   - 以独立 OS 进程启动 S、E、X、各 Bᵢ（C09.1）；每个域有自己的 `--db`、`.writer.lock`、`writer_epoch`。
   - launcher 只负责启动、顺序、监督、换代和指纹计算。它不持任何域的写者身份，不产生交易意图。
   - 生产部署中，域 bin 只能由 launcher 拉起。手工用生产库路径启动域 bin 的情形，是 TB-09 部署隔离（#1383/#1401）的验收对象，本 ADR 不把「约定」算作锁。
4. **凭据**：launcher 从 profile 的 `credential_scope` 指向的来源读取凭据，**只**写进 X 子进程的启动环境。S、E、Bᵢ、查询壳和宿主 shell 环境都不含凭据（ADR 0028:36、C09.7）。
5. **启动门**
   - `production` profile 下，launcher 在启动时和失效触发后计算 TB-09 指纹，交给 X。
   - X 把指纹连同 profile 版本持久化到自己的域库；指纹缺失或不符就拒绝发送，不回退旧 writer（ADR 0028:46、ADR 0030:44）。
   - 启动时检测 checkpoint 并恢复（ADR 0038:60）。
6. **换代**：先确认旧 X 和它的发送子进程已退出，再提升 `writer_epoch`、取得排他进程锁（C09.8）。旧 epoch 的写入被 `store.rs:466-467` 拒绝。
7. **与 #1523/#1527 的对接**
   - #1523 的「正式 paper launcher」交付物按本 ADR 读作：`production_session` 与 `x_execution_session` 骨架，加 X 内的 HL adapter。
   - #1523:24 写的「`hl_paper_launcher.py` 或类似路径」如果落成 Python 文件，只能是被 X 进程内部调用的 adapter 模块，不得有可独立运行的 `__main__` 发送路径（C09.7「X 的适配器在 X 内部受控调用」）。
   - #1527:23「确认正式生产入口」的答案是：S 输入侧在 `production_session` 下的路径。它已有的 `theta_replay` 收据仍只算组件证据（#1527:3）。
   - 因此 #1527 的入口路径、二进制身份和恢复收据都要等本 ADR 落地、且 `production_session` 由 #1523 交付后才能产出：#1527 被 #1530 与 #1523 阻塞（原生边已接，见「受影响票与 blocking 边变化」）。
8. **最高测试缝**：AT-01/02/03/16 以 `production_session` 拉起的独立进程为最高缝（C12.1）。直接调用 `s_structure_session` 等域 bin 只算组件证据。

### 四、第二写权的锁定与影子标记（三层缺一不算锁）

1. **持久域层**：S/E/B/X 每个域一个库、一个 `.writer.lock`（`session_transport.rs:53`）。第二个写者进程取锁失败时，以 `StaleWriter：<域>独占进程锁不可得` 退出（`:62`）。持久 `writer_epoch` 不等时被拒（`store.rs:466-467`）。影子臂不持有任何域库的 epoch 和锁，物理上无法写入任何域库，这是「失去写权」的第一义。
2. **凭据层**：场所写凭据只存在于 X 进程。C11.3 已定「研究/测试目录、开关默认 false、手工 y 或 ALO 都不构成唯一发送锁」，所以 `lmt_executor.py` 的 LMT-only、`hl_verify_sdk.py:6` 的 Alo 双保险都不算锁，只有凭据不在场才算。
3. **构建层**
   - 生产二进制集合 = `cargo build --release --features production_session` 产出的 `production_session`、`s_structure_session`、`e_economic_session`、`b_chong_session`、`x_execution_session`。
   - `production_session` feature 不含 `backtest_bin` 和 `nautilus`，因此 `theta_backtest` 等 21 个门内 bin 不进生产部署，`theta_strategy`/`theta_pi_strategy`/`backtest_engine` 不编入生产二进制（`nautilus/mod.rs:69-83`）。
   - 生产进程不加载 `newchan_rust` cdylib，所以 `lib.rs:2810-2837` 导出的旧引擎没有进入生产进程的路径。
4. **影子标记**：影子臂输出必须带 provenance（core id、run id、输入 hash；C11.4）。它的产物不得被 E/B 读为事实，不进 TStage 触闸计数（ADR 0038:74）。标记载体是本 ADR 清单加 FU-12 的模块级清单，不另建台账。
5. **与 `generation-constitution.md:35` 的关系**：该条在「现役线名单」下，管的是代码谱系名分（π 相对 v1/dual），不是生产 owner 裁定。π 的结构核收编为 S 的唯一计算者，与它一致。π 回路的经济部分按 ADR 0027:64 冻结为影子。锚点 `pi_theta_fill_loop_overlay` 的名分更新，和 `Cargo.toml:54`「CLI 生产入口」旧注释的改写，归 FU-12 落模块级终态时一并提交，注释须带票号（AGENTS.md #793）。本 ADR 不改这两个文件。
6. 本 ADR 不新增任何同一判断的宽严两档。影子臂不接在生产路径的任何 if-else 上（#799 判别式），失败只能是失败（ADR 0027:32），不设 legacy 开关。

### 五、与 FU-12 #1532 的分工

| | FU-08（本 ADR） | FU-12 #1532 |
|---|---|---|
| 粒度 | 入口级：命令、preset、feature、service、直发 | 模块级：旧引擎和简化臂内部函数、FFI 导出逐项 |
| 产出 | 五类清单、每个入口唯一终态、失去写权的机制、launcher seam、三层锁 | 每个入口的替代入口细节、逐函数收编去向（落进哪个 owner）、退场条件与时间表、删除执行（含 `live/runner.py`）、`generation-constitution.md:35` 锚点与 `Cargo.toml:54` 注释更新 |
| 改动权限 | 定终态 | 只能把「冻结影子」收窄为「删除」，不得让任何入口重新取得 (a)(b)(c) |
| 默认启动不触发旧核 | 定义：默认启动 = `production_session` | 提供逐条清单；由 TB-10-I #1413 按清单逐条核 |
| 带仓迁移 | 不涉及 | 不涉及，归 FU-13 #1535 |

### 六、迁移步骤（顺序固定，不跳步）

1. 落 ADR 0039；#1523 按裁定三交付 `production_session` 与 `x_execution_session` 骨架。这一步可以无凭据 dry-run，属于 #1523:3 允许的离线 Step 2。
2. TB-09-A #1383：在受测 profile 下，以 `hl_verify_sdk.py` 为负对照，证明生产部署拒绝旧直发；跑通第二写者拒绝和凭据只在 X 两条测试。
3. FU-12 #1532 落模块级终态清单；TB-10-I #1413 按清单逐条核默认启动只到新核。
4. FU-13 #1535 定带仓迁移和回退；TB-10 #1368 执行切换，先禁旧发送，后导入新核（C11.5）。
5. 任何一步失败都按失败处理，不回退旧 writer（ADR 0027:32、ADR 0030:44）。

### 七、票面验收第 4 项的承接

票面「AT-01/02/03/16 独立进程通过」不能作为关本票的条件：#1530 原生 blocking #1360（TB-02，承接 AT-01/02/03）和 #1368（TB-10，承接 AT-16），以它为关票条件会形成循环。本 ADR 把它定为下游验收（验收 B14–B16），由保留的 blocking 边承接。

## 验收清单

**A. 本裁定落地（关本票条件；命令在仓库根目录执行）**

1. 文件存在：`find docs/adr -name '*-production-entry-inventory-and-unique-launcher.md' | wc -l | tr -d ' '` 输出 `1`。
2. 可执行命令，输出为空：
   ```
   f=$(find docs/adr -name '*-production-entry-inventory-and-unique-launcher.md'); for k in production_session x_execution_session s_structure_session e_economic_session b_chong_session theta_backtest pi_theta_fill_loop ThetaPiStream PyThetaStream theta_pi_strategy run_organic run_positional run_recursive PyFugueV3Stream PySpiralStream PyRecStream PyRecursiveOrchestrator 'trading_system/backtest/runner.py' 'trading_system/live/runner.py' databento_live_runner.py chanlun_strategy.py lmt_executor.py maker_optimizer.py theta_rec_strategy.py broker_config.py risk_controls.py hl_verify_sdk.py hl_verify_nautilus.py buy_28316_warrant.py realtime_server.py gateway.py dev.ps1 OUT_OF_SCOPE_PRESERVE; do rg -q -F -- "$k" "$f" || echo "MISSING $k"; done
   ```
3. 可执行命令：`rg -c '^> 正本：本文件是' "$(find docs/adr -name '*-production-entry-inventory-and-unique-launcher.md')"` 输出 `1`。
4. 可执行命令（写权闭包锁定当前提交），输出为空：
   ```
   diff <(rg -l 'HYPERLIQUID_PRIVATE_KEY|placeOrder\(|submit_order\(|cancel_order\(' trading_system src scripts analysis rust/src tests --glob '*.py' --glob '*.rs' | sort) <(printf '%s\n' analysis/buy_28316_warrant.py rust/src/theta_v0/nautilus/order_adapter.rs rust/src/theta_v0/nautilus/theta_pi_strategy.rs rust/src/theta_v0/nautilus/theta_strategy.rs trading_system/config/broker_config.py trading_system/execution/lmt_executor.py trading_system/execution/maker_optimizer.py trading_system/live/hl_verify_nautilus.py trading_system/live/hl_verify_sdk.py trading_system/strategy/theta_rec_strategy.py)
   ```
5. 可执行命令（bin 门控快照）：
   ```
   cd rust && cargo metadata --no-deps --format-version 1 --offline | jq -c '[.packages[] | select(.name=="newchan_rust") | .targets[] | select(.kind[]=="bin") | ((.["required-features"] // []) | join(","))] | group_by(.) | map({(.[0]): length}) | add'
   ```
   输出 `{"":42,"backtest_bin":21,"lav_seal":1,"s_session":3}`。
6. 可执行命令：
   ```
   cd rust && cargo metadata --no-deps --format-version 1 --offline | jq -r '.packages[] | select(.name=="newchan_rust") | .targets[] | select(.kind[]=="bin") | select(.name=="theta_backtest") | .["required-features"][]'
   ```
   输出 `backtest_bin`。
7. 票状态：`gh api repos/xy7365527-lang/NewChanlun/issues/1413/dependencies/blocked_by --jq 'map(.number)|contains([1532])'` 输出 `true`。
8. 票状态：`gh api repos/xy7365527-lang/NewChanlun/issues/1383/dependencies/blocked_by --jq 'map(.number)|contains([1523,1530])'` 输出 `true`。
9. 票状态：`gh api repos/xy7365527-lang/NewChanlun/issues/1530/dependencies/blocking --jq 'map(.number)|sort|join(",")'` 输出 `1360,1365,1368,1369,1383,1525,1527,1531,1532,1533,1551`。
9a. 票状态：`gh api repos/xy7365527-lang/NewChanlun/issues/1527/dependencies/blocked_by --jq 'map(.number)|contains([1523,1530])'` 输出 `true`。

**B. 本裁定要求的机械锁（由列明的下游票验收，判据同样二值；不作关本票条件）**

10. 可执行命令（#1523）：`cd rust && cargo build --release --features production_session --bin production_session --bin x_execution_session` 退出码 `0`。
11. 可执行命令（#1523），退出码 `0`：
    ```
    cd rust && cargo metadata --no-deps --format-version 1 --offline | jq -e '.packages[] | select(.name=="newchan_rust") | .features | has("production_session") and ((.production_session // []) | (index("backtest_bin") == null and index("nautilus") == null))'
    ```
12. 测试（#1383）：`cd rust && cargo test --release --features production_session --test production_session_entry second_domain_writer_rejected_by_writer_lock -- --exact` 退出码 `0`。该测试断言：`production_session` 在 simulation profile 下运行时，以同一 E 库路径另起的 `e_economic_session serve` 进程退出码为 `1`，且 stderr 含 `独占进程锁不可得`。
13. 测试（#1383）：`cd rust && cargo test --release --features production_session --test production_session_entry credential_key_only_in_x_child -- --exact` 退出码 `0`。该测试断言：launcher 记录的各子进程启动环境键集中，凭据键只出现在 `x_execution_session`。
14. 票状态：`gh issue view 1383 --json state --jq .state` 输出 `CLOSED`。
15. 票状态：`gh issue view 1360 --json state --jq .state` 输出 `CLOSED`。
16. 票状态：`gh issue view 1413 --json state --jq .state` 输出 `CLOSED`。

## 受影响票与 blocking 边变化

**受影响票**

- #1323：Decisions so far 加一行。
- #1340：C10.1、C11 的 launcher 具名为 `production_session`。
- FU 票：#1531（FU-09）、#1532（FU-12，分工见裁定五）、#1533（FU-10）、#1551（FU-20）、#1535（FU-13，迁移步骤 4）。
- 实施票：#1523（交付 launcher 与 X 骨架，验收 B10/B11；阻塞 #1383、#1527）、#1527（S 输入侧路径归 `production_session`，被 #1530、#1523 阻塞）、#1525（现存被阻塞边，无新动作）。
- TB 票：#1359（TB-01，已 CLOSED，无动作）、#1360（TB-02，B15）、#1365（TB-09）、#1383（TB-09-A，B12–B14）、#1401（TB-09-B，换代 seam）、#1368（TB-10）、#1413（TB-10-I，B16）、#1369（TB-11）。
- #1546（已 CLOSED 作废，无动作）。

**现存原生边（实跑核实）**

- `gh api …/issues/1530/dependencies/blocking` 返回 1531、1533、1532、1551、1360、1365、1368、1369、1525、1527。其中 #1525（TB-04 纵片 C：RiskGate 与订单责任回路接线）一条是本稿起草后由他处接上的现存边；#1527 一条是本次修订按下文「新增」实接（2026-10-20）。
- `…/1527/dependencies/blocked_by` 返回 1567、1530、1523（后两条为本次实接）；`…/1523/dependencies/blocking` 返回 1383、1525、1527（1383、1527 两条为本次实接）；`…/1383/dependencies/blocked_by` 返回 1351、1362、1523（1523 为本次实接）。
- `…/1530/dependencies/blocked_by` 返回 1546（CLOSED）。

**处置**

- **保留**（8 条现存出边，本票关闭即满足，不删）：
  - #1530 → #1531、#1533、#1532、#1551；
  - #1530 → #1360（TB-02）、#1365（TB-09）、#1368（TB-10）、#1369（TB-11）。
  - 上一稿写「TB-01/02/09/10/11 无边可删」不对，予以更正：后四条原生边都存在，并承接本稿裁定七（AT-01/02/03/16）。
- **保留**：#1546 → #1530。#1546 已 CLOSED，评论 6262306720 称它的边随关闭失效，无需动作。
- **新增** #1532 → #1413：TB-10-I 须按 FU-12 模块级清单逐条核。#1413 现有 blocked_by 为 1402、1360、1363、1366、1367、1365、1351，不含 1532。
- **新增** #1530 → #1383：TB-09-A 的负对照和两条测试以本 ADR 的 launcher 名和锁机制为准。#1383 接边前 blocked_by 为 1362、1351。
- **保留** #1530 → #1525：现存边，本稿不动。
- **新增（已实接）** #1530 → #1527：#1527:24 要求「精确的 producer 入口路径（文件:行号）」与 CLI 调用边界，本 ADR 裁定三·7 把答案定为 `production_session` 下的 S 输入侧路径；本 ADR 未落地前 #1527 无法确定入口。
- **新增（已实接）** #1523 → #1527：#1527 的二进制身份与成功/失败收据要以 `production_session` 拉起的进程为最高缝（裁定三·8、C12.1），而 `production_session` 由 #1523 交付（裁定三·7、迁移步骤 1）；直接跑域 bin 或 `theta_replay` 只算组件证据（#1527:3）。代价照实：#1527 从此要等 #1523 关闭，而 #1523 又经 #1365 间接等本票。
- 两条新边不成环：接边前 `…/1527/dependencies/blocking` 为空，#1527 不阻塞任何票，因此从 #1527 出发沿 blocking 走不回 #1530 或 #1523。
- **不新增** #1530 → #1523：#1523 已被 #1365 阻塞，#1365 又被 #1530 阻塞，传递关系已覆盖。
- **新增（已实接，2026-10-20）** #1523 → #1383：记在 #1383 名下的 B12/B13 两条测试要拉起 `production_session` 与 `x_execution_session`，这两个二进制由 #1523 交付（B10/B11）。不成环：#1383 唯一出边是 #1401，#1401 无出边；#1365 的 blocked_by 不含 #1383，所以「#1523 被 #1365 阻塞」不会经 #1383 绕回。代价照实：#1383 从此要等 #1523 关闭。
- **不新增** #1530 → #1359：TB-01 已 CLOSED。
- **删除**：无。

**与票面「依赖的 FU」的关系（`…/1530/dependencies/blocked_by` 中两者都没有边）**

- **FU-06**：没有独立票，`gh issue list --state all --search "FU-06 in:title"` 返回 0 条。它的内容就是 #1340:396「R2 组织已采纳；具体技术承载由 C09 提出随本稿审阅」。本稿直接消费已采纳的 C09.1/7/8，不需要等待任何未决票，所以无边。
- **FU-07**：主体 #1352 已 CLOSED，落为 ADR 0030（ADR 0030:4）。余项 #1560（OPEN，P 类）只签 SR v1 阈值和退出动作表。本稿只消费已裁的 ADR 0030:44「不回退旧 writer」；launcher 启动门的二值判据来自 ADR 0028:46「失败即拒绝发送」，不依赖 SR 阈值，所以无边。

## 授权引用

本条依 #1323 Notes N-1 预授权判定（用户 2026-10-18 原话：「其实很多技术的东西 完全不需要我来做 我完全信任你的判断」；记录于 #1323 评论 [6261281111](https://github.com/xy7365527-lang/NewChanlun/issues/1323#issuecomment-6261281111)）。

本票退回重写并确认为 T 类，见 #1530 评论 [6276895863](https://github.com/xy7365527-lang/NewChanlun/issues/1530#issuecomment-6276895863)（「退回重写（用户 2026-10-20 批复：「同意」）……本票仍为 **T 类**（用户确认）」）和分级评论 [6276984750](https://github.com/xy7365527-lang/NewChanlun/issues/1530#issuecomment-6276984750)。

## 引用自证

| 引用 | 核实命令 | 结果摘要 |
|---|---|---|
| ADR 0027:10/:28/:32/:46/:62/:64/:89/:107 | Read `docs/adr/0027-classification-chong-operating-program-ownership.md` 全文 | :10 背景段三条执行线与「若直接把任一现有入口扶正」；:28 重操作程序行含「以及唯一生产交易意图」；:32「两个 owner 组成一套生产程序……失败必须作为失败处理……」；:46 裁定三原文；:62/:64 裁定五原文；:89「本 ADR 不发明类型、API、事件 schema、目录或迁移步骤」；:107「π 和 organic 赋格并行双 owner \| 拒绝」，均逐字一致 |
| CONTEXT.md:14-15 | Read `CONTEXT.md` 1-60 行 | :14「Classification 下游唯一拥有重操作状态和生产交易意图的领域 owner。它不拥有 Classification，也不重新判定结构事实。」:15「_Avoid_: 设置第二个重操作 owner；以旧入口或另一套判据接管失败」 |
| ADR 0028:22/:25/:36/:46/:63 | `cat -n docs/adr/0028-first-capability-verification-profile.md \| sed -n '1,29p'` 与 `sed -n '30,70p'` | :22 `credential_scope`；:25 `evidence_class \| experiment`；:36、:46、:63 原文逐字如正文 |
| ADR 0030:44、:4 | `grep -n "不回退旧 writer" docs/adr/0030-*.md`；`sed -n '1,6p'` | :44「无法连接场所是失败态（保留最大责任、重试、不切判据、不回退旧 writer）」；:4 写明走 #1352（FU-07） |
| ADR 0038:60/:74 | `cat -n docs/adr/0038-dual-run-determinism-recovery-seek-asknown.md \| sed -n '58,80p'` | :60「**恢复**：launcher 启动时检测上次 checkpoint……」；:74「`evidence_class ∈ {simulation, experiment, production}`……simulation 产出不进 TStage 触闸计数」；:1-3 三行正本头先例 |
| 当前最大 ADR 编号 0038 | `ls docs/adr/ \| tail -30` | 末项 `0038-dual-run-determinism-recovery-seek-asknown.md` |
| graph-engineering.md:23、:63 | Read `.sandcastle/docs/graph-engineering.md` | :23 T 类定义原文；:63 N-1 用户原话 |
| generation-constitution.md:32/:35 | `sed -n '26,36p' docs/agents/generation-constitution.md` | :32「## 2. 现役线名单」；:35「**π 是唯一现役引擎**。锚定方式 = 家族根函数名（`pi_theta_fill_loop_overlay`），不锚行号（行号随版本漂移）。」 |
| AGENTS.md #793/#799/#804 | Read `AGENTS.md` | :36 注释须带票号；:58 #799 判别式「接在同一个 if-else 上」；:70-76 #804 ①判别式与不落在形状上 |
| #1340 票面 :10/:22/:326/:332/:333/:340/:350-355/:359/:372-373/:396-397/:409-411/:424 | `gh issue view 1340 --json body --jq .body > /tmp/fu08_1340.md`；`grep -n` 与 `sed -n '318,362p;390,410p'` | 各行原文如正文所引；state OPEN；标题「SPEC：完整生产程序、结构全分类与逐重经营（#1323，R2）」 |
| SPEC.md:422 | `sed -n '422p' .chanlun/review-results/issue1339-r2-spec-20260909/view/spec/SPEC.md` | 「\| AT-16 \| 每条入口/preset/feature/服务/直发路径有实际终态；默认启动、影子和带仓迁移不能绕过 owner/X \| C11；Destination 2/3 \|」 |
| OLD-ENTRY-DISPOSITIONS.md:14、:20-25、OE 各组 | `sed -n '14p;20,25p' <path>`；`grep -n "^### OE" <path>`；`sed -n` 读 :33/:63/:78/:93/:108/:123/:138/:153/:168/:183/:198/:212/:229/:245/:260/:274/:498 冻结路径 | :14 原文命中；X1–X6 在 :20-25，X6 含「文件删除是后续逐依赖验收事项」；45 个组标题：OE01 :31 newchan CLI/Bottle/dev.ps1；OE03 :61 gateway；OE04 :76 overlay/realtime_server；OE05 :91 `orchestrator/recursive.py`；OE06 :106 PyRecursiveOrchestrator；OE07 :121 chanlun_strategy/maker_optimizer/lmt_executor；OE08 :136 `theta_v0/ffi.rs`/`stream.rs`/theta_rec_strategy；OE09 :151 nautilus/backtest_engine；OE10 :166 fugue_v3；OE11 :181 recursive_t；OE12 :196 spiral；OE13 :210 NT backtest/live/paper 装配面；OE14 :227 `hl_verify_*.py`、`buy_28316_warrant.py`；OE15 :243；OE16 :258 `trading/runner.rs`；OE17 :272 positional；OE32 :496 `theta_v0/backtest/fill.rs`；OE33 :511 parser/classifier；OE35 :541 研究 runner；OE41 :631；OE42 :646；OE45 :689 |
| TB01-ENTRY-RECON.md:31 | `sed -n '31p' .chanlun/review-results/issue1340-publication-20260909/payload/inputs/TB01-ENTRY-RECON.md` | 「前者TradingNode只在docstring，实际:61报尚未实装且:69返回1；后者没有S会话命令。需要真实外部S会话launcher。」 |
| #1323 票面 :32/:33/:41/:88 | `gh issue view 1323 --json body --jq .body > /tmp/fu08_1323.md`；`grep -n` | :32 Destination ②、:33 Destination ③ 原文；:41 N-1 条；:88「结论是仓内尚无 ADR 0027 端到端入口」 |
| #1323 评论 6261281111 | `gh api repos/xy7365527-lang/NewChanlun/issues/comments/6261281111` | xy7365527-lang，2026-10-19T06:38:25Z，含「用户 2026-10-18 原话：「其实很多技术的东西 完全不需要我来做 我完全信任你的判断」」 |
| #1530 评论 6276895863 / 6276984750 / 6263993315 | `gh api repos/xy7365527-lang/NewChanlun/issues/1530/comments --paginate`；`gh api …/issues/comments/6276895863`、`…/6276984750` | 6276895863：xy7365527-lang 2026-10-19T20:43:22Z，「退回重写（用户 2026-10-20 批复：「同意」）」，含四条约束；6276984750：xy7365527-lang 2026-10-19T20:48:09Z，「分级：T — …」；6263993315：上一稿与 BLOCK 审查 |
| #1546 与评论 6262306720 | `gh issue view 1546 --json state`；`gh api …/issues/1546/comments` | CLOSED；6262306720 xy7365527-lang「本票是早先错误方案建出的，作废关闭；它对 FU 票的 blocking 边随关闭失效」 |
| #1530 原生边 | `gh api repos/xy7365527-lang/NewChanlun/issues/1530/dependencies/blocking`、`…/blocked_by` | blocking：1531、1533、1532、1551、1360、1365、1368、1369；blocked_by：1546（closed） |
| #1413/#1383/#1360/#1365/#1368/#1369/#1532/#1523/#1527/#1401 的 blocked_by | `gh api …/issues/<n>/dependencies/blocked_by --jq '[.[].number]\|join(",")'` | 1413：1402,1360,1363,1366,1367,1365,1351；1383：接边前 1362,1351，现为 1351,1362,1523（落地时再加 1530）；1360：1359,1530,1531,1533,1534,1559；1365：1362,1530,1532,1535,1539,1540,1561；1368：…,1530,1532,…；1369：…,1530,…；1532：1530,1544；1523：1362,1363,1366,1365；1527：接边前 1567，接边后 1567,1530,1523；1401：1383,1362,1351 |
| 相关票状态与标题 | `gh issue view <n> --json number,state,title` | 1359 CLOSED；1360/1365/1368/1369/1413/1383/1401/1402/1531/1532/1533/1535/1551/1523/1527/1560/1323 OPEN；1546 CLOSED；1560 标题「FU-07 余项：系统风险条件 SR v1 阈值与退出动作表签署（#1323，P 类）」 |
| FU-06 无独立票；FU-07 → #1352 | `gh issue list --state all --search "FU-06 in:title"`、`"FU-07 in:title"` | FU-06 返回 0 条；FU-07 返回 1560 OPEN、1352 CLOSED |
| #1531/#1532/#1533/#1551 登记 FU-08 | `gh issue view <n> --json body --jq .body \| grep -n 'FU-08\|1530'` | 1531:24、1532:23、1533:24、1551:27 均含「FU-08（#1530）」 |
| #1532 票面 | `gh issue view 1532 --json body --jq .body` | 决策问题「每条旧入口的文件路径、终态类别、替代入口 / 默认启动不触发旧核的机制 / 失去写权和退场条件」 |
| #1413 票面 :7/:12 | `gh issue view 1413 --json body --jq .body \| grep -n …` | :7「默认launcher/preset/feature/service只装配新完整程序」；:12 OUT_OF_SCOPE_PRESERVE 原文 |
| #1383 票面 :7/:12 | 同上 | :7「正式launcher在具名部署启动时核验旧入口及凭据/身份作用域」；:12「唯一X持写凭据……单靠目录/开关false/手工y/进程epoch不算阻断」 |
| #1523 票面 :3/:24 | 同上 | :3「允许不消费未完成前置的准备和契约验证；不得绕过 S/E/B/X 或使旧直发路径取得生产权」；:24「正式 launcher 代码落盘（`trading_system/execution/hl_paper_launcher.py` 或类似路径）」 |
| #1527 票面 :3/:23 | 同上 | :3「已有 theta_replay 二进制与单日双跑仅为组件证据」；:23「**确认正式生产入口**」 |
| #1560 票面 | `gh issue view 1560 --json body --jq .body \| head -20` | 只签 SR-1..4 阈值与动作表；「结构、范围、退休四条件、崩溃续做均已裁」 |
| Cargo.toml :50/:54/:56/:70-73/:78-81/:245-253 | `grep -n …`；`sed -n '68,74p'`；`grep -n '^\[features\]' -A12` | `default = []`；:54「CLI 生产入口：解 theta_v0::backtest 的 #[cfg(test)] 门控为公开 API……」；:56 `backtest_bin = ["nautilus", "dep:sha2"]`；`theta_backtest` 的 `required-features = ["backtest_bin"]` 在 :73；s_session 三门在 :81/:248/:253（:76 是注释） |
| 67 个 bin 的分组计数 | `cargo metadata --no-deps --format-version 1 --offline \| jq …group_by…` | `{"":42,"backtest_bin":21,"lav_seal":1,"s_session":3}`；backtest_bin 组含 theta_backtest 与 20 个探针 |
| feature 定义 | `cargo metadata … \| jq -c '.features \| {default, s_session, backtest_bin, lav_seal, nautilus}'` | 无 `production_session`；`nautilus` 引入 nautilus-* crate；`jq -e` 验收式当前退出码 1 |
| theta_backtest.rs:17/:30/:106 | `cat -n rust/src/bin/theta_backtest.rs \| sed -n '15,18p;30p;106p'` | 原文如正文 |
| fill.rs:746/:791/:4767；runner.rs:250；stream.rs:117；backtest_engine.rs:141 | `grep -n` 各函数签名 | `pub(super) fn pi_theta_fill_loop<F>(` 等三处；`pub fn run_theta_v0_pi(`；`pub fn push_bar(&mut self, bar: Bar, p_t: f64, nav: f64) -> f64 {`；`pub fn run_theta_backtest(` |
| ffi.rs:59-61 | `sed -n '59,61p' rust/src/theta_v0/ffi.rs` | `#[pyclass(name = "ThetaStream", unsendable)]`、`pub struct PyThetaStream {`、`core: ThetaPiStream,` |
| nautilus/mod.rs:69-83；theta_strategy.rs:136；theta_pi_strategy.rs:161；order_adapter.rs:23 | `rg -n '^#\[cfg\|^pub mod' rust/src/theta_v0/nautilus/mod.rs`；`rg -n 'submit_order\(' …` | :69-70、:76-77 `#[cfg(feature = "nautilus")]` 门 theta_strategy/theta_pi_strategy；:82-83 `#[cfg(all(feature = "nautilus", feature = "backtest_bin"))] pub mod backtest_engine;`；两处 `self.submit_order(order, None, None, None)`；order_adapter :23 为文档行 |
| trading/runner.rs:313、positional.rs:1066、recursive_position.rs:140 | `grep -n "pub fn run_organic(\|pub fn run_positional(\|pub fn run_recursive(" …` | 三个签名逐字命中 |
| lib.rs:2810-2837、:948 | `cat -n rust/src/lib.rs \| sed -n '2806,2840p'`；`grep -n 'pyclass(name = "RecursiveOrchestrator")'` | :2810 `fn newchan_rust(`；:2811-2815 organic 族；:2817-2821 spiral/fugue_v3；:2822-2832 recursive_t 三项带 `#[cfg(not(test))]`；:2834 PyThetaStream；:2835-2837 PyBiEngine/PyOnlineMacdState/PyRecursiveOrchestrator；:948 命中 |
| s_structure_session.rs:25-29 | `cat -n rust/src/bin/s_structure_session.rs \| sed -n '25,29p'` | 「## 生产零调用纪律（不变）」；:27-28「不写第二份生产判定器；不调用 `ThetaPiStream` 资金推进冒充 S；不启动 E/B/X」；:29 不复用 `trading_system/persistence/database.py` 共库 |
| e_economic_session.rs / b_chong_session.rs | `cat -n` 两文件 | 各 7 行；`cli(false)` / `cli(true)` |
| service.rs:565/:595 | `grep -n "pub fn cli(is_b: bool)\|process_lock(&config.db"` | `pub fn cli(is_b: bool) -> Result<(), String> {`；`let _lock = process_lock(&config.db, &config.domain)?;` |
| session_transport.rs:40/:53/:62 | `sed -n '40,63p' rust/src/session_transport.rs`；`grep -n "独占进程锁不可得"` | `pub fn process_lock(`；`format!("{}.writer.lock", …)`；:62 `format!("StaleWriter：{domain_name}独占进程锁不可得：{e}")` |
| store.rs:466-467 | `sed -n '460,470p' rust/src/economic_session/store.rs` | `if positive(&meta(c, "writer_epoch")?, …)? != expected {` / `return Err("StaleWriter：持久epoch不等于获准写者".into());` |
| 仓内无 X、无 production_session | `grep -rln "x_execution_session\|唯一执行入口 X\|X 独占\|production_session" rust/src trading_system scripts` | 零命中 |
| pyproject.toml:34-35；cli.py:42/:325/:327；server.py:420/:617；gateway.py:855/:946；realtime_server.py:25/:67 | 各 `grep -n` | 全部行号与内容命中如正文 |
| dev.ps1:30/:38 | `grep -n "newchan\|python\|cargo" scripts/dev.ps1` | :30 `& python -m newchan.cli serve --port 8765 --no-browser`；:38 `& python -m uvicorn newchan.gateway:app --host 127.0.0.1 --port 8766 --reload` |
| trading_system/README.md:12/:15/:18 | `cat -n trading_system/README.md \| sed -n '1,22p'` | 三条命令逐字可见 |
| backtest/runner.py:8-9；live/runner.py:61/:69；databento_live_runner.py:3 | `sed -n '1,13p'`；`sed -n '61p;69p'`；`sed -n '1,25p'` | 「→ BacktestEngine → ChanlunStrategy.on_bar → ChanlunBridge.feed → Rust 引擎」；`print("实盘入口为阶段4骨架，尚未实装。…")` / `return 1`；「数据-only TradingNode（无 exec client，不下单）」 |
| chanlun_strategy.py:20-21；lmt_executor.py:70；maker_optimizer.py:89；theta_rec_strategy.py:123；broker_config.py:79；risk_controls.py:1 | `grep -n` 各行；`grep -m1 -n '"""'` | import `LmtExecutor`/`MakerOptimizer`；`self._strategy.submit_order(order)`；`self._strategy.cancel_order(order)`；`self.submit_order(order)`；`private_key=os.environ.get("HYPERLIQUID_PRIVATE_KEY"),`；「PreTradeRiskGate —— paper 试运行风控四条」 |
| 研究回测脚本首行 | `grep -m1 -n '"""\|^#'` 各文件；`sed -n '2,3p' analyze_fugue_v3_losses.py`；`sed -n '1,4p' scripts/…` | `backtest_t_fugue.py:1 # FROZEN(#951)…`；fugue_v3/unn/unn_stream/spiral_stream/compare_spiral_unn 的 :1 docstring；analyze :2「fugue_v3 深亏标的交易行为解剖」；batch_backtest :2、av_recursive_run :2 如正文 |
| hl_verify_sdk.py:6/:87；hl_verify_nautilus.py:6/:128 | `sed -n '1,8p;87p'`；`grep -n "HYPERLIQUID_PRIVATE_KEY"` | order 阶段「LMT 买单 0.001 BTC @ 远低于市价（tif=Alo 双保险）→ 立即撤单」；`pk = os.environ.get("HYPERLIQUID_PRIVATE_KEY")`；NT 版 post-only LMT → 撤单，:128 读同一变量 |
| buy_28316_warrant.py:21/:170/:197 | `grep -n "placeOrder\|cancelOrder\|^PORT"` | `PORT = 7496          # TWS live`；`trade = ib.placeOrder(contract, order)`；`ib.cancelOrder(order)` |
| 写权闭包 10 个文件 | 验收 A4 的 `diff` 命令实跑 | 退出码 0，无差异 |
| TWS 只读脚本 | `grep -rl "7496" … --include='*.py'`；`grep -n "PORTS\|7496"` | `scripts/tws/fetch_1m_tws.py:38 PORTS = [7496, …]`、`analysis/_tencent_warrants_quotes.py:61 ib.connect(…7496…)`；均不在下单命中集内 |
| 交易外 service | `git ls-files \| grep -iE '\.plist$\|\.service$'`；`grep -n "ExecStart"` | `.sandcastle/launchd/` 五个 plist；`fengliang.service:9`、`openclaw.service:10`、`topo-daemon.service:9` 的 ExecStart；`external/setup-c/com.junyu.k4-weekly.plist` |
| `.chanlun/definitions/` 无 launcher 冲突 | `grep -rln "launcher" .chanlun/definitions/` | 零命中 |
| 基线 commit | `git log -1 --format='%h %H'` | `b63a3fd586`，工作树 clean |
| #1527/#1523/#1525 边（修订后） | `gh api graphql` 查 `issue(number:N){blockedBy{nodes{number}} blocking{nodes{number}}}` | 1527 blockedBy=1523,1530,1567、blocking=空；1523 blocking=1527,1525；1530 blocking 含 1525、1527；#1525 标题「[impl] TB-04 纵片 C：RiskGate 与订单责任回路接线」，#1567 标题「[grilling] s-admission 首份锚的签发与无锚库接管（#1527）」 |
| #1527 票面 :24（范围第 1 项） | `gh issue view 1527 --json body --jq .body \| grep -n '精确的 producer 入口路径'` | :24「精确的 producer 入口路径（文件:行号）」 |
| lib.rs:2811-2832 导出标识符 | `sed -n 2811,2832p rust/src/lib.rs` | 2811 `PyOrganicTape`；2812 `run_organic_rust`；2813 `run_recursive_rust`；2814 `run_positional_rust`；2815 `PyUnnStream`；2817 `PySpiralStream`；2818 `run_spiral`；2820 `PyFugueV3Stream`；2821 `run_fugue_v3`；2823 `run_recursive_t`；2829 `PyTFugueStream`；2832 `PyRecStream`（后三项均带 `#[cfg(not(test))]`） |
| #1383/#1401/#1365 边（第二次修订后） | `gh api graphql` 查 blockedBy/blocking | 1383 blockedBy=1523,1351,1362、blocking=1401；1401 blocking=空；1365 blockedBy 不含 1383；1523 blocking=1383,1527,1525 |

## 修订（#1523，汇总回环与来源边界，2026-10-28）

（append-only：裁定一至七原文不改，本节为现行修订。依据 #1523 P 类裁定稿 28G，用户 2026-10-28 批复，异家族独审 28E（B3 外各项 PASS）与 28H（B3 修复 PASS）；草稿、考古与审查原件在 `/Volumes/AgentStorage/decisions/1523/`。）

**原因**：原「受影响票与 blocking 边变化」节的无环论证只看原生 blocked_by。#1362、#1365、#1366 的 AC 第一条都要求全部内部子片交付，汇总票关闭须等每张 open 子票。把这种等待计为边后，实图有一个 29 票的强连通分量，经 #1383、#1524、#1525 三处回到 TB 链。本修订把三者改为等待 #1523 下的两张组件叶，#1523 保留全部原义务。

1. **无环论证补记汇总边。** 原「新增（已实接，2026-10-20）#1523 → #1383」一条的「不成环」论证，在计入「#1365 汇总等 #1383」后不成立。本修订把 #1383 的前置由 #1523 改接到组件叶 [#1769](https://github.com/xy7365527-lang/NewChanlun/issues/1769)（C-L，launcher 与 X 骨架）；#1525 改等 [#1770](https://github.com/xy7365527-lang/NewChanlun/issues/1770)（C-API）、#1422、#1425；#1524 改等 #1769 与 #1366，父票由 #1362 改为 #1382。改后对全图（计入汇总隐式边）跑 Tarjan SCC，结果为空。
2. **验收 A8 改写。** 原 A8 在本修订前成立，现改为：`gh api repos/xy7365527-lang/NewChanlun/issues/1383/dependencies/blocked_by --jq 'map(.number)|contains([1769,1530])'` 输出 `true`。
3. **B10、B11 的验收票**写作「#1523（经子票 #1769）」。B12、B13 不变，仍归 #1383，测试写在 `production_session_entry`，运行对象是 #1769 交付的两个二进制。
4. **「原始输入来源」只指 S 的结构源声明**（裁定三·2）。取值受 ADR 0020 重写的裁定二与 ADR 0023 个股线约束，只能是个股线美股 tick 或显式 TestOnly 结构输入。场所执行行情（HL 永续成交与 bar）只出现在 X 的「adapter 选择」下，不进 `S.AcceptInput`。本条是澄清，不改任何裁定。
5. **A4 写权闭包列表**在 #1770 合入时加入其 HL adapter 源文件路径，作为 X 内唯一合法的场所写代码；原 10 个旧文件不变。在 #1770 合入前，A4 命令按原列表执行。
6. **实时漂移记录。** 原节记 #1523 阻塞 #1383、#1525、#1527。本修订落地后读数为：#1523 阻塞 #1413、#1675；#1769 阻塞 #1383、#1524、#1770；#1770 阻塞 #1525；#1527 不再被 #1523 阻塞（该边已移到 #1675）。#1523 的 blocked_by 保持 #1362、#1363、#1365、#1366。

## 修订（#1539，恢复读法与引用行号，2026-11-02）

（append-only：裁定一至七与 #1523 修订节原文不改。依据同 ADR 0038「修订（#1539）」节。）

1. **恢复读法。** :27、:147、:289 引用的 ADR 0038:60 按 ADR 0038「修订（#1539）」读：`production_session` 启动时检测 checkpoint，加载快照后重放持久日志。experiment/production profile 下不回退 checkpoint 之后已持久的 X/E/Bᵢ 事实，在途订单先只读对账、外部未知按最大责任保留，与 :288「不回退旧 writer」同向；simulation profile 按该修订第 2 条执行。
2. **引用行号。** :26、:288、:326、:412、:427 所引「ADR 0030:44」现址为 ADR 0030:48（#1599 批复插入观察期段后漂移，提交 `40f11b540f`），文义不变。
