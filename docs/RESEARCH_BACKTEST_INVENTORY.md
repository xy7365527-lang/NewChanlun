# 我们做过的工作清单（已核实版）

> **可信度声明**：本清单的每一条都来自项目持久记忆库 `~/.claude/.../memory/` 下的真实记忆文件（条目 slug 列在每条末尾的 `[slug]`，可逐条回溯核对）。
> 已**删除**所有无法核实的条目（包括上一版里我从文件名推断的 `_china`/`_us` 双版本报告、拓扑选股中美双市场、Markov regime、EURUSD/CL 曲率等——这些记忆库里没有直接支撑，我不能保证真实，故剔除）。
> 不含具体数字（多源自模拟/不同数据基底，不可比）。走向标注：✅成立 / ❌被证伪 / 🔧工程 / 🔬探索 / ⚠️方法论戒律。
> 编纂日期：2026-06-08

---

## 一、缠论引擎（底层工程）

1. **缠论引擎 Rust 重写** 🔧 — 把笔、线段、中枢、走势四层从 Python 逐位移植到 Rust，结果与 Python 完全一致（bit-exact）。`[project_engine_rust_rewrite]`
2. **递归编排器全 Rust 化** 🔧 — 把七层递归（级别涌现）整条链路搬到 Rust，逐 bar 驱动全链 + 短路缓存 + 递归栈。`[project_recursive_orchestrator_rust]`
3. **引擎逐 bar 增量化优化** 🔧 — 找出并修掉 6 个 O(N²) 性能热点，保持 bit-exact。`[project_engine_incremental_optimization]`
4. **笔中枢 O(N²) 性能墙排查** 🔧 — 定位真正的两处瓶颈（逐笔/每 bar 全量重算买卖点）。`[project_bi_zhongshu_on2_two_walls]`
5. **Segment（线段）resume 增量回归排查** 🔧 — 发现"续算"与"全量算"结果不一致的 bug，确认生产/回测曾跑在错误的线段上。`[project_segment_resume_regression]`
6. **MoveTuple 索引语义厘清** 🔧 — 搞清楚走势对象里的索引是"笔索引"而非"bar 索引"（时间锚定的关键）。`[project_move_tuple_stroke_index]`
7. **递归级别涌现边界研究** 🔬 — 研究一段行情最高能涌现到第几级（级别 ~ 时间尺度，长序列须用批量而非流式）。`[project_recursive_level_emergence]`
8. **配置 σ 本体论修正** 🔧 — 修正走势方向状态机的转换图（54 边 → 81 边，σ 可跳变）。`[project_config_sigma_ontology]`

---

## 二、交易策略回测

### 策略版本迭代（A→D→E→G/H→I）
9. **背驰门控进出场（版本 D）** ❌ — 用 MACD 背驰做进出场门控，结果跑输买入持有（门控=减暴露）。`[project_divergence_gate_entry_exit_falsified]`
10. **背驰定位器作主轴（版本 E）** ✅ — 把买卖点（次级别底背驰买、L2 顶背驰卖）当主轴而非过滤器，能跑赢买入持有。`[project_divergence_locator_entry_exit]`
11. **完整版赋格 G/H** 🔬 — 补全降成本的完整模块，效果依标的而定。`[project_complete_fugue_v2]`
12. **最佳组合版本 I** 🔬 — 多重赋格 + 动态级别归属（出场级别是强趋势收益主因）。`[project_best_combo_version_i]`
13. **版本 I 动态级别归属** 🔬 — 买点归最高涌现层、定出场级别 + 多重赋格降成本。`[project_version_i_dynamic_level]`
14. **版本 I 完整重写** 🔬 — 取消阉割、用真实 type1/2/3 买卖点重做，确认"线段是降成本最优层级"（印证缠师"多重赋格"第40课）。`[project_version_i_complete_rewrite]`
15. **多级别 C 路径** ❌ — 多级别递归（高层定方向+低层定背驰）择时，结果劣于单级别。`[project_multilevel_c_path_falsified]`

