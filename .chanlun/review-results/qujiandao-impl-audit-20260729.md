# 区间套（多级别嵌套背驰）实现状态核查

**日期**：2026-07-29
**范围**：纯调研，不改仓；亲手核查（未派 Task/子代理）
**触发背景**：`.chanlun/definitions/beichi.md:298` 声称区间套"已实现"，但 `grep -rn nested_divergence_search rust/src/` 零命中；
#647 定稿后"入场时点力度"教义正解需要一份现状清单。

## 统计口径（可复核）

- grep 关键词：`nested_divergence_search` `divergences_in_bar_range` `NestCertificate` `n_delta` `chi_bool`
  `select_theta` `interval_necessity_tower` `nest_chain_complete` `d_top` `judge_divergence`
  `trend_diverging_segment` `a_nested_divergence` `a_xiaozhuan_da` `a_level_fsm_newchan` `nested_pipeline`
- 文件覆盖：`rust/src/**/*.rs` 全量；`src/newchan/**/*.py` + `tests/*.py` + `scripts/*.py` + `analysis/*.py` 全量（用
  `grep -rln` 排除 `tests/`/`/tests/` 判断"非测试引用"）
- 接线判定方法：对每个候选符号，`grep -rn "<符号>("` 找调用点，人工核对调用点是否位于
  `#[cfg(test)]`/`mod tests`/`tests/*.py` 内；Rust 侧另沿调用链核实是否被
  `theta_v0/backtest/runner.rs`（当前生产 `pi_theta_step` runner）或 `recursive_t`（T 引擎，经 PyO3
  `run_recursive_t`/`PyRecStream` 导出）实际到达。

---

## 1. 真实实现清单

### 1.1 Rust（`rust/src/`）

| 函数/模块 | 位置 | 功能一句话 | 接线状态 |
|---|---|---|---|
| `NestCertificate` / `NestRung` / `n_delta()` | `theta_v0/classifier/nest.rs` | N^δ 良基递归证书：装配多级 rung 链，`n_delta()` 判定区间套整体是否成立（0/1） | **判据路径**——经 `econ_positive.rs::build_gate_certificate` → `admission.rs::chain_driven_level_projection` → `backtest/runner.rs`（当前 `pi_theta_step` 生产 runner），是活跃的区间套生产门 |
| `cand_predicate.rs`（Cand 谓词） | `theta_v0/classifier/cand_predicate.rs` | 单级 Cand=0/1，喂入 `NestRung.cand`，任一级 Cand=0 ⟹ `n_delta()=0` | **判据路径**——`n_delta()` 判定的直接前置输入 |
| `nest_index.rs` | `theta_v0/classifier/nest_index.rs` | `NestCertificate` 的索引/查表读出层，声明"判定谓词唯一来源不变" | **判据路径**（支撑层，非独立判据） |
| `IntervalNecessity` / `interval_necessity_tower()` | `theta_v0/classifier/interval_necessity.rs` | 计算某分类结果的多级必要条件塔 | **仅测试用/孤立**——`grep -rn` 全仓零外部调用者，仅本文件 4 处测试自调用 |
| `Interval` / `NestLevel` / `chi_bool()` / `select_theta()` | `theta_v0/strategy/nest.rs` | 区间套确认 N^δ 的"v1"实现（Sel_Θ 选优 + χ 递归证书），供 `strategy::interp::assemble_gamma` → `strategy::recognize()` 消费 | **仅测试用（已退役路径）**——`recognize()` 的所有调用点核实后均在 `strategy/mod.rs` 的 `#[cfg(test)]` 块内；`runner.rs:181` 注释明确"已退役 v1 runner 走 `strategy::recognize`……v1 家族已由 #499 退役"。代码未删除，但不在当前生产判据链上 |
| `d_top()` / `level_signal_is_buy()` / `divergence_window()` / `select_child_in_window()` | `recursive_t/divergence.rs` | 区间套链贯通递归触发器：每级别独立腿消费该级别 d_top（读法乙），逐层向下窗口收缩 | **判据路径（另一条，非 theta_v0 主线）**——被 `recursive_t/rec_stream.rs:375` 调用；`recursive_t` 经 `lib.rs` 的 `run_recursive_t`/`PyTFugueStream`/`PyRecStream` 走 PyO3 导出，是独立于 theta_v0 backtest 的"T 引擎"生产路径 |
| `nest_chain_complete()` | `recursive_t/divergence.rs:539` | 区间套链贯通判定（`d_top` 的非诊断版原型，逻辑与 `d_top` 重复） | **孤立死代码**——全仓 grep 零外部调用者；`d_top()` 并未调用它，而是内联重写了同一套逻辑（各自独立维护） |
| `judge_divergence()` / `trend_diverging_segment()` | `recursive_t/divergence.rs` | 走势完美判据（背驰触发 type1）/ 背驰段判据（去创新高） | **判据路径（T 引擎）**——`operator.rs`、`rec_stream.rs`、`t_engine_run.rs` 调用 |

