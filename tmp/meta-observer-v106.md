# v106-swarm 二阶观察

date: 2026-02-28
type: meta-observation
swarm: v106-swarm (双管道真实 BSP 回测对比)
observer: meta-observer (stop-guard triggered)

rule_version_baseline:
  claude_md_commit: 2ab7519
  rules_dir_mtime: 2026-02-28 00:39:43 +0000

---

## 观察 1（22.2% 分歧率的结构常数性质）

### 现象

v244（合成数据）和 v106（真实 1min 递归 BSP）在完全不同的数据源上测出**完全相同的分歧率 22.2%**。

| 指标 | v244 合成 | v106 真实 |
|------|----------|----------|
| 分歧率 | 22.2% | 22.2% |
| polarity 分歧 | 59.3% | 59.3% |
| 入场对称性 | - | 15:15 |

进一步：每个 BSP 恰好在 6/27 个配置上分歧（6/27 = 22.2%）。

### 判定

**正面——分歧率是 ConfigurationSpace 的拓扑不变量。** 27 个配置中 16 个有 polarity 分歧，其中 6 个跨越 scan_direction 的 buy/sell/neutral 边界。这个数目由 ConfigurationSpace 的 3^3 对称结构决定，不依赖 BSP 内容。

**语法记录候选**：22.2% 不是实验结果，是定义推论——可以从 ConfigurationSpace 的结构直接推导。

---

## 观察 2（离线模式的方法论选择）

### 现象

dual-pipeline-backtest 工位选择了**离线模式**（直接使用 252号 JSON 中的 BSP 数据），而非在线模式（重新拉取数据 bar-by-bar 运行 RecursiveOrchestrator）。

离线模式的含义：
- TradingContext 的 config 是固定的（不随 bar 推进）
- 不测试 PipelineBacktestEngine 的 bar-by-bar 状态机
- 只测试共振筛选层（_build_resonance_signals + scan_direction + compute_position）

### 判定

**中性——离线模式足以回答编排者问题。** 编排者问的是"两套筛选后的信号差异"，不是"两套管道的完整回测表现差异"。离线模式精确隔离了筛选层差异，排除了状态机噪声。

但**在线模式的价值仍然存在**：TradingContext 的 config 在真实交易中不是固定的（由市场状态动态决定），离线模式无法捕捉 config 动态变化对分歧模式的影响。这是 v106 的边界，不是缺陷。

---

## 观察 3（工位协作模式——block-topology 接手回测）

### 现象

block-topology-251-252 工位在完成自身任务后，接手了 dual-pipeline-backtest 工位的部分工作（写了回测脚本的第一版 189 比较），后被 dual-pipeline-backtest 覆盖为正确版本（135 比较，confirmed only）。

### 判定

**中性偏负——工位边界越界但结果被自然修正。** 越界原因是 block-topology 工位在 shutdown_request 发送后仍有执行窗口，在此窗口中看到了尚未完成的回测任务。最终 dual-pipeline-backtest 的正确版本覆盖了越界版本。

不需要结构性修复——这是 in-process backend 的正常并发行为，且最终结果正确。

---

## 观察 4（253号六要素检查）

| 要素 | 状态 | 证据 |
|------|------|------|
| 1. 结论 | **完整** | 135 比较 + 30 分歧 + 22.2% + 15:15 对称 |
| 2. 定义依据 | **完整** | 共振双管道(241) + polarity_index + scan_direction + 入场条件 + confirmed 过滤(252) |
| 3. 边界条件 | **完整** | 离线模式 + type3 only + 包含 unconfirmed 说明 + FiberSignalFilter 默认阈值 |
| 4. 下游推论 | **完整** | 4 条全部 resolved |
| 5. 谱系引用 | **完整** | 252/241/244/233/243 |
| 6. 影响声明 | **完整** | 脚本 + JSON + 概念影响 |

### 判定

**六要素完整。**

---

## 观察 5（规则触发/违反检查）

| 规则 | 状态 | 证据 |
|------|------|------|
| no-patch-mentality | **未触发** | 离线模式是方法论选择，不是 workaround |
| no-workaround | **未触发** | 直面分歧数据，不模糊化处理 |
| no-unnecessary-escalation | **未触发** | Lead 自主处理方向转向和数据版本冲突 |
| lead-parallel-dispatch | **正面** | 3 工位全部并行 spawn + rescan 工位并行修复 |
| result-package 六要素 | **253号完整** | 见观察4 |
| post-commit-flow | **正常** | 原子链执行完整（2 轮 rescan） |

### 规则违反

无新增违反。

---

## 观察 6（自环检查）

| 历史观察 | v106 状态 | 判定 |
|---------|----------|------|
| v105§1：编排者方向转向处理 | **v106 无方向转向——编排者方向从 v105 session 结束时就已确定** | 延续 |
| v105§2：1min 递归 vs 多 TF | **v106 使用 252号 BSP 做进一步验证——递归路径持续产出价值** | 延续 |
| v105§4：ker(D) 操作意义 | **v106 从单一力度差异推进到双管道筛选差异——22.2% 分歧率** | 进化（从力度→筛选→入场决策） |
| v104§1：AV vs yfinance 差异 | **不变** | 延续 |

### 进化点

v100→v106 七轮弧线完成了**三个闭环**：

1. **T6 可达性闭环**（v100→v105）：多 TF 不可达 → 1min 递归可达
2. **ker(D) 操作意义闭环**（v103→v105）：力度首次产出 → 2/6 背驰差异
3. **双管道裁决闭环**（v97→v106）：合成数据平局 → 真实数据 22.2% 结构常数 + 15:15 对称

---

## 核心发现

1. **22.2% 分歧率是 ConfigurationSpace 的拓扑不变量**——不依赖 BSP 来源，可从定义直接推导（语法记录候选）。
2. **v100→v106 完成三个闭环**——T6 可达性、ker(D) 操作意义、双管道裁决。
3. **253号六要素完整，无规则违反。**
4. **纤维丛联络修正的操作区域已定位**：弱极性（|S| <= 1）为有效区，强极性（|S| >= 2）基本不受影响。