### 独立择时实验
16. **BTC 杠杆实验** ❌ — 测试 7 种杠杆/止损方案能否救活亏损策略，结论：问题在信号层，sizing 救不了。`[project_btc_leverage_signal_layer]`
17. **挣股数（降成本）实验** ❌ — 测试"持仓成本降到 0 后免费持有"，发现 1min 持仓太短从未触发。`[project_earning_shares_empty_domain]`
18. **降成本提款机 bug 排查** ⚠️ — 发现降成本的 trim 被错误建模成"只赚不赔"，污染了所有赋格类回测结果。`[project_costreduction_moneyprinter_bug]`

### 回测方法论
19. **swing 代理方向陷阱** ⚠️ — 用 swing 笔端点代理走势方向是标签错配（走势方向应由中枢定义）。`[project_trend_direction_proxy]`
20. **回测基准可证伪性反思** ⚠️ — "在超长上涨数据上说跑不赢买入持有"这种结论其实没有信息量，需随机门控对照。`[project_backtest_benchmark_falsifiability]`
21. **零前视回测改造** 🔧 — 用真实成交价消除回测里的"未来函数"。`[project_zero_lookahead_backtest]`

### M1 Rust 驱动回测（工程化）
22. **M1 E 版 Rust 回测** 🔧 — 因 Python 太慢（百万 bar 要几小时），改用 Rust 引擎驱动 E 版回测（标的 ES/Brent）。`[project_m1_e_rust_backtest]`
23. **M1 Version I Rust 引擎** 🔧 — Rust 驱动版本 I 完整版回测，多标的并行。`[project_m1_i_rust_engine]`
24. **OKLO E 基线漂移核对** 🔧 — 核对同一策略因数据重拉导致的基线收益漂移。`[project_oklo_e_baseline_drift]`
25. **OKLO 数据基底不匹配核对** ⚠️ — 同一策略因数据文件不同收益差一倍，提醒不同数据基底不可同表比较。`[project_oklo_data_basis_mismatch]`

---

## 三、资本流动 / 宏观市场研究

26. **资本旋转动力学框架** 🔬 — 提出 L=M×ω（金油比×货币）与 GDP 协整的框架，空转可拓扑检测。`[project_capital_rotation]`
27. **卢麒元框架整理** 🔬 — 资本三流、金油两锚、空转压价、沉没才能流转、价值回归。`[reference_lu_qiyuan]`
28. **ω regime → 美股方向** ❌ — 测试金油比 regime 能否预测美股方向，真实数据上方向是反的（且 M2 曾把否定结果包装成确认）。`[project_omega_regime_falsified]`
29. **COT 持仓领先性检验** ❌ — 测试 COT 期货持仓能否领先价格/流量，结论：不领先；缠论残差走势结构才是唯一有效流量源。`[project_cot_not_leading_flow]`
30. **K4 折叠通道模型** 🔬 — 把市场状态建模成四顶点（货币 M / 生产 P / 商品 C / 不动产 R）之间的折叠通道，金/油降为通道观测量。`[project_k4_fold_channel_model]`
31. **M2 K4 选股门控 e2e** ❌ — 把 K4 选股作为门控接入交易系统，发现门控反而毁掉 alpha。`[project_m2_k4_selection_gate]`
32. **全市场 K4 数据源映射 + 跨国数据管线** 🔧 — 梳理各市场数据源/分辨率（连续合约须 `.v.0`），建跨国数据管线（含中国本土 IF/沪金/在岸人民币代理）。`[project_global_k4_data_sources]`
33. **K4 1min 期货映射分歧** 🔬 — 1min 期货版 K4 顶点映射（R=国债、C=油）与正典不同，由编排者显式覆盖。`[project_k4_1min_futures_mapping]`
34. **最高级别 σ 作 regime 观测量的陷阱** ⚠️ — 发现用最高涌现级别方向做 regime 判别会被"跨年走势"冻结成常数，误导判断。`[project_highest_level_sigma_frozen]`

---

## 四、残差 / 曲率 / 流量流速研究