### 1.2 Python（`src/newchan/`）

| 函数/模块 | 位置 | 功能一句话 | 接线状态 |
|---|---|---|---|
| `divergences_from_level` / `_detect_trend_divergence` / `_detect_consolidation_divergence` | `a_divergence.py` | 单级别背驰/盘整背驰主检测（beichi.md 核心定义的直接实现） | **判据路径**——被 `a_buysellpoint_v1.py`、`a_recursive_engine.py`、`a_topology.py`、`a_xiaozhuan_da.py`、多个 `analysis/*.py` 回测脚本引用 |
| `divergences_in_bar_range()` | `a_divergence_v1.py:612` | 区间套单级别检测入口——限定 bar 范围内的背驰检测（复用检测逻辑零修改） | **判据路径**——被 `a_nested_divergence.py:417` 调用，同时多个 `analysis/*_backtest*.py` 直接引用 v1 版本主检测 |
| `nested_divergence_search()` | `a_nested_divergence.py:465` | 区间套主编排：从最高递归级别向下搜索到 level 1（自下而上构造、自上而下搜索） | **接入 observability API，未接入实盘决策**——被 `nested_pipeline.py::run_nested_search` 包装后接入 `server.py` 的 `/api/nested_divergence`（query 参数 `include_nested_divergence` 门控，默认关闭）与 `ab_bridge_newchan.py`；`topology/multi_tf_adapter.py` 亦调用。全仓未发现任何 Nautilus Trader 策略文件引用该函数——即不在实盘/回测下单决策路径上，仅作诊断/展示端点 |
| `a_xiaozhuan_da.py`（小转大跨级别联动） | `src/newchan/a_xiaozhuan_da.py` | 本级别未背驰但次级别突发背驰触发本级别走势终结 | **判据路径**——接入 `orchestrator/xiaozhuan_da_orchestrator.py` |
| `a_level_fsm_newchan.py`（级别递归引擎） | `src/newchan/a_level_fsm_newchan.py` | 级别递归构造，区间套跨级别坐标系的前提设施 | **判据路径**——接入 `a_recursive_engine.py`、`orchestrator/recursive.py`、`ab_bridge_newchan.py` |
| `dif_peak_for_range()` / `histogram_peak_for_range()` | `a_divergence_v1.py` | T6/T7 工具函数：DIF 峰值 / HIST 峰值 | **判据路径**——已集成到 `_detect_trend_divergence`/`_detect_consolidation_divergence`（OR 关系三维度之二，见 beichi.md 已结算问题#2） |

**结论对照 beichi.md:298 的具体断言**——"跨级别 `nested_divergence_search`"**函数确实存在**（`src/newchan/a_nested_divergence.py:465`），只是该函数在 **Python** 侧而非 Rust 侧；#647 触发本次核查的调研者只在 `rust/src/` 下 grep，方法论上搜错了目录，并非该函数不存在。但需订正的是它的**接线深度**：仅到 observability API，未到任何交易决策路径（Python 或 Rust）。

---

## 2. beichi.md「已实现」表逐行核对

对照 `.chanlun/definitions/beichi.md:294-303` 的表格，逐行给出 真/假/部分 判定：

