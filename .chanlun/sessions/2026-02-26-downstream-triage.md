# 下游推论分诊 + 异步自指审计

session: v58-swarm downstream-triage
date: 2026-02-26

## 任务1：218号下游推论标记（已完成）

- 218号-1: ceremony_scan.py 扩展 → **resolved**（commit 3412cfa）
- 218号-2: ceremony.md 缩减 → **resolved**（commit 7120720）

已在 `218-lead-parallelization.md` 下游推论节标注。

## 任务2：各号下游推论评估

### 218号-3：Codex API 不可用时不阻塞蜂群

**状态：未解决，需要设计决策**

当前实现（`src/newchan/codex/engine.py`）：
- 有模型级 fallback：gpt-5.3-codex → gpt-5.2-codex（5xx 错误时自动降级）
- 无 API 级 fallback：OpenAI API 整体不可用时（无 key、网络中断），直接抛异常
- `_create_client` 在 OPENAI_API_KEY 缺失时 raise ValueError
- `call_with_fallback` 仅捕获 `APIError` 做模型切换，不处理 API 完全不可达

218号-3 要求"Codex API 不可用时不阻塞蜂群——Gemini 单向质询可作为降级方案"。当前无此降级路径。

**四分法分类：选择** — 降级策略有多种合理方案：
1. codex 工位捕获异常后跳过，仅保留 Gemini 质询
2. codex 模块内部集成 Gemini fallback
3. ceremony_scan 层面检测 OPENAI_API_KEY 缺失时不生成 codex 工位

### 216号-1：异质诊断应包含实际代码验证步骤

**状态：待观察（部分基础设施已存在）**

ceremony_scan.py 的 VDW 机制（`validation_cmd` + `_run_validation_cmd`）为 roadmap 任务提供了验证驱动工位生成。但该机制作用于 roadmap 任务级别，不作用于 codex/gemini 诊断结果级别。

诊断结果（`ReviewResult`）目前无"已验证/未验证"标注。VDW 的 validation_cmd 模式可以扩展到诊断结果，但需要定义"验证"在诊断上下文中的具体含义。

### 216号-2：诊断结果应标注"已验证/未验证"

**状态：未解决**

`ReviewResult` dataclass（`src/newchan/codex/modes.py`）无 verified 字段。需要在 ReviewResult 或其下游消费者中增加验证状态标注。

**四分法分类：定理** — 216号的逻辑推论。实现方式明确：在 ReviewResult 增加 `verified: bool` 字段 + 验证步骤。但与 216号-1 耦合——先确定验证步骤的定义，再标注。

### 220号-1：risk 模块设计方向根本性改变

**状态：删除已完成，重建未开始**

`src/newchan/risk/` 已删除（commit c6a83c9）。220号要求的不是独立风控层，而是买卖点系统的扩展。当前 BSP 代码（`buysellpoint_engine.py`、`buysellpoint_state.py`）有结构性退出（`run_exit_idx/run_exit_side`），但这是走势结构退出，不是220号定义的交易退出条件。

**四分法分类：选择** — 需要设计决策：退出条件、仓位管理、短差程序如何集成到现有 BSP 架构中。

### 220号-2：回测引擎支持短差/成本归零/多级别联动

**状态：未解决，需要设计决策**

当前代码无短差程序模拟、成本归零追踪、多级别联动实现。

### 220号-3：退出条件实现应在 BuySellPoint 判定逻辑中

**状态：未解决，需要设计决策**

现有 `_collect_exit_segments`（`a_divergence_v1.py`）是走势离开中枢的结构分析，不是交易退出条件。220号定义的退出条件（买入程序判断条件被否定 → 退出）需要新的实现。

### 220号-4：仓位管理追踪当前阶段

**状态：未解决，需要设计决策**

无现有实现。三阶段（降成本/归零/挣股票）追踪需要新的状态机。

## 任务3：异步自指审计

**结论：scan 检测有误，218号确实被219号引用。**

dag.yaml 第2201行：`from: '219', to: '218'`
219号 frontmatter 第16行：`depends_on: ['217', '075', '218']`

218号在谱系 DAG 中被219号 depends_on 引用，不存在"未被任何后续谱系引用"的问题。

**误报原因推测**：scan 可能仅检查了 dag.yaml 的 nodes 部分而未检查 edges 部分，或检查时219号尚未写入 dag.yaml。
