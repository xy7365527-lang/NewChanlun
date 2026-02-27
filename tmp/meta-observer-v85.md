# v85-swarm 二阶观察

date: 2026-02-27
type: meta-observation
swarm: v85-swarm
observer: meta-observer

rule_version_baseline:
  claude_md_commit: "e56ebbc"
  rules_dir_mtime: "2026-02-27 10:00:00 +0000"

---

## 观察 1（收敛信号）：v85 终结了"连续修复轮"模式

### 现象

v80-v84 连续五轮修复/审计轮（v80: 228号下游推论修复、v81: 审计修复5处、v82: 228号下游推论 resolved、v83: 代码实现+审计、v84: 审计缺口修复）。v84 meta-observer 判定"连续五轮无域推进是结构性信号"并指出"ceremony_scan 只检测修复债务，不生成战略推进"。

v85 使用 manual_dispatch 模式绕过 ceremony_scan 的干净终止，由编排者批准自主推进，执行了纲举目张分析后直接推进§31-33 交易管道集成。

### 判定

**v85 终结了连续修复轮模式。** 理由：

1. v85 的产出是**新代码模块**（pipeline.py 357行 + test_pipeline.py 357行），不是对已有代码的修复/声明修正/审计。这是 v76-swarm 以来（6轮后）第一个域推进轮。

2. 终结路径不是通过改进 ceremony_scan 实现的（v83/v84 meta-observer 建议的 roadmap_tasks 检测方向），而是通过 manual_dispatch 绕过 ceremony_scan。这意味着 ceremony_scan 的"默认修复存量"行为问题**未被修复，只是被绕过**。

3. v84 meta-observer 的边界条件"如果 v85 开始域推进而非继续修复，'连续修复轮'信号自然终止"——已满足。

### 边界条件

如果 v86 回到 ceremony_scan 驱动模式且再次产出修复轮，则：
- "连续修复轮"模式并未被终结，只是被 manual_dispatch 中断了一轮
- ceremony_scan 的"默认修复存量"行为需要被正式处理（语法记录或代码修改）

---

## 观察 2（发散信号）：manual_dispatch 绕过 ceremony_scan——workaround 还是正确行为？

### 现象

v85 的触发条件是 ceremony_scan 输出干净终止（无工位），编排者批准自主推进。Lead 使用 manual_dispatch 模式，绕过 ceremony_scan 直接 spawn 工位。

### 判定

**不构成 workaround**，但暴露了 ceremony_scan 的设计缺口。分析：

1. **workaround 的判别标准**（no-workaround.md）：workaround = 绕过概念矛盾让代码先跑通。v85 的情况不是概念矛盾——ceremony_scan 确实检测到"无工位"（干净终止），Lead 基于编排者的"自主推进"授权做出了"我来决定推进什么"的判断。这是一个**权限升级**（Lead 临时获得编排者的方向决策权），不是概念绕过。

2. **ceremony_scan 的设计缺口**：ceremony_scan 的扫描维度只有（a）谱系未结算项、（b）下游推论 unresolved、（c）spec-execution-gap 审计残留。它不包含（d）战略目标推进检测。当 a/b/c 全部干净时，ceremony_scan 输出"无工位"——但这不意味着蜂群无事可做。v85 证明了存在 ceremony_scan 无法检测但有价值的工作。

3. **v83/v84 meta-observer 已识别此缺口**：多次建议"ceremony_scan 增加 roadmap_tasks 检测"。v85 没有修复这个缺口，而是用 manual_dispatch 绕过。从功能上看是正确的（产出了有价值的管道集成代码），但从架构上看是**手动补偿了自动化的不足**。

### 分类

四分法：**选择**——ceremony_scan 是否应增加战略推进检测是一个设计选择，不是定理推导可决定的。但 v83/v84 meta-observer 已经提出了这个选择且连续三轮未被处理。

---

## 观察 3（设计质量）：TradingContext 的可变/不可变混合设计

### 现象

pipeline.py 的 TradingContext 使用 `@dataclass(slots=True)` 但**未使用 `frozen=True`**。不可变语义通过约定（管道函数创建新 ctx 而非修改原 ctx）和 `_replace_ctx` 辅助函数实现，而非语言层面强制。

其中 CostTracker 被明确标记为"唯一的可变组件"——管道函数在操作前 deep copy 以保证旧 ctx 不变（pipeline.py 第7-8行、第98行、第254行、第282行）。

### 判定

**这是一个有意的设计权衡，不是疏忽，但与仓库惯例存在张力。**

1. **仓库惯例**：本仓库的 dataclass 惯例是 `frozen=True, slots=True`（通过 grep 确认：`a_topology.py`、`a_stroke.py`、`backtest.py`、`nesting/bsp.py`、`nesting/horizontal.py`、`trading/state_machine.py`、`trading/position_sizing.py` 等模块均使用 `frozen=True`）。TradingContext 是罕见的不使用 `frozen=True` 的 dataclass。