| 原文要求 | beichi.md 声称 | 核查判定 | 依据 |
|---|---|---|---|
| A/B/C三段力度比较 | ✅ 已实现 | **真** | `a_divergence.py::_compute_force` 落地，`_detect_trend_divergence`/`_detect_consolidation_divergence` 消费 |
| MACD面积比较 | ✅ 已实现 | **真** | `macd_area_for_range` 存在且被消费（本次未逐字节复核内部公式，仅核对接线） |
| 黄白线回拉0轴检查 | ✅ 已实现（T4） | **真** | `a_divergence_v1.py::_b_segment_crosses_zero()`，`_detect_trend_divergence` 前提检查 |
| 黄白线创新高检查 | ✅ 工具函数已实现（T6），未集成 | **真（表述本身已如实标注未集成，无需订正）** | `dif_peak_for_range()` 存在；beichi.md 后文"已结算问题#2"记录了后续已集成，表格该行的"未集成"是历史快照，与下文#2 结算结论有轻微不同步（非核查判定的"假"，是文档内部前后不一致） |
| 柱子伸长高度比较 | ✅ 工具函数已实现（T7），未集成 | 同上 | 同上，`histogram_peak_for_range()` |
| 盘整背驰 | ✅ 已实现 | **真** | `_detect_consolidation_divergence` 落地 |
| **区间套（多级别嵌套）** | ✅ 已实现：单级别 `divergences_in_bar_range` + 跨级别 `nested_divergence_search` | **部分真**——两个函数都**确实存在**（Python 侧，`a_divergence_v1.py:612` / `a_nested_divergence.py:465`），单元测试齐全（18+20 项 GREEN）；但"已实现"未标注**接线深度**——仅到 observability API（`server.py` 可选 query 参数），未接入任何交易决策路径（Python 无 NT 策略引用，Rust 无对应实现）。表格断言的是"函数存在性"，若读者据此推断"区间套已参与实盘/回测下单决策"则**假** | 见 1.2 节接线状态列 |
| 小转大 | ✅ 已实现 | **真** | `a_xiaozhuan_da.py` 接入 `xiaozhuan_da_orchestrator.py` |
| confirmed时机 | ✅ 基本实现 | **未能判定**（本次未深入复核 confirmed 状态机的具体触发时点是否符合"跟随trend"原文口径，需要单独的时序核查，超出本任务范围） | — |

**净计数**：8 行核对，**真 6**，**部分真 1**（区间套行——函数存在但接线深度未如实标注），**未能判定 1**（confirmed时机，需专项核查）。无"假"（无"函数不存在但声称已实现"的情况）。

**重要澄清**：beichi.md 本身的技术描述并非虚假陈述——它没有撒谎说函数存在于 Rust；问题出在 #647 触发本次核查的调研者的检索方法（只查了 `rust/src/`），以及 beichi.md 的"✅ 已实现"标签**未注明实现语言/层级/接线深度**，导致跨 session 读者（含 Rust-only 检索的 agent）产生"零命中=未实现"的误判。**建议**（仅供参考，不在本任务授权范围内执行）：该表格行补充"语言：Python；接线：observability API，非交易决策路径"的限定语。

---

## 3. 复用评估：入场时点力度 = 小级别盘背（79课口径）差距清单

目标判据：不看价格阈值，看决策时点次级别是否处于盘整背驰（79 课）；精确定位用 27 课区间套逐级收缩。仅列差距，不做设计。

**可直接复用**：
- `a_divergence.py::_detect_consolidation_divergence`——盘整背驰的单级别判定逻辑本身就是目标判据的核心谓词，无需重写
- `a_divergence_v1.py::divergences_in_bar_range`——限定 bar 范围内检测的接口形状（level_id + bar_range）与"次级别在决策时点范围内是否盘背"的需求同构
- `a_level_fsm_newchan.py`——级别递归引擎已提供跨级别 bar 坐标系映射，区间套/入场力度判据都需要这一层

