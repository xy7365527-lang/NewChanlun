---
id: '217'
number: 217
title: 元观察——v56-swarm 并行工位隐式依赖 + ceremony_scan 误报模式
type: meta-rule
status: 已结算
date: 2026-02-26
source: meta-observer 二阶观察
session: v56-swarm
rule_version_baseline:
  claude_md_commit: "97f3ba3186f1919ff6abf02dea4247724d47c79a"
  rules_dir_mtime: "2026-02-23 21:10:15 +0000"
depends_on:
  - '216'   # v55-swarm 元观察（异质诊断 + 任务粒度）
  - '175'   # 异质否定设计意图
---

# 217号：元观察——v56-swarm 并行工位隐式依赖 + ceremony_scan 误报模式

## 观察 1：并行工位的 API 变更传播超出 relevant_files 声明

**现象**：gateway-mtf 工位重构了 TFOrchestrator（timeframes.py），将 snapshot 类型从 BiEngineSnapshot 改为 RecursiveOrchestratorSnapshot。该工位的 relevant_files 声明为 gateway.py / timeframes.py / recursive.py。但实际影响传播到：
- test_real_data_e2e.py（`.strokes` → `.bi_snapshot.strokes`，`.n_fractals` → `.bi_snapshot.n_fractals`）
- test_stream_determinism.py（`orch.bus` → `orch.sessions["5m"].engine.bus`）
- test_two_tf_determinism.py（全面适配新 API）

**根因**：relevant_files 是静态声明，无法捕获 API 接口变更的传播链。当一个工位修改了公共接口（如 TFOrchestrator 的返回类型），所有消费该接口的测试文件都需要适配。

**影响**：Lead 需要在并行工位完成后手动修复回归。当前机制可工作（全量测试会暴露问题），但修复成本随并行度增加而增加。

**建议**：不需要新机制——全量测试已经是充分的安全网。但 roadmap subtask 的 relevant_files 应被理解为"直接修改的文件"而非"影响范围"。

## 观察 2：ceremony_scan genealogy_anomaly_detection 误报

**现象**：ceremony_scan 报告"编号 216 在 settled/ 中存在但 block-topology 无对应映射"。实际 216 号的 block 文件存在（hash 命名：9df3ca7c...），且 relations.jsonl 中有对应条目。

**根因**：genealogy_anomaly_detection 用谱系编号在 block 文件名中做字面量匹配，但 block 文件以内容 hash 命名，不含编号。应改为读取 block JSON 内容中的 id 字段匹配。

**分类**：行动类（代码 bug），不是概念层问题。

## 观察 3：stagnation 检测的语义问题

**现象**：ceremony_scan 报告"t-1 到 t 之间 settled 计数未变化（均为 216），RTAS 循环可能停滞"。实际 v56-swarm 产出了大量代码（9 个文件，1417 行新增），但不产出新谱系。

**根因**：stagnation 检测仅以谱系增量为指标，不考虑代码产出。代码工位蜂群（如 v56-swarm P1/P2）的正常模式就是"大量代码产出 + 零谱系增量"。

**建议**：stagnation 检测应区分蜂群类型——概念蜂群（应产谱系）vs 代码蜂群（应产代码+测试）。或者增加 commit 增量作为辅助指标。

## 自环检查

与 216号对比：
- 216号观察"异质诊断应包含实际代码验证步骤" → v56-swarm 中 Codex 诊断准确率显著提高（5/5 缺口确认存在 vs v55-swarm 的 2/4），说明 VDW 机制和 Codex 读取实际代码的改进有效。**收敛信号**。
- 216号观察"任务粒度默认偏小" → v56-swarm 的 P2 工位（gateway-mtf）粒度适中（重构整个 TFOrchestrator），但引发了隐式依赖问题。**发散信号**——粒度增大后新问题出现。