2. **不使用 frozen 的原因**：CostTracker 是可变的（内部维护 `_entries` 和 `_exits` 列表，通过 `add_entry()`/`add_exit()` 方法原位修改）。如果 TradingContext 使用 `frozen=True`，它无法持有可变的 CostTracker 引用——Python 的 `frozen=True` 只冻结字段赋值，不冻结字段引用的可变对象的内部修改。因此 `frozen=True` 在这里提供的是**虚假的不可变保证**——字段不可重新赋值，但 `ctx.cost_tracker.add_entry(x)` 仍会修改内部状态。

3. **实际不可变性**：pipeline.py 通过 `_replace_ctx` 函数和 `copy.deepcopy(ctx.cost_tracker)` 实现了**运行时不可变语义**。每次管道函数操作前 deep copy CostTracker，确保旧 ctx 的 cost_tracker 不受影响。测试 `test_context_immutability` 验证了原 ctx 的 horizontal 不变（但未验证 cost_tracker 不变）。

4. **严格性评估**：
   - 优点：docstring 第7-8行明确声明"CostTracker 是唯一的可变组件"——声明与实际一致（090号严格性）
   - 缺点：不可变语义依赖约定而非编译器/运行时强制——如果未来开发者在管道函数中忘记 deep copy，旧 ctx 会被静默修改
   - 替代方案：将 CostTracker 改为 frozen dataclass（每次操作返回新实例而非原位修改）→ TradingContext 可使用 `frozen=True` → 不可变性从约定升级为语言强制

### 边界条件

如果 CostTracker 被重构为不可变设计（frozen dataclass + replace），这个观察自然收敛。如果 CostTracker 保持可变，则 pipeline.py 的 deep copy 约定是正确的补偿手段，但需要测试覆盖来守护（当前 `test_context_immutability` 仅验证 horizontal 不变，未验证 cost_tracker 不变）。

---

## 观察 4（收敛信号）：三模块连接的完整性

### 现象

pipeline.py 连接了三个模块：
- topology (config_space.Configuration, polarity_index) → scan_configuration
- nesting (horizontal.advance, bsp.is_negated, resonance.*) → locate_bsp, compute_position, check_negation
- trading (FourLayerStateMachine, CostTracker, PositionSizer) → execute_entry, execute_exit

### 判定

**连接完整，无未连接接口。** 具体验证：

| §章节 | 概念 | pipeline 函数 | 连接状态 |
|-------|------|-------------|---------|
| §31 配置扫描 | polarity_index → 方向 | scan_configuration | 已连接 |
| §31 区间套定位 | advance → 横向推进 | locate_bsp | 已连接 |
| §32 共振仓位 | resonance_check + classify + strength → position | compute_position | 已连接 |
| §33 入场 | state_machine.entry + cost_tracker.add_entry | execute_entry | 已连接 |
| §33 退场 | state_machine.exit_normal/exit_abort + cost_tracker.add_exit | execute_exit | 已连接 |
| §33 否定 | is_negated | check_negation | 已连接 |
| 聚合 | locate → position → entry 单步 | pipeline_step | 已连接 |

**未覆盖但在管道外部的概念**：
- 降成本追踪的"传递效率 eta"计算（cost_reduction.py 有定义，pipeline 只使用 add_entry/add_exit，未调用传递效率函数）——这是合理的：传递效率是回测/报告层面的概念，不属于单步管道推进
- `nesting.reverse_propagation`（反向传播）——pipeline 当前是正向管道，反向传播是独立的信号传递方向

---

## 观察 5（设计质量）：测试覆盖的充分性

### 现象

25个测试，8个测试类，覆盖了 pipeline.py 的全部 8 个公共函数（7个纯函数 + create_context 工厂）。

### 判定

**基本路径覆盖充分，但存在两个未覆盖的边界场景**：

| 覆盖的路径 | 测试 |
|-----------|------|
| 工厂创建（3个配置） | TestCreateContext (3) |
| 配置扫描三方向 | TestScanConfiguration (3) |
| BSP 推进+阻塞+六步完成+不可变性 | TestLocateBSP (4) |
| 三层/单层/无共振 | TestComputePosition (3) |
| L0入场+L1公理违反+成本记录 | TestExecuteEntry (3) |
| 正常退出+止损退出+级联退出 | TestExecuteExit (3) |
| 否定/未否定 | TestCheckNegation (2) |
| 单步推进+等待+完整买入+完整止损 | TestPipelineStep (4) |

**未覆盖的边界场景**：

1. **CostTracker 不可变性**：`test_context_immutability` 仅验证 `ctx.horizontal` 在 `locate_bsp` 后不变，未验证 `execute_entry`/`execute_exit` 后原 ctx 的 `cost_tracker` 不变。考虑到观察3指出的 deep copy 约定风险，这是一个有价值的测试缺口。