**缺环节（未落地，需要新工作）**：
1. **决策时点可判性**——现有 `nested_divergence_search`/`divergences_in_bar_range` 均为**全窗回溯式**调用（喂入完整 snapshot 后一次性搜索），未见任何"给定当前 bar 索引 i，只用 ≤i 数据判定次级别是否盘背"的因果封装。Rust 侧 `theta_v0` 有对应的因果范式（`classify_with_tower` + `newly_confirmed_step`，`runner.rs` 注释里的 [A] 适配器），但 Python 区间套侧没有类比物——需要新写一个"前缀因果版" `divergences_in_bar_range` 变体
2. **防未来函数（no-lookahead）**——`_detect_consolidation_divergence`/`_detect_trend_divergence` 判定盘整背驰需要"两次同向离开段"都已走完，若用于**入场时点**判据，需要验证：判定所需的最后一段离开段的终点是否严格 ≤ 决策 bar，现有实现未见此类边界断言/测试（18 个 `divergences_in_bar_range` 测试场景聚焦"完全落入 bar_range 过滤"，未见"决策时点 = bar_range 右端点"的因果测试用例）
3. **段边界在决策时点的可判性**——次级别盘整背驰依赖"当下笔/线段"已完成分型确认；若决策时点恰好落在次级别笔/线段构造中途（未确认），现有 `a_level_fsm_newchan.py` 是否能给出"部分构造"下的稳定读出，本次未核实，需专项测试
4. **Rust 侧对等实现缺失**——若"生产路径"最终要落地 Rust（`theta_v0`/`pi_theta_step`），Python 侧的 `_detect_consolidation_divergence` 判据尚无 Rust 移植；Rust `theta_v0/classifier/divergence.rs` 已有 `segments_diverge`/`segments_diverge_or` 等力度比较原语，可作为移植起点，但"次级别 vs 决策级别"的跨级映射需要复用 `recursive_tower.rs` 而非现有 `nest.rs`（`nest.rs` 是 N^δ 区间套证书链，语义是"逐级收缩验证转折点"，不是"任意时点查次级别盘背状态"——两者判据形状不同，不能直接套用）

**一句话结论**（供 stdout 摘要引用）：核心判据函数（盘整背驰检测）已存在且可直接复用，但"因果性封装"和"决策时点可判性"两个环节完全空缺，且 Rust 生产路径无对等实现——这是新工作的主体，非集成工作。

---

## 4. 定义层谱系：区间套（27课）与24课面积口径的关系

半页梳理（beichi.md 原文谱系节 + 已结算问题#5）：

- **24课**是背驰的**奠基定义**：单级别 A/B/C 三段力度比较（面积/振幅口径），产出"背驰-买卖点定理"（充要条件：任一背驰⟺某级别买卖点）。这是区间套的**基础谓词**——区间套的每一层收缩，用的仍是24课这套单级别判据，只是级别参数变化。
- **27课**在24课之上叠加**跨级别递归结构**：区间套定理本身不引入新的力度判据，而是声称"低级别背驰是高级别背驰的**必要条件**（非充分）"——这是一个**关系公理**，不是判据公理。操作上：从大级别背驰段出发，在其内部（时间范围子集）用次级别24课判据再验一次，逐级收缩直到最低可观察级别。
- **33课**补了一层解释：从中枢角度看，背驰的本质是"最后中枢向上/向下离开力度不对称"，这与24课的 A/B/C 力度比较是**同一构造在不同视角下的重述**（同构，非新增判据）。
- **37课**补充了区间套的递归终止/延展条件：背驰段c内部若形成次级别趋势（≥2次级中枢），可继续套用 a+A+b+B+c 递推——这是27课递归过程的**边界情形补丁**，不是新公理。

**关系总结**：24课=判据（What：如何判定一次背驰），27课=结构（How：如何把判据在级别间递归收缩定位转折点）。区间套不是"另一种背驰"，而是"24课判据在多个级别上分别独立成立 + 时间范围逐级收缩"的**组合调用协议**。这也解释了为何本次核查的复用评估（第3节）结论是"核心判据可直接复用"——因为24课判据从代码层面看就是 `_detect_consolidation_divergence` 本身，27课区间套结构则是 `nested_divergence_search` 的外层编排，两者在代码中确实是分离的两层，与定义层的分层完全对应。

---

## 附：未能判定项

- beichi.md「confirmed时机」行——"跟随trend的confirmed状态"的具体触发时点是否符合原文口径，本次未深入代码内部状态机逻辑核实，需专项核查。
- `theta_v0/classifier/recursive_tower.rs` 中 `compose_level`/`detect_centers_windowed_resume` 等函数与"次级别在任意决策时点的可判性"之间的精确复用关系，本次仅确认该文件是塔构造基础设施、被 `nest.rs`/`interval_necessity.rs` 引用，未逐函数核实是否已支持"任意 bar 索引查询"语义（第3节差距清单#1的判断基于 grep 未发现对应函数名，非逐行读码穷尽排除）。
