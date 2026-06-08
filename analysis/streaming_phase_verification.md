# 第三阶段：测试不回归验证 + 修改文件汇总（team d3ac74c8 / task 6）

> 验证范围：streaming 引擎修复（5 项）+ 两阶段 streaming 回测脚本。
> 认识论等级：回归判定 L0（确定性 stash 对比）；回测结果 L2（单标的 QQQ）。

---

## 0. 回归判定（核心结论：零回归）

**方法**：`git stash` 掉 src/ 引擎跟踪改动 → 在 HEAD 基线跑相同失败文件 → 对比。

| 状态 | 全量 pytest tests/ | 失败文件子集（8 个）|
|---|---|---|
| 含引擎改动（工作树） | 37 failed, 4572 passed, 109 skipped | 37 failed, 111 passed |
| HEAD 基线（引擎改动 stash） | — | **37 failed**, 111 passed |

**结论**：含/不含引擎改动，失败数**完全一致（37）**，且全部集中在同 8 个文件 →
**这 37 个失败是预存的（pre-existing），引擎改动引入 0 个新回归**。

预存失败分布（均为基础设施/环境依赖，与缠论引擎无关）：

| 文件 | 失败数 | 性质 |
|---|---|---|
| test_live_pool.py | 12 | 实时数据池（需 live 连接） |
| test_reconnect.py | 9 | 断线重连（需 live gap 数据） |
| test_mcp_bridge.py | 6 | MCP bridge（需外部 server） |
| test_claude_audit.py | 4 | MCP server 调用 |
| test_backpressure.py | 2 | 背压（环境依赖） |
| test_topo_schema.py | 2 | 真实数据 schema（需缓存数据文件） |
| test_concept_registry.py | 1 | JSON 解码（需数据文件） |
| test_morse_landscape.py | 1 | 真实数据硬约束 |

> 缠论引擎核心测试（笔/线段/中枢/买卖点/online_persistence/settle_trigger/cost_reduction）
> **全部通过**——引擎修复未破坏任何缠论结构语义。

---

## 1. 修改文件汇总

### src/ 引擎修复（5 项，前序工位产出，本工位只验证）

| 文件 | 改动 | 对应问题 |
|---|---|---|
| `a_stroke.py` | `new_raw_gap_min` 参数（笔 gap 阈值，默认 3） | #4 笔 gap |
| `bi_engine.py` | 透传 `new_raw_gap_min`；Snapshot 暴露 `merged_to_raw` | #3 #4 |
| `core/recursion/buysellpoint_engine.py` | 接 `df_macd`/`merged_to_raw` 透传背驰 | #3 MACD |
| `orchestrator/recursive.py` | 透传 `new_raw_gap_min`；`enable_macd_divergence` | #3 #4 |
| `a_macd.py` | 增量 OnlineMacdState（流式 MACD） | #3 背驰 |
| `a_online_persistence.py` | settle 阈值读出扩展（settle 阶梯） | — |

### scripts/ + analysis/（两阶段回测）

| 文件 | 工位 | 状态 |
|---|---|---|
| `scripts/streaming_engine_verify.py` + `analysis/streaming_engine_verify_qqq.md` | task 4 | ✅ 运行通过 |
| `scripts/streaming_backtest.py` + `analysis/streaming_backtest_qqq.md` | task 5 | ✅ 运行通过 |
| `scripts/correct_multilevel_backtest.py` + `analysis/correct_multilevel_backtest_qqq.md` | 多级别（§5b streaming 对比） | ✅ 运行通过 |

---

## 2. 回测结果汇总

| 脚本 | 信号 | 后视 | 关键结果 |
|---|---|---|---|
| streaming_engine_verify | RecursiveOrchestrator（1000 根窗口） | 无 | gap≥3：72 笔/6 线段/1 中枢；confirmed=0（源不完备显现） |
| streaming_backtest（confirmed） | confirmed 买卖点 | 无 | A/B 组 0 交易（confirmed 不触发，定义缺口已显现） |
| correct_multilevel §5b（candidate） | streaming candidate | 无 | A_stream +184% / B1_stream +951% / B&H +1346% |
| correct_multilevel §2（TV-label） | TV 缠论指标 | **带后视** | A +2991%（含信号集后视） |

**跨脚本一致结论**：
1. **confirmed≈0** 是 v1 引擎结构性特征（type1 背驰 C 段恒空 + type3 关联 Move 强制
   unsettled），非 bug；streaming 缠论实际可交易形态 = candidate（右侧确认靠 PH settle）。
2. **信号集后视量化**：TV-label A（+2991%）− streaming candidate A_stream（+184%）= **+2807%**
   全部归因于 TV 指标重绘后视。去后视后缠论主级别**跑输 buy&hold**。
3. **PH settle 是过滤器非 alpha 源**：B1_stream（+951%）改善 A_stream（+184%）但仍跑输 B&H。

---

## 3. 结果包六要素

- **结论**：引擎修复零回归（§0）；两阶段 streaming 回测脚本运行通过；信号集后视已量化（§2）。
- **定义依据**：confirmed = 所在 Move.settled（a_buysellpoint_v1）；candidate = 买卖点出现即左侧候选
  （chanlun_ph_backtest 口径）；PH settle = §10 因果 settle 判据。
- **边界条件**：37 个预存失败若依赖的外部环境（live 连接/数据文件）变化，其状态可能改变——但
  与本工位引擎改动无因果关系；type2 候选补全后 candidate 覆盖变化，A/B 结果可能移动。
- **下游推论**：streaming 因果路径是认识论最高的可交易估计；confirmed 路径需先修 type3 触发缺口
  （a_zhongshu_v1 突破段定义，已标记独立诊断，按 no-workaround 未擅改）才有交易。
- **谱系引用**：267号（FSM）、§10 因果 settle、§17.3 规则3（PH 过滤器）、002号源不完备、426号
  （对象否定对象，PH 门控）。
- **影响声明**：本工位未改任何 src/ 或测试；新增本验证报告。引擎改动与回测脚本为前序工位产出，
  本工位仅做回归判定与运行验证。