2. **pipeline_step 的 execute_entry 路径**：`pipeline_step` 内部有条件入场逻辑（第340-349行：horizontal.is_complete + resonance_signals + position > 0），但 `test_full_pipeline_buy` 没有通过 `pipeline_step` 走完这条路径（它直接调用了 `locate_bsp` + `execute_entry`），`test_step_with_bsp_advances` 也只走到了第一步（horizontal 未完成，不触发入场）。pipeline_step 的条件入场路径在25个测试中**未被直接测试**。

### 严重程度

低。两个未覆盖场景不影响当前功能正确性：
- CostTracker 不可变性已由 deep copy 代码保证，缺少的只是守护测试
- pipeline_step 的条件入场路径的各子步骤已被单独测试覆盖（locate_bsp + compute_position + execute_entry 各自有测试），聚合函数的路径覆盖是锦上添花

---

## 观察 6（自环检查）：228-1 步骤7-10 误伤问题

### 现象

v84 meta-observer 核心发现1："228-1 resolved 状态不一致是唯一的持续发散信号，已连续三轮观察（v82 发现、v83 未处理、v84 未处理且 P6 修复增加了检测盲区）。"

### 判定

v85 未处理 228-1。这是第四轮未处理。但 v85 是域推进轮（manual_dispatch 模式），不包含 hook/审计修复工作——跳过是因果上合理的。

228-1 的发散状态是否需要升级？标准：如果一个发散信号连续 N 轮未处理，且处理方案已知（v82 提出 B-v2/B-v3），则应被下一个 ceremony_scan 轮检出并处理。但 ceremony_scan 无法检出 228-1 的状态不一致——因为 P6 修复后 YAML status: resolved 在优先级链最高位。

这形成了一个**检测盲区闭环**：228-1 的 resolved 状态不准确 → P6 让 YAML status 优先 → downstream_audit 不检出 → ceremony_scan 不生成工位 → 228-1 永远不被处理。除非有人手动将 228-1 的 YAML status 降级为 partially_resolved，或者修复 ceremony-step-guard.sh 中的步骤7-10 逻辑。

### 分类

四分法：**行动**——方案已知（修改 YAML status 或修复 hook），不需要编排者决策。但这个行动需要有人执行——在 manual_dispatch 模式下不被自动检出。

---

## 自环检查

| 历史观察 | v85 状态 | 判定 |
|---------|---------|------|
| 连续修复轮（v80-v84 五轮） | **已终结**——v85 是域推进轮 | 收敛（但通过 manual_dispatch 绕过而非修复 ceremony_scan） |
| 228-1 步骤7-10 误伤 | **未解决**——第四轮未处理 | 持续发散 + 检测盲区闭环 |
| ceremony_scan 只检测修复债务不生成战略推进 | **未解决**——被 manual_dispatch 绕过 | 持续（问题存在但不阻塞功能） |
| P6 YAML 自声明优先级设计风险 | **未扩大**——v85 未新增 YAML frontmatter 谱系 | 维持 |
| spec-execution-gap 同构模式 036号 | **未触发**——v85 无新声明-能力不一致发现 | 维持待观察 |

## 核心发现

1. **v85 终结了连续修复轮模式**，但终结方式是 manual_dispatch 绕过而非修复 ceremony_scan 的检测缺口。ceremony_scan 的"默认修复存量不推进战略"行为仍是未处理的语法记录候选——已连续三轮 meta-observer 指出。

2. **pipeline.py 的设计质量总体良好**：7个纯函数 + 不可变语义 + 明确的三模块连接。CostTracker 的可变性是有意的设计权衡且声明充分，但与仓库 `frozen=True` 惯例存在张力。测试覆盖率充分但 CostTracker 不可变性守护和 pipeline_step 条件入场路径存在边界缺口。

3. **228-1 检测盲区闭环**是新发现：P6 修复创造的 YAML 优先级 + 228-1 不准确的 resolved 状态 = downstream_audit 永远不检出 = ceremony_scan 永远不生成修复工位。这是一个自我强化的盲区，需要外部干预才能打破。

## 下游推论

1. ceremony_scan 的战略推进检测缺口应被正式记录为**语法记录候选**（已连续三轮 meta-observer 建议，v85 用 manual_dispatch 绕过进一步证实了缺口的存在）
2. 228-1 的检测盲区闭环需要外部干预——要么修复 ceremony-step-guard.sh 步骤7-10 逻辑，要么将 228-1 YAML status 降级为 partially_resolved
3. pipeline.py 的 CostTracker 不可变性应增加守护测试（`execute_entry`/`execute_exit` 后原 ctx 的 `cost_tracker.total_entries` 不变）