35. **残差→流量桥梁的理论裁决** ❌ — 用规范场论（杨-米尔斯）想从残差推资金流量，发现缺"金融度规"，桥断了，不能用 Hodge star；流量须独立源。`[project_residual_to_flow_no_bridge]`
36. **残差缠论流量流速实验**（本 session 亲手做） 🔬 — 对残差（DX+6E 1min）跑缠论递归，提取"流量"（力度/振幅）和"流速"（级别涌现速度）并对齐宏观转折点；结论：流速是空信号，流量只在内生反转处有信号。`[project_residual_flow_velocity_l2]`
37. **Γ→Δ 判别式全验证** 🔬 — 用最小周期让级别涌现、σ 取最高级别走势；1h 期货上复现原研究的增益。`[project_gamma_delta_full_verification]`
38. **Γ→Δ 30 分钟版** 🔬 — 在 30min 周期上做判别式，消除日线退化；"走资沉没非吸收态"等否定结论。`[project_gamma_delta_30m]`
39. **1min 分辨率不可约性** 🔬 — 否证"把周期细化就能补偿信息损失"的想法。`[project_1min_resolution_irreducible]`

---

## 五、拓扑 / 持久同调（PH）/ 领先性

40. **PH settle 与买卖点用途边界** 🔬 — 检验持久同调"结算"与缠论买卖点的关系；结论：是必要条件（candidate 层）但不是充分确认（confirmed 还需 MACD 背驰），用作门控是越界。`[project_ph_settle_usage_boundary]`
41. **PH 性能瓶颈排查** 🔧 — 确认 PH 本身不是性能瓶颈，真瓶颈是流式逐 bar 全量重算 O(N²)。`[project_ph_perf_streaming_oN2]`
42. **纵向圈闭合实测** 🔬 — 测试 ES（标普）→ Mag7/原油 的领先滞后关系（注：alpha 未回测）。`[project_vertical_closure_test]`

---

## 六、基础设施 / 方法论（支撑性）

43. **量化系统总纲领** 🔧 — 定义四大支柱（缠论引擎/PH 拓扑/K4 选股/IBKR 执行）+ 五里程碑（M1 回测→M5 加固），路线图见 `docs/ROADMAP.md`。`[project_quant_system_architecture]`
44. **多 session 并行资源调度** 🔧 — 解决多回测 session 抢资源的问题（改用 load 阈值而非 PID 门控）。`[feedback_pid_gating_fragile_multisession]`
45. **Rust 构建环境修复** 🔧 — 修复 `.local/bin/cc` 劫持破坏 cargo 链接的问题。`[reference_cc_hijack_rust_build]`
46. **停机守卫僵尸任务清理** 🔧 — 清理中断 session 遗留的 in_progress 僵尸任务。`[reference_stopguard_zombie_tasks]`
47. **Cowork 研究输出归档** 🔧 — 四轮定量研究的输出文件归档。`[reference_cowork_research]`
48. **用户实盘方向记录** 📌 — 押注金油比下降（油涨）、超大级别周线线段一买、2025-02-20 regime 断点。`[user_trading_direction]`

---

## 大局总结

- **引擎层（一、五部分工程）已扎实**：缠论四层 + 递归 + 持久同调都用 Rust 重写并验证（bit-exact），性能瓶颈也摸清。
- **策略层（二部分）**：版本 A→E→G/H→I 的长迭代，**最有效的是版本 E（背驰定位器作主轴）**；大量"加料"（背驰门控、降成本、杠杆、多级别）都被证伪——**加得越多反而越差**。
- **宏观层（三、四部分）野心最大、否定也最多**：金油比/ω/COT/残差→流量这些宏观桥梁基本都被真实数据证伪，目前**唯一站得住的流量来源是缠论残差自身的走势结构**。
- **整个项目最大的特点：否定性结果远多于确认性结果**——大部分市场假设被自己的回测打掉了，剩下几条经得起检验的窄结论。

---

> **本版本与上一版的区别**：上一版含约 30 条我从文件名推断、无法核实的条目（已全部删除）。本版 48 条**每一条都对应一个真实记忆条目**（末尾 `[slug]` 可在 `memory/` 目录逐条核对）。
> **仍需注意**：本清单证明"这些工作做过"，但**不保证其中每个实验用的是真实数据还是模拟数据**——部分实验（尤其早期宏观研究）用了合成/代理数据，结论等级见各记忆条目的 L0–L3 标注。要确认某项的数据真实性，需单独核对该项的原始脚本与数据缓存。
