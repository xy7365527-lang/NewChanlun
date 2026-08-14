# #960（map #787）模块清单遗漏审计——乙层目标模块图的输入完整性核查

- 日期：2026-08-14
- 票据：feed [#960](https://github.com/xy7365527-lang/NewChanlun/issues/960)（parent map [#787](https://github.com/xy7365527-lang/NewChanlun/issues/787)）
- 边界：只判不改。本报告零代码改动，唯一写入物是本文件。
- 快照：`main` @ `a04e5f4a19267157aac4ea03226fa9de052a7681`（2026-08-14 12:14 +0800）。全部实测数来自该快照的 `git ls-files` + 源码静态读；未跑 `cargo build`（见 090）。
- 方法：① 把 map #787 **全裁定**里出现过的模块清单逐份抽出来（票面疆域表、范围条、#789 重复实现普查的区代号、ADR 0010–0022 与 `.chanlun/definitions/` 六概念正本的受影响代码清单、SPEC #847）；② 用 `git ls-files` 实测仓内模块全集；③ 逐模块对照，按四条判定规则（见 §3.1）给每个不在清单里的模块定性。

## 0. 一句话结论

**#787 全裁定的模块清单没有一份覆盖仓内模块全集，且三份清单互不一致。** 对 #960 最承重的三条真遗漏：① **`rust/src/trading/`**（organic-fugue-v2 交易层，31 文件 / 34,431 行，pymodule 五导出、96 个 Python 调用文件，ADR 0017 T-11 与 `qujiantao.md` 的受影响代码落点都在它里面）在疆域表与范围条里**没有任何条目**；② **`theta_v0` 的子模块清单**（疆域表写 classifier/backtest/strategy/nautilus 四格）漏了 `parser/`（5,688 行）、`ledger/`、`closed_loop/`、`complete/`、`venue_fee/` 五格加六个顶层 `.rs`；③ **`rust/tests/`** 是 formal→rust 边（#790 E7）的落点，图上**有边无区**。此外裁定自己的受影响代码清单已点到 `rust/src/spiral/`、`rust/src/fugue_v3/`、`rust/src/bin/`、`scripts/`、`frontend/`、`topological-computation/` 等路径，而这些模块同样不进三份清单任何一格——**「清单」与「清单的宿主」在裁定内部就对不上**。

## 1. 裁定侧模块清单盘点

### 1.1 三份清单原文

**清单 A —— #787 票面「疆域」表（六行）：**

| 区 | 内容 |
|---|---|
| `src/newchan` | Python 引擎（`docs/ROADMAP.md` 五层递归管线） |
| `rust/src/*.rs` | 8 层逐位等价重写（bi/seg/zs/move/BSP/PH/MACD/Orchestrator） |
| `rust/src/theta_v0/` | classifier / backtest / strategy / nautilus 适配层 |
| `formal/` | Lean 形式化链（149 文件） |
| `analysis/` | 脚本→已订正（#844）：664 条目中 .md 309、.py 312 |
| `trading_system/` | `rec_t_strategy.py` 那套 |

**清单 B —— #787「范围（2026-07-30 编排者追加裁定）」条（六项）：**

> 范围以 [疆域测绘 #790] 的六区为锚（`src/newchan` / `rust/src/*.rs` 顶层散件 / `rust/src/theta_v0/` / `rust/src/recursive_t/` / `formal/` / `trading_system/`）。

**清单 C —— #789 重复实现普查的「区代号」（五档）：**

> 区代号：**顶层** = `rust/src/*.rs` 顶层散件（#764 判现役）；**Θ** = `rust/src/theta_v0/`（π 现役主线）；**前代** = `spiral/`+`fugue_v3/`+`recursive_t/`+`trading/`（#761–#763 判现役）；**Py** = `src/newchan/`；**Lean** = `formal/Origin/`。

### 1.2 三份清单的相互出入（逐格对表）

| 模块（实际存在） | 清单 A 疆域表 | 清单 B 范围条 | 清单 C #789 区代号 |
|---|---|---|---|
| `src/newchan/` | ✅ | ✅ | ✅（Py） |
| `rust/src/*.rs` 顶层散件 | ✅ | ✅ | ✅（顶层） |
| `rust/src/theta_v0/` | ✅ | ✅ | ✅（Θ） |
| `formal/` | ✅ | ✅ | ✅（Lean，且窄化为 `formal/Origin/`） |
| `analysis/` | ✅ | ❌ | ❌（消费面不在矩阵内） |
| `trading_system/` | ✅ | ✅ | ❌（消费面不在矩阵内） |
| `rust/src/recursive_t/` | ❌ | ✅ | ✅（并入「前代」档） |
| `rust/src/trading/` | ❌ | ❌ | ✅（并入「前代」档） |
| `rust/src/spiral/` | ❌ | ❌ | ✅（并入「前代」档） |
| `rust/src/fugue_v3/` | ❌ | ❌ | ✅（并入「前代」档） |
| 其余（见 §2 全集） | ❌ | ❌ | ❌ |

**注（折叠读 vs 字面读）**：按 #790 报告 §四-3 的折叠读（spiral/fugue_v3/recursive_t/trading 四族「属『顶层散件』名片同一区」），清单 A 的 `rust/src/*.rs` 行覆盖四族；按字面读（疆域表只写「8 层」名单）则四族全部无条目。上表取字面读；折叠读的后果见下条第 2 点——同一条裁定里两种读法并存。

三处结构性出入：

1. **B 自称「以 #790 的六区为锚」，但列的六项不是 #790 的六区**——#790 票面六区是 `src/newchan` / `rust/src/*.rs` / `rust/src/theta_v0/` / `formal/` / **`analysis/`** / `trading_system/`；B 把 `analysis/` 换成 `recursive_t/`，未说明。`analysis/` 是 #790 实测出的**最大消费面**（132 个脚本 `import newchan_rust`、72 个脚本 `from newchan.*`），乙层图画边绕不开它，而范围条偏偏把它排除了。
2. **「顶层散件」一词双义**：#790 报告 §四-3 明写 spiral/fugue_v3/recursive_t/trading 四族「属『顶层散件』名片同一区」（折叠读）；B 却把 recursive_t 单列为第六区（单列读）。同一份裁定里两种读法并存——折叠读则 B 第六区多余，单列读则 trading/spiral/fugue_v3 三族无条目。**#960 若拿这份清单切图，四族里该进图几个取决于先猜中哪种读法。**
3. **C 把四族统称「前代」，而 #762 已判现役**——「前代」是代际、「现役」是名分，两轴不矛盾但极易读错（现役的 `trading/` 族因此长期顶着「前代」标签，见 §4.1 的 GUARD-ROLE 头注释）。

### 1.3 裁定自身引用、清单却不含的路径（在案证据）

「清单」与「清单的宿主」在裁定内部对不上的证据（全为逐字摘引）：

| 出处 | 引到的路径 | 宿主模块 | 宿主在清单 A/B 有没有条目 |
|---|---|---|---|
| ADR 0017 T-11 | `rust/src/trading/positional_fusion.rs:91-104` `SUB_SPAWN_FRAC`、`rust/src/trading/unified_necessity.rs:993-995` | `trading/` | ❌ |
| ADR 0017 T-11（附） | `rust/src/spiral/params.rs:37` `SUB_SPAWN_FRAC = 1.0/LAMBDA`（全仓第二份同名常数，互不引用） | `spiral/` | ❌ |
| ADR 0016 | `rust/src/spiral/params.rs:32` `LAMBDA = 2.0` vs `rust/src/fugue_v3/mod.rs:111` `LAMBDA = 3.0`（两个互斥 λ） | `spiral/` `fugue_v3/` | ❌ |
| `qujiantao.md:1062/1068` | **伪引文** `rust/src/trading/positional.rs:458`（`TrendScope::Ancestor` 唯一溯源，内容编造） | `trading/` | ❌ |
| `bi.md:342` | 「theta_v0 那套消费方全在 `rust/src/bin/*`」 | `bin/` | ❌ |
| `qujiantao.md:1302` | `date_to_timestamp` 31 份复制（`rust/src/bin/` 下 30 份） | `bin/` | ❌ |
| ADR 0019 | 两个只读探针 bin `rust/src/bin/p938_*.rs` | `bin/` | ❌ |
| #815 M-3（#787 正文内） | 「`detect_levels` 上游全是 `scripts/` 研究脚本」 | `scripts/` | ❌ |
| `bi.md` U-13 / `xianduan.md` V-6 | **自登记未核** `frontend/`、`trading_system/`、`prototypes/`、NT 适配层，状态「无人跟进」「须新开票」 | `frontend/` `prototypes/` | ❌ |
| ADR 0017 附带发现 | `topological-computation/signifier_net/corpora/chanlun/031-第31课.md` 第二副本（行号体系完全不同） | `topological-computation/` | ❌ |
| `qujiantao.md:570` | `rust/tests/econ_oddeven_diagnosis.rs` | `rust/tests/` | ❌ |

**即：裁定明确知道这些模块存在、且对其中多数给过行号级引用，但三份模块清单一份都没收它们。** #960 题面「输入材料」列的六份勘察底座（#789/#790/#792/#827/arch-survey §三/六概念正本）同样没有一份补齐这个清单。

## 2. 实测模块全集（本快照）

代码行 = 代码扩展名（.py/.rs/.ts/.tsx/.lean/.sh）行数和；文件数 = `git ls-files` 全部 tracked 文件（含数据/文档）。「现役」引 #761–#763/#764 名分核定与 GUARD-ROLE 头注释。

| 模块 | 文件 | 代码行 | 现役/状态 | pymodule 导出 | Python 调用面（.py，analysis+trading_system+scripts+tests+src） |
|---|---|---|---|---|---|
| `src/newchan/` | 204 | 48,222 | 活（analysis/ 72 脚本消费） | —（纯 Python） | — |
| `rust/src/*.rs` 顶层散件（17 个 .rs） | 17 | 12,153 | 现役（14+2 单列+lib.rs） | 17（3 类+14 函数：PyBiEngine/PyOnlineMacdState/PyRecursiveOrchestrator + segments_from_strokes_v1 等） | 全部 `newchan_rust` import 面 142 个文件（132 analysis / 8 trading_system / 1 scripts / 1 src） |
| `rust/src/theta_v0/` | 199 | 177,619 | 现役（π 唯一现役引擎；**零 Python 出口**，lib.rs pymodule 段 `theta` 命中 0 次） | **0** | 0 |
| `rust/src/recursive_t/` | 16 | 17,453 | 现役（#762 二次订正） | 4（run_recursive_t / PyTFugueStream / run_t_fugue / PyRecStream=RecTStream） | 6 文件（3 analysis / 3 trading_system，含 `rec_t_strategy.py` 生产策略） |
| **`rust/src/trading/`** | 31 | 34,431 | 现役（GUARD-ROLE） | **5**（PyOrganicTape / run_organic_rust / run_recursive_rust / run_positional_rust / PyUnnStream） | **96 文件（91 analysis / 5 trading_system）——全仓最大单族消费面** |
| **`rust/src/spiral/`** | 12 | 3,648 | 现役（#762 改判；fugue_v3 硬依赖其 signal 层） | 2（PySpiralStream / run_spiral） | 2 文件（trading_system：`compare_spiral_unn.py` + `backtest_spiral_stream.py`，真流式对账） |
| **`rust/src/fugue_v3/`** | 11 | 2,356 | 现役（#762 改判；recursive_t flat 支硬依赖其会计层） | 2（PyFugueV3Stream / run_fugue_v3） | 1（`trading_system/backtest_fugue_v3.py` 真调用） |
| **`rust/src/bin/`** | 53 | 42,972 | 活（14 个 cargo [[bin]]，多数 `required-features=backtest_bin`；theta_v0 的主要消费面） | — | — |
| **`rust/tests/`** | 26（10 .rs） | 2,960 | 活（CI `cargo test`；formal↔rust E7 边落点：`fixtures/*.json` + 5 个 parity 测试 include_str!） | — | — |
| `formal/` | 154（144 .lean） | 59,266 | 活（仅对 CI 活，fixture-drift job） | — | — |
| `analysis/` | 698（348 .md + 314 .py） | 98,898（.py） | 活（最大研究消费面） | — | — |
| `trading_system/` | 94（38 .py + 54 data_cache json） | 4,976（.py） | 活（离下单最近的一区） | — | — |
| **`scripts/`** | 249（230 .py） | 87,064 | 活（`check_fixture_drift.py` = CI fixture-drift job 执行者；detect_levels 研究脚本宿主） | — | — |
| **`tests/`** | 270（.py） | 82,682 | 活（Python 侧测试面，`test_rust_*_equivalence.py` 九组逐位等价测试宿主——#789 票面线索第一条点名要查它） | — | — |
| **`topological-computation/`** | 1,351（164 .py） | 92,570 | 独立研究系统（signifier_net）；含 109 份课程语料第二副本（`signifier_net/corpora/chanlun/`，行号体系与正本不同，AGENTS.md 已钉正本路径 `docs/chanlun/text/blog/`） | — | — |
| **`frontend/`** | 39（17 .ts + 10 .tsx） | 3,148 | 图表前端（React/Vite，ChanTheoryPrimitive；消费 `src/newchan/server.py` HTTP） | — | — |
| **`chanlun/`** | 378（332 .md） | 2,825（.py） | 工作草稿区（roster/archive/escalate/harness/plans/review-results）——**目录名与 Python 引擎近名、与 `.chanlun/` 同名异物** | — | — |
| `.chanlun/` | 1,216 | — | 谱系与工作草稿（#790 边界条明确排除） | — | — |
| **`prototypes/`** | 27（6 .rs） | 4,377 | 原型区（#842 产物在抛弃分支，此处另有一套） | — | — |
| **`experiments/`** | 7 | 748 | 实验区 | — | — |
| **`external/`** | 5 | 73 | 外部资料 | — | — |
| **`spec/theorems/`** | 13 | — | Lean 定理 spec 文档 | — | — |
| **`tradingview-mcp-plugin/` + `tradingview-chanlun.plugin`** | 5 | — | TV 集成侧 | — | — |
| **根级散件 .py** | 2 | — | `tws_margin_check.py`（IBKR/TWS 保证金检查）、`vcp_screener_v2.py`（VCP 选股器）——K4 选股 / IBKR 执行支柱的边角 | — | — |
| `p7_inputs/` / `references/` / `nautilus_trader/` | 2 tracked + 1 空目录 | — | 输入数据 / 模板 / 空目录（0 tracked 文件） | — | — |
| `docs/` | 245 | — | 文档（含五族架构稿 `docs/spiral_engine_v2_architecture.md`、`docs/reference-theta-v0.md`、`docs/unified_recursive_operator_T.md`） | — | — |
| `tmp/` | 755 | 35,444 | 工作垃圾（另有 `.scratch/`、`.auto-memory-snapshot/` 等） | — | — |

全仓 tracked 文件 6,856；上表覆盖全部代码承载目录。**行数含空行与注释，是量级不是精确数。**

## 3. 遗漏判定

### 3.1 判定规则（四条）

- **R1 必须进图（真遗漏）**：在「K 线→下单」链上有代码实体，且满足其一——(a) 含缠论判定逻辑或交易链逻辑；(b) 裁定受影响代码清单已点到行号；(c) 经 lib.rs pymodule 导出被 Python 消费；(d) 在编译树 `pub mod` 里。
- **R2 折叠包含、须定颗粒度**：被 #790 折进「顶层散件名片同一区」的四族——对 #960 来说折叠读与单列读必须选一个写死在图里。
- **R3 出界但须显式声明**：不在统一范围（非交易链的研究/数据/草稿/工具），但**不许无声缺席**——按「凡单例者显式声明为单例」的同构纪律，乙层图须有一行「以下模块出界 + 理由」。
- **R4 无代码价值**：纯工作垃圾（`tmp/` 等），图上不列。

### 3.2 逐模块判定表

| 模块 | 判定 | 理由（证据见 §1.3/§2） |
|---|---|---|
| `rust/src/trading/` | **R1 真遗漏（最承重）** | 五导出 + 96 调用文件 + ADR 0017/qujiantao 落点宿主；详情 §4.1 |
| `rust/src/spiral/` `rust/src/fugue_v3/` | **R1 真遗漏** | #762 判现役；ADR 0016 双 λ 落点；fugue_v3 有真调用；详情 §4.2 |
| `rust/src/bin/` | **R1 真遗漏** | theta_v0 的主要消费面（`bi.md:342`）；详情 §4.3 |
| `rust/tests/` | **R1 真遗漏** | E7 边落点（图上边有区无）；对拍测试锁宿主（#804「载体只能是测试锁」） |
| `scripts/` | **R1 真遗漏** | `check_fixture_drift.py` 是 E7 边的执行者（进图即可，其余脚本可归 R3 研究面） |
| `tests/` | **R1 真遗漏（次级）** | Python 对拍宿主（`test_rust_*_equivalence.py` 九组）；#789 票面点名、报告却没把它归入任何区 |
| `recursive_t` vs 其余三族的颗粒度 | **R2 须定颗粒度** | B 单列 recursive_t、折叠其余三族，无判据（见 §1.2-2） |
| `topological-computation/` | **R3 出界须声明** | 独立研究系统 + 语料第二副本风险宿主（ADR 0017 附带发现建议「钉死」正本路径）；不进统一范围，但图上须显式出界声明 |
| `frontend/` | **R3 出界须声明** | 可视化消费端；bi.md U-13/xianduan.md V-6 已自登记「未核、无人跟进」——裁定自己知道漏了它，却停在「登记」未进清单 |
| `chanlun/` | **R3 出界须声明** | 工作草稿区；目录名有命名风险（近名 `newchan` 包、同名异物 `.chanlun/`） |
| `.chanlun/` | R3（已有声明） | #790 边界条已明确排除，维持 |
| `prototypes/` `experiments/` `external/` `spec/theorems/` | **R3 出界须声明** | 原型/实验/外部/定理 spec 区，零清单条目（bi.md U-13 已把 prototypes 列为未核区） |
| `tradingview-mcp-plugin/` `tradingview-chanlun.plugin` | **R3 出界须声明** | TV 集成侧（K4/TV 支柱），不在 K 线→下单链 |
| 根级散件 .py（`tws_margin_check.py`/`vcp_screener_v2.py`） | **R3 出界须声明** | K4 选股 / IBKR 执行支柱边角脚本 |
| `p7_inputs/` `references/` `nautilus_trader/`(空) | R3/R4 | 输入数据/模板/空目录，一行为宜 |
| `docs/` | R3 | 文档层（含五族架构稿） |
| `tmp/` `.scratch/` `.auto-memory-snapshot/` `src/newchan.egg-info`(未跟踪) | R4 | 工作垃圾/构建物 |

## 4. 高信号遗漏逐条（对 #960 的输入影响）

### 4.1 `rust/src/trading/`（organic-fugue-v2 交易层）——全仓最大单族 Python 消费面不在清单里

- 31 文件 / 34,431 行；`lib.rs` `mod trading;` 进编译树；GUARD-ROLE 头注释名分「现役」并记录两次核查（名分表原判 deprecated → #763 C7-E4 扩面复核 60+ 调用推翻）。
- pymodule 五导出（PyOrganicTape / run_organic_rust / run_recursive_rust / run_positional_rust / PyUnnStream），**96 个 .py 文件调用（analysis 91 + trading_system 5）——比被单列进范围条的 recursive_t（6 个调用文件）大 16 倍**。
- 裁定落点宿主：ADR 0017 T-11（`positional_fusion.rs:91-104` 的 `SUB_SPAWN_FRAC`、`unified_necessity.rs:993-995`）、`qujiantao.md:1062` 全仓首处生产代码伪引文（`positional.rs:458`）。
- 对 #960 的直接影响：乙层图要回答「同一套判定在每级复用同一个模块」，而 `trading/` 族是**缠论判定的第二大实现族**（#789 矩阵「前代」档引 legacy census §1.4 的 28 文件模式分派链，覆盖 unn/urs/nrf/iso/pcf/nif*/rnf/fusion_v* 全部变体；现目录 31 文件含 3 个 `#[cfg(test)]` 消融守卫）。它不进清单，收敛判据的「40 套口径」计数和「模块有哪些」题面就先天缺一块。

### 4.2 `rust/src/spiral/` + `rust/src/fugue_v3/`——#787 正文对二者只有一句「旧 ladder」引文

- #762 二次订正已把 spiral（fugue_v3 硬依赖其 signal 层）、fugue_v3（recursive_t flat 支硬依赖其会计层）改判**现役**；最近提交 2026-08-07（#943 双 λ 收敛）。
- ADR 0016 记录的两个互斥 λ 就在这两个模块里（`spiral/params.rs:32` `LAMBDA=2.0` vs `fugue_v3/mod.rs:111` `LAMBDA=3.0`）——**裁定的受影响代码清单点了名，模块清单没收**。
- #787 正文对二者的全部提及 = `theta_v0/mod.rs:13-18` 引文里的「旧 ladder」三个字 + #943 一句。正文从未给二者名分条目——这是「清单遗漏」的教科书形态：引文里有、清单里无。

### 4.3 `rust/src/bin/`——π 引擎的消费面、六区图上的「无区边」

- 53 个 .rs / 42,972 行；14 个 cargo [[bin]]（多数 `required-features=backtest_bin`）。
- `bi.md:342`（教义正本）逐字：「theta_v0（π）对本组文件生产引用 = 0 ⟹ 族I Rust 是生产主路径，**theta_v0 那套消费方全在 `rust/src/bin/*`**」——π 引擎（17.8 万行）的**全部消费面**住在一个清单里没有任何条目的模块里。
- 连带：#790 E9「theta_v0 → analysis/trading_system 死边候选」的起点就是这些 bin；`date_to_timestamp` 31 份复制（`qujiantao.md:1302`）有 30 份在 bin/ 下；近期全部探针票（#907/#911/#915/#938/#944）的探针代码都落在 `rust/src/bin/p*.rs`。
- 对 #960 的直接影响：乙层图画「谁消费谁的产出」时，π 的消费边全部指向一个无名节点。

### 4.4 `rust/tests/` + `tests/`——对拍锁的宿主在图上没有格

- #804 总缝规则明写行为不变式的「载体只能是测试锁」，而本仓全部对拍锁住在这两个目录：`rust/tests/`（`theta_v0_lean_parity.rs` 等 5 个 include_str! fixtures 的 parity 测试 + `fixtures/*.json`，formal↔rust E7 边的落点）与 `tests/`（Python 侧 `test_rust_*_equivalence.py` 九组逐位等价测试——#789 票面线索第一条点名要查它，其报告 §3.1 也实际核查了）。
- 六区图上 E7 是一条**有边无区**的边（起点 formal 有区，终点 `rust/tests/fixtures/` 没有区）。#960 若按「模块间边」切图，对拍边必须落在图上。

### 4.5 消费面清单的盲区：analysis/ 被订正过、scripts/ 从没被订正过

- #844 对 `analysis/` 做了名分订正（664 条目里近一半是文档），但**同一个测量没有覆盖 `scripts/`**——实测 `scripts/` 有 230 个 .py / 87,064 行，与 `analysis/`（314 .py / 98,898 行）同量级，且 `check_fixture_drift.py`（CI fixture-drift job 的执行者）就住在里面。
- #815 M-3 的 N-1 可达性举证（「detect_levels 上游全是 `scripts/` 研究脚本」）已经把 `scripts/` 当作研究脚本区在用，但三份清单里它一格不占。
- 另注意一个**同源异名**：`analysis/` 与 `scripts/` 是两个独立的研究脚本区，乙层图需要明说二者的关系（#790 E4 的 sys.path 注入只覆盖 `analysis/`）。

### 4.6 其余出界模块的一行注

`topological-computation/`（1,351 文件）是独立研究系统，但承载**课程语料第二副本**（109 份，行号体系与正本不同）——AGENTS.md 已钉正本路径，乙层图应把「语料正本在 `docs/chanlun/text/blog/`、副本在 `topological-computation/`」写进图注，防止未来探针再读到第二副本的假行号。`frontend/`、`prototypes/`、根级散件 .py 的判定同 §3.2。

## 5. 清单本身的质量问题（非遗漏，但影响 #960 直接使用）

1. **范围条引用漂移**（§1.2-1）：自称锚定 #790 六区，实际列出的六项 ≠ #790 六区。
2. **数字漂移**：疆域表 `formal/`「149 文件」vs 实测 154（144 .lean）；#844 订正的 analysis 数字（664/309/312）在本快照已漂到 698/348/314（两周 +34 文件）。#960 若引疆域表数字须重新实测。
3. **「8 层」清单 vs 17 个顶层 .rs**：疆域表 rust/src/*.rs 行写「8 层（bi/seg/zs/move/BSP/PH/MACD/Orchestrator）」，实测顶层 17 个 .rs，其中 `lib.rs`、`level`、`fractal`、`stroke`、`segment_layers`、`c_segment_verify`、`segment_tangency_tests`、`divergence`、`bi_zhongshu_bsp` 9 个不在「8 层」名单里（#790 名片记 17 文件 / 14 现役 + 2 单列，数字对但名单写法引歧义）。
4. **theta_v0 子格清单不完整**（§1.1 清单 A）：写 classifier/backtest/strategy/nautilus 四格，漏 `parser/`（5,688 行，**比 nautilus 适配层 1,569 行大 3.6 倍**）、`ledger/`（669）、`closed_loop/`（2,333）、`complete/`（733）、`venue_fee/`（286）五个子目录 + 六个顶层 `.rs`（`mod.rs`/`config.rs`/`env_registry.rs`/`lineage_book.rs`/`types.rs`/`venue_fee.rs`——末一个与子目录同名并存）。其中 `strategy/ledger.rs` 是 #914 测出的三阶段账本（`:149`）落点；`theta_v0/ledger/`（`mod.rs`/`separate.rs`）是 #849 查实的净额/逐仓对账所在区——两个 `ledger` 同名异物，且都不在疆域表的子格清单里。
5. **代际标签与名分标签混用**（§1.2-3）：#789 区代号「前代」档四族全被 #762 判现役——「前代」一词在乙层图讨论里会系统性误导「这四族该淘汰」。

## 6. 对 #960 的建议输入（不改图，只给判据）

1. **模块清单的生成规则**：取并集——三份清单 ∪ 裁定受影响代码清单点到路径的宿主模块 ∪ lib.rs pymodule 导出反查宿主模块（#762 方法论）∪ 编译树 `pub mod` 全家 ∪ CI 三个 job 的执行面（`scripts/check_fixture_drift.py`、`rust/tests/`）——再按甲层口径切判据模块。并集结果即本报告 §2 全集表，**其中 §3.2 判 R1 的 8 个模块是当前清单必须补的**。
2. **#960 建议先补三问**（grilling 素材，非本报告裁定）：① 「顶层散件」到底包不包四族（折叠读 vs 单列读，二选一写死）；② `analysis/` 与 `scripts/` 两个研究消费区在图上是一个节点还是两个（E4 只覆盖 analysis）；③ R3 出界模块的显式声明形式（一行一模块 + 理由，还是整段边界声明）。
3. **顺手可做的登记**：`bi.md` U-13 / `xianduan.md` V-6 的「未核 frontend/、trading_system/、prototypes/、NT 适配层，无人跟进」与本审计同源——#960 收口时若乙层图已覆盖这些区，该两处登记可一并销账。

## 090 照实

- **未跑 `cargo build`**：全部结论来自 `git ls-files` + 源码/导出表静态读 + 既有名分报告（#761–#764、#790、#844、GUARD-ROLE 头注释），无一条依赖编译（参照 #849 先例声明）。pymodule 导出表以 lib.rs:2681-2717 源码为据，未做编译验证。
- **行数是量级**：`wc -l` 含空行注释；theta_v0「17.8 万行」与 #787 正文「17 万行」的差即含注释口径差，不作定论。
- **未逐文件分类 `analysis/` 与 `scripts/`**：沿用 #844/#790 的既有结论（analysis 近半是文档），`scripts/` 的 230 个 .py 未逐份判别「研究脚本 vs 工具」，若 #960 需要 scripts/ 的细粒度名分须另做。
- **Python 调用面计数口径**：`--include='*.py'` 的 `grep -rl`，命中的是导出名在文件中出现（含注释/字符串的假阳性可能略高估；recursive_t 的 6 文件、trading 的 96 文件均为量级数）。trading 族 96 个文件的「真调用」口径以 #763 C7-E4 的扩面核查（60+ 处真实非测试调用）为旁证，本轮未逐份重验。
- **`spiral` 的 Python 调用面**：`PySpiralStream` 被 trading_system 的 2 个文件使用（`compare_spiral_unn.py`/`backtest_spiral_stream.py`，均与 UnnStream 流式对账）；`run_spiral` 无 .py 命中——spiral 的活度主要靠 fugue_v3 编译期硬依赖（#762 判据）加这 2 处 Python 面。
- **快照之外的变化**：本报告数字对应 main @ `a04e5f4a19`；#960 走图时若 main 已前进，§2 全集表的数字须重测（文件名与模块边界不受影响）。
